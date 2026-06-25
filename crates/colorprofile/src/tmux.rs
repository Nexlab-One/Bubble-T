//! Tmux color capability detection.

use std::process::Command;

use crate::Profile;

/// Returns the color profile from tmux when running inside a tmux session.
///
/// Outside tmux this returns [`Profile::NoTty`]. Inside tmux the default is
/// [`Profile::Ansi256`], upgraded to [`Profile::TrueColor`] when `tmux info`
/// reports `Tc` or `RGB` capabilities.
pub fn tmux_profile(environ: &[(&str, &str)]) -> Profile {
    tmux_from_env(environ)
}

fn tmux_from_env(environ: &[(&str, &str)]) -> Profile {
    let tmux = lookup(environ, "TMUX");
    if tmux.is_none_or(str::is_empty) {
        return Profile::NoTty;
    }

    let profile = Profile::Ansi256;
    let Ok(output) = Command::new("tmux").arg("info").output() else {
        return profile;
    };

    for line in output.stdout.split(|&b| b == b'\n') {
        let line = String::from_utf8_lossy(line);
        if (line.contains("Tc") || line.contains("RGB")) && line.contains("true") {
            return Profile::TrueColor;
        }
    }

    profile
}

fn lookup<'a>(environ: &'a [(&str, &str)], key: &str) -> Option<&'a str> {
    environ.iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outside_tmux_is_no_tty() {
        assert_eq!(tmux_profile(&[]), Profile::NoTty);
    }

    #[test]
    fn empty_tmux_var_is_no_tty() {
        assert_eq!(tmux_profile(&[("TMUX", "")]), Profile::NoTty);
    }
}
