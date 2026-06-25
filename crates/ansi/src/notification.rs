//! Desktop notification OSC sequences.

use crate::seq::osc_bel;

/// Sends a desktop notification using iTerm's OSC 9.
///
/// `OSC 9 ; Mc BEL`
pub fn notify(body: &str) -> String {
    osc_bel(&format!("9;{body}"))
}

/// Sends an extensible desktop notification (OSC 99).
///
/// `OSC 99 ; metadata ; payload BEL`
pub fn desktop_notification(payload: &str, metadata: &[&str]) -> String {
    osc_bel(&format!("99;{};{payload}", metadata.join(":")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iterm_notify() {
        assert_eq!(notify("hello"), "\x1b]9;hello\x07");
    }
}
