use crate::lexer::Token;
use std::iter::Peekable;

pub type Word = Vec<Expand>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expand {
    Literal(String),
    Var(String),
    Home,
    Sub(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeItem {
    Word(Word),
    And,
    Or,
    Pipe,
    Redirect,
    Append,
    Input,
    Background,
    Bang,
    StatementEnd,
    Error(String),
}

pub struct Walker<I: Iterator<Item = Token>> {
    tokens: Peekable<I>,
    pending: Option<TreeItem>,
}

impl<I: Iterator<Item = Token>> Walker<I> {
    pub fn new(tokens: I) -> Self {
        Self {
            tokens: tokens.peekable(),
            pending: None,
        }
    }
}

impl<I: Iterator<Item = Token>> Iterator for Walker<I> {
    type Item = TreeItem;
    fn next(&mut self) -> Option<TreeItem> {
        if let Some(item) = self.pending.take() {
            return Some(item);
        }
        let mut word = Vec::new();
        while let Some(token) = self.tokens.next() {
            let operator = match token {
                Token::Word(text) => {
                    if word.is_empty()
                        && text.starts_with('~')
                        && (text.len() == 1 || text.as_bytes().get(1) == Some(&b'/'))
                    {
                        word.push(Expand::Home);
                        if text.len() > 1 {
                            word.push(Expand::Literal(text[1..].into()));
                        }
                    } else {
                        word.push(Expand::Literal(text));
                    }
                    continue;
                }
                Token::Single(text) => {
                    word.push(Expand::Literal(text));
                    continue;
                }
                Token::Variable(name) => {
                    word.push(Expand::Var(name));
                    continue;
                }
                Token::Sub(text) => {
                    word.push(Expand::Sub(text));
                    continue;
                }
                Token::Double(parts) => {
                    for part in parts {
                        match part {
                            Token::Word(text) | Token::Single(text) => {
                                word.push(Expand::Literal(text))
                            }
                            Token::Variable(name) => word.push(Expand::Var(name)),
                            Token::Sub(text) => word.push(Expand::Sub(text)),
                            _ => unreachable!(),
                        }
                    }
                    continue;
                }
                Token::Space => {
                    if !word.is_empty() {
                        return Some(TreeItem::Word(word));
                    }
                    continue;
                }
                Token::End => TreeItem::StatementEnd,
                Token::Pipe => TreeItem::Pipe,
                Token::And => TreeItem::And,
                Token::Or => TreeItem::Or,
                Token::Amp => TreeItem::Background,
                Token::Input => TreeItem::Input,
                Token::Output => TreeItem::Redirect,
                Token::Append => TreeItem::Append,
                Token::Bang => TreeItem::Bang,
                Token::Error(message) => TreeItem::Error(message),
            };
            if !word.is_empty() {
                self.pending = Some(operator);
                return Some(TreeItem::Word(word));
            }
            return Some(operator);
        }
        if word.is_empty() {
            None
        } else {
            Some(TreeItem::Word(word))
        }
    }
}
