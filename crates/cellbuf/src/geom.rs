//! Geometry types for buffer regions.

/// An `(x, y)` position on the grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct Position {
    /// Column index (0-based).
    pub x: i32,
    /// Row index (0-based).
    pub y: i32,
}

impl Position {
    /// Creates a position at `(x, y)`.
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Returns true when `self` lies inside `rect`.
    pub fn in_rect(self, rect: Rectangle) -> bool {
        self.x >= rect.min.x && self.y >= rect.min.y && self.x < rect.max.x && self.y < rect.max.y
    }
}

/// An axis-aligned rectangle on the grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct Rectangle {
    /// Inclusive minimum corner.
    pub min: Position,
    /// Exclusive maximum corner.
    pub max: Position,
}

impl Rectangle {
    /// Creates a rectangle from origin `(x, y)` with size `(width, height)`.
    pub const fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            min: Position::new(x, y),
            max: Position::new(x + width, y + height),
        }
    }

    /// Returns the width in columns.
    pub const fn width(self) -> i32 {
        self.max.x - self.min.x
    }

    /// Returns the height in rows.
    pub const fn height(self) -> i32 {
        self.max.y - self.min.y
    }
}

/// Shorthand for [`Rectangle::new`].
pub const fn rect(x: i32, y: i32, width: i32, height: i32) -> Rectangle {
    Rectangle::new(x, y, width, height)
}

/// Shorthand for [`Position::new`].
pub const fn pos(x: i32, y: i32) -> Position {
    Position::new(x, y)
}
