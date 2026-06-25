//! OSC 8 hyperlink sequences.

use crate::seq::osc_bel;

/// Starts an OSC 8 hyperlink. Pass an empty `uri` to reset.
pub fn set_hyperlink(uri: &str, params: &[&str]) -> String {
    let p = params.join(":");
    osc_bel(&format!("8;{p};{uri}"))
}

/// Resets the active hyperlink.
pub fn reset_hyperlink(params: &[&str]) -> String {
    set_hyperlink("", params)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hyperlink_with_id() {
        assert_eq!(
            set_hyperlink("https://example.com", &["id=abc"]),
            "\x1b]8;id=abc;https://example.com\x07"
        );
    }

    #[test]
    fn reset_link() {
        assert_eq!(super::reset_hyperlink(&[]), "\x1b]8;;\x07");
    }
}
