//! XTerm-specific control sequences (modifyOtherKeys, XTMODKEYS).

/// Enables modifyOtherKeys mode 1.
pub const SET_MODIFY_OTHER_KEYS1: &str = "\x1b[>4;1m";
/// Enables modifyOtherKeys mode 2.
pub const SET_MODIFY_OTHER_KEYS2: &str = "\x1b[>4;2m";
/// Resets modifyOtherKeys mode.
pub const RESET_MODIFY_OTHER_KEYS: &str = "\x1b[>4m";
/// Queries modifyOtherKeys mode.
pub const QUERY_MODIFY_OTHER_KEYS: &str = "\x1b[?4m";

/// Sets XTerm modifyOtherKeys mode (`0` disable, `1` or `2` enable).
pub fn modify_other_keys(mode: i32) -> String {
    format!("\x1b[>4;{mode}m")
}

/// Sets xterm key modifier options (`CSI > Pp ; Pv m`).
pub fn key_modifier_options(p: i32, v: Option<i32>) -> String {
    match v {
        Some(v) => format!("\x1b[>{p};{v}m"),
        None => format!("\x1b[>{p}m"),
    }
}

/// Queries xterm key modifier options (`CSI ? Pp m`).
pub fn query_key_modifier_options(p: i32) -> String {
    if p > 0 {
        format!("\x1b[?{p}m")
    } else {
        "\x1b[?m".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modify_other_keys_modes() {
        assert_eq!(modify_other_keys(2), SET_MODIFY_OTHER_KEYS2);
        assert_eq!(modify_other_keys(0), "\x1b[>4;0m");
    }
}
