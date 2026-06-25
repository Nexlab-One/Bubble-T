//! Escape-sequence parsing: incremental decoder and parameter types.

mod decode;
mod handler;
mod parser;
mod seq;

pub use decode::{
    DecodeState, DecodedSequence, WidthMethod, command, decode_sequence, decode_sequence_wc,
    has_csi_prefix, has_dcs_prefix, has_esc_prefix, has_osc_prefix, has_st_prefix,
};
pub use handler::{Action, Handler};
pub use parser::Parser;
pub use seq::{
    Cmd, DEFAULT_PARAM_VALUE, HAS_MORE_FLAG, MAX_PARAM, MAX_PARAMS_SIZE, MISSING_COMMAND,
    MISSING_PARAM, PARAM_MASK, Param, has_more, param_at,
};
