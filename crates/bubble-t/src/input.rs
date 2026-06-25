//! Input handling system for the Bubble Tea TUI framework.
//!
//! This module provides the core input processing functionality for `bubble-t`.
//! It is responsible for reading terminal events (keyboard, mouse, resize, focus, paste)
//! and converting them into messages that can be processed by the application's model
//! following the Model-View-Update (MVU) pattern.
//!
//! # Key Components
//!
//! - [`InputHandler`] - The main event processor that runs the input loop
//! - [`InputSource`] - Enum defining different input sources (terminal or custom)
//!
//! # Examples
//!
//! Basic usage with terminal input:
//!
//! ```rust
//! use bubble_t::input::{InputHandler, InputSource};
//! use tokio::sync::mpsc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let (tx, rx) = mpsc::unbounded_channel();
//! let input_handler = InputHandler::new(tx);
//!
//! // Start the input processing loop
//! tokio::spawn(async move {
//!     input_handler.run().await
//! });
//! # Ok(())
//! # }
//! ```
//!
//! Using a custom input source:
//!
//! ```rust
//! use bubble_t::input::{InputHandler, InputSource};
//! use tokio::sync::mpsc;
//! use std::pin::Pin;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let (tx, rx) = mpsc::unbounded_channel();
//! let custom_reader = Box::pin(std::io::Cursor::new("hello\n"));
//! let input_source = InputSource::Custom(custom_reader);
//! let input_handler = InputHandler::with_source(tx, input_source);
//!
//! // Process input from the custom source
//! input_handler.run().await?;
//! # Ok(())
//! # }
//! ```

use crate::event::{
    Mouse, MouseButton, MouseClickMsg, MouseEventMsg, MouseMotionMsg, MouseReleaseMsg,
    MouseWheelMsg,
};
use crate::key::{Key, KeyEventMsg, key_event_from_crossterm};
use crate::query_parser::parse_responses;
use crate::{Error, KeyPressMsg, MouseMsg, WindowSizeMsg};
use crossterm::event::{
    Event, EventStream, KeyCode, KeyEvent, MouseButton as CMouseButton, MouseEvent, MouseEventKind,
};
use futures::StreamExt;
use std::io::Read;
use std::pin::Pin;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};

/// Represents different input sources that the `InputHandler` can read from.
///
/// This enum allows the program to read input from either the standard crossterm
/// event stream (for regular terminal input) or from a custom async reader.
pub enum InputSource {
    /// Standard terminal input using crossterm's event stream.
    /// This is the default and handles keyboard, mouse, and resize events.
    Terminal,

    /// Custom input reader that implements `AsyncRead + Send + Unpin`.
    /// This allows reading input from files, network streams, or other sources.
    /// The custom reader is expected to provide line-based input.
    Custom(Pin<Box<dyn AsyncRead + Send + Unpin>>),
}

/// `InputHandler` is responsible for processing terminal events and sending them
/// as messages to the `Program`'s event loop.
///
/// It continuously reads events from the `crossterm` event stream and converts
/// them into appropriate `Msg` types.
pub struct InputHandler {
    /// The sender half of an MPSC channel used to send messages
    /// to the `Program`'s event loop.
    pub event_tx: crate::event::EventSender,

    /// The input source to read from.
    pub input_source: InputSource,
}

impl InputHandler {
    /// Creates a new `InputHandler` with the given message sender using terminal input.
    ///
    /// This constructor sets up the input handler to read from the standard terminal
    /// using crossterm's event stream. This is the most common usage pattern.
    ///
    /// # Arguments
    ///
    /// * `event_tx` - An `EventSender` to send processed events to the main program loop
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bubble_t::input::InputHandler;
    /// use tokio::sync::mpsc;
    ///
    /// let (tx, rx) = mpsc::unbounded_channel();
    /// let input_handler = InputHandler::new(tx);
    /// ```
    pub fn new<T>(event_tx: T) -> Self
    where
        T: Into<crate::event::EventSender>,
    {
        Self {
            event_tx: event_tx.into(),
            input_source: InputSource::Terminal,
        }
    }

    /// Creates a new `InputHandler` with a custom input source.
    ///
    /// This constructor allows you to specify a custom input source instead of
    /// the default terminal input. This is useful for testing, reading from files,
    /// or processing input from network streams.
    ///
    /// # Arguments
    ///
    /// * `event_tx` - An `EventSender` to send processed events to the main program loop
    /// * `input_source` - The `InputSource` to read from (terminal or custom reader)
    ///
    /// # Examples
    ///
    /// Reading from a file:
    ///
    /// ```rust
    /// use bubble_t::input::{InputHandler, InputSource};
    /// use tokio::sync::mpsc;
    /// use std::pin::Pin;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let (tx, rx) = mpsc::unbounded_channel();
    /// let file_content = std::io::Cursor::new("test input\n");
    /// let custom_source = InputSource::Custom(Box::pin(file_content));
    /// let input_handler = InputHandler::with_source(tx, custom_source);
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_source<T>(event_tx: T, input_source: InputSource) -> Self
    where
        T: Into<crate::event::EventSender>,
    {
        Self {
            event_tx: event_tx.into(),
            input_source,
        }
    }

    /// Runs the input handler loop asynchronously.
    ///
    /// This method continuously reads events from the configured input source
    /// and processes them until the loop terminates. It converts raw terminal
    /// events into typed `Msg` objects and sends them through the event channel
    /// to the main program loop.
    ///
    /// The loop terminates when:
    /// - The event sender channel is closed (receiver dropped)
    /// - An I/O error occurs while reading input
    /// - EOF is reached for custom input sources
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on normal termination, or an `Error` if an I/O error occurs.
    ///
    /// # Errors
    ///
    /// This function returns an error if:
    /// - There's an I/O error reading from the input source
    /// - The underlying crossterm event stream encounters an error
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bubble_t::input::InputHandler;
    /// use tokio::sync::mpsc;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let (tx, mut rx) = mpsc::unbounded_channel();
    /// let input_handler = InputHandler::new(tx);
    ///
    /// // Run the input handler in a separate task
    /// let input_task = tokio::spawn(async move {
    ///     input_handler.run().await
    /// });
    ///
    /// // Process incoming messages
    /// while let Some(msg) = rx.recv().await {
    ///     // Handle the message...
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn run(self) -> Result<(), Error> {
        let event_tx = self.event_tx;
        match self.input_source {
            InputSource::Terminal => Self::run_terminal_input(event_tx).await,
            InputSource::Custom(reader) => Self::run_custom_input(event_tx, reader).await,
        }
    }

    /// Runs the terminal input handler using crossterm's event stream.
    ///
    /// This method processes standard terminal events including:
    /// - Keyboard input (keys and modifiers)
    /// - Mouse events (clicks, movements, scrolling)
    /// - Terminal resize events
    /// - Focus gained/lost events
    /// - Paste events (when bracketed paste is enabled)
    ///
    /// # Arguments
    ///
    /// * `event_tx` - Channel sender for dispatching processed events
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` when the event stream ends normally, or an `Error`
    /// if there's an I/O error reading from the terminal.
    ///
    /// # Errors
    ///
    /// Returns an error if crossterm's event stream encounters an I/O error.
    async fn run_terminal_input(event_tx: crate::event::EventSender) -> Result<(), Error> {
        let mut event_stream = EventStream::new();
        let mut response_buf = Vec::new();

        loop {
            drain_terminal_responses(&event_tx, &mut response_buf)?;

            if !crossterm::event::poll(std::time::Duration::from_millis(16))? {
                continue;
            }

            let event = event_stream.next().await;
            let Some(event) = event else {
                break;
            };

            match event {
                Ok(Event::Key(key_event)) => {
                    if !send_key_event(&event_tx, key_event) {
                        break;
                    }
                }
                Ok(Event::Mouse(mouse_event)) => {
                    if !send_mouse_event(&event_tx, mouse_event) {
                        break;
                    }
                }
                Ok(Event::Resize(width, height)) => {
                    let msg = WindowSizeMsg { width, height };
                    if event_tx.send(Box::new(msg)).is_err() {
                        break;
                    }
                }
                Ok(Event::FocusGained) => {
                    let msg = crate::FocusMsg;
                    if event_tx.send(Box::new(msg)).is_err() {
                        break;
                    }
                }
                Ok(Event::FocusLost) => {
                    let msg = crate::BlurMsg;
                    if event_tx.send(Box::new(msg)).is_err() {
                        break;
                    }
                }
                Ok(Event::Paste(pasted_text)) => {
                    if event_tx
                        .send(Box::new(crate::event::PasteStartMsg))
                        .is_err()
                    {
                        break;
                    }
                    let msg = crate::event::PasteMsg(pasted_text);
                    if event_tx.send(Box::new(msg)).is_err() {
                        break;
                    }
                    if event_tx.send(Box::new(crate::event::PasteEndMsg)).is_err() {
                        break;
                    }
                }
                Err(e) => {
                    return Err(Error::Io(e));
                }
            }

            drain_terminal_responses(&event_tx, &mut response_buf)?;
        }

        Ok(())
    }

    /// Feeds raw bytes through the query parser (used by custom input sources).
    pub fn feed_query_bytes(event_tx: &crate::event::EventSender, data: &[u8]) -> bool {
        let (messages, _) = parse_responses(data);
        for msg in messages {
            if event_tx.send(msg).is_err() {
                return false;
            }
        }
        true
    }

    async fn run_custom_input(
        event_tx: crate::event::EventSender,
        reader: Pin<Box<dyn AsyncRead + Send + Unpin>>,
    ) -> Result<(), Error> {
        let mut buf_reader = BufReader::new(reader);
        let mut line = String::new();
        let mut response_buf = Vec::new();

        loop {
            line.clear();
            match buf_reader.read_line(&mut line).await {
                Ok(0) => break,
                Ok(_) => {
                    response_buf.extend_from_slice(line.as_bytes());
                    let (messages, consumed) = parse_responses(&response_buf);
                    if consumed > 0 {
                        response_buf.drain(..consumed);
                    }
                    for msg in messages {
                        if event_tx.send(msg).is_err() {
                            return Ok(());
                        }
                    }

                    for ch in line.trim().chars() {
                        let press = KeyPressMsg(Key {
                            text: ch.to_string(),
                            r#mod: crate::key::KeyMod::default(),
                            code: KeyCode::Char(ch),
                            shifted_code: None,
                            base_code: None,
                            is_repeat: false,
                        });
                        if event_tx.send(Box::new(press)).is_err() {
                            return Ok(());
                        }
                    }

                    if line.ends_with('\n') {
                        let press = KeyPressMsg(Key {
                            text: String::new(),
                            r#mod: crate::key::KeyMod::default(),
                            code: KeyCode::Enter,
                            shifted_code: None,
                            base_code: None,
                            is_repeat: false,
                        });
                        if event_tx.send(Box::new(press)).is_err() {
                            return Ok(());
                        }
                    }
                }
                Err(e) => return Err(Error::Io(e)),
            }
        }

        Ok(())
    }
}

fn drain_terminal_responses(
    event_tx: &crate::event::EventSender,
    buffer: &mut Vec<u8>,
) -> Result<(), Error> {
    let Ok(mut tty) = term::open_tty_input() else {
        return Ok(());
    };

    let mut chunk = [0u8; 256];
    loop {
        match tty.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => buffer.extend_from_slice(&chunk[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
        // Only drain bytes already waiting; avoid blocking the crossterm loop.
        break;
    }

    if buffer.is_empty() {
        return Ok(());
    }

    let (messages, consumed) = parse_responses(buffer);
    if consumed > 0 {
        buffer.drain(..consumed);
    }
    for msg in messages {
        if event_tx.send(msg).is_err() {
            break;
        }
    }

    if let Some((key_msg, consumed)) = ansi::kitty::try_parse_key(buffer) {
        buffer.drain(..consumed);
        let parsed = crate::key::key_event_from_kitty(&key_msg);
        let boxed: crate::Msg = match parsed {
            KeyEventMsg::Press(msg) => Box::new(msg),
            KeyEventMsg::Release(msg) => Box::new(msg),
        };
        let _ = event_tx.send(boxed);
    }

    if let Some((mouse_ev, consumed)) = ansi::mouse::try_parse_mouse(buffer) {
        buffer.drain(..consumed);
        let (v2, legacy) = mouse_event_from_ansi(&mouse_ev);
        let v2_boxed: crate::Msg = match v2 {
            MouseEventMsg::Click(msg) => Box::new(msg),
            MouseEventMsg::Release(msg) => Box::new(msg),
            MouseEventMsg::Wheel(msg) => Box::new(msg),
            MouseEventMsg::Motion(msg) => Box::new(msg),
        };
        let _ = event_tx.send(v2_boxed);
        let _ = event_tx.send(Box::new(legacy));
    }
    Ok(())
}

fn mouse_event_from_ansi(ev: &ansi::mouse::MouseEvent) -> (MouseEventMsg, MouseMsg) {
    use crate::key::KeyMod;
    use ansi::mouse::MouseButton as ABtn;

    let mut mods = KeyMod::default();
    if ev.shift {
        mods = mods.union(KeyMod::SHIFT);
    }
    if ev.alt {
        mods = mods.union(KeyMod::ALT);
    }
    if ev.ctrl {
        mods = mods.union(KeyMod::CTRL);
    }

    let button = match ev.button {
        ABtn::Left => MouseButton::Left,
        ABtn::Middle => MouseButton::Middle,
        ABtn::Right => MouseButton::Right,
        ABtn::WheelUp => MouseButton::WheelUp,
        ABtn::WheelDown => MouseButton::WheelDown,
        ABtn::WheelLeft => MouseButton::WheelLeft,
        ABtn::WheelRight => MouseButton::WheelRight,
        ABtn::Backward => MouseButton::Backward,
        ABtn::Forward => MouseButton::Forward,
        ABtn::None => MouseButton::None,
        ABtn::Button10 | ABtn::Button11 => MouseButton::None,
    };

    let base = Mouse {
        x: ev.x.max(0) as u16,
        y: ev.y.max(0) as u16,
        button,
        r#mod: mods,
    };

    let legacy_kind = match (ev.wheel, ev.motion, ev.release, button) {
        (_, _, true, _) => crossterm::event::MouseEventKind::Up(crossterm_button_legacy(button)),
        (true, _, _, MouseButton::WheelUp) => crossterm::event::MouseEventKind::ScrollUp,
        (true, _, _, MouseButton::WheelDown) => crossterm::event::MouseEventKind::ScrollDown,
        (true, _, _, MouseButton::WheelLeft) => crossterm::event::MouseEventKind::ScrollLeft,
        (true, _, _, MouseButton::WheelRight) => crossterm::event::MouseEventKind::ScrollRight,
        (_, true, _, btn) if btn != MouseButton::None => {
            crossterm::event::MouseEventKind::Drag(crossterm_button_legacy(btn))
        }
        (_, true, _, _) => crossterm::event::MouseEventKind::Moved,
        (_, _, _, btn) if btn != MouseButton::None => {
            crossterm::event::MouseEventKind::Down(crossterm_button_legacy(btn))
        }
        _ => crossterm::event::MouseEventKind::Moved,
    };

    let legacy = MouseMsg {
        x: base.x,
        y: base.y,
        button: legacy_kind,
        modifiers: mods.to_crossterm(),
    };

    let v2 = if ev.wheel {
        MouseEventMsg::Wheel(MouseWheelMsg(base))
    } else if ev.release {
        MouseEventMsg::Release(MouseReleaseMsg(base))
    } else if ev.motion || button == MouseButton::None {
        MouseEventMsg::Motion(MouseMotionMsg(base))
    } else {
        MouseEventMsg::Click(MouseClickMsg(base))
    };

    (v2, legacy)
}

fn crossterm_button_legacy(btn: MouseButton) -> CMouseButton {
    match btn {
        MouseButton::Left => CMouseButton::Left,
        MouseButton::Right => CMouseButton::Right,
        MouseButton::Middle => CMouseButton::Middle,
        _ => CMouseButton::Left,
    }
}

fn send_key_event(event_tx: &crate::event::EventSender, key_event: KeyEvent) -> bool {
    #[cfg(target_os = "windows")]
    {
        if !key_event.is_press() {
            return true;
        }
    }

    let Some(parsed) = key_event_from_crossterm(key_event) else {
        return true;
    };

    let boxed: crate::Msg = match parsed {
        KeyEventMsg::Press(msg) => Box::new(msg),
        KeyEventMsg::Release(msg) => Box::new(msg),
    };
    event_tx.send(boxed).is_ok()
}

fn send_mouse_event(event_tx: &crate::event::EventSender, mouse_event: MouseEvent) -> bool {
    let (v2, legacy) = mouse_event_from_crossterm(&mouse_event);
    let v2_boxed: crate::Msg = match v2 {
        MouseEventMsg::Click(msg) => Box::new(msg),
        MouseEventMsg::Release(msg) => Box::new(msg),
        MouseEventMsg::Wheel(msg) => Box::new(msg),
        MouseEventMsg::Motion(msg) => Box::new(msg),
    };
    event_tx.send(v2_boxed).is_ok() && event_tx.send(Box::new(legacy)).is_ok()
}

fn mouse_event_from_crossterm(event: &MouseEvent) -> (MouseEventMsg, MouseMsg) {
    use crate::key::KeyMod;

    let base = Mouse {
        x: event.column,
        y: event.row,
        button: MouseButton::None,
        r#mod: KeyMod::from_crossterm(event.modifiers),
    };

    let legacy = MouseMsg {
        x: event.column,
        y: event.row,
        button: event.kind,
        modifiers: event.modifiers,
    };

    let v2 = match event.kind {
        MouseEventKind::Down(btn) => MouseEventMsg::Click(MouseClickMsg(Mouse {
            button: crossterm_button(btn),
            ..base
        })),
        MouseEventKind::Up(btn) => MouseEventMsg::Release(MouseReleaseMsg(Mouse {
            button: crossterm_button(btn),
            ..base
        })),
        MouseEventKind::ScrollUp => MouseEventMsg::Wheel(MouseWheelMsg(Mouse {
            button: MouseButton::WheelUp,
            ..base
        })),
        MouseEventKind::ScrollDown => MouseEventMsg::Wheel(MouseWheelMsg(Mouse {
            button: MouseButton::WheelDown,
            ..base
        })),
        MouseEventKind::ScrollLeft => MouseEventMsg::Wheel(MouseWheelMsg(Mouse {
            button: MouseButton::WheelLeft,
            ..base
        })),
        MouseEventKind::ScrollRight => MouseEventMsg::Wheel(MouseWheelMsg(Mouse {
            button: MouseButton::WheelRight,
            ..base
        })),
        MouseEventKind::Moved => MouseEventMsg::Motion(MouseMotionMsg(base)),
        MouseEventKind::Drag(btn) => MouseEventMsg::Motion(MouseMotionMsg(Mouse {
            button: crossterm_button(btn),
            ..base
        })),
    };

    (v2, legacy)
}

fn crossterm_button(btn: CMouseButton) -> MouseButton {
    match btn {
        CMouseButton::Left => MouseButton::Left,
        CMouseButton::Right => MouseButton::Right,
        CMouseButton::Middle => MouseButton::Middle,
    }
}
