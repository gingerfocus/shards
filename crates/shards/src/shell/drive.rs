use super::{
    streams::{Fd, Streams},
    task::Task,
    Session, ShellError,
};
use parser_sh::command::{Cmd, Redirection, SimpleCmd};
use std::{fs::OpenOptions, os::fd::FromRawFd, process::Command};

fn error(message: impl ToString) -> ShellError {
    ShellError(message.to_string())
}

pub fn run_command(cmd: Cmd, streams: Streams, state: &mut Session) -> Result<i32, ShellError> {
    match cmd {
        Cmd::And(left, right) => {
            let status = run_command(*left, Streams::default(), state)?;
            state.previous_status = status;
            if state.exit {
                return Ok(status);
            }
            if status == 0 {
                run_command(*right, streams, state)
            } else {
                Ok(status)
            }
        }
        Cmd::Or(left, right) => {
            let status = run_command(*left, Streams::default(), state)?;
            state.previous_status = status;
            if state.exit {
                return Ok(status);
            }
            if status != 0 {
                run_command(*right, streams, state)
            } else {
                Ok(status)
            }
        }
        Cmd::Not(inner) => Ok(if run_command(*inner, streams, state)? == 0 {
            1
        } else {
            0
        }),
        Cmd::Empty => Ok(0),
        other => {
            let tasks = spawn_command(other, streams, state)?;
            let mut status = 0;
            for task in tasks {
                status = task.wait().map_err(error)?;
            }
            Ok(status)
        }
    }
}

fn spawn_command(cmd: Cmd, streams: Streams, state: &mut Session) -> Result<Vec<Task>, ShellError> {
    match cmd {
        Cmd::Simple(simple) => Ok(vec![spawn_simple(simple, streams, state)?]),
        Cmd::Pipeline(left, right) => {
            let mut pipes = [0; 2];
            if unsafe { libc::pipe(pipes.as_mut_ptr()) } != 0 {
                return Err(error(std::io::Error::last_os_error()));
            }
            for fd in pipes {
                if unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) } == -1 {
                    unsafe {
                        libc::close(pipes[0]);
                        libc::close(pipes[1]);
                    }
                    return Err(error(std::io::Error::last_os_error()));
                }
            }
            let left_streams = Streams {
                stdin: streams.stdin,
                stdout: unsafe { Fd::from_raw_fd(pipes[1]) },
                stderr: Fd::Inherit,
            };
            let right_streams = Streams {
                stdin: unsafe { Fd::from_raw_fd(pipes[0]) },
                stdout: streams.stdout,
                stderr: streams.stderr,
            };
            let mut tasks = spawn_command(*left, left_streams, state)?;
            tasks.extend(spawn_command(*right, right_streams, state)?);
            Ok(tasks)
        }
        _ => Err(error(
            "conditional commands cannot be used as pipeline stages",
        )),
    }
}

fn spawn_simple(
    simple: SimpleCmd,
    mut streams: Streams,
    state: &mut Session,
) -> Result<Task, ShellError> {
    let mut assignments = Vec::new();
    for (name, word) in simple.assignments {
        assignments.push((name, state.expand(&word)?));
    }
    for redirection in simple.redirections {
        match redirection {
            Redirection::Input(path) => {
                streams.stdin = Fd::Piped(
                    OpenOptions::new()
                        .read(true)
                        .open(state.expand(&path)?)
                        .map_err(error)?
                        .into(),
                );
            }
            Redirection::Output(path) => {
                streams.stdout = Fd::Piped(
                    OpenOptions::new()
                        .create(true)
                        .write(true)
                        .truncate(true)
                        .open(state.expand(&path)?)
                        .map_err(error)?
                        .into(),
                );
            }
            Redirection::Append(path) => {
                streams.stdout = Fd::Piped(
                    OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(state.expand(&path)?)
                        .map_err(error)?
                        .into(),
                );
            }
        }
    }
    let Some(name) = simple.cmd else {
        for (key, value) in assignments {
            state.removed.remove(&key);
            state.variables.insert(key, value);
        }
        return Ok(Task::Builtin(0));
    };
    let name = state.expand(&name)?;
    let args = simple
        .args
        .iter()
        .map(|word| state.expand(word))
        .collect::<Result<Vec<_>, _>>()?;
    let status = match name.as_str() {
        ":" | "true" => Some(0),
        "false" => Some(1),
        "exit" => {
            state.exit = true;
            Some(
                args.first()
                    .and_then(|s| s.parse::<i32>().ok())
                    .unwrap_or(state.previous_status),
            )
        }
        "cd" => {
            let path = args.first().map(String::as_str).unwrap_or(state.home());
            Some(match std::env::set_current_dir(path) {
                Ok(()) => 0,
                Err(e) => {
                    eprintln!("cd: {e}");
                    1
                }
            })
        }
        "export" => {
            for arg in &args {
                if let Some((key, value)) = arg.split_once('=') {
                    state.variables.insert(key.into(), value.into());
                    state.removed.remove(key);
                    state.exported.insert(key.into());
                } else {
                    state.removed.remove(arg);
                    state.exported.insert(arg.clone());
                }
            }
            Some(0)
        }
        "unset" => {
            for arg in &args {
                state.variables.remove(arg);
                state.exported.remove(arg);
                state.removed.insert(arg.clone());
            }
            Some(0)
        }
        "shards" => Some(match args.as_slice() {
            [command, language] if command == "lang" => {
                if let Some(language) = crate::language::Language::parse(language) {
                    state.requested_language = Some(language);
                    0
                } else {
                    eprintln!("shards: expected shards lang rust, sh, or julia");
                    2
                }
            }
            _ => {
                eprintln!("shards: expected shards lang rust, sh, or julia");
                2
            }
        }),
        _ => None,
    };
    if let Some(status) = status {
        return Ok(Task::Builtin(status));
    }
    let mut process = Command::new(&name);
    process
        .args(args)
        .stdin(streams.stdin)
        .stdout(streams.stdout)
        .stderr(streams.stderr);
    for key in &state.exported {
        if let Some(value) = state.variables.get(key) {
            process.env(key, value);
        }
    }
    for key in &state.removed {
        process.env_remove(key);
    }
    process.envs(assignments);
    match process.spawn() {
        Ok(child) => Ok(Task::System(child)),
        Err(e) => {
            eprintln!("{name}: {e}");
            Ok(Task::Builtin(if e.kind() == std::io::ErrorKind::NotFound {
                127
            } else {
                126
            }))
        }
    }
}
