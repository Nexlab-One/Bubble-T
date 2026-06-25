# lipgloss API Reference

Terminal styling framework (`crates/lipgloss` and siblings), porting [charmbracelet/lipgloss](https://github.com/charmbracelet/lipgloss) for Rust.

## Crate structure

| Crate | Role |
|-------|------|
| `lipgloss` | Core `Style`, colors, layout, compositing, `OutputContext` |
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

Common methods: `foreground`, `background`, `bold`, `italic`, `underline`, `hyperlink`, `width`, `height`, `align`, `border`, `margin`, `padding`, `render`.

### `OutputContext` (v2)

Preferred path for color profile detection and downsampling:

```rust
use lipgloss::OutputContext;

let ctx = OutputContext::from_env();
let downsampled = ctx.downsample(&styled);
```

The legacy `Renderer` type remains as a thin compatibility wrapper; new code should use `OutputContext`.

### Compositing (v2)

Layer trees over `cellbuf` with z-order blending:

```rust
use lipgloss::{Canvas, Compositor, Layer};

let comp = Compositor::new(vec![
    Layer::new("background", vec![]).id("bg").z(0),
    Layer::new("panel", vec![]).id("panel").x(2).y(1).z(1),
]);
let hit = comp.hit(2, 1); // top-most layer id at (x, y)
let mut canvas = Canvas::new(40, 10);
canvas.compose(&comp);
println!("{}", canvas.render());
```

### Blending

```rust
use lipgloss::{Blend1D, Blend2D, blend_border, Color};

let gradient = Blend1D(5, vec![Color::from("#f00"), Color::from("#00f")]);
```
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

Hierarchical ASCII/Unicode tree rendering with customizable branch characters, optional `width()`, and `indenter_style()`.

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

Styles respect terminal color capability via `OutputContext` / `colorprofile`. Use `AdaptiveColor` / `CompleteColor` for light/dark variants.
