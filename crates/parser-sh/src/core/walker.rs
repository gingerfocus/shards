use std::iter::Peekable;

use crate::prelude::*;

pub struct Walker<I>
where
    I: Iterator<Item = Token>,
{
    tokens: Peekable<I>,
}

impl<I> Walker<I>
where
    I: Iterator<Item = Token>,
{
    pub fn new(tokens: I) -> Self {
        Self {
            tokens: tokens.peekable(),
        }
    }

    // Reads some text. This is by far the most powerful of the reads. The
    // best way to understand this is that if it could be on either side of
    // an equal sign and make sence then here it is.
    // fn read_expression(&mut self) -> Vec<Expand> {
    //     todo!()
    // }

    // Polls the next value from this token walker. A return value of `None`
    // from this funtion means that there is not enough info to produce a
    // token yet. A return value of `Some(None)` means there are no more
    // tokens that can be produced.
    //
    // This will only happen if this has been called.
    // pub fn poll_next(&mut self, next: Option<Token>) -> Option<Option<TreeItem>> {
    //     todo!()
    // }
}

impl TryFrom<Vec<Expand>> for TreeItem {
    type Error = ();

    fn try_from(value: Vec<Expand>) -> std::result::Result<Self, Self::Error> {
        if value.is_empty() {
            Err(())
        } else {
            Ok(TreeItem::Word(value))
        }
    }
}

impl<I> Iterator for Walker<I>
where
    I: Iterator<Item = Token>,
{
    type Item = TreeItem;

    fn next(&mut self) -> Option<Self::Item> {
        let mut expr = vec![];
        while let Some(t) = self.tokens.peek() {
            match t {
                Token::Newline => {
                    has!(TreeItem::try_from(std::mem::take(&mut expr)).ok());
                    let _ = self.tokens.next();
                    return Some(TreeItem::StatmentEnd);
                }

                Token::Pipe => {
                    let _ = self.tokens.next();
                    if let Some(Token::Pipe) = self.tokens.peek() {
                        let _ = self.tokens.next();
                        return Some(TreeItem::Or);
                    }
                    return Some(TreeItem::Pipe);
                }
                Token::Amp => {
                    let _ = self.tokens.next();
                    match self.tokens.peek() {
                        Some(Token::Amp) => {
                            let _ = self.tokens.next();
                            return Some(TreeItem::And);
                        }
                        Some(_) => todo!(),
                        None => return Some(TreeItem::Background),
                    }
                }
                Token::SemiColor => todo!(),
                Token::LeftArrow => todo!(),
                Token::RightArrow => {
                    // dont consume the token if we already have something
                    // buffered
                    has!(TreeItem::try_from(std::mem::take(&mut expr)).ok());

                    // now it can go
                    let _ = self.tokens.next();
                    match self.tokens.peek() {
                        Some(Token::RightArrow) => {
                            let _ = self.tokens.next();
                            return Some(TreeItem::Append);
                        }
                        Some(Token::Space) | Some(Token::Ident(_)) => {
                            return Some(TreeItem::Redirect);
                        }
                        Some(_) => todo!("bad token"),
                        None => todo!("expected string. found eof"),
                    }
                }
                Token::OpenParen => todo!(),
                Token::CloseParen => todo!(),
                Token::Doller => {
                    let _ = self.tokens.next();
                    match self.tokens.peek() {
                        Some(Token::Ident(_)) => {
                            let Some(Token::Ident(s)) = self.tokens.next() else {
                                unreachable!()
                            };
                            expr.push(Expand::Var(s))
                        }
                        Some(Token::Space) => {
                            expr.push(Expand::Literal(String::from("$")));
                        }
                        // this is handled in tokenization
                        Some(Token::OpenParen) => unreachable!(),
                        Some(_) => todo!(),
                        None => todo!(),
                    }
                }
                Token::DoubleQuote(_) => {
                    let Some(Token::DoubleQuote(v)) = self.tokens.next() else {
                        unreachable!()
                    };
                    dbg!(&v);
                    let mut v = v.into_iter();
                    let mut e = vec![];
                    while let Some(t) = v.next() {
                        match t {
                            Token::Doller => {
                                if let Some(Token::Ident(s)) = v.next() {
                                    e.push(Expand::Var(s));
                                } else {
                                    e.push(Expand::Literal(String::from("")));
                                }
                            }
                            Token::Ident(s) => e.push(Expand::Literal(s)),
                            Token::Sub(_) => todo!("evaluate this shit"),
                            _ => unreachable!("bad token in double quotes"),
                        }
                    }
                    return Some(TreeItem::Word(e));
                }
                Token::SingleQuote(_) => {
                    let Some(Token::SingleQuote(s)) = self.tokens.next() else {
                        unreachable!()
                    };
                    expr.push(Expand::Literal(s))
                }
                Token::Tab => todo!(),
                Token::Glob => todo!(),
                Token::OpenBraket => todo!(),
                Token::CloseBraket => todo!(),
                Token::Pound => todo!("this is not a comment it an expansion"),
                Token::Tilde => {
                    log::warn!("doing bad expansion of any tilde to home");
                    expr.push(Expand::Home);
                }
                Token::Equal => {
                    // let a = TreeItem::Assign(expr, todo!());
                    todo!()
                }
                Token::Percent => todo!(),
                Token::Ident(_) => {
                    let Some(Token::Ident(s)) = self.tokens.next() else {
                        unreachable!()
                    };
                    expr.push(Expand::Literal(s));
                }

                Token::Space => {
                    let _ = self.tokens.next();
                    if !expr.is_empty() {
                        return Some(TreeItem::Word(expr));
                    }
                }
                Token::Sub(_) => {
                    let Some(Token::Sub(s)) = self.tokens.next() else {
                        unreachable!()
                    };
                    expr.push(Expand::Sub(s))
                }
                Token::Bang => todo!(),

                // Cant Start an expression
                Token::Huh => todo!(),
                Token::Comment => {
                    has!(TreeItem::try_from(std::mem::take(&mut expr)).ok());
                    let _ = self.tokens.next();
                    return Some(TreeItem::StatmentEnd);
                }
            }
        }

        // TODO: this can drop data when there is no newline at the end of the file
        None
    }
}

#[derive(Debug)]
pub enum TreeItem {
    Word(Vec<Expand>),
    /// Matching [`Equals`]. Can be used when assigning a variable or
    /// making an env for a command.
    /// can also sometimes be in arg position where it can just be stringifyed
    // Assign(Vec<Expand>, Vec<Expand>),
    Append,
    Redirect,
    /// `&`
    Background,
    /// `||`
    Or,
    /// `&&`
    And,
    /// `|`
    Pipe,
    /// `# *[`Token`]`
    Comment, // (String),
    /// a ';' of '\n'
    StatmentEnd,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Expand {
    Literal(String),
    Var(String),
    /// `~`
    Home,
    Brace(String, ExpandAction, Vec<Expand>),
    Sub(String),
}

/// What the brace does expansion does:
/// https://pubs.opengroup.org/onlinepubs/9699919799/utilities/V3_chap02.html#tag_18_06_02
/// If true test for unset or null, if false, only unset
/// For prefix/suffix, true for largest false for smallest
#[derive(Debug, PartialEq, Eq)]
pub enum ExpandAction {
    UseDefault(bool),
    // AssignDefault(bool),
    // IndicateError(bool),
    // UseAlternate(bool),
    // RmSmallestSuffix,
    // RmLargestSuffix,
    // RmSmallestPrefix,
    // RmLargestPrefix,
    // StringLength,
    /// ${var}
    None,
}
