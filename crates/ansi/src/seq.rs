//! Internal helpers for assembling CSI and OSC sequences.

/// Two-byte CSI prefix (`ESC [`).
pub const CSI: &str = "\x1b[";

/// OSC prefix (`ESC ]`).
pub const OSC: &str = "\x1b]";

/// DCS prefix (`ESC P`).
pub const DCS: &str = "\x1bP";

/// String terminator (`ESC \`).
pub const ST: &str = "\x1b\\";

/// Bell terminator used by many OSC sequences.
pub const BEL: &str = "\x07";

/// Builds `CSI <n><final>` omitting `n` when it is zero or one.
pub(crate) fn csi_n(n: i32, final_byte: u8) -> String {
    let fb = final_byte as char;
    if n <= 1 {
        format!("\x1b[{fb}")
    } else {
        format!("\x1b[{n}{fb}")
    }
}

/// Builds `CSI <params><final>` where params is already formatted (may be empty).
pub(crate) fn csi_params(params: &str, final_byte: u8) -> String {
    let fb = final_byte as char;
    format!("\x1b[{params}{fb}")
}

/// Builds `CSI ?<n><final>` for DEC private sequences.
pub(crate) fn csi_dec(n: i32, final_byte: u8) -> String {
    let fb = final_byte as char;
    if n <= 1 {
        format!("\x1b[?{fb}")
    } else {
        format!("\x1b[?{n}{fb}")
    }
}

/// Builds an OSC sequence terminated with BEL.
pub(crate) fn osc_bel(body: &str) -> String {
    format!("\x1b]{body}\x07")
}

/// Builds a DCS sequence terminated with ST.
pub(crate) fn dcs_st(body: &str) -> String {
    format!("\x1bP{body}\x1b\\")
}
