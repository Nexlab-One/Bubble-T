//! Cell-based "cursed" renderer over `cellbuf` + `colorprofile`.

use std::io::Write;

use ansi::background::{
    RESET_BACKGROUND_COLOR, RESET_FOREGROUND_COLOR, set_background_color, set_foreground_color,
};
use ansi::mode::{
    MODE_UNICODE_CORE, RESET_MODE_SYNCHRONIZED_OUTPUT, SET_MODE_SYNCHRONIZED_OUTPUT, reset_mode,
    set_mode,
};
use ansi::progress::{
    RESET_PROGRESS_BAR, SET_INDETERMINATE_PROGRESS_BAR, set_error_progress_bar, set_progress_bar,
    set_warning_progress_bar,
};
use cellbuf::{Screen, set_content};
use colorprofile::{Profile, Writer};

use crate::view::{ProgressBar, ProgressBarState};

/// Options applied around each rendered frame.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RenderFrameOptions {
    /// Default terminal foreground color for the frame.
    pub foreground_color: Option<ansi::color::Color>,
    /// Default terminal background color for the frame.
    pub background_color: Option<ansi::color::Color>,
    /// Native terminal progress indicator.
    pub progress_bar: Option<ProgressBar>,
}

/// Diff-based OutputContext that parses styled content into a cell grid.
pub struct CursedRenderer {
    screen: Screen,
    profile: Profile,
    sync_output: bool,
    unicode_mode: bool,
    last_options: RenderFrameOptions,
}

impl CursedRenderer {
    /// Creates a OutputContext sized for `width`×`height` with the given color profile.
    pub fn new(width: usize, height: usize, profile: Profile) -> Self {
        Self {
            screen: Screen::new(width.max(1), height.max(1)),
            profile,
            sync_output: false,
            unicode_mode: false,
            last_options: RenderFrameOptions::default(),
        }
    }

    /// Returns the active color profile.
    pub fn profile(&self) -> Profile {
        self.profile
    }

    /// Sets the color profile used for downsampling emitted ANSI.
    pub fn set_profile(&mut self, profile: Profile) {
        self.profile = profile;
    }

    /// Returns whether synchronized output (DEC mode 2026) is enabled.
    pub fn sync_output(&self) -> bool {
        self.sync_output
    }

    /// Enables or disables synchronized output wrapping on subsequent renders.
    pub fn set_sync_output(&mut self, enabled: bool) {
        self.sync_output = enabled;
    }

    /// Returns whether Unicode grapheme width mode (DEC mode 2027) is enabled.
    pub fn unicode_mode(&self) -> bool {
        self.unicode_mode
    }

    /// Enables or disables Unicode width mode tracking.
    pub fn set_unicode_mode(&mut self, enabled: bool) {
        self.unicode_mode = enabled;
    }

    /// Sequences to enable OutputContext terminal modes on startup.
    pub fn startup_mode_sequences(&self) -> String {
        let mut out = String::new();
        if self.sync_output {
            out.push_str(SET_MODE_SYNCHRONIZED_OUTPUT);
        }
        if self.unicode_mode {
            out.push_str(&set_mode(&[MODE_UNICODE_CORE]));
        }
        out
    }

    /// Sequences to restore terminal modes on shutdown.
    pub fn shutdown_mode_sequences(&self) -> String {
        let mut out = String::new();
        if self.unicode_mode {
            out.push_str(&reset_mode(&[MODE_UNICODE_CORE]));
        }
        if self.sync_output {
            out.push_str(RESET_MODE_SYNCHRONIZED_OUTPUT);
        }
        out
    }

    /// Resizes the internal screen buffers.
    pub fn resize(&mut self, width: usize, height: usize) {
        self.screen.resize(width.max(1), height.max(1));
    }

    /// Parses `content` into the cell buffer, diffs against the previous frame,
    /// and returns downsampled ANSI for the changed cells plus any view-driven
    /// terminal side effects in `options`.
    pub fn render(&mut self, content: &str, options: &RenderFrameOptions) -> String {
        set_content(self.screen.buffer(), content);
        let mut raw = self.screen.render();
        raw = prepend_view_sequences(&mut self.last_options, options.clone(), raw);
        let downsampled = downsample(&raw, self.profile);
        wrap_sync_output(&downsampled, self.sync_output)
    }
}

fn prepend_view_sequences(
    last: &mut RenderFrameOptions,
    next: RenderFrameOptions,
    raw: String,
) -> String {
    let mut prefix = String::new();

    if last.foreground_color != next.foreground_color {
        match next.foreground_color {
            Some(color) => prefix.push_str(&set_foreground_color(&color_to_osc_string(color))),
            None => prefix.push_str(RESET_FOREGROUND_COLOR),
        }
    }
    if last.background_color != next.background_color {
        match next.background_color {
            Some(color) => prefix.push_str(&set_background_color(&color_to_osc_string(color))),
            None => prefix.push_str(RESET_BACKGROUND_COLOR),
        }
    }
    if last.progress_bar != next.progress_bar {
        prefix.push_str(&progress_bar_sequence(next.progress_bar.as_ref()));
    }

    *last = next;
    if prefix.is_empty() {
        raw
    } else {
        prefix.push_str(&raw);
        prefix
    }
}

pub(crate) fn progress_bar_sequence(bar: Option<&ProgressBar>) -> String {
    match bar {
        None
        | Some(ProgressBar {
            state: ProgressBarState::None,
            ..
        }) => RESET_PROGRESS_BAR.to_string(),
        Some(ProgressBar {
            state: ProgressBarState::Default,
            value,
        }) => set_progress_bar(*value),
        Some(ProgressBar {
            state: ProgressBarState::Error,
            value,
        }) => set_error_progress_bar(*value),
        Some(ProgressBar {
            state: ProgressBarState::Indeterminate,
            ..
        }) => SET_INDETERMINATE_PROGRESS_BAR.to_string(),
        Some(ProgressBar {
            state: ProgressBarState::Warning,
            value,
        }) => set_warning_progress_bar(*value),
    }
}

fn color_to_osc_string(color: ansi::color::Color) -> String {
    use ansi::color::{Color, indexed_to_rgb};
    let rgb = match color {
        Color::Rgb(c) => c,
        Color::Indexed(i) => indexed_to_rgb(i.0),
        Color::Basic(c) => indexed_to_rgb(c as u8),
    };
    rgb_to_hex(rgb)
}

fn rgb_to_hex(c: ansi::color::RgbColor) -> String {
    format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)
}

fn wrap_sync_output(raw: &str, sync: bool) -> String {
    if !sync || raw.is_empty() {
        return raw.to_string();
    }
    format!("{SET_MODE_SYNCHRONIZED_OUTPUT}{raw}{RESET_MODE_SYNCHRONIZED_OUTPUT}")
}

fn downsample(raw: &str, profile: Profile) -> String {
    if profile == Profile::TrueColor {
        return raw.to_string();
    }

    let mut buf = Vec::with_capacity(raw.len());
    {
        let mut writer = Writer::with_profile(&mut buf, profile);
        let _ = writer.write_all(raw.as_bytes());
        let _ = writer.flush();
    }
    String::from_utf8(buf).unwrap_or_else(|_| raw.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ansi::color::RgbColor;

    #[test]
    fn renders_content_into_ansi() {
        let mut renderer = CursedRenderer::new(10, 2, Profile::TrueColor);
        let out = renderer.render("Hi", &RenderFrameOptions::default());
        assert!(out.contains('H'));
    }

    #[test]
    fn resize_updates_screen() {
        let mut renderer = CursedRenderer::new(5, 5, Profile::TrueColor);
        renderer.resize(20, 10);
        let out = renderer.render("x", &RenderFrameOptions::default());
        assert!(out.contains('x'));
    }

    #[test]
    fn diff_skips_unchanged_content() {
        let mut renderer = CursedRenderer::new(10, 2, Profile::TrueColor);
        let first = renderer.render("Hi", &RenderFrameOptions::default());
        let second = renderer.render("Hi", &RenderFrameOptions::default());
        assert!(first.contains('H'));
        assert!(second.is_empty() || !second.contains('H'));
    }

    #[test]
    fn applies_foreground_color_once() {
        let mut renderer = CursedRenderer::new(5, 1, Profile::TrueColor);
        let color = ansi::color::Color::Rgb(RgbColor { r: 255, g: 0, b: 0 });
        let opts = RenderFrameOptions {
            foreground_color: Some(color),
            ..Default::default()
        };
        let first = renderer.render("", &opts);
        assert!(first.contains("10;#ff0000"));
        let second = renderer.render("", &opts);
        assert!(!second.contains("10;#ff0000"));
    }

    #[test]
    fn progress_bar_emits_sequence() {
        let mut renderer = CursedRenderer::new(5, 1, Profile::TrueColor);
        let opts = RenderFrameOptions {
            progress_bar: Some(ProgressBar::default_bar(42)),
            ..Default::default()
        };
        let out = renderer.render("", &opts);
        assert!(out.contains("9;4;1;42"));
    }

    #[test]
    fn sync_output_wraps_non_empty_render() {
        let mut renderer = CursedRenderer::new(5, 1, Profile::TrueColor);
        renderer.set_sync_output(true);
        let out = renderer.render("x", &RenderFrameOptions::default());
        assert!(out.contains(SET_MODE_SYNCHRONIZED_OUTPUT));
        assert!(out.contains(RESET_MODE_SYNCHRONIZED_OUTPUT));
    }

    #[test]
    fn startup_modes_include_unicode_and_sync() {
        let mut renderer = CursedRenderer::new(5, 1, Profile::TrueColor);
        renderer.set_sync_output(true);
        renderer.set_unicode_mode(true);
        let seq = renderer.startup_mode_sequences();
        assert!(seq.contains(SET_MODE_SYNCHRONIZED_OUTPUT));
        assert!(seq.contains("\x1b[?2027h"));
    }
}
