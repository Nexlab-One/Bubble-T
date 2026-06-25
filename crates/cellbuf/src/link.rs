//! Hyperlink metadata carried per cell.

/// OSC 8 hyperlink attached to one or more cells.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct Link {
    /// Hyperlink URL.
    pub url: String,
    /// Optional OSC 8 parameters (the middle field).
    pub params: String,
}

impl Link {
    /// Clears the hyperlink.
    pub fn reset(&mut self) {
        self.url.clear();
        self.params.clear();
    }

    /// Returns true when no URL or params are set.
    pub fn is_empty(&self) -> bool {
        self.url.is_empty() && self.params.is_empty()
    }
}
