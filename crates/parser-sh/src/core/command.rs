use crate::{
    lexer::Token,
    walker::{Expand, TreeItem, Walker, Word},
};
use std::{fmt, iter::Peekable};

#[derive(Debug, PartialEq, Eq)]
pub enum Cmd {
    Simple(SimpleCmd),
    Pipeline(Box<Cmd>, Box<Cmd>),
    And(Box<Cmd>, Box<Cmd>),
    Or(Box<Cmd>, Box<Cmd>),
    Not(Box<Cmd>),
    Empty,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Redirection {
    Input(Word),
    Output(Word),
    Append(Word),
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct SimpleCmd {
    pub cmd: Option<Word>,
    pub args: Vec<Word>,
    pub assignments: Vec<(String, Word)>,
    pub redirections: Vec<Redirection>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CmdError {
    MissingCommand,
    MissingWord,
    BackgroundUnsupported,
    Syntax(String),
}

impl fmt::Display for CmdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCommand => f.write_str("expected a command"),
            Self::MissingWord => f.write_str("expected a word after redirection"),
            Self::BackgroundUnsupported => f.write_str("background jobs are not supported"),
            Self::Syntax(message) => f.write_str(message),
        }
    }
}
impl std::error::Error for CmdError {}

pub struct Parser<I: Iterator<Item = Token>> {
    tokens: Peekable<Walker<I>>,
}

impl<I: Iterator<Item = Token>> Parser<I> {
    pub fn new(tokens: I) -> Self {
        Self {
            tokens: Walker::new(tokens).peekable(),
        }
    }

    pub fn next(&mut self) -> Option<Result<Cmd, CmdError>> {
        while matches!(self.tokens.peek(), Some(TreeItem::StatementEnd)) {
            self.tokens.next();
        }
        self.tokens.peek()?;
        let result = self.and_or();
        if matches!(self.tokens.peek(), Some(TreeItem::StatementEnd)) {
            self.tokens.next();
        }
        Some(result)
    }

    fn and_or(&mut self) -> Result<Cmd, CmdError> {
        let mut left = self.pipeline()?;
        loop {
            let op = match self.tokens.peek() {
                Some(TreeItem::And) => true,
                Some(TreeItem::Or) => false,
                _ => break,
            };
            self.tokens.next();
            let right = self.pipeline()?;
            left = if op {
                Cmd::And(Box::new(left), Box::new(right))
            } else {
                Cmd::Or(Box::new(left), Box::new(right))
            };
        }
        Ok(left)
    }

    fn pipeline(&mut self) -> Result<Cmd, CmdError> {
        let mut negated = false;
        while matches!(self.tokens.peek(), Some(TreeItem::Bang)) {
            self.tokens.next();
            negated = !negated;
        }
        let mut left = self.simple()?;
        while matches!(self.tokens.peek(), Some(TreeItem::Pipe)) {
            self.tokens.next();
            let right = self.simple()?;
            left = Cmd::Pipeline(Box::new(left), Box::new(right));
        }
        if negated {
            Ok(Cmd::Not(Box::new(left)))
        } else {
            Ok(left)
        }
    }

    fn simple(&mut self) -> Result<Cmd, CmdError> {
        let mut command = SimpleCmd::default();
        loop {
            match self.tokens.peek() {
                Some(TreeItem::Word(_)) => {
                    let Some(TreeItem::Word(word)) = self.tokens.next() else {
                        unreachable!()
                    };
                    if command.cmd.is_none() {
                        if let Some(assignment) = assignment(word.clone()) {
                            command.assignments.push(assignment);
                            continue;
                        }
                        command.cmd = Some(word);
                    } else {
                        command.args.push(word);
                    }
                }
                Some(TreeItem::Input | TreeItem::Redirect | TreeItem::Append) => {
                    let operator = self.tokens.next().unwrap();
                    let word = match self.tokens.next() {
                        Some(TreeItem::Word(word)) => word,
                        _ => return Err(CmdError::MissingWord),
                    };
                    command.redirections.push(match operator {
                        TreeItem::Input => Redirection::Input(word),
                        TreeItem::Redirect => Redirection::Output(word),
                        TreeItem::Append => Redirection::Append(word),
                        _ => unreachable!(),
                    });
                }
                Some(TreeItem::Background) => {
                    self.tokens.next();
                    return Err(CmdError::BackgroundUnsupported);
                }
                Some(TreeItem::Bang) if command.cmd.is_some() => {
                    self.tokens.next();
                    command.args.push(vec![Expand::Literal("!".into())]);
                }
                Some(TreeItem::Error(_)) => {
                    let Some(TreeItem::Error(message)) = self.tokens.next() else {
                        unreachable!()
                    };
                    return Err(CmdError::Syntax(message));
                }
                _ => break,
            }
        }
        if command.cmd.is_none()
            && command.assignments.is_empty()
            && command.redirections.is_empty()
        {
            Err(CmdError::MissingCommand)
        } else {
            Ok(Cmd::Simple(command))
        }
    }
}

fn assignment(word: Word) -> Option<(String, Word)> {
    let Expand::Literal(first) = word.first()? else {
        return None;
    };
    let (name, value) = first.split_once('=')?;
    if name.is_empty()
        || !name.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
        || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return None;
    }
    let name = name.to_owned();
    let mut rest = vec![Expand::Literal(value.into())];
    rest.extend(word.into_iter().skip(1));
    Some((name, rest))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    fn parse(input: &str) -> Cmd {
        Parser::new(Lexer::new(input.chars()))
            .next()
            .unwrap()
            .unwrap()
    }

    #[test]
    fn parses_words_and_assignments_without_expanding() {
        let Cmd::Simple(cmd) = parse("A=hello echo \"$A world\"") else {
            panic!()
        };
        assert_eq!(cmd.assignments[0].0, "A");
        assert_eq!(cmd.cmd, Some(vec![Expand::Literal("echo".into())]));
        assert_eq!(
            cmd.args[0],
            vec![Expand::Var("A".into()), Expand::Literal(" world".into())]
        );
    }

    #[test]
    fn operators_have_shell_precedence() {
        assert!(matches!(parse("a || b && c | d"), Cmd::And(_, _)));
        assert!(matches!(parse("a | b && c"), Cmd::And(_, _)));
    }

    #[test]
    fn reports_incomplete_syntax() {
        for source in [
            "echo 'missing",
            "echo \"missing",
            "echo $(missing",
            "echo ${missing",
            "echo >",
        ] {
            assert!(
                Parser::new(Lexer::new(source.chars()))
                    .next()
                    .unwrap()
                    .is_err(),
                "{source}"
            );
        }
    }
}
