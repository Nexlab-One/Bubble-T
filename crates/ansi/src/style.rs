//! Fluent SGR style builder.

use std::fmt;

use crate::color::{
    Color, background_color_string, foreground_color_string, underline_color_string,
};
use crate::sgr::{Attr, RESET_STYLE, attr_to_param};

/// ANSI underline style variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Underline {
    /// No underline.
    None = 0,
    /// Single underline.
    Single = 1,
    /// Double underline.
    Double = 2,
    /// Curly underline.
    Curly = 3,
    /// Dotted underline.
    Dotted = 4,
    /// Dashed underline.
    Dashed = 5,
}

/// An ANSI SGR style: a list of semicolon-separated parameter strings.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Style {
    parts: Vec<String>,
}

impl Style {
    /// Creates an empty style (renders as reset).
    pub fn new() -> Self {
        Self::default()
    }

    /// Builds a style from raw SGR attribute codes.
    pub fn from_attrs(attrs: &[Attr]) -> Self {
        let mut s = Self::new();
        for &a in attrs {
            s.parts.push(attr_to_param(a));
        }
        s
    }

    /// Appends the reset attribute.
    pub fn reset(mut self) -> Self {
        self.parts.push("0".into());
        self
    }

    /// Appends bold.
    pub fn bold(mut self) -> Self {
        self.parts.push("1".into());
        self
    }

    /// Appends faint.
    pub fn faint(mut self) -> Self {
        self.parts.push("2".into());
        self
    }

    /// Appends or clears italic.
    pub fn italic(mut self, on: bool) -> Self {
        self.parts.push(if on { "3" } else { "23" }.into());
        self
    }

    /// Appends or clears underline.
    pub fn underline(mut self, on: bool) -> Self {
        self.parts.push(if on { "4" } else { "24" }.into());
        self
    }

    /// Appends an underline style variant.
    pub fn underline_style(mut self, u: Underline) -> Self {
        match u {
            Underline::None => self.underline(false),
            Underline::Single => self.underline(true),
            Underline::Double => {
                self.parts.push("4:2".into());
                self
            }
            Underline::Curly => {
                self.parts.push("4:3".into());
                self
            }
            Underline::Dotted => {
                self.parts.push("4:4".into());
                self
            }
            Underline::Dashed => {
                self.parts.push("4:5".into());
                self
            }
        }
    }

    /// Appends or clears slow blink.
    pub fn blink(mut self, on: bool) -> Self {
        self.parts.push(if on { "5" } else { "25" }.into());
        self
    }

    /// Appends or clears reverse video.
    pub fn reverse(mut self, on: bool) -> Self {
        self.parts.push(if on { "7" } else { "27" }.into());
        self
    }

    /// Appends or clears strikethrough.
    pub fn strikethrough(mut self, on: bool) -> Self {
        self.parts.push(if on { "9" } else { "29" }.into());
        self
    }

    /// Appends normal intensity (clears bold/faint).
    pub fn normal(mut self) -> Self {
        self.parts.push("22".into());
        self
    }

    /// Appends a foreground color, or default when `None`.
    pub fn foreground_color(mut self, color: Option<Color>) -> Self {
        match color {
            None => self.parts.push("39".into()),
            Some(c) => self.parts.push(foreground_color_string(c)),
        }
        self
    }

    /// Appends a background color, or default when `None`.
    pub fn background_color(mut self, color: Option<Color>) -> Self {
        match color {
            None => self.parts.push("49".into()),
            Some(c) => self.parts.push(background_color_string(c)),
        }
        self
    }

    /// Appends an underline color, or default when `None`.
    pub fn underline_color(mut self, color: Option<Color>) -> Self {
        match color {
            None => self.parts.push("59".into()),
            Some(c) => self.parts.push(underline_color_string(c)),
        }
        self
    }

    /// Wraps `text` with this style and a trailing reset.
    pub fn styled(&self, text: &str) -> String {
        if self.parts.is_empty() {
            return text.to_string();
        }
        format!("{}{text}{RESET_STYLE}", self)
    }
}

impl fmt::Display for Style {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.parts.is_empty() {
            return f.write_str(RESET_STYLE);
        }
        write!(f, "\x1b[{}m", self.parts.join(";"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::{IndexedColor, RgbColor};

    #[test]
    fn empty_is_reset() {
        assert_eq!(Style::new().to_string(), "\x1b[m");
    }

    #[test]
    fn bold_underline_indexed_fg() {
        let s = Style::new()
            .bold()
            .underline(true)
            .foreground_color(Some(IndexedColor(255).into()));
        assert_eq!(s.to_string(), "\x1b[1;4;38;5;255m");
    }

    #[test]
    fn rgb_black_fg() {
        let s = Style::new()
            .bold()
            .underline(true)
            .foreground_color(Some(RgbColor { r: 0, g: 0, b: 0 }.into()));
        assert_eq!(s.to_string(), "\x1b[1;4;38;2;0;0;0m");
    }

    #[test]
    fn nil_colors_reset_defaults() {
        let s = Style::new()
            .foreground_color(None)
            .background_color(None)
            .underline_color(None);
        assert_eq!(s.to_string(), "\x1b[39;49;59m");
    }
}
