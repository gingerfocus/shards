use crate::parse::Fd;
use crate::prelude::*;

use crate::{
    parse::{Cmd, SimpleCmd, Streams},
    task::Task,
};

use std::os::fd::FromRawFd;

#[derive(Debug)]
pub enum DriverError {
    Spawn,
    Pipe,
}

impl fmt::Display for DriverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DriverError::Spawn => f.write_str("failed to spawn command"),
            DriverError::Pipe => f.write_str("failed to open pipe"),
        }
    }
}
impl Context for DriverError {}

/// Main entry point for the eval process. Takes a command and returns a list
/// of handles to tasks that are asociated with that command.
pub fn run_command(
    cmd: Cmd,
    streams: Streams,
    state: &mut ShellState,
) -> Result<Vec<Task>, DriverError> {
    log::info!("cmd is: {:?}", cmd);
    use self::builtins::ShellBuiltin;

    match cmd {
        Cmd::Simple(SimpleCmd { cmd, args, env }) => match cmd.as_str() {
            "exit" => {
                log::info!("running exit command");
                // TODO: fuse the stdin fd to stdout so anything in this
                // pipe line ignores this call.
                // This should be done for all builtins.
                Ok(vec![Task::Builtin(builtins::Exit::run(&args, state))])
            }
            "cd" => Ok(vec![Task::Builtin(builtins::Cd::run(&args, state))]),
            cmd => {
                log::info!("Running command: [{}, {:?}]", cmd, args);

                let child = std::process::Command::new(cmd)
                    .args(args)
                    .envs(env)
                    .stdout(streams.stdout)
                    .stdin(streams.stdin)
                    .stderr(streams.stderr)
                    .spawn()
                    .change_context(DriverError::Spawn)?;

                // for handling things like ^C and ^Z
                // let stdin = child.stdin.take().unwrap();

                Ok(vec![Task::System(child)])
            }
        },
        Cmd::Pipeline(c, d) => {
            let mut pipes = [0; 2];

            if unsafe { libc::pipe2(pipes.as_mut_ptr(), libc::O_CLOEXEC) } != 0 {
                let err = Err(std::io::Error::last_os_error());
                return err.change_context(DriverError::Pipe);
            }

            log::info!("made pipes: {:?}", pipes);

            let mut sc = streams;
            sc.stdout = unsafe { Fd::from_raw_fd(pipes[1]) };

            let sd = Streams {
                stdin: unsafe { Fd::from_raw_fd(pipes[0]) },
                ..Default::default()
            };

            let mut a = run_command(*c, sc, state)?;
            let b = run_command(*d, sd, state)?;

            a.extend(b);
            Ok(a)
        }
        Cmd::And(_, _) => todo!(),
        Cmd::Or(_, _) => todo!(),
        Cmd::Not(_) => todo!(),
        Cmd::Empty => Ok(vec![]),
    }
}

mod builtins {
    use crate::shell::ShellState;

    pub(crate) trait ShellBuiltin {
        fn run(args: &[String], state: &mut ShellState) -> i32;
    }

    pub struct Exit;
    impl ShellBuiltin for Exit {
        fn run(args: &[String], state: &mut ShellState) -> i32 {
            let _ = args;
            eprintln!("exit");

            state.exit = true;
            0
        }
    }

    pub struct Cd;
    impl ShellBuiltin for Cd {
        fn run(args: &[String], state: &mut ShellState) -> i32 {
            let _ = state;
            let dir = args
                .iter()
                .next()
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| state.home().to_owned());

            // libc::chdir(dir)
            // nix::unistd::chdir(&dir);
            std::env::set_current_dir(dir).unwrap();
            0
        }
    }
}
