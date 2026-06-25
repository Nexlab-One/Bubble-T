//! Kitty graphics pixel decoder.

use std::io::Read;

use flate2::read::ZlibDecoder;
use image::RgbaImage;

use super::protocol::{PNG, RGB, RGBA};

/// Decodes Kitty graphics pixel payloads.
#[derive(Debug, Clone)]
pub struct Decoder {
    /// Decompress zlib payload first.
    pub decompress: bool,
    /// One of [`RGBA`], [`RGB`], or [`PNG`].
    pub format: i32,
    /// Image width (required for raw RGB/RGBA).
    pub width: i32,
    /// Image height (required for raw RGB/RGBA).
    pub height: i32,
}

impl Default for Decoder {
    fn default() -> Self {
        Self {
            decompress: false,
            format: RGBA,
            width: 0,
            height: 0,
        }
    }
}

impl Decoder {
    /// Decodes pixel bytes into an RGBA image.
    pub fn decode(&self, data: &[u8]) -> Result<RgbaImage, DecodeError> {
        let format = if self.format == 0 { RGBA } else { self.format };
        let mut reader: Box<dyn Read> = if self.decompress {
            Box::new(ZlibDecoder::new(data))
        } else {
            Box::new(data)
        };

        match format {
            RGBA | RGB => self.decode_raw(&mut reader, format == RGBA),
            PNG => {
                let img =
                    image::load_from_memory(data).map_err(|e| DecodeError::Png(e.to_string()))?;
                Ok(img.to_rgba8())
            }
            _ => Err(DecodeError::UnsupportedFormat(format)),
        }
    }

    fn decode_raw(&self, reader: &mut dyn Read, alpha: bool) -> Result<RgbaImage, DecodeError> {
        if self.width <= 0 || self.height <= 0 {
            return Err(DecodeError::MissingDimensions);
        }
        let mut img = RgbaImage::new(self.width as u32, self.height as u32);
        let mut buf = [0u8; 4];
        for y in 0..self.height {
            for x in 0..self.width {
                let n = if alpha { 4 } else { 3 };
                reader.read_exact(&mut buf[..n]).map_err(DecodeError::Io)?;
                img.put_pixel(
                    x as u32,
                    y as u32,
                    image::Rgba([buf[0], buf[1], buf[2], if alpha { buf[3] } else { 0xff }]),
                );
            }
        }
        Ok(img)
    }
}

/// Error returned when Kitty pixel decoding fails.
#[derive(Debug)]
pub enum DecodeError {
    /// Missing width/height for raw formats.
    MissingDimensions,
    /// PNG decode failure.
    Png(String),
    /// I/O failure while reading pixels.
    Io(std::io::Error),
    /// Unsupported pixel format.
    UnsupportedFormat(i32),
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingDimensions => f.write_str("kitty decode requires width and height"),
            Self::Png(e) => write!(f, "kitty png decode error: {e}"),
            Self::Io(e) => write!(f, "kitty decode io error: {e}"),
            Self::UnsupportedFormat(fmt) => write!(f, "unsupported kitty format: {fmt}"),
        }
    }
}

impl std::error::Error for DecodeError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics::kitty::KittyEncoder;

    #[test]
    fn roundtrip_rgba() {
        let mut img = RgbaImage::new(2, 1);
        img.put_pixel(0, 0, image::Rgba([1, 2, 3, 4]));
        img.put_pixel(1, 0, image::Rgba([5, 6, 7, 8]));
        let bytes = KittyEncoder::default().encode(&img).unwrap();
        let decoded = Decoder {
            width: 2,
            height: 1,
            ..Default::default()
        }
        .decode(&bytes)
        .unwrap();
        assert_eq!(decoded.get_pixel(0, 0), img.get_pixel(0, 0));
    }
}
