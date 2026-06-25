//! Terminal image placement in the cell grid.
//!
//! Helpers mirror upstream ultraviolet image placement: Sixel sequences occupy a
//! wide first-row cell with cursor-forward padding on subsequent rows; Kitty uses
//! virtual Unicode placeholders after a one-time transmit sequence.

use ansi::color::{Color, RgbColor};
use ansi::cursor::cursor_forward;
use ansi::graphics::kitty::{
    DIRECT, Options as KittyOptions, PLACEHOLDER, RGBA, SHARED_MEMORY, TRANSMIT_AND_PUT,
    encode_graphics_with_fallback, kitty_transmission_from_env, placeholder_fg,
};
use ansi::graphics::{kitty_graphics, sixel_graphics};
use ansi::sixel::Encoder as SixelEncoder;
use image::RgbaImage;

use crate::buffer::Buffer;
use crate::cell::Cell;
use crate::geom::Rectangle;
use crate::style::Style;

/// Supported terminal image protocols.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImageProtocol {
    /// DEC Sixel DCS graphics.
    #[default]
    Sixel,
    /// Kitty graphics with Unicode virtual placement.
    Kitty,
}

/// Optional Kitty placement settings for [`place_image`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImagePlacementConfig {
    /// Kitty `t=` transmission byte. Zero means direct (`d`).
    pub kitty_transmission: u8,
    /// When true, fall back to direct transmission if shared memory fails.
    pub kitty_shm_fallback: bool,
}

impl Default for ImagePlacementConfig {
    fn default() -> Self {
        Self {
            kitty_transmission: DIRECT,
            kitty_shm_fallback: true,
        }
    }
}

impl ImagePlacementConfig {
    /// Reads [`IMAGE_TRANSMISSION`] and enables SHM fallback by default.
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            kitty_transmission: kitty_transmission_from_env(),
            kitty_shm_fallback: true,
        }
    }
}

/// Cell-grid rectangle for image placement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageArea {
    /// Left column (0-based).
    pub x: i32,
    /// Top row (0-based).
    pub y: i32,
    /// Width in terminal cells.
    pub width: i32,
    /// Height in terminal cells.
    pub height: i32,
}

impl ImageArea {
    /// Creates a new area.
    #[must_use]
    pub const fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Returns the bounding rectangle clipped to `bounds`.
    #[must_use]
    pub fn clip(self, bounds: Rectangle) -> Self {
        let x = self.x.max(bounds.min.x);
        let y = self.y.max(bounds.min.y);
        let w = (self.x + self.width).min(bounds.max.x) - x;
        let h = (self.y + self.height).min(bounds.max.y) - y;
        Self {
            x,
            y,
            width: w.max(0),
            height: h.max(0),
        }
    }
}

/// Computes terminal cell dimensions for a pixel-sized image.
#[must_use]
pub fn cell_dimensions(pixel_w: u32, pixel_h: u32, cell_w: u32, cell_h: u32) -> (i32, i32) {
    if cell_w == 0 || cell_h == 0 {
        return (0, 0);
    }
    (
        (pixel_w / cell_w).max(1) as i32,
        (pixel_h / cell_h).max(1) as i32,
    )
}

/// Writes an image into `buf` at `area`. Returns prefix sequences (Kitty transmit).
pub fn place_image(
    buf: &mut Buffer,
    img: &RgbaImage,
    area: ImageArea,
    protocol: ImageProtocol,
    kitty_id: u32,
) -> Result<String, ImageError> {
    place_image_with_config(
        buf,
        img,
        area,
        protocol,
        kitty_id,
        ImagePlacementConfig::default(),
    )
}

/// Like [`place_image`] with explicit Kitty transmission settings.
pub fn place_image_with_config(
    buf: &mut Buffer,
    img: &RgbaImage,
    area: ImageArea,
    protocol: ImageProtocol,
    kitty_id: u32,
    config: ImagePlacementConfig,
) -> Result<String, ImageError> {
    let area = area.clip(buf.bounds());
    if area.width <= 0 || area.height <= 0 {
        return Ok(String::new());
    }

    match protocol {
        ImageProtocol::Sixel => {
            place_sixel(buf, img, area);
            Ok(String::new())
        }
        ImageProtocol::Kitty => place_kitty(buf, img, area, kitty_id, config),
    }
}

fn place_sixel(buf: &mut Buffer, img: &RgbaImage, area: ImageArea) {
    let payload = SixelEncoder.encode(img);
    let seq = sixel_graphics(0, 1, 0, payload.as_bytes());
    let forward = cursor_forward(area.width);
    for row in 0..area.height {
        let y = area.y + row;
        let content = if row == 0 {
            format!("{seq}{forward}")
        } else {
            forward.clone()
        };
        let mut cell = Cell {
            width: area.width.max(1) as u8,
            ..Default::default()
        };
        cell.append(content.chars());
        buf.set_cell(area.x, y, Some(cell));
    }
}

fn place_kitty(
    buf: &mut Buffer,
    img: &RgbaImage,
    area: ImageArea,
    kitty_id: u32,
    config: ImagePlacementConfig,
) -> Result<String, ImageError> {
    let (w, h) = img.dimensions();
    let transmission = if config.kitty_transmission == 0 {
        DIRECT
    } else {
        config.kitty_transmission
    };
    let opts = KittyOptions {
        id: kitty_id as i32,
        action: TRANSMIT_AND_PUT,
        transmission,
        format: RGBA,
        image_width: w as i32,
        image_height: h as i32,
        columns: area.width,
        rows: area.height,
        virtual_placement: true,
        quiet: 2,
        ..Default::default()
    };
    let prefix = if config.kitty_shm_fallback && transmission == SHARED_MEMORY {
        encode_graphics_with_fallback(img, &opts).map_err(ImageError::Kitty)?
    } else {
        ansi::graphics::kitty::encode_graphics(img, &opts).map_err(ImageError::Kitty)?
    };

    let (r, g, b) = placeholder_fg(kitty_id);
    let style = if r == 0 && g == 0 {
        Style {
            fg: Some(Color::Indexed(ansi::color::IndexedColor(b))),
            ..Style::default()
        }
    } else {
        Style {
            fg: Some(Color::Rgb(RgbColor { r, g, b })),
            ..Style::default()
        }
    };

    let extra = (kitty_id >> 24) & 0xff;
    for row in 0..area.height {
        let y = area.y + row;
        for col in 0..area.width {
            let x = area.x + col;
            let mut content = String::new();
            if col == 0 {
                content.push(PLACEHOLDER);
                content.push(ansi::graphics::kitty::diacritic(row));
                content.push(ansi::graphics::kitty::diacritic(0));
                if extra > 0 {
                    content.push(ansi::graphics::kitty::diacritic(extra as i32));
                }
            } else {
                content.push(PLACEHOLDER);
            }
            let mut cell = Cell {
                width: 1,
                style: style.clone(),
                ..Default::default()
            };
            cell.append(content.chars());
            buf.set_cell(x, y, Some(cell));
        }
    }

    Ok(prefix)
}

/// Builds a Kitty query probe sequence for capability detection.
#[must_use]
pub fn kitty_probe(id: i32) -> String {
    kitty_graphics(
        b"AAAA",
        &[&format!("i={id}"), "s=1", "v=1", "a=q", "t=d", "f=24"],
    )
}

/// Error placing images in the cell grid.
#[derive(Debug)]
pub enum ImageError {
    /// Kitty graphics encoding failed.
    Kitty(ansi::graphics::kitty::EncodeError),
}

impl std::fmt::Display for ImageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Kitty(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ImageError {}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    #[test]
    fn cell_dimensions_math() {
        assert_eq!(cell_dimensions(800, 600, 8, 16), (100, 37));
    }

    #[test]
    fn place_sixel_writes_rows() {
        let mut buf = Buffer::new(10, 5);
        let img = RgbaImage::from_pixel(3, 6, Rgba([255, 0, 0, 255]));
        place_image(
            &mut buf,
            &img,
            ImageArea::new(1, 1, 4, 3),
            ImageProtocol::Sixel,
            0,
        )
        .unwrap();
        let cell = buf.cell(1, 1).unwrap();
        assert!(cell.content().contains('\u{1b}') || cell.width > 1);
    }
}
