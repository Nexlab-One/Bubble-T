//! C1 control characters (0x80–0x9F).

/// Padding.
pub const PAD: u8 = 0x80;
/// High octet preset.
pub const HOP: u8 = 0x81;
/// Break permitted here.
pub const BPH: u8 = 0x82;
/// No break here.
pub const NBH: u8 = 0x83;
/// Index.
pub const IND: u8 = 0x84;
/// Next line.
pub const NEL: u8 = 0x85;
/// Start of selected area.
pub const SSA: u8 = 0x86;
/// End of selected area.
pub const ESA: u8 = 0x87;
/// Horizontal tab set.
pub const HTS: u8 = 0x88;
/// Horizontal tab with justification.
pub const HTJ: u8 = 0x89;
/// Vertical tab set.
pub const VTS: u8 = 0x8A;
/// Partial line forward.
pub const PLD: u8 = 0x8B;
/// Partial line backward.
pub const PLU: u8 = 0x8C;
/// Reverse index.
pub const RI: u8 = 0x8D;
/// Single shift 2.
pub const SS2: u8 = 0x8E;
/// Single shift 3.
pub const SS3: u8 = 0x8F;
/// Device control string.
pub const DCS: u8 = 0x90;
/// Private use 1.
pub const PU1: u8 = 0x91;
/// Private use 2.
pub const PU2: u8 = 0x92;
/// Set transmit state.
pub const STS: u8 = 0x93;
/// Cancel character.
pub const CCH: u8 = 0x94;
/// Message waiting.
pub const MW: u8 = 0x95;
/// Start of guarded area.
pub const SPA: u8 = 0x96;
/// End of guarded area.
pub const EPA: u8 = 0x97;
/// Start of string.
pub const SOS: u8 = 0x98;
/// Single graphic character introducer.
pub const SGCI: u8 = 0x99;
/// Single character introducer.
pub const SCI: u8 = 0x9A;
/// Control sequence introducer.
pub const CSI: u8 = 0x9B;
/// String terminator.
pub const ST: u8 = 0x9C;
/// Operating system command.
pub const OSC: u8 = 0x9D;
/// Privacy message.
pub const PM: u8 = 0x9E;
/// Application program command.
pub const APC: u8 = 0x9F;
