# bubble-t API Reference

Core MVU framework crate (`crates/bubble-t`). Concepts align with [Bubble Tea (Go)](https://github.com/charmbracelet/bubbletea): a `Program` drives an event loop, your `Model` handles messages, and `Cmd` values perform async side effects.

## Architecture

```
Model (state)  ──update(msg)──►  optional Cmd
     ▲                                │
     │                                ▼
     └──────────── Msg ◄──── async / IO / timers
     
view() ──► String ──► Terminal
```

Every message is a `Box<dyn Any + Send + Sync>` (`Msg`). Downcast with `msg.downcast_ref::<T>()`.

## Core traits and types

### `Model`

```rust
pub trait Model: Send + 'static {
    fn init() -> (Self, Option<Cmd>) where Self: Sized;
    fn update(&mut self, msg: Msg) -> Option<Cmd>;
    fn view(&self) -> String;
}
```

- **`init`** — construct initial state; return optional startup command (timers, HTTP, etc.).
- **`update`** — pure state transition; return `None` or a `Cmd` for side effects.
- **`view`** — render current state as a string (styled via lipgloss in application code).

### `Program` / `ProgramBuilder`

```rust
let program = Program::<MyModel>::builder()
    .with_alt_screen(true)
    .build()?;
program.run().await?;
```

`Program` owns the terminal, input loop, and command executor. Use `ProgramConfig` for mouse, bracketed paste, and focus reporting.

### `Cmd`

Type alias: `Pin<Box<dyn Future<Output = Option<Msg>> + Send>>`.

Constructors (re-exported from `command`):

| Function | Purpose |
|----------|---------|
| `tick(duration, f)` | Periodic timer messages |
| `every(duration, f)` | Repeating interval |
| `batch(cmds)` | Run commands concurrently |
| `sequence(cmds)` | Run commands in order |
| `quit()` | Exit the program |
| `window_size()` | Request terminal dimensions |
| `set_window_title(title)` | Set terminal title |
| `exec_process(cmd)` | Spawn subprocess |
| `enable_mouse_*` / `disable_mouse` | Mouse capture |
| `enter_alt_screen` / `exit_alt_screen` | Alternate screen buffer |

## Messages (`event` module)

Common message types:

- `KeyMsg` — keyboard input (`key`, `modifiers`)
- `MouseMsg` — mouse events
- `WindowSizeMsg` — `{ width, height }`
- `PasteMsg` — bracketed paste content
- `QuitMsg`, `FocusMsg`, `BlurMsg`

Custom messages: `Box::new(MyMsg { ... }) as Msg`.

## Terminal abstraction

- `Terminal` trait — real terminal I/O
- `DummyTerminal` — testing without a TTY
- `TerminalInterface` — abstraction used by `Program`

## Gradient utilities

`gradient_filled_segment`, `charm_default_gradient`, `lerp_rgb` — used by progress bar examples for Charm-style color ramps.

## Memory monitoring

`MemoryMonitor`, `MemorySnapshot`, `MemoryHealth` — optional allocation tracking during development.

## Features

| Feature | Default | Description |
|---------|---------|-------------|
| `tokio-runtime` | yes | Async runtime (recommended) |
| `logging` | yes | File logging via `log` crate |
| `mouse-support` | no | Compile-time flag for mouse handlers |
| `testing` | no | Test helpers |

## Typical update loop pattern

```rust
fn update(&mut self, msg: Msg) -> Option<Cmd> {
    if let Some(key) = msg.downcast_ref::<KeyMsg>() {
        if key.key == KeyCode::Char('q') {
            return Some(quit());
        }
    }
    if let Some(size) = msg.downcast_ref::<WindowSizeMsg>() {
        self.width = size.width;
    }
    None
}
```

## Relation to Go Bubble Tea

| Go | bubble-t |
|----|---------|
| `tea.Model` | `Model` trait |
| `tea.Msg` | `Msg` |
| `tea.Cmd` | `Cmd` |
| `tea.Program` | `Program` |
| `tea.Batch` | `batch()` |
| `tea.Sequence` | `sequence()` |
| `tea.Quit` | `quit()` |

The Rust port uses async/await (`tokio`) instead of Go goroutines, but the message/command separation is preserved.
