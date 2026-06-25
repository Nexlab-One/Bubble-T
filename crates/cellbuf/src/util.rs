//! Small helpers shared by cellbuf internals.

/// Returns the number of lines in `s` (newline count + 1).
pub fn height(s: &str) -> usize {
    s.bytes().filter(|&b| b == b'\n').count() + 1
}

/// Clamps `v` to `[low, high]`.
pub fn clamp(v: i32, low: i32, high: i32) -> i32 {
    let (low, high) = if high < low { (high, low) } else { (low, high) };
    v.max(low).min(high)
}

/// Absolute value.
pub fn abs(v: i32) -> i32 {
    v.abs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn height_counts_newlines() {
        assert_eq!(height("a\nb\nc"), 3);
        assert_eq!(height(""), 1);
    }

    #[test]
    fn clamp_orders_bounds() {
        assert_eq!(clamp(5, 10, 0), 5);
    }
}
