//! Rushi's Rust-style external command call parser.

use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub struct Call {
    pub program: String,
    pub args: Vec<String>,
}

#[derive(Debug)]
pub enum ParseError {
    Syntax(&'static str),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Syntax(message) => write!(f, "rushi: {message}"),
        }
    }
}

impl std::error::Error for ParseError {}

struct Cursor<'a> {
    rest: &'a str,
}

impl Cursor<'_> {
    fn peek(&self) -> Option<char> {
        self.rest.chars().next()
    }

    fn next(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.rest = &self.rest[ch.len_utf8()..];
        Some(ch)
    }

    fn skip_space(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.next();
        }
    }

    fn argument(&mut self) -> Result<String, ParseError> {
        if matches!(self.peek(), Some('"' | '\'')) {
            let quote = self.next().unwrap();
            let mut result = String::new();
            loop {
                match self.next() {
                    Some(ch) if ch == quote => return Ok(result),
                    Some('\\') if quote == '"' => {
                        let escaped = self.next().ok_or(ParseError::Syntax("unfinished escape"))?;
                        result.push(escaped);
                    }
                    Some(ch) => result.push(ch),
                    None => return Err(ParseError::Syntax("unterminated quoted argument")),
                }
            }
        }

        let mut result = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_whitespace() || matches!(ch, ',' | ')') {
                break;
            }
            result.push(ch);
            self.next();
        }
        if result.is_empty() {
            Err(ParseError::Syntax("expected an argument"))
        } else {
            Ok(result)
        }
    }
}

/// Parse one function-style external command, such as `ls()` or `ls("-la")`.
pub fn parse_call(input: &str) -> Result<Call, ParseError> {
    let input = input.trim();
    let open = input
        .find('(')
        .ok_or(ParseError::Syntax("expected `(` after command name"))?;
    let program = input[..open].trim();
    if program.is_empty()
        || !program
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/'))
    {
        return Err(ParseError::Syntax("invalid command name"));
    }

    let mut cursor = Cursor {
        rest: &input[open + 1..],
    };
    let mut args = Vec::new();
    loop {
        cursor.skip_space();
        if cursor.peek() == Some(')') {
            cursor.next();
            break;
        }
        if cursor.peek().is_none() {
            return Err(ParseError::Syntax("expected `)`"));
        }
        args.push(cursor.argument()?);
        cursor.skip_space();
        match cursor.next() {
            Some(',') => continue,
            Some(')') => break,
            _ => return Err(ParseError::Syntax("expected `,` or `)` after argument")),
        }
    }

    let tail = cursor.rest.trim();
    if !tail.is_empty() && tail != ";" {
        return Err(ParseError::Syntax("unexpected text after command call"));
    }
    Ok(Call {
        program: program.to_owned(),
        args,
    })
}

#[cfg(test)]
mod tests {
    use super::{parse_call, Call};

    #[test]
    fn parses_empty_call() {
        assert_eq!(
            parse_call(" ls() ").unwrap(),
            Call {
                program: "ls".into(),
                args: vec![]
            }
        );
    }

    #[test]
    fn parses_arguments_without_invoking_a_shell() {
        assert_eq!(
            parse_call("ls(-la, 'a directory')").unwrap(),
            Call {
                program: "ls".into(),
                args: vec!["-la".into(), "a directory".into()]
            }
        );
    }

    #[test]
    fn rejects_trailing_code() {
        assert!(parse_call("ls() something_else()").is_err());
    }

    #[test]
    fn parses_language_switch_call() {
        assert_eq!(
            parse_call("shards(lang, sh)").unwrap(),
            Call {
                program: "shards".into(),
                args: vec!["lang".into(), "sh".into()]
            }
        );
    }
}
