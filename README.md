# bubble-t

[![CI](https://github.com/Nexlab-One/Bubble-T/actions/workflows/ci.yml/badge.svg)](https://github.com/Nexlab-One/Bubble-T/actions/workflows/ci.yml)

A Rust reimagining of the [Bubble Tea](https://github.com/charmbracelet/bubbletea) TUI framework — inspired by, and paying homage to, the original Go project from Charmbracelet.

Build terminal user interfaces with the Model-View-Update pattern, async commands, and rich styling.

> **Status:** Active development. Core APIs are stabilizing under the `bubble-t` crate (formerly `bubbletea-rs` / `bubble-t`).

## Monorepo layout

This repository is a single Cargo workspace containing the full Rust Bubble Tea ecosystem:

| Crate | Path | Purpose |
|-------|------|---------|
| **bubble-t** | `crates/bubble-t` | Core MVU framework with async runtime |
| **bubble-t-widgets** | `crates/bubble-t-widgets` | Pre-built UI components (spinners, inputs, tables, etc.) |
| **lipgloss** | `crates/lipgloss` | Terminal styling (colors, borders, layouts) |
| **lipgloss-list / -table / -tree** | `crates/lipgloss-*` | Styled list, table, and tree renderers |
| **lipgloss-extras** | `crates/lipgloss-extras` | Feature-gated facade over lipgloss components |

## Quick start

From crates.io (when published):

```toml
[dependencies]
bubble-t = "0.1.12"
bubble-t-widgets = "0.1.12"
lipgloss-extras = { version = "0.1.12", features = ["full"] }
tokio = { version = "1", features = ["full"] }
```

From this monorepo (path dependencies):

```toml
[dependencies]
bubble-t = { path = "../crates/bubble-t" }
bubble-t-widgets = { path = "../crates/bubble-t-widgets" }
lipgloss-extras = { path = "../crates/lipgloss-extras", features = ["full"] }
tokio = { version = "1", features = ["full"] }
```

Minimal application:

```rust
use bubble_t::{Model, Msg, Cmd, Program};

struct App { count: i32 }

impl Model for App {
    fn init() -> (Self, Option<Cmd>) {
        (Self { count: 0 }, None)
    }
    fn update(&mut self, _msg: Msg) -> Option<Cmd> { None }
    fn view(&self) -> String { format!("count: {}", self.count) }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    Program::<App>::builder().build()?.run().await?;
    Ok(())
}
```

## Examples

Forty-plus examples mirror the upstream Go Bubble Tea gallery. See [examples/README.md](examples/README.md).

```bash
cd examples/simple
cargo run
```

Or from the workspace root:

```bash
cargo run -p simple-example
```

## Development

Requires Rust **1.92+** (edition 2024). The repo pins stable via `rust-toolchain.toml`.

```bash
cargo test --workspace --all-features
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo doc --no-deps --all-features
```

## Documentation

- [bubble-t API](docs/API-BUBBLETEA-RS.md) — core MVU framework
- [bubble-t-widgets API](docs/API-BUBBLES-RS.md) — UI components
- [lipgloss API](docs/API-LIPGLOSS.md) — styling and layout

## Inspiration and credits

- [Bubble Tea (Go)](https://github.com/charmbracelet/bubbletea) — original design and inspiration
- [Charm](https://charm.sh) — CLI design philosophy
- [Elm Architecture](https://guide.elm-lang.org/architecture/) — Model-View-Update pattern
- Original Rust ports by [whit3rabbit](https://github.com/whit3rabbit); fork maintained by [Nexlab-One](https://github.com/Nexlab-One)

## License

MIT — see [LICENSE](LICENSE).
