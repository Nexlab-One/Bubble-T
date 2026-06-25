//! Incremental escape-sequence decoder.

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::c0::{CAN, DEL, ESC, SUB};
use crate::c1::{APC, CSI, DCS, OSC, PM, SOS, ST};

use super::parser::Parser;
use super::seq::Cmd;

/// Decoder state carried across calls to [`decode_sequence`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum DecodeState {
    /// Ground / normal text state.
    #[default]
    Normal = 0,
    /// Collecting private-mode prefix (`<`, `=`, `>`, `?`).
    Prefix = 1,
    /// Collecting numeric parameters.
    Params = 2,
    /// Collecting intermediate bytes.
    Intermed = 3,
    /// After ESC, before CSI/DCS/OSC/etc.
    Escape = 4,
    /// Collecting OSC/DCS/APC string data.
    String = 5,
}

/// Result of decoding one sequence from the input buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedSequence<'a> {
    /// The raw bytes of the decoded unit (one control, one escape sequence, or one grapheme).
    pub sequence: &'a [u8],
    /// Terminal cell width (0 for controls/escapes, ≥1 for printable text).
    pub width: usize,
    /// Number of input bytes consumed.
    pub consumed: usize,
    /// Parser state to pass to the next call.
    pub state: DecodeState,
}

/// Decodes the first escape sequence or printable grapheme cluster from `input`.
pub fn decode_sequence<'a>(
    input: &'a [u8],
    state: DecodeState,
    parser: Option<&mut Parser>,
) -> DecodedSequence<'a> {
    decode_sequence_with_method(input, state, parser, WidthMethod::Grapheme)
}

/// Like [`decode_sequence`] but uses wide-character width instead of grapheme clustering.
pub fn decode_sequence_wc<'a>(
    input: &'a [u8],
    state: DecodeState,
    parser: Option<&mut Parser>,
) -> DecodedSequence<'a> {
    decode_sequence_with_method(input, state, parser, WidthMethod::WideChar)
}

/// Width calculation strategy for printable text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidthMethod {
    /// Grapheme-cluster width (Unicode mode 2027 semantics).
    Grapheme,
    /// Per-rune width without grapheme clustering.
    WideChar,
}

fn decode_sequence_with_method<'a>(
    input: &'a [u8],
    mut state: DecodeState,
    mut parser: Option<&mut Parser>,
    method: WidthMethod,
) -> DecodedSequence<'a> {
    let mut i = 0usize;
    while i < input.len() {
        let c = input[i];

        match state {
            DecodeState::Normal => {
                if c == ESC {
                    if let Some(p) = parser.as_mut() {
                        p.reset_collect();
                    }
                    state = DecodeState::Escape;
                    i += 1;
                    continue;
                }
                if c == CSI || c == DCS {
                    if let Some(p) = parser.as_mut() {
                        p.reset_collect();
                    }
                    state = DecodeState::Prefix;
                    i += 1;
                    continue;
                }
                if c == OSC || c == APC || c == SOS || c == PM {
                    if let Some(p) = parser.as_mut() {
                        p.reset_string();
                        p.set_string_kind(c);
                    }
                    state = DecodeState::String;
                    i += 1;
                    continue;
                }
                if let Some(p) = parser.as_mut() {
                    p.data_len = 0;
                    p.params_len = 0;
                    p.cmd = 0;
                }
                if c > US && c < DEL {
                    return finish(input, i, i + 1, 1, DecodeState::Normal);
                }
                if c <= US || c == DEL || c < 0xC0 {
                    return finish(input, i, i + 1, 0, DecodeState::Normal);
                }
                if utf8::starts_with(c) {
                    let (seq, width) = first_cluster(&input[i..], method);
                    return finish(input, i, i + seq.len(), width, DecodeState::Normal);
                }
                return finish(input, i, i, 0, DecodeState::Normal);
            }
            DecodeState::Prefix => {
                if (b'<'..=b'?').contains(&c) {
                    if let Some(p) = parser.as_mut() {
                        p.set_prefix(c);
                    }
                    i += 1;
                    continue;
                }
                state = DecodeState::Params;
            }
            DecodeState::Params => {
                if c.is_ascii_digit() {
                    if let Some(p) = parser.as_mut() {
                        p.push_digit(c);
                    }
                    i += 1;
                    continue;
                }
                if c == b':'
                    && let Some(p) = parser.as_mut()
                {
                    p.mark_subparam();
                }
                if c == b';' || c == b':' {
                    if let Some(p) = parser.as_mut() {
                        p.advance_param();
                    }
                    i += 1;
                    continue;
                }
                state = DecodeState::Intermed;
            }
            DecodeState::Intermed => {
                if (b' '..=b'/').contains(&c) {
                    if let Some(p) = parser.as_mut() {
                        p.set_intermediate(c);
                    }
                    i += 1;
                    continue;
                }
                if let Some(p) = parser.as_mut() {
                    p.bump_last_param();
                }
                if (b'@'..=b'~').contains(&c) {
                    if let Some(p) = parser.as_mut() {
                        p.set_final(c);
                    }
                    if has_dcs_prefix(input) {
                        if let Some(p) = parser.as_mut() {
                            p.data_len = 0;
                            p.set_string_kind(b'P');
                        }
                        state = DecodeState::String;
                        i += 1;
                        continue;
                    }
                    return finish(input, 0, i + 1, 0, DecodeState::Normal);
                }
                return finish(input, 0, i, 0, DecodeState::Normal);
            }
            DecodeState::Escape => {
                if c == b'[' || c == b'P' {
                    if let Some(p) = parser.as_mut() {
                        p.reset_collect();
                    }
                    state = DecodeState::Prefix;
                    i += 1;
                    continue;
                }
                if c == b']' || c == b'X' || c == b'^' || c == b'_' {
                    if let Some(p) = parser.as_mut() {
                        p.reset_string();
                        p.set_string_kind(c);
                    }
                    state = DecodeState::String;
                    i += 1;
                    continue;
                }
                if c == b'P' {
                    if let Some(p) = parser.as_mut() {
                        p.reset_collect();
                        p.set_string_kind(b'P');
                    }
                    state = DecodeState::Prefix;
                    i += 1;
                    continue;
                }
                if (b' '..=b'/').contains(&c) {
                    if let Some(p) = parser.as_mut() {
                        p.set_intermediate(c);
                    }
                    i += 1;
                    continue;
                }
                if (b'0'..=b'~').contains(&c) {
                    if let Some(p) = parser.as_mut() {
                        p.set_final(c);
                    }
                    return finish(input, 0, i + 1, 0, DecodeState::Normal);
                }
                return finish(input, 0, i, 0, DecodeState::Normal);
            }
            DecodeState::String => match c {
                0x07 if has_osc_prefix(input) => {
                    if let Some(p) = parser.as_mut() {
                        p.parse_osc_cmd();
                    }
                    return finish(input, 0, i + 1, 0, DecodeState::Normal);
                }
                CAN | SUB if has_osc_prefix(input) => {
                    if let Some(p) = parser.as_mut() {
                        p.parse_osc_cmd();
                    }
                    return finish(input, 0, i, 0, DecodeState::Normal);
                }
                ST if has_osc_prefix(input) => {
                    if let Some(p) = parser.as_mut() {
                        p.parse_osc_cmd();
                    }
                    return finish(input, 0, i + 1, 0, DecodeState::Normal);
                }
                ESC if has_st_prefix(&input[i..]) => {
                    if has_osc_prefix(input)
                        && let Some(p) = parser.as_mut()
                    {
                        p.parse_osc_cmd();
                    }
                    return finish(input, 0, i + 2, 0, DecodeState::Normal);
                }
                ESC => return finish(input, 0, i, 0, DecodeState::Normal),
                _ => {
                    if let Some(p) = parser.as_mut() {
                        p.put_data(c);
                        if c == b';' && has_osc_prefix(input) {
                            p.parse_osc_cmd();
                        }
                    }
                    i += 1;
                    continue;
                }
            },
        }
    }

    DecodedSequence {
        sequence: input,
        width: 0,
        consumed: input.len(),
        state,
    }
}

const US: u8 = 0x1F;

fn finish(
    input: &[u8],
    start: usize,
    end: usize,
    width: usize,
    state: DecodeState,
) -> DecodedSequence<'_> {
    DecodedSequence {
        sequence: &input[start..end],
        width,
        consumed: end,
        state,
    }
}

fn first_cluster(b: &[u8], method: WidthMethod) -> (&[u8], usize) {
    let s = std::str::from_utf8(b).unwrap_or("");
    match method {
        WidthMethod::Grapheme => {
            let cluster = s.graphemes(true).next().unwrap_or("");
            (cluster.as_bytes(), cluster.width())
        }
        WidthMethod::WideChar => {
            let ch = s.chars().next().unwrap_or('\0');
            let bytes = ch.len_utf8();
            (&b[..bytes], ch.width().unwrap_or(0))
        }
    }
}

mod utf8 {
    pub fn starts_with(b: u8) -> bool {
        (0xC0..=0xF4).contains(&b)
    }
}

/// Returns true when `b` begins with a CSI prefix (`CSI` or `ESC [`).
pub fn has_csi_prefix(b: &[u8]) -> bool {
    !b.is_empty() && (b[0] == CSI || (b.len() > 1 && b[0] == ESC && b[1] == b'['))
}

/// Returns true when `b` begins with an OSC prefix.
pub fn has_osc_prefix(b: &[u8]) -> bool {
    !b.is_empty() && (b[0] == OSC || (b.len() > 1 && b[0] == ESC && b[1] == b']'))
}

/// Returns true when `b` begins with a DCS prefix.
pub fn has_dcs_prefix(b: &[u8]) -> bool {
    !b.is_empty() && (b[0] == DCS || (b.len() > 1 && b[0] == ESC && b[1] == b'P'))
}

/// Returns true when `b` begins with an ST prefix (`ST` or `ESC \`).
pub fn has_st_prefix(b: &[u8]) -> bool {
    !b.is_empty() && (b[0] == ST || (b.len() > 1 && b[0] == ESC && b[1] == b'\\'))
}

/// Returns true when `b` begins with an ESC prefix.
pub fn has_esc_prefix(b: &[u8]) -> bool {
    !b.is_empty() && b[0] == ESC
}

/// Packs a command from prefix, intermediate, and final bytes.
pub fn command(prefix: u8, inter: u8, final_byte: u8) -> Cmd {
    Cmd::pack(prefix, inter, final_byte)
}

#[cfg(test)]
mod tests {
    use super::super::seq::MISSING_PARAM;
    use super::*;

    #[test]
    fn ascii_printable() {
        let d = decode_sequence(b"a", DecodeState::Normal, None);
        assert_eq!(d.sequence, b"a");
        assert_eq!(d.width, 1);
        assert_eq!(d.consumed, 1);
    }

    #[test]
    fn sgr_red_bold() {
        let input = b"\x1b[31;1m";
        let mut p = Parser::new();
        let d = decode_sequence(input, DecodeState::Normal, Some(&mut p));
        assert_eq!(d.sequence, input);
        assert_eq!(p.params(), &[31, 1]);
        assert_eq!(p.command().final_byte(), b'm');
    }

    #[test]
    fn csi_private_mode() {
        let input = b"\x1b[?1049h";
        let mut p = Parser::new();
        let d = decode_sequence(input, DecodeState::Normal, Some(&mut p));
        assert_eq!(d.sequence, input);
        assert_eq!(p.params(), &[1049]);
        assert_eq!(p.command().prefix(), b'?');
        assert_eq!(p.command().final_byte(), b'h');
    }

    #[test]
    fn osc_window_title() {
        let input = b"\x1b]2;Hi\x07";
        let mut p = Parser::new();
        let d = decode_sequence(input, DecodeState::Normal, Some(&mut p));
        assert_eq!(d.sequence, input);
        assert_eq!(p.command().raw(), 2);
    }

    #[test]
    fn mixed_text_and_sgr() {
        let input = b"\x1b[0mHello\x1b[1m";
        let mut state = DecodeState::Normal;
        let mut p = Parser::new();
        let mut out = Vec::new();

        let mut rest = input.as_slice();
        while !rest.is_empty() {
            let d = decode_sequence(rest, state, Some(&mut p));
            out.push(d.sequence.to_vec());
            state = d.state;
            rest = &rest[d.consumed..];
        }

        assert_eq!(out[0], b"\x1b[0m");
        assert_eq!(out[1], b"H");
        assert_eq!(out[5], b"o");
        assert_eq!(out[6], b"\x1b[1m");
    }

    #[test]
    fn trailing_semicolon_param() {
        let input = b"\x1b[4;m";
        let mut p = Parser::new();
        let _ = decode_sequence(input, DecodeState::Normal, Some(&mut p));
        assert_eq!(p.params()[0], 4);
        assert_eq!(p.params()[1], MISSING_PARAM);
    }

    #[test]
    fn subparams() {
        let input = b"\x1b[38:2:255:0:255;1m";
        let mut p = Parser::new();
        let _ = decode_sequence(input, DecodeState::Normal, Some(&mut p));
        assert!(p.params()[0] & super::super::seq::HAS_MORE_FLAG != 0);
    }
}
