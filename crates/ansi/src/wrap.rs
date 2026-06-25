//! Width-aware line wrapping.

use crate::parse::{DecodeState, decode_sequence, decode_sequence_wc};
use crate::width::Method;

/// Hard-wraps `s` to `limit` cells, breaking at any character boundary.
pub fn hardwrap(s: &str, limit: usize, preserve_space: bool) -> String {
    Method::GraphemeWidth.hardwrap(s, limit, preserve_space)
}

/// Word-wraps `s` to `limit` cells without breaking words when possible.
pub fn wordwrap(s: &str, limit: usize, breakpoints: &str) -> String {
    Method::GraphemeWidth.wordwrap(s, limit, breakpoints)
}

/// Wraps `s` to `limit` cells, breaking words only when necessary.
pub fn wrap(s: &str, limit: usize, breakpoints: &str) -> String {
    Method::GraphemeWidth.wrap(s, limit, breakpoints)
}

impl Method {
    /// Hard-wraps text to `limit` cells.
    pub fn hardwrap(self, s: &str, limit: usize, preserve_space: bool) -> String {
        if limit == 0 {
            return s.to_string();
        }
        hardwrap_impl(self, s, limit, preserve_space)
    }

    /// Word-wraps text to `limit` cells.
    pub fn wordwrap(self, s: &str, limit: usize, breakpoints: &str) -> String {
        if limit == 0 {
            return s.to_string();
        }
        wordwrap_impl(self, s, limit, breakpoints)
    }

    /// Wraps text to `limit` cells, hard-breaking long words when needed.
    pub fn wrap(self, s: &str, limit: usize, breakpoints: &str) -> String {
        if limit == 0 {
            return s.to_string();
        }
        wrap_impl(self, s, limit, breakpoints)
    }
}

const NBSP: char = '\u{00A0}';

fn hardwrap_impl(method: Method, s: &str, limit: usize, preserve_space: bool) -> String {
    let decode = match method {
        Method::WcWidth => decode_sequence_wc,
        Method::GraphemeWidth => decode_sequence,
    };

    let mut buf = String::new();
    let mut cur_width = 0usize;
    let mut state = DecodeState::Normal;
    let mut rest = s.as_bytes();

    let add_newline = |buf: &mut String, cur_width: &mut usize| {
        buf.push('\n');
        *cur_width = 0;
    };

    while !rest.is_empty() {
        let d = decode(rest, state, None);
        let chunk = std::str::from_utf8(d.sequence).unwrap_or("");

        if d.width > 0 {
            if cur_width + d.width > limit {
                add_newline(&mut buf, &mut cur_width);
            }
            if !preserve_space
                && cur_width == 0
                && chunk.chars().all(|c| c.is_whitespace() && c != NBSP)
            {
                state = d.state;
                rest = &rest[d.consumed..];
                continue;
            }
            buf.push_str(chunk);
            cur_width += d.width;
        } else if chunk == "\n" {
            add_newline(&mut buf, &mut cur_width);
        } else {
            buf.push_str(chunk);
        }

        state = d.state;
        rest = &rest[d.consumed..];
    }

    buf
}

fn wordwrap_impl(method: Method, s: &str, limit: usize, breakpoints: &str) -> String {
    let decode = match method {
        Method::WcWidth => decode_sequence_wc,
        Method::GraphemeWidth => decode_sequence,
    };

    let mut buf = String::new();
    let mut word = String::new();
    let mut space = String::new();
    let mut cur_width = 0usize;
    let mut word_len = 0usize;
    let mut state = DecodeState::Normal;
    let mut rest = s.as_bytes();

    let flush_space = |buf: &mut String, space: &mut String, cur_width: &mut usize| {
        *cur_width += Method::GraphemeWidth.string_width(space);
        buf.push_str(space);
        space.clear();
    };

    let flush_word = |buf: &mut String,
                      word: &mut String,
                      space: &mut String,
                      cur_width: &mut usize,
                      word_len: &mut usize| {
        if word.is_empty() {
            return;
        }
        flush_space(buf, space, cur_width);
        *cur_width += *word_len;
        buf.push_str(word);
        word.clear();
        *word_len = 0;
    };

    let add_newline = |buf: &mut String, space: &mut String, cur_width: &mut usize| {
        buf.push('\n');
        *cur_width = 0;
        space.clear();
    };

    while !rest.is_empty() {
        let d = decode(rest, state, None);
        let chunk = std::str::from_utf8(d.sequence).unwrap_or("");

        if d.width > 0 {
            let ch = chunk.chars().next().unwrap_or('\0');
            if ch.is_whitespace() && ch != NBSP {
                flush_word(
                    &mut buf,
                    &mut word,
                    &mut space,
                    &mut cur_width,
                    &mut word_len,
                );
                space.push_str(chunk);
            } else if chunk.chars().any(|c| breakpoints.contains(c) || c == '-') {
                flush_space(&mut buf, &mut space, &mut cur_width);
                flush_word(
                    &mut buf,
                    &mut word,
                    &mut space,
                    &mut cur_width,
                    &mut word_len,
                );
                buf.push_str(chunk);
                cur_width += d.width;
            } else {
                word.push_str(chunk);
                word_len += d.width;
                if cur_width + Method::GraphemeWidth.string_width(&space) + word_len > limit
                    && word_len < limit
                {
                    add_newline(&mut buf, &mut space, &mut cur_width);
                }
            }
        } else if chunk == "\n" {
            if word_len == 0 && !space.is_empty() {
                if cur_width + Method::GraphemeWidth.string_width(&space) > limit {
                    cur_width = 0;
                } else {
                    buf.push_str(&space);
                }
                space.clear();
            }
            flush_word(
                &mut buf,
                &mut word,
                &mut space,
                &mut cur_width,
                &mut word_len,
            );
            add_newline(&mut buf, &mut space, &mut cur_width);
        } else {
            word.push_str(chunk);
        }

        state = d.state;
        rest = &rest[d.consumed..];
    }

    flush_word(
        &mut buf,
        &mut word,
        &mut space,
        &mut cur_width,
        &mut word_len,
    );
    buf
}

fn wrap_impl(method: Method, s: &str, limit: usize, breakpoints: &str) -> String {
    let decode = match method {
        Method::WcWidth => decode_sequence_wc,
        Method::GraphemeWidth => decode_sequence,
    };

    let mut buf = String::new();
    let mut word = String::new();
    let mut space = String::new();
    let mut space_width = 0usize;
    let mut cur_width = 0usize;
    let mut word_len = 0usize;
    let mut state = DecodeState::Normal;
    let mut rest = s.as_bytes();

    let flush_space =
        |buf: &mut String, space: &mut String, space_width: &mut usize, cur_width: &mut usize| {
            if space.is_empty() && *space_width == 0 {
                return;
            }
            *cur_width += *space_width;
            buf.push_str(space);
            space.clear();
            *space_width = 0;
        };

    let flush_word = |buf: &mut String,
                      word: &mut String,
                      space: &mut String,
                      space_width: &mut usize,
                      cur_width: &mut usize,
                      word_len: &mut usize| {
        if word.is_empty() {
            return;
        }
        flush_space(buf, space, space_width, cur_width);
        *cur_width += *word_len;
        buf.push_str(word);
        word.clear();
        *word_len = 0;
    };

    let add_newline =
        |buf: &mut String, space: &mut String, space_width: &mut usize, cur_width: &mut usize| {
            buf.push('\n');
            *cur_width = 0;
            space.clear();
            *space_width = 0;
        };

    while !rest.is_empty() {
        let d = decode(rest, state, None);
        let chunk = std::str::from_utf8(d.sequence).unwrap_or("");

        if d.width > 0 {
            let ch = chunk.chars().next().unwrap_or('\0');
            if ch.is_whitespace() && ch != NBSP {
                flush_word(
                    &mut buf,
                    &mut word,
                    &mut space,
                    &mut space_width,
                    &mut cur_width,
                    &mut word_len,
                );
                space.push_str(chunk);
                space_width += d.width;
            } else if chunk.chars().any(|c| breakpoints.contains(c)) {
                flush_space(&mut buf, &mut space, &mut space_width, &mut cur_width);
                if cur_width + word_len + d.width > limit {
                    word.push_str(chunk);
                    word_len += d.width;
                } else {
                    flush_word(
                        &mut buf,
                        &mut word,
                        &mut space,
                        &mut space_width,
                        &mut cur_width,
                        &mut word_len,
                    );
                    buf.push_str(chunk);
                    cur_width += d.width;
                }
            } else {
                if word_len + d.width > limit {
                    flush_word(
                        &mut buf,
                        &mut word,
                        &mut space,
                        &mut space_width,
                        &mut cur_width,
                        &mut word_len,
                    );
                }
                word.push_str(chunk);
                word_len += d.width;
                if cur_width + word_len + space_width > limit {
                    add_newline(&mut buf, &mut space, &mut space_width, &mut cur_width);
                }
                if word_len == limit {
                    flush_word(
                        &mut buf,
                        &mut word,
                        &mut space,
                        &mut space_width,
                        &mut cur_width,
                        &mut word_len,
                    );
                }
            }
        } else if chunk == "\n" {
            if word_len == 0 {
                if cur_width + space_width > limit {
                    cur_width = 0;
                } else {
                    buf.push_str(&space);
                }
                space.clear();
                space_width = 0;
            }
            flush_word(
                &mut buf,
                &mut word,
                &mut space,
                &mut space_width,
                &mut cur_width,
                &mut word_len,
            );
            add_newline(&mut buf, &mut space, &mut space_width, &mut cur_width);
        } else {
            word.push_str(chunk);
        }

        state = d.state;
        rest = &rest[d.consumed..];
    }

    if word_len == 0 {
        if cur_width + space_width > limit {
            cur_width = 0;
        } else {
            buf.push_str(&space);
        }
        space.clear();
        space_width = 0;
    }

    flush_word(
        &mut buf,
        &mut word,
        &mut space,
        &mut space_width,
        &mut cur_width,
        &mut word_len,
    );
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hardwrap_breaks_line() {
        assert_eq!(hardwrap("hello world", 5, false), "hello\nworld");
    }

    #[test]
    fn wrap_preserves_sgr() {
        let s = "\x1b[31mhello world\x1b[0m";
        let got = wrap(s, 5, "");
        assert!(got.contains('\n'));
        assert!(got.contains("\x1b[31m"));
    }
}
