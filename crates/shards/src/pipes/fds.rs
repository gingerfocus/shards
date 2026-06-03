use libc::O_CLOEXEC;
use std::ffi::CStr;
use std::io::{Read, Write};
use std::os::fd::OwnedFd;
use std::os::unix::io::{AsRawFd, FromRawFd, RawFd};

pub const PIPE_ERROR: &'static str = "An error occurred while setting up pipe";

/// The first "high fd", which is considered outside the range of valid user-specified redirections
/// (like >&5).
pub const FIRST_HIGH_FD: RawFd = 10;

/// A sentinel value indicating no timeout.
pub const NO_TIMEOUT: u64 = u64::MAX;

// /// A helper type for managing and automatically closing a file descriptor
// pub struct AutoCloseFd {
//     fd: RawFd,
// }
//
// impl Read for AutoCloseFd {
//     fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
//         unsafe {
//             match libc::read(self.as_raw_fd(), buf.as_mut_ptr() as *mut _, buf.len()) {
//                 -1 => Err(std::io::Error::last_os_error()),
//                 bytes => Ok(bytes as usize),
//             }
//         }
//     }
// }
//
// impl Write for AutoCloseFd {
//     fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
//         match unsafe { libc::write(self.as_raw_fd(), buf.as_ptr() as *const _, buf.len()) } {
//             -1 => Err(std::io::Error::last_os_error()),
//             bytes => Ok(bytes as usize),
//         }
//     }
//
//     fn flush(&mut self) -> std::io::Result<()> {
//         // We don't buffer anything so this is a no-op.
//         Ok(())
//     }
// }
//
// impl AutoCloseFd {
//     // Closes the fd if not already closed.
//     pub fn close(&mut self) {
//         if self.fd != -1 {
//             _ = unistd::close(self.fd);
//             self.fd = -1;
//         }
//     }
//
//     // Returns the fd.
//     pub fn fd(&self) -> RawFd {
//         self.fd
//     }
//
//     // Returns the fd, transferring ownership to the caller.
//     pub fn acquire(&mut self) -> RawFd {
//         let temp = self.fd;
//         self.fd = -1;
//         temp
//     }
//
//     // Resets to a new fd, taking ownership.
//     pub fn reset(&mut self, new_fd: RawFd) {
//         if new_fd == self.fd {
//             return;
//         }
//         self.close();
//         self.fd = new_fd;
//     }
//
//     // Returns if this has a valid fd.
//     pub fn is_valid(&self) -> bool {
//         self.fd >= 0
//     }
//
//     // Create a new AutoCloseFd instance taking ownership of the passed fd
//     pub fn new(fd: RawFd) -> Self {
//         AutoCloseFd { fd }
//     }
//
//     // Create a new AutoCloseFd without an open fd
//     pub fn empty() -> Self {
//         AutoCloseFd { fd: -1 }
//     }
// }
//
// impl FromRawFd for AutoCloseFd {
//     unsafe fn from_raw_fd(fd: RawFd) -> Self {
//         AutoCloseFd { fd }
//     }
// }
//
// impl AsRawFd for AutoCloseFd {
//     fn as_raw_fd(&self) -> RawFd {
//         self.fd()
//     }
// }
//
// impl Default for AutoCloseFd {
//     fn default() -> AutoCloseFd {
//         AutoCloseFd::empty()
//     }
// }
//
// impl Drop for AutoCloseFd {
//     fn drop(&mut self) {
//         self.close()
//     }
// }

/// Helper type returned from make_autoclose_pipes.
#[derive(Debug)]
pub struct AutoClosePipes {
    /// Read end of the pipe.
    pub read: OwnedFd,

    /// Write end of the pipe.
    pub write: OwnedFd,
}

// /// Construct a pair of connected pipes, set to close-on-exec.
// /// \return None on fd exhaustion.
// pub fn make_autoclose_pipes() -> Option<AutoClosePipes> {

// int pipes[2] = {-1, -1};
//
// bool already_cloexec = false;
// if (pipe2(pipes, O_CLOEXEC) < 0) {
//     FLOGF(warning, PIPE_ERROR);
//     wperror(L"pipe2");
//     return none();
// }
// already_cloexec = true;
//
// autoclose_fd_t read_end{pipes[0]};
// autoclose_fd_t write_end{pipes[1]};
//
// // Ensure our fds are out of the user range.
// read_end = heightenize_fd(std::move(read_end), already_cloexec);
// if (!read_end.valid()) return none();
//
// write_end = heightenize_fd(std::move(write_end), already_cloexec);
// if (!write_end.valid()) return none();

//     let readp = AutoCloseFd::new(pipes.read);
//     let writep = AutoCloseFd::new(pipes.write);
//     if !readp.is_valid() || !writep.is_valid() {
//         None
//     } else {
//         Some(AutoClosePipes {
//             read: readp,
//             write: writep,
//         })
//     }
// }

/// creates a file that is told to close
pub fn open_cloexec(path: &CStr, flags: i32, mode: libc::c_int) -> RawFd {
    unsafe { libc::open(path.as_ptr(), flags | O_CLOEXEC, mode) }
}

/// Close a file descriptor \p fd, retrying on EINTR.
pub fn exec_close(fd: RawFd) {
    assert!(fd >= 0, "Invalid fd");
    while unsafe { libc::close(fd) } == -1 {
        if unsafe { libc::__errno_location().read() } != libc::EINTR {
            // perror("close");
            break;
        }
    }
}
