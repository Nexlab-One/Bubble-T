//! ANSI/VT escape-sequence construction and parsing.
//!
//! This crate is the Rust port of [`charmbracelet/x/ansi`]. It is the dependency
//! root of the workspace: the cell buffer, color profile, Lip Gloss, and the core
//! input/query parser all build on the escape-sequence primitives defined here.
//!
//! The crate is split into two halves that upstream keeps together:
//!
//! - **Builders** — functions and constants that emit terminal control sequences
//!   (CSI, OSC, DCS, SGR, mode set/reset, cursor movement, screen control, Kitty
//!   keyboard, OSC 52 clipboard, OSC 8 hyperlinks, and the device queries used by
//!   the runtime).
//! - **Parser** — a byte-stream decoder that recognises C0/C1 controls, CSI/OSC/DCS
//!   sequences, and the report sequences the runtime issues queries for (DA, DECRPM,
//!   XTVERSION, XTGETTCAP, OSC color, Kitty key events).
//!
//! [`charmbracelet/x/ansi`]: https://github.com/charmbracelet/x/tree/main/ansi

#![warn(missing_docs)]

pub mod background;
pub mod c0;
pub mod c1;
pub mod clipboard;
pub mod color;
pub mod ctrl;
pub mod cursor;
pub mod cwd;
pub mod finalterm;
pub mod focus;
pub mod graphics;
pub mod hyperlink;
pub mod kitty;
pub mod mode;
pub mod mouse;
pub mod notification;
pub mod palette;
pub mod parse;
pub mod passthrough;
pub mod paste;
pub mod progress;
pub mod query;
pub mod screen;
pub mod seq;
pub mod sgr;
pub mod sixel;
pub mod status;
pub mod style;
pub mod title;
pub mod truncate;
pub mod urxvt;
pub mod width;
pub mod winop;
pub mod wrap;
pub mod xterm;

pub use graphics::{kitty_graphics, sixel_graphics};
pub use sixel::{Decoder as SixelDecoder, Encoder as SixelEncoder};

/// Control Sequence Introducer: the two-byte prefix (`ESC [`) that begins a CSI
/// sequence.
pub const CSI: &str = seq::CSI;

/// Operating System Command: the prefix (`ESC ]`) that begins an OSC sequence such
/// as window-title, clipboard (OSC 52), or hyperlink (OSC 8) control strings.
pub const OSC: &str = seq::OSC;

/// Device Control String prefix (`ESC P`).
pub const DCS: &str = seq::DCS;

/// Writes `sequence` to `writer`.
pub fn execute(writer: &mut impl std::io::Write, sequence: &str) -> std::io::Result<usize> {
    writer.write(sequence.as_bytes())
}

#[cfg(test)]
mod integration {
    use super::*;
    use crate::parse::{DecodeState, Parser, decode_sequence};
    use crate::sgr::{ATTR_BOLD, ATTR_RED_FOREGROUND, select_graphic_rendition};
    use crate::style::Style;

    #[test]
    fn builder_matches_decoder_for_sgr() {
        let seq = select_graphic_rendition(&[ATTR_RED_FOREGROUND, ATTR_BOLD]);
        let mut p = Parser::new();
        let d = decode_sequence(seq.as_bytes(), DecodeState::Normal, Some(&mut p));
        assert_eq!(d.sequence, seq.as_bytes());
        assert_eq!(p.params(), &[31, 1]);
    }

    #[test]
    fn style_roundtrip() {
        let seq = Style::new()
            .bold()
            .foreground_color(Some(color::IndexedColor(255).into()))
            .to_string();
        assert_eq!(seq, "\x1b[1;38;5;255m");
    }
}
