//! OSC 52 clipboard manipulation sequences.

use base64::{Engine as _, engine::general_purpose::STANDARD};

use crate::seq::osc_bel;

/// System clipboard selector (`c`).
pub const SYSTEM_CLIPBOARD: char = 'c';
/// Primary selection clipboard selector (`p`).
pub const PRIMARY_CLIPBOARD: char = 'p';

/// Sets clipboard `selector` to `data` (base64-encoded on the wire).
pub fn set_clipboard(selector: char, data: &str) -> String {
    let payload = if data.is_empty() {
        String::new()
    } else {
        STANDARD.encode(data.as_bytes())
    };
    osc_bel(&format!("52;{selector};{payload}"))
}

/// Sets the system clipboard.
pub fn set_system_clipboard(data: &str) -> String {
    set_clipboard(SYSTEM_CLIPBOARD, data)
}

/// Sets the primary selection clipboard.
pub fn set_primary_clipboard(data: &str) -> String {
    set_clipboard(PRIMARY_CLIPBOARD, data)
}

/// Clears clipboard `selector`.
pub fn reset_clipboard(selector: char) -> String {
    set_clipboard(selector, "")
}

/// Clears the system clipboard.
pub const RESET_SYSTEM_CLIPBOARD: &str = "\x1b]52;c;\x07";
/// Clears the primary clipboard.
pub const RESET_PRIMARY_CLIPBOARD: &str = "\x1b]52;p;\x07";

/// Requests clipboard contents from `selector`.
pub fn request_clipboard(selector: char) -> String {
    osc_bel(&format!("52;{selector};?"))
}

/// Requests the system clipboard.
pub const REQUEST_SYSTEM_CLIPBOARD: &str = "\x1b]52;c;?\x07";
/// Requests the primary clipboard.
pub const REQUEST_PRIMARY_CLIPBOARD: &str = "\x1b]52;p;?\x07";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_system_clipboard_encodes() {
        assert_eq!(set_system_clipboard("hi"), "\x1b]52;c;aGk=\x07");
    }

    #[test]
    fn request_primary() {
        assert_eq!(
            request_clipboard(PRIMARY_CLIPBOARD),
            REQUEST_PRIMARY_CLIPBOARD
        );
    }
}
