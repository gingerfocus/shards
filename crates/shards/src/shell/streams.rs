use std::os::fd::{FromRawFd, OwnedFd, RawFd};
use std::process::Stdio;

#[derive(Debug, Default)]
pub enum Fd {
    #[default]
    Inherit,
    Piped(OwnedFd),
}

impl From<Fd> for Stdio {
    fn from(value: Fd) -> Self {
        match value {
            Fd::Inherit => Stdio::inherit(),
            Fd::Piped(fd) => Stdio::from(fd),
        }
    }
}

impl FromRawFd for Fd {
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        Self::Piped(unsafe { OwnedFd::from_raw_fd(fd) })
    }
}

#[derive(Debug, Default)]
pub struct Streams {
    pub stdin: Fd,
    pub stdout: Fd,
    pub stderr: Fd,
}
