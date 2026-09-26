mod cli;

use clap::Parser;
use std::io::{IsTerminal, Read};
use std::process::ExitCode;

use shards::julia::{Event as JuliaEvent, Session as JuliaSession};
use shards::language::Language;
use shards::shell::{Event as ShellEvent, Session as ShSession};

struct Frontend {
    language: Language,
    sh: ShSession,
    julia: JuliaSession,
    status: i32,
    exit: bool,
}

impl Frontend {
    fn new(language: Language) -> Self {
        Self {
            language,
            sh: ShSession::default(),
            julia: JuliaSession::default(),
            status: 0,
            exit: false,
        }
    }

    fn prompt(&self) -> &'static str {
        match self.language {
            Language::Rust => "shards:rust> ",
            Language::Sh => "shards:sh> ",
            Language::Julia => "shards:julia> ",
        }
    }

    fn run_line(&mut self, line: &str) -> std::result::Result<(), String> {
        let line = line.trim();
        if line.is_empty()
            || (self.language == Language::Rust && line.starts_with("//"))
            || (self.language == Language::Sh && line.starts_with('#'))
            || (self.language == Language::Julia && line.starts_with('#'))
        {
            return Ok(());
        }

        match self.language {
            Language::Rust => {
                let call = parser_rust::parse_call(line).map_err(|error| error.to_string())?;
                if call.program == "shards" {
                    match call.args.as_slice() {
                        [command, name] if command == "lang" => {
                            self.language = Language::parse(name).ok_or_else(|| {
                                "shards: expected `shards(lang, sh|rust|julia)`".to_owned()
                            })?;
                            self.status = 0;
                        }
                        _ => {
                            return Err("shards: expected `shards(lang, sh|rust|julia)`".to_owned())
                        }
                    }
                } else if call.program == "exit" && call.args.is_empty() {
                    self.exit = true;
                    self.status = 0;
                } else {
                    self.status =
                        shards::rushi::run_call(call).map_err(|error| error.to_string())?;
                }
            }
            Language::Sh => match self.sh.run_line(line).map_err(|error| error.to_string())? {
                ShellEvent::Continue(status) => self.status = status,
                ShellEvent::Switch(language) => {
                    self.language = language;
                    self.status = 0;
                }
                ShellEvent::Exit(status) => {
                    self.status = status;
                    self.exit = true;
                }
            },
            Language::Julia => match self
                .julia
                .run_line(line)
                .map_err(|error| error.to_string())?
            {
                JuliaEvent::Continue(status) => self.status = status,
                JuliaEvent::Switch(language) => {
                    self.language = language;
                    self.status = 0;
                }
                JuliaEvent::Exit(status) => {
                    self.status = status;
                    self.exit = true;
                }
            },
        }
        Ok(())
    }
}

fn main() -> ExitCode {
    let args = cli::ShardsArgs::parse();
    let mut frontend = Frontend::new(Language::parse(&args.lang).unwrap());

    if let Some(command) = args.command {
        return run_source(&command, &mut frontend);
    }

    if let Some(path) = args.file {
        let mut source = String::new();
        let result = if path.as_os_str() == "-" {
            std::io::stdin().read_to_string(&mut source)
        } else {
            std::fs::read_to_string(&path).map(|text| {
                source = text;
                source.len()
            })
        };
        return match result {
            Ok(_) => run_source(&source, &mut frontend),
            Err(error) => {
                eprintln!("shards: {}: {error}", path.display());
                ExitCode::FAILURE
            }
        };
    }

    if args.interactive || std::io::stdin().is_terminal() {
        while let Some(line) = next(frontend.prompt()) {
            if let Err(error) = frontend.run_line(&line) {
                eprintln!("{error}");
                frontend.status = 1;
            }
            if frontend.exit {
                break;
            }
        }
        exit_code(frontend.status)
    } else {
        let mut source = String::new();
        match std::io::stdin().read_to_string(&mut source) {
            Ok(_) => run_source(&source, &mut frontend),
            Err(error) => {
                eprintln!("shards: stdin: {error}");
                ExitCode::FAILURE
            }
        }
    }
}

fn exit_code(status: i32) -> ExitCode {
    ExitCode::from(if status < 0 { 1 } else { status.min(255) as u8 })
}

fn run_source(source: &str, frontend: &mut Frontend) -> ExitCode {
    for line in source.lines() {
        if let Err(error) = frontend.run_line(line) {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
        if frontend.exit {
            break;
        }
    }
    exit_code(frontend.status)
}

#[cfg(test)]
mod tests {
    use super::{Frontend, Language};

    #[test]
    fn switches_parsers_in_one_session() {
        let mut frontend = Frontend::new(Language::Rust);
        frontend.run_line("shards(lang, sh)").unwrap();
        assert_eq!(frontend.language, Language::Sh);

        frontend.run_line("shards lang rust").unwrap();
        assert_eq!(frontend.language, Language::Rust);

        frontend.run_line("shards(lang, julia)").unwrap();
        assert_eq!(frontend.language, Language::Julia);
        frontend.run_line("value = \"remembered\"").unwrap();
        frontend.run_line("shards(lang, sh)").unwrap();
        assert_eq!(frontend.language, Language::Sh);
        frontend.run_line("shards lang julia").unwrap();
        assert_eq!(frontend.language, Language::Julia);
        frontend.run_line("string(value)").unwrap();
    }

    #[test]
    fn julia_exit_sets_process_status() {
        let mut frontend = Frontend::new(Language::Julia);
        frontend.run_line("exit(7)").unwrap();
        assert!(frontend.exit);
        assert_eq!(frontend.status, 7);
    }
}

use std::fmt;

use resu::{Context, Result, ResultExt};

pub fn next(prompt: &str) -> Option<String> {
    loop {
        crossterm::terminal::enable_raw_mode().unwrap();
        let res = readline(prompt);
        crossterm::terminal::disable_raw_mode().unwrap();

        match res {
            Ok(ReadlineOutput::Line(s)) => return Some(s),
            Ok(ReadlineOutput::Exit) => {
                eprintln!("^C");
                continue;
            }
            Ok(ReadlineOutput::Eof) => return None,
            Err(e) => {
                // this often comes after some shit so it is best to just do this
                log::error!("\r\n\n{:?}", e);
                continue;
            }
        }
    }
}

#[derive(Debug)]
enum PromptError {
    /// Error when writing data
    Write,
}

impl fmt::Display for PromptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PromptError::Write => f.write_str("failed to write data"),
        }
    }
}
impl Context for PromptError {}

#[derive(Debug, Default, Clone)]
struct LineBuffer {
    /// Buffer that data is written to
    buf: Vec<char>,
    /// Position of cursor, if none then the cursor is at the end. Repersents
    /// the distance from the left edge. Aka the start of the buffer is 0.
    pos: Option<usize>,
}

impl fmt::Display for LineBuffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for c in self.buf.iter() {
            std::fmt::Write::write_char(f, *c)?;
        }
        Ok(())
    }
}

impl LineBuffer {
    /// Adds a character to the buffer. Returns true if a render is needed.
    fn push(&mut self, c: char) -> InsertResult {
        if let Some(ofst) = self.pos.as_mut() {
            if *ofst > self.buf.len() {
                unreachable!("cursor out of buffer")
            } else {
                self.buf.insert(*ofst, c);
                *ofst += 1;
                InsertResult::Render
            }
        } else {
            self.buf.push(c);
            InsertResult::Render
        }
    }

    /// Removes the character directly to the left of the cursor. Returns true
    /// if a render is needed.
    fn pop(&mut self) -> InsertResult {
        if let Some(ofst) = self.pos.as_mut() {
            if *ofst == 0 {
                // there is nothing to remove at the start of the word
                InsertResult::None
            } else {
                self.buf.remove(*ofst - 1);
                *ofst -= 1;
                InsertResult::Render
            }
        } else {
            self.buf.pop();
            InsertResult::Render
        }
    }

    fn left(&mut self) -> InsertResult {
        if let Some(ofst) = self.pos.as_mut() {
            if *ofst == 0 {
                InsertResult::None
            } else {
                *ofst -= 1;
                InsertResult::Render
            }
        } else if self.buf.is_empty() {
            InsertResult::None
        } else {
            self.pos = Some(self.buf.len() - 1);
            InsertResult::Render
        }
    }

    fn right(&mut self) -> InsertResult {
        if let Some(ofst) = self.pos.as_mut() {
            *ofst += 1;
            if *ofst >= self.buf.len() {
                self.pos = None;
            }
            InsertResult::Render
        } else {
            InsertResult::None
        }
    }

    // Sets the buffer to the specified buffer
    // fn set(&mut self, buf: &str) -> InsertResult {
    //     self.buf = buf.chars().collect();
    //     if let Some(ofst) = self.pos {
    //         if ofst >= self.buf.len() {
    //             self.pos = None;
    //         }
    //     }
    //     InsertResult::Render
    // }
}

enum InsertResult {
    Render,
    Done,
    None,
}

enum ReadlineOutput {
    Line(String),
    /// When C-d is pressed on an empty time
    Eof,
    /// Corisponds to C-c
    Exit,
}

/// Expects the terminal to be in raw mod when called.
fn readline(prompt: &str) -> Result<ReadlineOutput, PromptError> {
    let mut stdout = std::io::stdout();

    let mut buff = LineBuffer::default();

    // let mut hist = 0usize;

    render_line(&mut stdout, prompt, &buff).unwrap();

    use crossterm::event::Event as E;
    use crossterm::event::KeyCode as K;
    use crossterm::event::KeyModifiers as Km;

    while let Ok(read) = crossterm::event::read() {
        let result = match read {
            E::Key(k) => match (k.code, k.modifiers) {
                (K::Backspace, _) => buff.pop(),
                (K::Enter, _) => InsertResult::Done,
                (K::Char(ch), Km::NONE) => buff.push(ch),
                (K::Char(ch), Km::SHIFT) => buff.push(ch.to_ascii_uppercase()),
                (K::Char('c'), Km::CONTROL) => {
                    return Ok(ReadlineOutput::Exit);
                }
                (K::Char('d'), Km::CONTROL) => {
                    if buff.buf.is_empty() {
                        return Ok(ReadlineOutput::Eof);
                    }
                    InsertResult::None
                }
                (K::Char('l'), Km::CONTROL) => {
                    // the call to render flushes these changes
                    crossterm::queue!(
                        stdout,
                        crossterm::cursor::MoveTo(0, 0),
                        crossterm::terminal::Clear(crossterm::terminal::ClearType::All)
                    )
                    .change_context(PromptError::Write)?;

                    InsertResult::Render
                }

                (K::Left, _) => buff.left(),
                (K::Right, _) => buff.right(),
                // (K::Up, _) => {
                //     hist += 1;
                //     if let Some(p) = state.get_history(hist) {
                //         buff.set(p);
                //         InsertResult::Render
                //     } else {
                //         hist -= 1;
                //         InsertResult::None
                //     }
                // }
                // (K::Down, _) => {
                //     hist = hist.saturating_sub(1);
                //     if let Some(s) = state.get_history(hist) {
                //         buff.set(s)
                //     } else {
                //         if !buff.buf.is_empty() {
                //             buff.set("")
                //         } else {
                //             InsertResult::None
                //         }
                //     }
                // }

                // crossterm::event::KeyCode::Tab => todo!(),
                // crossterm::event::KeyCode::BackTab => todo!(),
                (K::Esc, _) => {
                    return Ok(ReadlineOutput::Eof);
                }

                // Most keys no one cares about
                _ => InsertResult::None,
            },
            E::Paste(pasted) => {
                for ch in pasted.chars() {
                    buff.push(ch);
                }
                InsertResult::Render
            }
            E::Resize(_, _) => InsertResult::Render,
            E::FocusGained | E::FocusLost | E::Mouse(_) => InsertResult::None,
        };

        match result {
            InsertResult::Render => {
                render_line(&mut stdout, prompt, &buff).unwrap();
            }
            InsertResult::Done => {
                break;
            }
            InsertResult::None => {}
        }
    }

    print!("\r\n");

    // buff implements `Display`
    Ok(ReadlineOutput::Line(ToString::to_string(&buff)))
}

fn render_line(
    stdout: &mut std::io::Stdout,
    prompt: &str,
    line: &LineBuffer,
) -> Result<(), PromptError> {
    let pos = line.pos.unwrap_or(line.buf.len()) + prompt.len();
    let pos = pos as u16;

    crossterm::execute!(
        stdout,
        // clear the line
        crossterm::cursor::MoveToColumn(0),
        crossterm::terminal::Clear(crossterm::terminal::ClearType::UntilNewLine),
        // write the new line
        crossterm::style::Print(format!("{}{}", prompt, line)),
        // put the cursor where we want it
        crossterm::cursor::MoveToColumn(pos),
    )
    .change_context(PromptError::Write)
}

// fn main() -> std::process::ExitCode {
//     match shards() {
//         Ok(()) => std::process::ExitCode::SUCCESS,
//         Err(e) => {
//             eprintln!("{:?}", e);
//             return std::process::ExitCode::FAILURE;
//         }
//     }
// }
//
// fn shards() -> Result<(), ShardsError> {
//     let args = RushiArgs::gen();
//
//     args.debug.then(|| {
//         let name = args
//             .debug_file
//             .clone()
//             .unwrap_or_else(|| PathBuf::from("rushi.log"));
//         let Ok(file) = File::create(name) else {
//             return;
//         };
//
//         let _ = simplelog::WriteLogger::init(
//             simplelog::LevelFilter::Info,
//             simplelog::Config::default(),
//             file,
//         );
//
//         log::info!("Debug mode enabled");
//     });
//
//     let interpreter = Interpreter::new(Lang::Rust)
//         .change_context(ShardsError::Ast)
//         .attach_printable("failed to start shards. Do you have any langs intalled?")?;
//
//     // let mut env = UserState::new(&args);
//
//     // source user and system config
//     // let mut paths = ConfigPaths::new(&args);
//     // paths.source(&interpreter, &mut env, &mut sys);
//
//     // let (lsp, rx) = Client::start("rust-analyzer", &[""], None, HashMap::new(), 0, "rls", 100)?;
//     // lsp.initialize(true).await?;
//
//     eprintln!("Welcome to Shards!");
//
//     log::info!("Starting main event loop");
//     // let fatal = true;
//     while let Some(line) = crate::line::next() {
//         parse(&interpreter, line)?;
//     }
//
//     // restore_term_foreground_process_group_for_exit();
//
//     Ok(())
// }
