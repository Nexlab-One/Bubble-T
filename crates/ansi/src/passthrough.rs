//! GNU Screen and tmux passthrough DCS wrappers.

use crate::c0::ESC;

/// Wraps `seq` in a GNU Screen passthrough DCS (`DCS data ST`).
///
/// When `limit` is zero, the full sequence is wrapped once. Otherwise the
/// payload is chunked to respect Screen's 768-byte limit.
pub fn screen_passthrough(seq: &str, limit: usize) -> String {
    let mut out = String::from("\x1bP");
    if limit > 0 {
        let bytes = seq.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            let end = (i + limit).min(bytes.len());
            out.push_str(&seq[i..end]);
            if end < bytes.len() {
                out.push_str("\x1b\\\x1bP");
            }
            i = end;
        }
    } else {
        out.push_str(seq);
    }
    out.push_str("\x1b\\");
    out
}

/// Wraps `seq` in a tmux passthrough DCS (`DCS tmux ; data ST`).
///
/// ESC bytes in `seq` are doubled as required by tmux.
pub fn tmux_passthrough(seq: &str) -> String {
    let mut out = String::from("\x1bPtmux;");
    for b in seq.bytes() {
        if b == ESC {
            out.push(ESC as char);
        }
        out.push(b as char);
    }
    out.push_str("\x1b\\");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tmux_doubles_esc() {
        let seq = tmux_passthrough("\x1b[31m");
        assert!(seq.contains("\x1b\x1b"));
    }
}
