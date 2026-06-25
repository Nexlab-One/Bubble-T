//! Default Sixel / xterm 256-color palette entries.

use image::Rgba;

/// Returns the default palette color at `index` (0–255).
#[must_use]
pub fn default_palette_entry(index: usize) -> Rgba<u8> {
    if index < SIXEL_DEFAULTS.len() {
        return SIXEL_DEFAULTS[index];
    }
    if (16..232).contains(&index) {
        return xterm_cube(index - 16);
    }
    if (232..256).contains(&index) {
        let level = (index - 232) as u8;
        let v = 8 + level.saturating_mul(10);
        return Rgba([v, v, v, 0xff]);
    }
    Rgba([0, 0, 0, 0xff])
}

/// Returns a clone of the full 256-entry default palette.
#[must_use]
pub fn default_palette() -> [Rgba<u8>; 256] {
    std::array::from_fn(default_palette_entry)
}

fn xterm_cube(i: usize) -> Rgba<u8> {
    let r = (i / 36) % 6;
    let g = (i / 6) % 6;
    let b = i % 6;
    Rgba([
        if r == 0 {
            0
        } else {
            55 + u8::try_from(r * 40).unwrap_or(255)
        },
        if g == 0 {
            0
        } else {
            55 + u8::try_from(g * 40).unwrap_or(255)
        },
        if b == 0 {
            0
        } else {
            55 + u8::try_from(b * 40).unwrap_or(255)
        },
        0xff,
    ])
}

const SIXEL_DEFAULTS: [Rgba<u8>; 16] = [
    Rgba([0, 0, 0, 255]),
    Rgba([51, 51, 204, 255]),
    Rgba([204, 36, 36, 255]),
    Rgba([51, 204, 51, 255]),
    Rgba([204, 51, 204, 255]),
    Rgba([51, 204, 204, 255]),
    Rgba([204, 204, 51, 255]),
    Rgba([120, 120, 120, 255]),
    Rgba([69, 69, 69, 255]),
    Rgba([87, 87, 153, 255]),
    Rgba([153, 69, 69, 255]),
    Rgba([87, 153, 87, 255]),
    Rgba([153, 87, 153, 255]),
    Rgba([87, 153, 153, 255]),
    Rgba([153, 153, 87, 255]),
    Rgba([204, 204, 204, 255]),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_has_256_entries() {
        let p = default_palette();
        assert_eq!(p.len(), 256);
        assert_eq!(p[255], Rgba([238, 238, 238, 255]));
    }
}
