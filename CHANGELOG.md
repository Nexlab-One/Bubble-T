# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0] - Unreleased (v2 parity)

### Added
- Foundation crates: `ansi`, `cellbuf`, `colorprofile`, `term`, `harmonica`
- Bubble Tea v2 `View` struct with declarative terminal options (alt screen, mouse, focus, title, cursor, progress bar, keyboard enhancements)
- v2 messages: `KeyPressMsg`/`KeyReleaseMsg`, split mouse messages, paste start/end, terminal query/clipboard/color messages
- `CursedRenderer` cell-diff renderer over `cellbuf` + `colorprofile`
- Lip Gloss v2 compositing (`Layer`, `Compositor`, `Canvas`, `blend_at` z-order blend)
- Lip Gloss `OutputContext` for profile-aware downsampling
- Kitty graphics shared-memory transmission (`t=s`) on POSIX and Windows, with direct-transmission fallback
- Examples: `capability`, `cursor-color`, `keyboard-enhancements`

### Fixed
- Restored v2 `program.rs` and `terminal.rs` after accidental v1 checkout regression (CursedRenderer, declarative View diffing, compositor auto-render, `on_mouse`/`LayerMouseMsg`, terminal queries, ColorProfileMsg/EnvMsg startup)
- Lip Gloss `OutputContext` naming fallout from bulk replace (`style/render.rs`, `style/transform.rs`, tests)
- `lipgloss-tree` golden tests: parallel-safe profile handling and CRLF normalization on Windows

### Changed
- **Breaking:** `Model::view()` returns `View`, not `String`
- **Breaking:** Imperative alt-screen/mouse/focus commands removed; set `View` fields instead
- **Breaking:** `KeyMsg` split into press/release; use `KeyPressMsg` or `legacy_key_msg()` during migration
- Widgets updated for v2 messages and `View` output
- `lipgloss::Renderer` documented as deprecated in favor of `OutputContext`; examples use `lipgloss::output`
- `Blend1D`/`Blend2D`/`BlendBorder` Go-parity aliases added alongside `blend_1d`/`blend_2d`

### Migration

1. Change `fn view(&self) -> String` to `fn view(&self) -> View` with `View::new(content)`.
2. Move alt-screen, mouse, focus, and title settings from `ProgramBuilder` toggles / commands into `View` fields each frame.
3. Replace `KeyMsg` handlers with `KeyPressMsg` (or `legacy_key_msg(&msg)` temporarily).
4. Use `OutputContext` instead of `lipgloss::Renderer` for new styling code.

## [0.1.12] - 2026-06-24

### Changed
- Renamed **`bubblet`** → **`bubble-t`** (crate + library `bubble_t`)
- Renamed **`bubbletea-widgets`** → **`bubble-t-widgets`** (crate + library `bubble_t_widgets`)
- Unified all workspace crate versions to **0.1.12** via `[workspace.package]`
- Synchronized MIT LICENSE (whit3rabbit 2025, Nexlab-One 2026) across all published crates

## [0.0.10] - 2026-06-24

### Added
- Monorepo workspace consolidating `bubble-t` (core), `lipgloss-*`, and `bubble-t-widgets`
- `rust-toolchain.toml` pinning stable Rust with rustfmt and clippy
- API documentation under `docs/` for core, widgets, and lipgloss
- Sentrux structural quality rules (`.sentrux/rules.toml`)
- `cargo audit` security check in CI

### Security
- Replaced unmaintained `clipboard` crate with `arboard` in bubble-t-widgets (fixes RUSTSEC-2021-0019 / RUSTSEC-2022-0056)
- Removed unused `async-std-runtime` feature (discontinued upstream, RUSTSEC-2025-0052)
- Sanitized download filenames and restricted URL schemes in `progress-download` example

### Changed
- Renamed core crate from `bubbletea-rs` to **`bubble-t`** (v0.0.10)
- Migrated all workspace members to Rust edition 2024
- Updated dependencies to latest compatible versions (tokio 1.52, reqwest 0.13, rand 0.10, crossterm 0.29, etc.)
- README, LICENSE, and CI workflows updated for Nexlab-One fork ([Bubble-T](https://github.com/Nexlab-One/Bubble-T))
- Consolidated external dependencies into `[workspace.dependencies]`; examples use `{ workspace = true }`
- MSRV bumped to **1.92.0** for latest stable Rust / edition 2024

### Consolidated upstream sources
- lipgloss-rs @ `3d19aa0b5b23314ccfb2b47d711a9e3a170a261b`
- bubbles-rs @ `91020d7ad7c387723c2f8316135860119d4b6e60`

## [0.0.9] - 2025-01-22

### Fixed
- Eliminate double key events on Windows (#10)

### Changed
- Minor formatting fixes in `src/program.rs` for CI compliance

## [0.0.8] - 2025-01-XX

### Added
- Handle `RequestWindowSizeMsg` in Program
- Responsive layout and window size handling to paginator example
- Examples for paginator, window size handling, and table components
- Split-editors example

### Fixed
- Batch command tests for non-blocking behavior
- Various formatting and CI fixes

### Changed
- Dependency updates via Dependabot

## [0.0.7] and earlier

See [GitHub releases](https://github.com/Nexlab-One/Bubble-T/releases) for earlier versions.
