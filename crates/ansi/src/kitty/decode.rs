//! Kitty keyboard protocol CSI `u` decode.
//!
//! Port of the key-event parsing used by upstream Bubble Tea when Kitty keyboard
//! enhancements are enabled.

/// Modifier keys in the Kitty keyboard protocol (1-based in CSI, 0-based here).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct KittyMod(u32);

impl KittyMod {
    /// Shift modifier.
    pub const SHIFT: Self = Self(1 << 0);
    /// Alt / Option modifier.
    pub const ALT: Self = Self(1 << 1);
    /// Control modifier.
    pub const CTRL: Self = Self(1 << 2);
    /// Super modifier.
    pub const SUPER: Self = Self(1 << 3);
    /// Hyper modifier.
    pub const HYPER: Self = Self(1 << 4);
    /// Meta modifier.
    pub const META: Self = Self(1 << 5);

    /// Returns whether all bits in `other` are set in `self`.
    pub fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Union of two modifier sets.
    pub fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Raw modifier bitmask.
    pub fn bits(self) -> u32 {
        self.0
    }
}

/// Kitty key event type when event-type reporting is enabled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KittyEventType {
    /// Key press (default).
    #[default]
    Press,
    /// Key repeat while held.
    Repeat,
    /// Key release.
    Release,
}

/// Decoded Kitty keyboard CSI `u` event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KittyKeyEvent {
    /// Primary key code (Unicode code point or Kitty functional code).
    pub code: u32,
    /// Modifier keys held during the event.
    pub r#mod: KittyMod,
    /// Press, repeat, or release.
    pub event_type: KittyEventType,
    /// Shifted key code when disambiguation is available.
    pub shifted_code: Option<u32>,
    /// Base layout key code when disambiguation is available.
    pub base_code: Option<u32>,
    /// Associated text reported with the key event.
    pub text: String,
}

/// Attempts to parse a complete Kitty `CSI u` key sequence from `input`.
///
/// Returns `(event, bytes_consumed)` when `input` begins with a complete
/// keyboard sequence. Stack/query sequences (`CSI >`, `CSI <`, `CSI =`, `CSI ?`)
/// return `None`.
pub fn try_parse_key(input: &[u8]) -> Option<(KittyKeyEvent, usize)> {
    if input.len() < 4 || input[0] != 0x1b || input[1] != b'[' {
        return None;
    }

    let mut i = 2usize;
    while i < input.len() {
        if input[i] == b'u' {
            let body = std::str::from_utf8(&input[2..i]).ok()?;
            if body.starts_with('<')
                || body.starts_with('=')
                || body.starts_with('?')
                || body.starts_with('>')
            {
                return None;
            }
            let ev = try_parse_key_csi_body(body)?;
            return Some((ev, i + 1));
        }
        i += 1;
    }
    None
}

/// Parses the CSI body between `[` and `u` (without the prefix/suffix bytes).
pub fn try_parse_key_csi_body(body: &str) -> Option<KittyKeyEvent> {
    let parts: Vec<&str> = body.split(';').collect();
    if parts.is_empty() {
        return None;
    }

    let code_part = parts[0];
    let key_code: u32 = code_part
        .split(':')
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);

    let mut mods = KittyMod::default();
    let mut event_type = KittyEventType::Press;
    let mut shifted_code = None;
    let mut base_code = None;
    let mut text = String::new();

    if parts.len() > 1 {
        let mod_part = parts[1];
        let mod_tokens: Vec<&str> = mod_part.split(':').collect();
        if let Some(m) = mod_tokens.first().and_then(|s| s.parse::<u32>().ok())
            && m > 1
        {
            mods = decode_mod(m - 1);
        }
        match mod_tokens.get(1).and_then(|s| s.parse().ok()) {
            Some(2) => event_type = KittyEventType::Repeat,
            Some(3) => event_type = KittyEventType::Release,
            _ => {}
        }
    }

    if let Some(shift_part) = code_part.split(':').nth(1)
        && let Ok(ch) = shift_part.parse::<u32>()
        && ch > 0
    {
        shifted_code = Some(ch);
    }
    if let Some(base_part) = code_part.split(':').nth(2)
        && let Ok(ch) = base_part.parse::<u32>()
        && ch > 0
    {
        base_code = Some(ch);
    }

    if parts.len() > 2 {
        for chunk in parts[2].split(':') {
            if let Ok(ch) = chunk.parse::<u32>()
                && ch > 0
                && let Some(c) = char::from_u32(ch)
            {
                text.push(c);
            }
        }
    }

    if text.is_empty()
        && key_code < 128
        && let Some(c) = char::from_u32(key_code)
    {
        if mods.contains(KittyMod::SHIFT) {
            text = c.to_uppercase().collect();
        } else if !mods.contains(KittyMod::CTRL) && !mods.contains(KittyMod::ALT) {
            text = c.to_string();
        }
    }

    Some(KittyKeyEvent {
        code: key_code,
        r#mod: mods,
        event_type,
        shifted_code,
        base_code,
        text,
    })
}

fn decode_mod(raw: u32) -> KittyMod {
    let mut mods = KittyMod::default();
    if raw & 1 != 0 {
        mods = mods.union(KittyMod::SHIFT);
    }
    if raw & 2 != 0 {
        mods = mods.union(KittyMod::ALT);
    }
    if raw & 4 != 0 {
        mods = mods.union(KittyMod::CTRL);
    }
    if raw & 8 != 0 {
        mods = mods.union(KittyMod::SUPER);
    }
    if raw & 16 != 0 {
        mods = mods.union(KittyMod::HYPER);
    }
    if raw & 32 != 0 {
        mods = mods.union(KittyMod::META);
    }
    mods
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_enter_press() {
        let seq = b"\x1b[13;1:1u";
        let (ev, n) = try_parse_key(seq).expect("kitty key");
        assert_eq!(n, seq.len());
        assert_eq!(ev.code, 13);
        assert_eq!(ev.event_type, KittyEventType::Press);
    }

    #[test]
    fn parse_release_event() {
        let ev = try_parse_key_csi_body("97;1:3").expect("body");
        assert_eq!(ev.event_type, KittyEventType::Release);
        assert_eq!(ev.code, 97);
    }

    #[test]
    fn parse_repeat_event() {
        let ev = try_parse_key_csi_body("97;1:2").expect("body");
        assert_eq!(ev.event_type, KittyEventType::Repeat);
    }

    #[test]
    fn parse_shifted_disambiguation() {
        let ev = try_parse_key_csi_body("97:65:97;2:1").expect("body");
        assert_eq!(ev.shifted_code, Some(65));
        assert_eq!(ev.base_code, Some(97));
        assert!(ev.r#mod.contains(KittyMod::SHIFT));
    }

    #[test]
    fn ignores_stack_sequences() {
        assert!(try_parse_key(b"\x1b[>1u").is_none());
        assert!(try_parse_key(b"\x1b[?u").is_none());
    }

    #[test]
    fn modifier_bits() {
        let ev = try_parse_key_csi_body("97;9:1").expect("body");
        assert!(ev.r#mod.contains(KittyMod::SUPER));
    }
}
