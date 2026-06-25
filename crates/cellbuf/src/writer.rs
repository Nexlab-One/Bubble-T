//! Writing styled strings into a cell buffer.

use ansi::parse::{DecodeState, Parser, decode_sequence, has_csi_prefix, has_osc_prefix};
use ansi::width::Method;

use crate::buffer::Buffer;
use crate::cell::Cell;
use crate::geom::{Position, Rectangle};
use crate::style::{read_link, read_style};

/// Clears `buf` and writes `content`, parsing embedded ANSI SGR and OSC 8 sequences.
pub fn set_content(buf: &mut Buffer, content: &str) {
    set_content_rect(buf, content, buf.bounds());
}

/// Clears `rect` within `buf` and writes `content` into it.
pub fn set_content_rect(buf: &mut Buffer, content: &str, rect: Rectangle) {
    let normalized = content.replace("\r\n", "\n").replace('\n', "\r\n");
    buf.clear_rect(rect);
    print_string(
        buf,
        Method::GraphemeWidth,
        rect.min.x,
        rect.min.y,
        rect,
        &normalized,
        true,
        "",
    );
}

/// Writes `content` at `(x, y)` without clearing first.
pub fn print_at(buf: &mut Buffer, x: i32, y: i32, content: &str) {
    print_string(
        buf,
        Method::GraphemeWidth,
        x,
        y,
        buf.bounds(),
        content,
        false,
        "",
    );
}

/// Blends `content` onto `buf` at `(x, y)`, skipping transparent (whitespace-only) cells.
///
/// Used by Lip Gloss compositing so higher z-index layers do not erase underlying content
/// where the overlay is blank space without an explicit background color.
pub fn blend_at(buf: &mut Buffer, x: i32, y: i32, content: &str) {
    let mut overlay = Buffer::new(buf.width(), buf.height());
    print_at(&mut overlay, x, y, content);
    for row in 0..buf.height() {
        for col in 0..buf.width() {
            let x = i32::try_from(col).unwrap_or(i32::MAX);
            let y = i32::try_from(row).unwrap_or(i32::MAX);
            let Some(cell) = overlay.cell(x, y) else {
                continue;
            };
            if cell_is_transparent(cell) {
                continue;
            }
            buf.set_cell(x, y, Some(cell.clone()));
        }
    }
}

fn cell_is_transparent(cell: &Cell) -> bool {
    if cell.is_empty() {
        return true;
    }
    cell.ch == Some(' ') && cell.comb.is_empty() && cell.style.bg.is_none() && cell.link.is_empty()
}

#[allow(clippy::too_many_arguments)]
fn print_string(
    buf: &mut Buffer,
    _method: Method,
    mut x: i32,
    mut y: i32,
    bounds: Rectangle,
    content: &str,
    truncate: bool,
    tail: &str,
) {
    let mut parser = Parser::new();
    let mut state = DecodeState::Normal;
    let mut rest = content.as_bytes();
    let mut style = crate::style::Style::default();
    let mut link = crate::link::Link::default();
    let mut pending = Cell::default();
    let tail_cell = if truncate && !tail.is_empty() {
        Some(Cell::from_grapheme(tail))
    } else {
        None
    };

    while !rest.is_empty() {
        let d = decode_sequence(rest, state, Some(&mut parser));
        let chunk = std::str::from_utf8(d.sequence).unwrap_or("");

        if d.width > 0 {
            pending.width = pending.width.saturating_add(d.width as u8);
            pending.append(chunk.chars());

            if !truncate && x + i32::from(pending.width) > bounds.max.x && y + 1 < bounds.max.y {
                x = bounds.min.x;
                y += 1;
            }

            if Position::new(x, y).in_rect(bounds) {
                if let Some(ref tail_c) = tail_cell
                    && x + i32::from(pending.width) > bounds.max.x - i32::from(tail_c.width)
                {
                    let mut cell = tail_c.clone();
                    cell.style = style.clone();
                    cell.link = link.clone();
                    buf.set_cell(x, y, Some(cell));
                    x += i32::from(tail_c.width);
                    pending.reset();
                    state = d.state;
                    rest = &rest[d.consumed..];
                    continue;
                }

                let mut cell = std::mem::take(&mut pending);
                cell.style = style.clone();
                cell.link = link.clone();
                buf.set_cell(x, y, Some(cell));
                x += d.width as i32;
                pending.reset();
            }
        } else if has_csi_prefix(d.sequence) && parser.command().final_byte() == b'm' {
            read_style(parser.params(), &mut style);
        } else if has_osc_prefix(d.sequence) && parser.command().raw() == 8 {
            read_link(parser.data(), &mut link);
        } else if chunk == "\n" {
            y += 1;
        } else if chunk == "\r" {
            x = bounds.min.x;
        } else {
            pending.append(chunk.chars());
        }

        state = d.state;
        rest = &rest[d.consumed..];
    }

    if !pending.is_empty() && Position::new(x, y).in_rect(bounds) {
        pending.style = style;
        pending.link = link;
        buf.set_cell(x, y, Some(pending));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ansi::color::{BasicColor, Color};

    #[test]
    fn set_content_plain_text() {
        let mut buf = Buffer::new(5, 1);
        set_content(&mut buf, "hi");
        assert_eq!(buf.cell(0, 0).unwrap().content(), "h");
        assert_eq!(buf.cell(1, 0).unwrap().content(), "i");
    }

    #[test]
    fn set_content_applies_sgr() {
        let mut buf = Buffer::new(5, 1);
        set_content(&mut buf, "\x1b[31mR\x1b[0m");
        let cell = buf.cell(0, 0).unwrap();
        assert_eq!(cell.content(), "R");
        assert_eq!(cell.style.fg, Some(Color::Basic(BasicColor::Red)));
    }

    #[test]
    fn wide_char_in_buffer() {
        let mut buf = Buffer::new(4, 1);
        set_content(&mut buf, "世");
        assert_eq!(buf.cell(0, 0).unwrap().width, 2);
    }
}
