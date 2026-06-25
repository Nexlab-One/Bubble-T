//! # bubble-t
//!
//! Rust port of Bubble Tea v2 — declarative [`View`], cell-based rendering, and
//! the full v2 message family (key press/release, mouse split, clipboard, terminal queries).

#![warn(missing_docs)]

pub mod command;
pub mod error;
pub mod event;
pub mod gradient;
pub mod input;
pub mod key;
pub mod logging;
pub mod memory;
pub mod model;
pub mod program;
pub mod query_parser;
pub mod renderer;
pub mod signals;
pub mod terminal;
pub mod view;
pub mod view_runtime;

pub use command::{
    Batch, Cmd, batch, cancel_all_timers, cancel_timer, clear_screen, every, every_with_id,
    exec_process, interrupt, printf, println, quit, raw, read_clipboard, read_primary_clipboard,
    request_background_color, request_capability, request_cursor_color, request_cursor_position,
    request_foreground_color, request_terminal_version, sequence, set_clipboard,
    set_primary_clipboard, suspend, tick, window_size,
};
pub use error::Error;
pub use event::{
    BackgroundColorMsg, BatchCmdMsg, BatchMsgInternal, BlurMsg, CancelAllTimersMsg, CancelTimerMsg,
    CapabilityMsg, ClearScreenMsg, ClipboardMsg, ColorProfileMsg, CursorColorMsg,
    CursorPositionMsg, EnvMsg, EventReceiver, EventSender, FocusMsg, ForegroundColorMsg,
    InterruptMsg, KeyboardEnhancementsMsg, KillMsg, LayerMouseMsg, LightDarkMsg, ModeReportMsg,
    Mouse, MouseButton, MouseClickMsg, MouseMotionMsg, MouseMsg, MouseReleaseMsg, MouseWheelMsg,
    Msg, PasteEndMsg, PasteMsg, PasteStartMsg, PrintMsg, PrintfMsg, QuitMsg, RawCmdMsg,
    ReadClipboardCmdMsg, ReadPrimaryClipboardCmdMsg, RequestBackgroundColorCmdMsg,
    RequestCapabilityCmdMsg, RequestCursorColorCmdMsg, RequestCursorPositionCmdMsg,
    RequestForegroundColorCmdMsg, RequestTerminalVersionCmdMsg, RequestWindowSizeMsg, ResumeMsg,
    SetClipboardCmdMsg, SetPrimaryClipboardCmdMsg, SuspendMsg, TerminalVersionMsg, WindowSizeMsg,
};
pub use gradient::{
    charm_default_gradient, gradient_filled_segment, gradient_filled_segment_with_buffer, lerp_rgb,
};
pub use input::{InputHandler, InputSource};
pub use key::{Key, KeyEventMsg, KeyMod, KeyMsg, KeyPressMsg, KeyReleaseMsg, legacy_key_msg};
pub use memory::{MemoryHealth, MemoryMonitor, MemorySnapshot};
pub use model::Model;
pub use program::{Program, ProgramBuilder, ProgramConfig};
pub use terminal::{DummyTerminal, Terminal, TerminalInterface};
pub use view::{
    Cursor, CursorShape, KeyboardEnhancements, MouseMode, OnMouseFn, Position, ProgressBar,
    ProgressBarState, View,
};

#[cfg(feature = "logging")]
pub use logging::log_to_file;

pub mod prelude {
    //! Common re-exports for application code.

    pub use crate::{Cmd, Error, KeyPressMsg, Model, MouseClickMsg, Msg, Program, QuitMsg, View};

    #[cfg(feature = "logging")]
    pub use crate::log_to_file;
}
