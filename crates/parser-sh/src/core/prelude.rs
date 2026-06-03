#![allow(unused_imports)]

pub use resu::{Context, Report, Result, ResultExt};
pub use std::{fmt, io};

pub use crate::lexer::Token;

/// The inverse of the `?` operator. If you have something then return it
/// otherwise just keep looking I guess.
#[macro_export]
macro_rules! has {
    ($option:expr) => {{
        let this: Option<_> = $option;
        match this {
            Some(value) => return Some(value),
            None => {}
        }
    }};
}

pub(crate) use crate::has;
