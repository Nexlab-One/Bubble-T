//! OSC 9;4 native terminal progress bar sequences.

use crate::seq::osc_bel;

/// Resets the progress bar to its default state (hidden).
///
/// OSC `9;4;0` BEL
pub const RESET_PROGRESS_BAR: &str = "\x1b]9;4;0\x07";

/// Sets the progress bar to `percentage` (0–100) in the default state.
///
/// OSC `9;4;1;{percentage}` BEL
pub fn set_progress_bar(percentage: i32) -> String {
    let pct = percentage.clamp(0, 100);
    osc_bel(&format!("9;4;1;{pct}"))
}

/// Sets the progress bar to `percentage` (0–100) in the error state.
pub fn set_error_progress_bar(percentage: i32) -> String {
    let pct = percentage.clamp(0, 100);
    osc_bel(&format!("9;4;2;{pct}"))
}

/// Sets the progress bar to the indeterminate state.
pub const SET_INDETERMINATE_PROGRESS_BAR: &str = "\x1b]9;4;3\x07";

/// Sets the progress bar to `percentage` (0–100) in the warning state.
pub fn set_warning_progress_bar(percentage: i32) -> String {
    let pct = percentage.clamp(0, 100);
    osc_bel(&format!("9;4;4;{pct}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_progress_bar() {
        assert_eq!(set_progress_bar(50), "\x1b]9;4;1;50\x07");
    }

    #[test]
    fn clamps_percentage() {
        assert_eq!(set_progress_bar(150), "\x1b]9;4;1;100\x07");
    }
}
