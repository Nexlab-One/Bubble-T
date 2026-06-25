//! Kitty keyboard protocol sequence builders and CSI `u` key decode.

mod decode;

pub use decode::{KittyEventType, KittyKeyEvent, KittyMod, try_parse_key, try_parse_key_csi_body};

/// Kitty keyboard progressive enhancement flags.
///
/// See <https://sw.kovidgoyal.net/kitty/keyboard-protocol/#progressive-enhancement>.
pub const KITTY_DISAMBIGUATE_ESCAPE_CODES: i32 = 1;
/// Report press, release, and repeat event types.
pub const KITTY_REPORT_EVENT_TYPES: i32 = 2;
/// Report alternate (shifted/base) key codes.
pub const KITTY_REPORT_ALTERNATE_KEYS: i32 = 4;
/// Report all keys as escape codes.
pub const KITTY_REPORT_ALL_KEYS_AS_ESCAPE_CODES: i32 = 8;
/// Report associated text with key events.
pub const KITTY_REPORT_ASSOCIATED_KEYS: i32 = 16;

/// All Kitty keyboard flags combined.
pub const KITTY_ALL_FLAGS: i32 = KITTY_DISAMBIGUATE_ESCAPE_CODES
    | KITTY_REPORT_EVENT_TYPES
    | KITTY_REPORT_ALTERNATE_KEYS
    | KITTY_REPORT_ALL_KEYS_AS_ESCAPE_CODES
    | KITTY_REPORT_ASSOCIATED_KEYS;

/// Query the terminal for enabled Kitty keyboard flags (`CSI ? u`).
pub const REQUEST_KITTY_KEYBOARD: &str = "\x1b[?u";

/// Requests keyboard enhancements from the terminal.
///
/// `mode` is typically `1` (set flags, unset others), `2` (set flags, keep others),
/// or `3` (unset flags, keep others).
pub fn kitty_keyboard(flags: i32, mode: i32) -> String {
    format!("\x1b[={flags};{mode}u")
}

/// Pushes `flags` onto the Kitty keyboard stack (`CSI > flags u`).
pub fn push_kitty_keyboard(flags: i32) -> String {
    if flags > 0 {
        format!("\x1b[>{flags}u")
    } else {
        "\x1b[>u".to_string()
    }
}

/// Disables Kitty keyboard enhancements by pushing zero onto the stack.
pub const DISABLE_KITTY_KEYBOARD: &str = "\x1b[>u";

/// Pops `n` entries from the Kitty keyboard stack (`CSI < n u`).
pub fn pop_kitty_keyboard(n: i32) -> String {
    if n > 0 {
        format!("\x1b[<{n}u")
    } else {
        "\x1b[<u".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kitty_keyboard_sequence() {
        assert_eq!(kitty_keyboard(3, 1), "\x1b[=3;1u");
    }

    #[test]
    fn push_and_pop() {
        assert_eq!(push_kitty_keyboard(2), "\x1b[>2u");
        assert_eq!(pop_kitty_keyboard(1), "\x1b[<1u");
    }
}
