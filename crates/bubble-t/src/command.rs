//! This module provides functions for creating and managing commands.
//! Commands are asynchronous operations that can produce messages to update the model.

use crate::event::{
    BatchCmdMsg, ClearScreenMsg, InterruptMsg, KillMsg, Msg, PrintMsg, PrintfMsg, QuitMsg,
    RawCmdMsg, ReadClipboardCmdMsg, ReadPrimaryClipboardCmdMsg, RequestBackgroundColorCmdMsg,
    RequestCapabilityCmdMsg, RequestCursorColorCmdMsg, RequestCursorPositionCmdMsg,
    RequestForegroundColorCmdMsg, RequestTerminalVersionCmdMsg, RequestWindowSizeMsg,
    SetClipboardCmdMsg, SetPrimaryClipboardCmdMsg, SuspendMsg, next_timer_id,
};
use std::future::Future;
use std::pin::Pin;
use std::process::Command as StdCommand;
use std::sync::OnceLock;
use std::time::Duration;
use tokio::process::Command as TokioCommand;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;

/// A command represents an asynchronous operation that may produce a message.
///
/// Commands are typically created by the `init` and `update` methods of your
/// `Model` and are then executed by the `Program`'s event loop.
///
/// The `Cmd` type is a `Pin<Box<dyn Future<Output = Option<Msg>> + Send>>`,
/// which means it's a boxed, pinned future that returns an `Option<Msg>`.
/// If the command produces a message, it will be sent back to the `Program`
/// to be processed by the `update` method.
pub type Cmd = Pin<Box<dyn Future<Output = Option<Msg>> + Send>>;

/// A batch command that executes multiple commands concurrently.
///
/// This struct is used internally by the `batch` function to group multiple
/// commands together for concurrent execution.
#[allow(dead_code)]
pub struct Batch {
    commands: Vec<Cmd>,
}

#[allow(dead_code)]
impl Batch {
    /// Creates a new `Batch` from a vector of `Cmd`s.
    pub(crate) fn new(commands: Vec<Cmd>) -> Self {
        Self { commands }
    }

    /// Consumes the `Batch` and returns the inner vector of `Cmd`s.
    pub(crate) fn into_commands(self) -> Vec<Cmd> {
        self.commands
    }
}

/// Global environment variables to be applied to external process commands.
///
/// Set by `Program::new()` from `ProgramConfig.environment` and read by
/// `exec_process` when spawning commands. If unset, no variables are injected.
pub static COMMAND_ENV: OnceLock<std::collections::HashMap<String, String>> = OnceLock::new();

/// Creates a command that quits the application.
///
/// This command sends a `QuitMsg` to the program, which will initiate the
/// shutdown process.
///
/// # Examples
///
/// ```
/// use bubble_t::{command, Model, Msg, KeyMsg};
/// use crossterm::event::KeyCode;
///
/// struct MyModel;
///
/// impl Model for MyModel {
///     fn init() -> (Self, Option<command::Cmd>) {
///         (Self {}, None)
///     }
///
///     fn update(&mut self, msg: Msg) -> Option<command::Cmd> {
///         // Quit when 'q' is pressed
///         if let Some(key_msg) = msg.downcast_ref::<KeyMsg>() {
///             if key_msg.key == KeyCode::Char('q') {
///                 return Some(command::quit());
///             }
///         }
///         None
///     }
///     
///     fn view(&self) -> bubble_t::View {
///         bubble_t::View::new("Press 'q' to quit")
///     }
/// }
/// ```
pub fn quit() -> Cmd {
    Box::pin(async { Some(Box::new(QuitMsg) as Msg) })
}

/// Creates a command that kills the application immediately.
///
/// This command sends a `KillMsg` to the program, which will cause the event loop
/// to terminate as soon as possible with `Error::ProgramKilled`.
///
/// # Examples
///
/// ```
/// use bubble_t::{command, Model, Msg};
///
/// struct MyModel {
///     has_error: bool,
/// }
///
/// impl Model for MyModel {
///     fn init() -> (Self, Option<command::Cmd>) {
///         (Self { has_error: false }, None)
///     }
///
///     fn update(&mut self, msg: Msg) -> Option<command::Cmd> {
///         // Force kill on critical error
///         if self.has_error {
///             return Some(command::kill());
///         }
///         None
///     }
///     
///     fn view(&self) -> bubble_t::View {
///         bubble_t::View::new("Running...")
///     }
/// }
/// ```
pub fn kill() -> Cmd {
    Box::pin(async { Some(Box::new(KillMsg) as Msg) })
}

/// Creates a command that interrupts the application.
///
/// This command sends an `InterruptMsg` to the program, typically used
/// to signal an external interruption (e.g., Ctrl+C).
pub fn interrupt() -> Cmd {
    Box::pin(async { Some(Box::new(InterruptMsg) as Msg) })
}

/// Creates a command that suspends the application.
///
/// This command sends a `SuspendMsg` to the program, which can be used
/// to temporarily pause the application and release terminal control.
pub fn suspend() -> Cmd {
    Box::pin(async { Some(Box::new(SuspendMsg) as Msg) })
}

/// Creates a command that executes a batch of commands concurrently.
///
/// The commands in the batch will be executed in parallel immediately when
/// this command is processed by the program. This is a non-blocking operation
/// that spawns each command in its own task, allowing for smooth animations
/// and responsive user interfaces.
///
/// # Arguments
///
/// * `cmds` - A vector of commands to execute concurrently
///
/// # Returns
///
/// A command that immediately dispatches all provided commands for concurrent execution
///
/// # Examples
///
/// ```
/// use bubble_t::{command, Model, Msg};
/// use std::time::Duration;
///
/// struct MyModel;
///
/// impl Model for MyModel {
///     fn init() -> (Self, Option<command::Cmd>) {
///         let model = Self {};
///         // Execute multiple operations concurrently
///         let cmd = command::batch(vec![
///             command::window_size(),
///             command::tick(Duration::from_secs(1), |_| {
///                 Box::new("InitialTickMsg") as Msg
///             }),
///         ]);
///         (model, Some(cmd))
///     }
///     
///     fn update(&mut self, msg: Msg) -> Option<command::Cmd> {
///         None
///     }
///     
///     fn view(&self) -> bubble_t::View {
///         bubble_t::View::new("Loading...")
///     }
/// }
/// ```
pub fn batch(cmds: Vec<Cmd>) -> Cmd {
    Box::pin(async move {
        // Don't wait for commands - just wrap them for immediate spawning
        Some(Box::new(BatchCmdMsg(cmds)) as Msg)
    })
}

/// Creates a command that executes a sequence of commands sequentially.
///
/// The commands in the sequence will be executed one after another in order.
/// All messages produced by the commands will be collected and returned.
/// This is useful when you need to perform operations that depend on the
/// completion of previous operations.
///
/// # Arguments
///
/// * `cmds` - A vector of commands to execute sequentially
///
/// # Returns
///
/// A command that executes all provided commands in sequence
///
/// # Examples
///
/// ```
/// use bubble_t::{command, Model, Msg};
///
/// struct MyModel;
///
/// impl Model for MyModel {
///     fn init() -> (Self, Option<command::Cmd>) {
///         let model = Self {};
///         // Execute operations in order
///         let cmd = command::sequence(vec![
///             command::clear_screen(),
///         ]);
///         (model, Some(cmd))
///     }
///     
///     fn update(&mut self, msg: Msg) -> Option<command::Cmd> {
///         None
///     }
///     
///     fn view(&self) -> bubble_t::View {
///         bubble_t::View::new("Ready")
///     }
/// }
/// ```
pub fn sequence(cmds: Vec<Cmd>) -> Cmd {
    Box::pin(async move {
        let mut results = Vec::new();
        for cmd in cmds {
            if let Some(msg) = cmd.await {
                results.push(msg);
            }
        }
        if results.is_empty() {
            None
        } else {
            Some(Box::new(crate::event::BatchMsgInternal { messages: results }) as Msg)
        }
    })
}

/// Creates a command that produces a single message after a delay.
///
/// This command will send a message produced by the provided closure `f`
/// after the specified `duration`. Unlike `every()`, this produces only
/// one message and then completes. It's commonly used for one-shot timers
/// that can be re-armed in the update method.
///
/// Note: Due to tokio's interval implementation, the first tick is consumed
/// to ensure the message is sent after a full duration, not immediately.
///
/// # Arguments
///
/// * `duration` - The duration to wait before sending the message
/// * `f` - A closure that takes a `Duration` and returns a `Msg`
///
/// # Returns
///
/// A command that will produce a single message after the specified duration
///
/// # Examples
///
/// ```
/// use bubble_t::{command, Model, Msg};
/// use std::time::Duration;
///
/// #[derive(Debug)]
/// struct TickMsg;
///
/// struct MyModel {
///     counter: u32,
/// }
///
/// impl Model for MyModel {
///     fn init() -> (Self, Option<command::Cmd>) {
///         let model = Self { counter: 0 };
///         // Start a timer that fires after 1 second
///         let cmd = command::tick(Duration::from_secs(1), |_| {
///             Box::new(TickMsg) as Msg
///         });
///         (model, Some(cmd))
///     }
///
///     fn update(&mut self, msg: Msg) -> Option<command::Cmd> {
///         if msg.downcast_ref::<TickMsg>().is_some() {
///             self.counter += 1;
///             // Re-arm the timer for another tick
///             return Some(command::tick(Duration::from_secs(1), |_| {
///                 Box::new(TickMsg) as Msg
///             }));
///         }
///         None
///     }
///     
///     fn view(&self) -> bubble_t::View {
///         bubble_t::View::new(format!("Counter: {}", self.counter))
///     }
/// }
/// ```
pub fn tick<F>(duration: Duration, f: F) -> Cmd
where
    F: Fn(Duration) -> Msg + Send + 'static,
{
    Box::pin(async move {
        let mut ticker = interval(duration);
        // The first tick completes immediately; advance once to move to the start
        ticker.tick().await; // consume the immediate tick
        // Now wait for one full duration before emitting
        ticker.tick().await;
        Some(f(duration))
    })
}

/// Creates a command that produces messages repeatedly at a regular interval.
///
/// This command will continuously send messages produced by the provided closure `f`
/// after every `duration` until the program exits or the timer is cancelled.
/// Unlike `tick()`, this creates a persistent timer that keeps firing.
///
/// Warning: Be careful not to call `every()` repeatedly for the same timer,
/// as this will create multiple concurrent timers that can overwhelm the
/// event loop. Instead, call it once and use `cancel_timer()` if needed.
///
/// # Arguments
///
/// * `duration` - The duration between messages
/// * `f` - A closure that takes a `Duration` and returns a `Msg`
///
/// # Returns
///
/// A command that will produce messages repeatedly at the specified interval
///
/// # Examples
///
/// ```
/// use bubble_t::{command, Model, Msg};
/// use std::time::Duration;
///
/// #[derive(Debug)]
/// struct ClockTickMsg;
///
/// struct MyModel {
///     time_elapsed: Duration,
/// }
///
/// impl Model for MyModel {
///     fn init() -> (Self, Option<command::Cmd>) {
///         let model = Self { time_elapsed: Duration::from_secs(0) };
///         // Start a timer that fires every second
///         let cmd = command::every(Duration::from_secs(1), |_| {
///             Box::new(ClockTickMsg) as Msg
///         });
///         (model, Some(cmd))
///     }
///
///     fn update(&mut self, msg: Msg) -> Option<command::Cmd> {
///         if msg.downcast_ref::<ClockTickMsg>().is_some() {
///             self.time_elapsed += Duration::from_secs(1);
///             // No need to re-arm - it keeps firing automatically
///         }
///         None
///     }
///     
///     fn view(&self) -> bubble_t::View {
///         bubble_t::View::new(format!("Time elapsed: {:?}", self.time_elapsed))
///     }
/// }
/// ```
pub fn every<F>(duration: Duration, f: F) -> Cmd
where
    F: Fn(Duration) -> Msg + Send + 'static,
{
    let timer_id = next_timer_id();
    let cancellation_token = CancellationToken::new();

    Box::pin(async move {
        Some(Box::new(crate::event::EveryMsgInternal {
            duration,
            func: Box::new(f),
            cancellation_token,
            timer_id,
        }) as Msg)
    })
}

/// Creates a command that produces messages repeatedly at a regular interval with cancellation support.
///
/// This command will continuously send messages produced by the provided closure `f`
/// after every `duration` until the program exits or the timer is cancelled.
/// The returned timer ID can be used with `cancel_timer()` to stop the timer.
///
/// # Arguments
///
/// * `duration` - The duration between messages
/// * `f` - A closure that takes a `Duration` and returns a `Msg`
///
/// # Returns
///
/// Returns a tuple containing:
/// - The command to start the timer
/// - A timer ID that can be used with `cancel_timer()`
///
/// # Examples
///
/// ```
/// use bubble_t::{command, Model, Msg};
/// use std::time::Duration;
///
/// #[derive(Debug)]
/// struct AnimationFrameMsg;
///
/// #[derive(Debug)]
/// struct StartAnimationMsg(u64); // Contains timer ID
///
/// struct MyModel {
///     animation_timer_id: Option<u64>,
///     is_animating: bool,
/// }
///
/// impl Model for MyModel {
///     fn init() -> (Self, Option<command::Cmd>) {
///         let model = Self {
///             animation_timer_id: None,
///             is_animating: false,
///         };
///         // Start animation timer and get its ID
///         let (cmd, timer_id) = command::every_with_id(
///             Duration::from_millis(16), // ~60 FPS
///             |_| Box::new(AnimationFrameMsg) as Msg
///         );
///         // Send a message with the timer ID so we can store it
///         let batch = command::batch(vec![
///             cmd,
///             Box::pin(async move {
///                 Some(Box::new(StartAnimationMsg(timer_id)) as Msg)
///             }),
///         ]);
///         (model, Some(batch))
///     }
///
///     fn update(&mut self, msg: Msg) -> Option<command::Cmd> {
///         if let Some(start_msg) = msg.downcast_ref::<StartAnimationMsg>() {
///             self.animation_timer_id = Some(start_msg.0);
///             self.is_animating = true;
///         }
///         None
///     }
///     
///     fn view(&self) -> bubble_t::View {
///         if self.is_animating {
///             bubble_t::View::new("Animating...")
///         } else {
///             bubble_t::View::new("Stopped")
///         }
///     }
/// }
/// ```
pub fn every_with_id<F>(duration: Duration, f: F) -> (Cmd, u64)
where
    F: Fn(Duration) -> Msg + Send + 'static,
{
    let timer_id = next_timer_id();
    let cancellation_token = CancellationToken::new();

    let cmd = Box::pin(async move {
        Some(Box::new(crate::event::EveryMsgInternal {
            duration,
            func: Box::new(f),
            cancellation_token,
            timer_id,
        }) as Msg)
    });

    (cmd, timer_id)
}

/// Creates a command that executes an external process.
///
/// This command spawns an external process asynchronously and returns a message
/// produced by the provided closure with the process's output. The process runs
/// in the background and doesn't block the UI.
///
/// # Arguments
///
/// * `cmd` - The `std::process::Command` to execute
/// * `f` - A closure that processes the command output and returns a `Msg`
///
/// # Returns
///
/// A command that executes the external process
///
/// # Examples
///
/// ```
/// use bubble_t::{command, Model, Msg};
/// use std::process::Command;
///
/// #[derive(Debug)]
/// struct GitStatusMsg(String);
///
/// struct MyModel {
///     git_status: String,
/// }
///
/// impl Model for MyModel {
///     fn init() -> (Self, Option<command::Cmd>) {
///         let model = Self { git_status: String::new() };
///         // Run git status command
///         let mut cmd = Command::new("git");
///         cmd.arg("status").arg("--short");
///         
///         let exec_cmd = command::exec_process(cmd, |result| {
///             match result {
///                 Ok(output) => {
///                     let status = String::from_utf8_lossy(&output.stdout).to_string();
///                     Box::new(GitStatusMsg(status)) as Msg
///                 }
///                 Err(e) => {
///                     Box::new(GitStatusMsg(format!("Error: {}", e))) as Msg
///                 }
///             }
///         });
///         (model, Some(exec_cmd))
///     }
///
///     fn update(&mut self, msg: Msg) -> Option<command::Cmd> {
///         if let Some(GitStatusMsg(status)) = msg.downcast_ref::<GitStatusMsg>() {
///             self.git_status = status.clone();
///         }
///         None
///     }
///     
///     fn view(&self) -> bubble_t::View {
///         bubble_t::View::new(format!("Git status:\n{}", self.git_status))
///     }
/// }
/// ```
pub fn exec_process<F>(cmd: StdCommand, f: F) -> Cmd
where
    F: Fn(Result<std::process::Output, std::io::Error>) -> Msg + Send + 'static,
{
    Box::pin(async move {
        // Apply configured environment variables, if any
        let mut cmd = cmd;
        if let Some(env) = crate::command::COMMAND_ENV.get() {
            for (k, v) in env.iter() {
                cmd.env(k, v);
            }
        }
        let output = TokioCommand::from(cmd).output().await;
        Some(f(output))
    })
}

/// Creates a command that clears the terminal screen.
///
/// This command sends a `ClearScreenMsg` to the program, which will clear
/// all content from the terminal screen.
pub fn clear_screen() -> Cmd {
    Box::pin(async { Some(Box::new(ClearScreenMsg) as Msg) })
}

/// Creates a command that requests the current window size.
///
/// This command sends a `RequestWindowSizeMsg` to the program. The terminal
/// will respond with a `WindowSizeMsg` containing its current dimensions.
/// This is useful for responsive layouts that adapt to terminal size.
///
/// # Examples
///
/// ```
/// use bubble_t::{command, Model, Msg, WindowSizeMsg};
///
/// struct MyModel {
///     width: u16,
///     height: u16,
/// }
///
/// impl Model for MyModel {
///     fn init() -> (Self, Option<command::Cmd>) {
///         let model = Self { width: 0, height: 0 };
///         // Get initial window size
///         (model, Some(command::window_size()))
///     }
///
///     fn update(&mut self, msg: Msg) -> Option<command::Cmd> {
///         if let Some(size_msg) = msg.downcast_ref::<WindowSizeMsg>() {
///             self.width = size_msg.width;
///             self.height = size_msg.height;
///         }
///         None
///     }
///     
///     fn view(&self) -> bubble_t::View {
///         bubble_t::View::new(format!("Window size: {}x{}", self.width, self.height))
///     }
/// }
/// ```
pub fn window_size() -> Cmd {
    Box::pin(async { Some(Box::new(RequestWindowSizeMsg) as Msg) })
}

/// Creates a command that prints a line to the terminal.
///
/// This command sends a `PrintMsg` to the program, which will print the
/// provided string to the terminal. This is useful for debugging or
/// outputting information that should appear outside the normal UI.
///
/// # Arguments
///
/// * `s` - The string to print
///
/// # Examples
///
/// ```
/// use bubble_t::{command, Model, Msg};
///
/// struct MyModel {
///     debug_mode: bool,
/// }
///
/// impl Model for MyModel {
///     fn init() -> (Self, Option<command::Cmd>) {
///         (Self { debug_mode: true }, None)
///     }
///
///     fn update(&mut self, msg: Msg) -> Option<command::Cmd> {
///         if self.debug_mode {
///             // Note: In practice, msg doesn't implement Debug by default
///             // This is just for demonstration
///             return Some(command::println(
///                 "Received a message".to_string()
///             ));
///         }
///         None
///     }
///     
///     fn view(&self) -> bubble_t::View {
///         bubble_t::View::new("Debug mode active")
///     }
/// }
/// ```
pub fn println(s: String) -> Cmd {
    Box::pin(async move { Some(Box::new(PrintMsg(s)) as Msg) })
}

/// Creates a command that prints formatted text to the terminal.
///
/// This command sends a `PrintfMsg` to the program, which will print the
/// provided formatted string to the terminal.
pub fn printf(s: String) -> Cmd {
    Box::pin(async move { Some(Box::new(PrintfMsg(s)) as Msg) })
}

/// Creates a command that cancels a specific timer.
///
/// This command sends a `CancelTimerMsg` to the program, which will stop
/// the timer with the given ID. Use this with timer IDs returned by
/// `every_with_id()` to stop repeating timers.
///
/// # Arguments
///
/// * `timer_id` - The ID of the timer to cancel
///
/// # Returns
///
/// A command that cancels the specified timer
///
/// # Examples
///
/// ```
/// use bubble_t::{command, Model, Msg, KeyMsg};
/// use crossterm::event::KeyCode;
/// use std::time::Duration;
///
/// struct MyModel {
///     timer_id: Option<u64>,
/// }
///
/// impl Model for MyModel {
///     fn init() -> (Self, Option<command::Cmd>) {
///         (Self { timer_id: Some(123) }, None)
///     }
///
///     fn update(&mut self, msg: Msg) -> Option<command::Cmd> {
///         // Cancel timer when user presses 's' for stop
///         if let Some(key_msg) = msg.downcast_ref::<KeyMsg>() {
///             if key_msg.key == KeyCode::Char('s') {
///                 if let Some(id) = self.timer_id {
///                     self.timer_id = None;
///                     return Some(command::cancel_timer(id));
///                 }
///             }
///         }
///         None
///     }
///     
///     fn view(&self) -> bubble_t::View {
///         if self.timer_id.is_some() {
///             bubble_t::View::new("Timer running. Press 's' to stop.")
///         } else {
///             bubble_t::View::new("Timer stopped.")
///         }
///     }
/// }
/// ```
pub fn cancel_timer(timer_id: u64) -> Cmd {
    Box::pin(async move { Some(Box::new(crate::event::CancelTimerMsg { timer_id }) as Msg) })
}

/// Creates a command that cancels all active timers.
///
/// This command sends a `CancelAllTimersMsg` to the program, which will stop
/// all currently running timers.
pub fn cancel_all_timers() -> Cmd {
    Box::pin(async move { Some(Box::new(crate::event::CancelAllTimersMsg) as Msg) })
}

/// Sends a raw escape sequence to the terminal without processing.
pub fn raw(seq: impl Into<String>) -> Cmd {
    let seq = seq.into();
    Box::pin(async move { Some(Box::new(RawCmdMsg(seq)) as Msg) })
}

/// Requests the terminal cursor position; the response arrives as [`CursorPositionMsg`](crate::CursorPositionMsg).
pub fn request_cursor_position() -> Cmd {
    Box::pin(async { Some(Box::new(RequestCursorPositionCmdMsg) as Msg) })
}

/// Requests the default terminal foreground color.
pub fn request_foreground_color() -> Cmd {
    Box::pin(async { Some(Box::new(RequestForegroundColorCmdMsg) as Msg) })
}

/// Requests the default terminal background color.
pub fn request_background_color() -> Cmd {
    Box::pin(async { Some(Box::new(RequestBackgroundColorCmdMsg) as Msg) })
}

/// Requests the terminal cursor color.
pub fn request_cursor_color() -> Cmd {
    Box::pin(async { Some(Box::new(RequestCursorColorCmdMsg) as Msg) })
}

/// Queries the terminal name/version via XTVERSION.
pub fn request_terminal_version() -> Cmd {
    Box::pin(async { Some(Box::new(RequestTerminalVersionCmdMsg) as Msg) })
}

/// Queries a terminfo/termcap capability (e.g. `"RGB"`, `"Tc"`).
pub fn request_capability(name: impl Into<String>) -> Cmd {
    let name = name.into();
    Box::pin(async move { Some(Box::new(RequestCapabilityCmdMsg(name)) as Msg) })
}

/// Sets the system clipboard via OSC 52.
pub fn set_clipboard(data: impl Into<String>) -> Cmd {
    let data = data.into();
    Box::pin(async move { Some(Box::new(SetClipboardCmdMsg(data)) as Msg) })
}

/// Reads the system clipboard via OSC 52.
pub fn read_clipboard() -> Cmd {
    Box::pin(async { Some(Box::new(ReadClipboardCmdMsg) as Msg) })
}

/// Sets the primary selection clipboard via OSC 52.
pub fn set_primary_clipboard(data: impl Into<String>) -> Cmd {
    let data = data.into();
    Box::pin(async move { Some(Box::new(SetPrimaryClipboardCmdMsg(data)) as Msg) })
}

/// Reads the primary selection clipboard via OSC 52.
pub fn read_primary_clipboard() -> Cmd {
    Box::pin(async { Some(Box::new(ReadPrimaryClipboardCmdMsg) as Msg) })
}

#[cfg(test)]
mod command_tests {
    use super::*;
    use crate::event::{
        RawCmdMsg, ReadClipboardCmdMsg, RequestCursorPositionCmdMsg, SetClipboardCmdMsg,
    };

    #[tokio::test]
    async fn raw_command_returns_msg() {
        let cmd = raw("\x1b[c");
        let Some(msg) = cmd.await else {
            panic!("command should emit a message");
        };
        assert!(msg.is::<RawCmdMsg>());
    }

    #[tokio::test]
    async fn request_cursor_position_returns_msg() {
        let cmd = request_cursor_position();
        let Some(msg) = cmd.await else {
            panic!("command should emit a message");
        };
        assert!(msg.is::<RequestCursorPositionCmdMsg>());
    }

    #[tokio::test]
    async fn clipboard_commands_return_msgs() {
        let Some(set) = set_clipboard("hello").await else {
            panic!("set_clipboard should emit a message");
        };
        assert!(set.is::<SetClipboardCmdMsg>());
        let Some(read) = read_clipboard().await else {
            panic!("read_clipboard should emit a message");
        };
        assert!(read.is::<ReadClipboardCmdMsg>());
    }
}
