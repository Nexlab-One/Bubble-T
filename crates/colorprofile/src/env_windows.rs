//! Windows-specific color profile detection.

use crate::Profile;

/// Returns a Windows color profile when `$TERM` is unset or empty.
pub fn windows_color_profile(environ: &[(&str, &str)]) -> Option<Profile> {
    if lookup(environ, "ConEmuANSI") == Some("ON") {
        return Some(Profile::TrueColor);
    }

    if let Some(ansicon) = lookup(environ, "ANSICON")
        && !ansicon.is_empty()
    {
        let ver = lookup(environ, "ANSICON_VER").and_then(|v| v.parse::<u32>().ok());
        if ver.is_none_or(|v| v < 181) {
            return Some(Profile::Ansi);
        }
        return Some(Profile::Ansi256);
    }

    // Modern Windows consoles (Windows Terminal, recent ConHost) support VT
    // sequences even when `$TERM` is unset.
    Some(Profile::TrueColor)
}

fn lookup<'a>(environ: &'a [(&str, &str)], key: &str) -> Option<&'a str> {
    environ.iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conemu_is_truecolor() {
        assert_eq!(
            windows_color_profile(&[("ConEmuANSI", "ON")]),
            Some(Profile::TrueColor)
        );
    }

    #[test]
    fn ansicon_old_is_ansi() {
        assert_eq!(
            windows_color_profile(&[("ANSICON", "1"), ("ANSICON_VER", "100")]),
            Some(Profile::Ansi)
        );
    }

    #[test]
    fn ansicon_new_is_256() {
        assert_eq!(
            windows_color_profile(&[("ANSICON", "1"), ("ANSICON_VER", "181")]),
            Some(Profile::Ansi256)
        );
    }

    #[test]
    fn bare_windows_defaults_truecolor() {
        assert_eq!(windows_color_profile(&[]), Some(Profile::TrueColor));
    }
}
