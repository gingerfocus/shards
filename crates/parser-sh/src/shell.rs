use crate::drive::run_command;
use crate::parse::{Parser, Prompter};
use crate::prelude::*;

// use nix::unistd::Uid;
// use os_pipe::{dup_stderr, dup_stdin, dup_stdout, PipeReader, PipeWriter};
// use std::env;
// use std::fs::{File, OpenOptions};
// use std::io::{self, BufRead, BufReader, Write};
// use std::process::{self, Stdio};
// use crate::util::StaticMap;

use crate::parse::{Cmd, CmdError, Streams};
// use crate::util::AtomicSlice;
// use crate::walker::TreeItem;

#[derive(Debug)]
pub struct ShellState {
    pub exit: bool,
    home: String,
    /// The most recent exit status of a command
    prev: i32,
    // __cache: StaticMap<String, String>,
    hist: Vec<String>,
}

impl Default for ShellState {
    fn default() -> Self {
        Self {
            exit: false,
            prev: 0,
            home: std::env::var("HOME").unwrap(),
            // __cache: StaticMap::new()
            hist: Vec::new(),
        }
    }
}

impl ShellState {
    pub fn home(&self) -> &str {
        &self.home
    }

    pub fn get_env_exact(&self, key: &str) -> Option<String> {
        std::env::var(key).ok()
    }

    /// Gets an variable from the current scope. This matches on the smallest
    /// substring first. it returns the rest of the unmatched slice as the
    /// second. if the variable doesn't exist it will return ("", key).
    pub fn get_env<'a>(&'a self, key: &'a str) -> (String, &'a str) {
        for i in 0..key.len() {
            log::info!("trying key: {}", &key[0..i + 1]);
            if let Some(v) = self.get_env_exact(&key[0..i + 1]) {
                return (v, &key[i + 1..]);
            }
        }
        (String::new(), key)
    }

    pub fn get_history(&self, index: usize) -> Option<&str> {
        let leng = self.hist.len();
        if index > leng {
            None
        } else {
            self.hist.get(leng - index).map(|s| s.as_str())
        }
    }

    /// Adds a command to this shells history
    pub fn add_history(&mut self, item: impl Into<String>) {
        self.hist.push(item.into())
    }
}

#[derive(Debug)]
pub enum ShellError {
    Parse,
    Spawn,
    Task,
}

impl fmt::Display for ShellError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ShellError::Parse => f.write_str("failed to parse input"),
            ShellError::Spawn => f.write_str("failed spawn thing"),
            ShellError::Task => f.write_str("background task had a problem"),
        }
    }
}
impl Context for ShellError {}

pub(crate) enum CommandSource<I>
where
    I: Iterator<Item = Token>,
{
    Interactive(Prompter),
    NonInteractively(Parser<I>),
}

impl<I> CommandSource<I>
where
    I: Iterator<Item = Token>,
{
    pub fn next(&mut self, state: &mut ShellState) -> Option<Result<Cmd, CmdError>> {
        match self {
            CommandSource::Interactive(i) => i.next(state),
            CommandSource::NonInteractively(s) => s.next(state),
        }
    }
}

/// The shell works through a cycle of getting some tokens. Collecting them
/// into a Command. Then running it.
pub struct Shell<I>
where
    I: Iterator<Item = Token>,
{
    cmmds: CommandSource<I>,
    state: ShellState,
    // taskp: Vec<Task>,
}

impl Shell<std::iter::Empty<Token>> {
    pub fn interactive() -> Shell<std::iter::Empty<Token>> {
        Shell {
            cmmds: CommandSource::Interactive(Prompter::default()),
            state: ShellState::default(),
        }
    }

    pub fn login() -> Shell<std::iter::Empty<Token>> {
        panic!("I'm not ready for this shit yet. Try next year.")
    }
}

// Sudo code for how this could be inplemented
// fn poll(//
//     // reader: AtomicSlice<u8>
//     // lexxer: todo!()
//     // walker: Walker
//     // parser: Parser
// ) -> Option<()> {
//     fn get_next_charr(reader: AtomicSlice<u8>) -> Option<Option<char>> {
//         todo!()
//     }
//
//     fn get_next_token() -> Option<Option<Token>> {
//         todo!()
//     }
//
//     fn get_next_astre() -> Option<Option<TreeItem>> {
//         todo!()
//     }
//
//     fn get_next_comnd() -> Option<Option<Cmd>> {
//         todo!()
//     }
//
//     // let c = get_next_charr(reader)?;
//     // let t = get_next_token(lexxer, c)?;
//     // let a = get_next_astre(walker, t)?;
//     // let c = get_next_comnd(parser, t)?;
//     // let e = run_comnd(c).unwrap();
//     todo!()
// }

impl<I> Shell<I>
where
    I: Iterator<Item = Token>,
{
    /// Contructs a new shell in command mode. The given input will be run until
    /// exhaustion then the shell will exit.
    ///
    /// This argument is ussaually a [`Lexer`] but is generic so it can take
    /// many types of lexers.
    pub fn sourced(tokens: I) -> Shell<I> {
        Shell {
            cmmds: CommandSource::NonInteractively(Parser::new(tokens)),
            state: ShellState::default(),
        }
    }

    /// Runs the main event loop for this shell. Gets commands from the its
    /// stream and evaluates them.
    ///
    /// # Errors
    /// When this function shell is interactive (almost) all error are treated
    /// non-fatally. A message is printed to the user and the shell continues
    /// as normal. When running non-interactively the command this will return
    /// and error.
    ///
    /// ## Login
    /// When this is ran as a login shell it will refuse to panic or error.
    /// The shell will attempt to restart itself whenever some thing bad
    /// happens.
    pub fn run(self, live: bool) -> Result<(), ShellError> {
        self.run_with_output(live, Streams::default()).map(|_| {})
    }

    pub fn run_with_output(
        mut self,
        live: bool,
        _streams: Streams,
    ) -> Result<std::process::Output, ShellError> {
        while let Some(res) = self.cmmds.next(&mut self.state) {
            let cmd = {
                match (res, live) {
                    (Ok(cmd), _) => cmd,
                    (Err(e), true) => {
                        eprintln!("{:?}", e);
                        continue;
                    }
                    (Err(e), false) => return Err(e.change_context(ShellError::Parse)),
                }
            };

            let res = run_command(cmd, Streams::default(), &mut self.state);

            let handles = match (res, live) {
                (Ok(a), _) => a,
                (Err(e), true) => {
                    eprintln!("{:?}", e);
                    continue;
                }
                (Err(e), false) => return Err(e.change_context(ShellError::Spawn)),
            };

            for h in handles {
                // loop {}
                // let _ = h.poll();
                self.state.prev = h.wait().change_context(ShellError::Spawn)?;
                // .attach("task had internal error")?;
            }

            if self.state.exit {
                log::info!("exiting beacuse flag was set");
                break;
            }
        }
        log::info!("no more commands.");

        // Ok(std::process::Output {
        //     status: todo!(),
        //     stdout: todo!(),
        //     stderr: todo!(),
        // })
        todo!()
    }

    // pub fn next_prompt(&mut self, prompt: &str) -> Option<String> {
    //     if self.is_interactive() {
    //         print!("{}", prompt);
    //         io::stdout().flush().unwrap();
    //     }
    //     self.lines.next()
    // }

    // // Not super satisfied with this as it is returning a String when it could be a
    // // reference, but this also allows handling stuff like $@ right here, as that would need to be
    // // stitched together here and thus it would own the value.
    // // Also, env:: calls in Rust seem to return ownership rather than references, which is
    // // nasty.
    // pub fn get_var(&self, key: &str) -> Option<String> {
    //     if let Ok(num) = key.parse::<u32>() {
    //         if num == 0 {
    //             Some(self.name.clone())
    //         } else {
    //             self.get_pos(num).map(String::from)
    //         }
    //     } else {
    //         match key {
    //             "@" | "*" => Some(self.positional.join(" ")), // these are technically more complicated but it works for now
    //             "#" => Some(self.positional.len().to_string()),
    //             "$" => Some(process::id().to_string()),
    //             _ => self
    //                 .vars
    //                 .get(key)
    //                 .map_or(env::var(key).ok(), |s| Some(String::from(s))),
    //         }
    //     }
    // }
    //
    // pub fn set_var(&mut self, key: String, val: String) {
    //     if env::var_os(&key).is_some() {
    //         env::set_var(key, val);
    //     } else {
    //         self.vars.insert(key, val);
    //     }
    // }
}
