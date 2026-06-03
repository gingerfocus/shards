#![feature(let_chains, vec_into_raw_parts)]

use crate::{lexer::Lexer, walker::Walker};

pub mod lexer;
mod prelude;
mod util;
pub mod walker;

pub fn parse(input: &str) -> impl Iterator<Item = self::walker::TreeItem> + '_ {
    let a = Lexer::new(input.chars());
    Walker::new(a)
}
