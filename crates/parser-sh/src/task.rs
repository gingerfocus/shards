use std::fmt::Display;

use crate::prelude::*;

#[derive(Debug)]
#[must_use]
pub enum Task {
    System(std::process::Child),
    Builtin(i32),
}

#[derive(Debug)]
pub enum TaskError {
    Wait,
}
impl Display for TaskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskError::Wait => f.write_str("failed to wait for process"),
        }
    }
}
impl Context for TaskError {}

impl Task {
    pub fn wait(self) -> Result<i32, TaskError> {
        match self {
            Task::System(mut c) => {
                let pid = c.id();
                // c.try_wait()
                let code = c
                    .wait()
                    .change_context(TaskError::Wait)?
                    .code()
                    .unwrap_or(-127);
                log::info!("process ({}): exit {}", pid, code);
                Ok(code)
            }
            Task::Builtin(code) => Ok(code),
        }
    }

    pub fn poll(&mut self) -> std::task::Poll<Result<i32, TaskError>> {
        match self {
            Task::System(c) => {
                let _ = c.try_wait();
                todo!()
            }
            Task::Builtin(_) => todo!(),
        }
    }
}
