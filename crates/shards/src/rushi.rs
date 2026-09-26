//! Rushi command execution. Syntax is owned by `parser-rust`.

use std::{fmt, io, process::Command};

pub use parser_rust::{parse_call, Call, ParseError};

#[derive(Debug)]
pub enum RushiError {
    Parse(ParseError),
    Spawn { program: String, source: io::Error },
}

impl fmt::Display for RushiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(error) => error.fmt(f),
            Self::Spawn { program, source } => write!(f, "rushi: {program}: {source}"),
        }
    }
}

impl std::error::Error for RushiError {}

pub fn run(input: &str) -> Result<i32, RushiError> {
    run_call(parse_call(input).map_err(RushiError::Parse)?)
}

/// Run an external command with inherited stdin, stdout, and stderr.
pub fn run_call(call: Call) -> Result<i32, RushiError> {
    let status = Command::new(&call.program)
        .args(&call.args)
        .status()
        .map_err(|source| RushiError::Spawn {
            program: call.program,
            source,
        })?;
    Ok(status.code().unwrap_or(1))
}
