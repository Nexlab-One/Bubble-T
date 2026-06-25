//! v2 keyboard message types (`KeyPressMsg`, `KeyReleaseMsg`, `Key`).
//!
//! The legacy [`KeyMsg`] struct (crossterm-shaped) remains for backward
//! compatibility while callers migrate to [`KeyPressMsg`].

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::fmt;

/// Modifier keys active during a key event (v2 bitflags).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct KeyMod(u32);

impl KeyMod {
    /// Shift modifier.
    pub const SHIFT: Self = Self(1 << 0);
    /// Alt / Option modifier.
    pub const ALT: Self = Self(1 << 1);
    /// Control modifier.
    pub const CTRL: Self = Self(1 << 2);
    /// Meta / Command modifier.
    pub const META: Self = Self(1 << 3);
    /// Hyper modifier.
    pub const HYPER: Self = Self(1 << 4);
    /// Super modifier.
    pub const SUPER: Self = Self(1 << 5);

    /// Returns whether all bits in `other` are set in `self`.
    pub fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Union of two modifier sets.
    pub fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Converts to crossterm [`KeyModifiers`].
    pub fn to_crossterm(self) -> KeyModifiers {
        let mut mods = KeyModifiers::empty();
        if self.contains(Self::SHIFT) {
            mods |= KeyModifiers::SHIFT;
        }
        if self.contains(Self::ALT) {
            mods |= KeyModifiers::ALT;
        }
        if self.contains(Self::CTRL) {
            mods |= KeyModifiers::CONTROL;
        }
        if self.contains(Self::META) {
            mods |= KeyModifiers::META;
        }
        if self.contains(Self::HYPER) {
            mods |= KeyModifiers::HYPER;
        }
        if self.contains(Self::SUPER) {
            mods |= KeyModifiers::SUPER;
        }
        mods
    }

    /// Builds from crossterm [`KeyModifiers`].
    pub fn from_crossterm(mods: KeyModifiers) -> Self {
        let mut out = Self::default();
        if mods.contains(KeyModifiers::SHIFT) {
            out = out.union(Self::SHIFT);
        }
        if mods.contains(KeyModifiers::ALT) {
            out = out.union(Self::ALT);
        }
        if mods.contains(KeyModifiers::CONTROL) {
            out = out.union(Self::CTRL);
        }
        if mods.contains(KeyModifiers::META) {
            out = out.union(Self::META);
        }
        if mods.contains(KeyModifiers::HYPER) {
            out = out.union(Self::HYPER);
        }
        if mods.contains(KeyModifiers::SUPER) {
            out = out.union(Self::SUPER);
        }
        out
    }
}

/// A single key press or release event (v2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Key {
    /// Printable text when the key represents character input.
    pub text: String,
    /// Modifier keys held during the event.
    pub r#mod: KeyMod,
    /// Logical key code (special keys and unshifted characters).
    pub code: KeyCode,
    /// Shifted key when disambiguation is available.
    pub shifted_code: Option<KeyCode>,
    /// Base layout key (PC-101) when disambiguation is available.
    pub base_code: Option<KeyCode>,
    /// Whether the terminal reports this as a repeat while held.
    pub is_repeat: bool,
}

impl Key {
    /// Human-readable key name for matching (`space` instead of `' '`).
    pub fn string(&self) -> String {
        if !self.text.is_empty() {
            return self.text.clone();
        }
        code_string(self.code)
    }

    /// Modifier-prefixed keystroke (`ctrl+shift+a`).
    pub fn keystroke(&self) -> String {
        let mut parts = Vec::new();
        if self.r#mod.contains(KeyMod::CTRL) {
            parts.push("ctrl");
        }
        if self.r#mod.contains(KeyMod::ALT) {
            parts.push("alt");
        }
        if self.r#mod.contains(KeyMod::SHIFT) {
            parts.push("shift");
        }
        if self.r#mod.contains(KeyMod::META) {
            parts.push("meta");
        }
        if self.r#mod.contains(KeyMod::HYPER) {
            parts.push("hyper");
        }
        if self.r#mod.contains(KeyMod::SUPER) {
            parts.push("super");
        }
        let key_str = self.string();
        parts.push(key_str.as_str());
        parts.join("+")
    }

    /// Builds a v2 [`Key`] from a crossterm [`KeyEvent`].
    pub fn from_crossterm(event: &KeyEvent) -> Self {
        let text = match event.code {
            KeyCode::Char(ch) => ch.to_string(),
            _ => String::new(),
        };
        Self {
            text,
            r#mod: KeyMod::from_crossterm(event.modifiers),
            code: event.code,
            shifted_code: None,
            base_code: None,
            is_repeat: matches!(event.kind, KeyEventKind::Repeat),
        }
    }

    /// Converts to the legacy crossterm-shaped [`KeyMsg`].
    pub fn to_legacy(&self) -> KeyMsg {
        KeyMsg {
            key: self.code,
            modifiers: self.r#mod.to_crossterm(),
        }
    }
}

/// v2 key-press message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyPressMsg(pub Key);

impl KeyPressMsg {
    /// Returns the underlying key event.
    pub fn key(&self) -> &Key {
        &self.0
    }

    /// Human-readable key name (`space` for space bar).
    pub fn string(&self) -> String {
        self.0.string()
    }

    /// Modifier-prefixed keystroke representation.
    pub fn keystroke(&self) -> String {
        self.0.keystroke()
    }

    /// Converts to legacy [`KeyMsg`] for transitional callers.
    pub fn to_legacy(&self) -> KeyMsg {
        self.0.to_legacy()
    }
}

impl fmt::Display for KeyPressMsg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.string())
    }
}

/// v2 key-release message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyReleaseMsg(pub Key);

impl KeyReleaseMsg {
    /// Returns the underlying key event.
    pub fn key(&self) -> &Key {
        &self.0
    }

    /// Human-readable key name.
    pub fn string(&self) -> String {
        self.0.string()
    }

    /// Modifier-prefixed keystroke representation.
    pub fn keystroke(&self) -> String {
        self.0.keystroke()
    }

    /// Converts to legacy [`KeyMsg`].
    pub fn to_legacy(&self) -> KeyMsg {
        self.0.to_legacy()
    }
}

impl fmt::Display for KeyReleaseMsg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.string())
    }
}

/// Union of press and release key events (v2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyEventMsg {
    /// Key was pressed.
    Press(KeyPressMsg),
    /// Key was released.
    Release(KeyReleaseMsg),
}

impl KeyEventMsg {
    /// Returns the underlying [`Key`], regardless of press or release.
    pub fn key(&self) -> &Key {
        match self {
            Self::Press(msg) => msg.key(),
            Self::Release(msg) => msg.key(),
        }
    }

    /// Converts to legacy [`KeyMsg`].
    pub fn to_legacy(&self) -> KeyMsg {
        match self {
            Self::Press(msg) => msg.to_legacy(),
            Self::Release(msg) => msg.to_legacy(),
        }
    }
}

/// Legacy v1-shaped key message (crossterm fields).
///
/// Prefer [`KeyPressMsg`] for new code. This type remains so existing
/// `downcast_ref::<KeyMsg>()` call sites keep compiling when input still
/// emits legacy messages, and as the return type of [`legacy_key_msg`].
#[derive(Debug, Clone)]
pub struct KeyMsg {
    /// The key code representing the physical key pressed.
    pub key: KeyCode,
    /// Modifier keys active during the key press.
    pub modifiers: KeyModifiers,
}

/// Extracts a legacy [`KeyMsg`] from any message, preferring v2 press events.
pub fn legacy_key_msg(msg: &crate::Msg) -> Option<KeyMsg> {
    if let Some(press) = msg.downcast_ref::<KeyPressMsg>() {
        return Some(press.to_legacy());
    }
    if let Some(release) = msg.downcast_ref::<KeyReleaseMsg>() {
        return Some(release.to_legacy());
    }
    msg.downcast_ref::<KeyMsg>().cloned()
}

fn code_string(code: KeyCode) -> String {
    match code {
        KeyCode::Char(' ') => "space".to_string(),
        KeyCode::Enter => "enter".to_string(),
        KeyCode::Tab => "tab".to_string(),
        KeyCode::BackTab => "shift+tab".to_string(),
        KeyCode::Backspace => "backspace".to_string(),
        KeyCode::Esc => "esc".to_string(),
        KeyCode::Insert => "insert".to_string(),
        KeyCode::Delete => "delete".to_string(),
        KeyCode::Home => "home".to_string(),
        KeyCode::End => "end".to_string(),
        KeyCode::PageUp => "pgup".to_string(),
        KeyCode::PageDown => "pgdown".to_string(),
        KeyCode::Up => "up".to_string(),
        KeyCode::Down => "down".to_string(),
        KeyCode::Left => "left".to_string(),
        KeyCode::Right => "right".to_string(),
        KeyCode::F(n) => format!("f{n}"),
        KeyCode::Char(ch) => ch.to_string(),
        KeyCode::Null => "null".to_string(),
        _ => "unknown".to_string(),
    }
}

/// Classifies a crossterm key event into press or release v2 messages.
pub fn key_event_from_crossterm(event: KeyEvent) -> Option<KeyEventMsg> {
    let key = Key::from_crossterm(&event);
    match event.kind {
        KeyEventKind::Press => Some(KeyEventMsg::Press(KeyPressMsg(key))),
        KeyEventKind::Release => Some(KeyEventMsg::Release(KeyReleaseMsg(key))),
        KeyEventKind::Repeat => Some(KeyEventMsg::Press(KeyPressMsg(Key {
            is_repeat: true,
            ..key
        }))),
    }
}

/// Best-effort parser for Kitty `CSI u` key events.
///
/// Returns `(message, bytes_consumed)` when `input` begins with a complete
/// Kitty keyboard sequence. Partial sequences return `None`.
pub fn try_parse_kitty_key(input: &[u8]) -> Option<(KeyEventMsg, usize)> {
    let (ev, n) = ansi::kitty::try_parse_key(input)?;
    Some((key_event_from_kitty(&ev), n))
}

/// Converts a decoded [`ansi::kitty::KittyKeyEvent`] into a v2 key message.
pub fn key_event_from_kitty(ev: &ansi::kitty::KittyKeyEvent) -> KeyEventMsg {
    use ansi::kitty::{KittyEventType, KittyMod};

    let mut mods = KeyMod::default();
    if ev.r#mod.contains(KittyMod::SHIFT) {
        mods = mods.union(KeyMod::SHIFT);
    }
    if ev.r#mod.contains(KittyMod::ALT) {
        mods = mods.union(KeyMod::ALT);
    }
    if ev.r#mod.contains(KittyMod::CTRL) {
        mods = mods.union(KeyMod::CTRL);
    }
    if ev.r#mod.contains(KittyMod::SUPER) {
        mods = mods.union(KeyMod::SUPER);
    }
    if ev.r#mod.contains(KittyMod::HYPER) {
        mods = mods.union(KeyMod::HYPER);
    }
    if ev.r#mod.contains(KittyMod::META) {
        mods = mods.union(KeyMod::META);
    }

    let shifted_code = ev.shifted_code.and_then(char::from_u32).map(KeyCode::Char);
    let base_code = ev.base_code.and_then(char::from_u32).map(KeyCode::Char);

    let key = Key {
        text: ev.text.clone(),
        r#mod: mods,
        code: kitty_code_to_key_code(ev.code),
        shifted_code,
        base_code,
        is_repeat: ev.event_type == KittyEventType::Repeat,
    };

    match ev.event_type {
        KittyEventType::Release => KeyEventMsg::Release(KeyReleaseMsg(key)),
        KittyEventType::Press | KittyEventType::Repeat => KeyEventMsg::Press(KeyPressMsg(key)),
    }
}

fn kitty_code_to_key_code(code: u32) -> KeyCode {
    match code {
        57344 => KeyCode::Esc,
        57345 => KeyCode::Enter,
        57346 => KeyCode::Tab,
        57347 => KeyCode::Backspace,
        57348 => KeyCode::Insert,
        57349 => KeyCode::Delete,
        57350 => KeyCode::Left,
        57351 => KeyCode::Right,
        57352 => KeyCode::Up,
        57353 => KeyCode::Down,
        57354 => KeyCode::PageUp,
        57355 => KeyCode::PageDown,
        57356 => KeyCode::Home,
        57357 => KeyCode::End,
        57364..=57399 => KeyCode::F((code - 57364 + 1) as u8),
        127 => KeyCode::Backspace,
        9 => KeyCode::Tab,
        13 => KeyCode::Enter,
        27 => KeyCode::Esc,
        c if c < 128 => KeyCode::Char(c as u8 as char),
        c if char::from_u32(c).is_some() => KeyCode::Char(char::from_u32(c).unwrap_or('\0')),
        _ => KeyCode::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

    #[test]
    fn space_string_is_space_not_char() {
        let key = Key {
            text: String::new(),
            r#mod: KeyMod::default(),
            code: KeyCode::Char(' '),
            shifted_code: None,
            base_code: None,
            is_repeat: false,
        };
        assert_eq!(key.string(), "space");
    }

    #[test]
    fn keystroke_modifier_order() {
        let key = Key {
            text: "a".to_string(),
            r#mod: KeyMod::CTRL.union(KeyMod::SHIFT).union(KeyMod::ALT),
            code: KeyCode::Char('a'),
            shifted_code: None,
            base_code: None,
            is_repeat: false,
        };
        assert_eq!(key.keystroke(), "ctrl+alt+shift+a");
    }

    #[test]
    fn legacy_key_msg_from_press() {
        let press = KeyPressMsg(Key::from_crossterm(&KeyEvent::new(
            KeyCode::Char('q'),
            KeyModifiers::CONTROL,
        )));
        let msg: crate::Msg = Box::new(press);
        let legacy = legacy_key_msg(&msg).expect("key");
        assert_eq!(legacy.key, KeyCode::Char('q'));
        assert!(legacy.modifiers.contains(KeyModifiers::CONTROL));
    }

    #[test]
    fn parse_kitty_enter_press() {
        let seq = b"\x1b[13;1:1u";
        let (msg, n) = try_parse_kitty_key(seq).expect("kitty key");
        assert_eq!(n, seq.len());
        assert!(matches!(msg, KeyEventMsg::Press(_)));
    }

    #[test]
    fn key_event_from_crossterm_release() {
        let event =
            KeyEvent::new_with_kind(KeyCode::Esc, KeyModifiers::NONE, KeyEventKind::Release);
        let parsed = key_event_from_crossterm(event).expect("event");
        assert!(matches!(parsed, KeyEventMsg::Release(_)));
    }
}
