//! Per-cell SGR styling.

use ansi::color::{Color, read_style_color};
use ansi::parse::{Param, has_more};

use crate::link::Link;
use ansi::sgr::RESET_STYLE;
use ansi::style::{Style as AnsiStyle, Underline};

/// Bitmask of text attributes that change appearance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct AttrMask(u8);

impl AttrMask {
    /// Bold / increased intensity.
    pub const BOLD: Self = Self(1 << 0);
    /// Faint / decreased intensity.
    pub const FAINT: Self = Self(1 << 1);
    /// Italic.
    pub const ITALIC: Self = Self(1 << 2);
    /// Slow blink.
    pub const SLOW_BLINK: Self = Self(1 << 3);
    /// Rapid blink.
    pub const RAPID_BLINK: Self = Self(1 << 4);
    /// Reverse video.
    pub const REVERSE: Self = Self(1 << 5);
    /// Conceal / hidden.
    pub const CONCEAL: Self = Self(1 << 6);
    /// Strikethrough.
    pub const STRIKETHROUGH: Self = Self(1 << 7);

    fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }

    fn remove(&mut self, other: Self) {
        self.0 &= !other.0;
    }
}

/// Underline style carried by a cell.
pub type UnderlineStyle = Underline;

/// SGR styling state for a cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Style {
    /// Foreground color.
    pub fg: Option<Color>,
    /// Background color.
    pub bg: Option<Color>,
    /// Underline color.
    pub ul: Option<Color>,
    /// Attribute bitmask.
    pub attrs: AttrMask,
    /// Underline style variant.
    pub ul_style: UnderlineStyle,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            fg: None,
            bg: None,
            ul: None,
            attrs: AttrMask(0),
            ul_style: Underline::None,
        }
    }
}

impl Style {
    /// Clears all styling to defaults.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Returns true when no colors or attributes are set.
    pub fn is_empty(&self) -> bool {
        self.fg.is_none()
            && self.bg.is_none()
            && self.ul.is_none()
            && self.attrs.0 == 0
            && self.ul_style == Underline::None
    }

    /// Returns true when only non-visual attributes are set (safe for blank cells).
    pub fn is_clear(&self) -> bool {
        self.ul_style == Underline::None
            && self.fg.is_none()
            && self.bg.is_none()
            && self.ul.is_none()
            && self.attrs.0
                & !(AttrMask::BOLD.0
                    | AttrMask::FAINT.0
                    | AttrMask::ITALIC.0
                    | AttrMask::SLOW_BLINK.0
                    | AttrMask::RAPID_BLINK.0)
                == 0
    }

    /// Builds the ANSI sequence that applies this style.
    pub fn sequence(&self) -> String {
        if self.is_empty() {
            return RESET_STYLE.to_string();
        }

        let mut s = AnsiStyle::new();
        if self.attrs.contains(AttrMask::BOLD) {
            s = s.bold();
        }
        if self.attrs.contains(AttrMask::FAINT) {
            s = s.faint();
        }
        if self.attrs.contains(AttrMask::ITALIC) {
            s = s.italic(true);
        }
        if self.attrs.contains(AttrMask::SLOW_BLINK) {
            s = s.blink(true);
        }
        if self.attrs.contains(AttrMask::RAPID_BLINK) {
            s = s.blink(true);
        }
        if self.attrs.contains(AttrMask::REVERSE) {
            s = s.reverse(true);
        }
        if self.attrs.contains(AttrMask::CONCEAL) {
            s = s.reverse(false);
        }
        if self.attrs.contains(AttrMask::STRIKETHROUGH) {
            s = s.strikethrough(true);
        }
        if self.ul_style != Underline::None {
            s = s.underline_style(self.ul_style);
        }
        s = s.foreground_color(self.fg);
        s = s.background_color(self.bg);
        s = s.underline_color(self.ul);
        s.to_string()
    }

    /// Builds the minimal SGR sequence to transition from `other` to `self`.
    pub fn diff_sequence(&self, other: &Self) -> String {
        if other.is_empty() {
            return self.sequence();
        }

        let mut s = AnsiStyle::new();
        if self.fg != other.fg {
            s = s.foreground_color(self.fg);
        }
        if self.bg != other.bg {
            s = s.background_color(self.bg);
        }
        if self.ul != other.ul {
            s = s.underline_color(self.ul);
        }
        if self.attrs != other.attrs {
            if self.attrs.contains(AttrMask::BOLD) != other.attrs.contains(AttrMask::BOLD) {
                s = if self.attrs.contains(AttrMask::BOLD) {
                    s.bold()
                } else {
                    s.normal()
                };
            }
            if self.attrs.contains(AttrMask::ITALIC) != other.attrs.contains(AttrMask::ITALIC) {
                s = s.italic(self.attrs.contains(AttrMask::ITALIC));
            }
            if self.attrs.contains(AttrMask::REVERSE) != other.attrs.contains(AttrMask::REVERSE) {
                s = s.reverse(self.attrs.contains(AttrMask::REVERSE));
            }
            if self.attrs.contains(AttrMask::STRIKETHROUGH)
                != other.attrs.contains(AttrMask::STRIKETHROUGH)
            {
                s = s.strikethrough(self.attrs.contains(AttrMask::STRIKETHROUGH));
            }
        }
        if self.ul_style != other.ul_style {
            s = s.underline_style(self.ul_style);
        }
        s.to_string()
    }
}

/// Applies SGR parameters to `style`.
pub fn read_style(params: &[i32], style: &mut Style) {
    if params.is_empty() {
        style.reset();
        return;
    }

    let mut i = 0usize;
    while i < params.len() {
        let param = Param::from_raw(params[i]).value(0);
        match param {
            0 => style.reset(),
            1 => style.attrs.insert(AttrMask::BOLD),
            2 => style.attrs.insert(AttrMask::FAINT),
            3 => style.attrs.insert(AttrMask::ITALIC),
            4 => {
                if i + 1 < params.len() && has_more(params, i) {
                    let next = Param::from_raw(params[i + 1]).value(0);
                    i += 1;
                    style.ul_style = match next {
                        0 => Underline::None,
                        1 => Underline::Single,
                        2 => Underline::Double,
                        3 => Underline::Curly,
                        4 => Underline::Dotted,
                        5 => Underline::Dashed,
                        _ => Underline::Single,
                    };
                } else {
                    style.ul_style = Underline::Single;
                }
            }
            5 => style.attrs.insert(AttrMask::SLOW_BLINK),
            6 => style.attrs.insert(AttrMask::RAPID_BLINK),
            7 => style.attrs.insert(AttrMask::REVERSE),
            8 => style.attrs.insert(AttrMask::CONCEAL),
            9 => style.attrs.insert(AttrMask::STRIKETHROUGH),
            22 => {
                style.attrs.remove(AttrMask::BOLD);
                style.attrs.remove(AttrMask::FAINT);
            }
            23 => style.attrs.remove(AttrMask::ITALIC),
            24 => style.ul_style = Underline::None,
            25 => {
                style.attrs.remove(AttrMask::SLOW_BLINK);
                style.attrs.remove(AttrMask::RAPID_BLINK);
            }
            27 => style.attrs.remove(AttrMask::REVERSE),
            28 => style.attrs.remove(AttrMask::CONCEAL),
            29 => style.attrs.remove(AttrMask::STRIKETHROUGH),
            30..=37 => style.fg = Some(basic_color(param as u8 - 30).into()),
            38 => {
                let (color, n) = read_style_color(&params[i..]);
                if let Some(c) = color {
                    style.fg = Some(c);
                }
                if n > 0 {
                    i += n - 1;
                }
            }
            39 => style.fg = None,
            40..=47 => style.bg = Some(basic_color(param as u8 - 40).into()),
            48 => {
                let (color, n) = read_style_color(&params[i..]);
                if let Some(c) = color {
                    style.bg = Some(c);
                }
                if n > 0 {
                    i += n - 1;
                }
            }
            49 => style.bg = None,
            58 => {
                let (color, n) = read_style_color(&params[i..]);
                if let Some(c) = color {
                    style.ul = Some(c);
                }
                if n > 0 {
                    i += n - 1;
                }
            }
            59 => style.ul = None,
            90..=97 => style.fg = Some(basic_color(param as u8 - 90 + 8).into()),
            100..=107 => style.bg = Some(basic_color(param as u8 - 100 + 8).into()),
            _ => {}
        }
        i += 1;
    }
}

/// Parses an OSC 8 hyperlink payload into `link`.
pub fn read_link(data: &[u8], link: &mut Link) {
    let parts: Vec<&[u8]> = data.split(|&b| b == b';').collect();
    if parts.len() != 3 {
        return;
    }
    link.params = String::from_utf8_lossy(parts[1]).into_owned();
    link.url = String::from_utf8_lossy(parts[2]).into_owned();
}

fn basic_color(v: u8) -> ansi::color::BasicColor {
    use ansi::color::BasicColor::*;
    match v {
        0 => Black,
        1 => Red,
        2 => Green,
        3 => Yellow,
        4 => Blue,
        5 => Magenta,
        6 => Cyan,
        7 => White,
        8 => BrightBlack,
        9 => BrightRed,
        10 => BrightGreen,
        11 => BrightYellow,
        12 => BrightBlue,
        13 => BrightMagenta,
        14 => BrightCyan,
        _ => BrightWhite,
    }
}
