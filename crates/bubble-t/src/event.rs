//! This module defines the various message types used in `bubble-t`.
//! Messages are events that trigger updates in your application's model.
//! They are typically sent by commands or the input handler.

use std::any::Any;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

/// A message represents any event that can trigger a model update.
///
/// `Msg` is a type alias for `Box<dyn Any + Send>`, allowing it to hold
/// any type that implements `Any` and `Send`. This provides flexibility
/// in defining custom message types for your application.
pub type Msg = Box<dyn Any + Send>;

/// Event sender abstraction that can be either bounded or unbounded.
///
/// `EventSender` provides a unified interface for sending messages through
/// either bounded or unbounded channels. This abstraction allows the framework
/// to switch between different channel types without changing the API.
///
/// # Examples
///
/// ```
/// use bubble_t::event::{EventSender, Msg};
/// use tokio::sync::mpsc;
///
/// // Create from unbounded channel
/// let (tx, _rx) = mpsc::unbounded_channel::<Msg>();
/// let sender = EventSender::from_unbounded(tx);
///
/// // Send a message
/// let msg: Msg = Box::new("Hello");
/// sender.send(msg).unwrap();
/// ```
#[derive(Clone)]
pub enum EventSender {
    /// Unbounded channel sender used for unlimited-capacity message delivery.
    Unbounded(mpsc::UnboundedSender<Msg>),
    /// Bounded channel sender that applies backpressure when full.
    Bounded(mpsc::Sender<Msg>),
}

impl EventSender {
    /// Send a message through the channel.
    ///
    /// Attempts to send a message through the underlying channel. For unbounded
    /// channels, this will only fail if the receiver has been dropped. For bounded
    /// channels, this may also fail due to backpressure (channel full).
    ///
    /// # Arguments
    ///
    /// * `msg` - The message to send
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the message was sent successfully, or an error if:
    /// - The channel is closed (`Error::ChannelClosed`)
    /// - The channel is full (`Error::ChannelFull`) for bounded channels
    ///
    /// # Examples
    ///
    /// ```
    /// use bubble_t::event::{EventSender, Msg};
    /// use tokio::sync::mpsc;
    ///
    /// let (tx, _rx) = mpsc::unbounded_channel::<Msg>();
    /// let sender = EventSender::from_unbounded(tx);
    ///
    /// let msg: Msg = Box::new(42);
    /// match sender.send(msg) {
    ///     Ok(()) => println!("Message sent!"),
    ///     Err(e) => eprintln!("Failed to send: {}", e),
    /// }
    /// ```
    pub fn send(&self, msg: Msg) -> Result<(), crate::Error> {
        match self {
            // Unbounded send fails only when the receiver is closed.
            EventSender::Unbounded(tx) => tx.send(msg).map_err(|_| crate::Error::ChannelClosed),
            // Bounded send can fail due to Full (backpressure) or Closed.
            EventSender::Bounded(tx) => tx.try_send(msg).map_err(Into::into),
        }
    }

    /// Check if the sender is closed.
    ///
    /// Returns `true` if the receiver side of the channel has been dropped,
    /// meaning that any future send operations will fail.
    ///
    /// # Returns
    ///
    /// `true` if the channel is closed, `false` otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use bubble_t::event::{EventSender, Msg};
    /// use tokio::sync::mpsc;
    ///
    /// let (tx, rx) = mpsc::unbounded_channel::<Msg>();
    /// let sender = EventSender::from_unbounded(tx);
    ///
    /// assert!(!sender.is_closed());
    /// drop(rx); // Drop the receiver
    /// assert!(sender.is_closed());
    /// ```
    pub fn is_closed(&self) -> bool {
        match self {
            EventSender::Unbounded(tx) => tx.is_closed(),
            EventSender::Bounded(tx) => tx.is_closed(),
        }
    }

    /// Create an EventSender from an UnboundedSender (for backward compatibility).
    ///
    /// This method creates an `EventSender` wrapping an unbounded channel sender.
    /// Unbounded channels have unlimited capacity and never apply backpressure.
    ///
    /// # Arguments
    ///
    /// * `tx` - The unbounded sender to wrap
    ///
    /// # Returns
    ///
    /// An `EventSender` that uses the provided unbounded channel
    ///
    /// # Examples
    ///
    /// ```
    /// use bubble_t::event::{EventSender, Msg};
    /// use tokio::sync::mpsc;
    ///
    /// let (tx, _rx) = mpsc::unbounded_channel::<Msg>();
    /// let sender = EventSender::from_unbounded(tx);
    /// ```
    pub fn from_unbounded(tx: mpsc::UnboundedSender<Msg>) -> Self {
        EventSender::Unbounded(tx)
    }

    /// Create an EventSender from a bounded Sender (for testing).
    ///
    /// This method creates an `EventSender` wrapping a bounded channel sender.
    /// Bounded channels have limited capacity and will apply backpressure when full.
    /// This is primarily used in testing scenarios to verify behavior under
    /// backpressure conditions.
    ///
    /// # Arguments
    ///
    /// * `tx` - The bounded sender to wrap
    ///
    /// # Returns
    ///
    /// An `EventSender` that uses the provided bounded channel
    ///
    /// # Examples
    ///
    /// ```
    /// use bubble_t::event::{EventSender, Msg};
    /// use tokio::sync::mpsc;
    ///
    /// let (tx, _rx) = mpsc::channel::<Msg>(10); // Capacity of 10
    /// let sender = EventSender::from_bounded(tx);
    /// ```
    pub fn from_bounded(tx: mpsc::Sender<Msg>) -> Self {
        EventSender::Bounded(tx)
    }
}

impl From<mpsc::UnboundedSender<Msg>> for EventSender {
    fn from(tx: mpsc::UnboundedSender<Msg>) -> Self {
        EventSender::Unbounded(tx)
    }
}

impl From<mpsc::Sender<Msg>> for EventSender {
    fn from(tx: mpsc::Sender<Msg>) -> Self {
        EventSender::Bounded(tx)
    }
}

/// Event receiver abstraction that can be either bounded or unbounded.
///
/// `EventReceiver` provides a unified interface for receiving messages from
/// either bounded or unbounded channels. This abstraction allows the framework
/// to switch between different channel types without changing the API.
///
/// # Examples
///
/// ```no_run
/// use bubble_t::event::{EventReceiver, EventSender, Msg};
/// use tokio::sync::mpsc;
///
/// async fn example() {
///     let (tx, rx) = mpsc::unbounded_channel::<Msg>();
/// let mut receiver = EventReceiver::Unbounded(rx);
/// let sender = EventSender::from_unbounded(tx);
///
/// // Send and receive a message
/// sender.send(Box::new(42)).unwrap();
///     if let Some(msg) = receiver.recv().await {
///         // Process the message
///     }
/// }
/// ```
pub enum EventReceiver {
    /// Unbounded channel receiver counterpart for unlimited-capacity channels.
    Unbounded(mpsc::UnboundedReceiver<Msg>),
    /// Bounded channel receiver that may yield `None` when closed and drained.
    Bounded(mpsc::Receiver<Msg>),
}

impl EventReceiver {
    /// Receive the next message from the channel.
    ///
    /// Asynchronously waits for the next message from the channel. Returns `None`
    /// when the sender side has been dropped and all messages have been received.
    ///
    /// # Returns
    ///
    /// - `Some(Msg)` if a message was received
    /// - `None` if the channel is closed and empty
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use bubble_t::event::{EventReceiver, Msg};
    /// use tokio::sync::mpsc;
    ///
    /// async fn example() {
    ///     let (tx, rx) = mpsc::unbounded_channel::<Msg>();
    /// let mut receiver = EventReceiver::Unbounded(rx);
    ///
    /// // Send a message
    /// tx.send(Box::new("Hello")).unwrap();
    ///
    /// // Receive the message
    /// match receiver.recv().await {
    ///     Some(msg) => {
    ///         if let Some(text) = msg.downcast_ref::<&str>() {
    ///             println!("Received: {}", text);
    ///         }
    ///     }
    ///     None => println!("Channel closed"),
    /// }
    /// }
    /// ```
    pub async fn recv(&mut self) -> Option<Msg> {
        match self {
            EventReceiver::Unbounded(rx) => rx.recv().await,
            EventReceiver::Bounded(rx) => rx.recv().await,
        }
    }
}

/// Global event sender set by Program on startup so commands can emit messages
/// back into the event loop from background tasks.
///
/// This global static is initialized once when a `Program` starts running and
/// provides a way for background tasks and commands to send messages back to
/// the main event loop. It uses `OnceLock` to ensure thread-safe one-time
/// initialization.
///
/// # Examples
///
/// ```
/// use bubble_t::event::{EVENT_SENDER, Msg};
///
/// // In a background task or command
/// if let Some(sender) = EVENT_SENDER.get() {
///     let msg: Msg = Box::new("Task completed");
///     sender.send(msg).unwrap();
/// }
/// ```
///
/// # Note
///
/// This is automatically initialized by the framework. User code should only
/// read from it, never write to it.
pub static EVENT_SENDER: OnceLock<EventSender> = OnceLock::new();

/// Global timer ID generator for unique timer identification.
///
/// This atomic counter ensures that each timer created in the application
/// receives a unique identifier. The counter starts at 1 and increments
/// atomically to avoid race conditions in multi-threaded environments.
static TIMER_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Generates a unique timer ID.
///
/// This function atomically increments and returns a unique identifier
/// for timers. Each call is guaranteed to return a different value,
/// even when called from multiple threads simultaneously.
///
/// # Returns
///
/// A unique `u64` identifier for a timer
///
/// # Examples
///
/// ```
/// use bubble_t::event::next_timer_id;
///
/// let id1 = next_timer_id();
/// let id2 = next_timer_id();
/// assert_ne!(id1, id2);
/// ```
pub fn next_timer_id() -> u64 {
    TIMER_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// Mouse event routed through a compositor layer hit test.
#[derive(Debug, Clone)]
pub struct LayerMouseMsg {
    /// Id of the hit layer (from [`lipgloss::Layer::id`]).
    pub layer_id: String,
    /// Layer bounds `(x, y, width, height)`.
    pub bounds: (i32, i32, i32, i32),
    /// Underlying mouse event.
    pub mouse: MouseMsg,
}

/// A message indicating a mouse input event (legacy v1 shape).
#[derive(Debug, Clone)]
pub struct MouseMsg {
    /// The column coordinate of the mouse event.
    pub x: u16,
    /// The row coordinate of the mouse event.
    pub y: u16,
    /// The `crossterm::event::MouseEventKind` representing the type of mouse event.
    pub button: crossterm::event::MouseEventKind,
    /// The `crossterm::event::KeyModifiers` active during the mouse event.
    pub modifiers: crossterm::event::KeyModifiers,
}

/// Mouse button identifiers (v2, X11-style).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MouseButton {
    /// No button.
    #[default]
    None,
    /// Left button.
    Left,
    /// Middle button (scroll wheel click).
    Middle,
    /// Right button.
    Right,
    /// Scroll wheel up.
    WheelUp,
    /// Scroll wheel down.
    WheelDown,
    /// Scroll wheel left.
    WheelLeft,
    /// Scroll wheel right.
    WheelRight,
    /// Browser back button.
    Backward,
    /// Browser forward button.
    Forward,
}

/// Mouse event payload shared by v2 mouse message types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mouse {
    /// Zero-based column.
    pub x: u16,
    /// Zero-based row.
    pub y: u16,
    /// Button involved in the event.
    pub button: MouseButton,
    /// Modifier keys held during the event.
    pub r#mod: crate::key::KeyMod,
}

impl Mouse {
    /// Converts to the legacy [`MouseMsg`] shape.
    pub fn to_legacy(&self) -> MouseMsg {
        use crossterm::event::{MouseButton as CMouseButton, MouseEventKind};
        let kind = match self.button {
            MouseButton::Left => MouseEventKind::Down(CMouseButton::Left),
            MouseButton::Middle => MouseEventKind::Down(CMouseButton::Middle),
            MouseButton::Right => MouseEventKind::Down(CMouseButton::Right),
            MouseButton::WheelUp => MouseEventKind::ScrollUp,
            MouseButton::WheelDown => MouseEventKind::ScrollDown,
            MouseButton::WheelLeft => MouseEventKind::ScrollLeft,
            MouseButton::WheelRight => MouseEventKind::ScrollRight,
            MouseButton::Backward | MouseButton::Forward => MouseEventKind::Moved,
            MouseButton::None => MouseEventKind::Moved,
        };
        MouseMsg {
            x: self.x,
            y: self.y,
            button: kind,
            modifiers: self.r#mod.to_crossterm(),
        }
    }
}

/// v2 mouse click message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MouseClickMsg(pub Mouse);

impl MouseClickMsg {
    /// Returns the underlying mouse event.
    pub fn mouse(&self) -> &Mouse {
        &self.0
    }
}

/// v2 mouse release message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MouseReleaseMsg(pub Mouse);

impl MouseReleaseMsg {
    /// Returns the underlying mouse event.
    pub fn mouse(&self) -> &Mouse {
        &self.0
    }
}

/// v2 mouse wheel message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MouseWheelMsg(pub Mouse);

impl MouseWheelMsg {
    /// Returns the underlying mouse event.
    pub fn mouse(&self) -> &Mouse {
        &self.0
    }
}

/// v2 mouse motion message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MouseMotionMsg(pub Mouse);

impl MouseMotionMsg {
    /// Returns the underlying mouse event.
    pub fn mouse(&self) -> &Mouse {
        &self.0
    }
}

/// Union of v2 mouse message variants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MouseEventMsg {
    /// Button press.
    Click(MouseClickMsg),
    /// Button release.
    Release(MouseReleaseMsg),
    /// Scroll wheel.
    Wheel(MouseWheelMsg),
    /// Pointer motion.
    Motion(MouseMotionMsg),
}

/// Bracketed-paste start marker (v2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PasteStartMsg;

/// Bracketed-paste end marker (v2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PasteEndMsg;

/// A message indicating that text was pasted into the terminal (bracketed paste).
///
/// This message is generated when bracketed paste mode is enabled and the user
/// pastes text into the terminal. The pasted content is captured as a single
/// string, preserving newlines and special characters.
///
/// # Examples
///
/// ```
/// use bubble_t::event::PasteMsg;
///
/// // Handling a paste event in your model's update method
/// let paste_msg = PasteMsg("Hello\nWorld".to_string());
/// // The text contains the exact pasted content
/// assert_eq!(paste_msg.0, "Hello\nWorld");
/// ```
///
/// # Note
///
/// Bracketed paste mode must be enabled with `EnableBracketedPasteMsg` for
/// these messages to be generated.
#[derive(Debug, Clone)]
pub struct PasteMsg(pub String);

/// A message indicating a change in the terminal window size.
#[derive(Debug, Clone)]
pub struct WindowSizeMsg {
    /// The new width of the terminal window.
    pub width: u16,
    /// The new height of the terminal window.
    pub height: u16,
}

/// A message to signal the application to quit.
///
/// Sending this message to the `Program` will initiate a graceful shutdown.
#[derive(Debug, Clone)]
pub struct QuitMsg;

/// A message to forcefully kill the application immediately.
///
/// Sending this message to the `Program` will cause it to terminate as soon as
/// possible. The event loop will stop without invoking the model's `update` and
/// will return an `Error::ProgramKilled`.
#[derive(Debug, Clone)]
pub struct KillMsg;

/// A message to signal an application interruption.
///
/// This is typically sent when an interrupt signal (e.g., Ctrl+C) is received.
#[derive(Debug, Clone)]
pub struct InterruptMsg;

/// A message to signal the application to suspend.
///
/// This can be used to temporarily pause the application, for example, when
/// another process needs control of the terminal.
#[derive(Debug, Clone)]
pub struct SuspendMsg;

/// A message to signal the application to resume after suspension.
#[derive(Debug, Clone)]
pub struct ResumeMsg;

/// A message indicating that the terminal gained focus.
#[derive(Debug, Clone)]
pub struct FocusMsg;

/// A message indicating that the terminal lost focus.
#[derive(Debug, Clone)]
pub struct BlurMsg;

/// An internal message type used to batch multiple messages together.
/// This is not exposed as a public API.
#[derive(Debug)]
pub struct BatchMsgInternal {
    /// Aggregated messages to dispatch as a single batch.
    pub messages: Vec<Msg>,
}

/// A message containing commands to be executed concurrently.
/// This enables non-blocking batch operations that spawn commands immediately.
pub struct BatchCmdMsg(pub Vec<crate::Cmd>);

/// A message to signal the terminal to enter the alternate screen buffer.
#[derive(Debug, Clone)]
pub struct EnterAltScreenMsg;

/// A message to signal the terminal to exit the alternate screen buffer.
#[derive(Debug, Clone)]
pub struct ExitAltScreenMsg;

/// A message to signal the terminal to enable mouse cell motion reporting.
#[derive(Debug, Clone)]
pub struct EnableMouseCellMotionMsg;

/// A message to signal the terminal to enable all mouse motion reporting.
#[derive(Debug, Clone)]
pub struct EnableMouseAllMotionMsg;

/// A message to signal the terminal to disable mouse reporting.
#[derive(Debug, Clone)]
pub struct DisableMouseMsg;

/// A message to signal the terminal to enable bracketed paste mode.
#[derive(Debug, Clone)]
pub struct EnableBracketedPasteMsg;

/// A message to signal the terminal to disable bracketed paste mode.
#[derive(Debug, Clone)]
pub struct DisableBracketedPasteMsg;

/// A message to signal the terminal to enable focus reporting.
#[derive(Debug, Clone)]
pub struct EnableReportFocusMsg;

/// A message to signal the terminal to disable focus reporting.
#[derive(Debug, Clone)]
pub struct DisableReportFocusMsg;

/// A message to signal the terminal to show the cursor.
#[derive(Debug, Clone)]
pub struct ShowCursorMsg;

/// A message to signal the terminal to hide the cursor.
#[derive(Debug, Clone)]
pub struct HideCursorMsg;

/// A message to signal the terminal to clear the screen.
#[derive(Debug, Clone)]
pub struct ClearScreenMsg;

/// A message to signal the terminal to request its current window size.
///
/// The terminal will respond with a `WindowSizeMsg` containing its dimensions.
#[derive(Debug, Clone)]
pub struct RequestWindowSizeMsg;

/// A message to print a line to the terminal.
///
/// This message causes the program to print text to the terminal output.
/// The text will be printed as-is with a newline appended.
///
/// # Examples
///
/// ```
/// use bubble_t::event::PrintMsg;
///
/// // Print a simple message
/// let msg = PrintMsg("Hello, Terminal!".to_string());
/// ```
///
/// # Note
///
/// This bypasses the normal view rendering and directly outputs to the terminal.
/// Use sparingly as it can interfere with the TUI display.
#[derive(Debug, Clone)]
pub struct PrintMsg(pub String);

/// A message to print formatted text to the terminal.
///
/// Similar to `PrintMsg`, but the text is treated as pre-formatted and
/// printed exactly as provided without adding a newline.
///
/// # Examples
///
/// ```
/// use bubble_t::event::PrintfMsg;
///
/// // Print formatted text without automatic newline
/// let msg = PrintfMsg("Progress: 50%\r".to_string());
/// ```
///
/// # Note
///
/// This bypasses the normal view rendering and directly outputs to the terminal.
/// Useful for progress indicators or custom formatting that requires precise
/// control over newlines and carriage returns.
#[derive(Debug, Clone)]
pub struct PrintfMsg(pub String);

/// A message to set the terminal window title.
///
/// This message updates the terminal window's title bar with the provided string.
/// Not all terminals support this feature.
///
/// # Examples
///
/// ```
/// use bubble_t::event::SetWindowTitleMsg;
///
/// // Set a custom window title
/// let msg = SetWindowTitleMsg("My App - Document.txt".to_string());
/// ```
///
/// # Platform Support
///
/// - **Unix/Linux**: Generally supported in most terminal emulators
/// - **macOS**: Supported in Terminal.app and iTerm2
/// - **Windows**: Supported in Windows Terminal and newer console hosts
#[derive(Debug, Clone)]
pub struct SetWindowTitleMsg(pub String);

/// An internal message used to start a recurring timer.
///
/// This structure is used internally by the framework to manage recurring
/// timers created with the `every()` command. It contains the timer's
/// configuration and a cancellation token for stopping the timer.
///
/// # Note
///
/// This is not exposed as a public API and should not be used directly
/// by application code. Use the `every()` command function instead.
pub struct EveryMsgInternal {
    /// Interval between timer ticks.
    pub duration: std::time::Duration,
    /// Function invoked on each tick producing a message.
    pub func: Box<dyn Fn(std::time::Duration) -> Msg + Send>,
    /// Token used to cancel the running timer.
    pub cancellation_token: CancellationToken,
    /// Unique identifier for this timer instance.
    pub timer_id: u64,
}

impl std::fmt::Debug for EveryMsgInternal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EveryMsgInternal")
            .field("duration", &self.duration)
            .field("timer_id", &self.timer_id)
            .field("func", &"<closure>")
            .finish()
    }
}

/// A message to cancel a specific timer.
///
/// This message stops a running timer identified by its unique ID.
/// If the timer has already completed or been cancelled, this message
/// has no effect.
///
/// # Fields
///
/// * `timer_id` - The unique identifier of the timer to cancel
///
/// # Examples
///
/// ```
/// use bubble_t::event::{CancelTimerMsg, next_timer_id};
///
/// // Cancel a specific timer
/// let timer_id = next_timer_id();
/// let cancel_msg = CancelTimerMsg { timer_id };
/// ```
#[derive(Debug, Clone)]
pub struct CancelTimerMsg {
    /// The unique identifier of the timer to cancel.
    pub timer_id: u64,
}

/// A message to cancel all active timers.
///
/// This message stops all currently running timers in the program.
/// This is useful during cleanup or when transitioning between different
/// application states.
///
/// # Examples
///
/// ```
/// use bubble_t::event::CancelAllTimersMsg;
///
/// // Cancel all timers
/// let cancel_all = CancelAllTimersMsg;
/// ```
///
/// # Use Cases
///
/// - Application shutdown
/// - State transitions that invalidate existing timers
/// - Error recovery scenarios
#[derive(Debug, Clone)]
pub struct CancelAllTimersMsg;

// --- v2 terminal query / response messages ---

/// Terminal color profile detected at startup or upgraded via capability query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColorProfileMsg(pub colorprofile::Profile);

/// Environment variables visible to the program (useful over SSH).
#[derive(Debug, Clone, Default)]
pub struct EnvMsg(pub std::collections::HashMap<String, String>);

impl EnvMsg {
    /// Returns the value of `key`, or an empty string when absent.
    pub fn getenv(&self, key: &str) -> String {
        self.0.get(key).cloned().unwrap_or_default()
    }

    /// Returns `(value, true)` when `key` is present in the environment map.
    pub fn lookup_env(&self, key: &str) -> (String, bool) {
        match self.0.get(key) {
            Some(v) => (v.clone(), true),
            None => (String::new(), false),
        }
    }
}

/// OSC 52 clipboard read response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardMsg {
    /// Pasted or read clipboard text.
    pub content: String,
    /// Selection byte: `c` system clipboard, `p` primary selection.
    pub selection: char,
}

/// Default terminal foreground color (OSC 10 response).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForegroundColorMsg(pub ansi::color::RgbColor);

impl ForegroundColorMsg {
    /// Returns whether the color reads as dark using relative luminance.
    pub fn is_dark(&self) -> bool {
        color_is_dark(self.0)
    }
}

/// Default terminal background color (OSC 11 response).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackgroundColorMsg(pub ansi::color::RgbColor);

impl BackgroundColorMsg {
    /// Returns whether the color reads as dark using relative luminance.
    pub fn is_dark(&self) -> bool {
        color_is_dark(self.0)
    }
}

/// Terminal cursor color (OSC 12 response).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorColorMsg(pub ansi::color::RgbColor);

impl CursorColorMsg {
    /// Returns whether the color reads as dark using relative luminance.
    pub fn is_dark(&self) -> bool {
        color_is_dark(self.0)
    }
}

/// Returns whether the terminal background is dark (convenience for adaptive styling).
pub fn has_dark_background(msg: &BackgroundColorMsg) -> bool {
    msg.is_dark()
}

/// Cursor position report (CPR / DECXCPR response).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorPositionMsg {
    /// Zero-based column.
    pub x: u16,
    /// Zero-based row.
    pub y: u16,
}

/// Terminal name/version from XTVERSION.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalVersionMsg {
    /// Reported terminal name/version string.
    pub name: String,
}

/// Termcap/Terminfo capability response (XTGETTCAP).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityMsg {
    /// Capability payload (e.g. `RGB`, `Tc`).
    pub content: String,
}

/// DEC mode report response (DECRPM).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModeReportMsg {
    /// Reported mode.
    pub mode: ansi::mode::Mode,
    /// Reported setting.
    pub value: ansi::mode::ModeSetting,
}

/// Kitty keyboard enhancement flags reported by the terminal (`CSI ? u`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyboardEnhancementsMsg {
    /// Bitmask of enabled Kitty keyboard features.
    pub flags: i32,
}

/// Terminal light/dark preference report (`CSI ? 997 ; mode n`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LightDarkMsg {
    /// `true` when the terminal reports dark mode.
    pub dark: bool,
}

impl LightDarkMsg {
    /// Returns whether the terminal prefers a dark color scheme.
    pub fn is_dark(self) -> bool {
        self.dark
    }
}

impl KeyboardEnhancementsMsg {
    /// Returns whether key disambiguation is supported (any non-zero flags).
    pub fn supports_key_disambiguation(self) -> bool {
        self.flags > 0
    }

    /// Returns whether press/release/repeat events are supported.
    pub fn supports_event_types(self) -> bool {
        self.flags & ansi::kitty::KITTY_REPORT_EVENT_TYPES != 0
    }

    /// Returns whether alternate key codes are reported.
    pub fn supports_alternate_keys(self) -> bool {
        self.flags & ansi::kitty::KITTY_REPORT_ALTERNATE_KEYS != 0
    }

    /// Returns whether all keys are reported as escape codes.
    pub fn supports_all_keys_as_escape_codes(self) -> bool {
        self.flags & ansi::kitty::KITTY_REPORT_ALL_KEYS_AS_ESCAPE_CODES != 0
    }

    /// Returns whether associated text is reported with key events.
    pub fn supports_associated_text(self) -> bool {
        self.flags & ansi::kitty::KITTY_REPORT_ASSOCIATED_KEYS != 0
    }
}

// --- Internal terminal command messages (handled by the program runtime) ---

/// Writes raw escape data to the terminal without processing.
#[derive(Debug, Clone)]
pub struct RawCmdMsg(pub String);

/// Requests cursor position (CPR).
#[derive(Debug, Clone, Copy)]
pub struct RequestCursorPositionCmdMsg;

/// Requests default foreground color (OSC 10 query).
#[derive(Debug, Clone, Copy)]
pub struct RequestForegroundColorCmdMsg;

/// Requests default background color (OSC 11 query).
#[derive(Debug, Clone, Copy)]
pub struct RequestBackgroundColorCmdMsg;

/// Requests cursor color (OSC 12 query).
#[derive(Debug, Clone, Copy)]
pub struct RequestCursorColorCmdMsg;

/// Requests terminal version (XTVERSION).
#[derive(Debug, Clone, Copy)]
pub struct RequestTerminalVersionCmdMsg;

/// Requests a terminfo capability (XTGETTCAP).
#[derive(Debug, Clone)]
pub struct RequestCapabilityCmdMsg(pub String);

/// Sets the system clipboard via OSC 52.
#[derive(Debug, Clone)]
pub struct SetClipboardCmdMsg(pub String);

/// Reads the system clipboard via OSC 52.
#[derive(Debug, Clone, Copy)]
pub struct ReadClipboardCmdMsg;

/// Sets the primary selection clipboard via OSC 52.
#[derive(Debug, Clone)]
pub struct SetPrimaryClipboardCmdMsg(pub String);

/// Reads the primary selection clipboard via OSC 52.
#[derive(Debug, Clone, Copy)]
pub struct ReadPrimaryClipboardCmdMsg;

fn color_is_dark(c: ansi::color::RgbColor) -> bool {
    let r = f64::from(c.r);
    let g = f64::from(c.g);
    let b = f64::from(c.b);
    (0.299 * r + 0.587 * g + 0.114 * b) / 255.0 < 0.5
}
