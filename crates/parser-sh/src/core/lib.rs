use crate::{lexer::Lexer, walker::Walker};

pub mod lexer;
pub mod command;
mod prelude;
mod util;
pub mod walker;

pub use util::StaticMap;

pub fn parse(input: &str) -> impl Iterator<Item = self::walker::TreeItem> + '_ {
    let a = Lexer::new(input.chars());
    Walker::new(a)
}
