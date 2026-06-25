//! Sixel image decoder.

use image::RgbaImage;

use crate::sixel::color::decode_color;
use crate::sixel::control::{
    CARRIAGE_RETURN, COLOR_INTRODUCER, LINE_BREAK, RASTER_ATTRIBUTE, REPEAT_INTRODUCER,
};
use crate::sixel::palette_default::default_palette;
use crate::sixel::raster::decode_raster;
use crate::sixel::repeat::decode_repeat;

/// Error returned when Sixel data cannot be decoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// Invalid raster attributes.
    InvalidRaster,
    /// Invalid palette color command.
    InvalidColor,
    /// Invalid RLE repeat command.
    InvalidRepeat,
    /// Unexpected end of data.
    UnexpectedEof,
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidRaster => write!(f, "invalid raster attributes"),
            Self::InvalidColor => write!(f, "invalid sixel color"),
            Self::InvalidRepeat => write!(f, "invalid sixel repeat"),
            Self::UnexpectedEof => write!(f, "unexpected end of sixel data"),
        }
    }
}

impl std::error::Error for DecodeError {}

/// Decodes Sixel payload bytes into an RGBA image.
#[derive(Debug, Default, Clone, Copy)]
pub struct Decoder;

impl Decoder {
    /// Decodes Sixel data from `data`.
    pub fn decode(&self, data: &[u8]) -> Result<RgbaImage, DecodeError> {
        let mut pos = 0usize;
        let mut width = 0i32;
        let mut height = 0i32;

        if data.first() == Some(&RASTER_ATTRIBUTE) {
            let (raster, n) = decode_raster(data).map_err(|_| DecodeError::InvalidRaster)?;
            if n == 0 {
                return Err(DecodeError::InvalidRaster);
            }
            pos = n;
            width = raster.ph;
            height = raster.pv;
        }

        if width <= 0 || height <= 0 {
            let (w, h) = self.scan_size(&data[pos..]);
            width = w as i32;
            height = h as i32;
        }

        let width = width.max(0) as u32;
        let height = height.max(0) as u32;
        let mut img = RgbaImage::new(width.max(1), height.max(1));
        let mut palette = default_palette();
        let mut current_x = 0i32;
        let mut current_band_y = 0i32;
        let mut current_palette_index = 0usize;

        while pos < data.len() {
            let b = data[pos];
            pos += 1;
            let mut count = 1i32;
            match b {
                LINE_BREAK => {
                    current_band_y += 1;
                    current_x = 0;
                    continue;
                }
                CARRIAGE_RETURN => {
                    current_x = 0;
                    continue;
                }
                COLOR_INTRODUCER => {
                    let start = pos - 1;
                    while pos < data.len() {
                        let c = data[pos];
                        if !c.is_ascii_digit() && c != b';' {
                            break;
                        }
                        pos += 1;
                    }
                    let (color, _) =
                        decode_color(&data[start..pos]).map_err(|_| DecodeError::InvalidColor)?;
                    current_palette_index = color.pc.max(0) as usize;
                    if color.pu > 0 {
                        palette[current_palette_index.min(255)] = color.to_rgba();
                    }
                    continue;
                }
                REPEAT_INTRODUCER => {
                    let start = pos - 1;
                    while pos < data.len() {
                        let c = data[pos];
                        if !c.is_ascii_digit() && !(b'?'..=b'~').contains(&c) {
                            break;
                        }
                        pos += 1;
                    }
                    let (repeat, _) =
                        decode_repeat(&data[start..pos]).map_err(|_| DecodeError::InvalidRepeat)?;
                    count = repeat.count;
                    self.write_pixels(
                        &mut img,
                        &mut current_x,
                        current_band_y,
                        repeat.ch,
                        count,
                        palette[current_palette_index.min(255)],
                    );
                    continue;
                }
                b'?'..=b'~' => {
                    self.write_pixels(
                        &mut img,
                        &mut current_x,
                        current_band_y,
                        b,
                        count,
                        palette[current_palette_index.min(255)],
                    );
                }
                _ => {}
            }
        }

        Ok(img)
    }

    fn write_pixels(
        &self,
        img: &mut RgbaImage,
        x: &mut i32,
        band_y: i32,
        sixel: u8,
        count: i32,
        color: image::Rgba<u8>,
    ) {
        let masked = (sixel - b'?') & 63;
        for _ in 0..count {
            let base_x = *x;
            let mut y_offset = 0;
            let mut m = masked;
            while m != 0 {
                if m & 1 != 0 {
                    let px = base_x;
                    let py = band_y * 6 + y_offset;
                    if px >= 0 && py >= 0 {
                        let (w, h) = img.dimensions();
                        if (px as u32) < w && (py as u32) < h {
                            img.put_pixel(px as u32, py as u32, color);
                        }
                    }
                }
                y_offset += 1;
                m >>= 1;
            }
            *x += 1;
        }
    }

    /// Scans legacy sixel data without raster attributes to infer size.
    #[must_use]
    pub fn scan_size(&self, data: &[u8]) -> (usize, usize) {
        let mut max_width = 0usize;
        let mut band_count = 0usize;
        let mut current_width = 0usize;
        let mut new_band = true;

        let mut i = 0usize;
        while i < data.len() {
            let b = data[i];
            match b {
                LINE_BREAK => {
                    current_width = 0;
                    new_band = true;
                }
                CARRIAGE_RETURN => current_width = 0,
                REPEAT_INTRODUCER | b'?'..=b'~' => {
                    let mut count = 1usize;
                    if b == REPEAT_INTRODUCER
                        && let Ok((r, n)) = decode_repeat(&data[i..])
                    {
                        if n == 0 {
                            return (max_width, band_count * 6);
                        }
                        i += n - 1;
                        count = r.count.max(1) as usize;
                    }
                    current_width += count;
                    if new_band {
                        new_band = false;
                        band_count += 1;
                    }
                    max_width = max_width.max(current_width);
                }
                _ => {}
            }
            i += 1;
        }
        (max_width, band_count * 6)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sixel::Encoder;

    #[test]
    fn scan_size_cases() {
        let d = Decoder;
        assert_eq!(d.scan_size(b"~~~~~~-~~~~~~-"), (6, 12));
        assert_eq!(d.scan_size(b"~~~~~~-~~~~~~"), (6, 12));
        assert_eq!(d.scan_size(b""), (0, 0));
        assert_eq!(d.scan_size(b"~$~~$~~~$~~~~$~~~~~$~~~~~~"), (6, 6));
        assert_eq!(d.scan_size(b"??????"), (6, 6));
        assert_eq!(d.scan_size(b"??!20?"), (22, 6));
    }

    #[test]
    fn roundtrip_solid() {
        let mut img = RgbaImage::new(3, 6);
        for px in img.pixels_mut() {
            *px = image::Rgba([255, 0, 0, 255]);
        }
        let payload = Encoder.encode(&img);
        let decoded = Decoder.decode(payload.as_bytes()).unwrap();
        assert_eq!(decoded.dimensions(), img.dimensions());
        for y in 0..6 {
            for x in 0..3 {
                assert_eq!(decoded.get_pixel(x, y), img.get_pixel(x, y));
            }
        }
    }
}
