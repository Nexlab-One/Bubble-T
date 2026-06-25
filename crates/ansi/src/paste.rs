//! Bracketed paste marker sequences.

/// Opening marker emitted by the terminal when bracketed paste mode is active.
pub const BRACKETED_PASTE_START: &str = "\x1b[200~";
/// Closing marker emitted by the terminal when bracketed paste mode is active.
pub const BRACKETED_PASTE_END: &str = "\x1b[201~";
