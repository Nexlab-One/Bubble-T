//! OSC window and icon title sequences.

use crate::seq::osc_bel;

/// Sets icon name and window title (OSC 0).
pub fn set_icon_name_window_title(title: &str) -> String {
    osc_bel(&format!("0;{title}"))
}

/// Sets the icon name (OSC 1).
pub fn set_icon_name(name: &str) -> String {
    osc_bel(&format!("1;{name}"))
}

/// Sets the window title (OSC 2).
pub fn set_window_title(title: &str) -> String {
    osc_bel(&format!("2;{title}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_title() {
        assert_eq!(set_window_title("Bubble Tea"), "\x1b]2;Bubble Tea\x07");
    }
}
