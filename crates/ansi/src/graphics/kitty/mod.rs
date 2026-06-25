//! Kitty terminal graphics protocol (image rendering).
//!
//! Port of [`charmbracelet/x/ansi/kitty`] graphics support. Keyboard protocol lives
//! in [`crate::kitty`].
//!
//! [`charmbracelet/x/ansi/kitty`]: https://github.com/charmbracelet/x/tree/main/ansi/kitty

mod decoder;
mod encoder;
mod options;
mod protocol;
mod shm;
mod writer;

pub use decoder::{DecodeError as KittyDecodeError, Decoder as KittyDecoder};
pub use encoder::Encoder as KittyEncoder;
pub use options::Options;
pub use protocol::*;
pub use shm::{SHM_NAME_PREFIX, ShmError, shared_memory_available};
pub use writer::{
    EncodeError, encode_graphics, encode_graphics_file, encode_graphics_from_path,
    encode_graphics_with_fallback, kitty_transmission_from_env,
};
