//! Screen erase, scroll, and margin sequences.

use crate::seq::csi_n;

/// Clears part or all of the display (ED).
pub fn erase_display(n: i32) -> String {
    csi_n(n, b'J')
}

/// Clears part or all of the current line (EL).
pub fn erase_line(n: i32) -> String {
    csi_n(n, b'K')
}

/// Clears from cursor to end of screen.
pub const ERASE_SCREEN_BELOW: &str = "\x1b[J";
/// Clears from cursor to beginning of screen.
pub const ERASE_SCREEN_ABOVE: &str = "\x1b[1J";
/// Clears the entire visible screen.
pub const ERASE_ENTIRE_SCREEN: &str = "\x1b[2J";
/// Clears the entire display including scrollback.
pub const ERASE_ENTIRE_DISPLAY: &str = "\x1b[3J";

/// Clears from cursor to end of line.
pub const ERASE_LINE_RIGHT: &str = "\x1b[K";
/// Clears from cursor to beginning of line.
pub const ERASE_LINE_LEFT: &str = "\x1b[1K";
/// Clears the entire current line.
pub const ERASE_ENTIRE_LINE: &str = "\x1b[2K";

/// Scrolls the screen up `n` lines (SU).
pub fn scroll_up(n: i32) -> String {
    csi_n(n, b'S')
}

/// Scrolls the screen down `n` lines (SD).
pub fn scroll_down(n: i32) -> String {
    csi_n(n, b'T')
}

/// Inserts `n` blank lines at the cursor (IL).
pub fn insert_line(n: i32) -> String {
    csi_n(n, b'L')
}

/// Deletes `n` lines at the cursor (DL).
pub fn delete_line(n: i32) -> String {
    csi_n(n, b'M')
}

/// Inserts `n` blank characters at the cursor (ICH).
pub fn insert_character(n: i32) -> String {
    csi_n(n, b'@')
}

/// Deletes `n` characters at the cursor (DCH).
pub fn delete_character(n: i32) -> String {
    csi_n(n, b'P')
}

/// Erases `n` characters at the cursor (ECH).
pub fn erase_character(n: i32) -> String {
    csi_n(n, b'X')
}

/// Repeats the previous character `n` times (REP).
pub fn repeat_previous_character(n: i32) -> String {
    csi_n(n, b'b')
}

/// Sets top and bottom scrolling margins (DECSTBM).
pub fn set_top_bottom_margins(top: i32, bottom: i32) -> String {
    let t = if top > 0 {
        top.to_string()
    } else {
        String::new()
    };
    let b = if bottom > 0 {
        bottom.to_string()
    } else {
        String::new()
    };
    format!("\x1b[{t};{b}r")
}

/// Resets the terminal to its initial state (RIS).
pub const RESET_INITIAL_STATE: &str = "\x1bc";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn erase_defaults() {
        assert_eq!(erase_display(0), "\x1b[J");
        assert_eq!(erase_line(2), "\x1b[2K");
    }
}
