//! Error types for cell buffer operations.

use std::fmt;

/// Errors returned by cell-buffer operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CellbufError {
    /// Position is outside buffer bounds.
    OutOfBounds {
        /// Column index.
        x: usize,
        /// Row index.
        y: usize,
        /// Buffer width.
        width: usize,
        /// Buffer height.
        height: usize,
    },
    /// Invalid scroll region dimensions.
    InvalidScrollRegion,
    /// UTF-8 content could not be decoded.
    InvalidUtf8,
}

impl fmt::Display for CellbufError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfBounds {
                x,
                y,
                width,
                height,
            } => write!(f, "cell ({x},{y}) outside buffer bounds ({width}x{height})"),
            Self::InvalidScrollRegion => write!(f, "invalid scroll region"),
            Self::InvalidUtf8 => write!(f, "invalid UTF-8 in cell content"),
        }
    }
}

impl std::error::Error for CellbufError {}
