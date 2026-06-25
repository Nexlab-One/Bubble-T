//! Sixel and Kitty in-band graphics sequence builders.
//!
//! Port of [`charmbracelet/x/ansi`] graphics helpers for terminals that support
//! inline image protocols.
//!
//! [`charmbracelet/x/ansi`]: https://github.com/charmbracelet/x/tree/main/ansi

pub mod kitty;

/// Builds a DCS sixel graphics sequence.
///
/// `DCS p1 ; p2 ; p3 ; q [payload] ST`
pub fn sixel_graphics(p1: i32, p2: i32, p3: i32, payload: &[u8]) -> String {
    let mut out = String::from("\x1bP");
    if p1 >= 0 {
        out.push_str(&p1.to_string());
    }
    out.push(';');
    if p2 >= 0 {
        out.push_str(&p2.to_string());
    }
    if p3 > 0 {
        out.push(';');
        out.push_str(&p3.to_string());
    }
    out.push('q');
    out.push_str(&String::from_utf8_lossy(payload));
    out.push_str("\x1b\\");
    out
}

/// Builds a Kitty APC graphics sequence.
///
/// `APC _G [options] ; [payload] ST`
pub fn kitty_graphics(payload: &[u8], opts: &[&str]) -> String {
    let mut out = String::from("\x1b_G");
    if !opts.is_empty() {
        out.push_str(&opts.join(","));
    }
    if !payload.is_empty() {
        out.push(';');
        out.push_str(&String::from_utf8_lossy(payload));
    }
    out.push_str("\x1b\\");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sixel_prefix() {
        let seq = sixel_graphics(0, 1, 0, b"#1;2;3");
        assert!(seq.starts_with("\x1bP0;1q"));
        assert!(seq.ends_with("\x1b\\"));
    }

    #[test]
    fn kitty_graphics_options() {
        let seq = kitty_graphics(b"data", &["a=1", "f=24"]);
        assert!(seq.starts_with("\x1b_Ga=1,f=24;data"));
    }

    #[test]
    fn kitty_graphics_cases_match_upstream() {
        assert_eq!(kitty_graphics(&[], &[]), "\x1b_G\x1b\\");
        assert_eq!(kitty_graphics(b"test", &[]), "\x1b_G;test\x1b\\");
        assert_eq!(
            kitty_graphics(b"test", &["a=t", "f=100"]),
            "\x1b_Ga=t,f=100;test\x1b\\"
        );
        assert_eq!(
            kitty_graphics(&[], &["q=2", "C=1", "f=24"]),
            "\x1b_Gq=2,C=1,f=24\x1b\\"
        );
        assert_eq!(
            kitty_graphics(b"\x1b_G", &["a=t"]),
            "\x1b_Ga=t;\x1b_G\x1b\\"
        );
    }
}
