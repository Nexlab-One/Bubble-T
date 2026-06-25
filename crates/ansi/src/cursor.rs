//! Cursor movement, shape, and position sequences.

use crate::seq::csi_n;

/// Saves the cursor position (DECSC).
pub const SAVE_CURSOR: &str = "\x1b7";
/// Restores the cursor position (DECRC).
pub const RESTORE_CURSOR: &str = "\x1b8";

/// Requests the cursor position (CPR).
pub const REQUEST_CURSOR_POSITION_REPORT: &str = "\x1b[6n";
/// Requests extended cursor position (DECXCPR).
pub const REQUEST_EXTENDED_CURSOR_POSITION_REPORT: &str = "\x1b[?6n";

/// Moves the cursor up `n` cells (CUU).
pub fn cursor_up(n: i32) -> String {
    csi_n(n, b'A')
}

/// Moves the cursor down `n` cells (CUD).
pub fn cursor_down(n: i32) -> String {
    csi_n(n, b'B')
}

/// Moves the cursor forward/right `n` cells (CUF).
pub fn cursor_forward(n: i32) -> String {
    csi_n(n, b'C')
}

/// Moves the cursor back/left `n` cells (CUB).
pub fn cursor_backward(n: i32) -> String {
    csi_n(n, b'D')
}

/// Moves the cursor to the beginning of the line `n` lines down (CNL).
pub fn cursor_next_line(n: i32) -> String {
    csi_n(n, b'E')
}

/// Moves the cursor to the beginning of the line `n` lines up (CPL).
pub fn cursor_previous_line(n: i32) -> String {
    csi_n(n, b'F')
}

/// Moves the cursor to column `col` (CHA). Default column is 1.
pub fn cursor_horizontal_absolute(col: i32) -> String {
    let n = if col > 0 { col } else { 1 };
    csi_n(n, b'G')
}

/// Moves the cursor to row `row` (VPA). Default row is 1.
pub fn cursor_vertical_absolute(row: i32) -> String {
    let n = if row > 0 { row } else { 1 };
    csi_n(n, b'd')
}

/// Moves the cursor to `(row, col)` (CUP). Defaults are row=1, col=1.
pub fn cursor_position(row: i32, col: i32) -> String {
    let r = if row > 0 {
        row.to_string()
    } else {
        String::new()
    };
    let c = if col > 0 {
        col.to_string()
    } else {
        String::new()
    };
    format!("\x1b[{r};{c}H")
}

/// Saves cursor + attributes (SCP, mode 2026 extension).
pub fn save_cursor_position() -> String {
    "\x1b[s".to_string()
}

/// Restores cursor + attributes (RCP).
pub fn restore_cursor_position() -> String {
    "\x1b[u".to_string()
}

/// Cursor shape variants for DECSCUSR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CursorShape {
    /// Block cursor.
    Block = 1,
    /// Underline cursor.
    Underline = 3,
    /// Bar cursor.
    Bar = 5,
}

/// Sets the cursor shape (DECSCUSR).
pub fn set_cursor_shape(shape: CursorShape) -> String {
    format!("\x1b[{} q", shape as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cup_home() {
        assert_eq!(cursor_position(1, 1), "\x1b[1;1H");
    }

    #[test]
    fn cuu_default() {
        assert_eq!(cursor_up(1), "\x1b[A");
    }
}
