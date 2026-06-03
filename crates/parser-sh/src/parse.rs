use rush_core::lexer::Lexer;
use rush_core::walker::{TreeItem, Walker};

use crate::prelude::*;

use crate::util::{OwnedCharBuffer, StaticMap};

use std::iter::Peekable;
use std::os::fd::{FromRawFd, OwnedFd, RawFd};
use std::process::Stdio;

/// The parser reads in tokens and converts them into commands.
/// Parser takes in a token stream and outputs commands.
pub(crate) struct Parser<I>
where
    I: Iterator<Item = Token>,
{
    tokens: Peekable<Walker<I>>,
}

impl<I> Parser<I>
where
    I: Iterator<Item = Token>,
{
    pub fn new(tokens: I) -> Parser<I> {
        Parser {
            tokens: Walker::new(tokens).peekable(),
        }
    }

    pub fn next(&mut self, state: &ShellState) -> Option<Result<Cmd, CmdError>> {
        // when there are no tokens left return
        self.tokens.peek()?;

        log::info!("getting next command.");

        Some(self.get_next(state))
    }

    fn get_next(&mut self, state: &ShellState) -> Result<Cmd, CmdError> {
        let mut cmd = SimpleCmd::default();
        loop {
            let Some(token) = self.tokens.next() else {
                return Ok(cmd.build());
            };
            log::info!("got token: {:?}", token);

            // Todo: things build args and then spaces deliminate them.
            // This way args can contain variables and other as many tokens
            // not one. This reduces the load on the tokenizer.

            match token {
                TreeItem::Word(v) => {
                    let a = v.into_iter().map(|e| crate::walker::expand(e, state)).fold(
                        String::new(),
                        |mut s, e| {
                            s.push_str(&e);
                            s
                        },
                    );
                    cmd.push_ident(a);
                }
                TreeItem::Or => {
                    let c = cmd.build();
                    return Ok(Cmd::Or(Box::new(c), Box::new(self.get_next(state)?)));
                }
                TreeItem::And => {
                    let c = cmd.build();
                    return Ok(Cmd::And(Box::new(c), Box::new(self.get_next(state)?)));
                }
                TreeItem::Pipe => {
                    let c = cmd.build();
                    return Ok(Cmd::Pipeline(Box::new(c), Box::new(self.get_next(state)?)));
                }
                TreeItem::StatmentEnd => return Ok(cmd.build()),
                // TreeItem::Assign(_, _) => todo!(),
                TreeItem::Append => todo!(),
                TreeItem::Redirect => todo!(),
                TreeItem::Background => todo!(),
                TreeItem::Comment => {}
            }
        }
    }
}

#[derive(Debug)]
pub enum CmdError {
    BadToken(Token),
    MissingName,
    SubShell,
}
impl fmt::Display for CmdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CmdError::BadToken(t) => write!(f, "invalid token recieved: {:?}", t),
            CmdError::MissingName => f.write_str("a name is needed to call it"),
            CmdError::SubShell => f.write_str("Error in subshell. This could recurse."),
        }
    }
}
impl Context for CmdError {}

/// If you want to create one
#[derive(Debug, PartialEq)]
pub enum Cmd {
    Simple(SimpleCmd),
    Pipeline(Box<Cmd>, Box<Cmd>),
    And(Box<Cmd>, Box<Cmd>),
    Or(Box<Cmd>, Box<Cmd>),
    Not(Box<Cmd>),
    Empty,
}

/// The most basic command - it, its arguments, and its redirections.
#[derive(Debug, Default, PartialEq)]
pub struct SimpleCmd {
    pub cmd: String,
    pub args: Vec<String>,
    pub env: StaticMap<String, String>,
}

#[derive(Debug, Default)]
pub enum Fd {
    /// Use the parents stdin and stdout. This is almost always what stderr is.
    #[default]
    Inherit,
    Piped(OwnedFd),
}
impl From<Fd> for Stdio {
    fn from(value: Fd) -> Self {
        match value {
            Fd::Inherit => Stdio::inherit(),
            Fd::Piped(fd) => Stdio::from(fd),
        }
    }
}
impl FromRawFd for Fd {
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        Self::Piped(unsafe { OwnedFd::from_raw_fd(fd) })
    }
}

#[derive(Debug, Default)]
pub struct Streams {
    pub stdin: Fd,
    pub stdout: Fd,
    pub stderr: Fd,
}

impl SimpleCmd {
    pub fn push_ident(&mut self, ident: String) {
        if self.cmd.is_empty() {
            self.cmd = ident;
        } else {
            self.args.push(ident);
        }
    }

    pub fn build(self) -> Cmd {
        if self.cmd.is_empty() {
            Cmd::Empty
        } else {
            Cmd::Simple(self)
        }
    }
}

// use nix::unistd::User;
// use os_pipe::pipe;

// // The parser struct. Keeps track of current location in a peekable iter of tokens
// pub struct Parser {
//     shell: Shell,
//     lexer: Lexer,
// }
//
// impl Parser {
//     pub fn new(lexer: Lexer, shell: Shell) -> Parser {
//         Parser { shell, lexer }
//     }
//
//     pub fn get(&mut self) -> Result<Cmd, String> {
//         let mut node = self.get_pipe()?;
//         while let Some(Op(Op::And)) | Some(Op(Op::Or)) = self.lexer.peek() {
//             if let Some(Op(Op::And)) = self.lexer.next_token(&mut self.shell) {
//                 node = Cmd::And(Box::new(node), Box::new(self.get_pipe()?));
//             } else {
//                 node = Cmd::Or(Box::new(node), Box::new(self.get_pipe()?));
//             }
//         }
//         Ok(node)
//     }
//
//     pub fn get_pipe(&mut self) -> Result<Cmd, String> {
//         let mut node = self.get_simple()?;
//         while let Some(Op(Op::Pipe)) = self.lexer.peek() {
//             self.lexer.next();
//             node = Cmd::Pipeline(Box::new(node), Box::new(self.get_simple()?));
//         }
//         Ok(node)
//     }
//
//     pub fn get_simple(&mut self) -> Result<Cmd, String> {
//         if let Some(Op(Op::Bang)) = self.lexer.peek() {
//             self.lexer.next();
//             Ok(Cmd::Not(Box::new(self.get_simple()?)))
//         } else {
//             let mut result = Vec::new();
//             let mut io = Io::new();
//             let mut map = HashMap::new();
//
//             loop {
//                 match self.lexer.peek() {
//                     Some(Word(_)) => {
//                         if let Some(Word(mut expansions)) = self.lexer.next() {
//                             if let [Literal(_)] = &expansions[..] {
//                                 result.push(expansions.pop().unwrap().get_name())
//                             } else {
//                                 let word = self.expand_word(expansions);
//                                 if !word.is_empty() {
//                                     result.push(word)
//                                 }
//                             }
//                         }
//                     }
//                     Some(Assign(_, _)) => {
//                         if let Some(Assign(key, var)) = self.lexer.next() {
//                             map.insert(key, self.expand_word(var));
//                         }
//                     }
//                     Some(Op(Op::Less)) => {
//                         self.lexer.next_token(&mut self.shell);
//                         io.set_stdin(self.token_to_fd(&io)?);
//                     }
//                     Some(Op(Op::More)) => {
//                         self.lexer.next_token(&mut self.shell);
//                         io.set_stdout(self.token_to_fd(&io)?);
//                     }
//                     Some(Integer(_)) => {
//                         if let Some(Integer(int)) = self.lexer.next() {
//                             if let Some(Op(_)) = self.lexer.peek() {
//                                 self.lexer.next();
//                                 match int {
//                                     0 => io.set_stdin(self.token_to_fd(&io)?),
//                                     1 => io.set_stdout(self.token_to_fd(&io)?),
//                                     2 => io.set_stderr(self.token_to_fd(&io)?),
//                                     _ => todo!(),
//                                 }
//                             } else {
//                                 result.push(int.to_string());
//                             }
//                         }
//                     }
//                     _ => break,
//                 }
//             }
//             if result.is_empty() {
//                 if map.is_empty() {
//                     Err(String::from("rush: expected command but found none"))
//                 } else {
//                     map = map
//                         .into_iter()
//                         .filter_map(|(k, v)| {
//                             if env::var_os(&k).is_some() {
//                                 env::set_var(k, v);
//                                 None
//                             } else {
//                                 Some((k, v))
//                             }
//                         })
//                         .collect();
//                     self.shell.borrow_mut().vars.extend(map);
//                     Ok(Cmd::Empty)
//                 }
//             } else {
//                 let mut cmd = Simple::new(result.remove(0), result, io);
//                 if !map.is_empty() {
//                     cmd.add_env(map);
//                 }
//                 Ok(Cmd::Simple(cmd))
//             }
//         }
//     }
//

// fn get_next_fd<I: Iterator<Item = char>>(lexer: &mut Lexer<I>) -> Result<Stdio, String> {
//     let Some(Token::Amp) = lexer.next() else {
//         panic!("rush: expected redirection location but found none");
//     };
//
//     let Some(Token::Int(i)) = lexer.next() else {
//         panic!();
//     };
//
//     Ok(Stdio::null())
// }

/// Prompter reads user input. Then creates a [`Parser`] to turn it into
/// commands.
#[derive(Default)]
pub(crate) struct Prompter {
    commads: Option<Parser<Lexer<OwnedCharBuffer>>>,
}

impl Prompter {
    pub fn next(&mut self, state: &mut ShellState) -> Option<Result<Cmd, CmdError>> {
        loop {
            if let Some(cmd) = self.commads.as_mut().and_then(|i| i.next(state)) {
                return Some(cmd);
            }

            crossterm::terminal::enable_raw_mode().unwrap();
            let res = read_line("$> ", state);
            crossterm::terminal::disable_raw_mode().unwrap();

            let line = match res {
                Ok(ReadlineOutput::Line(s)) => s,
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
            };
            state.add_history(line.trim_end());
            log::info!("got line: {}", line.trim());

            let p = Parser::new(Lexer::new(OwnedCharBuffer::new(line)));
            _ = self.commads.insert(p);
        }
    }
}

#[derive(Debug)]
enum PromptError {
    /// Temporary value used to when for whatever reason I have not made the
    /// code yet. This is better than a panic beacuse we restore the terminal
    /// before continuing
    Unimplemented(&'static str),

    /// Error when writing data
    Write,
}
impl fmt::Display for PromptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PromptError::Unimplemented(msg) => write!(f, "unimplemented!({:?})", msg),
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

    fn enter(&mut self) -> InsertResult {
        if self.pos.is_none() {
            self.buf.push('\n');
            InsertResult::Done
        } else {
            todo!()
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

    /// Sets the buffer to the specified buffer
    fn set(&mut self, buf: &str) -> InsertResult {
        self.buf = buf.chars().collect();
        if let Some(ofst) = self.pos {
            if ofst >= self.buf.len() {
                self.pos = None;
            }
        }
        InsertResult::Render
    }
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
fn read_line(prompt: &str, state: &mut ShellState) -> Result<ReadlineOutput, PromptError> {
    let mut stdout = std::io::stdout();

    let mut buff = LineBuffer::default();

    let mut hist = 0usize;

    render_line(&mut stdout, prompt, &buff).unwrap();

    use crossterm::event::Event as E;
    use crossterm::event::KeyCode as K;
    use crossterm::event::KeyModifiers as Km;

    while let Ok(read) = crossterm::event::read() {
        let result = match read {
            E::Key(k) => match (k.code, k.modifiers) {
                (K::Backspace, _) => buff.pop(),
                (K::Enter, _) => {
                    if buff.pos.is_some() {
                        return Err(Report::new(PromptError::Unimplemented(
                            "handle new line in middle of word",
                        )));
                    }

                    buff.enter()
                }
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
                (K::Up, _) => {
                    hist += 1;
                    if let Some(p) = state.get_history(hist) {
                        buff.set(p);
                        InsertResult::Render
                    } else {
                        hist -= 1;
                        InsertResult::None
                    }
                }
                (crossterm::event::KeyCode::Down, _) => {
                    hist = hist.saturating_sub(1);
                    if let Some(s) = state.get_history(hist) {
                        buff.set(s)
                    } else if !buff.buf.is_empty() {
                        buff.set("")
                    } else {
                        InsertResult::None
                    }
                }
                // crossterm::event::KeyCode::Tab => todo!(),
                // crossterm::event::KeyCode::BackTab => todo!(),
                (K::Esc, _) => {
                    return Ok(ReadlineOutput::Eof);
                }

                // Most keys no one cares about
                _ => InsertResult::None,
            },
            E::Paste(_) => unimplemented!(),
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
