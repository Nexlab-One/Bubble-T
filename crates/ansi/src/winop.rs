//! XTWINOPS window manipulation sequences.
//!
//! Port of [`charmbracelet/x/ansi`] window operation builders.
//!
//! [`charmbracelet/x/ansi`]: https://github.com/charmbracelet/x/tree/main/ansi

/// Resize the terminal window (deprecated constant kept for parity).
pub const RESIZE_WINDOW_WIN_OP: i32 = 4;
/// Request window size in pixels (`CSI 4 ; height ; width t`).
pub const REQUEST_WINDOW_SIZE_WIN_OP: i32 = 14;
/// Request cell size in pixels (`CSI 6 ; height ; width t`).
pub const REQUEST_CELL_SIZE_WIN_OP: i32 = 16;

/// Builds an XTWINOPS sequence (`CSI Ps ; ... t`).
pub fn window_op(p: i32, ps: &[i32]) -> String {
    if p <= 0 {
        return String::new();
    }
    if ps.is_empty() {
        return format!("\x1b[{p}t");
    }
    let mut params = vec![p.to_string()];
    for &v in ps {
        if v >= 0 {
            params.push(v.to_string());
        }
    }
    format!("\x1b[{}t", params.join(";"))
}

/// Alias for [`window_op`].
pub fn xtwinops(p: i32, ps: &[i32]) -> String {
    window_op(p, ps)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_param() {
        assert_eq!(window_op(14, &[]), "\x1b[14t");
    }

    #[test]
    fn multiple_params() {
        assert_eq!(window_op(8, &[0, 0, 100, 200]), "\x1b[8;0;0;100;200t");
    }
}
