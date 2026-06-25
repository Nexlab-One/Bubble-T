//! Two-dimensional cell grid storage.

use crate::cell::{Cell, MAX_CELL_WIDTH, blank_cell, empty_cell};
use crate::geom::{Rectangle, rect};

/// One row of cells. `None` entries represent blank cells.
pub type Line = Vec<Option<Cell>>;

/// A width × height grid of styled cells.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Buffer {
    /// Row-major cell storage.
    pub lines: Vec<Line>,
}

impl Buffer {
    /// Creates and resizes a buffer to `width` × `height`.
    pub fn new(width: usize, height: usize) -> Self {
        let mut b = Self::default();
        b.resize(width, height);
        b
    }

    /// Returns the width in columns.
    pub fn width(&self) -> usize {
        self.lines.first().map_or(0, Line::len)
    }

    /// Returns the height in rows.
    pub fn height(&self) -> usize {
        self.lines.len()
    }

    /// Returns the buffer bounds as a rectangle.
    pub fn bounds(&self) -> Rectangle {
        rect(0, 0, self.width() as i32, self.height() as i32)
    }

    /// Resizes the buffer, preserving overlapping content.
    pub fn resize(&mut self, width: usize, height: usize) {
        if width == 0 || height == 0 {
            self.lines.clear();
            return;
        }

        if width > self.width() {
            let extra = width - self.width();
            for line in &mut self.lines {
                line.extend((0..extra).map(|_| None));
            }
        } else if width < self.width() {
            for line in &mut self.lines {
                line.truncate(width);
            }
        }

        if height > self.lines.len() {
            let start = self.lines.len();
            for _ in start..height {
                self.lines.push(vec![None; width]);
            }
        } else if height < self.lines.len() {
            self.lines.truncate(height);
        }
    }

    /// Returns an immutable reference to the cell at `(x, y)`.
    pub fn cell(&self, x: i32, y: i32) -> Option<&Cell> {
        if y < 0 || y as usize >= self.lines.len() {
            return None;
        }
        self.lines[y as usize].get(x as usize)?.as_ref()
    }

    /// Returns a blank cell reference when the slot is unset.
    pub fn cell_or_blank(&self, x: i32, y: i32) -> Cell {
        self.cell(x, y).cloned().unwrap_or_else(blank_cell)
    }

    /// Sets the cell at `(x, y)`, handling wide-character placeholders.
    pub fn set_cell(&mut self, x: i32, y: i32, cell: Option<Cell>) -> bool {
        if y < 0 || y as usize >= self.lines.len() {
            return false;
        }
        let line = &mut self.lines[y as usize];
        let width = line.len();
        if x < 0 || x as usize >= width {
            return false;
        }
        set_line_cell(line, x as usize, cell);
        true
    }

    /// Clears the entire buffer with blank cells.
    pub fn clear(&mut self) {
        self.clear_rect(self.bounds());
    }

    /// Clears `rect` with blank cells.
    pub fn clear_rect(&mut self, rect: Rectangle) {
        self.fill_rect(None, rect);
    }

    /// Fills `rect` with `cell` (or blank when `None`).
    pub fn fill_rect(&mut self, cell: Option<Cell>, rect: Rectangle) {
        let step = cell.as_ref().map_or(1, |c| c.width.max(1) as i32);
        for y in rect.min.y..rect.max.y {
            let mut x = rect.min.x;
            while x < rect.max.x {
                self.set_cell(x, y, cell.clone());
                x += step;
            }
        }
    }

    /// Returns a plain-text rendering (no ANSI), trimming trailing spaces per line.
    pub fn plain_string(&self) -> String {
        let mut out = String::new();
        for (i, line) in self.lines.iter().enumerate() {
            out.push_str(&line_string(line));
            if i + 1 < self.lines.len() {
                out.push_str("\r\n");
            }
        }
        out
    }
}

fn line_string(line: &Line) -> String {
    let mut s = String::new();
    for slot in line {
        match slot {
            None => s.push(' '),
            Some(c) if c.is_empty() => {}
            Some(c) => s.push_str(&c.content()),
        }
    }
    s.trim_end().to_string()
}

fn set_line_cell(line: &mut Line, x: usize, cell: Option<Cell>) {
    let width = line.len();
    if x >= width {
        return;
    }

    // Overwriting part of a wide cell blanks the whole cluster.
    if let Some(prev) = line[x].clone() {
        if prev.width > 1 {
            for j in 0..prev.width as usize {
                if x + j < width {
                    line[x + j] = Some(prev.to_blank());
                }
            }
        } else if prev.width == 0 {
            for j in 1..MAX_CELL_WIDTH as usize {
                if x >= j
                    && let Some(wide) = line[x - j].clone()
                    && wide.width > 1
                    && j < wide.width as usize
                {
                    for k in 0..wide.width as usize {
                        if x - j + k < width {
                            line[x - j + k] = Some(wide.to_blank());
                        }
                    }
                    break;
                }
            }
        }
    }

    let cell = cell.map(|mut c| {
        if c.width > 1 && x + c.width as usize > width {
            c = c.to_blank();
        }
        c
    });

    line[x] = cell.clone();

    if let Some(c) = cell
        && c.width > 1
    {
        for j in 1..c.width as usize {
            if x + j < width {
                line[x + j] = Some(empty_cell());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::Cell;

    #[test]
    fn resize_and_set() {
        let mut b = Buffer::new(3, 2);
        b.set_cell(1, 0, Some(Cell::new('x', &[])));
        assert_eq!(b.cell(1, 0).unwrap().content(), "x");
    }

    #[test]
    fn wide_cell_placeholders() {
        let mut b = Buffer::new(4, 1);
        b.set_cell(0, 0, Some(Cell::new('世', &[])));
        assert_eq!(b.cell(0, 0).unwrap().width, 2);
        assert!(b.cell(1, 0).unwrap().is_empty());
    }
}
