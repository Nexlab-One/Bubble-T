//! Focus-in/focus-out report sequences.

/// Focus-in report (used with focus-event mode).
pub const FOCUS: &str = "\x1b[I";
/// Focus-out report (used with focus-event mode).
pub const BLUR: &str = "\x1b[O";
