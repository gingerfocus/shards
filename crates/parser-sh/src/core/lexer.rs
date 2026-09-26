use std::iter::Peekable;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Word(String),
    Variable(String),
    Single(String),
    Double(Vec<Token>),
    Sub(String),
    Space,
    End,
    Pipe,
    And,
    Or,
    Amp,
    Input,
    Output,
    Append,
    Bang,
    Error(String),
}

pub struct Lexer<I: Iterator<Item = char>>(Peekable<I>);

impl<I: Iterator<Item = char>> Lexer<I> {
    pub fn new(input: I) -> Self {
        Self(input.peekable())
    }

    fn dollar(&mut self) -> Token {
        match self.0.peek().copied() {
            Some('(') => {
                self.0.next();
                let mut depth = 1;
                let mut text = String::new();
                for c in self.0.by_ref() {
                    if c == '(' {
                        depth += 1;
                    }
                    if c == ')' {
                        depth -= 1;
                        if depth == 0 {
                            return Token::Sub(text);
                        }
                    }
                    text.push(c);
                }
                Token::Error("unclosed command substitution".into())
            }
            Some('{') => {
                self.0.next();
                let mut name = String::new();
                for c in self.0.by_ref() {
                    if c == '}' {
                        return Token::Variable(name);
                    }
                    name.push(c);
                }
                Token::Error("unclosed variable expansion".into())
            }
            Some(c) if c.is_ascii_alphabetic() || c == '_' => {
                let mut name = String::new();
                while let Some(c) = self.0.peek().copied() {
                    if !c.is_ascii_alphanumeric() && c != '_' {
                        break;
                    }
                    name.push(c);
                    self.0.next();
                }
                Token::Variable(name)
            }
            Some('?') | Some('$') => Token::Variable(self.0.next().unwrap().to_string()),
            _ => Token::Word("$".into()),
        }
    }

    fn quote(&mut self, delimiter: char) -> Token {
        let mut literal = String::new();
        let mut parts = Vec::new();
        let mut closed = false;
        while let Some(c) = self.0.next() {
            if c == delimiter {
                closed = true;
                break;
            }
            if delimiter == '"' && c == '$' {
                if !literal.is_empty() {
                    parts.push(Token::Word(std::mem::take(&mut literal)));
                }
                let expansion = self.dollar();
                if matches!(expansion, Token::Error(_)) {
                    return expansion;
                }
                parts.push(expansion);
            } else if delimiter == '"' && c == '\\' {
                match self.0.peek().copied() {
                    Some('$' | '"' | '\\') => literal.push(self.0.next().unwrap()),
                    Some('\n') => {
                        self.0.next();
                    }
                    _ => literal.push(c),
                }
            } else {
                literal.push(c);
            }
        }
        if !closed {
            return Token::Error("unclosed quote".into());
        }
        if delimiter == '\'' {
            Token::Single(literal)
        } else {
            parts.push(Token::Word(literal));
            Token::Double(parts)
        }
    }
}

impl<I: Iterator<Item = char>> Iterator for Lexer<I> {
    type Item = Token;
    fn next(&mut self) -> Option<Token> {
        let c = self.0.next()?;
        Some(match c {
            ' ' | '\t' => Token::Space,
            '\n' | ';' => Token::End,
            '|' => {
                if matches!(self.0.peek(), Some('|')) {
                    self.0.next();
                    Token::Or
                } else {
                    Token::Pipe
                }
            }
            '&' => {
                if matches!(self.0.peek(), Some('&')) {
                    self.0.next();
                    Token::And
                } else {
                    Token::Amp
                }
            }
            '>' => {
                if matches!(self.0.peek(), Some('>')) {
                    self.0.next();
                    Token::Append
                } else {
                    Token::Output
                }
            }
            '<' => Token::Input,
            '!' => Token::Bang,
            '$' => self.dollar(),
            '\'' | '"' => self.quote(c),
            '#' => {
                while let Some(c) = self.0.peek().copied() {
                    if c == '\n' {
                        break;
                    }
                    self.0.next();
                }
                Token::End
            }
            '\\' => match self.0.next() {
                Some('\n') => return self.next(),
                Some(c) => Token::Word(c.to_string()),
                None => Token::Word("\\".into()),
            },
            c => {
                let mut text = c.to_string();
                while let Some(c) = self.0.peek().copied() {
                    if matches!(
                        c,
                        ' ' | '\t' | '\n' | ';' | '|' | '&' | '>' | '<' | '$' | '\'' | '"' | '\\'
                    ) {
                        break;
                    }
                    text.push(c);
                    self.0.next();
                }
                Token::Word(text)
            }
        })
    }
}
