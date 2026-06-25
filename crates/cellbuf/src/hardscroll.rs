//! Scroll-optimized screen diffing via line hashing.
//!
//! Port of upstream `hardscroll.go` / `hashmap.go` heuristics: detect vertical
//! shifts between frames and emit scroll sequences instead of rewriting lines.

use ansi::screen::{scroll_down, scroll_up};

use crate::buffer::Buffer;
use crate::cell::Cell;

const NEW_INDEX: i32 = -1;

#[derive(Default, Clone, Copy)]
struct HashEntry {
    value: u64,
    old_count: i32,
    new_count: i32,
    old_index: i32,
    new_index: i32,
}

struct HashmapState {
    oldhash: Vec<u64>,
    newhash: Vec<u64>,
    oldnum: Vec<i32>,
    htab: Vec<HashEntry>,
}

/// Computes a simple rolling hash for a buffer row.
pub fn line_hash(buf: &Buffer, y: usize) -> u64 {
    let mut h = 0u64;
    let width = buf.width();
    for x in 0..width {
        let ch = buf
            .cell(x as i32, y as i32)
            .and_then(|c| c.ch)
            .unwrap_or(' ');
        h = h.wrapping_add(h << 5).wrapping_add(u64::from(ch as u32));
    }
    h
}

/// Attempts to scroll the current buffer to match `new` and append scroll ANSI.
///
/// Returns `true` when a scroll optimization was applied.
pub fn try_scroll_optimize(cur: &mut Buffer, new: &Buffer, pending: &mut String) -> bool {
    let height = cur.height().min(new.height());
    if height < 3 {
        return false;
    }

    let mut state = HashmapState {
        oldhash: (0..height).map(|y| line_hash(cur, y)).collect(),
        newhash: (0..height).map(|y| line_hash(new, y)).collect(),
        oldnum: vec![NEW_INDEX; height],
        htab: vec![HashEntry::default(); height * 2],
    };

    update_hashmap(cur, new, &mut state);

    // Pass 1 — scroll up
    let mut y = 0usize;
    while y < height {
        while y < height && (state.oldnum[y] == NEW_INDEX || state.oldnum[y] <= y as i32) {
            y += 1;
        }
        if y >= height {
            break;
        }

        let shift = state.oldnum[y] - y as i32;
        let start = y;
        y += 1;
        while y < height && state.oldnum[y] != NEW_INDEX && state.oldnum[y] - y as i32 == shift {
            y += 1;
        }
        let end = y - 1 + shift as usize;

        if shift > 0 && apply_scroll(cur, shift, start, end, height, pending) {
            scroll_old_hashes(&mut state.oldhash, shift, start, end);
            continue;
        }
    }

    // Pass 2 — scroll down
    if height > 0 {
        y = height - 1;
        while y > 0 {
            while y > 0 && (state.oldnum[y] == NEW_INDEX || state.oldnum[y] >= y as i32) {
                y -= 1;
            }
            if state.oldnum[y] == NEW_INDEX || state.oldnum[y] >= y as i32 {
                break;
            }

            let shift = state.oldnum[y] - y as i32;
            let end = y;
            while y > 0
                && state.oldnum[y - 1] != NEW_INDEX
                && state.oldnum[y - 1] - (y - 1) as i32 == shift
            {
                y -= 1;
            }
            let start = y + 1 - (-shift) as usize;

            if shift < 0 {
                apply_scroll(cur, shift, start, end, height, pending);
            }
        }
    }

    !pending.is_empty()
}

fn update_hashmap(cur: &Buffer, new: &Buffer, state: &mut HashmapState) {
    let height = state.oldnum.len();
    state.htab.fill(HashEntry::default());

    for (i, &hash) in state.oldhash.iter().enumerate().take(height) {
        let idx = state
            .htab
            .iter()
            .position(|e| e.value == 0 || e.value == hash)
            .unwrap_or(0);
        state.htab[idx].value = hash;
        state.htab[idx].old_count += 1;
        state.htab[idx].old_index = i as i32;
    }

    for (i, &hash) in state.newhash.iter().enumerate().take(height) {
        let idx = state
            .htab
            .iter()
            .position(|e| e.value == 0 || e.value == hash)
            .unwrap_or(0);
        state.htab[idx].value = hash;
        state.htab[idx].new_count += 1;
        state.htab[idx].new_index = i as i32;
        state.oldnum[i] = NEW_INDEX;
    }

    for entry in &state.htab {
        if entry.value == 0 {
            continue;
        }
        if entry.old_count == 1 && entry.new_count == 1 && entry.old_index != entry.new_index {
            state.oldnum[entry.new_index as usize] = entry.old_index;
        }
    }

    grow_hunks(cur, new, state);

    let mut i = 0usize;
    while i < height {
        while i < height && state.oldnum[i] == NEW_INDEX {
            i += 1;
        }
        if i >= height {
            break;
        }
        let start = i;
        let shift = state.oldnum[i] - i as i32;
        i += 1;
        while i < height && state.oldnum[i] != NEW_INDEX && state.oldnum[i] - i as i32 == shift {
            i += 1;
        }
        let size = i - start;
        if size < 3 || size + min(size / 8, 2) < abs_i32(shift) as usize {
            for slot in &mut state.oldnum[start..i] {
                *slot = NEW_INDEX;
            }
        }
    }

    grow_hunks(cur, new, state);
}

fn grow_hunks(cur: &Buffer, new: &Buffer, state: &mut HashmapState) {
    let height = state.oldnum.len();
    let mut back_limit = 0usize;
    let mut back_ref_limit = 0usize;
    let mut i = 0usize;

    while i < height && state.oldnum[i] == NEW_INDEX {
        i += 1;
    }

    while i < height {
        let start = i;
        let shift = state.oldnum[i] - i as i32;

        i = start + 1;
        while i < height && state.oldnum[i] != NEW_INDEX && state.oldnum[i] - i as i32 == shift {
            i += 1;
        }
        let end = i;

        while i < height && state.oldnum[i] == NEW_INDEX {
            i += 1;
        }
        let next_hunk = i;
        let mut forward_limit = i;
        let forward_ref_limit = if i >= height || state.oldnum[i] >= i as i32 {
            i
        } else {
            state.oldnum[i] as usize
        };

        let mut back = start as i32 - 1;
        let back_limit_i = if shift < 0 {
            back_ref_limit as i32 + (-shift)
        } else {
            back_limit as i32
        };
        while back >= back_limit_i {
            let back_u = back as usize;
            if state.newhash[back_u] == state.oldhash[(back + shift) as usize]
                || cost_effective(cur, new, state, back_u, (back + shift) as usize, shift < 0)
            {
                state.oldnum[back_u] = back + shift;
            } else {
                break;
            }
            back -= 1;
        }

        let mut fwd = end;
        if shift > 0 {
            forward_limit = forward_ref_limit.saturating_sub(shift as usize);
        }
        while fwd < forward_limit {
            if state.newhash[fwd] == state.oldhash[(fwd as i32 + shift) as usize]
                || cost_effective(
                    cur,
                    new,
                    state,
                    (fwd as i32 + shift) as usize,
                    fwd,
                    shift > 0,
                )
            {
                state.oldnum[fwd] = fwd as i32 + shift;
            } else {
                break;
            }
            fwd += 1;
        }

        back_limit = fwd;
        back_ref_limit = back_limit;
        if shift > 0 {
            back_ref_limit += shift as usize;
        }
        i = next_hunk;
    }
}

fn cost_effective(
    cur: &Buffer,
    new: &Buffer,
    state: &HashmapState,
    from: usize,
    to: usize,
    blank: bool,
) -> bool {
    if from == to {
        return false;
    }

    let new_from = {
        let idx = state.oldnum[from];
        if idx == NEW_INDEX { from as i32 } else { idx }
    };

    let cost_before = if blank {
        update_cost_blank(new, to)
    } else {
        update_cost(cur, new, to, to)
    } + update_cost(cur, new, new_from as usize, from);

    let cost_after = if new_from as usize == from {
        update_cost_blank(new, from)
    } else {
        update_cost(cur, new, new_from as usize, from)
    } + update_cost(cur, new, from, to);

    cost_before >= cost_after
}

fn update_cost(cur: &Buffer, new: &Buffer, from_y: usize, to_y: usize) -> i32 {
    let width = new.width();
    if width <= 1 {
        return 0;
    }
    let mut cost = 0;
    for x in 1..width {
        let a = line_cell(cur, from_y, x);
        let b = line_cell(new, to_y, x);
        if !a.equal(&b) {
            cost += 1;
        }
    }
    cost
}

fn update_cost_blank(new: &Buffer, to_y: usize) -> i32 {
    let width = new.width();
    if width <= 1 {
        return 0;
    }
    let mut cost = 0;
    for x in 1..width {
        let b = line_cell(new, to_y, x);
        if !b.is_blank() {
            cost += 1;
        }
    }
    cost
}

fn line_cell(buf: &Buffer, y: usize, x: usize) -> Cell {
    buf.cell(x as i32, y as i32).cloned().unwrap_or_default()
}

fn apply_scroll(
    cur: &mut Buffer,
    shift: i32,
    start: usize,
    end: usize,
    height: usize,
    pending: &mut String,
) -> bool {
    let n = shift.unsigned_abs() as usize;
    if n == 0 || end >= height {
        return false;
    }

    if shift > 0 {
        pending.push_str(&scroll_up(n as i32));
        for y in start..=end.saturating_sub(n) {
            let src = y + n;
            if src > end {
                break;
            }
            copy_row(cur, src, y);
        }
        for y in (end + 1).saturating_sub(n)..=end {
            clear_row(cur, y);
        }
    } else {
        pending.push_str(&scroll_down(n as i32));
        for y in (start..=end).rev() {
            if y < n {
                break;
            }
            copy_row(cur, y - n, y);
        }
        for y in start..start + n.min(end + 1) {
            clear_row(cur, y);
        }
    }

    true
}

fn copy_row(buf: &mut Buffer, src_y: usize, dst_y: usize) {
    let width = buf.width();
    for x in 0..width {
        let cell = buf.cell(x as i32, src_y as i32).cloned();
        buf.set_cell(x as i32, dst_y as i32, cell);
    }
}

fn clear_row(buf: &mut Buffer, y: usize) {
    let width = buf.width();
    for x in 0..width {
        buf.set_cell(x as i32, y as i32, None);
    }
}

fn scroll_old_hashes(hashes: &mut [u64], shift: i32, top: usize, bot: usize) {
    if hashes.is_empty() {
        return;
    }
    let n = shift.unsigned_abs() as usize;
    let size = bot - top + 1 - n;
    if shift > 0 {
        hashes.copy_within(top + n..top + n + size, top);
    } else if shift < 0 {
        hashes.copy_within(top..top + size, top - n);
    }
}

fn abs_i32(v: i32) -> i32 {
    v.abs()
}

fn min(a: usize, b: usize) -> usize {
    a.min(b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::Cell;

    #[test]
    fn detects_upward_scroll() {
        let mut cur = Buffer::new(4, 4);
        cur.set_cell(0, 0, Some(Cell::new('A', &[])));
        cur.set_cell(0, 1, Some(Cell::new('B', &[])));
        cur.set_cell(0, 2, Some(Cell::new('C', &[])));
        cur.set_cell(0, 3, Some(Cell::new('D', &[])));

        let mut new = Buffer::new(4, 4);
        new.set_cell(0, 0, Some(Cell::new('B', &[])));
        new.set_cell(0, 1, Some(Cell::new('C', &[])));
        new.set_cell(0, 2, Some(Cell::new('D', &[])));

        let mut pending = String::new();
        assert!(try_scroll_optimize(&mut cur, &new, &mut pending));
        assert!(pending.contains("\x1b["));
    }

    #[test]
    fn grow_hunks_extends_shift() {
        let mut cur = Buffer::new(5, 5);
        let mut new = Buffer::new(5, 5);
        for y in 0..5 {
            cur.set_cell(0, y, Some(Cell::new(char::from(b'A' + y as u8), &[])));
            new.set_cell(0, y, Some(Cell::new(char::from(b'B' + y as u8), &[])));
        }
        for y in 1..5 {
            new.set_cell(0, y - 1, cur.cell(0, y).cloned());
        }
        let mut pending = String::new();
        let _ = try_scroll_optimize(&mut cur, &new, &mut pending);
    }
}
