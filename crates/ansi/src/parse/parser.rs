//! Optional parser state collector used by [`super::decode_sequence`] and
//! [`Parser::advance`].

use super::decode::DecodeState;
use super::handler::{Action, Handler};
use super::seq::{Cmd, MAX_PARAMS_SIZE, MISSING_COMMAND, MISSING_PARAM, Param};

/// Collects parameters, data, and the packed command while decoding.
#[derive(Debug, Clone)]
pub struct Parser {
    params: Vec<i32>,
    data: Vec<u8>,
    pub(crate) data_len: i32,
    pub(crate) params_len: usize,
    pub(crate) cmd: i32,
    handler: Handler,
    decode_state: DecodeState,
    last_dispatched_cmd: i32,
    string_kind: u8,
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

impl Parser {
    /// Creates a parser with a 32-parameter buffer and 64 KiB data buffer.
    pub fn new() -> Self {
        Self {
            params: vec![MISSING_PARAM; MAX_PARAMS_SIZE],
            data: vec![0u8; 64 * 1024],
            data_len: 0,
            params_len: 0,
            cmd: 0,
            handler: Handler::default(),
            decode_state: DecodeState::Normal,
            last_dispatched_cmd: 0,
            string_kind: 0,
        }
    }

    /// Installs handler callbacks for [`Self::advance`].
    pub fn set_handler(&mut self, handler: Handler) {
        self.handler = handler;
    }

    /// Returns the current decode state.
    pub fn state(&self) -> DecodeState {
        self.decode_state
    }

    /// Resets parser state to ground.
    pub fn reset(&mut self) {
        self.clear();
        self.decode_state = DecodeState::Normal;
        self.data_len = 0;
    }

    /// Advances the parser by one byte and dispatches completed sequences.
    pub fn advance(&mut self, b: u8) -> Action {
        use super::decode::decode_sequence;

        let prev_state = self.decode_state;
        let byte = [b];
        let d = decode_sequence(&byte, self.decode_state, Some(self));
        self.decode_state = d.state;

        if d.width > 0 {
            if let Some(print) = self.handler.print
                && let Ok(text) = std::str::from_utf8(d.sequence)
            {
                for ch in text.chars() {
                    print(ch);
                }
            }
            return Action::Print;
        }

        if d.width == 0
            && d.consumed == 1
            && (b <= 0x1F || b == 0x7F)
            && self.decode_state == DecodeState::Normal
        {
            if let Some(execute) = self.handler.execute {
                execute(b);
            }
            return Action::Execute;
        }

        if self.decode_state == DecodeState::Normal
            && prev_state != DecodeState::Normal
            && self.cmd != self.last_dispatched_cmd
        {
            return self.dispatch_sequence(prev_state);
        }

        if self.decode_state == DecodeState::Normal
            && prev_state == DecodeState::String
            && self.cmd != self.last_dispatched_cmd
        {
            return self.dispatch_sequence(prev_state);
        }

        Action::Collect
    }

    fn dispatch_sequence(&mut self, prev_state: DecodeState) -> Action {
        self.last_dispatched_cmd = self.cmd;
        let cmd = self.command();
        let params: Vec<Param> = self.params().iter().copied().map(Param::from_raw).collect();
        let data = self.data().to_vec();

        match prev_state {
            DecodeState::Prefix | DecodeState::Params | DecodeState::Intermed => {
                if let Some(handle) = self.handler.handle_csi {
                    handle(cmd, &params);
                }
            }
            DecodeState::Escape => {
                if let Some(handle) = self.handler.handle_esc {
                    handle(cmd);
                }
            }
            DecodeState::String if self.data_len != 0 || !data.is_empty() => {
                match self.string_kind {
                    b'P' => {
                        if let Some(handle) = self.handler.handle_dcs {
                            handle(cmd, &params, &data);
                        }
                    }
                    b']' if self.cmd != MISSING_COMMAND
                        && let Some(handle) = self.handler.handle_osc =>
                    {
                        handle(self.cmd, &data);
                    }
                    b'X' if let Some(handle) = self.handler.handle_sos => handle(&data),
                    b'^' if let Some(handle) = self.handler.handle_pm => handle(&data),
                    b'_' if let Some(handle) = self.handler.handle_apc => handle(&data),
                    _ => {}
                }
            }
            _ => {}
        }

        Action::Dispatch
    }

    /// Resizes the parameter buffer.
    pub fn set_params_size(&mut self, size: usize) {
        self.params = vec![MISSING_PARAM; size.max(1)];
    }

    /// Resizes the data buffer. `size <= 0` means unlimited growth via `Vec`.
    pub fn set_data_size(&mut self, size: i32) {
        if size <= 0 {
            self.data = Vec::new();
            self.data_len = -1;
        } else {
            self.data = vec![0u8; size as usize];
            self.data_len = 0;
        }
    }

    /// Clears collected parameters and command without resetting buffers.
    pub fn clear(&mut self) {
        if !self.params.is_empty() {
            self.params[0] = MISSING_PARAM;
        }
        self.params_len = 0;
        self.cmd = 0;
    }

    /// Returns collected parameters (only the active prefix is meaningful).
    pub fn params(&self) -> &[i32] {
        &self.params[..self.params_len]
    }

    /// Returns parameter `i`, falling back to `default`.
    pub fn param(&self, i: usize, default: i32) -> (i32, bool) {
        if i >= self.params_len {
            return (default, false);
        }
        (Param::from_raw(self.params[i]).value(default), true)
    }

    /// Returns the packed command for the last completed sequence.
    pub fn command(&self) -> Cmd {
        Cmd(self.cmd)
    }

    /// Returns collected string-sequence data.
    pub fn data(&self) -> &[u8] {
        let len = if self.data_len < 0 {
            self.data.len()
        } else {
            self.data_len as usize
        };
        &self.data[..len.min(self.data.len())]
    }

    pub(crate) fn reset_collect(&mut self) {
        if !self.params.is_empty() {
            self.params[0] = MISSING_PARAM;
        }
        self.cmd = 0;
        self.params_len = 0;
        self.data_len = 0;
    }

    pub(crate) fn reset_string(&mut self) {
        self.cmd = MISSING_COMMAND;
        self.data_len = 0;
    }

    pub(crate) fn set_string_kind(&mut self, kind: u8) {
        self.string_kind = kind;
    }

    pub(crate) fn set_prefix(&mut self, b: u8) {
        self.cmd &= !(0xFF << super::seq::PREFIX_SHIFT);
        self.cmd |= (b as i32) << super::seq::PREFIX_SHIFT;
    }

    pub(crate) fn set_intermediate(&mut self, b: u8) {
        self.cmd &= !(0xFF << super::seq::INTERMED_SHIFT);
        self.cmd |= (b as i32) << super::seq::INTERMED_SHIFT;
    }

    pub(crate) fn set_final(&mut self, b: u8) {
        self.cmd &= !super::seq::FINAL_MASK;
        self.cmd |= b as i32;
    }

    pub(crate) fn push_digit(&mut self, b: u8) {
        if self.params_len >= self.params.len() {
            return;
        }
        if self.params[self.params_len] == MISSING_PARAM {
            self.params[self.params_len] = 0;
        }
        self.params[self.params_len] *= 10;
        self.params[self.params_len] += (b - b'0') as i32;
    }

    pub(crate) fn mark_subparam(&mut self) {
        if self.params_len < self.params.len() {
            self.params[self.params_len] |= super::seq::HAS_MORE_FLAG;
        }
    }

    pub(crate) fn advance_param(&mut self) {
        self.params_len += 1;
        if self.params_len < self.params.len() {
            self.params[self.params_len] = MISSING_PARAM;
        }
    }

    pub(crate) fn bump_last_param(&mut self) {
        if self.params_len > 0 && self.params_len < self.params.len() - 1 {
            self.params_len += 1;
        } else if self.params_len == 0 && !self.params.is_empty() && self.params[0] != MISSING_PARAM
        {
            self.params_len = 1;
        }
    }

    pub(crate) fn put_data(&mut self, b: u8) {
        if self.data_len < 0 {
            self.data.push(b);
        } else if (self.data_len as usize) < self.data.len() {
            let i = self.data_len as usize;
            self.data[i] = b;
            self.data_len += 1;
        }
    }

    pub(crate) fn parse_osc_cmd(&mut self) {
        if self.cmd != MISSING_COMMAND {
            return;
        }
        let len = if self.data_len < 0 {
            self.data.len()
        } else {
            self.data_len as usize
        };
        for &d in &self.data[..len] {
            if !d.is_ascii_digit() {
                break;
            }
            if self.cmd == MISSING_COMMAND {
                self.cmd = 0;
            }
            self.cmd *= 10;
            self.cmd += (d - b'0') as i32;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::seq::MISSING_PARAM;
    use super::*;

    #[test]
    fn advance_dispatches_csi_sgr() {
        let mut p = Parser::new();
        for b in b"\x1b[31;1m" {
            let action = p.advance(*b);
            if *b == b'm' {
                assert_eq!(action, Action::Dispatch);
            }
        }
        assert_eq!(p.params(), &[31, 1]);
        assert_eq!(p.command().final_byte(), b'm');
    }

    #[test]
    fn advance_print_and_execute() {
        let mut p = Parser::new();
        assert_eq!(p.advance(b'a'), Action::Print);
        assert_eq!(p.advance(0x07), Action::Execute);
    }

    #[test]
    fn advance_trailing_semicolon_param() {
        let mut p = Parser::new();
        for b in b"\x1b[4;m" {
            p.advance(*b);
        }
        assert_eq!(p.params()[0], 4);
        assert_eq!(p.params()[1], MISSING_PARAM);
    }
}
