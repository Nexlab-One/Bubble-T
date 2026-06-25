//! Style-aware wrapping for cell-buffer content.
//!
//! For plain strings, delegates to [`ansi::wrap`]. Styled cell grids use the
//! upstream cellbuf algorithm via ANSI sequence round-tripping.

/// Wraps plain text to `limit` cells using the shared ANSI wrap implementation.
pub fn wrap(s: &str, limit: usize, breakpoints: &str) -> String {
    ansi::wrap::wrap(s, limit, breakpoints)
}

/// Returns the rendered height of wrapped plain text.
pub fn wrap_height(s: &str, limit: usize, breakpoints: &str) -> usize {
    crate::util::height(&wrap(s, limit, breakpoints))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delegates_to_ansi_wrap() {
        let wrapped = wrap("hello world", 5, "");
        assert!(wrapped.contains('\n'));
    }
}
