//! Width-aware styled cell grid and screen diffing.
//!
//! This crate is the Rust port of [`charmbracelet/x/cellbuf`]. It models a terminal
//! screen as a grid of styled [`Cell`]s and provides the buffer/screen diff that the
//! core "cursed" OutputContext uses to emit the minimal set of escape sequences per frame.
//! Lip Gloss compositing is also layered on top of this grid.
//!
//! [`charmbracelet/x/cellbuf`]: https://github.com/charmbracelet/x/tree/main/cellbuf

#![warn(missing_docs)]

mod buffer;
mod cell;
mod diff;
mod error;
mod geom;
mod graphics;
mod hardscroll;
mod link;
mod render;
mod screen;
mod style;
mod tabstop;
mod util;
mod wrap;
mod writer;

pub use buffer::{Buffer, Line};
pub use cell::{Cell, MAX_CELL_WIDTH, blank_cell, empty_cell};
pub use diff::{LineDiff, buffers_equal, diff_lines};
pub use error::CellbufError;
pub use geom::{Position, Rectangle, pos, rect};
pub use graphics::{
    ImageArea, ImageError, ImagePlacementConfig, ImageProtocol, cell_dimensions, kitty_probe,
    place_image, place_image_with_config,
};
pub use hardscroll::{line_hash, try_scroll_optimize};
pub use link::Link;
pub use render::{
    RenderCaps, RenderPen, render, render_from_home, render_line, render_line_with_caps,
    render_with_caps,
};
pub use screen::{Screen, ScreenCursor, ScreenOptions, ScrollRegion};
pub use style::{AttrMask, Style, UnderlineStyle, read_link, read_style};
pub use tabstop::{DEFAULT_TAB_INTERVAL, DEFAULT_TAB_WIDTH, TabStops};
pub use util::{abs, clamp, height};
pub use wrap::{wrap, wrap_height};
pub use writer::{blend_at, print_at, set_content, set_content_rect};
