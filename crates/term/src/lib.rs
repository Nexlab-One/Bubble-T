//! Cross-platform raw mode, TTY access, and terminal size.
//!
//! This crate is the Rust port of [`charmbracelet/x/term`]. It provides the
//! platform abstraction the runtime needs to drive a terminal directly: entering and
//! restoring raw mode, opening the controlling TTY, and querying the window size.
//!
//! [`charmbracelet/x/term`]: https://github.com/charmbracelet/x/tree/main/term

#![warn(missing_docs)]

mod sys;

use std::fs::File;
use std::io::{self, IsTerminal};
#[cfg(unix)]
use std::os::fd::AsRawFd;
#[cfg(windows)]
use std::os::windows::io::AsRawHandle;

/// The dimensions of a terminal window, in character cells.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WindowSize {
    /// Width in columns.
    pub width: u16,
    /// Height in rows.
    pub height: u16,
}

/// Platform-specific saved terminal state.
#[derive(Debug, Clone)]
pub struct State {
    inner: sys::PlatformState,
}

/// Returns whether `file` is connected to a terminal.
pub fn is_terminal(file: &File) -> bool {
    sys::is_terminal(raw_fd(file))
}

/// Returns whether the standard output stream is a terminal.
pub fn is_stdout_terminal() -> bool {
    io::stdout().is_terminal()
}

/// Returns whether the standard input stream is a terminal.
pub fn is_stdin_terminal() -> bool {
    io::stdin().is_terminal()
}

/// Puts `file` into raw mode and returns the previous terminal state.
pub fn make_raw(file: &File) -> io::Result<State> {
    sys::make_raw(raw_fd(file))
}

/// Returns the current terminal state for `file`.
pub fn get_state(file: &File) -> io::Result<State> {
    sys::get_state(raw_fd(file))
}

/// Restores a previous terminal state on `file`.
pub fn restore(file: &File, state: &State) -> io::Result<()> {
    sys::restore(raw_fd(file), state)
}

/// Applies `state` to `file`.
pub fn set_state(file: &File, state: &State) -> io::Result<()> {
    sys::set_state(raw_fd(file), state)
}

/// Returns the visible size of the terminal attached to `file`.
pub fn get_size(file: &File) -> io::Result<WindowSize> {
    sys::get_size(raw_fd(file))
}

/// Opens the controlling terminal device for read/write.
///
/// On Unix this opens `/dev/tty`. On Windows this opens `CONOUT$` (the active
/// console output device). Use [`open_tty_input`] on Windows when raw mode on
/// stdin is required.
pub fn open_tty() -> io::Result<File> {
    sys::open_tty()
}

/// Opens the controlling terminal input device.
///
/// On Unix this is the same as [`open_tty`]. On Windows this opens `CONIN$`.
pub fn open_tty_input() -> io::Result<File> {
    sys::open_tty_input()
}

/// Returns the size of the controlling terminal, when available.
pub fn open_tty_size() -> io::Result<WindowSize> {
    let tty = open_tty()?;
    get_size(&tty)
}

#[cfg(unix)]
fn raw_fd(file: &File) -> std::os::fd::RawFd {
    file.as_raw_fd()
}

#[cfg(windows)]
fn raw_fd(file: &File) -> std::os::windows::io::RawHandle {
    file.as_raw_handle()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_size_default_is_zero() {
        assert_eq!(
            WindowSize::default(),
            WindowSize {
                width: 0,
                height: 0
            }
        );
    }

    #[test]
    fn mock_size_roundtrip() {
        let size = WindowSize {
            width: 80,
            height: 24,
        };
        assert_eq!(size.width, 80);
        assert_eq!(size.height, 24);
    }

    #[test]
    fn stdout_terminal_check_does_not_panic() {
        let _ = is_stdout_terminal();
    }

    #[test]
    fn open_tty_size_when_available() {
        if let Ok(tty) = open_tty()
            && let Ok(size) = get_size(&tty)
        {
            assert!(size.width > 0);
            assert!(size.height > 0);
        }
    }
}
