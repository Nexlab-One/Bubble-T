//! Linux-console palette control sequences.

use crate::color::RgbColor;
use crate::seq::OSC;

/// OSC sequence that resets the Linux-console color palette to defaults.
pub const RESET_PALETTE: &str = "\x1b]R\x07";

/// Builds an OSC palette sequence (`OSC P n rrggbb BEL`) for index `0..=15`.
///
/// This sequence is specific to the Linux console and may not work in other
/// terminal emulators.
pub fn set_palette(index: u8, color: RgbColor) -> String {
    if index > 15 {
        return String::new();
    }
    format!(
        "{OSC}P{index:x}{r:02x}{g:02x}{b:02x}\x07",
        index = index,
        r = color.r,
        g = color.g,
        b = color.b
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_red_palette_slot() {
        assert_eq!(
            set_palette(1, RgbColor { r: 255, g: 0, b: 0 }),
            "\x1b]P1ff0000\x07"
        );
    }
}
