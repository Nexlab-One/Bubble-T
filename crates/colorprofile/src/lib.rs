//! Terminal color-profile detection and ANSI downsampling.
//!
//! This crate is the Rust port of [`charmbracelet/colorprofile`]. It classifies a
//! terminal's color capability into a [`Profile`] from environment inspection
//! (`TERM`, `COLORTERM`, `NO_COLOR`, `CLICOLOR*`, and terminal-specific variables)
//! and provides the writer that downsamples [`ansi`] color sequences to the highest
//! fidelity the active profile supports.
//!
//! [`charmbracelet/colorprofile`]: https://github.com/charmbracelet/colorprofile

#![warn(missing_docs)]

mod env;
#[cfg(not(windows))]
mod env_other;
#[cfg(windows)]
mod env_windows;
mod terminfo;
mod tmux;
mod writer;

pub use env::{detect, env_profile};
pub use terminfo::terminfo_profile;
pub use tmux::tmux_profile;
pub use writer::Writer;

use ansi::color::{Color, convert16, convert256};

/// The color capability of a terminal, from richest to poorest.
///
/// Downsampling always maps a color to the nearest representable value in the active
/// profile, so a true-color value renders as an ANSI-256 or ANSI-16 approximation on
/// less capable terminals and is dropped entirely under [`Profile::NoTty`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Profile {
    /// Output is not a TTY; all styling is stripped.
    NoTty,
    /// No color; styling reduced to plain ASCII text.
    Ascii,
    /// 16-color (4-bit) palette.
    Ansi,
    /// 256-color (8-bit) palette.
    Ansi256,
    /// 24-bit "true color" (16.7M colors).
    TrueColor,
}

impl PartialOrd for Profile {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Profile {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (*self as u8).cmp(&(*other as u8))
    }
}

impl Profile {
    /// Converts `color` to the nearest color representable in this profile.
    pub fn convert_color(self, color: Color) -> Color {
        match self {
            Self::TrueColor => color,
            Self::Ansi256 => match color {
                Color::Basic(c) => Color::Basic(c),
                Color::Indexed(i) => Color::Indexed(i),
                Color::Rgb(_) => Color::Indexed(convert256(color)),
            },
            Self::Ansi => match color {
                Color::Basic(c) => Color::Basic(c),
                other => Color::Basic(convert16(other)),
            },
            Self::Ascii | Self::NoTty => color,
        }
    }
}

impl std::fmt::Display for Profile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TrueColor => f.write_str("TrueColor"),
            Self::Ansi256 => f.write_str("ANSI256"),
            Self::Ansi => f.write_str("ANSI"),
            Self::Ascii => f.write_str("Ascii"),
            Self::NoTty => f.write_str("NoTty"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ansi::color::RgbColor;

    #[test]
    fn convert_rgb_to_256() {
        let c = Profile::Ansi256.convert_color(RgbColor { r: 255, g: 0, b: 0 }.into());
        assert!(matches!(c, Color::Indexed(_)));
    }
}
