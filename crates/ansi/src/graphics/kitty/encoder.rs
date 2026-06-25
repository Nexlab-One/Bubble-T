//! Kitty graphics pixel encoder.

use std::io::Write;

use flate2::Compression;
use flate2::write::ZlibEncoder;
use image::codecs::png::PngEncoder;
use image::{ColorType, ExtendedColorType, ImageEncoder, RgbaImage};

use super::protocol::{PNG, RGB, RGBA};

/// Encodes image pixels for Kitty graphics transmission.
#[derive(Debug, Clone)]
pub struct Encoder {
    /// Use zlib compression on output.
    pub compress: bool,
    /// One of [`RGBA`], [`RGB`], or [`PNG`].
    pub format: i32,
}

impl Default for Encoder {
    fn default() -> Self {
        Self {
            compress: false,
            format: RGBA,
        }
    }
}

impl Encoder {
    /// Encodes `img` into raw (optionally compressed) pixel bytes.
    pub fn encode(&self, img: &RgbaImage) -> Result<Vec<u8>, EncodeError> {
        let format = if self.format == 0 { RGBA } else { self.format };
        let mut buf = Vec::new();
        {
            let mut writer: Box<dyn Write> = if self.compress {
                Box::new(ZlibEncoder::new(&mut buf, Compression::default()))
            } else {
                Box::new(&mut buf)
            };

            match format {
                RGBA | RGB => {
                    let (w, h) = img.dimensions();
                    for y in 0..h {
                        for x in 0..w {
                            let px = img.get_pixel(x, y);
                            if format == RGBA {
                                writer.write_all(&[px[0], px[1], px[2], px[3]])?;
                            } else {
                                writer.write_all(&[px[0], px[1], px[2]])?;
                            }
                        }
                    }
                }
                PNG => {
                    let mut png_buf = Vec::new();
                    PngEncoder::new(&mut png_buf).write_image(
                        img.as_raw(),
                        img.width(),
                        img.height(),
                        ExtendedColorType::from(ColorType::Rgba8),
                    )?;
                    writer.write_all(&png_buf)?;
                }
                _ => return Err(EncodeError::UnsupportedFormat(format)),
            }

            if self.compress {
                writer.flush()?;
            }
        }
        Ok(buf)
    }
}

/// Error returned when Kitty pixel encoding fails.
#[derive(Debug)]
pub enum EncodeError {
    /// I/O failure during encoding.
    Io(std::io::Error),
    /// PNG encoding failure.
    Png(String),
    /// Unsupported pixel format.
    UnsupportedFormat(i32),
}

impl std::fmt::Display for EncodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "kitty encode io error: {e}"),
            Self::Png(e) => write!(f, "kitty encode png error: {e}"),
            Self::UnsupportedFormat(fmt) => write!(f, "unsupported kitty format: {fmt}"),
        }
    }
}

impl std::error::Error for EncodeError {}

impl From<std::io::Error> for EncodeError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<image::ImageError> for EncodeError {
    fn from(value: image::ImageError) -> Self {
        Self::Png(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    #[test]
    fn encodes_rgba_pixels() {
        let mut img = RgbaImage::new(2, 2);
        img.put_pixel(0, 0, Rgba([255, 0, 0, 255]));
        let data = Encoder {
            format: RGB,
            ..Default::default()
        }
        .encode(&img)
        .unwrap();
        assert_eq!(data.len(), 2 * 2 * 3);
    }
}
