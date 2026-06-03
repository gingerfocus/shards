use std::fmt;

use resu::{Context, Result, ResultExt};

pub fn next() -> Option<String> {
    loop {
        crossterm::terminal::enable_raw_mode().unwrap();
        let res = readline("$> ");
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
