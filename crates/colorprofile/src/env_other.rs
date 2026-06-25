//! Non-Windows platform stubs for Windows-only detection.

use crate::Profile;

/// Returns a Windows color profile when `$TERM` is unset or empty.
pub fn windows_color_profile(_environ: &[(&str, &str)]) -> Option<Profile> {
    None
}
