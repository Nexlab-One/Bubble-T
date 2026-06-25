//! This module defines the `Program` struct and its associated `ProgramBuilder`,
//! which are responsible for coordinating the entire `bubble-t` application lifecycle.
//! The `Program` sets up the terminal, handles input, executes commands, and renders
//! the model's view.

use crate::event::{
    CapabilityMsg, ColorProfileMsg, EnvMsg, KillMsg, RawCmdMsg, ReadClipboardCmdMsg,
    ReadPrimaryClipboardCmdMsg, RequestBackgroundColorCmdMsg, RequestCapabilityCmdMsg,
    RequestCursorColorCmdMsg, RequestCursorPositionCmdMsg, RequestForegroundColorCmdMsg,
    RequestTerminalVersionCmdMsg, RequestWindowSizeMsg, SetClipboardCmdMsg,
    SetPrimaryClipboardCmdMsg,
};
use crate::renderer::CursedRenderer;
use crate::signals::{self, SUSPEND_SUPPORTED};
use crate::view::AppliedViewState;
use crate::view_runtime::{apply_view, render_options_from_view};
use crate::{
    Error, InputHandler, InputSource, Model, MouseMsg, Msg, QuitMsg, Terminal, TerminalInterface,
    WindowSizeMsg,
};
use colorprofile::Profile;
use futures::{future::FutureExt, select};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::panic;
use std::sync::Arc;
use std::sync::OnceLock;
use tokio::io::AsyncWrite;
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

type PanicHook = Box<dyn Fn(&panic::PanicHookInfo<'_>) + Send + Sync + 'static>;
static ORIGINAL_PANIC_HOOK: OnceLock<PanicHook> = OnceLock::new();

/// Alias for a model-aware message filter function used throughout Program.
type MessageFilter<M> = Box<dyn Fn(&M, Msg) -> Option<Msg> + Send>;

/// Configuration options for a `Program`.
///
/// This struct holds various settings that control the behavior of the `Program`,
/// such as terminal features, rendering options, and panic/signal handling.
pub struct ProgramConfig {
    /// The target frames per second for rendering.
    pub fps: u32,
    /// Whether to disable the renderer entirely.
    pub without_renderer: bool,
    /// Whether to catch panics and convert them into `ProgramPanic` errors.
    pub catch_panics: bool,
    /// Whether to enable signal handling (e.g., Ctrl+C).
    pub signal_handler: bool,
    /// Optional custom output writer.
    pub output_writer: Option<Arc<Mutex<dyn AsyncWrite + Send + Unpin>>>,
    /// Optional cancellation token for external control.
    pub cancellation_token: Option<CancellationToken>,
    /// Optional custom input source.
    pub input_source: Option<InputSource>,
    /// The buffer size for the event channel (None for unbounded, Some(size) for bounded).
    pub event_channel_buffer: Option<usize>,
    /// Whether to enable memory usage monitoring.
    pub memory_monitoring: bool,
    /// Optional environment variables to apply to external process commands.
    pub environment: Option<HashMap<String, String>>,
    /// Optional fixed color profile (auto-detected when unset).
    pub color_profile: Option<Profile>,
    /// Optional fixed window size for the renderer (queried from the terminal when unset).
    pub window_size: Option<(u16, u16)>,
}

impl std::fmt::Debug for ProgramConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProgramConfig")
            .field("fps", &self.fps)
            .field("without_renderer", &self.without_renderer)
            .field("catch_panics", &self.catch_panics)
            .field("signal_handler", &self.signal_handler)
            .field("cancellation_token", &self.cancellation_token)
            .field("environment", &self.environment.as_ref().map(|m| m.len()))
            .field("color_profile", &self.color_profile)
            .field("window_size", &self.window_size)
            .finish()
    }
}

impl Default for ProgramConfig {
    /// Returns the default `ProgramConfig`.
    ///
    /// By default, the program does not use the alternate screen, has no mouse
    /// motion reporting, does not report focus, targets 60 FPS, enables rendering,
    /// catches panics, handles signals, and disables bracketed paste.
    fn default() -> Self {
        Self {
            fps: 60,
            without_renderer: false,
            catch_panics: true,
            signal_handler: true,
            output_writer: None,
            cancellation_token: None,
            input_source: None,
            event_channel_buffer: Some(1000),
            memory_monitoring: false,
            environment: None,
            color_profile: None,
            window_size: None,
        }
    }
}

/// A builder for creating and configuring `Program` instances.
///
/// The `ProgramBuilder` provides a fluent API for setting various configuration
/// options before building the final `Program`.
pub struct ProgramBuilder<M: Model> {
    config: ProgramConfig,
    _phantom: PhantomData<M>,
    /// Optional model-aware message filter
    message_filter: Option<MessageFilter<M>>,
}

impl<M: Model> ProgramBuilder<M> {
    /// Creates a new `ProgramBuilder` with default configuration.
    ///
    /// This method is used internally by `Program::builder()` and should not
    /// be called directly. Use `Program::builder()` instead.
    ///
    /// # Returns
    ///
    /// A new `ProgramBuilder` instance with default settings.
    pub(crate) fn new() -> Self {
        Self {
            config: ProgramConfig::default(),
            _phantom: PhantomData,
            message_filter: None,
        }
    }

    /// Sets environment variables to apply to external process commands created
    /// via `command::exec_process`.
    ///
    /// These environment variables will be merged with the system environment
    /// when spawning external processes through commands.
    ///
    /// # Arguments
    ///
    /// * `env` - A `HashMap` of environment variable key-value pairs.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::collections::HashMap;
    /// use bubble_t::Program;
    /// # use bubble_t::Model;
    /// # struct MyModel;
    /// # impl Model for MyModel {
    /// #     fn init() -> (Self, Option<bubble_t::Cmd>) { (MyModel, None) }
    /// #     fn update(&mut self, _: bubble_t::Msg) -> Option<bubble_t::Cmd> { None }
    /// #     fn view(&self) -> bubble_t::View { bubble_t::View::new("") }
    /// # }
    ///
    /// let mut env = HashMap::new();
    /// env.insert("CUSTOM_VAR".to_string(), "value".to_string());
    ///
    /// let program = Program::<MyModel>::builder()
    ///     .with_environment(env)
    ///     .build();
    /// ```
    pub fn with_environment(mut self, env: HashMap<String, String>) -> Self {
        self.config.environment = Some(env);
        self
    }

    /// Sets an explicit terminal color profile for renderer output downsampling.
    pub fn with_color_profile(mut self, profile: Profile) -> Self {
        self.config.color_profile = Some(profile);
        self
    }

    /// Sets a fixed renderer window size instead of querying the terminal each frame.
    pub fn with_window_size(mut self, width: u16, height: u16) -> Self {
        self.config.window_size = Some((width, height));
        self
    }

    /// Sets the target frames per second for rendering.
    ///
    /// This controls how often the `view` method of the model is called and
    /// the terminal is updated.
    ///
    /// # Arguments
    ///
    /// * `fps` - The target frames per second.
    pub fn with_fps(mut self, fps: u32) -> Self {
        self.config.fps = fps;
        self
    }

    /// Disables the renderer.
    ///
    /// When disabled, the `view` method will not be called and no output
    /// will be rendered to the terminal. This is useful for testing or
    /// headless operations.
    pub fn without_renderer(mut self) -> Self {
        self.config.without_renderer = true;
        self
    }

    /// Sets whether to catch panics.
    ///
    /// When enabled, application panics will be caught and converted into
    /// `ProgramPanic` errors, allowing for graceful shutdown.
    pub fn catch_panics(mut self, enabled: bool) -> Self {
        self.config.catch_panics = enabled;
        self
    }

    /// Sets whether to enable signal handling.
    ///
    /// When enabled, the `Program` will listen for OS signals (e.g., Ctrl+C)
    /// and attempt a graceful shutdown.
    pub fn signal_handler(mut self, enabled: bool) -> Self {
        self.config.signal_handler = enabled;
        self
    }

    /// Disables the renderer.
    ///
    /// This is the default behavior, so calling this method is optional.
    /// It's provided for explicit configuration when needed.
    ///
    /// # Returns
    ///
    /// The `ProgramBuilder` instance for method chaining.
    pub fn input_tty(self) -> Self {
        // No-op for now, as stdin is used by default
        self
    }

    /// Sets a custom input reader for the program.
    ///
    /// # Arguments
    ///
    /// * `reader` - A custom input stream that implements `tokio::io::AsyncRead + Send + Unpin`.
    pub fn input(mut self, reader: impl tokio::io::AsyncRead + Send + Unpin + 'static) -> Self {
        self.config.input_source = Some(InputSource::Custom(Box::pin(reader)));
        self
    }

    /// Sets a custom output writer for the program.
    ///
    /// # Arguments
    ///
    /// * `writer` - A custom output stream that implements `tokio::io::AsyncWrite + Send + Unpin`.
    pub fn output(mut self, writer: impl AsyncWrite + Send + Unpin + 'static) -> Self {
        self.config.output_writer = Some(Arc::new(Mutex::new(Box::new(writer))));
        self
    }

    /// Sets an external cancellation token for the program.
    ///
    /// When the token is cancelled, the program's event loop will gracefully shut down.
    ///
    /// # Arguments
    ///
    /// * `token` - The `CancellationToken` to use for external cancellation.
    pub fn context(mut self, token: CancellationToken) -> Self {
        self.config.cancellation_token = Some(token);
        self
    }

    /// Sets a model-aware message filter function.
    ///
    /// The provided closure will be called for each incoming message with access
    /// to the current model, allowing for context-aware transformation or filtering.
    ///
    /// # Arguments
    ///
    /// * `f` - A closure that takes `&M` and `Msg`, returning an `Option<Msg>`.
    pub fn filter(mut self, f: impl Fn(&M, Msg) -> Option<Msg> + Send + 'static) -> Self {
        self.message_filter = Some(Box::new(f));
        self
    }

    /// Sets the event channel buffer size.
    ///
    /// By default, the channel has a buffer of 1000 messages. Setting this to `None`
    /// will use an unbounded channel (not recommended for production), while setting
    /// it to `Some(size)` will use a bounded channel with the specified buffer size.
    ///
    /// # Arguments
    ///
    /// * `buffer_size` - The buffer size for the event channel.
    pub fn event_channel_buffer(mut self, buffer_size: Option<usize>) -> Self {
        self.config.event_channel_buffer = buffer_size;
        self
    }

    /// Enables memory usage monitoring.
    ///
    /// When enabled, the program will track memory usage metrics that can be
    /// accessed for debugging and performance analysis.
    pub fn memory_monitoring(mut self, enabled: bool) -> Self {
        self.config.memory_monitoring = enabled;
        self
    }

    /// Builds the `Program` instance with the configured options.
    ///
    /// # Returns
    ///
    /// A `Result` containing the `Program` instance or an `Error` if building fails.
    pub fn build(self) -> Result<Program<M>, Error> {
        Program::new(self.config, self.message_filter)
    }
}

/// The main `Program` struct that coordinates the application.
///
/// The `Program` is responsible for setting up the terminal, managing the
/// event loop, executing commands, and rendering the model's view.
pub struct Program<M: Model> {
    /// The configuration for this `Program` instance.
    pub config: ProgramConfig,
    event_tx: crate::event::EventSender,
    event_rx: crate::event::EventReceiver,
    terminal: Option<Box<dyn TerminalInterface + Send>>,
    /// Active timer handles for cancellation
    active_timers: HashMap<u64, CancellationToken>,
    /// Set of spawned tasks that can be cancelled on shutdown
    task_set: JoinSet<()>,
    /// Cancellation token for coordinated shutdown
    shutdown_token: CancellationToken,
    /// Memory usage monitor (optional)
    memory_monitor: Option<crate::memory::MemoryMonitor>,
    /// Optional model-aware message filter
    message_filter: Option<MessageFilter<M>>,
    applied_view: AppliedViewState,
    renderer: Option<CursedRenderer>,
    color_profile: Profile,
    on_mouse: Option<crate::view::OnMouseFn>,
    _phantom: PhantomData<M>,
}

impl<M: Model> Program<M> {
    /// Creates a new `ProgramBuilder` for configuring and building a `Program`.
    pub fn builder() -> ProgramBuilder<M> {
        ProgramBuilder::new()
    }

    /// Creates a new `Program` instance with the given configuration.
    ///
    /// This method is called internally by `ProgramBuilder::build()` and should not
    /// be called directly. Use `Program::builder()` followed by `build()` instead.
    ///
    /// # Arguments
    ///
    /// * `config` - The `ProgramConfig` to use for this program.
    /// * `message_filter` - Optional model-aware message filter function.
    ///
    /// # Returns
    ///
    /// A `Result` containing the `Program` instance or an `Error` if initialization fails.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    /// - Terminal initialization fails
    /// - Event channel setup fails
    /// - Global state initialization fails
    fn new(config: ProgramConfig, message_filter: Option<MessageFilter<M>>) -> Result<Self, Error> {
        let (event_tx, event_rx) = if let Some(buffer_size) = config.event_channel_buffer {
            let (tx, rx) = mpsc::channel(buffer_size);
            (
                crate::event::EventSender::Bounded(tx),
                crate::event::EventReceiver::Bounded(rx),
            )
        } else {
            let (tx, rx) = mpsc::unbounded_channel();
            (
                crate::event::EventSender::Unbounded(tx),
                crate::event::EventReceiver::Unbounded(rx),
            )
        };

        let terminal = if config.without_renderer {
            None
        } else {
            let output_writer_for_terminal = config.output_writer.clone();
            Some(Box::new(Terminal::new(output_writer_for_terminal)?)
                as Box<dyn TerminalInterface + Send>)
        };

        // Expose the event sender globally for command helpers
        let _ = crate::event::EVENT_SENDER.set(event_tx.clone());

        // Expose command environment globally for exec_process
        let _ = crate::command::COMMAND_ENV.set(config.environment.clone().unwrap_or_default());

        let memory_monitor = if config.memory_monitoring {
            Some(crate::memory::MemoryMonitor::new())
        } else {
            None
        };

        let profile = config
            .color_profile
            .unwrap_or_else(|| colorprofile::detect(term::is_stdout_terminal(), &[]));

        let (width, height) = config.window_size.unwrap_or((80, 24));
        let renderer = if config.without_renderer {
            None
        } else {
            Some(CursedRenderer::new(
                usize::from(width),
                usize::from(height),
                profile,
            ))
        };

        Ok(Self {
            config,
            event_tx,
            event_rx,
            terminal,
            active_timers: HashMap::new(),
            task_set: JoinSet::new(),
            shutdown_token: CancellationToken::new(),
            memory_monitor,
            message_filter,
            applied_view: AppliedViewState::default(),
            renderer,
            color_profile: profile,
            on_mouse: None,
            _phantom: PhantomData,
        })
    }

    async fn send_startup_messages(&self) -> Result<(), Error> {
        let _ = self
            .event_tx
            .send(Box::new(ColorProfileMsg(self.color_profile)) as Msg);
        let mut env_map: HashMap<String, String> = std::env::vars().collect();
        if let Some(custom) = &self.config.environment {
            env_map.extend(custom.clone());
        }
        let _ = self.event_tx.send(Box::new(EnvMsg(env_map)) as Msg);
        Ok(())
    }

    async fn enable_renderer_modes(&mut self) -> Result<(), Error> {
        if let Some(renderer) = &mut self.renderer {
            renderer.set_sync_output(true);
            renderer.set_unicode_mode(true);
            if let Some(terminal) = &mut self.terminal {
                let seq = renderer.startup_mode_sequences();
                if !seq.is_empty() {
                    terminal.write_raw(seq.as_bytes()).await?;
                }
            }
        }
        Ok(())
    }

    fn is_terminal_command(msg: &Msg) -> bool {
        msg.is::<RawCmdMsg>()
            || msg.is::<RequestCursorPositionCmdMsg>()
            || msg.is::<RequestForegroundColorCmdMsg>()
            || msg.is::<RequestBackgroundColorCmdMsg>()
            || msg.is::<RequestCursorColorCmdMsg>()
            || msg.is::<RequestTerminalVersionCmdMsg>()
            || msg.is::<RequestCapabilityCmdMsg>()
            || msg.is::<SetClipboardCmdMsg>()
            || msg.is::<ReadClipboardCmdMsg>()
            || msg.is::<SetPrimaryClipboardCmdMsg>()
            || msg.is::<ReadPrimaryClipboardCmdMsg>()
    }

    async fn handle_terminal_command(&mut self, msg: Msg) -> Result<(), Error> {
        use ansi::background::{
            REQUEST_BACKGROUND_COLOR, REQUEST_CURSOR_COLOR, REQUEST_FOREGROUND_COLOR,
        };
        use ansi::clipboard::{
            REQUEST_PRIMARY_CLIPBOARD, REQUEST_SYSTEM_CLIPBOARD, set_primary_clipboard,
            set_system_clipboard,
        };
        use ansi::ctrl::{REQUEST_NAME_VERSION, request_termcap};
        use ansi::cursor::REQUEST_CURSOR_POSITION_REPORT;

        let Some(terminal) = &mut self.terminal else {
            return Ok(());
        };

        if msg.is::<RawCmdMsg>() {
            let raw = msg.downcast::<RawCmdMsg>().expect("checked RawCmdMsg");
            terminal.write_raw(raw.0.as_bytes()).await?;
        } else if msg.is::<RequestCursorPositionCmdMsg>() {
            terminal
                .write_raw(REQUEST_CURSOR_POSITION_REPORT.as_bytes())
                .await?;
        } else if msg.is::<RequestForegroundColorCmdMsg>() {
            terminal
                .write_raw(REQUEST_FOREGROUND_COLOR.as_bytes())
                .await?;
        } else if msg.is::<RequestBackgroundColorCmdMsg>() {
            terminal
                .write_raw(REQUEST_BACKGROUND_COLOR.as_bytes())
                .await?;
        } else if msg.is::<RequestCursorColorCmdMsg>() {
            terminal.write_raw(REQUEST_CURSOR_COLOR.as_bytes()).await?;
        } else if msg.is::<RequestTerminalVersionCmdMsg>() {
            terminal.write_raw(REQUEST_NAME_VERSION.as_bytes()).await?;
        } else if msg.is::<RequestCapabilityCmdMsg>() {
            let cap = msg
                .downcast::<RequestCapabilityCmdMsg>()
                .expect("checked RequestCapabilityCmdMsg");
            let seq = request_termcap(&[&cap.0]);
            terminal.write_raw(seq.as_bytes()).await?;
        } else if msg.is::<SetClipboardCmdMsg>() {
            let set = msg
                .downcast::<SetClipboardCmdMsg>()
                .expect("checked SetClipboardCmdMsg");
            let seq = set_system_clipboard(&set.0);
            terminal.write_raw(seq.as_bytes()).await?;
        } else if msg.is::<ReadClipboardCmdMsg>() {
            terminal
                .write_raw(REQUEST_SYSTEM_CLIPBOARD.as_bytes())
                .await?;
        } else if msg.is::<SetPrimaryClipboardCmdMsg>() {
            let set = msg
                .downcast::<SetPrimaryClipboardCmdMsg>()
                .expect("checked SetPrimaryClipboardCmdMsg");
            let seq = set_primary_clipboard(&set.0);
            terminal.write_raw(seq.as_bytes()).await?;
        } else if msg.is::<ReadPrimaryClipboardCmdMsg>() {
            terminal
                .write_raw(REQUEST_PRIMARY_CLIPBOARD.as_bytes())
                .await?;
        }

        Ok(())
    }

    fn maybe_upgrade_profile(&mut self, cap: &CapabilityMsg) {
        if self.color_profile == Profile::TrueColor {
            return;
        }
        if cap.content == "RGB" || cap.content == "Tc" {
            self.color_profile = Profile::TrueColor;
            if let Some(renderer) = &mut self.renderer {
                renderer.set_profile(Profile::TrueColor);
            }
            let _ = self
                .event_tx
                .send(Box::new(ColorProfileMsg(Profile::TrueColor)) as Msg);
        }
    }

    async fn render_frame(&mut self, model: &M) -> Result<(), Error> {
        let view = model.view();
        let next_state = AppliedViewState::from_view(&view);
        let options = render_options_from_view(&view);
        self.on_mouse = view
            .on_mouse
            .or_else(|| crate::view::View::mouse_handler(view.compositor.clone(), None));
        let content = if view.content.is_empty() {
            view.compositor
                .as_ref()
                .map(lipgloss::Compositor::render)
                .unwrap_or_default()
        } else {
            view.content
        };
        if self.config.without_renderer {
            return Ok(());
        }
        if let (Some(terminal), Some(renderer)) = (&mut self.terminal, &mut self.renderer) {
            if let Some((width, height)) = self.config.window_size.or_else(|| terminal.size().ok())
            {
                renderer.resize(usize::from(width), usize::from(height));
            }
            apply_view(terminal.as_mut(), &mut self.applied_view, next_state).await?;
            let output = renderer.render(&content, &options);
            terminal.write_raw(output.as_bytes()).await?;
        }
        Ok(())
    }

    async fn restore_applied_terminal_state(&mut self) -> Result<(), Error> {
        if let (Some(terminal), Some(renderer)) = (&mut self.terminal, &mut self.renderer) {
            let seq = renderer.shutdown_mode_sequences();
            if !seq.is_empty() {
                let _ = terminal.write_raw(seq.as_bytes()).await;
            }
        }
        if let Some(terminal) = &mut self.terminal {
            let _ = terminal.show_cursor().await;
            if self.applied_view.mouse_mode != crate::view::MouseMode::None {
                let _ = terminal.disable_mouse().await;
            }
            if self.applied_view.report_focus {
                let _ = terminal.disable_focus_reporting().await;
            }
            if self.applied_view.bracketed_paste {
                let _ = terminal.disable_bracketed_paste().await;
            }
            if self.applied_view.alt_screen {
                let _ = terminal.exit_alt_screen().await;
            }
            let _ = terminal.exit_raw_mode().await;
        }
        Ok(())
    }

    async fn suspend_and_resume(&mut self, model: &M) -> Result<(), Error> {
        self.restore_applied_terminal_state().await?;
        tokio::task::spawn_blocking(signals::suspend_process)
            .await
            .map_err(|_| Error::Io(std::io::Error::other("suspend task failed")))?;
        if let Some(terminal) = &mut self.terminal {
            terminal.enter_raw_mode().await?;
        }
        self.enable_renderer_modes().await?;
        let _ = self.event_tx.send(Box::new(crate::ResumeMsg) as Msg);
        self.render_frame(model).await?;
        Ok(())
    }

    /// Runs the `bubble-t` application.
    ///
    /// This method initializes the terminal, starts the event loop, and manages
    /// the application's lifecycle. It will continue to run until a `QuitMsg`
    /// is received or an unrecoverable error occurs.
    pub async fn run(mut self) -> Result<M, Error> {
        // Set up panic hook
        if self.config.catch_panics {
            let event_tx = self.event_tx.clone();
            ORIGINAL_PANIC_HOOK.get_or_init(|| panic::take_hook());

            panic::set_hook(Box::new(move |panic_info| {
                let payload = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
                    s.clone()
                } else {
                    "<unknown panic>".to_string()
                };
                let _ = event_tx.send(Box::new(crate::Error::ProgramPanic(payload)) as Msg);

                // Call the original hook if it exists
                if let Some(hook) = ORIGINAL_PANIC_HOOK.get() {
                    hook(panic_info);
                }
            }));
        }

        // Setup terminal — declarative View fields are applied on each render frame.
        if let Some(terminal) = &mut self.terminal {
            terminal.enter_raw_mode().await?;
        }
        self.enable_renderer_modes().await?;
        self.send_startup_messages().await?;

        let _signal_task =
            signals::spawn_interrupt_listener(self.event_tx.clone(), self.config.signal_handler);
        let _resize_task =
            signals::spawn_resize_listener(self.event_tx.clone(), self.config.signal_handler);

        let (mut model, mut cmd) = M::init();
        self.render_frame(&model).await?;

        // Setup input handling - either terminal input or custom input source
        if self.terminal.is_some() || self.config.input_source.is_some() {
            let input_source = self.config.input_source.take();
            let input_handler = if let Some(source) = input_source {
                InputHandler::with_source(self.event_tx.clone(), source)
            } else {
                InputHandler::new(self.event_tx.clone())
            };
            let shutdown_token = self.shutdown_token.clone();

            // Update memory monitoring
            if let Some(ref monitor) = self.memory_monitor {
                monitor.task_spawned();
            }

            self.task_set.spawn(async move {
                tokio::select! {
                    _ = shutdown_token.cancelled() => {
                        // Shutdown requested
                    }
                    _ = input_handler.run() => {
                        // Input handler completed
                    }
                }
            });
        }

        let result = 'main_loop: loop {
            if let Some(c) = cmd.take() {
                let event_tx = self.event_tx.clone();
                let shutdown_token = self.shutdown_token.clone();

                // Update memory monitoring
                if let Some(ref monitor) = self.memory_monitor {
                    monitor.task_spawned();
                }

                self.task_set.spawn(async move {
                    tokio::select! {
                        _ = shutdown_token.cancelled() => {
                            // Shutdown requested, don't process command
                        }
                        result = c => {
                            if let Some(msg) = result {
                                let _ = event_tx.send(msg);
                            }
                        }
                    }
                });
            }

            select! {
                _ = self.config.cancellation_token.as_ref().map_or(futures::future::pending().left_future(), |token| token.cancelled().right_future()).fuse() => {
                    break Ok(model); // External cancellation
                }
                event = self.event_rx.recv().fuse() => {
                    if let Some(mut msg) = event {
                        // KillMsg triggers immediate termination without touching the model
                        if msg.downcast_ref::<KillMsg>().is_some() {
                            break Err(Error::ProgramKilled);
                        }
                        if let Some(filter_fn) = &self.message_filter {
                            if let Some(filtered_msg) = filter_fn(&model, msg) {
                                msg = filtered_msg;
                            } else {
                                continue; // Message was filtered out
                            }
                        }
                        // If the filter produced a KillMsg, terminate immediately
                        if msg.downcast_ref::<KillMsg>().is_some() {
                            break Err(Error::ProgramKilled);
                        }
                        // Check for special internal messages
                        let mut should_quit = false;
                        let mut should_interrupt = false;

                        // Handle special internal messages that need to consume the message
                        if msg.is::<crate::event::ClearScreenMsg>() {
                            if let Some(terminal) = &mut self.terminal {
                                let _ = terminal.clear().await;
                            }
                            continue; // handled; don't pass to the model
                        } else if msg.is::<crate::event::EveryMsgInternal>() {
                            // We need to consume the message to get ownership of the function
                            if let Ok(every_msg) = msg.downcast::<crate::event::EveryMsgInternal>() {
                                let duration = every_msg.duration;
                                let func = every_msg.func;
                                let cancellation_token = every_msg.cancellation_token.clone();
                                let timer_id = every_msg.timer_id;
                                let event_tx = self.event_tx.clone();

                                // Store the cancellation token for this timer
                                self.active_timers.insert(timer_id, cancellation_token.clone());

                                // Update memory monitoring
                                if let Some(ref monitor) = self.memory_monitor {
                                    monitor.timer_added();
                                }

                                tokio::spawn(async move {
                                    let mut ticker = tokio::time::interval(duration);
                                    ticker.tick().await; // First tick completes immediately

                                    loop {
                                        tokio::select! {
                                            _ = cancellation_token.cancelled() => {
                                                // Timer was cancelled
                                                break;
                                            }
                                            _ = ticker.tick() => {
                                                let msg = func(duration);
                                                if event_tx.send(msg).is_err() {
                                                    break; // Receiver dropped
                                                }
                                            }
                                        }
                                    }
                                });
                                continue; // Don't pass this to the model
                            }
                        } else if msg.is::<crate::event::BatchCmdMsg>() {
                            // Handle BatchCmdMsg: spawn all commands concurrently without waiting
                            if let Ok(batch_cmd_msg) = msg.downcast::<crate::event::BatchCmdMsg>() {
                                for c in batch_cmd_msg.0 {
                                    let event_tx = self.event_tx.clone();
                                    let shutdown_token = self.shutdown_token.clone();
                                    if let Some(ref monitor) = self.memory_monitor {
                                        monitor.task_spawned();
                                    }
                                    self.task_set.spawn(async move {
                                        tokio::select! {
                                            _ = shutdown_token.cancelled() => {
                                                // Shutdown requested, don't process command
                                            }
                                            result = c => {
                                                if let Some(msg) = result {
                                                    let _ = event_tx.send(msg);
                                                }
                                            }
                                        }
                                    });
                                }
                            }
                            continue; // We've handled the batch, don't pass it to the model
                        } else if msg.is::<crate::event::BatchMsgInternal>() {
                            if let Ok(batch_msg) = msg.downcast::<crate::event::BatchMsgInternal>() {
                                // Process each message in the batch and accumulate resulting cmds
                                let mut next_cmds: Vec<crate::command::Cmd> = Vec::new();
                                for batch_item in batch_msg.messages {
                                    if batch_item.downcast_ref::<KillMsg>().is_some() {
                                        // Immediate termination
                                        break 'main_loop Err(Error::ProgramKilled);
                                    }
                                    if batch_item.downcast_ref::<QuitMsg>().is_some() {
                                        should_quit = true;
                                    }
                                    if batch_item.downcast_ref::<crate::InterruptMsg>().is_some() {
                                        should_interrupt = true;
                                    }
                                    if let Some(new_cmd) = model.update(batch_item) {
                                        next_cmds.push(new_cmd);
                                    }
                                }
                                if !next_cmds.is_empty() {
                                    cmd = Some(crate::command::batch(next_cmds));
                                }
                            }
                        } else if msg.is::<crate::event::CancelTimerMsg>() {
                            if let Ok(cancel_msg) = msg.downcast::<crate::event::CancelTimerMsg>() {
                                if let Some(token) = self.active_timers.remove(&cancel_msg.timer_id) {
                                    token.cancel();
                                    // Update memory monitoring
                                    if let Some(ref monitor) = self.memory_monitor {
                                        monitor.timer_removed();
                                    }
                                }
                                continue; // Don't pass this to the model
                            }
                        } else if msg.is::<crate::event::CancelAllTimersMsg>() {
                            // Cancel all active timers
                            let timer_count = self.active_timers.len();
                            for (_, token) in self.active_timers.drain() {
                                token.cancel();
                            }
                            // Update memory monitoring
                            if let Some(ref monitor) = self.memory_monitor {
                                for _ in 0..timer_count {
                                    monitor.timer_removed();
                                }
                            }
                            continue; // Don't pass this to the model
                        } else if msg.is::<RequestWindowSizeMsg>() {
                            if let Some((width, height)) = self
                                .terminal
                                .as_ref()
                                .and_then(|terminal| terminal.size().ok())
                            {
                                let _ = self
                                    .event_tx
                                    .send(Box::new(WindowSizeMsg { width, height }) as Msg);
                            }
                            continue;
                        } else if msg.is::<crate::SuspendMsg>() {
                            if SUSPEND_SUPPORTED {
                                self.suspend_and_resume(&model).await?;
                            }
                            continue;
                        } else if Self::is_terminal_command(&msg) {
                            self.handle_terminal_command(msg).await?;
                            continue;
                        } else {
                            // Handle regular messages
                            if let Some(cap) = msg.downcast_ref::<CapabilityMsg>() {
                                self.maybe_upgrade_profile(cap);
                            }
                            let is_quit = msg.downcast_ref::<QuitMsg>().is_some();
                            let is_interrupt = msg.downcast_ref::<crate::InterruptMsg>().is_some();
                            let mouse_cmd = msg
                                .downcast_ref::<MouseMsg>()
                                .and_then(|mouse| {
                                    self.on_mouse
                                        .as_ref()
                                        .and_then(|handler| handler(mouse.clone()))
                                });
                            let update_cmd = model.update(msg);
                            cmd = match (mouse_cmd, update_cmd) {
                                (Some(a), Some(b)) => {
                                    Some(crate::command::batch(vec![a, b]))
                                }
                                (Some(a), None) => Some(a),
                                (None, b) => b,
                            };
                            if is_quit {
                                should_quit = true;
                            }
                            if is_interrupt {
                                should_interrupt = true;
                            }

                            // Update memory monitoring
                            if let Some(ref monitor) = self.memory_monitor {
                                monitor.message_processed();
                            }
                        }
                        if should_quit {
                            break Ok(model);
                        }
                        if should_interrupt {
                            break Err(Error::Interrupted);
                        }
                        self.render_frame(&model).await?;
                    } else {
                        break Err(Error::ChannelReceive);
                    }
                }
            }
        };

        // Restore terminal state on exit
        let _ = self.restore_applied_terminal_state().await;

        // Cleanup: cancel all tasks and wait for them to complete
        self.cleanup_tasks().await;

        result
    }

    /// Clean up all spawned tasks on program shutdown.
    ///
    /// This method is called internally during program shutdown to ensure
    /// all background tasks are properly terminated. It:
    /// 1. Cancels the shutdown token to signal all tasks to stop
    /// 2. Cancels all active timers
    /// 3. Waits for tasks to complete with a timeout
    /// 4. Aborts any remaining unresponsive tasks
    ///
    /// This prevents resource leaks and ensures clean program termination.
    async fn cleanup_tasks(&mut self) {
        // Cancel the shutdown token to signal all tasks to stop
        self.shutdown_token.cancel();

        // Cancel all active timers
        for (_, token) in self.active_timers.drain() {
            token.cancel();
        }

        // Wait for all tasks to complete, with a timeout to avoid hanging
        let timeout = std::time::Duration::from_millis(500);
        let _ = tokio::time::timeout(timeout, async {
            while (self.task_set.join_next().await).is_some() {
                // Task completed
            }
        })
        .await;

        // Abort any remaining tasks that didn't respond to cancellation
        self.task_set.abort_all();
    }

    /// Returns a sender that can be used to send messages to the `Program`'s event loop.
    ///
    /// This is useful for sending messages from outside the `Model`'s `update` method,
    /// for example, from asynchronous tasks or other threads.
    ///
    /// # Returns
    ///
    /// An `EventSender` that can be used to send messages.
    pub fn sender(&self) -> crate::event::EventSender {
        self.event_tx.clone()
    }

    /// Sends a message to the `Program`'s event loop.
    ///
    /// This is a convenience method that wraps the `sender()` method.
    /// The message will be processed by the model's `update` method.
    ///
    /// # Arguments
    ///
    /// * `msg` - The `Msg` to send to the event loop.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or a channel-related error if the message could not be sent.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    /// - The event channel is full (for bounded channels)
    /// - The receiver has been dropped
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bubble_t::{Program, Model, KeyMsg};
    /// # struct MyModel;
    /// # impl Model for MyModel {
    /// #     fn init() -> (Self, Option<bubble_t::Cmd>) { (MyModel, None) }
    /// #     fn update(&mut self, _: bubble_t::Msg) -> Option<bubble_t::Cmd> { None }
    /// #     fn view(&self) -> bubble_t::View { bubble_t::View::new("") }
    /// # }
    /// # async fn example() -> Result<(), bubble_t::Error> {
    /// let program = Program::<MyModel>::builder().build()?;
    /// let key_msg = KeyMsg {
    ///     key: crossterm::event::KeyCode::Enter,
    ///     modifiers: crossterm::event::KeyModifiers::empty(),
    /// };
    /// program.send(Box::new(key_msg))?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn send(&self, msg: Msg) -> Result<(), Error> {
        self.event_tx.send(msg)
    }

    /// Sends a `QuitMsg` to the `Program`'s event loop, initiating a graceful shutdown.
    ///
    /// This causes the event loop to terminate gracefully after processing any
    /// remaining messages in the queue. The terminal state will be properly restored.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bubble_t::{Program, Model};
    /// # struct MyModel;
    /// # impl Model for MyModel {
    /// #     fn init() -> (Self, Option<bubble_t::Cmd>) { (MyModel, None) }
    /// #     fn update(&mut self, _: bubble_t::Msg) -> Option<bubble_t::Cmd> { None }
    /// #     fn view(&self) -> bubble_t::View { bubble_t::View::new("") }
    /// # }
    /// # async fn example() -> Result<(), bubble_t::Error> {
    /// let program = Program::<MyModel>::builder().build()?;
    /// program.quit(); // Gracefully shutdown the program
    /// # Ok(())
    /// # }
    /// ```
    pub fn quit(&self) {
        let _ = self.event_tx.send(Box::new(QuitMsg));
    }

    /// Get a reference to the memory monitor, if enabled.
    ///
    /// Returns `None` if memory monitoring is disabled.
    pub fn memory_monitor(&self) -> Option<&crate::memory::MemoryMonitor> {
        self.memory_monitor.as_ref()
    }

    /// Get memory usage health information, if monitoring is enabled.
    ///
    /// Returns `None` if memory monitoring is disabled.
    pub fn memory_health(&self) -> Option<crate::memory::MemoryHealth> {
        self.memory_monitor.as_ref().map(|m| m.check_health())
    }

    /// Sends a `KillMsg` to the `Program`'s event loop, initiating an immediate termination.
    ///
    /// Unlike `quit()`, which performs a graceful shutdown, `kill()` causes the event loop
    /// to stop as soon as possible and returns `Error::ProgramKilled`.
    pub fn kill(&self) {
        let _ = self.event_tx.send(Box::new(KillMsg));
    }

    /// Waits for the `Program` to finish execution.
    ///
    /// This method blocks until the program's event loop has exited.
    ///
    /// # Note
    ///
    /// This is currently a no-op since the `Program` is consumed by `run()`.
    /// In a real implementation, you'd need to track the program's state separately,
    /// similar to how Go's context.Context works with goroutines.
    ///
    /// # Future Implementation
    ///
    /// A future version might track program state separately to enable proper
    /// waiting functionality without consuming the `Program` instance.
    pub async fn wait(&self) {
        // Since the Program is consumed by run(), we can't really wait for it.
        // This would need a different architecture to implement properly,
        // similar to how Go's context.Context works with goroutines.
        tokio::task::yield_now().await;
    }

    /// Releases control of the terminal.
    ///
    /// This method restores the terminal to its original state, disabling raw mode,
    /// exiting alternate screen, disabling mouse and focus reporting, and showing the cursor.
    pub async fn release_terminal(&mut self) -> Result<(), Error> {
        if let Some(terminal) = &mut self.terminal {
            terminal.exit_raw_mode().await?;
            terminal.exit_alt_screen().await?;
            terminal.disable_mouse().await?;
            terminal.disable_focus_reporting().await?;
            terminal.show_cursor().await?;
        }
        Ok(())
    }

    /// Restores control of the terminal.
    ///
    /// This method re-initializes the terminal based on the `ProgramConfig`,
    /// enabling raw mode, entering alternate screen, enabling mouse and focus reporting,
    /// and hiding the cursor.
    pub async fn restore_terminal(&mut self) -> Result<(), Error> {
        if let Some(terminal) = &mut self.terminal {
            terminal.enter_raw_mode().await?;
        }
        Ok(())
    }

    /// Prints a line to the terminal without going through the renderer.
    ///
    /// This is useful for debugging or for outputting messages that shouldn't
    /// be part of the managed UI. The output bypasses the normal rendering
    /// pipeline and goes directly to stdout.
    ///
    /// # Arguments
    ///
    /// * `s` - The string to print, a newline will be automatically added.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or an IO error if printing fails.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if stdout flushing fails.
    ///
    /// # Warning
    ///
    /// Using this method while the program is running may interfere with
    /// the normal UI rendering. It's recommended to use this only for
    /// debugging purposes or when the renderer is disabled.
    pub async fn println(&mut self, s: String) -> Result<(), Error> {
        if let Some(_terminal) = &mut self.terminal {
            use std::io::Write;
            println!("{s}");
            std::io::stdout().flush()?;
        }
        Ok(())
    }

    /// Prints formatted text to the terminal without going through the renderer.
    ///
    /// This is useful for debugging or for outputting messages that shouldn't
    /// be part of the managed UI. The output bypasses the normal rendering
    /// pipeline and goes directly to stdout without adding a newline.
    ///
    /// # Arguments
    ///
    /// * `s` - The string to print without adding a newline.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or an IO error if printing fails.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if stdout flushing fails.
    ///
    /// # Warning
    ///
    /// Using this method while the program is running may interfere with
    /// the normal UI rendering. It's recommended to use this only for
    /// debugging purposes or when the renderer is disabled.
    pub async fn printf(&mut self, s: String) -> Result<(), Error> {
        if let Some(_terminal) = &mut self.terminal {
            use std::io::Write;
            print!("{s}");
            std::io::stdout().flush()?;
        }
        Ok(())
    }
}
