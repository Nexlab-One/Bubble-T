# bubble-t API Reference

Core MVU framework crate (`crates/bubble-t`). Concepts align with [Bubble Tea v2 (Go)](https://github.com/charmbracelet/bubbletea): a `Program` drives an event loop, your `Model` handles messages, and `Cmd` values perform async side effects.

## Architecture

```
Model (state)  ──update(msg)──►  optional Cmd
     ▲                                │
     │                                ▼
     └──────────── Msg ◄──── async / IO / timers

view() ──► View ──► CursedRenderer (cellbuf diff) ──► Terminal
```

Every message is a `Box<dyn Any + Send>` (`Msg`). Downcast with `msg.downcast_ref::<T>()`.

## Foundation crates

| Crate | Role |
|-------|------|
| `ansi` | Escape-sequence builders + incremental parser |
| `cellbuf` | Width-aware styled cell grid + screen diff |
| `colorprofile` | Profile detection + ANSI downsampling |
| `term` | Cross-platform raw mode / TTY / size |
| `lipgloss` | Styled string rendering (compositing via `Canvas`/`Compositor`) |

## Core traits and types

### `Model`

```rust
pub trait Model: Send + Sized + 'static {
    fn init() -> (Self, Option<Cmd>);
    fn update(&mut self, msg: Msg) -> Option<Cmd>;
    fn view(&self) -> View;
}
```

- **`init`** — construct initial state; return optional startup command.
- **`update`** — state transition; return `None` or a `Cmd`.
- **`view`** — declarative frame: content, cursor, terminal modes, title, progress bar, optional `on_mouse`.

### `View`

```rust
let mut view = View::new(content);
view.alt_screen = true;
view.mouse_mode = MouseMode::CellMotion;
view.cursor = Some(Cursor::new(Position::new(x, y)));
view.window_title = "My App".into();
view.on_mouse = Some(Box::new(|msg| { /* handle MouseMsg */ None }));
```

The runtime diffs `View` fields each frame and applies terminal changes (no imperative toggle commands).

### `Program` / `ProgramBuilder`

```rust
let program = Program::<MyModel>::builder()
    .with_color_profile(profile)
    .with_window_size(120, 40)
    .build()?;
program.run().await?;
```

Declarative terminal options come from `View` each frame. Builder options cover color profile, window size, environment, and I/O overrides.

### `Cmd`

Async side effects returning optional messages. Key constructors:

| Function | Purpose |
|----------|---------|
| `tick` / `every` | Timers |
| `batch` / `sequence` | Command composition |
| `quit` | Exit |
| `window_size` | Request terminal dimensions |
| `read_clipboard` / `set_clipboard` | OSC 52 clipboard |
| `request_terminal_version` / `request_capability` | Terminal queries |
| `request_foreground_color` / `request_background_color` | OSC 10–12 color queries |
| `exec_process` / `suspend` | Subprocess / shell suspend |

Imperative alt-screen/mouse/focus toggles are **removed** — set fields on `View` instead.

## Messages (`event` module)

v2 keyboard:

- `KeyPressMsg` / `KeyReleaseMsg` — `code`, `text`, `mod`, `shifted_code`, `base_code`, `is_repeat`
- `KeyMsg` — union wrapper for legacy matching via `legacy_key_msg()`

Mouse (v2 + legacy):

- `MouseClickMsg`, `MouseReleaseMsg`, `MouseWheelMsg`, `MouseMotionMsg`
- `MouseMsg` — legacy shape still emitted for compatibility; `View::on_mouse` receives this

Other:

- `PasteMsg`, `PasteStartMsg`, `PasteEndMsg`
- `WindowSizeMsg`, `ClipboardMsg`, `ColorProfileMsg`, `EnvMsg`
- `ForegroundColorMsg`, `BackgroundColorMsg`, `CursorColorMsg`
- `TerminalVersionMsg`, `CapabilityMsg`, `CursorPositionMsg`, `ModeReportMsg`
- `KeyboardEnhancementsMsg`, `FocusMsg`, `BlurMsg`, `QuitMsg`

## Renderer

`CursedRenderer` diffs styled content through `cellbuf` with `colorprofile` downsampling, synchronized output (mode 2026), and Unicode width mode (2027).

## Typical update loop

```rust
fn update(&mut self, msg: Msg) -> Option<Cmd> {
    if let Some(key) = msg.downcast_ref::<KeyPressMsg>() {
        if key.code == crossterm::event::KeyCode::Char('q') {
            return Some(quit());
        }
    }
    if let Some(size) = msg.downcast_ref::<WindowSizeMsg>() {
        self.width = size.width;
    }
    None
}

fn view(&self) -> View {
    View::new(self.render_content())
}
```

## Relation to Go Bubble Tea v2

| Go v2 | bubble-t |
|-------|----------|
| `tea.Model` | `Model` |
| `tea.View` | `View` |
| `tea.KeyPressMsg` | `KeyPressMsg` |
| `tea.Program` | `Program` |
| `tea.WithColorProfile` | `ProgramBuilder::with_color_profile` |
| OSC 52 clipboard | `set_clipboard` / `read_clipboard` |

The Rust port uses Tokio async/await instead of Go goroutines.
