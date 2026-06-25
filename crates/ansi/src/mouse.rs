//! Mouse event encoding and SGR/X10 decoding.
//!
//! Port of [`charmbracelet/x/ansi`] mouse helpers used for tests, simulation,
//! and input layers that parse raw escape sequences.
//!
//! [`charmbracelet/x/ansi`]: https://github.com/charmbracelet/x/tree/main/ansi

/// Mouse button identifiers (X11 numbering).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum MouseButton {
    /// No button / release.
    #[default]
    None = 0,
    /// Left button.
    Left = 1,
    /// Middle button.
    Middle = 2,
    /// Right button.
    Right = 3,
    /// Scroll wheel up.
    WheelUp = 4,
    /// Scroll wheel down.
    WheelDown = 5,
    /// Scroll wheel left.
    WheelLeft = 6,
    /// Scroll wheel right.
    WheelRight = 7,
    /// Browser back.
    Backward = 8,
    /// Browser forward.
    Forward = 9,
    /// Extended button 10.
    Button10 = 10,
    /// Extended button 11.
    Button11 = 11,
}

impl MouseButton {
    /// Returns a human-readable button name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Left => "left",
            Self::Middle => "middle",
            Self::Right => "right",
            Self::WheelUp => "wheelup",
            Self::WheelDown => "wheeldown",
            Self::WheelLeft => "wheelleft",
            Self::WheelRight => "wheelright",
            Self::Backward => "backward",
            Self::Forward => "forward",
            Self::Button10 => "button10",
            Self::Button11 => "button11",
        }
    }
}

/// Decoded SGR or X10 mouse event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MouseEvent {
    /// Button pressed or wheel direction.
    pub button: MouseButton,
    /// Zero-based column.
    pub x: i32,
    /// Zero-based row.
    pub y: i32,
    /// Shift modifier held.
    pub shift: bool,
    /// Alt modifier held.
    pub alt: bool,
    /// Control modifier held.
    pub ctrl: bool,
    /// Motion/drag event.
    pub motion: bool,
    /// Wheel event.
    pub wheel: bool,
    /// Button release (SGR `m` suffix).
    pub release: bool,
}

const BIT_SHIFT: u8 = 0b0000_0100;
const BIT_ALT: u8 = 0b0000_1000;
const BIT_CTRL: u8 = 0b0001_0000;
const BIT_MOTION: u8 = 0b0010_0000;
const BIT_WHEEL: u8 = 0b0100_0000;
const BIT_ADD: u8 = 0b1000_0000;
const BITS_MASK: u8 = 0b0000_0011;

/// X10 mouse coordinate offset.
pub const X10_OFFSET: u8 = 32;

/// Encodes a mouse button byte for X10/SGR reporting.
pub fn encode_mouse_button(
    button: MouseButton,
    motion: bool,
    shift: bool,
    alt: bool,
    ctrl: bool,
) -> u8 {
    let mut m = match button {
        MouseButton::None => BITS_MASK,
        MouseButton::Left => 0,
        MouseButton::Middle => 1,
        MouseButton::Right => 2,
        MouseButton::WheelUp => BIT_WHEEL,
        MouseButton::WheelDown => 1 | BIT_WHEEL,
        MouseButton::WheelLeft => 2 | BIT_WHEEL,
        MouseButton::WheelRight => 3 | BIT_WHEEL,
        MouseButton::Backward => BIT_ADD,
        MouseButton::Forward => 1 | BIT_ADD,
        MouseButton::Button10 => 2 | BIT_ADD,
        MouseButton::Button11 => 3 | BIT_ADD,
    };

    if shift {
        m |= BIT_SHIFT;
    }
    if alt {
        m |= BIT_ALT;
    }
    if ctrl {
        m |= BIT_CTRL;
    }
    if motion {
        m |= BIT_MOTION;
    }
    m
}

/// Builds an X10 mouse sequence (`CSI M Cb Cx Cy`).
pub fn mouse_x10(button_byte: u8, x: i32, y: i32) -> String {
    format!(
        "\x1b[M{}{}{}",
        char::from(X10_OFFSET.wrapping_add(button_byte)),
        char::from(X10_OFFSET.wrapping_add((x as u8).saturating_add(1))),
        char::from(X10_OFFSET.wrapping_add((y as u8).saturating_add(1)))
    )
}

/// Builds an SGR mouse sequence (`CSI < Cb ; Cx ; Cy M|m`).
pub fn mouse_sgr(button_byte: u8, x: i32, y: i32, release: bool) -> String {
    let suffix = if release { 'm' } else { 'M' };
    let x = x.abs();
    let y = y.abs();
    format!("\x1b[<{button_byte};{};{}{suffix}", x + 1, y + 1)
}

/// Decodes an SGR mouse CSI body (`<Cb;Cx;Cy` with final `M` or `m`).
pub fn decode_sgr_mouse(params: &[i32], release: bool) -> Option<MouseEvent> {
    if params.len() < 3 {
        return None;
    }
    decode_button_byte(params[0] as u8, params[1] - 1, params[2] - 1, release)
}

/// Decodes an X10 mouse payload (three bytes after `ESC M`).
pub fn decode_x10_mouse(bytes: &[u8]) -> Option<MouseEvent> {
    if bytes.len() < 3 {
        return None;
    }
    let b = bytes[0].wrapping_sub(X10_OFFSET);
    let x = i32::from(bytes[1].wrapping_sub(X10_OFFSET).saturating_sub(1));
    let y = i32::from(bytes[2].wrapping_sub(X10_OFFSET).saturating_sub(1));
    decode_button_byte(b, x, y, b & BITS_MASK == BITS_MASK)
}

fn decode_button_byte(b: u8, x: i32, y: i32, release: bool) -> Option<MouseEvent> {
    let shift = b & BIT_SHIFT != 0;
    let alt = b & BIT_ALT != 0;
    let ctrl = b & BIT_CTRL != 0;
    let motion = b & BIT_MOTION != 0;
    let wheel = b & BIT_WHEEL != 0;
    let add = b & BIT_ADD != 0;
    let bits = b & BITS_MASK;

    let button = if add {
        match bits {
            0 => MouseButton::Backward,
            1 => MouseButton::Forward,
            2 => MouseButton::Button10,
            3 => MouseButton::Button11,
            _ => MouseButton::None,
        }
    } else if wheel {
        match bits {
            0 => MouseButton::WheelUp,
            1 => MouseButton::WheelDown,
            2 => MouseButton::WheelLeft,
            3 => MouseButton::WheelRight,
            _ => MouseButton::None,
        }
    } else {
        match bits {
            0 => MouseButton::Left,
            1 => MouseButton::Middle,
            2 => MouseButton::Right,
            3 if motion => MouseButton::None,
            3 => MouseButton::None,
            _ => MouseButton::None,
        }
    };

    Some(MouseEvent {
        button,
        x,
        y,
        shift,
        alt,
        ctrl,
        motion,
        wheel,
        release: release || (button == MouseButton::None && !motion),
    })
}

/// Attempts to parse a complete mouse escape sequence from `input`.
///
/// Returns `(event, bytes_consumed)` when `input` begins with an X10 or SGR mouse
/// report. Partial sequences return `None`.
pub fn try_parse_mouse(input: &[u8]) -> Option<(MouseEvent, usize)> {
    if input.len() >= 6 && input[0] == 0x1b && input[1] == b'[' && input[2] == b'<' {
        return parse_sgr_sequence(input);
    }
    if input.len() >= 6 && input[0] == 0x1b && input[1] == b'[' && input[2] == b'M' {
        return decode_x10_mouse(&input[3..6]).map(|ev| (ev, 6));
    }
    if input.len() >= 5 && input[0] == 0x1b && input[1] == b'M' {
        return decode_x10_mouse(&input[2..5]).map(|ev| (ev, 5));
    }
    None
}

fn parse_sgr_sequence(input: &[u8]) -> Option<(MouseEvent, usize)> {
    let mut i = 3usize;
    let mut parts: Vec<i32> = Vec::new();
    let mut current = String::new();
    while i < input.len() {
        let c = input[i];
        if c == b';' {
            parts.push(current.parse().ok()?);
            current.clear();
            i += 1;
            continue;
        }
        if c == b'M' || c == b'm' {
            parts.push(current.parse().ok()?);
            let ev = decode_sgr_mouse(&parts, c == b'm')?;
            return Some((ev, i + 1));
        }
        current.push(c as char);
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_left_button() {
        assert_eq!(
            encode_mouse_button(MouseButton::Left, false, false, false, false),
            0
        );
    }

    #[test]
    fn sgr_roundtrip() {
        let b = encode_mouse_button(MouseButton::Left, false, true, false, false);
        let seq = mouse_sgr(b, 10, 20, false);
        let (ev, n) = try_parse_mouse(seq.as_bytes()).unwrap();
        assert_eq!(n, seq.len());
        assert_eq!(ev.button, MouseButton::Left);
        assert_eq!(ev.x, 10);
        assert_eq!(ev.y, 20);
        assert!(ev.shift);
        assert!(!ev.release);
    }

    #[test]
    fn sgr_release_suffix() {
        let b = encode_mouse_button(MouseButton::None, false, false, false, false);
        let seq = mouse_sgr(b, 5, 5, true);
        let (ev, _) = try_parse_mouse(seq.as_bytes()).unwrap();
        assert!(ev.release);
    }

    #[test]
    fn x10_roundtrip() {
        let b = encode_mouse_button(MouseButton::Right, false, false, false, false);
        let seq = mouse_x10(b, 3, 7);
        let (ev, n) = try_parse_mouse(seq.as_bytes()).unwrap();
        assert_eq!(n, seq.len());
        assert_eq!(ev.button, MouseButton::Right);
        assert_eq!(ev.x, 3);
        assert_eq!(ev.y, 7);
    }
}
