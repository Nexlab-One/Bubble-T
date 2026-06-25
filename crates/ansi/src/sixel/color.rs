//! Sixel color definitions and parsing.

use image::Rgba;

use crate::sixel::control::COLOR_INTRODUCER;
use crate::sixel::palette_default::default_palette_entry;

/// Error returned when a Sixel color command is invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodeColorError;

impl std::fmt::Display for DecodeColorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid sixel color")
    }
}

impl std::error::Error for DecodeColorError {}

/// A Sixel palette color entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Color {
    /// Palette index (0–255).
    pub pc: i32,
    /// Color system: 0 default, 1 HLS, 2 RGB.
    pub pu: i32,
    /// First component (hue 0–360 or red 0–100).
    pub px: i32,
    /// Second component (lightness/saturation or green 0–100).
    pub py: i32,
    /// Third component (saturation or blue 0–100).
    pub pz: i32,
}

impl Color {
    /// Converts this palette entry to RGBA.
    #[must_use]
    pub fn to_rgba(self) -> Rgba<u8> {
        match self.pu {
            1 => hls_to_rgba(self.px, self.py, self.pz),
            2 => rgb100_to_rgba(self.px, self.py, self.pz),
            _ => default_palette_entry(self.pc as usize),
        }
    }
}

/// Converts an 8-bit RGBA color to Sixel 0–100 channel values.
#[must_use]
pub(crate) fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> SixelColor {
    SixelColor {
        red: convert_channel(u32::from(r) * 0x101),
        green: convert_channel(u32::from(g) * 0x101),
        blue: convert_channel(u32::from(b) * 0x101),
        alpha: convert_channel(u32::from(a) * 0x101),
    }
}

/// Internal flat color used for palette quantization (channels 0–100).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct SixelColor {
    pub red: u32,
    pub green: u32,
    pub blue: u32,
    pub alpha: u32,
}

impl SixelColor {
    #[must_use]
    pub fn from_rgba8(r: u8, g: u8, b: u8, a: u8) -> Self {
        from_rgba(r, g, b, a)
    }
}

/// Writes `#pc` or `#pc;pu;px;py;pz` to `out`.
pub fn write_color(out: &mut String, pc: i32, pu: i32, px: i32, py: i32, pz: i32) {
    out.push(char::from(COLOR_INTRODUCER));
    out.push_str(&pc.to_string());
    if pu > 0 && pu <= 2 {
        out.push(';');
        out.push_str(&pu.to_string());
        out.push(';');
        out.push_str(&px.to_string());
        out.push(';');
        out.push_str(&py.to_string());
        out.push(';');
        out.push_str(&pz.to_string());
    }
}

/// Decodes a Sixel color command from `data` (starting with `#`).
pub fn decode_color(data: &[u8]) -> Result<(Color, usize), DecodeColorError> {
    if data.is_empty() || data[0] != COLOR_INTRODUCER || data.len() < 2 {
        return Err(DecodeColorError);
    }

    let mut values = [0i32; 5];
    let mut index = 0usize;
    let mut current = 0i32;
    let mut n = 1usize;

    while n < data.len() && index < 5 {
        let b = data[n];
        if b == b';' {
            values[index] = current;
            current = 0;
            index += 1;
            if index == 1 {
                // After Pu, remaining components share the same delimiter loop.
            }
        } else if b.is_ascii_digit() {
            current = current.saturating_mul(10) + i32::from(b - b'0');
        } else {
            break;
        }
        n += 1;
    }
    if index < 5 {
        values[index] = current;
    }

    Ok((
        Color {
            pc: values[0],
            pu: values[1],
            px: values[2],
            py: values[3],
            pz: values[4],
        },
        n,
    ))
}

pub(crate) fn convert_channel(c: u32) -> u32 {
    (c + 328) * 100 / 0xffff
}

fn palval(n: i32, a: i32, m: i32) -> u8 {
    ((n * a + m / 2) / m).clamp(0, 255) as u8
}

fn rgb100_to_rgba(r: i32, g: i32, b: i32) -> Rgba<u8> {
    Rgba([
        palval(r, 0xff, 100),
        palval(g, 0xff, 100),
        palval(b, 0xff, 100),
        0xff,
    ])
}

fn hls_to_rgba(h: i32, l: i32, s: i32) -> Rgba<u8> {
    let hf = h as f64;
    let lf = (l as f64 / 100.0).clamp(0.0, 1.0);
    let sf = (s as f64 / 100.0).clamp(0.0, 1.0);
    if sf <= f64::EPSILON {
        let v = (lf * 255.0).round() as u8;
        return Rgba([v, v, v, 0xff]);
    }
    let c = (1.0 - (2.0 * lf - 1.0).abs()) * sf;
    let x = c * (1.0 - ((hf / 60.0) % 2.0 - 1.0).abs());
    let m = lf - c / 2.0;
    let (rp, gp, bp) = match hf {
        h if (0.0..60.0).contains(&h) => (c, x, 0.0),
        h if (60.0..120.0).contains(&h) => (x, c, 0.0),
        h if (120.0..180.0).contains(&h) => (0.0, c, x),
        h if (180.0..240.0).contains(&h) => (0.0, x, c),
        h if (240.0..300.0).contains(&h) => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    Rgba([
        ((rp + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((gp + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((bp + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        0xff,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_rgb_color() {
        let (c, n) = decode_color(b"#1;2;100;50;25").unwrap();
        assert_eq!(c.pc, 1);
        assert_eq!(c.pu, 2);
        assert_eq!(c.px, 100);
        assert_eq!(c.py, 50);
        assert_eq!(c.pz, 25);
        assert_eq!(n, 14);
    }

    #[test]
    fn decode_index_only() {
        let (c, n) = decode_color(b"#7").unwrap();
        assert_eq!(c.pc, 7);
        assert_eq!(n, 2);
    }
}
