//! Packed command and parameter types used by the sequence decoder.

/// Bit flag indicating a parameter has following sub-parameters.
pub const HAS_MORE_FLAG: i32 = i32::MIN;

/// Mask stripping the [`HAS_MORE_FLAG`] bit from a parameter.
pub const PARAM_MASK: i32 = !HAS_MORE_FLAG;

/// Sentinel for a missing parameter or command.
pub const MISSING_PARAM: i32 = PARAM_MASK;

/// Alias for [`MISSING_PARAM`].
pub const MISSING_COMMAND: i32 = MISSING_PARAM;

/// Maximum CSI/DCS parameter value.
pub const MAX_PARAM: i32 = u16::MAX as i32;

/// Maximum number of parameters collected per sequence.
pub const MAX_PARAMS_SIZE: usize = 32;

/// Default value substituted for missing parameters.
pub const DEFAULT_PARAM_VALUE: i32 = 0;

/// Shift for the private-mode prefix byte in a packed command.
pub const PREFIX_SHIFT: u32 = 8;

/// Shift for the intermediate byte in a packed command.
pub const INTERMED_SHIFT: u32 = 16;

/// Mask for the final command byte.
pub const FINAL_MASK: i32 = 0xFF;

/// Packed CSI/DCS command with prefix and intermediate bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cmd(pub(crate) i32);

impl Cmd {
    /// Creates a packed command from prefix, intermediate, and final bytes.
    pub const fn pack(prefix: u8, inter: u8, final_byte: u8) -> Self {
        let mut c = final_byte as i32;
        c |= (prefix as i32) << PREFIX_SHIFT;
        c |= (inter as i32) << INTERMED_SHIFT;
        Self(c)
    }

    /// Returns the packed integer value.
    pub const fn raw(self) -> i32 {
        self.0
    }

    /// Returns the private-mode prefix byte (`<`, `=`, `>`, `?`), or 0.
    pub const fn prefix(self) -> u8 {
        ((self.0 >> PREFIX_SHIFT) & FINAL_MASK) as u8
    }

    /// Returns the intermediate byte, or 0.
    pub const fn intermediate(self) -> u8 {
        ((self.0 >> INTERMED_SHIFT) & FINAL_MASK) as u8
    }

    /// Returns the final command byte.
    pub const fn final_byte(self) -> u8 {
        (self.0 & FINAL_MASK) as u8
    }
}

/// A CSI/DCS parameter, optionally marked as having sub-parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Param(pub(crate) i32);

impl Param {
    /// Wraps a raw packed parameter value from the parser buffer.
    pub const fn from_raw(raw: i32) -> Self {
        Self(raw)
    }

    /// Packs a parameter value, optionally marking it as having sub-parameters.
    pub const fn pack(value: i32, has_more: bool) -> Self {
        let mut p = value & PARAM_MASK;
        if has_more {
            p |= HAS_MORE_FLAG;
        }
        Self(p)
    }

    /// Returns the parameter value, or `default` when missing.
    pub const fn value(self, default: i32) -> i32 {
        let p = self.0 & PARAM_MASK;
        if p == MISSING_PARAM { default } else { p }
    }

    /// Returns true when this parameter has following sub-parameters.
    pub const fn has_more(self) -> bool {
        self.0 & HAS_MORE_FLAG != 0
    }

    /// Returns the raw packed value.
    pub const fn raw(self) -> i32 {
        self.0
    }
}

/// Returns parameter `i` from `params`, or -1 when out of bounds / missing.
pub fn param_at(params: &[i32], i: usize) -> i32 {
    if i >= params.len() {
        return -1;
    }
    let p = params[i] & PARAM_MASK;
    if p == MISSING_PARAM { -1 } else { p }
}

/// Returns true when parameter `i` has the [`HAS_MORE_FLAG`] set.
pub fn has_more(params: &[i32], i: usize) -> bool {
    i < params.len() && params[i] & HAS_MORE_FLAG != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmd_roundtrip() {
        let c = Cmd::pack(b'?', b' ', b'h');
        assert_eq!(c.prefix(), b'?');
        assert_eq!(c.intermediate(), b' ');
        assert_eq!(c.final_byte(), b'h');
    }

    #[test]
    fn param_subparams() {
        let p = Param::pack(38, true);
        assert!(p.has_more());
        assert_eq!(p.value(0), 38);
    }
}
