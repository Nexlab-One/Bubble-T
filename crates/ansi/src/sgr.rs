//! SGR attribute constants and sequence builders.

use crate::style::Style;

/// SGR (Select Graphic Rendition) attribute code.
pub type Attr = i32;

/// Resets all attributes.
pub const ATTR_RESET: Attr = 0;
/// Bold / increased intensity.
pub const ATTR_BOLD: Attr = 1;
/// Faint / decreased intensity.
pub const ATTR_FAINT: Attr = 2;
/// Italic.
pub const ATTR_ITALIC: Attr = 3;
/// Underline.
pub const ATTR_UNDERLINE: Attr = 4;
/// Slow blink.
pub const ATTR_BLINK: Attr = 5;
/// Rapid blink.
pub const ATTR_RAPID_BLINK: Attr = 6;
/// Reverse video.
pub const ATTR_REVERSE: Attr = 7;
/// Conceal / hidden.
pub const ATTR_CONCEAL: Attr = 8;
/// Strikethrough.
pub const ATTR_STRIKETHROUGH: Attr = 9;
/// Normal intensity (resets bold/faint).
pub const ATTR_NORMAL_INTENSITY: Attr = 22;
/// No italic.
pub const ATTR_NO_ITALIC: Attr = 23;
/// No underline.
pub const ATTR_NO_UNDERLINE: Attr = 24;
/// No blink.
pub const ATTR_NO_BLINK: Attr = 25;
/// No reverse.
pub const ATTR_NO_REVERSE: Attr = 27;
/// No conceal.
pub const ATTR_NO_CONCEAL: Attr = 28;
/// No strikethrough.
pub const ATTR_NO_STRIKETHROUGH: Attr = 29;

/// Black foreground.
pub const ATTR_BLACK_FOREGROUND: Attr = 30;
/// Red foreground.
pub const ATTR_RED_FOREGROUND: Attr = 31;
/// Green foreground.
pub const ATTR_GREEN_FOREGROUND: Attr = 32;
/// Yellow foreground.
pub const ATTR_YELLOW_FOREGROUND: Attr = 33;
/// Blue foreground.
pub const ATTR_BLUE_FOREGROUND: Attr = 34;
/// Magenta foreground.
pub const ATTR_MAGENTA_FOREGROUND: Attr = 35;
/// Cyan foreground.
pub const ATTR_CYAN_FOREGROUND: Attr = 36;
/// White foreground.
pub const ATTR_WHITE_FOREGROUND: Attr = 37;
/// Extended foreground color introducer (38).
pub const ATTR_EXTENDED_FOREGROUND: Attr = 38;
/// Default foreground.
pub const ATTR_DEFAULT_FOREGROUND: Attr = 39;

/// Black background.
pub const ATTR_BLACK_BACKGROUND: Attr = 40;
/// Red background.
pub const ATTR_RED_BACKGROUND: Attr = 41;
/// Green background.
pub const ATTR_GREEN_BACKGROUND: Attr = 42;
/// Yellow background.
pub const ATTR_YELLOW_BACKGROUND: Attr = 43;
/// Blue background.
pub const ATTR_BLUE_BACKGROUND: Attr = 44;
/// Magenta background.
pub const ATTR_MAGENTA_BACKGROUND: Attr = 45;
/// Cyan background.
pub const ATTR_CYAN_BACKGROUND: Attr = 46;
/// White background.
pub const ATTR_WHITE_BACKGROUND: Attr = 47;
/// Extended background color introducer (48).
pub const ATTR_EXTENDED_BACKGROUND: Attr = 48;
/// Default background.
pub const ATTR_DEFAULT_BACKGROUND: Attr = 49;

/// Extended underline color introducer (58).
pub const ATTR_EXTENDED_UNDERLINE: Attr = 58;
/// Default underline color.
pub const ATTR_DEFAULT_UNDERLINE: Attr = 59;

/// Bright black foreground.
pub const ATTR_BRIGHT_BLACK_FOREGROUND: Attr = 90;
/// Bright red foreground.
pub const ATTR_BRIGHT_RED_FOREGROUND: Attr = 91;
/// Bright green foreground.
pub const ATTR_BRIGHT_GREEN_FOREGROUND: Attr = 92;
/// Bright yellow foreground.
pub const ATTR_BRIGHT_YELLOW_FOREGROUND: Attr = 93;
/// Bright blue foreground.
pub const ATTR_BRIGHT_BLUE_FOREGROUND: Attr = 94;
/// Bright magenta foreground.
pub const ATTR_BRIGHT_MAGENTA_FOREGROUND: Attr = 95;
/// Bright cyan foreground.
pub const ATTR_BRIGHT_CYAN_FOREGROUND: Attr = 96;
/// Bright white foreground.
pub const ATTR_BRIGHT_WHITE_FOREGROUND: Attr = 97;

/// Bright black background.
pub const ATTR_BRIGHT_BLACK_BACKGROUND: Attr = 100;
/// Bright red background.
pub const ATTR_BRIGHT_RED_BACKGROUND: Attr = 101;
/// Bright green background.
pub const ATTR_BRIGHT_GREEN_BACKGROUND: Attr = 102;
/// Bright yellow background.
pub const ATTR_BRIGHT_YELLOW_BACKGROUND: Attr = 103;
/// Bright blue background.
pub const ATTR_BRIGHT_BLUE_BACKGROUND: Attr = 104;
/// Bright magenta background.
pub const ATTR_BRIGHT_MAGENTA_BACKGROUND: Attr = 105;
/// Bright cyan background.
pub const ATTR_BRIGHT_CYAN_BACKGROUND: Attr = 106;
/// Bright white background.
pub const ATTR_BRIGHT_WHITE_BACKGROUND: Attr = 107;

/// RGB color introducer sub-parameter (2).
pub const ATTR_RGB_INTRODUCER: Attr = 2;
/// Indexed color introducer sub-parameter (5).
pub const ATTR_INDEXED_INTRODUCER: Attr = 5;

/// SGR sequence that resets all attributes.
pub const RESET_STYLE: &str = "\x1b[m";

fn attr_string(a: Attr) -> String {
    match a {
        ATTR_RESET => "0".into(),
        ATTR_BOLD => "1".into(),
        ATTR_FAINT => "2".into(),
        ATTR_ITALIC => "3".into(),
        ATTR_UNDERLINE => "4".into(),
        ATTR_BLINK => "5".into(),
        ATTR_RAPID_BLINK => "6".into(),
        ATTR_REVERSE => "7".into(),
        ATTR_CONCEAL => "8".into(),
        ATTR_STRIKETHROUGH => "9".into(),
        ATTR_NORMAL_INTENSITY => "22".into(),
        ATTR_NO_ITALIC => "23".into(),
        ATTR_NO_UNDERLINE => "24".into(),
        ATTR_NO_BLINK => "25".into(),
        ATTR_NO_REVERSE => "27".into(),
        ATTR_NO_CONCEAL => "28".into(),
        ATTR_NO_STRIKETHROUGH => "29".into(),
        ATTR_BLACK_FOREGROUND => "30".into(),
        ATTR_RED_FOREGROUND => "31".into(),
        ATTR_GREEN_FOREGROUND => "32".into(),
        ATTR_YELLOW_FOREGROUND => "33".into(),
        ATTR_BLUE_FOREGROUND => "34".into(),
        ATTR_MAGENTA_FOREGROUND => "35".into(),
        ATTR_CYAN_FOREGROUND => "36".into(),
        ATTR_WHITE_FOREGROUND => "37".into(),
        ATTR_EXTENDED_FOREGROUND => "38".into(),
        ATTR_DEFAULT_FOREGROUND => "39".into(),
        ATTR_BLACK_BACKGROUND => "40".into(),
        ATTR_RED_BACKGROUND => "41".into(),
        ATTR_GREEN_BACKGROUND => "42".into(),
        ATTR_YELLOW_BACKGROUND => "43".into(),
        ATTR_BLUE_BACKGROUND => "44".into(),
        ATTR_MAGENTA_BACKGROUND => "45".into(),
        ATTR_CYAN_BACKGROUND => "46".into(),
        ATTR_WHITE_BACKGROUND => "47".into(),
        ATTR_EXTENDED_BACKGROUND => "48".into(),
        ATTR_DEFAULT_BACKGROUND => "49".into(),
        ATTR_EXTENDED_UNDERLINE => "58".into(),
        ATTR_DEFAULT_UNDERLINE => "59".into(),
        ATTR_BRIGHT_BLACK_FOREGROUND => "90".into(),
        ATTR_BRIGHT_RED_FOREGROUND => "91".into(),
        ATTR_BRIGHT_GREEN_FOREGROUND => "92".into(),
        ATTR_BRIGHT_YELLOW_FOREGROUND => "93".into(),
        ATTR_BRIGHT_BLUE_FOREGROUND => "94".into(),
        ATTR_BRIGHT_MAGENTA_FOREGROUND => "95".into(),
        ATTR_BRIGHT_CYAN_FOREGROUND => "96".into(),
        ATTR_BRIGHT_WHITE_FOREGROUND => "97".into(),
        ATTR_BRIGHT_BLACK_BACKGROUND => "100".into(),
        ATTR_BRIGHT_RED_BACKGROUND => "101".into(),
        ATTR_BRIGHT_GREEN_BACKGROUND => "102".into(),
        ATTR_BRIGHT_YELLOW_BACKGROUND => "103".into(),
        ATTR_BRIGHT_BLUE_BACKGROUND => "104".into(),
        ATTR_BRIGHT_MAGENTA_BACKGROUND => "105".into(),
        ATTR_BRIGHT_CYAN_BACKGROUND => "106".into(),
        ATTR_BRIGHT_WHITE_BACKGROUND => "107".into(),
        other => {
            let v = if other < 0 { 0 } else { other };
            v.to_string()
        }
    }
}

/// Builds an SGR (`CSI … m`) sequence for the given attributes.
pub fn select_graphic_rendition(attrs: &[Attr]) -> String {
    if attrs.is_empty() {
        return RESET_STYLE.to_string();
    }
    Style::from_attrs(attrs).to_string()
}

/// Alias for [`select_graphic_rendition`].
pub fn sgr(attrs: &[Attr]) -> String {
    select_graphic_rendition(attrs)
}

/// Converts a single attribute to its SGR parameter string.
pub(crate) fn attr_to_param(a: Attr) -> String {
    attr_string(a)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_sgr_resets() {
        assert_eq!(select_graphic_rendition(&[]), "\x1b[m");
    }

    #[test]
    fn bold_and_colors() {
        assert_eq!(
            select_graphic_rendition(&[ATTR_RED_FOREGROUND, ATTR_BOLD]),
            "\x1b[31;1m"
        );
    }
}
