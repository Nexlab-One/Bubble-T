//! Unicode-aware string width and ANSI stripping.

use crate::parse::{DecodeState, decode_sequence, decode_sequence_wc};

/// Display-width calculation strategy (mirrors upstream [`Method`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Method {
    /// Per-rune width without grapheme clustering.
    WcWidth,
    /// Grapheme-cluster width (Unicode mode 2027 semantics).
    #[default]
    GraphemeWidth,
}

impl Method {
    /// Returns the terminal cell width of `s`, ignoring ANSI escape sequences.
    pub fn string_width(self, s: &str) -> usize {
        string_width_with_method(self, s)
    }

    /// Removes ANSI escape sequences from `s`, preserving printable text.
    pub fn strip(self, s: &str) -> String {
        strip_with_method(s, self)
    }
}

/// Removes ANSI escape sequences from `s`.
pub fn strip(s: &str) -> String {
    Method::GraphemeWidth.strip(s)
}

/// Returns the terminal cell width of `s` using grapheme clustering.
pub fn string_width(s: &str) -> usize {
    Method::GraphemeWidth.string_width(s)
}

/// Returns the terminal cell width of `s` using wide-character width.
pub fn string_width_wc(s: &str) -> usize {
    Method::WcWidth.string_width(s)
}

fn string_width_with_method(method: Method, s: &str) -> usize {
    if s.is_empty() {
        return 0;
    }

    let decode = match method {
        Method::WcWidth => decode_sequence_wc,
        Method::GraphemeWidth => decode_sequence,
    };

    let mut width = 0usize;
    let mut state = DecodeState::Normal;
    let mut rest = s.as_bytes();

    while !rest.is_empty() {
        let d = decode(rest, state, None);
        width += d.width;
        state = d.state;
        rest = &rest[d.consumed..];
    }

    width
}

fn strip_with_method(s: &str, method: Method) -> String {
    let decode = match method {
        Method::WcWidth => decode_sequence_wc,
        Method::GraphemeWidth => decode_sequence,
    };

    let mut buf = String::new();
    let mut state = DecodeState::Normal;
    let mut rest = s.as_bytes();

    while !rest.is_empty() {
        let d = decode(rest, state, None);
        if d.width > 0 {
            buf.push_str(std::str::from_utf8(d.sequence).unwrap_or(""));
        }
        state = d.state;
        rest = &rest[d.consumed..];
    }

    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn width_ignores_sgr() {
        assert_eq!(string_width("\x1b[31mhello\x1b[0m"), 5);
    }

    #[test]
    fn width_wide_char() {
        assert_eq!(string_width("世"), 2);
    }

    #[test]
    fn strip_removes_codes() {
        assert_eq!(strip("\x1b[1;31mhi\x1b[0m"), "hi");
    }
}
