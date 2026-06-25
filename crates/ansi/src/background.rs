//! OSC 10/11/12 default foreground, background, and cursor color sequences.

use crate::seq::osc_bel;

/// Requests the current default terminal foreground color (OSC 10).
pub const REQUEST_FOREGROUND_COLOR: &str = "\x1b]10;?\x07";

/// Requests the current default terminal background color (OSC 11).
pub const REQUEST_BACKGROUND_COLOR: &str = "\x1b]11;?\x07";

/// Requests the current terminal cursor color (OSC 12).
pub const REQUEST_CURSOR_COLOR: &str = "\x1b]12;?\x07";

/// Resets the default terminal foreground color (OSC 110).
pub const RESET_FOREGROUND_COLOR: &str = "\x1b]110\x07";

/// Resets the default terminal background color (OSC 111).
pub const RESET_BACKGROUND_COLOR: &str = "\x1b]111\x07";

/// Resets the terminal cursor color (OSC 112).
pub const RESET_CURSOR_COLOR: &str = "\x1b]112\x07";

/// Sets the default terminal foreground color (OSC 10).
pub fn set_foreground_color(color: &str) -> String {
    osc_bel(&format!("10;{color}"))
}

/// Sets the default terminal background color (OSC 11).
pub fn set_background_color(color: &str) -> String {
    osc_bel(&format!("11;{color}"))
}

/// Sets the terminal cursor color (OSC 12).
pub fn set_cursor_color(color: &str) -> String {
    osc_bel(&format!("12;{color}"))
}

/// Formats an [`ansi::color::RgbColor`] as a hex string for OSC color sequences.
pub fn color_to_hex(r: u8, g: u8, b: u8) -> String {
    format!("#{:02x}{:02x}{:02x}", r, g, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_background() {
        assert_eq!(REQUEST_BACKGROUND_COLOR, "\x1b]11;?\x07");
    }

    #[test]
    fn set_foreground_hex() {
        assert_eq!(set_foreground_color("#ff0000"), "\x1b]10;#ff0000\x07");
    }
}
