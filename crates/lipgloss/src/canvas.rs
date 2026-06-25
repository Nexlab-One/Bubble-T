//! Cell-buffer canvas for Lip Gloss compositing.

use cellbuf::{Buffer, blend_at, render_from_home, set_content_rect};

use crate::layer::Compositor;

/// A width/height cell grid that can compose layers and emit ANSI.
pub struct Canvas {
    buf: Buffer,
}

impl Canvas {
    /// Creates a canvas with the given dimensions.
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            buf: Buffer::new(usize::from(width), usize::from(height)),
        }
    }

    /// Resizes the canvas grid.
    pub fn resize(&mut self, width: u16, height: u16) {
        self.buf.resize(usize::from(width), usize::from(height));
    }

    /// Clears the canvas.
    pub fn clear(&mut self) {
        self.buf.clear();
    }

    /// Returns the canvas width in cells.
    pub fn width(&self) -> usize {
        self.buf.width()
    }

    /// Returns the canvas height in cells.
    pub fn height(&self) -> usize {
        self.buf.height()
    }

    /// Draws a compositor's flattened layers onto this canvas (z-ordered blend).
    pub fn compose(&mut self, compositor: &Compositor) -> &mut Self {
        for layer in compositor.layers() {
            blend_at(&mut self.buf, layer.abs_x, layer.abs_y, &layer.content);
        }
        self
    }

    /// Writes styled `content` into the full canvas area.
    pub fn set_content(&mut self, content: &str) -> &mut Self {
        let bounds = self.buf.bounds();
        set_content_rect(&mut self.buf, content, bounds);
        self
    }

    /// Renders the canvas to an ANSI string.
    pub fn render(&self) -> String {
        let empty = Buffer::new(self.buf.width(), self.buf.height());
        render_from_home(&empty, &self.buf)
    }
}
