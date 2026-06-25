//! Terminal mode set/reset/request sequences.

use crate::seq::{csi_dec, csi_params};

/// How a mode is set or reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ModeSetting {
    /// Mode not recognized.
    NotRecognized = 0,
    /// Mode is set.
    Set = 1,
    /// Mode is reset.
    Reset = 2,
    /// Mode is permanently set.
    PermanentlySet = 3,
    /// Mode is permanently reset.
    PermanentlyReset = 4,
}

impl ModeSetting {
    /// Returns true when the mode is set (including permanently).
    pub const fn is_set(self) -> bool {
        matches!(self, Self::Set | Self::PermanentlySet)
    }

    /// Returns true when the mode is reset (including permanently).
    pub const fn is_reset(self) -> bool {
        matches!(self, Self::Reset | Self::PermanentlyReset)
    }
}

/// Identifies an ANSI or DEC private mode by number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mode {
    /// Standard ANSI mode (`CSI Pd h/l`).
    Ansi(i32),
    /// DEC private mode (`CSI ? Pd h/l`).
    Dec(i32),
}

impl Mode {
    /// Returns the numeric mode identifier.
    pub const fn number(self) -> i32 {
        match self {
            Self::Ansi(n) | Self::Dec(n) => n,
        }
    }

    /// Returns true for DEC private modes.
    pub const fn is_dec(self) -> bool {
        matches!(self, Self::Dec(_))
    }
}

// --- Commonly used DEC modes ---

/// Cursor keys mode (DECCKM).
pub const MODE_CURSOR_KEYS: Mode = Mode::Dec(1);
/// Origin mode (DECOM).
pub const MODE_ORIGIN: Mode = Mode::Dec(6);
/// Auto-wrap mode (DECAWM).
pub const MODE_AUTO_WRAP: Mode = Mode::Dec(7);
/// X10 mouse reporting.
pub const MODE_MOUSE_X10: Mode = Mode::Dec(9);
/// Text cursor enable (DECTCEM).
pub const MODE_TEXT_CURSOR_ENABLE: Mode = Mode::Dec(25);
/// Normal mouse tracking.
pub const MODE_MOUSE_NORMAL: Mode = Mode::Dec(1000);
/// Focus event reporting.
pub const MODE_FOCUS_EVENT: Mode = Mode::Dec(1004);
/// SGR extended mouse mode.
pub const MODE_MOUSE_EXT_SGR: Mode = Mode::Dec(1006);
/// Alternate screen buffer.
pub const MODE_ALT_SCREEN: Mode = Mode::Dec(1047);
/// Save cursor on alt-screen switch.
pub const MODE_SAVE_CURSOR: Mode = Mode::Dec(1048);
/// Alternate screen + save cursor + clear.
pub const MODE_ALT_SCREEN_SAVE_CURSOR: Mode = Mode::Dec(1049);
/// Bracketed paste mode.
pub const MODE_BRACKETED_PASTE: Mode = Mode::Dec(2004);
/// Synchronized output (mode 2026).
pub const MODE_SYNCHRONIZED_OUTPUT: Mode = Mode::Dec(2026);
/// Unicode grapheme width (mode 2027).
pub const MODE_UNICODE_CORE: Mode = Mode::Dec(2027);

/// Sets one or more terminal modes.
pub fn set_mode(modes: &[Mode]) -> String {
    set_or_reset_mode(false, modes)
}

/// Resets one or more terminal modes.
pub fn reset_mode(modes: &[Mode]) -> String {
    set_or_reset_mode(true, modes)
}

fn set_or_reset_mode(reset: bool, modes: &[Mode]) -> String {
    if modes.is_empty() {
        return String::new();
    }
    let cmd = if reset { 'l' } else { 'h' };

    if modes.len() == 1 {
        let m = modes[0];
        return match m {
            Mode::Dec(n) => csi_dec(n, cmd as u8),
            Mode::Ansi(n) => crate::seq::csi_n(n, cmd as u8),
        };
    }

    let mut ansi: Vec<String> = Vec::new();
    let mut dec: Vec<String> = Vec::new();
    for m in modes {
        match m {
            Mode::Ansi(n) => ansi.push(n.to_string()),
            Mode::Dec(n) => dec.push(n.to_string()),
        }
    }

    let mut out = String::new();
    if !ansi.is_empty() {
        out.push_str(&csi_params(&ansi.join(";"), cmd as u8));
    }
    if !dec.is_empty() {
        out.push_str(&format!("\x1b[?{}{cmd}", dec.join(";")));
    }
    out
}

/// Requests the setting of a mode (DECRQM).
pub fn request_mode(mode: Mode) -> String {
    let n = mode.number();
    match mode {
        Mode::Dec(_) => format!("\x1b[?{n}$p"),
        Mode::Ansi(_) => format!("\x1b[{n}$p"),
    }
}

/// Builds a mode report response (DECRPM).
pub fn report_mode(mode: Mode, value: ModeSetting) -> String {
    let v = (value as u8).min(4);
    let n = mode.number();
    match mode {
        Mode::Dec(_) => format!("\x1b[?{n};{v}$y"),
        Mode::Ansi(_) => format!("\x1b[{n};{v}$y"),
    }
}

/// Shows the text cursor (DECTCEM set).
pub const SHOW_CURSOR: &str = "\x1b[?25h";
/// Hides the text cursor (DECTCEM reset).
pub const HIDE_CURSOR: &str = "\x1b[?25l";

/// Enables the alternate screen with saved cursor and clear (1049).
pub const SET_MODE_ALT_SCREEN_SAVE_CURSOR: &str = "\x1b[?1049h";
/// Disables the alternate screen with saved cursor (1049).
pub const RESET_MODE_ALT_SCREEN_SAVE_CURSOR: &str = "\x1b[?1049l";

/// Enables bracketed paste mode.
pub const SET_MODE_BRACKETED_PASTE: &str = "\x1b[?2004h";
/// Disables bracketed paste mode.
pub const RESET_MODE_BRACKETED_PASTE: &str = "\x1b[?2004l";

/// Enables synchronized output.
pub const SET_MODE_SYNCHRONIZED_OUTPUT: &str = "\x1b[?2026h";
/// Disables synchronized output.
pub const RESET_MODE_SYNCHRONIZED_OUTPUT: &str = "\x1b[?2026l";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dec_set_single() {
        assert_eq!(set_mode(&[MODE_ALT_SCREEN_SAVE_CURSOR]), "\x1b[?1049h");
    }

    #[test]
    fn decrqm() {
        assert_eq!(request_mode(MODE_BRACKETED_PASTE), "\x1b[?2004$p");
    }
}
