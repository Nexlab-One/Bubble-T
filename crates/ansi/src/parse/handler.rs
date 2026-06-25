//! Handler callbacks invoked by [`super::parser::Parser::advance`].

use super::seq::{Cmd, Param};

type DcsHandler = fn(Cmd, &[Param], &[u8]);

/// Actions performed while advancing the parser one byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Byte ignored.
    Ignore,
    /// Parser state cleared.
    Clear,
    /// Printable character.
    Print,
    /// C0/C1 control executed.
    Execute,
    /// Collecting parameters or data.
    Collect,
    /// Sequence dispatched to a handler.
    Dispatch,
}

/// Callbacks for ANSI sequences and printable text.
///
/// All handlers are optional; unset callbacks are skipped.
#[derive(Default, Clone, Debug)]
pub struct Handler {
    /// Called for printable runes.
    pub print: Option<fn(char)>,
    /// Called for C0/C1 control bytes.
    pub execute: Option<fn(u8)>,
    /// Called when a CSI sequence completes.
    pub handle_csi: Option<fn(Cmd, &[Param])>,
    /// Called when an ESC sequence completes.
    pub handle_esc: Option<fn(Cmd)>,
    /// Called when a DCS sequence completes.
    pub handle_dcs: Option<DcsHandler>,
    /// Called when an OSC sequence completes.
    pub handle_osc: Option<fn(i32, &[u8])>,
    /// Called when a SOS (ESC X) string completes.
    pub handle_sos: Option<fn(&[u8])>,
    /// Called when a PM (ESC ^) string completes.
    pub handle_pm: Option<fn(&[u8])>,
    /// Called when an APC (ESC _) string completes.
    pub handle_apc: Option<fn(&[u8])>,
}

impl Handler {
    /// Creates an empty handler.
    pub fn new() -> Self {
        Self::default()
    }
}
