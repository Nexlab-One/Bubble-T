//! Tab stop tracking for width-aware screens.

/// Default terminal tab interval in columns.
pub const DEFAULT_TAB_WIDTH: u8 = 8;

/// Alias matching upstream naming.
pub const DEFAULT_TAB_INTERVAL: u8 = DEFAULT_TAB_WIDTH;

/// Tab stops for a screen row, stored as a compact bitmask per interval block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabStops {
    stops: Vec<u8>,
    interval: u8,
    width: usize,
}

impl TabStops {
    /// Creates tab stops for `width` columns at the given interval.
    pub fn new(width: usize, interval: u8) -> Self {
        let interval = interval.max(1);
        let size = width.div_ceil(usize::from(interval));
        let mut ts = Self {
            stops: vec![0; size.max(1)],
            interval,
            width: width.max(1),
        };
        ts.init(0, ts.width);
        ts
    }

    /// Creates tab stops with the default 8-column interval.
    pub fn default_stops(cols: usize) -> Self {
        Self::new(cols, DEFAULT_TAB_INTERVAL)
    }

    /// Returns whether `col` is a tab stop.
    pub fn is_stop(&self, col: i32) -> bool {
        let col = col as usize;
        let mask = self.mask(col);
        let i = col >> 3;
        if i >= self.stops.len() {
            return false;
        }
        self.stops[i] & mask != 0
    }

    /// Returns the next tab stop at or after `col`.
    pub fn next(&self, col: i32) -> i32 {
        self.find(col, 1)
    }

    /// Returns the previous tab stop before `col`.
    pub fn prev(&self, col: i32) -> i32 {
        self.find(col, -1)
    }

    /// Finds the next (`delta > 0`) or previous (`delta < 0`) tab stop.
    pub fn find(&self, col: i32, delta: i32) -> i32 {
        if delta == 0 {
            return col;
        }

        let prev = delta < 0;
        let mut count = delta.abs();
        let mut col = col;

        while count > 0 {
            if !prev {
                if col >= self.width as i32 - 1 {
                    return col;
                }
                col += 1;
            } else {
                if col < 1 {
                    return col;
                }
                col -= 1;
            }

            if self.is_stop(col) {
                count -= 1;
            }
        }

        col
    }

    /// Adds a tab stop at `col`.
    pub fn set(&mut self, col: i32) {
        let col = col as usize;
        let mask = self.mask(col);
        let i = col >> 3;
        if i < self.stops.len() {
            self.stops[i] |= mask;
        }
    }

    /// Removes the tab stop at `col`.
    pub fn reset(&mut self, col: i32) {
        let col = col as usize;
        let mask = self.mask(col);
        let i = col >> 3;
        if i < self.stops.len() {
            self.stops[i] &= !mask;
        }
    }

    /// Clears all tab stops.
    pub fn clear(&mut self) {
        self.stops.fill(0);
    }

    /// Resizes tab stops to `width` columns.
    pub fn resize(&mut self, width: usize) {
        if width == self.width {
            return;
        }

        if width < self.width {
            let size = width.div_ceil(usize::from(self.interval));
            self.stops.truncate(size.max(1));
        } else {
            let extra = (width - self.width).div_ceil(usize::from(self.interval));
            self.stops.resize(self.stops.len() + extra, 0);
        }

        self.init(self.width, width);
        self.width = width.max(1);
    }

    fn mask(&self, col: usize) -> u8 {
        1 << (col & (usize::from(self.interval) - 1))
    }

    fn init(&mut self, from: usize, to: usize) {
        for x in from..to {
            if x % usize::from(self.interval) == 0 {
                self.set(x as i32);
            } else {
                self.reset(x as i32);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_interval_stops() {
        let tabs = TabStops::new(24, DEFAULT_TAB_INTERVAL);
        assert!(tabs.is_stop(0));
        assert!(!tabs.is_stop(7));
        assert!(tabs.is_stop(8));
        assert!(!tabs.is_stop(15));
        assert!(tabs.is_stop(16));
    }

    #[test]
    fn custom_interval() {
        let tabs = TabStops::new(16, 4);
        assert!(tabs.is_stop(0));
        assert!(!tabs.is_stop(3));
        assert!(tabs.is_stop(4));
        assert!(tabs.is_stop(12));
    }

    #[test]
    fn set_and_reset() {
        let mut tabs = TabStops::new(16, 8);
        let custom = 9;
        tabs.set(custom);
        assert!(tabs.is_stop(custom));
        tabs.reset(8);
        assert!(!tabs.is_stop(8));
    }

    #[test]
    fn next_and_prev() {
        let tabs = TabStops::new(16, 8);
        assert_eq!(tabs.next(0), 8);
        assert_eq!(tabs.next(1), 8);
        assert_eq!(tabs.prev(9), 8);
    }

    #[test]
    fn resize_preserves_interval() {
        let mut tabs = TabStops::new(8, 4);
        tabs.resize(12);
        assert!(tabs.is_stop(8));
    }
}
