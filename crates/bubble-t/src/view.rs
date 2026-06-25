//! Declarative terminal view state for Bubble Tea v2.
//!
//! Models return a [`View`] from [`Model::view`](crate::Model::view) each frame.
//! The runtime diffs view fields against the previous frame and applies terminal
//! changes (alt screen, mouse mode, focus reporting, title, cursor, etc.).

use ansi::color::Color;

use crate::{Cmd, MouseMsg};

/// A zero-based terminal cell position.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Position {
    /// Column (0-based).
    pub x: u16,
    /// Row (0-based).
    pub y: u16,
}

impl Position {
    /// Creates a position at `(x, y)`.
    pub const fn new(x: u16, y: u16) -> Self {
        Self { x, y }
    }
}

/// Mouse reporting mode declared on each frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MouseMode {
    /// No mouse events.
    #[default]
    None,
    /// Click, release, wheel, and drag events.
    CellMotion,
    /// All mouse events including movement without a button held.
    AllMotion,
}

/// Keyboard enhancement features to request from the terminal.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KeyboardEnhancements {
    /// Request key repeat and release events (Kitty "Report event types").
    pub report_event_types: bool,
    /// Request alternate (shifted/base) key codes.
    pub report_alternate_keys: bool,
    /// Report all keys as escape codes instead of plain text.
    pub report_all_keys_as_escape_codes: bool,
    /// Report associated text with key events (requires escape-code mode).
    pub report_associated_text: bool,
}

impl KeyboardEnhancements {
    /// Builds the Kitty keyboard flag bitmask for this request.
    pub(crate) fn kitty_flags(self) -> i32 {
        use ansi::kitty::{
            KITTY_DISAMBIGUATE_ESCAPE_CODES, KITTY_REPORT_ALL_KEYS_AS_ESCAPE_CODES,
            KITTY_REPORT_ALTERNATE_KEYS, KITTY_REPORT_ASSOCIATED_KEYS, KITTY_REPORT_EVENT_TYPES,
        };
        let mut flags = KITTY_DISAMBIGUATE_ESCAPE_CODES;
        if self.report_event_types {
            flags |= KITTY_REPORT_EVENT_TYPES;
        }
        if self.report_alternate_keys {
            flags |= KITTY_REPORT_ALTERNATE_KEYS;
        }
        if self.report_all_keys_as_escape_codes {
            flags |= KITTY_REPORT_ALL_KEYS_AS_ESCAPE_CODES;
        }
        if self.report_associated_text {
            flags |= KITTY_REPORT_ASSOCIATED_KEYS;
        }
        flags
    }
}

/// Cursor shape variants matching DECSCUSR.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CursorShape {
    /// Block cursor.
    #[default]
    Block,
    /// Underline cursor.
    Underline,
    /// Bar cursor.
    Bar,
}

/// Declarative cursor position and appearance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cursor {
    /// Cell position of the cursor.
    pub position: Position,
    /// Cursor shape.
    pub shape: CursorShape,
    /// Whether the cursor blinks.
    pub blink: bool,
    /// Optional cursor color.
    pub color: Option<Color>,
}

impl Cursor {
    /// Creates a cursor at `position` with default shape and no blink override.
    pub fn new(position: Position) -> Self {
        Self {
            position,
            shape: CursorShape::default(),
            blink: true,
            color: None,
        }
    }
}

/// Native terminal progress bar state (OSC 9;4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProgressBarState {
    /// Hidden / reset.
    #[default]
    None,
    /// Default progress state.
    Default,
    /// Error state.
    Error,
    /// Indeterminate animation.
    Indeterminate,
    /// Warning state.
    Warning,
}

/// Native terminal progress indicator driven by [`View::progress_bar`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgressBar {
    /// Visual state reported to the terminal.
    pub state: ProgressBarState,
    /// Completion percentage in `[0, 100]`.
    pub value: i32,
}

impl ProgressBar {
    /// Creates a progress bar with `state` and `value` (clamped to `[0, 100]`).
    pub fn new(state: ProgressBarState, value: i32) -> Self {
        Self {
            state,
            value: value.clamp(0, 100),
        }
    }

    /// Returns a default-state progress bar at `value` percent.
    pub fn default_bar(value: i32) -> Self {
        Self::new(ProgressBarState::Default, value)
    }

    /// Completion fraction in `[0.0, 1.0]` derived from [`Self::value`].
    pub fn percent(&self) -> f64 {
        f64::from(self.value) / 100.0
    }
}

/// Optional mouse handler invoked by the runtime when mouse events occur.
pub type OnMouseFn = Box<dyn Fn(MouseMsg) -> Option<Cmd> + Send>;

/// Everything about how the current frame should look and behave in the terminal.
#[derive(Default)]
pub struct View {
    /// Styled screen content rendered each frame.
    pub content: String,
    /// Optional mouse message handler.
    pub on_mouse: Option<OnMouseFn>,
    /// Optional compositor for automatic layer hit-testing on mouse events.
    pub compositor: Option<lipgloss::Compositor>,
    /// Cursor to show (`None` hides the terminal cursor).
    pub cursor: Option<Cursor>,
    /// Terminal background color (`None` leaves the terminal default).
    pub background_color: Option<Color>,
    /// Terminal foreground color (`None` leaves the terminal default).
    pub foreground_color: Option<Color>,
    /// Window title (OSC 2). Empty string leaves the title unchanged.
    pub window_title: String,
    /// Native progress bar (`None` hides the progress indicator).
    pub progress_bar: Option<ProgressBar>,
    /// Use the alternate screen buffer.
    pub alt_screen: bool,
    /// Report focus-in/focus-out events.
    pub report_focus: bool,
    /// Disable bracketed paste mode (enabled by default when false).
    pub disable_bracketed_paste: bool,
    /// Mouse reporting mode.
    pub mouse_mode: MouseMode,
    /// Keyboard enhancement features to request.
    pub keyboard_enhancements: KeyboardEnhancements,
}

impl View {
    /// Creates a view with the given content and default terminal options.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            ..Self::default()
        }
    }

    /// Replaces the view content.
    pub fn set_content(&mut self, content: impl Into<String>) {
        self.content = content.into();
    }

    /// Convenience helper for models that only need to set text content.
    pub fn from_content(content: impl Into<String>) -> Self {
        Self::new(content)
    }

    /// Builds an [`OnMouseFn`] that hit-tests `compositor` and optionally chains `user`.
    pub fn mouse_handler(
        compositor: Option<lipgloss::Compositor>,
        user: Option<OnMouseFn>,
    ) -> Option<OnMouseFn> {
        use crate::command::{Cmd, batch};
        use crate::event::{LayerMouseMsg, Msg};

        match (compositor, user) {
            (None, user) => user,
            (Some(comp), None) => Some(Box::new(move |mouse| {
                let hit = comp.hit(i32::from(mouse.x), i32::from(mouse.y));
                if hit.is_empty() {
                    return None;
                }
                Some(Box::pin(async move {
                    Some(Box::new(LayerMouseMsg {
                        layer_id: hit.id().to_string(),
                        bounds: hit.bounds(),
                        mouse,
                    }) as Msg)
                }))
            })),
            (Some(comp), Some(user)) => Some(Box::new(move |mouse| {
                let hit = comp.hit(i32::from(mouse.x), i32::from(mouse.y));
                let layer_cmd: Option<Cmd> = if hit.is_empty() {
                    None
                } else {
                    let layer_id = hit.id().to_string();
                    let bounds = hit.bounds();
                    let mouse_clone = mouse.clone();
                    Some(Box::pin(async move {
                        Some(Box::new(LayerMouseMsg {
                            layer_id,
                            bounds,
                            mouse: mouse_clone,
                        }) as Msg)
                    }))
                };
                let user_cmd = user(mouse);
                match (layer_cmd, user_cmd) {
                    (Some(a), Some(b)) => Some(batch(vec![a, b])),
                    (Some(a), None) => Some(a),
                    (None, b) => b,
                }
            })),
        }
    }
}

impl std::fmt::Debug for View {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("View")
            .field("content", &self.content)
            .field("on_mouse", &self.on_mouse.as_ref().map(|_| "<fn>"))
            .field(
                "compositor",
                &self.compositor.as_ref().map(|_| "<compositor>"),
            )
            .field("cursor", &self.cursor)
            .field("background_color", &self.background_color)
            .field("foreground_color", &self.foreground_color)
            .field("window_title", &self.window_title)
            .field("progress_bar", &self.progress_bar)
            .field("alt_screen", &self.alt_screen)
            .field("report_focus", &self.report_focus)
            .field("disable_bracketed_paste", &self.disable_bracketed_paste)
            .field("mouse_mode", &self.mouse_mode)
            .field("keyboard_enhancements", &self.keyboard_enhancements)
            .finish()
    }
}

/// Terminal-relevant subset of [`View`] used for frame-to-frame diffing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AppliedViewState {
    pub alt_screen: bool,
    pub mouse_mode: MouseMode,
    pub report_focus: bool,
    pub bracketed_paste: bool,
    pub window_title: String,
    pub cursor: Option<Cursor>,
    pub keyboard_enhancements: KeyboardEnhancements,
    pub foreground_color: Option<ansi::color::Color>,
    pub background_color: Option<ansi::color::Color>,
    pub progress_bar: Option<ProgressBar>,
}

impl Default for AppliedViewState {
    fn default() -> Self {
        Self {
            alt_screen: false,
            mouse_mode: MouseMode::None,
            report_focus: false,
            bracketed_paste: true,
            window_title: String::new(),
            cursor: None,
            keyboard_enhancements: KeyboardEnhancements::default(),
            foreground_color: None,
            background_color: None,
            progress_bar: None,
        }
    }
}

impl AppliedViewState {
    pub(crate) fn from_view(view: &View) -> Self {
        Self {
            alt_screen: view.alt_screen,
            mouse_mode: view.mouse_mode,
            report_focus: view.report_focus,
            bracketed_paste: !view.disable_bracketed_paste,
            window_title: view.window_title.clone(),
            cursor: view.cursor.clone(),
            keyboard_enhancements: view.keyboard_enhancements,
            foreground_color: view.foreground_color,
            background_color: view.background_color,
            progress_bar: view.progress_bar.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_view_sets_content() {
        let view = View::new("hello");
        assert_eq!(view.content, "hello");
        assert!(!view.alt_screen);
        assert_eq!(view.mouse_mode, MouseMode::None);
    }

    #[test]
    fn set_content_updates_text() {
        let mut view = View::new("a");
        view.set_content("b");
        assert_eq!(view.content, "b");
    }

    #[test]
    fn applied_state_bracketed_paste_default_enabled() {
        let view = View::new("");
        let applied = AppliedViewState::from_view(&view);
        assert!(applied.bracketed_paste);
    }

    #[test]
    fn applied_state_respects_disable_bracketed_paste() {
        let mut view = View::new("");
        view.disable_bracketed_paste = true;
        let applied = AppliedViewState::from_view(&view);
        assert!(!applied.bracketed_paste);
    }

    #[test]
    fn progress_bar_clamps_value() {
        let bar = ProgressBar::new(ProgressBarState::Default, 150);
        assert_eq!(bar.value, 100);
    }

    #[test]
    fn progress_bar_percent_from_value() {
        let bar = ProgressBar::default_bar(50);
        assert!((bar.percent() - 0.5).abs() < f64::EPSILON);
    }
}
