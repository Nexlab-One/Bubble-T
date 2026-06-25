//! Kitty graphics sequence writer.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use image::RgbaImage;
use tempfile::Builder;

use super::encoder::{EncodeError as PixelEncodeError, Encoder};
use super::options::Options;
use super::protocol::{DIRECT, FILE, MAX_CHUNK_SIZE, SHARED_MEMORY, TEMP_FILE, ZLIB};
use super::shm::{self, ShmError};
use crate::graphics::kitty_graphics;

/// Pattern required in temp file names for Kitty graphics.
pub const GRAPHICS_TEMP_PATTERN: &str = "tty-graphics-protocol-";

/// Error returned when writing Kitty graphics sequences fails.
#[derive(Debug)]
pub enum EncodeError {
    /// Missing file path for file transmission.
    MissingFile,
    /// Shared memory transmission failed or is unavailable.
    SharedMemory(ShmError),
    /// Underlying pixel encoding failed.
    Pixel(PixelEncodeError),
    /// Filesystem error.
    Io(std::io::Error),
}

impl std::fmt::Display for EncodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingFile => f.write_str("missing file path for kitty file transmission"),
            Self::SharedMemory(e) => write!(f, "kitty shared memory transmission failed: {e}"),
            Self::Pixel(e) => write!(f, "{e}"),
            Self::Io(e) => write!(f, "kitty graphics io error: {e}"),
        }
    }
}

impl std::error::Error for EncodeError {}

impl From<ShmError> for EncodeError {
    fn from(value: ShmError) -> Self {
        Self::SharedMemory(value)
    }
}

impl From<std::io::Error> for EncodeError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

/// Parses [`IMAGE_TRANSMISSION`] environment values into a Kitty `t=` byte.
///
/// Recognized values: `shm`/`shared`/`s`, `temp`/`t`, `file`/`f`, `direct`/`d`.
/// Unknown values default to [`DIRECT`].
///
/// [`IMAGE_TRANSMISSION`]: encode_graphics_with_fallback
#[must_use]
pub fn kitty_transmission_from_env() -> u8 {
    match std::env::var("IMAGE_TRANSMISSION")
        .map(|v| v.to_ascii_lowercase())
        .as_deref()
    {
        Ok("shm") | Ok("shared") | Ok("s") => SHARED_MEMORY,
        Ok("temp") | Ok("tmp") => TEMP_FILE,
        Ok("file") | Ok("f") => FILE,
        Ok("direct") | Ok("d") => DIRECT,
        _ => DIRECT,
    }
}

/// Encodes graphics, falling back to direct transmission when shared memory fails.
///
/// When `opts.transmission` is [`SHARED_MEMORY`] and segment creation fails, the
/// image is re-encoded with `t=d` so callers can still render over slow links.
pub fn encode_graphics_with_fallback(
    img: &RgbaImage,
    opts: &Options,
) -> Result<String, EncodeError> {
    match encode_graphics(img, opts) {
        Ok(seq) => Ok(seq),
        Err(EncodeError::SharedMemory(_)) if opts.transmission == SHARED_MEMORY => {
            let mut fallback = opts.clone();
            fallback.transmission = DIRECT;
            encode_graphics(img, &fallback)
        }
        Err(err) => Err(err),
    }
}

/// Encodes and writes a Kitty graphics APC sequence for `img`.
pub fn encode_graphics(img: &RgbaImage, opts: &Options) -> Result<String, EncodeError> {
    let mut opts = opts.clone();
    if opts.transmission == 0 && !opts.file.is_empty() {
        opts.transmission = FILE;
    }

    let mut data = Vec::new();
    let encoder = Encoder {
        compress: opts.compression == ZLIB,
        format: opts.format,
    };

    match opts.transmission {
        DIRECT => {
            data = encoder.encode(img).map_err(EncodeError::Pixel)?;
        }
        SHARED_MEMORY => {
            let encoded = encoder.encode(img).map_err(EncodeError::Pixel)?;
            if !shm::shared_memory_available() {
                return Err(EncodeError::SharedMemory(ShmError::Unsupported));
            }
            let name = shm::create_with_data(&encoded)?;
            if opts.size <= 0 {
                opts.size = encoded.len() as i32;
            }
            data.extend_from_slice(name.as_bytes());
        }
        FILE => {
            if opts.file.is_empty() {
                return Err(EncodeError::MissingFile);
            }
            let path = Path::new(&opts.file);
            if !path.is_file() {
                return Err(EncodeError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "file is not a regular file",
                )));
            }
            data.extend_from_slice(path.as_os_str().as_encoded_bytes());
        }
        TEMP_FILE => {
            let mut file = Builder::new()
                .prefix(GRAPHICS_TEMP_PATTERN)
                .tempfile()
                .map_err(EncodeError::Io)?;
            let encoded = encoder.encode(img).map_err(EncodeError::Pixel)?;
            file.write_all(&encoded).map_err(EncodeError::Io)?;
            let (_, path) = file.keep().map_err(|e| EncodeError::Io(e.error))?;
            data.extend_from_slice(path.as_os_str().as_encoded_bytes());
        }
        _ => {
            data = encoder.encode(img).map_err(EncodeError::Pixel)?;
        }
    }

    let payload = STANDARD.encode(&data);
    if !opts.chunk {
        let opt_strings = opts.option_strings();
        let opt_refs: Vec<&str> = opt_strings.iter().map(String::as_str).collect();
        return Ok(kitty_graphics(payload.as_bytes(), &opt_refs));
    }

    write_chunked(&payload, &opts)
}

fn write_chunked(payload: &str, opts: &Options) -> Result<String, EncodeError> {
    let bytes = payload.as_bytes();
    let mut out = String::new();
    let mut first = true;
    let mut offset = 0usize;

    while offset < bytes.len() {
        let end = (offset + MAX_CHUNK_SIZE).min(bytes.len());
        let chunk = &bytes[offset..end];
        let is_last = end == bytes.len();
        let mut chunk_opts = if first {
            opts.option_strings()
        } else {
            let mut follow = Vec::new();
            if opts.quiet > 0 {
                follow.push(format!("q={}", opts.quiet));
            }
            if opts.action == super::protocol::FRAME {
                follow.push("a=f".to_string());
            }
            follow
        };
        if !(first && is_last) {
            chunk_opts.push(if is_last {
                "m=0".to_string()
            } else {
                "m=1".to_string()
            });
        }
        let refs: Vec<&str> = chunk_opts.iter().map(String::as_str).collect();
        out.push_str(&kitty_graphics(chunk, &refs));
        first = false;
        offset = end;
    }
    Ok(out)
}

/// Writes Kitty graphics from an on-disk image file using file transmission.
pub fn encode_graphics_file(path: impl AsRef<Path>, opts: &Options) -> Result<String, EncodeError> {
    let path = path.as_ref();
    let mut opts = opts.clone();
    opts.file = path.display().to_string();
    opts.transmission = FILE;
    let opt_strings = opts.option_strings();
    let opt_refs: Vec<&str> = opt_strings.iter().map(String::as_str).collect();
    let data = STANDARD.encode(path.as_os_str().as_encoded_bytes());
    Ok(kitty_graphics(data.as_bytes(), &opt_refs))
}

/// Loads an image from disk and encodes it with direct transmission.
pub fn encode_graphics_from_path(
    path: impl AsRef<Path>,
    opts: &Options,
) -> Result<String, EncodeError> {
    let bytes = fs::read(path.as_ref()).map_err(EncodeError::Io)?;
    let img = image::load_from_memory(&bytes)
        .map_err(|e| EncodeError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, e)))?
        .to_rgba8();
    encode_graphics(&img, opts)
}

#[allow(dead_code)]
fn temp_dir_hint() -> Option<PathBuf> {
    std::env::temp_dir().into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    #[test]
    fn direct_transmission_wraps_apc() {
        let mut img = RgbaImage::new(2, 2);
        for px in img.pixels_mut() {
            *px = Rgba([255, 0, 0, 255]);
        }
        let seq = encode_graphics(
            &img,
            &Options {
                transmission: DIRECT,
                format: super::super::protocol::RGB,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(seq.starts_with("\x1b_G"));
        assert!(seq.ends_with("\x1b\\"));
        assert!(seq.contains("f=24"));
    }

    #[test]
    fn chunked_large_image() {
        let mut img = RgbaImage::new(100, 100);
        for px in img.pixels_mut() {
            *px = Rgba([255, 0, 0, 255]);
        }
        let seq = encode_graphics(
            &img,
            &Options {
                transmission: DIRECT,
                format: super::super::protocol::RGB,
                chunk: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(seq.matches("\x1b\\").count() >= 2);
        assert!(seq.contains("m=1"));
        assert!(seq.contains("m=0"));
    }

    #[test]
    fn shared_memory_transmission_emits_name() {
        if !shm::shared_memory_available() {
            return;
        }
        let mut img = RgbaImage::new(2, 2);
        for px in img.pixels_mut() {
            *px = Rgba([255, 0, 0, 255]);
        }
        let seq = encode_graphics(
            &img,
            &Options {
                transmission: SHARED_MEMORY,
                format: super::super::protocol::RGB,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(seq.contains("t=s"));
        assert!(seq.starts_with("\x1b_G"));
        assert!(seq.ends_with("\x1b\\"));
    }

    #[test]
    fn shared_memory_fallback_to_direct() {
        let mut img = RgbaImage::new(2, 2);
        for px in img.pixels_mut() {
            *px = Rgba([255, 0, 0, 255]);
        }
        let seq = encode_graphics_with_fallback(
            &img,
            &Options {
                transmission: SHARED_MEMORY,
                format: super::super::protocol::RGB,
                ..Default::default()
            },
        )
        .unwrap();
        if shm::shared_memory_available() {
            assert!(seq.contains("t=s"));
        } else {
            assert!(!seq.contains("t=s"));
        }
    }

    #[test]
    fn env_transmission_parsing() {
        // SAFETY: test runs single-threaded; no concurrent env access.
        unsafe {
            std::env::set_var("IMAGE_TRANSMISSION", "shm");
        }
        assert_eq!(kitty_transmission_from_env(), SHARED_MEMORY);
        unsafe {
            std::env::set_var("IMAGE_TRANSMISSION", "direct");
        }
        assert_eq!(kitty_transmission_from_env(), DIRECT);
        unsafe {
            std::env::remove_var("IMAGE_TRANSMISSION");
        }
    }
}
