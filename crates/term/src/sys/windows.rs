//! Windows console terminal control.

use std::fs::OpenOptions;
use std::io;
use std::os::windows::io::RawHandle;

use windows_sys::Win32::Foundation::HANDLE;
use windows_sys::Win32::System::Console::{
    ENABLE_ECHO_INPUT, ENABLE_LINE_INPUT, ENABLE_PROCESSED_INPUT, ENABLE_VIRTUAL_TERMINAL_INPUT,
    GetConsoleMode, GetConsoleScreenBufferInfo, SetConsoleMode,
};

use crate::State;
use crate::WindowSize;

/// Saved Windows console mode.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PlatformState {
    pub mode: u32,
}

pub(crate) fn is_terminal(handle: RawHandle) -> bool {
    let mut mode = 0u32;
    unsafe { GetConsoleMode(handle as HANDLE, &mut mode) != 0 }
}

pub(crate) fn make_raw(handle: RawHandle) -> io::Result<State> {
    let mut mode = 0u32;
    let ok = unsafe { GetConsoleMode(handle as HANDLE, &mut mode) };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    let saved = PlatformState { mode };
    let raw = mode & !(ENABLE_ECHO_INPUT | ENABLE_PROCESSED_INPUT | ENABLE_LINE_INPUT)
        | ENABLE_VIRTUAL_TERMINAL_INPUT;
    let ok = unsafe { SetConsoleMode(handle as HANDLE, raw) };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(State { inner: saved })
}

pub(crate) fn get_state(handle: RawHandle) -> io::Result<State> {
    let mut mode = 0u32;
    let ok = unsafe { GetConsoleMode(handle as HANDLE, &mut mode) };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(State {
        inner: PlatformState { mode },
    })
}

pub(crate) fn set_state(handle: RawHandle, state: &State) -> io::Result<()> {
    let ok = unsafe { SetConsoleMode(handle as HANDLE, state.inner.mode) };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

pub(crate) fn restore(handle: RawHandle, state: &State) -> io::Result<()> {
    set_state(handle, state)
}

pub(crate) fn get_size(handle: RawHandle) -> io::Result<WindowSize> {
    let mut info = unsafe { std::mem::zeroed() };
    let ok = unsafe { GetConsoleScreenBufferInfo(handle as HANDLE, &mut info) };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(WindowSize {
        width: (info.srWindow.Right - info.srWindow.Left + 1) as u16,
        height: (info.srWindow.Bottom - info.srWindow.Top + 1) as u16,
    })
}

/// Opens the controlling console output device (`CONOUT$`).
pub(crate) fn open_tty() -> io::Result<std::fs::File> {
    OpenOptions::new().read(true).write(true).open("CONOUT$")
}

/// Opens the controlling console input device (`CONIN$`).
pub(crate) fn open_tty_input() -> io::Result<std::fs::File> {
    OpenOptions::new().read(true).write(true).open("CONIN$")
}
