//! A small Julia-style statement parser for the Shards Julia frontend.
//!
//! It handles one call or assignment at a time, leaving values for the executor.

use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub struct Call {
    pub function: String,
    pub args: Vec<Argument>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Argument {
    Identifier(String),
    String(String),
    Number(String),
    Call(Box<Call>),
}

#[derive(Debug, PartialEq, Eq)]
pub enum Statement {
    Call(Call),
    Assign(String, Argument),
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseError(pub &'static str);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "julia: {}", self.0)
    }
}

impl std::error::Error for ParseError {}

fn identifier_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

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

    fn argument(&mut self) -> Result<Argument, ParseError> {
        if self.peek() == Some('"') {
            self.next();
            let mut value = String::new();
            loop {
                match self.next() {
                    Some('"') => return Ok(Argument::String(value)),
                    Some('\\') => {
                        let escaped = self.next().ok_or(ParseError("unfinished string escape"))?;
                        value.push(match escaped {
                            'n' => '\n',
                            't' => '\t',
                            '"' => '"',
                            '\\' => '\\',
                            _ => return Err(ParseError("unsupported string escape")),
                        });
                    }
                    Some(ch) => value.push(ch),
                    None => return Err(ParseError("unterminated string")),
                }
            }
        }

        let mut value = String::new();
        while self.peek().is_some_and(identifier_char) {
            value.push(self.next().unwrap());
        }
        if value.is_empty() {
            Err(ParseError("expected an identifier or string argument"))
        } else if value.starts_with(|ch: char| ch.is_ascii_digit()) {
            if value.chars().all(|ch| ch.is_ascii_digit()) {
                Ok(Argument::Number(value))
            } else {
                Err(ParseError("invalid numeric argument"))
            }
        } else if self.peek() == Some('(') {
            self.next();
            Ok(Argument::Call(Box::new(self.call(value)?)))
        } else {
            Ok(Argument::Identifier(value))
        }
    }

    fn call(&mut self, function: String) -> Result<Call, ParseError> {
        let mut args = Vec::new();
        loop {
            self.skip_space();
            if self.peek() == Some(')') {
                self.next();
                break;
            }
            if self.peek().is_none() {
                return Err(ParseError("expected `)`"));
            }
            args.push(self.argument()?);
            self.skip_space();
            match self.next() {
                Some(',') => continue,
                Some(')') => break,
                _ => return Err(ParseError("expected `,` or `)`")),
            }
        }
        Ok(Call { function, args })
    }
}

/// Parse one Julia-style call, such as `ls()` or `ls("-la", path)`.
pub fn parse_call(input: &str) -> Result<Call, ParseError> {
    let input = input.trim();
    let open = input.find('(').ok_or(ParseError("expected `(`"))?;
    let function = input[..open].trim();
    if function.is_empty()
        || function.starts_with(|ch: char| ch.is_ascii_digit())
        || !function.chars().all(identifier_char)
    {
        return Err(ParseError("invalid function name"));
    }

    let mut cursor = Cursor {
        rest: &input[open + 1..],
    };
    let call = cursor.call(function.into())?;

    let tail = cursor.rest.trim();
    if !tail.is_empty() && tail != ";" {
        return Err(ParseError("unexpected text after call"));
    }
    Ok(call)
}

/// Parse one Julia-style statement. Variable values are resolved by the executor.
pub fn parse_statement(input: &str) -> Result<Statement, ParseError> {
    let input = input.trim();
    let mut name_end = 0;
    for (index, ch) in input.char_indices() {
        if identifier_char(ch) {
            name_end = index + ch.len_utf8();
        } else {
            break;
        }
    }
    if name_end > 0 {
        let name = &input[..name_end];
        let rest = input[name_end..].trim_start();
        if let Some(value) = rest.strip_prefix('=') {
            if name.starts_with(|ch: char| ch.is_ascii_digit()) {
                return Err(ParseError("invalid variable name"));
            }
            let mut cursor = Cursor {
                rest: value.trim_start(),
            };
            let expression = cursor.argument()?;
            let tail = cursor.rest.trim();
            if !tail.is_empty() && tail != ";" {
                return Err(ParseError("unexpected text after assignment"));
            }
            return Ok(Statement::Assign(name.into(), expression));
        }
    }
    parse_call(input).map(Statement::Call)
}

#[cfg(test)]
mod tests {
    use super::{parse_call, parse_statement, Argument, Call, Statement};

    #[test]
    fn parses_empty_call() {
        assert_eq!(
            parse_call("ls()").unwrap(),
            Call {
                function: "ls".into(),
                args: vec![]
            }
        );
    }

    #[test]
    fn keeps_strings_distinct_from_identifiers() {
        assert_eq!(
            parse_call("ls(\"-la\", path)").unwrap(),
            Call {
                function: "ls".into(),
                args: vec![
                    Argument::String("-la".into()),
                    Argument::Identifier("path".into())
                ]
            }
        );
    }

    #[test]
    fn rejects_trailing_source() {
        assert!(parse_call("ls() other()").is_err());
    }

    #[test]
    fn parses_assignment_and_nested_call_without_evaluating() {
        assert_eq!(
            parse_statement("value = string(\"a\", \"b\")").unwrap(),
            Statement::Assign(
                "value".into(),
                Argument::Call(Box::new(Call {
                    function: "string".into(),
                    args: vec![Argument::String("a".into()), Argument::String("b".into())],
                }))
            )
        );
    }

    #[test]
    fn parses_numeric_argument() {
        assert_eq!(
            parse_call("exit(7)").unwrap().args,
            vec![Argument::Number("7".into())]
        );
    }
}
