//! Environment-based color profile detection.

use crate::Profile;
use crate::terminfo::terminfo_profile;
use crate::tmux::tmux_profile;

#[cfg(not(windows))]
use crate::env_other::windows_color_profile;
#[cfg(windows)]
use crate::env_windows::windows_color_profile;

const DUMB_TERM: &str = "dumb";

/// Returns the color profile inferred from environment variables alone.
///
/// This respects `NO_COLOR`, `CLICOLOR`, `CLICOLOR_FORCE`, `COLORTERM`, and `TERM`
/// following the rules documented at [no-color.org](https://no-color.org/) and
/// [bixense.com/clicolors](https://bixense.com/clicolors/).
pub fn env_profile(environ: &[(&str, &str)]) -> Profile {
    color_profile(true, environ)
}

/// Detects the color profile from `is_tty` and `environ`.
///
/// When `is_tty` is false, the profile is [`Profile::NoTty`] unless
/// `CLICOLOR_FORCE=1` or `TTY_FORCE=1` is set. When inside a real terminal,
/// terminfo and tmux capabilities can raise the profile above the env baseline.
pub fn detect(is_tty: bool, environ: &[(&str, &str)]) -> Profile {
    let term = lookup(environ, "TERM");
    let is_dumb = term.is_none_or(|t| t == DUMB_TERM);
    let envp = color_profile(is_tty, environ);

    if envp == Profile::TrueColor || env_no_color(environ) {
        return envp;
    }

    if is_tty && !is_dumb {
        let term_name = term.unwrap_or("");
        let tip = terminfo_profile(term_name);
        let tmuxp = tmux_profile(environ);
        return max_profile(envp, max_profile(tip, tmuxp));
    }

    envp
}

fn color_profile(is_tty: bool, environ: &[(&str, &str)]) -> Profile {
    let term = lookup(environ, "TERM");
    let is_dumb = term == Some(DUMB_TERM) || (term.is_none() && !cfg!(windows));
    let mut profile = if !is_tty || is_dumb {
        Profile::NoTty
    } else {
        env_color_profile(environ)
    };

    if env_no_color(environ) && is_tty && profile > Profile::Ascii {
        profile = Profile::Ascii;
        return profile;
    }

    if cli_color_forced(environ) {
        if profile < Profile::Ansi {
            profile = Profile::Ansi;
        }
        let envp = env_color_profile(environ);
        if envp > profile {
            profile = envp;
        }
        return profile;
    }

    if cli_color(environ) && is_tty && !is_dumb && profile < Profile::Ansi {
        profile = Profile::Ansi;
    }

    profile
}

fn env_color_profile(environ: &[(&str, &str)]) -> Profile {
    let term = lookup(environ, "TERM");

    if term == Some(DUMB_TERM) {
        return Profile::NoTty;
    }

    let mut profile = if term.is_none_or(|t| t.is_empty()) {
        Profile::NoTty
    } else {
        Profile::Ansi
    };

    if term.is_none() || term.is_some_and(str::is_empty) {
        if let Some(wcp) = windows_color_profile(environ) {
            profile = wcp;
        }
    } else if let Some(term) = term {
        profile = terminal_name_profile(term, profile);
    }

    if lookup(environ, "WT_SESSION").is_some() {
        return Profile::TrueColor;
    }

    if lookup(environ, "GOOGLE_CLOUD_SHELL").is_some_and(parse_bool) {
        return Profile::TrueColor;
    }

    let term = term.unwrap_or("");
    if color_term(environ) && !term.starts_with("screen") && !term.starts_with("tmux") {
        return Profile::TrueColor;
    }

    if term.ends_with("256color") && profile < Profile::Ansi256 {
        profile = Profile::Ansi256;
    }

    if term.ends_with("direct") {
        return Profile::TrueColor;
    }

    profile
}

fn terminal_name_profile(term: &str, mut profile: Profile) -> Profile {
    if matches!(
        term,
        t if t.contains("alacritty")
            || t.contains("contour")
            || t.contains("foot")
            || t.contains("ghostty")
            || t.contains("kitty")
            || t.contains("rio")
            || t.contains("st")
            || t.contains("wezterm")
    ) {
        return Profile::TrueColor;
    }

    if term.starts_with("tmux") || term.starts_with("screen") {
        if profile < Profile::Ansi256 {
            profile = Profile::Ansi256;
        }
    } else if term.starts_with("xterm") && profile < Profile::Ansi {
        profile = Profile::Ansi;
    }

    profile
}

fn max_profile(a: Profile, b: Profile) -> Profile {
    if a > b { a } else { b }
}

fn lookup<'a>(environ: &'a [(&str, &str)], key: &str) -> Option<&'a str> {
    environ.iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
}

fn parse_bool(s: &str) -> bool {
    matches!(s, "1" | "true" | "TRUE" | "yes" | "YES")
}

fn env_no_color(environ: &[(&str, &str)]) -> bool {
    lookup(environ, "NO_COLOR").is_some_and(parse_bool)
}

fn cli_color(environ: &[(&str, &str)]) -> bool {
    lookup(environ, "CLICOLOR").is_some_and(parse_bool)
}

fn cli_color_forced(environ: &[(&str, &str)]) -> bool {
    lookup(environ, "CLICOLOR_FORCE").is_some_and(parse_bool)
}

pub(crate) fn is_tty_forced(environ: &[(&str, &str)]) -> bool {
    lookup(environ, "TTY_FORCE").is_some_and(parse_bool)
}

fn color_term(environ: &[(&str, &str)]) -> bool {
    let ct = lookup(environ, "COLORTERM")
        .unwrap_or("")
        .to_ascii_lowercase();
    matches!(ct.as_str(), "truecolor" | "24bit" | "yes" | "true")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truecolor_from_colorterm() {
        let env = [("COLORTERM", "truecolor"), ("TERM", "xterm-256color")];
        assert_eq!(env_profile(&env), Profile::TrueColor);
    }

    #[test]
    fn no_color_downgrades_to_ascii() {
        let env = [
            ("NO_COLOR", "1"),
            ("COLORTERM", "truecolor"),
            ("TERM", "xterm-256color"),
        ];
        assert_eq!(color_profile(true, &env), Profile::Ascii);
    }

    #[test]
    fn no_color_keeps_non_color_in_writer_path() {
        let env = [("NO_COLOR", "1"), ("TERM", "xterm-256color")];
        assert_eq!(detect(true, &env), Profile::Ascii);
    }

    #[test]
    fn dumb_term_is_no_tty() {
        let env = [("TERM", "dumb")];
        assert_eq!(color_profile(true, &env), Profile::NoTty);
    }

    #[test]
    fn wt_session_is_truecolor() {
        let env = [("WT_SESSION", "abc"), ("TERM", "xterm")];
        assert_eq!(env_color_profile(&env), Profile::TrueColor);
    }

    #[test]
    fn kitty_term_is_truecolor() {
        let env = [("TERM", "xterm-kitty")];
        assert_eq!(env_color_profile(&env), Profile::TrueColor);
    }

    #[test]
    fn screen_term_is_256() {
        let env = [("TERM", "screen-256color")];
        assert_eq!(env_color_profile(&env), Profile::Ansi256);
    }

    #[test]
    fn tmux_term_no_colorterm_is_256() {
        let env = [("TERM", "tmux-256color")];
        assert_eq!(env_color_profile(&env), Profile::Ansi256);
    }

    #[test]
    fn clicolor_force_upgrades() {
        let env = [("CLICOLOR_FORCE", "1"), ("TERM", "dumb")];
        assert_eq!(color_profile(true, &env), Profile::Ansi);
    }

    #[test]
    fn not_tty_is_no_tty() {
        let env = [("TERM", "xterm-256color")];
        assert_eq!(detect(false, &env), Profile::NoTty);
    }
}
