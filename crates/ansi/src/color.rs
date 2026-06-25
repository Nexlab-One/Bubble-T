//! Terminal color types and SGR color sequence fragments.

use crate::parse::Param;
use crate::sgr::{ATTR_INDEXED_INTRODUCER, ATTR_RGB_INTRODUCER};

/// ANSI 3-bit or 4-bit color (0–15).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum BasicColor {
    /// Black.
    Black = 0,
    /// Red.
    Red = 1,
    /// Green.
    Green = 2,
    /// Yellow.
    Yellow = 3,
    /// Blue.
    Blue = 4,
    /// Magenta.
    Magenta = 5,
    /// Cyan.
    Cyan = 6,
    /// White.
    White = 7,
    /// Bright black.
    BrightBlack = 8,
    /// Bright red.
    BrightRed = 9,
    /// Bright green.
    BrightGreen = 10,
    /// Bright yellow.
    BrightYellow = 11,
    /// Bright blue.
    BrightBlue = 12,
    /// Bright magenta.
    BrightMagenta = 13,
    /// Bright cyan.
    BrightCyan = 14,
    /// Bright white.
    BrightWhite = 15,
}

/// ANSI 256-color (8-bit) palette index (0–255).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IndexedColor(pub u8);

/// 24-bit RGB color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RgbColor {
    /// Red component (0–255).
    pub r: u8,
    /// Green component (0–255).
    pub g: u8,
    /// Blue component (0–255).
    pub b: u8,
}

impl RgbColor {
    /// Creates an RGB color from a 24-bit hex value (`0xRRGGBB`).
    pub const fn from_hex(hex: u32) -> Self {
        Self {
            r: ((hex >> 16) & 0xFF) as u8,
            g: ((hex >> 8) & 0xFF) as u8,
            b: (hex & 0xFF) as u8,
        }
    }
}

/// Any color representable in a terminal SGR sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Color {
    /// 16-color ANSI palette entry.
    Basic(BasicColor),
    /// 256-color palette index.
    Indexed(IndexedColor),
    /// 24-bit true color.
    Rgb(RgbColor),
}

impl From<BasicColor> for Color {
    fn from(value: BasicColor) -> Self {
        Self::Basic(value)
    }
}

impl From<IndexedColor> for Color {
    fn from(value: IndexedColor) -> Self {
        Self::Indexed(value)
    }
}

impl From<RgbColor> for Color {
    fn from(value: RgbColor) -> Self {
        Self::Rgb(value)
    }
}

/// SGR parameter fragment for a foreground color.
pub fn foreground_color_string(color: Color) -> String {
    match color {
        Color::Basic(c) => basic_foreground(c).to_string(),
        Color::Indexed(c) => format!("38;5;{}", c.0),
        Color::Rgb(c) => format!("38;2;{};{};{}", c.r, c.g, c.b),
    }
}

/// SGR parameter fragment for a background color.
pub fn background_color_string(color: Color) -> String {
    match color {
        Color::Basic(c) => basic_background(c).to_string(),
        Color::Indexed(c) => format!("48;5;{}", c.0),
        Color::Rgb(c) => format!("48;2;{};{};{}", c.r, c.g, c.b),
    }
}

/// SGR parameter fragment for an underline color.
pub fn underline_color_string(color: Color) -> String {
    match color {
        Color::Basic(c) => format!("58;5;{}", c as u8),
        Color::Indexed(c) => format!("58;5;{}", c.0),
        Color::Rgb(c) => format!("58;2;{};{};{}", c.r, c.g, c.b),
    }
}

fn basic_foreground(c: BasicColor) -> &'static str {
    match c {
        BasicColor::Black => "30",
        BasicColor::Red => "31",
        BasicColor::Green => "32",
        BasicColor::Yellow => "33",
        BasicColor::Blue => "34",
        BasicColor::Magenta => "35",
        BasicColor::Cyan => "36",
        BasicColor::White => "37",
        BasicColor::BrightBlack => "90",
        BasicColor::BrightRed => "91",
        BasicColor::BrightGreen => "92",
        BasicColor::BrightYellow => "93",
        BasicColor::BrightBlue => "94",
        BasicColor::BrightMagenta => "95",
        BasicColor::BrightCyan => "96",
        BasicColor::BrightWhite => "97",
    }
}

fn basic_background(c: BasicColor) -> &'static str {
    match c {
        BasicColor::Black => "40",
        BasicColor::Red => "41",
        BasicColor::Green => "42",
        BasicColor::Yellow => "43",
        BasicColor::Blue => "44",
        BasicColor::Magenta => "45",
        BasicColor::Cyan => "46",
        BasicColor::White => "47",
        BasicColor::BrightBlack => "100",
        BasicColor::BrightRed => "101",
        BasicColor::BrightGreen => "102",
        BasicColor::BrightYellow => "103",
        BasicColor::BrightBlue => "104",
        BasicColor::BrightMagenta => "105",
        BasicColor::BrightCyan => "106",
        BasicColor::BrightWhite => "107",
    }
}

/// Maps a basic color to its indexed palette equivalent.
pub fn basic_to_indexed(c: BasicColor) -> IndexedColor {
    IndexedColor(c as u8)
}

/// Returns the 24-bit RGB value for an xterm 256-color index.
pub fn indexed_to_rgb(index: u8) -> RgbColor {
    match index {
        0..=15 => BASIC_RGB[index as usize],
        16..=231 => {
            let i = index - 16;
            let r = i / 36;
            let g = (i % 36) / 6;
            let b = i % 6;
            RgbColor {
                r: CUBE[r as usize],
                g: CUBE[g as usize],
                b: CUBE[b as usize],
            }
        }
        232..=255 => {
            let gray = 8 + (index - 232) as u16 * 10;
            RgbColor {
                r: gray as u8,
                g: gray as u8,
                b: gray as u8,
            }
        }
    }
}

/// Converts any [`Color`] to the nearest xterm 256-color palette index.
pub fn convert256(color: Color) -> IndexedColor {
    match color {
        Color::Indexed(i) => i,
        Color::Basic(c) => IndexedColor(c as u8),
        Color::Rgb(c) => convert256_rgb(c),
    }
}

/// Converts any [`Color`] to the nearest 16-color ANSI palette entry.
pub fn convert16(color: Color) -> BasicColor {
    match color {
        Color::Basic(c) => c,
        Color::Indexed(i) => nearest_basic(indexed_to_rgb(i.0)),
        Color::Rgb(c) => nearest_basic(c),
    }
}

/// Parses an extended SGR color from `params` starting at the `38`/`48`/`58` introducer.
///
/// Returns the parsed color and the number of parameters consumed (including the introducer).
pub fn read_style_color(params: &[i32]) -> (Option<Color>, usize) {
    if params.len() < 2 {
        return (None, 0);
    }

    let first = param_value(params[0]);
    if !matches!(first, 38 | 48 | 58) {
        return (None, 0);
    }

    let color_type = param_value(params[1]);
    match color_type {
        ATTR_RGB_INTRODUCER => read_style_color_rgb(params),
        ATTR_INDEXED_INTRODUCER => read_style_color_indexed(params),
        _ => (None, 1),
    }
}

fn read_style_color_indexed(params: &[i32]) -> (Option<Color>, usize) {
    if params.len() < 3 {
        return (None, 2);
    }
    let index = param_value(params[2]) as u8;
    (Some(Color::Indexed(IndexedColor(index))), 3)
}

fn read_style_color_rgb(params: &[i32]) -> (Option<Color>, usize) {
    // Colon-separated sub-parameters: 38:2:r:g:b
    if Param::from_raw(params[0]).has_more()
        && Param::from_raw(params[1]).has_more()
        && params.len() >= 5
    {
        let r = param_value(params[2]) as u8;
        let g = param_value(params[3]) as u8;
        let b = param_value(params[4]) as u8;
        return (Some(Color::Rgb(RgbColor { r, g, b })), 5);
    }

    // Legacy semicolon form: 38;2;r;g;b
    if params.len() < 5 {
        return (None, 2);
    }
    let r = param_value(params[2]) as u8;
    let g = param_value(params[3]) as u8;
    let b = param_value(params[4]) as u8;
    (Some(Color::Rgb(RgbColor { r, g, b })), 5)
}

fn param_value(raw: i32) -> i32 {
    Param::from_raw(raw).value(0)
}

fn convert256_rgb(c: RgbColor) -> IndexedColor {
    let q2c = [0x00, 0x5f, 0x87, 0xaf, 0xd7, 0xff];

    let qr = to6_cube(c.r);
    let qg = to6_cube(c.g);
    let qb = to6_cube(c.b);

    let cr = q2c[qr];
    let cg = q2c[qg];
    let cb = q2c[qb];

    let ci = (36 * qr + 6 * qg + qb) as u8;
    if cr == c.r && cg == c.g && cb == c.b {
        return IndexedColor(16 + ci);
    }

    let grey_avg = (u16::from(c.r) + u16::from(c.g) + u16::from(c.b)) / 3;
    let grey_idx = if grey_avg > 238 {
        23
    } else {
        (grey_avg.saturating_sub(3) / 10) as u8
    };
    let grey = 8 + grey_idx * 10;

    let cube = RgbColor {
        r: cr,
        g: cg,
        b: cb,
    };
    let gray = RgbColor {
        r: grey,
        g: grey,
        b: grey,
    };

    if color_dist_sq(c, cube) <= color_dist_sq(c, gray) {
        IndexedColor(16 + ci)
    } else {
        IndexedColor(232 + grey_idx)
    }
}

fn to6_cube(v: u8) -> usize {
    if v < 48 {
        0
    } else if v < 115 {
        1
    } else {
        ((v as u16 - 35) / 40) as usize
    }
}

fn color_dist_sq(a: RgbColor, b: RgbColor) -> u32 {
    let dr = i32::from(a.r) - i32::from(b.r);
    let dg = i32::from(a.g) - i32::from(b.g);
    let db = i32::from(a.b) - i32::from(b.b);
    (dr * dr + dg * dg + db * db) as u32
}

fn nearest_basic(c: RgbColor) -> BasicColor {
    let mut best = BasicColor::Black;
    let mut best_dist = u32::MAX;
    for i in 0..16u8 {
        let bc = basic_from_u8(i);
        let dist = color_dist_sq(c, BASIC_RGB[i as usize]);
        if dist < best_dist {
            best_dist = dist;
            best = bc;
        }
    }
    best
}

const fn basic_from_u8(v: u8) -> BasicColor {
    match v {
        0 => BasicColor::Black,
        1 => BasicColor::Red,
        2 => BasicColor::Green,
        3 => BasicColor::Yellow,
        4 => BasicColor::Blue,
        5 => BasicColor::Magenta,
        6 => BasicColor::Cyan,
        7 => BasicColor::White,
        8 => BasicColor::BrightBlack,
        9 => BasicColor::BrightRed,
        10 => BasicColor::BrightGreen,
        11 => BasicColor::BrightYellow,
        12 => BasicColor::BrightBlue,
        13 => BasicColor::BrightMagenta,
        14 => BasicColor::BrightCyan,
        _ => BasicColor::BrightWhite,
    }
}

const CUBE: [u8; 6] = [0x00, 0x5f, 0x87, 0xaf, 0xd7, 0xff];

const BASIC_RGB: [RgbColor; 16] = [
    RgbColor {
        r: 0x00,
        g: 0x00,
        b: 0x00,
    },
    RgbColor {
        r: 0x80,
        g: 0x00,
        b: 0x00,
    },
    RgbColor {
        r: 0x00,
        g: 0x80,
        b: 0x00,
    },
    RgbColor {
        r: 0x80,
        g: 0x80,
        b: 0x00,
    },
    RgbColor {
        r: 0x00,
        g: 0x00,
        b: 0x80,
    },
    RgbColor {
        r: 0x80,
        g: 0x00,
        b: 0x80,
    },
    RgbColor {
        r: 0x00,
        g: 0x80,
        b: 0x80,
    },
    RgbColor {
        r: 0xc0,
        g: 0xc0,
        b: 0xc0,
    },
    RgbColor {
        r: 0x80,
        g: 0x80,
        b: 0x80,
    },
    RgbColor {
        r: 0xff,
        g: 0x00,
        b: 0x00,
    },
    RgbColor {
        r: 0x00,
        g: 0xff,
        b: 0x00,
    },
    RgbColor {
        r: 0xff,
        g: 0xff,
        b: 0x00,
    },
    RgbColor {
        r: 0x00,
        g: 0x00,
        b: 0xff,
    },
    RgbColor {
        r: 0xff,
        g: 0x00,
        b: 0xff,
    },
    RgbColor {
        r: 0x00,
        g: 0xff,
        b: 0xff,
    },
    RgbColor {
        r: 0xff,
        g: 0xff,
        b: 0xff,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb_foreground() {
        assert_eq!(
            foreground_color_string(RgbColor { r: 255, g: 0, b: 0 }.into()),
            "38;2;255;0;0"
        );
    }

    #[test]
    fn indexed_background() {
        assert_eq!(background_color_string(IndexedColor(57).into()), "48;5;57");
    }

    #[test]
    fn indexed_to_rgb_cube() {
        let c = indexed_to_rgb(57);
        assert_eq!(c.r, 0x5f);
        assert_eq!(c.g, 0x00);
        assert_eq!(c.b, 0xff);
    }

    #[test]
    fn convert256_red() {
        assert_eq!(
            convert256(RgbColor { r: 255, g: 0, b: 0 }.into()),
            IndexedColor(196)
        );
    }

    #[test]
    fn read_style_color_rgb() {
        let params = [38, 2, 255, 128, 64];
        let (color, n) = read_style_color(&params);
        assert_eq!(n, 5);
        assert_eq!(
            color,
            Some(Color::Rgb(RgbColor {
                r: 255,
                g: 128,
                b: 64
            }))
        );
    }
}
