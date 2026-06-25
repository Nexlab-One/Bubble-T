//! Terminal query response parsing helpers.
//!
//! These parsers mirror the response sequences emitted when the runtime issues
//! device queries (CPR, DECRPM, XTVERSION, XTGETTCAP, OSC color/clipboard).

use crate::color::RgbColor;
use crate::mode::{Mode, ModeSetting};
use base64::{Engine as _, engine::general_purpose::STANDARD};

/// Parsed CSI query response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CsiResponse {
    /// Cursor position report (`CSI Pl ; Pc R`).
    CursorPosition {
        /// Zero-based row.
        y: u16,
        /// Zero-based column.
        x: u16,
    },
    /// DEC mode report (`CSI ? Ps ; Pv $y`).
    ModeReport {
        /// Reported mode.
        mode: Mode,
        /// Reported setting.
        value: ModeSetting,
    },
    /// Terminal name/version (`CSI > name ; version q`).
    TerminalVersion {
        /// Reported name/version string.
        name: String,
    },
    /// Kitty keyboard enhancements flags (`CSI ? flags u`).
    KeyboardEnhancements {
        /// Supported enhancement flags.
        flags: i32,
    },
    /// Light/dark preference (`CSI ? 997 ; mode n`).
    LightDark {
        /// `true` when the terminal reports dark mode.
        dark: bool,
    },
}

/// Parsed OSC query response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OscResponse {
    /// Default foreground color (OSC 10).
    ForegroundColor(RgbColor),
    /// Default background color (OSC 11).
    BackgroundColor(RgbColor),
    /// Cursor color (OSC 12).
    CursorColor(RgbColor),
    /// Clipboard contents (OSC 52).
    Clipboard {
        /// Selection: `c` clipboard, `p` primary.
        selection: char,
        /// Decoded clipboard text.
        content: String,
    },
}

/// Parsed DCS query response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DcsResponse {
    /// Termcap/Terminfo capability (XTGETTCAP).
    Capability {
        /// Capability payload.
        content: String,
    },
}

/// Parses an OSC color payload (`#RRGGBB` or `rgb:RR/GG/BB`).
pub fn parse_osc_color(payload: &str) -> Option<RgbColor> {
    let trimmed = payload.trim();
    if trimmed.starts_with('#') && trimmed.len() >= 7 {
        let r = u8::from_str_radix(&trimmed[1..3], 16).ok()?;
        let g = u8::from_str_radix(&trimmed[3..5], 16).ok()?;
        let b = u8::from_str_radix(&trimmed[5..7], 16).ok()?;
        return Some(RgbColor { r, g, b });
    }
    if let Some(rgb) = trimmed.strip_prefix("rgb:") {
        let parts: Vec<&str> = rgb.split('/').collect();
        if parts.len() == 3 {
            let r = parse_xparse_component(parts[0])?;
            let g = parse_xparse_component(parts[1])?;
            let b = parse_xparse_component(parts[2])?;
            return Some(RgbColor { r, g, b });
        }
    }
    None
}

fn parse_xparse_component(s: &str) -> Option<u8> {
    let v = u16::from_str_radix(s, 16).ok()?;
    Some((v >> 8) as u8)
}

/// Parses a CSI query response body and final byte.
pub fn parse_csi_response(body: &str, private: bool, final_byte: char) -> Option<CsiResponse> {
    match final_byte {
        'R' if !private => {
            let (row, col) = parse_two_params(body)?;
            Some(CsiResponse::CursorPosition {
                x: col.saturating_sub(1),
                y: row.saturating_sub(1),
            })
        }
        'y' if private && body.ends_with('$') => {
            let inner = body.strip_suffix('$')?;
            let (mode_str, value_str) = inner.split_once(';')?;
            let mode_num: i32 = mode_str.parse().ok()?;
            let value_num: u8 = value_str.parse().ok()?;
            Some(CsiResponse::ModeReport {
                mode: Mode::Dec(mode_num),
                value: mode_setting_from_report(value_num),
            })
        }
        'q' => {
            let name = body.trim_start_matches(['>', '?']).to_string();
            Some(CsiResponse::TerminalVersion { name })
        }
        'u' if private => {
            let flags: i32 = body.parse().ok()?;
            Some(CsiResponse::KeyboardEnhancements { flags })
        }
        'n' if private => {
            let (code, value) = parse_two_params(body)?;
            if code == 997 {
                Some(CsiResponse::LightDark { dark: value == 1 })
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Parses an OSC query response body (`prefix;payload`).
pub fn parse_osc_response(body: &str) -> Option<OscResponse> {
    let (prefix, payload) = body.split_once(';')?;
    match prefix {
        "10" => parse_osc_color(payload).map(OscResponse::ForegroundColor),
        "11" => parse_osc_color(payload).map(OscResponse::BackgroundColor),
        "12" => parse_osc_color(payload).map(OscResponse::CursorColor),
        "52" => {
            let (sel, data) = payload.split_once(';')?;
            let selection = sel.chars().next().unwrap_or('c');
            let content = if data == "?" || data.is_empty() {
                String::new()
            } else {
                STANDARD
                    .decode(data)
                    .ok()
                    .and_then(|b| String::from_utf8(b).ok())?
            };
            Some(OscResponse::Clipboard { selection, content })
        }
        _ => None,
    }
}

/// Parses an XTGETTCAP DCS response body.
pub fn parse_dcs_response(body: &str) -> Option<DcsResponse> {
    if let Some(content) = body.strip_prefix("+r").or_else(|| body.strip_prefix("1+r")) {
        return Some(DcsResponse::Capability {
            content: content.to_string(),
        });
    }
    if let Some(hex) = body.strip_prefix("+q")
        && let Ok(bytes) = hex_decode_pairs(hex)
        && let Ok(text) = String::from_utf8(bytes)
    {
        return Some(DcsResponse::Capability { content: text });
    }
    None
}

fn mode_setting_from_report(value: u8) -> ModeSetting {
    match value {
        0 => ModeSetting::NotRecognized,
        1 => ModeSetting::Set,
        2 => ModeSetting::Reset,
        3 => ModeSetting::PermanentlySet,
        4 => ModeSetting::PermanentlyReset,
        _ => ModeSetting::NotRecognized,
    }
}

fn parse_two_params(body: &str) -> Option<(u16, u16)> {
    let (a, b) = body.split_once(';')?;
    let row: u16 = a.parse().ok()?;
    let col: u16 = b.parse().ok()?;
    Some((row, col))
}

fn hex_decode_pairs(hex: &str) -> Result<Vec<u8>, ()> {
    let hex = hex.trim_end_matches(';');
    if !hex.len().is_multiple_of(2) {
        return Err(());
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|_| ()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_light_dark_report() {
        let resp = parse_csi_response("997;1", true, 'n').unwrap();
        assert_eq!(resp, CsiResponse::LightDark { dark: true });
    }

    #[test]
    fn parses_osc_background() {
        let resp = parse_osc_response("11;#1e1e1e").unwrap();
        assert!(matches!(resp, OscResponse::BackgroundColor(_)));
    }
}
