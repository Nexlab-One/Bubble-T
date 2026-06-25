//! Sixel image encoder.

use image::RgbaImage;

use crate::sixel::color::{SixelColor, write_color};
use crate::sixel::control::{CARRIAGE_RETURN, COLOR_INTRODUCER, LINE_BREAK};
use crate::sixel::palette::{SixelPalette, new_palette};
use crate::sixel::raster::write_raster;
use crate::sixel::repeat::write_repeat;

/// Encodes raster images as Sixel payload bytes (after the DCS `q`, before ST).
#[derive(Debug, Default, Clone, Copy)]
pub struct Encoder;

impl Encoder {
    /// Encodes `img` into Sixel data (raster attributes, palette, pixels).
    pub fn encode(&self, img: &RgbaImage) -> String {
        let (w, h) = img.dimensions();
        if w == 0 || h == 0 {
            return String::new();
        }

        let mut out = String::new();
        write_raster(&mut out, 1, 1, w as i32, h as i32);

        let palette = new_palette(img, crate::sixel::palette::MAX_COLORS);
        for (index, color) in palette.colors().iter().enumerate() {
            write_color(
                &mut out,
                index as i32,
                2,
                color.red as i32,
                color.green as i32,
                color.blue as i32,
            );
        }

        let mut builder = SixelBuilder::new(w as usize, h as usize, palette);
        for y in 0..h {
            for x in 0..w {
                let px = img.get_pixel(x, y);
                builder.set_color(
                    x as usize,
                    y as usize,
                    SixelColor::from_rgba8(px[0], px[1], px[2], px[3]),
                );
            }
        }
        out.push_str(&builder.generate_pixels());
        out
    }
}

struct SixelBuilder {
    width: usize,
    height: usize,
    palette: SixelPalette,
    bits: Vec<u64>,
    image_data: String,
    repeat_byte: u8,
    repeat_count: i32,
}

impl SixelBuilder {
    fn new(width: usize, height: usize, palette: SixelPalette) -> Self {
        let bands = band_height(height);
        let palette_len = palette.colors().len().max(1);
        let bit_len = bands * width * 6 * palette_len;
        let words = bit_len.div_ceil(64);
        Self {
            width,
            height,
            palette,
            bits: vec![0; words],
            image_data: String::new(),
            repeat_byte: 0,
            repeat_count: 0,
        }
    }

    fn set_color(&mut self, x: usize, y: usize, color: SixelColor) {
        let bands = band_height(self.height);
        let palette_index = self.palette.color_index(color);
        let bit =
            bands * self.width * 6 * palette_index + (y / 6) * self.width * 6 + x * 6 + (y % 6);
        set_bit(&mut self.bits, bit);
    }

    fn generate_pixels(&mut self) -> String {
        self.image_data.clear();
        let bands = band_height(self.height);
        let palette_len = self.palette.colors().len();

        for band_y in 0..bands {
            if band_y > 0 {
                self.write_control(LINE_BREAK);
            }
            let mut wrote_color = false;
            for palette_index in 0..palette_len {
                let color = self.palette.colors()[palette_index];
                if color.alpha < 1 {
                    continue;
                }
                let first = bands * self.width * 6 * palette_index + band_y * self.width * 6;
                let next = first + self.width * 6;
                if !any_set(&self.bits, first, next) {
                    continue;
                }
                if wrote_color {
                    self.write_control(CARRIAGE_RETURN);
                }
                wrote_color = true;
                self.write_control(COLOR_INTRODUCER);
                self.image_data.push_str(&palette_index.to_string());
                let mut x = 0;
                while x < self.width {
                    let bit = first + x * 6;
                    let word = word_at(&self.bits, bit);
                    let p1 = byte(((word & 63) as u8).wrapping_add(b'?'));
                    let p2 = byte((((word >> 6) & 63) as u8).wrapping_add(b'?'));
                    let p3 = byte((((word >> 12) & 63) as u8).wrapping_add(b'?'));
                    let p4 = byte((((word >> 18) & 63) as u8).wrapping_add(b'?'));
                    self.write_image(p1);
                    if x + 1 >= self.width {
                        break;
                    }
                    self.write_image(p2);
                    if x + 2 >= self.width {
                        break;
                    }
                    self.write_image(p3);
                    if x + 3 >= self.width {
                        break;
                    }
                    self.write_image(p4);
                    x += 4;
                }
            }
        }
        self.write_control(LINE_BREAK);
        self.image_data.clone()
    }

    fn write_image(&mut self, r: u8) {
        if r == self.repeat_byte {
            self.repeat_count += 1;
            return;
        }
        self.flush_repeats();
        self.repeat_byte = r;
        self.repeat_count = 1;
    }

    fn write_control(&mut self, r: u8) {
        self.flush_repeats();
        self.repeat_byte = 0;
        self.repeat_count = 0;
        self.image_data.push(char::from(r));
    }

    fn flush_repeats(&mut self) {
        if self.repeat_count == 0 {
            return;
        }
        if self.repeat_count > 3 {
            write_repeat(&mut self.image_data, self.repeat_count, self.repeat_byte);
        } else {
            for _ in 0..self.repeat_count {
                self.image_data.push(char::from(self.repeat_byte));
            }
        }
    }
}

fn band_height(height: usize) -> usize {
    height.div_ceil(6)
}

fn set_bit(bits: &mut [u64], bit: usize) {
    let word = bit / 64;
    let offset = bit % 64;
    if let Some(w) = bits.get_mut(word) {
        *w |= 1u64 << offset;
    }
}

fn any_set(bits: &[u64], first: usize, next: usize) -> bool {
    (first..next).any(|b| {
        let word = b / 64;
        let offset = b % 64;
        bits.get(word).is_some_and(|w| w & (1u64 << offset) != 0)
    })
}

fn word_at(bits: &[u64], bit: usize) -> u64 {
    let word = bit / 64;
    let offset = bit % 64;
    let mut v = bits.get(word).copied().unwrap_or(0);
    if offset > 0 {
        v >>= offset;
        if let Some(next) = bits.get(word + 1) {
            v |= next << (64 - offset);
        }
    }
    v
}

fn byte(v: u8) -> u8 {
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    #[test]
    fn encodes_solid_image() {
        let mut img = RgbaImage::new(3, 6);
        for px in img.pixels_mut() {
            *px = Rgba([255, 0, 0, 255]);
        }
        let payload = Encoder.encode(&img);
        assert!(payload.starts_with('"'));
        assert!(payload.contains('#'));
    }
}
