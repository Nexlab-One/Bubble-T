//! Terminal management and abstraction for bubble-t.
//!
//! This module provides terminal interfaces and implementations for managing
//! terminal state, input/output operations, and feature toggling. It includes
//! both a full-featured terminal implementation using crossterm and a dummy
//! implementation for testing purposes.
//!
//! # Key Components
//!
//! - [`TerminalInterface`]: Trait defining terminal operations
//! - [`Terminal`]: Full crossterm-based terminal implementation
//! - [`DummyTerminal`]: No-op terminal for testing
//!
//! # Features
//!
//! - Raw mode management for capturing all key events
//! - Alternate screen buffer support
//! - Mouse event capture with different motion reporting levels
//! - Focus change reporting
//! - Bracketed paste mode for distinguishing pasted vs typed text
//! - Cursor visibility control
//! - Efficient rendering with buffering

use crate::Error;
use crate::renderer::{CursedRenderer, RenderFrameOptions};
use crate::view::Cursor;
use ansi::background::set_cursor_color;
use ansi::cursor::cursor_position;
use ansi::mode::{
    MODE_MOUSE_EXT_SGR, MODE_MOUSE_NORMAL, MODE_MOUSE_X10, Mode, RESET_MODE_BRACKETED_PASTE,
    SET_MODE_BRACKETED_PASTE, reset_mode, set_mode,
};
use ansi::title::set_window_title as ansi_set_window_title;
use crossterm::{
    cursor::{Hide, Show},
    event::{DisableFocusChange, EnableFocusChange},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{self, Write};
use std::sync::Arc;
use tokio::io::AsyncWrite;
use tokio::sync::Mutex;

/// A trait for abstracting terminal operations.
///
/// This trait provides a unified interface for terminal management across
/// different implementations. It supports both direct terminal access and
/// custom output writers for testing or alternative output destinations.
///
/// # Design Philosophy
///
/// All methods are designed to be idempotent - calling them multiple times
/// with the same parameters should be safe and efficient. Implementations
/// should track state to avoid unnecessary system calls.
///
/// # Example
///
/// ```rust
/// use bubble_t::terminal::{TerminalInterface, Terminal};
/// use bubble_t::Error;
///
/// # async fn example() -> Result<(), Error> {
/// let mut terminal = Terminal::new(None)?;
/// terminal.enter_raw_mode().await?;
/// terminal.hide_cursor().await?;
/// terminal.render("Hello, world!").await?;
/// terminal.show_cursor().await?;
/// terminal.exit_raw_mode().await?;
/// # Ok(())
/// # }
/// ```
#[async_trait::async_trait]
pub trait TerminalInterface {
    /// Construct a new terminal implementation.
    ///
    /// Accepts an optional asynchronous output writer. When provided, rendering
    /// will write to this writer instead of stdout. This is useful for testing
    /// or redirecting output to files, network streams, or other destinations.
    ///
    /// # Arguments
    ///
    /// * `output_writer` - Optional custom output destination. If `None`, uses stdout.
    ///
    /// # Returns
    ///
    /// A new terminal implementation instance.
    ///
    /// # Errors
    ///
    /// Returns an error if terminal initialization fails or if the output writer
    /// cannot be set up properly.
    fn new(output_writer: Option<Arc<Mutex<dyn AsyncWrite + Send + Unpin>>>) -> Result<Self, Error>
    where
        Self: Sized;
    /// Enable raw mode (disables canonical input processing).
    ///
    /// Raw mode allows the application to receive all key events immediately
    /// without line buffering or special character processing. This is essential
    /// for interactive TUI applications.
    ///
    /// # Effects
    ///
    /// - Disables line buffering (canonical mode)
    /// - Disables echo of typed characters
    /// - Enables immediate key event delivery
    /// - Disables special character processing (Ctrl+C, Ctrl+Z, etc.)
    ///
    /// # Errors
    ///
    /// Returns an error if the terminal cannot be switched to raw mode.
    async fn enter_raw_mode(&mut self) -> Result<(), Error>;
    /// Disable raw mode and restore canonical input processing.
    ///
    /// Restores the terminal to its original state with line buffering,
    /// echo, and special character processing enabled.
    ///
    /// # Effects
    ///
    /// - Re-enables line buffering (canonical mode)
    /// - Re-enables echo of typed characters
    /// - Restores special character processing
    ///
    /// # Errors
    ///
    /// Returns an error if the terminal cannot be restored to canonical mode.
    async fn exit_raw_mode(&mut self) -> Result<(), Error>;
    /// Enter the alternate screen buffer.
    ///
    /// Switches to an alternate screen buffer, preserving the current terminal
    /// content. This allows the application to run in a "clean" screen that
    /// can be restored when the application exits.
    ///
    /// # Effects
    ///
    /// - Saves current screen content
    /// - Switches to alternate buffer
    /// - Clears the alternate buffer
    ///
    /// # Errors
    ///
    /// Returns an error if the terminal doesn't support alternate screens
    /// or if the switch fails.
    async fn enter_alt_screen(&mut self) -> Result<(), Error>;
    /// Leave the alternate screen buffer.
    ///
    /// Switches back to the main screen buffer, restoring the original
    /// terminal content that was visible before entering alternate screen.
    ///
    /// # Effects
    ///
    /// - Restores original screen content
    /// - Switches back to main buffer
    ///
    /// # Errors
    ///
    /// Returns an error if the switch back to main screen fails.
    async fn exit_alt_screen(&mut self) -> Result<(), Error>;
    /// Enable basic mouse capture.
    ///
    /// Enables the terminal to capture mouse events including clicks,
    /// releases, and basic movement. The application will receive mouse
    /// events through the normal input stream.
    ///
    /// # Errors
    ///
    /// Returns an error if mouse capture cannot be enabled.
    async fn enable_mouse(&mut self) -> Result<(), Error>;
    /// Enable cell-motion mouse reporting.
    ///
    /// Enables mouse motion reporting when the mouse moves between different
    /// character cells. This provides more detailed movement tracking than
    /// basic mouse capture while being less intensive than all-motion reporting.
    ///
    /// # Errors
    ///
    /// Returns an error if cell-motion mouse reporting cannot be enabled.
    async fn enable_mouse_cell_motion(&mut self) -> Result<(), Error>;
    /// Enable high-resolution mouse reporting.
    ///
    /// Enables reporting of all mouse movement, including sub-cell movements.
    /// This provides the highest fidelity mouse tracking but generates many
    /// more events and should be used carefully to avoid overwhelming the
    /// application.
    ///
    /// # Performance Note
    ///
    /// This mode generates significantly more events than other mouse modes.
    /// Use only when precise mouse tracking is required.
    ///
    /// # Errors
    ///
    /// Returns an error if all-motion mouse reporting cannot be enabled.
    async fn enable_mouse_all_motion(&mut self) -> Result<(), Error>;
    /// Disable all mouse capture modes.
    ///
    /// Disables mouse event capture and reporting. After calling this,
    /// the terminal will not send mouse events to the application.
    ///
    /// # Errors
    ///
    /// Returns an error if mouse capture cannot be disabled.
    async fn disable_mouse(&mut self) -> Result<(), Error>;
    /// Enable terminal focus change reporting.
    ///
    /// Enables the terminal to report when it gains or loses focus.
    /// The application will receive focus/blur events when the terminal
    /// window becomes active or inactive.
    ///
    /// # Use Cases
    ///
    /// - Pausing animations when the terminal loses focus
    /// - Changing display intensity or colors
    /// - Triggering auto-save when focus is lost
    ///
    /// # Errors
    ///
    /// Returns an error if focus reporting cannot be enabled.
    async fn enable_focus_reporting(&mut self) -> Result<(), Error>;
    /// Disable terminal focus change reporting.
    ///
    /// Disables focus change event reporting. The terminal will no longer
    /// send focus/blur events to the application.
    ///
    /// # Errors
    ///
    /// Returns an error if focus reporting cannot be disabled.
    async fn disable_focus_reporting(&mut self) -> Result<(), Error>;
    /// Enable bracketed paste mode.
    ///
    /// Enables bracketed paste mode, which wraps pasted text in special
    /// escape sequences. This allows the application to distinguish between
    /// text that was typed character-by-character and text that was pasted
    /// as a block.
    ///
    /// # Benefits
    ///
    /// - Prevents auto-indentation from corrupting pasted code
    /// - Allows special handling of large text blocks
    /// - Improves security by identifying untrusted input
    ///
    /// # Errors
    ///
    /// Returns an error if bracketed paste mode cannot be enabled.
    async fn enable_bracketed_paste(&mut self) -> Result<(), Error>;
    /// Disable bracketed paste mode.
    ///
    /// Disables bracketed paste mode, returning to normal paste behavior
    /// where pasted text is indistinguishable from typed text.
    ///
    /// # Errors
    ///
    /// Returns an error if bracketed paste mode cannot be disabled.
    async fn disable_bracketed_paste(&mut self) -> Result<(), Error>;
    /// Show the cursor if hidden.
    ///
    /// Makes the cursor visible if it was previously hidden. This is typically
    /// called when exiting the application or when cursor visibility is needed
    /// for user input.
    ///
    /// # Errors
    ///
    /// Returns an error if the cursor visibility cannot be changed.
    async fn show_cursor(&mut self) -> Result<(), Error>;
    /// Hide the cursor if visible.
    ///
    /// Hides the cursor from view. This is commonly done in TUI applications
    /// to prevent the cursor from interfering with the visual layout or
    /// to create a cleaner appearance.
    ///
    /// # Errors
    ///
    /// Returns an error if the cursor visibility cannot be changed.
    async fn hide_cursor(&mut self) -> Result<(), Error>;
    /// Clear the visible screen contents.
    ///
    /// Clears the entire visible screen, typically filling it with the
    /// default background color. The cursor position may be reset to
    /// the top-left corner.
    ///
    /// # Errors
    ///
    /// Returns an error if the screen cannot be cleared.
    async fn clear(&mut self) -> Result<(), Error>;
    /// Render the provided content to the terminal.
    ///
    /// Displays the given content on the terminal screen. This typically
    /// involves clearing the screen and writing the new content from the
    /// top-left corner. Newlines in the content will be properly handled
    /// for the target terminal.
    ///
    /// # Arguments
    ///
    /// * `content` - The text content to display. May contain ANSI escape
    ///   sequences for colors and formatting.
    ///
    /// # Performance
    ///
    /// Implementations should buffer output efficiently to minimize the
    /// number of system calls and reduce flicker.
    ///
    /// # Errors
    ///
    /// Returns an error if the content cannot be written to the terminal
    /// or output writer.
    /// Render the provided content to the terminal.
    async fn render(&mut self, content: &str) -> Result<(), Error>;
    /// Sets the terminal window title (OSC 2). Empty titles are ignored.
    async fn set_window_title(&mut self, title: &str) -> Result<(), Error>;
    /// Moves the text cursor to a 0-based cell position.
    async fn set_cursor_position(&mut self, x: u16, y: u16) -> Result<(), Error>;
    /// Applies cursor shape from a declarative [`Cursor`].
    async fn set_cursor_style(&mut self, cursor: &Cursor) -> Result<(), Error>;
    /// Writes raw bytes to the active output stream.
    async fn write_raw(&mut self, data: &[u8]) -> Result<(), Error>;
    /// Get the current terminal size as (columns, rows).
    ///
    /// Returns the current dimensions of the terminal in character cells.
    /// This information is useful for layout calculations and ensuring
    /// content fits within the visible area.
    ///
    /// # Returns
    ///
    /// A tuple of `(width, height)` where:
    /// - `width` is the number of character columns
    /// - `height` is the number of character rows
    ///
    /// # Errors
    ///
    /// Returns an error if the terminal size cannot be determined.
    ///
    /// # Note
    ///
    /// Terminal size can change during program execution due to window
    /// resizing. Applications should handle size change events appropriately.
    fn size(&self) -> Result<(u16, u16), Error>;
}

/// Terminal state manager using crossterm for actual terminal control.
///
/// This is the primary terminal implementation that provides full terminal
/// control capabilities through the crossterm library. It maintains state
/// to ensure operations are idempotent and efficient.
///
/// # State Tracking
///
/// The terminal tracks various state flags to avoid unnecessary operations:
/// - Raw mode status
/// - Alternate screen status
/// - Mouse capture status
/// - Focus reporting status
/// - Cursor visibility
///
/// # Performance
///
/// - Uses a pre-allocated render buffer to minimize allocations
/// - Tracks state to avoid redundant terminal operations
/// - Efficiently handles newline conversion for cross-platform compatibility
///
/// # Example
///
/// ```rust
/// use bubble_t::terminal::{Terminal, TerminalInterface};
/// use bubble_t::Error;
///
/// # async fn example() -> Result<(), Error> {
/// let mut terminal = Terminal::new(None)?;
///
/// // Set up terminal for TUI mode
/// terminal.enter_raw_mode().await?;
/// terminal.enter_alt_screen().await?;
/// terminal.hide_cursor().await?;
///
/// // Render some content
/// terminal.render("Hello, TUI world!").await?;
///
/// // Clean up (or rely on Drop)
/// terminal.show_cursor().await?;
/// terminal.exit_alt_screen().await?;
/// terminal.exit_raw_mode().await?;
/// # Ok(())
/// # }
/// ```
const MODE_MOUSE_ALL_MOTION: Mode = Mode::Dec(1003);

/// Terminal state manager using crossterm for actual terminal control.
pub struct Terminal {
    raw_mode: bool,
    alt_screen: bool,
    mouse_mode: crate::view::MouseMode,
    focus_reporting: bool,
    bracketed_paste: bool,
    cursor_visible: bool,
    term_state: Option<term::State>,
    output_writer: Option<Arc<Mutex<dyn AsyncWrite + Send + Unpin>>>,
}

impl Terminal {
    /// Create a new [`Terminal`] instance.
    ///
    /// If an `output_writer` is provided, rendering is performed by writing to
    /// that asynchronous writer instead of directly to stdout.
    pub fn new(
        output_writer: Option<Arc<Mutex<dyn AsyncWrite + Send + Unpin>>>,
    ) -> Result<Self, Error> {
        Ok(Self {
            raw_mode: false,
            alt_screen: false,
            mouse_mode: crate::view::MouseMode::None,
            focus_reporting: false,
            bracketed_paste: false,
            cursor_visible: true,
            term_state: None,
            output_writer,
        })
    }

    async fn write_stdout_bytes(&mut self, data: &[u8]) -> Result<(), Error> {
        if let Some(writer) = &mut self.output_writer {
            use tokio::io::AsyncWriteExt;
            writer.lock().await.write_all(data).await?;
            writer.lock().await.flush().await?;
        } else {
            io::stdout().write_all(data)?;
            io::stdout().flush()?;
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl TerminalInterface for Terminal {
    fn new(output_writer: Option<Arc<Mutex<dyn AsyncWrite + Send + Unpin>>>) -> Result<Self, Error>
    where
        Self: Sized,
    {
        Terminal::new(output_writer)
    }

    async fn enter_raw_mode(&mut self) -> Result<(), Error> {
        if !self.raw_mode {
            let tty = term::open_tty()?;
            self.term_state = Some(term::make_raw(&tty)?);
            self.raw_mode = true;
        }
        Ok(())
    }

    async fn exit_raw_mode(&mut self) -> Result<(), Error> {
        if self.raw_mode {
            if let Some(state) = self.term_state.take() {
                let tty = term::open_tty()?;
                term::restore(&tty, &state)?;
            }
            self.raw_mode = false;
        }
        Ok(())
    }

    async fn enter_alt_screen(&mut self) -> Result<(), Error> {
        if !self.alt_screen {
            execute!(io::stdout(), EnterAlternateScreen)?;
            execute!(io::stdout(), terminal::Clear(terminal::ClearType::All))?;
            io::stdout().flush()?;
            self.alt_screen = true;
        }
        Ok(())
    }

    async fn exit_alt_screen(&mut self) -> Result<(), Error> {
        if self.alt_screen {
            execute!(io::stdout(), LeaveAlternateScreen)?;
            io::stdout().flush()?;
            self.alt_screen = false;
        }
        Ok(())
    }

    async fn enable_mouse(&mut self) -> Result<(), Error> {
        if self.mouse_mode != crate::view::MouseMode::CellMotion {
            let seq = set_mode(&[MODE_MOUSE_NORMAL, MODE_MOUSE_EXT_SGR]);
            self.write_stdout_bytes(seq.as_bytes()).await?;
            self.mouse_mode = crate::view::MouseMode::CellMotion;
        }
        Ok(())
    }

    async fn enable_mouse_cell_motion(&mut self) -> Result<(), Error> {
        self.enable_mouse().await
    }

    async fn enable_mouse_all_motion(&mut self) -> Result<(), Error> {
        if self.mouse_mode != crate::view::MouseMode::AllMotion {
            let seq = set_mode(&[MODE_MOUSE_NORMAL, MODE_MOUSE_ALL_MOTION, MODE_MOUSE_EXT_SGR]);
            self.write_stdout_bytes(seq.as_bytes()).await?;
            self.mouse_mode = crate::view::MouseMode::AllMotion;
        }
        Ok(())
    }

    async fn disable_mouse(&mut self) -> Result<(), Error> {
        if self.mouse_mode != crate::view::MouseMode::None {
            let seq = reset_mode(&[
                MODE_MOUSE_NORMAL,
                MODE_MOUSE_ALL_MOTION,
                MODE_MOUSE_EXT_SGR,
                MODE_MOUSE_X10,
            ]);
            self.write_stdout_bytes(seq.as_bytes()).await?;
            self.mouse_mode = crate::view::MouseMode::None;
        }
        Ok(())
    }

    async fn enable_focus_reporting(&mut self) -> Result<(), Error> {
        if !self.focus_reporting {
            execute!(io::stdout(), EnableFocusChange)?;
            self.focus_reporting = true;
        }
        Ok(())
    }

    async fn disable_focus_reporting(&mut self) -> Result<(), Error> {
        if self.focus_reporting {
            execute!(io::stdout(), DisableFocusChange)?;
            self.focus_reporting = false;
        }
        Ok(())
    }

    async fn enable_bracketed_paste(&mut self) -> Result<(), Error> {
        if !self.bracketed_paste {
            self.write_stdout_bytes(SET_MODE_BRACKETED_PASTE.as_bytes())
                .await?;
            self.bracketed_paste = true;
        }
        Ok(())
    }

    async fn disable_bracketed_paste(&mut self) -> Result<(), Error> {
        if self.bracketed_paste {
            self.write_stdout_bytes(RESET_MODE_BRACKETED_PASTE.as_bytes())
                .await?;
            self.bracketed_paste = false;
        }
        Ok(())
    }

    async fn show_cursor(&mut self) -> Result<(), Error> {
        if !self.cursor_visible {
            execute!(io::stdout(), Show)?;
            self.cursor_visible = true;
        }
        Ok(())
    }

    async fn hide_cursor(&mut self) -> Result<(), Error> {
        if self.cursor_visible {
            execute!(io::stdout(), Hide)?;
            self.cursor_visible = false;
        }
        Ok(())
    }

    async fn clear(&mut self) -> Result<(), Error> {
        execute!(io::stdout(), terminal::Clear(terminal::ClearType::All))?;
        Ok(())
    }

    async fn render(&mut self, content: &str) -> Result<(), Error> {
        let mut renderer = CursedRenderer::new(80, 24, colorprofile::Profile::TrueColor);
        let output = renderer.render(content, &RenderFrameOptions::default());
        self.write_raw(output.as_bytes()).await
    }

    async fn write_raw(&mut self, data: &[u8]) -> Result<(), Error> {
        self.write_stdout_bytes(data).await
    }

    async fn set_window_title(&mut self, title: &str) -> Result<(), Error> {
        if title.is_empty() {
            return Ok(());
        }
        let seq = ansi_set_window_title(title);
        self.write_stdout_bytes(seq.as_bytes()).await
    }

    async fn set_cursor_position(&mut self, x: u16, y: u16) -> Result<(), Error> {
        let seq = cursor_position(i32::from(y) + 1, i32::from(x) + 1);
        self.write_stdout_bytes(seq.as_bytes()).await
    }

    async fn set_cursor_style(&mut self, cursor: &Cursor) -> Result<(), Error> {
        let shape = cursor_shape_param(cursor.shape, cursor.blink);
        let seq = format!("\x1b[{shape} q");
        self.write_stdout_bytes(seq.as_bytes()).await?;
        if let Some(color) = cursor.color {
            let hex = cursor_color_hex(color);
            let seq = set_cursor_color(&hex);
            self.write_stdout_bytes(seq.as_bytes()).await?;
        }
        Ok(())
    }

    fn size(&self) -> Result<(u16, u16), Error> {
        if let Ok(size) = term::open_tty_size() {
            return Ok((size.width, size.height));
        }
        let (width, height) = terminal::size()?;
        Ok((width, height))
    }
}

fn cursor_shape_param(shape: crate::view::CursorShape, blink: bool) -> u8 {
    match (shape, blink) {
        (crate::view::CursorShape::Block, true) => 1,
        (crate::view::CursorShape::Block, false) => 2,
        (crate::view::CursorShape::Underline, true) => 3,
        (crate::view::CursorShape::Underline, false) => 4,
        (crate::view::CursorShape::Bar, true) => 5,
        (crate::view::CursorShape::Bar, false) => 6,
    }
}

fn cursor_color_hex(color: ansi::color::Color) -> String {
    use ansi::color::{Color, indexed_to_rgb};
    let rgb = match color {
        Color::Rgb(c) => c,
        Color::Indexed(i) => indexed_to_rgb(i.0),
        Color::Basic(c) => indexed_to_rgb(c as u8),
    };
    format!("#{:02x}{:02x}{:02x}", rgb.r, rgb.g, rgb.b)
}

impl Drop for Terminal {
    fn drop(&mut self) {
        if !self.cursor_visible {
            let _ = execute!(io::stdout(), Show);
        }
        if self.mouse_mode != crate::view::MouseMode::None {
            let _ = io::stdout().write_all(
                reset_mode(&[
                    MODE_MOUSE_NORMAL,
                    MODE_MOUSE_ALL_MOTION,
                    MODE_MOUSE_EXT_SGR,
                    MODE_MOUSE_X10,
                ])
                .as_bytes(),
            );
        }
        if self.focus_reporting {
            let _ = execute!(io::stdout(), DisableFocusChange);
        }
        if self.bracketed_paste {
            let _ = io::stdout().write_all(RESET_MODE_BRACKETED_PASTE.as_bytes());
        }
        if self.alt_screen {
            let _ = execute!(io::stdout(), LeaveAlternateScreen);
            let _ = io::stdout().flush();
        }
        if self.raw_mode
            && let Some(state) = self.term_state.take()
            && let Ok(tty) = term::open_tty()
        {
            let _ = term::restore(&tty, &state);
        }
    }
}

/// A no-op terminal implementation useful for tests and headless operation.
///
/// This terminal implementation provides the `TerminalInterface` without
/// actually performing any terminal operations. It's designed for testing,
/// headless environments, or situations where terminal control is not needed.
///
/// # Use Cases
///
/// - Unit testing TUI applications without requiring a real terminal
/// - Running applications in headless environments
/// - Debugging and development scenarios
/// - Performance testing without terminal I/O overhead
///
/// # Behavior
///
/// - All terminal control methods return success without doing anything
/// - `render()` writes to the output writer if provided, otherwise does nothing
/// - `size()` returns `(0, 0)` as a placeholder
///
/// # Example
///
/// ```rust
/// use bubble_t::terminal::{DummyTerminal, TerminalInterface};
/// use bubble_t::Error;
/// use std::sync::Arc;
/// use tokio::sync::Mutex;
///
/// # async fn example() -> Result<(), Error> {
/// // Create with no output (all operations are no-ops)
/// let mut dummy = DummyTerminal::new(None)?;
///
/// // These all succeed but do nothing
/// dummy.enter_raw_mode().await?;
/// dummy.hide_cursor().await?;
/// dummy.render("This won't be displayed").await?;
/// # Ok(())
/// # }
/// ```
pub struct DummyTerminal {
    output_writer: Option<Arc<Mutex<dyn AsyncWrite + Send + Unpin>>>,
}

#[async_trait::async_trait]
impl TerminalInterface for DummyTerminal {
    fn new(
        output_writer: Option<Arc<Mutex<dyn AsyncWrite + Send + Unpin>>>,
    ) -> Result<Self, Error> {
        Ok(Self { output_writer })
    }
    async fn enter_raw_mode(&mut self) -> Result<(), Error> {
        Ok(())
    }
    async fn exit_raw_mode(&mut self) -> Result<(), Error> {
        Ok(())
    }
    async fn enter_alt_screen(&mut self) -> Result<(), Error> {
        Ok(())
    }
    async fn exit_alt_screen(&mut self) -> Result<(), Error> {
        Ok(())
    }
    async fn enable_mouse(&mut self) -> Result<(), Error> {
        Ok(())
    }
    async fn enable_mouse_cell_motion(&mut self) -> Result<(), Error> {
        Ok(())
    }
    async fn enable_mouse_all_motion(&mut self) -> Result<(), Error> {
        Ok(())
    }
    async fn disable_mouse(&mut self) -> Result<(), Error> {
        Ok(())
    }
    async fn enable_focus_reporting(&mut self) -> Result<(), Error> {
        Ok(())
    }
    async fn disable_focus_reporting(&mut self) -> Result<(), Error> {
        Ok(())
    }
    async fn enable_bracketed_paste(&mut self) -> Result<(), Error> {
        Ok(())
    }
    async fn disable_bracketed_paste(&mut self) -> Result<(), Error> {
        Ok(())
    }
    async fn show_cursor(&mut self) -> Result<(), Error> {
        Ok(())
    }
    async fn hide_cursor(&mut self) -> Result<(), Error> {
        Ok(())
    }
    async fn clear(&mut self) -> Result<(), Error> {
        Ok(())
    }
    async fn render(&mut self, content: &str) -> Result<(), Error> {
        if let Some(writer) = &mut self.output_writer {
            use tokio::io::AsyncWriteExt;
            writer.lock().await.write_all(content.as_bytes()).await?;
            writer.lock().await.flush().await?;
        }
        Ok(())
    }
    async fn set_window_title(&mut self, _: &str) -> Result<(), Error> {
        Ok(())
    }
    async fn set_cursor_position(&mut self, _: u16, _: u16) -> Result<(), Error> {
        Ok(())
    }
    async fn set_cursor_style(&mut self, _: &Cursor) -> Result<(), Error> {
        Ok(())
    }
    async fn write_raw(&mut self, data: &[u8]) -> Result<(), Error> {
        if let Some(writer) = &mut self.output_writer {
            use tokio::io::AsyncWriteExt;
            writer.lock().await.write_all(data).await?;
            writer.lock().await.flush().await?;
        }
        Ok(())
    }
    fn size(&self) -> Result<(u16, u16), Error> {
        Ok((80, 24))
    }
}
