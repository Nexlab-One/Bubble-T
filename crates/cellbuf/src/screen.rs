//! Terminal screen with cursor tracking and scroll region.

use ansi::cursor::{cursor_position, cursor_up};
use ansi::screen::{erase_display, set_top_bottom_margins};

use crate::buffer::Buffer;
use crate::cell::blank_cell;
use crate::diff::diff_lines;
use crate::geom::Rectangle;
use crate::hardscroll::try_scroll_optimize;
use crate::link::Link;
use crate::render::{RenderCaps, RenderPen, render_with_caps};
use crate::style::Style;
use crate::tabstop::TabStops;

/// Cursor position and active pen on the screen.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScreenCursor {
    /// Column (0-based).
    pub x: i32,
    /// Row (0-based).
    pub y: i32,
    /// Active style at the cursor.
    pub style: Style,
    /// Active hyperlink at the cursor.
    pub link: Link,
}

impl ScreenCursor {
    /// Creates a cursor at the origin.
    pub fn new() -> Self {
        Self {
            x: -1,
            y: -1,
            ..Self::default()
        }
    }
}

/// Inclusive scroll region bounds (0-based rows).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollRegion {
    /// Top row (inclusive).
    pub top: i32,
    /// Bottom row (inclusive).
    pub bottom: i32,
}

/// Options controlling screen rendering behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenOptions {
    /// Use the alternate screen buffer.
    pub alt_screen: bool,
    /// Show the text cursor after rendering.
    pub show_cursor: bool,
    /// Optional terminal capabilities for ECH/REP optimizations.
    pub render_caps: RenderCaps,
}

impl Default for ScreenOptions {
    fn default() -> Self {
        Self {
            alt_screen: false,
            show_cursor: true,
            render_caps: RenderCaps::XTERM,
        }
    }
}

/// A terminal screen that diffs buffers and emits minimal ANSI updates.
#[derive(Debug, Clone)]
pub struct Screen {
    cur_buf: Buffer,
    new_buf: Buffer,
    cursor: ScreenCursor,
    scroll: Option<ScrollRegion>,
    opts: ScreenOptions,
    pending: String,
    force_clear: bool,
    pen: RenderPen,
    tabs: TabStops,
}

impl Screen {
    /// Creates a screen with the given dimensions.
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            cur_buf: Buffer::new(width, height),
            new_buf: Buffer::new(width, height),
            cursor: ScreenCursor::new(),
            scroll: None,
            opts: ScreenOptions::default(),
            pending: String::new(),
            force_clear: false,
            pen: RenderPen::new(),
            tabs: TabStops::default_stops(width),
        }
    }

    /// Returns tab stops for cursor movement optimizations.
    pub fn tabs(&self) -> &TabStops {
        &self.tabs
    }

    /// Returns a mutable reference to tab stops.
    pub fn tabs_mut(&mut self) -> &mut TabStops {
        &mut self.tabs
    }

    /// Returns a mutable reference to the target buffer for the next frame.
    pub fn buffer(&mut self) -> &mut Buffer {
        &mut self.new_buf
    }

    /// Returns the current cursor state.
    pub fn cursor(&self) -> &ScreenCursor {
        &self.cursor
    }

    /// Sets the scroll region (inclusive row bounds).
    pub fn set_scroll_region(&mut self, top: i32, bottom: i32) {
        self.scroll = Some(ScrollRegion { top, bottom });
        self.pending
            .push_str(&set_top_bottom_margins(top + 1, bottom + 1));
    }

    /// Clears the scroll region restriction.
    pub fn reset_scroll_region(&mut self) {
        self.scroll = None;
        self.pending.push_str(&set_top_bottom_margins(1, 0));
    }

    /// Returns the active scroll region, if any.
    pub fn scroll_region(&self) -> Option<ScrollRegion> {
        self.scroll
    }

    /// Resizes both buffers, preserving overlapping content.
    pub fn resize(&mut self, width: usize, height: usize) {
        self.cur_buf.resize(width, height);
        self.new_buf.resize(width, height);
        self.tabs.resize(width);
        self.force_clear = true;
    }

    /// Forces a full clear on the next render.
    pub fn redraw(&mut self) {
        self.force_clear = true;
    }

    /// Clears the target buffer.
    pub fn clear(&mut self) {
        self.new_buf.clear();
    }

    /// Clears a rectangle in the target buffer.
    pub fn clear_rect(&mut self, rect: Rectangle) {
        self.new_buf.clear_rect(rect);
    }

    /// Sets screen options.
    pub fn set_options(&mut self, opts: ScreenOptions) {
        self.opts = opts;
    }

    /// Scrolls content up within the scroll region by `lines` rows.
    pub fn scroll_up(&mut self, lines: i32) {
        let Some(region) = self.scroll else {
            return;
        };
        if lines <= 0 {
            return;
        }

        let top = region.top.max(0) as usize;
        let bottom = region.bottom.max(0) as usize;
        let height = self.new_buf.height();
        if bottom >= height {
            return;
        }

        let lines = lines as usize;
        for y in top..=bottom.saturating_sub(lines) {
            let src_y = y + lines;
            if src_y > bottom {
                break;
            }
            for x in 0..self.new_buf.width() {
                let cell = self.new_buf.cell(x as i32, src_y as i32).cloned();
                self.new_buf.set_cell(x as i32, y as i32, cell);
            }
        }
        for y in (bottom + 1).saturating_sub(lines)..=bottom {
            if y >= height {
                break;
            }
            for x in 0..self.new_buf.width() {
                self.new_buf
                    .set_cell(x as i32, y as i32, Some(blank_cell()));
            }
        }

        if self.cursor.y >= region.top && self.cursor.y <= region.bottom {
            self.cursor.y = (self.cursor.y - lines as i32).max(region.top);
        }

        self.pending.push_str(&cursor_up(lines as i32));
    }

    /// Computes ANSI for pending changes and swaps buffers.
    pub fn render(&mut self) -> String {
        let mut out = std::mem::take(&mut self.pending);

        if self.force_clear {
            out.push_str(&erase_display(2));
            out.push_str(&cursor_position(1, 1));
            self.cursor.x = 0;
            self.cursor.y = 0;
            self.pen.reset();
            self.force_clear = false;
        } else {
            let mut scroll_buf = self.cur_buf.clone();
            try_scroll_optimize(&mut scroll_buf, &self.new_buf, &mut out);
            if !out.is_empty() {
                self.cur_buf = scroll_buf;
            }
        }

        let diff = render_with_caps(
            &self.cur_buf,
            &self.new_buf,
            self.cursor.x,
            self.cursor.y,
            &mut self.pen,
            self.opts.render_caps,
        );
        out.push_str(&diff);

        if !diff.is_empty()
            && let Some(last) = diff_lines(&self.cur_buf, &self.new_buf).last()
        {
            self.cursor.x = last.end_x;
            self.cursor.y = last.y;
        }

        self.cur_buf = self.new_buf.clone();
        out
    }

    /// Returns screen width in columns.
    pub fn width(&self) -> usize {
        self.new_buf.width()
    }

    /// Returns screen height in rows.
    pub fn height(&self) -> usize {
        self.new_buf.height()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::Cell;

    #[test]
    fn new_screen_dimensions() {
        let s = Screen::new(10, 5);
        assert_eq!(s.width(), 10);
        assert_eq!(s.height(), 5);
    }

    #[test]
    fn render_single_cell_change() {
        let mut s = Screen::new(3, 1);
        s.buffer().set_cell(1, 0, Some(Cell::new('Q', &[])));
        let out = s.render();
        assert!(out.contains('Q'));
    }

    #[test]
    fn scroll_region_emits_sequence() {
        let mut s = Screen::new(4, 4);
        s.set_scroll_region(1, 3);
        assert!(s.pending.contains("\x1b["));
        assert_eq!(s.scroll_region(), Some(ScrollRegion { top: 1, bottom: 3 }));
    }

    #[test]
    fn redraw_forces_clear_sequence() {
        let mut s = Screen::new(2, 2);
        s.redraw();
        let out = s.render();
        assert!(out.contains("\x1b["));
    }
}
