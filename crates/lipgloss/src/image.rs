//! Inline terminal image helpers for Lip Gloss canvases.

use ansi::graphics::kitty::{
    Options as KittyOptions, RGBA, SHARED_MEMORY, TRANSMIT_AND_PUT, encode_graphics,
    encode_graphics_with_fallback,
};
use ansi::graphics::sixel_graphics;
use ansi::sixel::Encoder as SixelEncoder;
use cellbuf::{Buffer, ImageArea, ImagePlacementConfig, ImageProtocol};
use image::RgbaImage;

/// Kitty transmission settings for [`render_kitty`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KittyRenderConfig {
    /// Kitty `t=` transmission byte (`0` = direct).
    pub transmission: u8,
    /// Fall back to direct transmission when shared memory is unavailable.
    pub shm_fallback: bool,
}

impl Default for KittyRenderConfig {
    fn default() -> Self {
        Self {
            transmission: 0,
            shm_fallback: true,
        }
    }
}

impl From<ImagePlacementConfig> for KittyRenderConfig {
    fn from(value: ImagePlacementConfig) -> Self {
        Self {
            transmission: value.kitty_transmission,
            shm_fallback: value.kitty_shm_fallback,
        }
    }
}

impl From<KittyRenderConfig> for ImagePlacementConfig {
    fn from(value: KittyRenderConfig) -> Self {
        Self {
            kitty_transmission: value.transmission,
            kitty_shm_fallback: value.shm_fallback,
        }
    }
}

/// Renders an image into a Lip Gloss/ANSI string using Sixel inline graphics.
#[must_use]
pub fn render_sixel(img: &RgbaImage, columns: i32, rows: i32) -> String {
    let payload = SixelEncoder.encode(img);
    let _ = (columns, rows);
    sixel_graphics(0, 1, 0, payload.as_bytes())
}

/// Renders an image using Kitty transmission (direct by default).
pub fn render_kitty(
    img: &RgbaImage,
    columns: i32,
    rows: i32,
) -> Result<String, ansi::graphics::kitty::EncodeError> {
    render_kitty_with_config(img, columns, rows, KittyRenderConfig::default())
}

/// Renders an image using Kitty with explicit transmission settings.
pub fn render_kitty_with_config(
    img: &RgbaImage,
    columns: i32,
    rows: i32,
    config: KittyRenderConfig,
) -> Result<String, ansi::graphics::kitty::EncodeError> {
    let (w, h) = img.dimensions();
    let opts = KittyOptions {
        action: TRANSMIT_AND_PUT,
        format: RGBA,
        image_width: w as i32,
        image_height: h as i32,
        columns,
        rows,
        virtual_placement: true,
        quiet: 2,
        transmission: config.transmission,
        ..Default::default()
    };
    if config.shm_fallback && config.transmission == SHARED_MEMORY {
        encode_graphics_with_fallback(img, &opts)
    } else {
        encode_graphics(img, &opts)
    }
}

/// Composes an image into a cell buffer and returns `(prefix, rendered ANSI)`.
pub fn compose_image(
    buf: &mut Buffer,
    img: &RgbaImage,
    area: ImageArea,
    protocol: ImageProtocol,
    kitty_id: u32,
) -> Result<(String, String), cellbuf::ImageError> {
    compose_image_with_config(
        buf,
        img,
        area,
        protocol,
        kitty_id,
        ImagePlacementConfig::default(),
    )
}

/// Like [`compose_image`] with explicit Kitty transmission settings.
pub fn compose_image_with_config(
    buf: &mut Buffer,
    img: &RgbaImage,
    area: ImageArea,
    protocol: ImageProtocol,
    kitty_id: u32,
    config: ImagePlacementConfig,
) -> Result<(String, String), cellbuf::ImageError> {
    let prefix = cellbuf::place_image_with_config(buf, img, area, protocol, kitty_id, config)?;
    let empty = Buffer::new(buf.width(), buf.height());
    Ok((prefix, cellbuf::render_from_home(&empty, buf)))
}
