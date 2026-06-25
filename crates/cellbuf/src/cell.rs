//! A single styled grid cell.

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthChar;

use crate::link::Link;
use crate::style::Style;

/// Maximum width a terminal cell can occupy.
pub const MAX_CELL_WIDTH: u8 = 4;

/// A blank space cell with width 1 and no style.
pub fn blank_cell() -> Cell {
    Cell::blank()
}

/// An empty placeholder cell (width 0) used for wide-character continuations.
pub fn empty_cell() -> Cell {
    Cell::default()
}

/// A single cell in the terminal grid.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Cell {
    /// SGR styling applied to this cell.
    pub style: Style,
    /// OSC 8 hyperlink metadata.
    pub link: Link,
    /// Primary character; `None` when empty.
    pub ch: Option<char>,
    /// Combining characters appended to the primary character.
    pub comb: Vec<char>,
    /// Display width in terminal columns (0, 1, or 2).
    pub width: u8,
}

impl Cell {
    /// Creates a cell from a primary character and optional combining marks.
    pub fn new(ch: char, comb: &[char]) -> Self {
        let mut cell = Self {
            ch: Some(ch),
            comb: comb.to_vec(),
            ..Self::default()
        };
        cell.width = cell_text_width(ch, &cell.comb);
        cell
    }

    /// Creates a cell from the first grapheme cluster in `s`.
    pub fn from_grapheme(s: &str) -> Self {
        let cluster = s.graphemes(true).next().unwrap_or("");
        let mut chars = cluster.chars();
        let ch = chars.next();
        let comb: Vec<char> = chars.collect();
        match ch {
            None => Self::default(),
            Some(ch) => Self::new(ch, &comb),
        }
    }

    /// Creates a blank space cell with width 1.
    pub fn blank() -> Self {
        Self {
            ch: Some(' '),
            width: 1,
            ..Self::default()
        }
    }

    /// Clears content and styling.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Returns true when width is 0 and no content is present.
    pub fn is_empty(&self) -> bool {
        self.width == 0 && self.ch.is_none() && self.comb.is_empty()
    }

    /// Returns the visible text content without styling.
    pub fn content(&self) -> String {
        match self.ch {
            None => String::new(),
            Some(ch) if self.comb.is_empty() => ch.to_string(),
            Some(ch) => {
                let mut s = String::with_capacity(1 + self.comb.len());
                s.push(ch);
                for c in &self.comb {
                    s.push(*c);
                }
                s
            }
        }
    }

    /// Appends runes without changing width (for escape-sequence storage).
    pub fn append(&mut self, runes: impl IntoIterator<Item = char>) {
        for (i, r) in runes.into_iter().enumerate() {
            if i == 0 && self.ch.is_none() {
                self.ch = Some(r);
            } else {
                self.comb.push(r);
            }
        }
    }

    /// Returns true when this cell equals `other`.
    pub fn equal(&self, other: &Self) -> bool {
        self.width == other.width
            && self.ch == other.ch
            && self.comb == other.comb
            && self.style == other.style
            && self.link == other.link
    }

    /// Returns true when the cell is an unstyled blank space.
    pub fn is_blank(&self) -> bool {
        self.ch == Some(' ')
            && self.comb.is_empty()
            && self.style == Style::default()
            && self.link.is_empty()
    }

    /// Returns a blank cell preserving style and link.
    pub fn to_blank(&self) -> Self {
        Self {
            style: self.style.clone(),
            link: self.link.clone(),
            ch: Some(' '),
            comb: Vec::new(),
            width: 1,
        }
    }
}

fn cell_text_width(ch: char, comb: &[char]) -> u8 {
    let mut w = ch.width().unwrap_or(0);
    if w == 0 {
        return 0;
    }
    for c in comb {
        if c.width().unwrap_or(0) > 0 {
            break;
        }
    }
    let mut s = String::new();
    s.push(ch);
    for c in comb {
        s.push(*c);
    }
    w = s.chars().map(|c| c.width().unwrap_or(0)).sum();
    w.min(MAX_CELL_WIDTH as usize) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wide_char_width() {
        let c = Cell::new('世', &[]);
        assert_eq!(c.width, 2);
    }

    #[test]
    fn combining_mark() {
        let c = Cell::from_grapheme("e\u{0301}");
        assert_eq!(c.width, 1);
    }
}
