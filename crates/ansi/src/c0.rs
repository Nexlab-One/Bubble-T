//! C0 control characters (0x00–0x1F).

/// Null character (`\0`).
pub const NUL: u8 = 0x00;
/// Start of heading.
pub const SOH: u8 = 0x01;
/// Start of text.
pub const STX: u8 = 0x02;
/// End of text.
pub const ETX: u8 = 0x03;
/// End of transmission.
pub const EOT: u8 = 0x04;
/// Enquiry.
pub const ENQ: u8 = 0x05;
/// Acknowledge.
pub const ACK: u8 = 0x06;
/// Bell (`\a`).
pub const BEL: u8 = 0x07;
/// Backspace (`\b`).
pub const BS: u8 = 0x08;
/// Horizontal tab (`\t`).
pub const HT: u8 = 0x09;
/// Line feed (`\n`).
pub const LF: u8 = 0x0A;
/// Vertical tab (`\v`).
pub const VT: u8 = 0x0B;
/// Form feed (`\f`).
pub const FF: u8 = 0x0C;
/// Carriage return (`\r`).
pub const CR: u8 = 0x0D;
/// Shift out.
pub const SO: u8 = 0x0E;
/// Shift in.
pub const SI: u8 = 0x0F;
/// Data link escape.
pub const DLE: u8 = 0x10;
/// Device control 1.
pub const DC1: u8 = 0x11;
/// Device control 2.
pub const DC2: u8 = 0x12;
/// Device control 3.
pub const DC3: u8 = 0x13;
/// Device control 4.
pub const DC4: u8 = 0x14;
/// Negative acknowledge.
pub const NAK: u8 = 0x15;
/// Synchronous idle.
pub const SYN: u8 = 0x16;
/// End of transmission block.
pub const ETB: u8 = 0x17;
/// Cancel.
pub const CAN: u8 = 0x18;
/// End of medium.
pub const EM: u8 = 0x19;
/// Substitute.
pub const SUB: u8 = 0x1A;
/// Escape (`\x1b`).
pub const ESC: u8 = 0x1B;
/// File separator.
pub const FS: u8 = 0x1C;
/// Group separator.
pub const GS: u8 = 0x1D;
/// Record separator.
pub const RS: u8 = 0x1E;
/// Unit separator.
pub const US: u8 = 0x1F;
/// Delete character.
pub const DEL: u8 = 0x7F;

/// Locking shift 0 (alias for [`SI`]).
pub const LS0: u8 = SI;
/// Locking shift 1 (alias for [`SO`]).
pub const LS1: u8 = SO;
