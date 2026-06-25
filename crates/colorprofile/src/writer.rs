//! Downsampling writer that rewrites ANSI color sequences.

use std::io::{self, IsTerminal, Write};

use ansi::color::{Color, read_style_color};
use ansi::parse::{DecodeState, Parser, decode_sequence, has_csi_prefix, param_at};
use ansi::style::Style as AnsiStyle;
use ansi::width::Method;

use crate::Profile;
use crate::env::{detect, is_tty_forced};

/// Writer that downgrades ANSI color sequences to match a [`Profile`].
pub struct Writer<W: Write> {
    forward: W,
    profile: Profile,
}

impl<W: Write> Writer<W> {
    /// Creates a writer that detects the profile from `environ` and wraps `forward`.
    ///
    /// TTY detection uses [`std::io::stdout`]; pass [`Writer::with_profile`] when
    /// writing to a different destination.
    pub fn new(forward: W, environ: &[(&str, &str)]) -> Self {
        let is_tty = is_tty_forced(environ) || std::io::stdout().is_terminal();
        Self {
            profile: detect(is_tty, environ),
            forward,
        }
    }

    /// Creates a writer with an explicit profile.
    pub fn with_profile(forward: W, profile: Profile) -> Self {
        Self { forward, profile }
    }

    /// Returns the active color profile.
    pub fn profile(&self) -> Profile {
        self.profile
    }
}

impl<W: Write> Write for Writer<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self.profile {
            Profile::TrueColor => self.forward.write(buf),
            Profile::NoTty => {
                let plain = Method::GraphemeWidth.strip(std::str::from_utf8(buf).unwrap_or(""));
                self.forward.write_all(plain.as_bytes())?;
                Ok(buf.len())
            }
            Profile::Ascii | Profile::Ansi | Profile::Ansi256 => {
                self.downsample(buf)?;
                Ok(buf.len())
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        self.forward.flush()
    }
}

impl<W: Write> Writer<W> {
    fn downsample(&mut self, input: &[u8]) -> io::Result<()> {
        let mut parser = Parser::new();
        let mut state = DecodeState::Normal;
        let mut rest = input;

        while !rest.is_empty() {
            let d = decode_sequence(rest, state, Some(&mut parser));
            if has_csi_prefix(d.sequence) && parser.command().final_byte() == b'm' {
                let seq = downsample_sgr(&self.profile, parser.params());
                self.forward.write_all(seq.as_bytes())?;
            } else {
                self.forward.write_all(d.sequence)?;
            }
            state = d.state;
            rest = &rest[d.consumed..];
        }

        Ok(())
    }
}

fn downsample_sgr(profile: &Profile, params: &[i32]) -> String {
    let mut style = AnsiStyle::new();
    let mut i = 0usize;
    while i < params.len() {
        let param = param_at(params, i);
        match param {
            0 => style = AnsiStyle::new().reset(),
            1 => style = style.bold(),
            2 => style = style.faint(),
            3 => style = style.italic(true),
            4 => style = style.underline(true),
            5 | 6 => style = style.blink(true),
            7 => style = style.reverse(true),
            8 => style = style.reverse(false),
            9 => style = style.strikethrough(true),
            22 => style = style.normal(),
            23 => style = style.italic(false),
            24 => style = style.underline(false),
            25 => style = style.blink(false),
            27 => style = style.reverse(false),
            29 => style = style.strikethrough(false),
            30..=37 | 90..=97 => {
                if *profile >= Profile::Ansi {
                    let basic = basic_from_code(param);
                    style =
                        style.foreground_color(Some(profile.convert_color(Color::Basic(basic))));
                }
            }
            38 => {
                let (color, n) = read_style_color(&params[i..]);
                if *profile >= Profile::Ansi
                    && let Some(c) = color
                {
                    style = style.foreground_color(Some(profile.convert_color(c)));
                }
                if n > 0 {
                    i += n - 1;
                }
            }
            39 => {
                if *profile >= Profile::Ansi {
                    style = style.foreground_color(None);
                }
            }
            40..=47 | 100..=107 => {
                if *profile >= Profile::Ansi {
                    let basic = basic_from_code(param - 10);
                    style =
                        style.background_color(Some(profile.convert_color(Color::Basic(basic))));
                }
            }
            48 => {
                let (color, n) = read_style_color(&params[i..]);
                if *profile >= Profile::Ansi
                    && let Some(c) = color
                {
                    style = style.background_color(Some(profile.convert_color(c)));
                }
                if n > 0 {
                    i += n - 1;
                }
            }
            49 => {
                if *profile >= Profile::Ansi {
                    style = style.background_color(None);
                }
            }
            58 => {
                let (color, n) = read_style_color(&params[i..]);
                if *profile >= Profile::Ansi
                    && let Some(c) = color
                {
                    style = style.underline_color(Some(profile.convert_color(c)));
                }
                if n > 0 {
                    i += n - 1;
                }
            }
            59 if *profile >= Profile::Ansi => {
                style = style.underline_color(None);
            }
            _ => {}
        }
        i += 1;
    }
    style.to_string()
}

fn basic_from_code(param: i32) -> ansi::color::BasicColor {
    use ansi::color::BasicColor::*;
    match param {
        30 | 90 => {
            if param == 90 {
                BrightBlack
            } else {
                Black
            }
        }
        31 | 91 => {
            if param == 91 {
                BrightRed
            } else {
                Red
            }
        }
        32 | 92 => {
            if param == 92 {
                BrightGreen
            } else {
                Green
            }
        }
        33 | 93 => {
            if param == 93 {
                BrightYellow
            } else {
                Yellow
            }
        }
        34 | 94 => {
            if param == 94 {
                BrightBlue
            } else {
                Blue
            }
        }
        35 | 95 => {
            if param == 95 {
                BrightMagenta
            } else {
                Magenta
            }
        }
        36 | 96 => {
            if param == 96 {
                BrightCyan
            } else {
                Cyan
            }
        }
        37 | 97 => {
            if param == 97 {
                BrightWhite
            } else {
                White
            }
        }
        p if (40..=47).contains(&p) => basic_from_code(p - 10),
        p if (100..=107).contains(&p) => basic_from_code(p - 10),
        _ => White,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn downsample_truecolor_to_ansi256() {
        let mut buf = Cursor::new(Vec::new());
        let mut w = Writer::with_profile(&mut buf, Profile::Ansi256);
        write!(w, "\x1b[38;2;255;0;0mX\x1b[0m").unwrap();
        let out = String::from_utf8(buf.into_inner()).unwrap();
        assert!(out.contains("38;5;"));
    }

    #[test]
    fn no_color_profile_keeps_bold_strips_fg() {
        let mut buf = Cursor::new(Vec::new());
        let mut w = Writer::with_profile(&mut buf, Profile::Ascii);
        write!(w, "\x1b[1;31mBold Red\x1b[0m").unwrap();
        let out = String::from_utf8(buf.into_inner()).unwrap();
        assert!(out.contains("\x1b[1m") || out.contains("\x1b[1;"));
        assert!(!out.contains("31m") || out.contains("\x1b[0m"));
    }

    #[test]
    fn ascii_strips_rgb_keeps_italic() {
        let mut buf = Cursor::new(Vec::new());
        let mut w = Writer::with_profile(&mut buf, Profile::Ascii);
        write!(w, "\x1b[3;38;2;255;0;0mHi\x1b[0m").unwrap();
        let out = String::from_utf8(buf.into_inner()).unwrap();
        assert!(out.contains("\x1b[3"));
        assert!(!out.contains("38;2"));
    }
}
