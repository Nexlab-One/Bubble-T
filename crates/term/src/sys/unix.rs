//! Unix terminal control via termios.

use std::fs::File;
use std::io;
use std::mem;
use std::os::fd::RawFd;

use libc::{TIOCGWINSZ, termios, winsize};

use crate::State;
use crate::WindowSize;

/// Saved Unix termios state.
#[derive(Debug, Clone)]
pub(crate) struct PlatformState {
    pub termios: termios,
}

pub(crate) fn is_terminal(fd: RawFd) -> bool {
    let mut termios = mem::MaybeUninit::<termios>::uninit();
    unsafe { libc::tcgetattr(fd, termios.as_mut_ptr()) == 0 }
}

pub(crate) fn make_raw(fd: RawFd) -> io::Result<State> {
    let mut termios = get_termios(fd)?;
    let saved = PlatformState { termios };

    termios.c_iflag &= !(libc::IGNBRK
        | libc::BRKINT
        | libc::PARMRK
        | libc::ISTRIP
        | libc::INLCR
        | libc::IGNCR
        | libc::ICRNL
        | libc::IXON);
    termios.c_oflag &= !libc::OPOST;
    termios.c_lflag &= !(libc::ECHO | libc::ECHONL | libc::ICANON | libc::ISIG | libc::IEXTEN);
    termios.c_cflag &= !(libc::CSIZE | libc::PARENB);
    termios.c_cflag |= libc::CS8;
    termios.c_cc[libc::VMIN as usize] = 1;
    termios.c_cc[libc::VTIME as usize] = 0;

    set_termios(fd, &termios)?;
    Ok(State { inner: saved })
}

pub(crate) fn get_state(fd: RawFd) -> io::Result<State> {
    Ok(State {
        inner: PlatformState {
            termios: get_termios(fd)?,
        },
    })
}

pub(crate) fn set_state(fd: RawFd, state: &State) -> io::Result<()> {
    set_termios(fd, &state.inner.termios)
}

pub(crate) fn restore(fd: RawFd, state: &State) -> io::Result<()> {
    set_state(fd, state)
}

pub(crate) fn get_size(fd: RawFd) -> io::Result<WindowSize> {
    let mut ws = winsize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    let rc = unsafe { libc::ioctl(fd, TIOCGWINSZ, &mut ws) };
    if rc == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(WindowSize {
        width: ws.ws_col as u16,
        height: ws.ws_row as u16,
    })
}

pub(crate) fn open_tty() -> io::Result<File> {
    File::options().read(true).write(true).open("/dev/tty")
}

pub(crate) fn open_tty_input() -> io::Result<File> {
    open_tty()
}

fn get_termios(fd: RawFd) -> io::Result<termios> {
    let mut termios = mem::MaybeUninit::<termios>::uninit();
    let rc = unsafe { libc::tcgetattr(fd, termios.as_mut_ptr()) };
    if rc == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { termios.assume_init() })
}

fn set_termios(fd: RawFd, termios: &termios) -> io::Result<()> {
    let rc = unsafe { libc::tcsetattr(fd, libc::TCSANOW, termios) };
    if rc == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[allow(dead_code)]
pub(crate) fn _file_ref(_file: &File) {}
