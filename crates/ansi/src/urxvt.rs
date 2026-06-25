//! URxvt perl-extension OSC 777 sequences.

use crate::seq::osc_bel;

/// Calls a URxvt perl extension (`OSC 777 ; name ; params BEL`).
pub fn urxvt_ext(extension: &str, params: &[&str]) -> String {
    osc_bel(&format!("777;{extension};{}", params.join(";")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_with_params() {
        assert_eq!(
            urxvt_ext("foo", &["bar", "baz"]),
            "\x1b]777;foo;bar;baz\x07"
        );
    }
}
