//! OSC 7 working-directory notification.

use crate::seq::osc_bel;

/// Notifies the terminal of the current working directory (OSC 7).
///
/// `Pt` is a `file://` URL; use `localhost` for local paths.
pub fn notify_working_directory(host: &str, path: &str) -> String {
    let normalized = path.replace('\\', "/");
    osc_bel(&format!("7;file://{host}{normalized}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cwd_url() {
        assert_eq!(
            notify_working_directory("localhost", "/home/user"),
            "\x1b]7;file://localhost/home/user\x07"
        );
    }
}
