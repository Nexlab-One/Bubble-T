//! ANSI sequence emission from styled cells.

use ansi::cursor::cursor_position;
use ansi::hyperlink::{reset_hyperlink, set_hyperlink};
use ansi::screen::{erase_character, repeat_previous_character};

use crate::buffer::Buffer;
use crate::cell::{Cell, blank_cell};
use crate::diff::diff_lines;
use crate::link::Link;
use crate::style::Style;

/// Terminal capabilities used to optimize ANSI emission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RenderCaps(u8);

impl RenderCaps {
    /// No optional optimizations.
    pub const NONE: Self = Self(0);
    /// Erase Character (ECH) support.
    pub const ECH: Self = Self(1 << 0);
    /// Repeat Previous Character (REP) support.
    pub const REP: Self = Self(1 << 1);
    /// Typical xterm-like terminal supporting ECH and REP.
    pub const XTERM: Self = Self(Self::ECH.0 | Self::REP.0);

    /// Returns whether `self` contains all bits in `cap`.
    pub fn contains(self, cap: Self) -> bool {
        self.0 & cap.0 == cap.0
    }
}

/// Tracks the active pen while emitting ANSI.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RenderPen {
    style: Style,
    link: Link,
}

impl RenderPen {
    /// Creates an empty pen.
    pub fn new() -> Self {
        Self::default()
    }

    /// Resets the pen to defaults.
    pub fn reset(&mut self) {
        self.style = Style::default();
        self.link = Link::default();
    }

    /// Returns the currently active style.
    pub fn style(&self) -> &Style {
        &self.style
    }

    /// Emits style and hyperlink changes needed to draw `cell`.
    pub fn update_for_cell(&mut self, cell: &Cell, out: &mut String) {
        if cell.style != self.style {
            out.push_str(&cell.style.diff_sequence(&self.style));
            self.style = cell.style.clone();
        }
        if cell.link != self.link {
            if cell.link.is_empty() {
                out.push_str(&reset_hyperlink(&[]));
            } else if cell.link.params.is_empty() {
                out.push_str(&set_hyperlink(&cell.link.url, &[]));
            } else {
                let p = cell.link.params.as_str();
                out.push_str(&set_hyperlink(&cell.link.url, &[p]));
            }
            self.link = cell.link.clone();
        }
    }
}

/// Renders a contiguous run of cells on one row to ANSI.
///
/// `line` is the full row; `start_x` and `end_x` are half-open column bounds.
pub fn render_line(line: &[Option<Cell>], start_x: i32, end_x: i32, pen: &mut RenderPen) -> String {
    render_line_with_caps(line, start_x, end_x, pen, RenderCaps::NONE)
}

/// Like [`render_line`] but may emit ECH/REP when `caps` allows.
pub fn render_line_with_caps(
    line: &[Option<Cell>],
    start_x: i32,
    end_x: i32,
    pen: &mut RenderPen,
    caps: RenderCaps,
) -> String {
    let mut out = String::new();
    let width = line.len();
    let mut x = start_x.max(0) as usize;
    let end = (end_x.max(0) as usize).min(width);

    while x < end {
        let cell = line
            .get(x)
            .and_then(|slot| slot.as_ref())
            .cloned()
            .unwrap_or_else(blank_cell);

        if cell.is_empty() {
            x += 1;
            continue;
        }

        if caps.contains(RenderCaps::ECH) || caps.contains(RenderCaps::REP) {
            let run = run_length(&line[x..end.min(x + width)], &cell);
            if run > 1 {
                if caps.contains(RenderCaps::ECH)
                    && cell.is_blank()
                    && run > erase_character(run as i32).len()
                {
                    pen.update_for_cell(&cell, &mut out);
                    out.push_str(&erase_character(run as i32));
                    x += run;
                    continue;
                }
                if caps.contains(RenderCaps::REP)
                    && cell.comb.is_empty()
                    && cell.ch.is_some_and(|c| c.is_ascii())
                    && run > repeat_previous_character(run as i32).len()
                {
                    pen.update_for_cell(&cell, &mut out);
                    out.push_str(&cell.content());
                    if run > 1 {
                        out.push_str(&repeat_previous_character((run - 1) as i32));
                    }
                    x += run;
                    continue;
                }
            }
        }

        pen.update_for_cell(&cell, &mut out);
        out.push_str(&cell.content());
        x += cell.width.max(1) as usize;
    }

    out
}

fn run_length(line: &[Option<Cell>], sample: &Cell) -> usize {
    let mut n = 0usize;
    for slot in line {
        let cell = slot.as_ref().cloned().unwrap_or_else(blank_cell);
        if !cells_match(&cell, sample) {
            break;
        }
        n += cell.width.max(1) as usize;
        if n > 1 && cell.width > 1 {
            break;
        }
    }
    n.max(1)
}

fn cells_match(a: &Cell, b: &Cell) -> bool {
    a.ch == b.ch && a.comb == b.comb && a.style == b.style && a.link == b.link
}

/// Renders all differences between `old` and `new`, moving the cursor as needed.
pub fn render(
    old: &Buffer,
    new: &Buffer,
    cursor_x: i32,
    cursor_y: i32,
    pen: &mut RenderPen,
) -> String {
    render_with_caps(old, new, cursor_x, cursor_y, pen, RenderCaps::NONE)
}

/// Like [`render`] with optional ECH/REP optimizations.
pub fn render_with_caps(
    old: &Buffer,
    new: &Buffer,
    cursor_x: i32,
    cursor_y: i32,
    pen: &mut RenderPen,
    caps: RenderCaps,
) -> String {
    let diffs = diff_lines(old, new);
    let mut out = String::new();
    let mut cx = cursor_x;
    let mut cy = cursor_y;

    for diff in diffs {
        if diff.y != cy || diff.start_x != cx {
            out.push_str(&cursor_position(diff.y + 1, diff.start_x + 1));
            cy = diff.y;
        }

        let line = new
            .lines
            .get(diff.y as usize)
            .map(|l| l.as_slice())
            .unwrap_or(&[]);
        out.push_str(&render_line_with_caps(
            line,
            diff.start_x,
            diff.end_x,
            pen,
            caps,
        ));
        cx = diff.end_x;
    }

    out
}

/// Convenience wrapper that starts from an uninitialized cursor (`-1`, `-1`).
pub fn render_from_home(old: &Buffer, new: &Buffer) -> String {
    let mut pen = RenderPen::new();
    render(old, new, -1, -1, &mut pen)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::Buffer;
    use crate::cell::Cell;
    use crate::style::AttrMask;

    #[test]
    fn render_line_emits_content() {
        let mut line = Buffer::new(3, 1).lines.remove(0);
        line[1] = Some(Cell::new('X', &[]));
        let mut pen = RenderPen::new();
        let out = render_line(&line, 1, 2, &mut pen);
        assert_eq!(out, "X");
    }

    #[test]
    fn render_diff_moves_cursor() {
        let old = Buffer::new(3, 1);
        let mut new = Buffer::new(3, 1);
        new.set_cell(2, 0, Some(Cell::new('Z', &[])));
        let out = render_from_home(&old, &new);
        assert!(out.contains("\x1b["));
        assert!(out.contains('Z'));
        let _ = old;
    }

    #[test]
    fn render_bold_style_sequence() {
        let mut line = Buffer::new(1, 1).lines.remove(0);
        let mut cell = Cell::new('A', &[]);
        cell.style.attrs = AttrMask::BOLD;
        line[0] = Some(cell);
        let mut pen = RenderPen::new();
        let out = render_line(&line, 0, 1, &mut pen);
        assert!(out.contains("\x1b["));
        assert!(out.ends_with('A'));
    }

    #[test]
    fn render_rep_for_repeated_ascii() {
        let mut line = Buffer::new(5, 1).lines.remove(0);
        for slot in line.iter_mut().take(5) {
            *slot = Some(Cell::new('x', &[]));
        }
        let mut pen = RenderPen::new();
        let out = render_line_with_caps(&line, 0, 5, &mut pen, RenderCaps::REP);
        assert!(out.contains("\x1b[") && out.contains('b'));
    }
}
