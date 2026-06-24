# lipgloss API Reference

Terminal styling framework (`crates/lipgloss` and siblings), porting [charmbracelet/lipgloss](https://github.com/charmbracelet/lipgloss) for Rust.

## Crate structure

| Crate | Role |
|-------|------|
| `lipgloss` | Core `Style`, colors, layout, whitespace |
| `lipgloss-list` | Styled vertical lists |
| `lipgloss-table` | Styled tables |
| `lipgloss-tree` | Tree diagrams |
| `lipgloss-extras` | Facade with `full` feature enabling all components |

Use `lipgloss-extras` in applications:

```toml
lipgloss-extras = { path = "../crates/lipgloss-extras", features = ["full"] }
```

## Core types (`lipgloss`)

### `Style`

Fluent builder for terminal attributes:

```rust
use lipgloss_extras::lipgloss::{Style, Color};

let title = Style::new()
    .bold(true)
    .foreground(Color::from("#FF75C7"))
    .padding(1, 2)
    .border(lipgloss::Border::Rounded);

title.render("Hello")
```

Common methods: `foreground`, `background`, `bold`, `italic`, `underline`, `width`, `height`, `align`, `border`, `margin`, `padding`, `render`.

### `Color`

Supports hex (`#RRGGBB`), ANSI indices, and adaptive colors. Color conversion uses perceptual distance (CIE L\*a\*b\*) for ANSI palette matching, matching Go termenv behavior.

### Layout

- `JoinHorizontal`, `JoinVertical` — compose styled blocks
- `Place` — position content within a region
- `whitespace` module — background fill characters

## Component crates

### Lists (`lipgloss-list`)

```rust
use lipgloss_extras::list::List;

let output = List::new()
    .items(vec!["Alpha", "Beta", "Gamma"])
    .render();
```

### Tables (`lipgloss-table`)

Column headers, row styling, width constraints. See `examples/table` for bubble-t integration.

### Trees (`lipgloss-tree`)

Hierarchical ASCII/Unicode tree rendering with customizable branch characters.

## Prelude

```rust
use lipgloss_extras::prelude::*;
```

Re-exports lipgloss types; with `full` feature, also exports list/table/tree builders.

**Note:** Avoid glob-importing the prelude in test modules that also define local `new()` functions — use explicit imports (`Color`, `Style`) to prevent name collisions.

## Go lipgloss mapping

| Go | lipgloss-rs |
|----|-------------|
| `lipgloss.NewStyle()` | `Style::new()` |
| `Style.Foreground()` | `.foreground()` |
| `Style.Render()` | `.render()` |
| `lipgloss.JoinHorizontal()` | `JoinHorizontal()` |
| `lipgloss.Place()` | `Place()` |
| `list.Model` (bubbles) | `bubble-t-widgets::list` (interactive) vs `lipgloss-list` (static render) |

**Distinction:** lipgloss crates render static styled strings; bubble-t-widgets adds interactive state and bubble-t message handling. Use both together: lipgloss for appearance, widgets for behavior.

## Color profiles

Styles respect terminal color capability (truecolor vs ANSI). Use `AdaptiveColor` for light/dark terminal variants where supported.
