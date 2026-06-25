//! Parses terminal query responses into bubble-t messages.

use crate::event::{
    BackgroundColorMsg, CapabilityMsg, ClipboardMsg, CursorColorMsg, CursorPositionMsg,
    ForegroundColorMsg, KeyboardEnhancementsMsg, LightDarkMsg, ModeReportMsg, Msg,
    TerminalVersionMsg,
};
use ansi::query::{
    CsiResponse, DcsResponse, OscResponse, parse_csi_response, parse_dcs_response,
    parse_osc_response,
};

/// Parses as many complete terminal responses as possible from `input`.
///
/// Returns the parsed messages and the number of bytes consumed from the front of
/// `input`. Unrecognized bytes stop parsing so callers can retain a partial buffer.
pub fn parse_responses(input: &[u8]) -> (Vec<Msg>, usize) {
    let mut messages = Vec::new();
    let mut consumed = 0usize;

    while consumed < input.len() {
        let slice = &input[consumed..];
        if let Some((msg, n)) = try_parse_one(slice) {
            messages.push(msg);
            consumed += n;
        } else {
            break;
        }
    }

    (messages, consumed)
}

fn try_parse_one(input: &[u8]) -> Option<(Msg, usize)> {
    if input.is_empty() {
        return None;
    }

    if input[0] == 0x1b
        && let Some(result) = try_parse_escape(input)
    {
        return Some(result);
    }

    None
}

fn try_parse_escape(input: &[u8]) -> Option<(Msg, usize)> {
    if input.len() < 2 || input[0] != 0x1b {
        return None;
    }

    match input[1] {
        b'[' => try_parse_csi(input),
        b']' => try_parse_osc(input),
        b'P' => try_parse_dcs(input),
        _ => None,
    }
}

fn try_parse_csi(input: &[u8]) -> Option<(Msg, usize)> {
    let mut i = 2usize;
    let mut private = false;
    if i < input.len() && input[i] == b'?' {
        private = true;
        i += 1;
    }

    let start = i;
    while i < input.len() {
        let b = input[i];
        if is_csi_final_byte(b, private) {
            let body = std::str::from_utf8(&input[start..i]).ok()?;
            let final_byte = b as char;
            let total = i + 1;
            return csi_to_msg(body, private, final_byte, total);
        }
        i += 1;
    }
    None
}

fn is_csi_final_byte(b: u8, private: bool) -> bool {
    match b {
        b'R' | b'q' => true,
        b'y' | b'u' | b'n' => private,
        _ => false,
    }
}

fn csi_to_msg(body: &str, private: bool, final_byte: char, total: usize) -> Option<(Msg, usize)> {
    let resp = parse_csi_response(body, private, final_byte)?;
    let msg: Msg = match resp {
        CsiResponse::CursorPosition { x, y } => Box::new(CursorPositionMsg { x, y }),
        CsiResponse::ModeReport { mode, value } => Box::new(ModeReportMsg { mode, value }),
        CsiResponse::TerminalVersion { name } => Box::new(TerminalVersionMsg { name }),
        CsiResponse::KeyboardEnhancements { flags } => Box::new(KeyboardEnhancementsMsg { flags }),
        CsiResponse::LightDark { dark } => Box::new(LightDarkMsg { dark }),
    };
    Some((msg, total))
}

fn try_parse_osc(input: &[u8]) -> Option<(Msg, usize)> {
    let mut i = 2usize;
    while i < input.len() {
        let b = input[i];
        if b == 0x07 || (b == 0x1b && i + 1 < input.len() && input[i + 1] == b'\\') {
            let body = std::str::from_utf8(&input[2..i]).ok()?;
            let total = if b == 0x07 { i + 1 } else { i + 2 };
            return osc_to_msg(body, total);
        }
        i += 1;
    }
    None
}

fn osc_to_msg(body: &str, total: usize) -> Option<(Msg, usize)> {
    let resp = parse_osc_response(body)?;
    let msg: Msg = match resp {
        OscResponse::ForegroundColor(c) => Box::new(ForegroundColorMsg(c)),
        OscResponse::BackgroundColor(c) => Box::new(BackgroundColorMsg(c)),
        OscResponse::CursorColor(c) => Box::new(CursorColorMsg(c)),
        OscResponse::Clipboard { content, selection } => {
            Box::new(ClipboardMsg { content, selection })
        }
    };
    Some((msg, total))
}

fn try_parse_dcs(input: &[u8]) -> Option<(Msg, usize)> {
    let mut i = 2usize;
    while i + 1 < input.len() {
        if input[i] == 0x1b && input[i + 1] == b'\\' {
            let body = std::str::from_utf8(&input[2..i]).ok()?;
            let total = i + 2;
            return dcs_to_msg(body, total);
        }
        i += 1;
    }
    None
}

fn dcs_to_msg(body: &str, total: usize) -> Option<(Msg, usize)> {
    let resp = parse_dcs_response(body)?;
    let msg: Msg = match resp {
        DcsResponse::Capability { content } => Box::new(CapabilityMsg { content }),
    };
    Some((msg, total))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ansi::mode::{Mode, ModeSetting};

    #[test]
    fn parses_cursor_position_report() {
        let input = b"\x1b[12;34R";
        let (msgs, n) = parse_responses(input);
        assert_eq!(n, input.len());
        let msg = msgs[0].downcast_ref::<CursorPositionMsg>().unwrap();
        assert_eq!(msg.x, 33);
        assert_eq!(msg.y, 11);
    }

    #[test]
    fn parses_mode_report() {
        let input = b"\x1b[?2026;2$y";
        let (msgs, n) = parse_responses(input);
        assert_eq!(n, input.len());
        let msg = msgs[0].downcast_ref::<ModeReportMsg>().unwrap();
        assert_eq!(msg.mode, Mode::Dec(2026));
        assert_eq!(msg.value, ModeSetting::Reset);
    }

    #[test]
    fn parses_background_color_hex() {
        let input = b"\x1b]11;#1e1e1e\x07";
        let (msgs, n) = parse_responses(input);
        assert_eq!(n, input.len());
        let msg = msgs[0].downcast_ref::<BackgroundColorMsg>().unwrap();
        assert!(msg.is_dark());
    }

    #[test]
    fn parses_clipboard_response() {
        let input = b"\x1b]52;c;aGk=\x07";
        let (msgs, n) = parse_responses(input);
        assert_eq!(n, input.len());
        let msg = msgs[0].downcast_ref::<ClipboardMsg>().unwrap();
        assert_eq!(msg.content, "hi");
        assert_eq!(msg.selection, 'c');
    }

    #[test]
    fn parses_terminal_version() {
        let input = b"\x1b[>ghostty;1.0.0;q";
        let (msgs, n) = parse_responses(input);
        assert_eq!(n, input.len());
        let msg = msgs[0].downcast_ref::<TerminalVersionMsg>().unwrap();
        assert!(msg.name.contains("ghostty"));
    }

    #[test]
    fn parses_light_dark_report() {
        let input = b"\x1b[?997;1n";
        let (msgs, n) = parse_responses(input);
        assert_eq!(n, input.len());
        let msg = msgs[0].downcast_ref::<LightDarkMsg>().unwrap();
        assert!(msg.dark);
    }
}
