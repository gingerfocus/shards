//! Shell execution within the Shards frontend. Syntax comes from `parser-sh`.

mod drive;
mod streams;
mod task;

use std::{
    collections::{HashMap, HashSet},
    fmt,
    io::{Read, Seek, SeekFrom},
    sync::atomic::{AtomicU64, Ordering},
};

use parser_sh::{
    command::Parser,
    lexer::Lexer,
    walker::{Expand, Word},
};

use crate::language::Language;

#[derive(Debug)]
pub enum Event {
    Continue(i32),
    Switch(Language),
    Exit(i32),
}

#[derive(Debug)]
pub struct ShellError(String);

impl fmt::Display for ShellError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ShellError {}

#[derive(Debug, Clone)]
pub struct Session {
    pub(crate) exit: bool,
    pub(crate) requested_language: Option<Language>,
    pub(crate) previous_status: i32,
    home: String,
    variables: HashMap<String, String>,
    exported: HashSet<String>,
    removed: HashSet<String>,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            exit: false,
            requested_language: None,
            previous_status: 0,
            home: std::env::var("HOME").unwrap_or_default(),
            variables: HashMap::new(),
            exported: HashSet::new(),
            removed: HashSet::new(),
        }
    }
}

impl Session {
    pub(crate) fn home(&self) -> &str {
        &self.home
    }

    fn expand(&self, word: &Word) -> Result<String, ShellError> {
        let mut value = String::new();
        for part in word {
            match part {
                Expand::Literal(text) => value.push_str(text),
                Expand::Home => value.push_str(&self.home),
                Expand::Var(name) => match name.as_str() {
                    "?" => value.push_str(&self.previous_status.to_string()),
                    "$" => value.push_str(&std::process::id().to_string()),
                    _ => {
                        if let Some(stored) = self.variables.get(name) {
                            value.push_str(stored);
                        } else if !self.removed.contains(name) {
                            if let Ok(inherited) = std::env::var(name) {
                                value.push_str(&inherited);
                            }
                        }
                    }
                },
                Expand::Sub(command) => value.push_str(&self.substitute(command)?),
            }
        }
        Ok(value)
    }

    fn substitute(&self, input: &str) -> Result<String, ShellError> {
        static NEXT_SUBSTITUTION: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "shards-substitution-{}-{}",
            std::process::id(),
            NEXT_SUBSTITUTION.fetch_add(1, Ordering::Relaxed)
        ));
        let mut output = std::fs::OpenOptions::new()
            .write(true)
            .read(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| ShellError(e.to_string()))?;
        let result = (|| {
            let mut child = self.clone();
            let mut parser = Parser::new(Lexer::new(input.chars()));
            while let Some(command) = parser.next() {
                let command = command.map_err(|e| ShellError(e.to_string()))?;
                let stream = streams::Streams {
                    stdout: streams::Fd::Piped(
                        output
                            .try_clone()
                            .map_err(|e| ShellError(e.to_string()))?
                            .into(),
                    ),
                    ..Default::default()
                };
                child.previous_status = drive::run_command(command, stream, &mut child)?;
                if child.exit {
                    break;
                }
            }
            output
                .seek(SeekFrom::Start(0))
                .map_err(|e| ShellError(e.to_string()))?;
            let mut text = String::new();
            output
                .read_to_string(&mut text)
                .map_err(|e| ShellError(e.to_string()))?;
            Ok(text.trim_end_matches('\n').to_owned())
        })();
        let _ = std::fs::remove_file(path);
        result
    }

    pub fn run_line(&mut self, input: &str) -> Result<Event, ShellError> {
        let mut parser = Parser::new(Lexer::new(input.chars()));
        while let Some(command) = parser.next() {
            let command = command.map_err(|error| ShellError(error.to_string()))?;
            self.previous_status = drive::run_command(command, streams::Streams::default(), self)?;

            if let Some(language) = self.requested_language.take() {
                return Ok(Event::Switch(language));
            }
            if self.exit {
                return Ok(Event::Exit(self.previous_status));
            }
        }
        Ok(Event::Continue(self.previous_status))
    }
}

#[cfg(test)]
mod tests {
    use super::{Event, Session};

    #[test]
    fn assignments_and_short_circuiting_are_deferred() {
        let mut shell = Session::default();
        assert!(matches!(
            shell.run_line("A=first; false && A=wrong || A=$?"),
            Ok(Event::Continue(0))
        ));
        assert_eq!(shell.variables.get("A").map(String::as_str), Some("1"));
        shell.run_line("true || A=wrong; true && A=right").unwrap();
        assert_eq!(shell.variables.get("A").map(String::as_str), Some("right"));
    }

    #[test]
    fn command_arguments_and_redirections_expand_in_executor() {
        let mut shell = Session::default();
        let path = std::env::temp_dir().join(format!("shards-shell-test-{}", std::process::id()));
        let script = format!(
            "VALUE=hello; printf %s \"$VALUE world\" > {}",
            path.display()
        );
        shell.run_line(&script).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello world");
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn assignment_prefix_is_passed_to_external_command() {
        let mut shell = Session::default();
        let path =
            std::env::temp_dir().join(format!("shards-shell-env-test-{}", std::process::id()));
        let script = format!("VALUE=hello printenv VALUE > {}", path.display());
        shell.run_line(&script).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello\n");
        assert!(!shell.variables.contains_key("VALUE"));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn pipeline_and_substitution_use_current_shell_state() {
        let mut shell = Session::default();
        let path =
            std::env::temp_dir().join(format!("shards-shell-pipe-test-{}", std::process::id()));
        let script = format!(
            "VALUE=hello; printf %s $(printf %s $VALUE) | tr a-z A-Z > {}",
            path.display()
        );
        shell.run_line(&script).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "HELLO");
        std::fs::remove_file(path).unwrap();
        shell
            .run_line("INNER=outer; printf %s $(INNER=inner; printf %s $INNER) > /dev/null")
            .unwrap();
        assert_eq!(
            shell.variables.get("INNER").map(String::as_str),
            Some("outer")
        );
    }
}
