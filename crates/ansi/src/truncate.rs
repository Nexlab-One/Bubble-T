//! Width-aware truncation and slicing.

use crate::parse::{DecodeState, decode_sequence, decode_sequence_wc};
use crate::width::Method;

/// Truncates `s` to `length` cells, appending `tail` when truncated (grapheme width).
pub fn truncate(s: &str, length: usize, tail: &str) -> String {
    Method::GraphemeWidth.truncate(s, length, tail)
}

/// Truncates `s` to `length` cells using wide-character width.
pub fn truncate_wc(s: &str, length: usize, tail: &str) -> String {
    Method::WcWidth.truncate(s, length, tail)
}

/// Truncates from the left, prepending `prefix` when truncated (grapheme width).
pub fn truncate_left(s: &str, n: usize, prefix: &str) -> String {
    Method::GraphemeWidth.truncate_left(s, n, prefix)
}

/// Returns the substring covering cells `[left, right)` (grapheme width).
pub fn cut(s: &str, left: usize, right: usize) -> String {
    Method::GraphemeWidth.cut(s, left, right)
}

fn truncate_impl(method: Method, s: &str, length: usize, tail: &str) -> String {
    if method.string_width(s) <= length {
        return s.to_string();
    }

    let tail_width = method.string_width(tail);
    let budget = length.saturating_sub(tail_width);

    let decode = match method {
        Method::WcWidth => decode_sequence_wc,
        Method::GraphemeWidth => decode_sequence,
    };

    let mut buf = String::new();
    let mut cur_width = 0usize;
    let mut ignoring = false;
    let mut state = DecodeState::Normal;
    let mut rest = s.as_bytes();

    while !rest.is_empty() {
        let d = decode(rest, state, None);
        let chunk = std::str::from_utf8(d.sequence).unwrap_or("");

        if d.width > 0 {
            cur_width += d.width;
            if cur_width > budget && !ignoring {
                ignoring = true;
                buf.push_str(tail);
            }
            if !ignoring {
                buf.push_str(chunk);
            }
        } else if !ignoring {
            buf.push_str(chunk);
        }

        state = d.state;
        rest = &rest[d.consumed..];
    }

    buf
}

fn truncate_left_impl(method: Method, s: &str, n: usize, prefix: &str) -> String {
    if n == 0 {
        return s.to_string();
    }

    let decode = match method {
        Method::WcWidth => decode_sequence_wc,
        Method::GraphemeWidth => decode_sequence,
    };

    let mut buf = String::new();
    let mut cur_width = 0usize;
    let mut ignoring = true;
    let mut state = DecodeState::Normal;
    let mut rest = s.as_bytes();

    while !rest.is_empty() {
        if !ignoring {
            let tail = std::str::from_utf8(rest).unwrap_or("");
            buf.push_str(tail);
            break;
        }

        let d = decode(rest, state, None);
        let chunk = std::str::from_utf8(d.sequence).unwrap_or("");

        if d.width > 0 {
            cur_width += d.width;
            if cur_width > n {
                ignoring = false;
                buf.push_str(prefix);
                buf.push_str(chunk);
            }
        } else if cur_width > n {
            buf.push_str(chunk);
        }

        state = d.state;
        rest = &rest[d.consumed..];
    }

    buf
}

impl Method {
    pub(crate) fn truncate(self, s: &str, length: usize, tail: &str) -> String {
        truncate_impl(self, s, length, tail)
    }

    pub(crate) fn truncate_left(self, s: &str, n: usize, prefix: &str) -> String {
        truncate_left_impl(self, s, n, prefix)
    }

    pub(crate) fn cut(self, s: &str, left: usize, right: usize) -> String {
        if right <= left {
            return String::new();
        }
        if left == 0 {
            return self.truncate(s, right, "");
        }
        self.truncate_left(&self.truncate(s, right, ""), left, "")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_with_tail() {
        assert_eq!(truncate("hello world", 8, "…"), "hello w…");
    }

    #[test]
    fn truncate_preserves_sgr() {
        let s = "\x1b[31mhello\x1b[0m world";
        let got = truncate(s, 5, "");
        assert!(got.contains("\x1b[31m"));
        assert_eq!(Method::GraphemeWidth.string_width(&got), 5);
    }

    #[test]
    fn cut_range() {
        assert_eq!(cut("hello", 1, 4), "ell");
    }
}
