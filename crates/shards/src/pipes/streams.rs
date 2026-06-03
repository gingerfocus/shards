use std::os::fd::{OwnedFd, RawFd};

pub struct Streams {
    pub out: OwnedFd,
    pub err: OwnedFd,
    pub out_is_redirected: bool,
    pub err_is_redirected: bool,
}

impl Streams {
    pub fn new() -> Self {
        todo!()
    }

    /// gets info about the current terminal output
    pub fn stdin_fd(&self) -> Option<RawFd> {
        todo!()
    }

    /// Append a &str or String.
    pub fn append<Str: AsRef<str>>(&mut self, s: Str) -> bool {
        todo!()
    }
}
