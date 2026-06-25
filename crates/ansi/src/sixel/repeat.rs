//! Sixel RLE repeat introducer parsing.

use crate::sixel::control::REPEAT_INTRODUCER;

/// Error returned when a repeat command is invalid.
/// Error returned when a repeat command is invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodeRepeatError;

impl std::fmt::Display for DecodeRepeatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid sixel repeat")
    }
}

impl std::error::Error for DecodeRepeatError {}

/// A Sixel repeat introducer (`!count char`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Repeat {
    /// Repeat count.
    pub count: i32,
    /// Sixel data byte (`?`..=`~`).
    pub ch: u8,
}

/// Writes `!count char` to `out`.
pub fn write_repeat(out: &mut String, count: i32, ch: u8) {
    out.push(char::from(REPEAT_INTRODUCER));
    out.push_str(&count.to_string());
    out.push(char::from(ch));
}

/// Decodes a repeat command from `data` (starting with `!`).
pub fn decode_repeat(data: &[u8]) -> Result<(Repeat, usize), DecodeRepeatError> {
    if data.len() < 3 || data[0] != REPEAT_INTRODUCER {
        return Err(DecodeRepeatError);
    }

    let mut count = 0i32;
    let mut n = 1usize;
    while n < data.len() {
        let b = data[n];
        if b.is_ascii_digit() {
            count = count.saturating_mul(10) + i32::from(b - b'0');
            n += 1;
        } else {
            break;
        }
    }

    if n >= data.len() {
        return Err(DecodeRepeatError);
    }

    Ok((Repeat { count, ch: data[n] }, n + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_rle() {
        let (r, n) = decode_repeat(b"!20?").unwrap();
        assert_eq!(r.count, 20);
        assert_eq!(r.ch, b'?');
        assert_eq!(n, 4);
    }
}
