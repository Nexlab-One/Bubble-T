# Pager

<img width="1200" src="./pager.gif" />

A document viewer demonstrating the viewport component from bubble-t-widgets. This example shows how to create a responsive pager with styled headers, footers, and scrollable content.

## Key Learning Patterns Demonstrated

### 📜 **Viewport Component Usage**
- **Scrollable Content**: Using `bubble-t-widgets::viewport` for text display
- **Dynamic Content Loading**: Reading markdown files from disk  
- **Responsive Layout**: Adjusting viewport size based on header/footer height
- **Mouse & Keyboard Navigation**: Full scrolling support

### 🎨 **Advanced Lipgloss Styling**
- **Custom Borders**: Modifying border characters for visual connections
- **Dynamic Line Drawing**: Creating horizontal lines that fill available space
- **Layout Calculations**: Computing widths for responsive design
- **Styled Text Rendering**: Combining borders, padding, and content

### 💻 **Program Configuration**
- **Alternate Screen Buffer**: Full-screen TUI mode with `.alt_screen()`
- **Mouse Support**: Enabling mouse wheel scrolling with `.mouse_motion()`
- **File I/O Integration**: Loading external content at startup
- **Error Handling**: Graceful handling of missing files

## Usage

```bash
cargo run --bin pager
```

## Controls

- **↑/↓ Arrow Keys**: Scroll up/down
- **Page Up/Down**: Page scrolling
- **Home/End**: Jump to top/bottom
- **Mouse Wheel**: Scroll content
- **q** or **Esc**: Quit the program
- **Ctrl+C**: Force quit

## What You'll See

```
╭ Mr. Pager ──────────────────────────────────────────────────────────────╮
│ Glow                                                                     │
│ ====                                                                     │
│                                                                          │
│ A casual introduction. 你好世界!                                          │
│                                                                          │
│ ## Let's talk about artichokes                                           │
│                                                                          │
│ The _artichoke_ is mentioned as a garden plant in the 8th century BC    │
│ ...                                                                      │
╰──────────────────────────────────────────────────────────────────── 47% ╯
```

- **Styled Header**: Title with border connection to horizontal line
- **Scrollable Content**: Full markdown document with proper formatting
- **Styled Footer**: Scroll percentage with border and horizontal line
- **Responsive Layout**: Adapts to terminal resize
- **Full Navigation**: Keyboard and mouse support

This is a faithful port of the Go Bubble Tea pager example with identical behavior, UI styling, and navigation.