//! Terminfo database probing for true-color capability.

use crate::Profile;

/// Returns the color profile inferred from the terminfo database for `term`.
///
/// When `term` is empty or `"dumb"`, returns [`Profile::NoTty`]. When terminfo
/// cannot be loaded, defaults to [`Profile::Ansi`]. Terminals reporting the
/// `Tc` or `RGB` extended boolean capabilities are classified as
/// [`Profile::TrueColor`].
pub fn terminfo_profile(term: &str) -> Profile {
    if term.is_empty() || term == "dumb" {
        return Profile::NoTty;
    }

    let Ok(db) = terminfo::Database::from_name(term) else {
        return Profile::Ansi;
    };

    if db.raw("Tc").is_some() || db.raw("RGB").is_some() {
        Profile::TrueColor
    } else {
        Profile::Ansi
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dumb_term_is_no_tty() {
        assert_eq!(terminfo_profile("dumb"), Profile::NoTty);
    }

    #[test]
    fn empty_term_is_no_tty() {
        assert_eq!(terminfo_profile(""), Profile::NoTty);
    }

    #[test]
    fn unknown_term_defaults_ansi() {
        assert_eq!(
            terminfo_profile("this-terminal-definitely-does-not-exist-xyz"),
            Profile::Ansi
        );
    }
}
