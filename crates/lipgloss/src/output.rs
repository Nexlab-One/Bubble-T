//! Output-layer color profile integration (Lip Gloss v2).
//!
//! Terminal detection, background brightness, and ANSI downsampling live here.
//! All styled rendering resolves colors through [`OutputContext`].

use colorprofile::{Profile, Writer, detect, env_profile};
use std::io::IsTerminal;
use std::sync::{OnceLock, RwLock};

/// Color profiles supported when resolving styled tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorProfileKind {
    /// 24-bit true color.
    TrueColor,
    /// 256-color palette.
    ANSI256,
    /// 16-color ANSI palette.
    ANSI,
    /// No color support.
    NoColor,
}

impl From<Profile> for ColorProfileKind {
    fn from(p: Profile) -> Self {
        match p {
            Profile::TrueColor => Self::TrueColor,
            Profile::Ansi256 => Self::ANSI256,
            Profile::Ansi => Self::ANSI,
            Profile::Ascii | Profile::NoTty => Self::NoColor,
        }
    }
}

impl From<ColorProfileKind> for Profile {
    fn from(k: ColorProfileKind) -> Self {
        match k {
            ColorProfileKind::TrueColor => Profile::TrueColor,
            ColorProfileKind::ANSI256 => Profile::Ansi256,
            ColorProfileKind::ANSI => Profile::Ansi,
            ColorProfileKind::NoColor => Profile::Ascii,
        }
    }
}

/// Output stream capabilities (termenv-like).
#[derive(Debug, Clone)]
pub struct Output {
    /// Whether ANSI escape sequences are supported.
    pub supports_ansi: bool,
    /// Whether the stream behaves like a TTY.
    pub is_tty_like: bool,
}

/// Runtime output settings for styled string emission.
#[derive(Debug, Clone)]
pub struct OutputContext {
    profile: Profile,
    has_dark_background: bool,
    output: Option<Output>,
    explicit_profile: bool,
    explicit_background: bool,
}

impl Default for OutputContext {
    fn default() -> Self {
        Self::from_env()
    }
}

impl OutputContext {
    /// Creates a context with an explicit profile.
    pub fn new(profile: Profile) -> Self {
        Self {
            profile,
            has_dark_background: true,
            output: Some(detect_output()),
            explicit_profile: true,
            explicit_background: false,
        }
    }

    /// Creates a context from the current process environment.
    pub fn from_env() -> Self {
        let env: Vec<(String, String)> = std::env::vars().collect();
        let pairs: Vec<(&str, &str)> = env.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        Self {
            profile: detect(true, &pairs),
            has_dark_background: detect_dark_background(),
            output: Some(detect_output()),
            explicit_profile: false,
            explicit_background: false,
        }
    }

    /// Creates a context with custom output settings.
    pub fn with_output(output: Output) -> Self {
        let mut ctx = Self::from_env();
        ctx.output = Some(output);
        ctx
    }

    /// Returns the active [`Profile`].
    pub fn profile(&self) -> Profile {
        self.profile
    }

    /// Returns the legacy [`ColorProfileKind`] view of the profile.
    pub fn color_profile(&self) -> ColorProfileKind {
        self.profile.into()
    }

    /// Sets the color profile explicitly.
    pub fn set_color_profile(&mut self, kind: ColorProfileKind) {
        self.profile = kind.into();
        self.explicit_profile = true;
    }

    /// Sets whether the terminal background reads as dark.
    pub fn set_has_dark_background(&mut self, dark: bool) {
        self.has_dark_background = dark;
        self.explicit_background = true;
    }

    /// Returns whether the terminal background reads as dark.
    pub fn has_dark_background(&self) -> bool {
        self.has_dark_background
    }

    /// Returns a clone of the output configuration.
    pub fn output(&self) -> Option<Output> {
        self.output.clone()
    }

    /// Downsamples styled output for the active profile.
    pub fn downsample(&self, styled: &str) -> String {
        let mut out = Vec::new();
        let mut writer = Writer::with_profile(&mut out, self.profile);
        let _ = std::io::Write::write_all(&mut writer, styled.as_bytes());
        String::from_utf8(out).unwrap_or_else(|_| styled.to_string())
    }
}

static DEFAULT_OUTPUT: OnceLock<RwLock<OutputContext>> = OnceLock::new();

fn default_output_cell() -> &'static RwLock<OutputContext> {
    DEFAULT_OUTPUT.get_or_init(|| RwLock::new(OutputContext::from_env()))
}

/// Returns the process-wide default output context.
pub fn default_output() -> OutputContext {
    default_output_cell()
        .read()
        .map(|g| g.clone())
        .unwrap_or_else(|_| OutputContext::from_env())
}

/// Replaces the global default output context.
pub fn set_default_output(ctx: OutputContext) {
    if let Ok(mut guard) = default_output_cell().write() {
        *guard = ctx;
    }
}

/// Returns the default color profile kind.
pub fn color_profile() -> ColorProfileKind {
    default_output().color_profile()
}

/// Sets the default color profile kind.
pub fn set_color_profile(p: ColorProfileKind) {
    if let Ok(mut guard) = default_output_cell().write() {
        guard.set_color_profile(p);
    }
}

/// Returns whether the default context assumes a dark background.
pub fn has_dark_background() -> bool {
    default_output().has_dark_background()
}

/// Sets the default dark-background flag.
pub fn set_has_dark_background(b: bool) {
    if let Ok(mut guard) = default_output_cell().write() {
        guard.set_has_dark_background(b);
    }
}

/// Returns the env-only profile baseline (no terminfo/tmux probing).
pub fn env_only_profile(environ: &[(&str, &str)]) -> Profile {
    env_profile(environ)
}

fn detect_dark_background() -> bool {
    if let Ok(val) = std::env::var("COLORFGBG") {
        let parts: Vec<&str> = val.split(';').collect();
        if let Some(bg_str) = parts.last()
            && let Ok(bg) = bg_str.parse::<u8>()
        {
            return bg <= 6;
        }
    }
    true
}

fn detect_output() -> Output {
    let is_tty_like = std::io::stdout().is_terminal();
    let no_color = std::env::var("NO_COLOR").is_ok();
    Output {
        supports_ansi: is_tty_like && !no_color,
        is_tty_like,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_kind_roundtrip() {
        assert_eq!(
            ColorProfileKind::from(Profile::Ansi256),
            ColorProfileKind::ANSI256
        );
        assert_eq!(
            Profile::from(ColorProfileKind::TrueColor),
            Profile::TrueColor
        );
    }

    #[test]
    fn default_output_has_profile() {
        let ctx = default_output();
        let _ = ctx.profile();
    }
}
