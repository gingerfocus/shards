//! Execution for the Julia-style syntax parsed by `parser-julia`.

use std::{
    collections::HashMap,
    fmt,
    io::{self, Write},
    process::Command,
};

use parser_julia::{Argument, Call, Statement};

use crate::language::Language;

#[derive(Debug, PartialEq, Eq)]
pub enum Event {
    Continue(i32),
    Switch(Language),
    Exit(i32),
}

#[derive(Debug)]
pub struct JuliaError(String);

impl fmt::Display for JuliaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for JuliaError {}

fn error(message: impl ToString) -> JuliaError {
    JuliaError(format!("julia: {}", message.to_string()))
}

#[derive(Debug, Default)]
pub struct Session {
    variables: HashMap<String, String>,
}

impl Session {
    pub fn run_line(&mut self, input: &str) -> Result<Event, JuliaError> {
        let statement = parser_julia::parse_statement(input)
            .map_err(|source| JuliaError(source.to_string()))?;
        match statement {
            Statement::Assign(name, value) => {
                let value = self.evaluate(&value)?;
                self.variables.insert(name, value);
                Ok(Event::Continue(0))
            }
            Statement::Call(call) => self.call(call),
        }
    }

    fn evaluate(&self, expression: &Argument) -> Result<String, JuliaError> {
        match expression {
            Argument::String(value) | Argument::Number(value) => Ok(value.clone()),
            Argument::Identifier(name) => self
                .variables
                .get(name)
                .cloned()
                .ok_or_else(|| error(format!("undefined variable `{name}`"))),
            Argument::Call(call) if call.function == "string" => self.evaluate_all(&call.args),
            Argument::Call(call) => Err(error(format!(
                "function `{}` cannot be used as a value",
                call.function
            ))),
        }
    }

    fn evaluate_all(&self, args: &[Argument]) -> Result<String, JuliaError> {
        let mut result = String::new();
        for arg in args {
            result.push_str(&self.evaluate(arg)?);
        }
        Ok(result)
    }

    fn call(&mut self, call: Call) -> Result<Event, JuliaError> {
        match call.function.as_str() {
            "print" | "println" => {
                let value = self.evaluate_all(&call.args)?;
                let mut stdout = io::stdout().lock();
                stdout.write_all(value.as_bytes()).map_err(error)?;
                if call.function == "println" {
                    stdout.write_all(b"\n").map_err(error)?;
                }
                stdout.flush().map_err(error)?;
                Ok(Event::Continue(0))
            }
            "string" => {
                self.evaluate_all(&call.args)?;
                Ok(Event::Continue(0))
            }
            "shards" => {
                let [command, language] = call.args.as_slice() else {
                    return Err(error("expected shards(lang, sh|rust|julia)"));
                };
                if symbol(command) != Some("lang") {
                    return Err(error("expected shards(lang, sh|rust|julia)"));
                }
                let language = symbol(language)
                    .and_then(Language::parse)
                    .ok_or_else(|| error("expected shards(lang, sh|rust|julia)"))?;
                Ok(Event::Switch(language))
            }
            "exit" => {
                let status = match call.args.as_slice() {
                    [] => 0,
                    [value] => self
                        .evaluate(value)?
                        .parse()
                        .map_err(|_| error("invalid exit status"))?,
                    _ => return Err(error("exit takes at most one argument")),
                };
                Ok(Event::Exit(status))
            }
            program => {
                let args = call
                    .args
                    .iter()
                    .map(|arg| self.evaluate(arg))
                    .collect::<Result<Vec<_>, _>>()?;
                let status = Command::new(program)
                    .args(args)
                    .status()
                    .map_err(|source| error(format!("{program}: {source}")))?;
                Ok(Event::Continue(status.code().unwrap_or(1)))
            }
        }
    }
}

fn symbol(argument: &Argument) -> Option<&str> {
    match argument {
        Argument::Identifier(value) | Argument::String(value) => Some(value),
        Argument::Number(_) => None,
        Argument::Call(_) => None,
    }
}
