# bubble-t-widgets API Reference

Reusable TUI components (`crates/bubble-t-widgets`), porting [charmbracelet/bubbles](https://github.com/charmbracelet/bubbles) for use with **bubble-t**.

## Design

Each component is a self-contained model with:

- Constructor (`new()`, module-specific factories)
- `update(msg) -> Option<Cmd>` — process v2 messages (`KeyPressMsg`, mouse, paste)
- `view() -> View` — render styled output (use `.content` when embedding in strings)
- `Component` trait — `focus()`, `blur()`, `focused()` for keyboard navigation

Components do not implement `bubble_t::Model` directly; compose them inside your application `Model`.

## `Component` trait

```rust
pub trait Component {
    fn focus(&mut self) -> Option<Cmd>;
    fn blur(&mut self);
    fn focused(&self) -> bool;
}
```

Use focus management when building forms with multiple inputs.

## Available components

| Module | Type alias | Description |
|--------|------------|-------------|
| `textinput` | `TextInput` | Single-line input with cursor, echo modes, validation |
| `textarea` | `TextArea` | Multi-line editor |
| `list` | `List` | Selectable list with filtering and pagination |
| `table` | `Table` | Columnar data display |
| `spinner` | `Spinner` | Animated loading indicator |
| `progress` | `Progress` | Progress bar |
| `paginator` | `Paginator` | Page navigation |
| `viewport` | `Viewport` | Scrollable content region |
| `help` | `HelpModel` | Key binding help footer |
| `filepicker` | `FilePicker` | Directory/file browser |
| `timer` / `stopwatch` | — | Time display widgets |
| `cursor` | `Cursor` | Blinking cursor helper |
| `key` | `Binding`, `KeyMap` | Declarative key bindings |

## Key bindings

```rust
use bubble_t_widgets::key::{new_binding, with_keys_str, with_help, Binding};

let quit = new_binding(vec![
    with_keys_str(&["q", "esc"]),
    with_help("q/esc", "quit"),
]);

if quit.matches(key_msg) { /* ... */ }
```

## Text input example

```rust
use bubble_t_widgets::textinput;

let mut input = textinput::new();
input.set_width(40);
input.set_placeholder("Name...");
let _ = input.focus();

// In your Model::update:
input.update(msg);

// In your Model::view:
input.view()
```

## List example

```rust
use bubble_t_widgets::list::{self, DefaultItem};

let items: Vec<DefaultItem> = vec![
    DefaultItem::new("Item 1", "Description"),
    DefaultItem::new("Item 2", "Description"),
];
let mut list = list::new(items);
list.set_size(80, 20);
```

## Styling

Components accept `lipgloss_extras::lipgloss::Style` for prompts, selections, and help text. Import styles from lipgloss, not raw ANSI escape codes.

## Prelude

```rust
use bubble_t_widgets::prelude::*;
```

Re-exports constructors (`textinput_new`, `list_new`, etc.), component types, and the `Component` trait.

## Go bubbles mapping

| Go (bubbles) | bubble-t-widgets |
|--------------|-------------------|
| `textinput.Model` | `textinput::Model` |
| `textarea.Model` | `textarea::Model` |
| `list.Model` | `list::Model` |
| `table.Model` | `table::Model` |
| `spinner.Model` | `spinner::Model` |
| `progress.Model` | `progress::Model` |
| `viewport.Model` | `viewport::Model` |
| `help.Model` | `help::Model` |
| `key.Binding` | `key::Binding` |

Update/view method names match Go; Rust uses `Option<Cmd>` return types consistent with bubble-t.
