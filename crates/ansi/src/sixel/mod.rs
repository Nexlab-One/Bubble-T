//! Sixel graphics encode/decode.
//!
//! Port of [`charmbracelet/x/ansi/sixel`].
//!
//! [`charmbracelet/x/ansi/sixel`]: https://github.com/charmbracelet/x/tree/main/ansi/sixel

mod color;
mod decoder;
mod encoder;
mod palette;
mod palette_default;
mod raster;
mod repeat;

pub use color::{Color, DecodeColorError, decode_color, write_color};
pub use decoder::{DecodeError, Decoder};
pub use encoder::Encoder;
pub use palette::{MAX_COLORS, SixelPalette, new_palette};
pub use raster::{DecodeRasterError, Raster, decode_raster, write_raster};
pub use repeat::{DecodeRepeatError, Repeat, decode_repeat, write_repeat};

/// Sixel control bytes.
pub mod control {
    /// Line break (`-`).
    pub const LINE_BREAK: u8 = b'-';
    /// Carriage return (`$`).
    pub const CARRIAGE_RETURN: u8 = b'$';
    /// Repeat introducer (`!`).
    pub const REPEAT_INTRODUCER: u8 = b'!';
    /// Color introducer (`#`).
    pub const COLOR_INTRODUCER: u8 = b'#';
    /// Raster attribute introducer (`"`).
    pub const RASTER_ATTRIBUTE: u8 = b'"';
}
