//! Line- and cell-level buffer diffing.

use crate::buffer::Buffer;
use crate::cell::Cell;

/// A contiguous run of changed cells on one row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineDiff {
    /// Row index.
    pub y: i32,
    /// First changed column (inclusive).
    pub start_x: i32,
    /// Last changed column (exclusive).
    pub end_x: i32,
}

/// Returns the rows that differ between `old` and `new`.
pub fn diff_lines(old: &Buffer, new: &Buffer) -> Vec<LineDiff> {
    let height = old.height().max(new.height());
    let width = old.width().max(new.width());
    let mut diffs = Vec::new();

    for y in 0..height {
        let mut start: Option<i32> = None;
        for x in 0..width {
            let a = normalized_cell(old, x as i32, y as i32);
            let b = normalized_cell(new, x as i32, y as i32);
            if !cells_equal(&a, &b) {
                if start.is_none() {
                    start = Some(x as i32);
                }
            } else if let Some(s) = start {
                diffs.push(LineDiff {
                    y: y as i32,
                    start_x: s,
                    end_x: x as i32,
                });
                start = None;
            }
        }
        if let Some(s) = start {
            diffs.push(LineDiff {
                y: y as i32,
                start_x: s,
                end_x: width as i32,
            });
        }
    }

    diffs
}

/// Returns true when every cell in the two buffers is equal.
pub fn buffers_equal(old: &Buffer, new: &Buffer) -> bool {
    diff_lines(old, new).is_empty()
}

fn normalized_cell(buf: &Buffer, x: i32, y: i32) -> Cell {
    buf.cell(x, y).cloned().unwrap_or_default()
}

fn cells_equal(a: &Cell, b: &Cell) -> bool {
    a.equal(b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::Buffer;
    use crate::cell::Cell;

    #[test]
    fn detects_single_cell_change() {
        let mut old = Buffer::new(3, 1);
        let mut new = Buffer::new(3, 1);
        old.set_cell(1, 0, Some(Cell::new('a', &[])));
        new.set_cell(1, 0, Some(Cell::new('b', &[])));
        let diffs = diff_lines(&old, &new);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].start_x, 1);
        assert_eq!(diffs[0].end_x, 2);
    }

    #[test]
    fn identical_buffers_empty_diff() {
        let a = Buffer::new(2, 2);
        let b = Buffer::new(2, 2);
        assert!(buffers_equal(&a, &b));
    }
}
