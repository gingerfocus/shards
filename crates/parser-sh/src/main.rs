#![feature(let_chains, vec_into_raw_parts)]

pub mod drive;
pub mod parse;
pub mod prelude;
pub mod shell;
pub mod task;
pub mod util;
pub mod walker;

use crate::prelude::*;

use std::{fs, process::ExitCode};

use rush_core::lexer::Lexer;
use std::{env, path::PathBuf};

fn main() -> ExitCode {
    use std::io::Read;
    let exit = match ShellMode::get() {
        ShellMode::Run(path) => {
            log::info!("running file: {:?}", path);

            let data = fs::read_to_string(path).unwrap();
            let input = data.chars();

            Shell::sourced(Lexer::new(input)).run(false)
        }
        ShellMode::Eval => {
            log::warn!("the implementation for reading from stdin is shitty.");
            let mut buf = String::new();
            // let a = AtomicBuffer::new();

            std::io::stdin().read_to_string(&mut buf).unwrap();

            let input = buf.chars();

            Shell::sourced(Lexer::new(input)).run(false)
        }
        ShellMode::Interactive => {
            log::info!("running interactive session");

            Shell::interactive().run(true)
        }
        ShellMode::Login => {
            log::info!("running login session");

            // Logins shells read /etc/profile and ~/.profile
            // and then function as normal interactive shells

            Shell::login().run(true)
        }
        ShellMode::Command(cmd) => {
            log::info!("running command: {:?}", cmd);

            let input = cmd.chars().peekable();

            Shell::sourced(Lexer::new(input)).run(false)
        }
    };

    match exit {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprint!("{:?}", e);
            // TODO: more complext exit codes
            ExitCode::FAILURE
        }
    }
}

pub enum ShellMode {
    Run(PathBuf),
    /// run code from stdin
    Eval,
    Interactive,
    Login,
    Command(String),
}

impl ShellMode {
    pub fn get() -> Self {
        use clap::Parser;
        Self::from(Args::parse())
    }
}

impl From<Args> for ShellMode {
    fn from(value: Args) -> Self {
        if ({ env::args().next().is_some_and(|name| name.starts_with('-')) } || value.login) && {
            // TODO: check if there is already a login shell
            true
        } {
            // if (this was started with the login shell prefix or it is
            // explicitally a login shell) and there is not already a login shell
            Self::Login
        } else if value.interactive {
            // if we are said to be interactive the we are
            Self::Interactive
        } else if let Some(cmd) = value.command {
            // if we have an input command then run that
            Self::Command(cmd)
        } else if let Some(file) = value.file {
            // if we have and input file then run that
            Self::Run(PathBuf::from(file))
        } else if crossterm::tty::IsTty::is_tty(&std::io::stdin()) {
            // if there is no input file and stdin is a tty then we are running
            // iteractivally
            Self::Interactive
        } else {
            // if there is no input file and stdin is not a tty then we should
            // run code we receive. This likely means we are in a pipeline
            Self::Eval
        }
    }
}

/// A shell to fixx the world. Kinda... not.
#[derive(Debug, clap::Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Command to run.
    #[arg(short, long)]
    command: Option<String>,

    /// Is this a login shell.
    #[arg(short, long)]
    login: bool,

    /// Is this an interactive shell.
    #[arg(short, long)]
    interactive: bool,

    /// File to execute. Pass `-` to read stdin, which is default
    file: Option<String>,
}
