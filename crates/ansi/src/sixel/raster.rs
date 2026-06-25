//! Sixel raster attribute parsing.

use crate::sixel::control::RASTER_ATTRIBUTE;

/// Error returned when raster attributes are invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodeRasterError;

impl std::fmt::Display for DecodeRasterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid raster attributes")
    }
}

impl std::error::Error for DecodeRasterError {}

/// Sixel raster attributes (`"pan;pad;ph;pv`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Raster {
    /// Pixel aspect numerator.
    pub pan: i32,
    /// Pixel aspect denominator.
    pub pad: i32,
    /// Horizontal pixel count.
    pub ph: i32,
    /// Vertical pixel count.
    pub pv: i32,
}

impl std::fmt::Display for Raster {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut out = String::new();
        write_raster(&mut out, self.pan, self.pad, self.ph, self.pv);
        f.write_str(&out)
    }
}

/// Writes raster attributes to `out`.
pub fn write_raster(out: &mut String, pan: i32, pad: i32, ph: i32, pv: i32) {
    let (pan, pad) = if pad == 0 { (1, 1) } else { (pan, pad) };
    out.push(char::from(RASTER_ATTRIBUTE));
    out.push_str(&pan.to_string());
    out.push(';');
    out.push_str(&pad.to_string());
    if ph > 0 || pv > 0 {
        out.push(';');
        out.push_str(&ph.to_string());
        out.push(';');
        out.push_str(&pv.to_string());
    }
}

/// Decodes raster attributes from `data` (starting with `"`).
pub fn decode_raster(data: &[u8]) -> Result<(Raster, usize), DecodeRasterError> {
    if data.is_empty() || data[0] != RASTER_ATTRIBUTE {
        return Err(DecodeRasterError);
    }

    let mut values = [0i32; 4];
    let mut index = 0usize;
    let mut current = 0i32;
    let mut n = 1usize;

    while n < data.len() && index < 4 {
        let b = data[n];
        if b == b';' {
            values[index] = current;
            current = 0;
            index += 1;
        } else if b.is_ascii_digit() {
            current = current.saturating_mul(10) + i32::from(b - b'0');
        } else {
            break;
        }
        n += 1;
    }
    if index < 4 {
        values[index] = current;
    }

    Ok((
        Raster {
            pan: values[0],
            pad: values[1],
            ph: values[2],
            pv: values[3],
        },
        n,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_raster() {
        let mut out = String::new();
        write_raster(&mut out, 1, 1, 10, 20);
        let (r, n) = decode_raster(out.as_bytes()).unwrap();
        assert_eq!(r.ph, 10);
        assert_eq!(r.pv, 20);
        assert_eq!(n, out.len());
    }
}
