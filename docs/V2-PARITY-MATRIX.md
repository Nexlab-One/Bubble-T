# Bubble Tea v2 Parity Matrix

This document maps every upstream Charm source area to its target crate and module
in this workspace, and records the current implementation status. It is the contract
for the v2 parity effort: each later tick closes rows by moving them from **Missing**
or **Partial** to **Present**.

## Upstream references

The tables below are derived from the upstream Go sources cloned into `reference/`
(gitignored, reference-only). Pinned versions:


| Repo                         | Path                     | Version cloned            |
| ---------------------------- | ------------------------ | ------------------------- |
| `charmbracelet/bubbletea`    | `reference/bubbletea`    | tag `v2.0.7`              |
| `charmbracelet/lipgloss`     | `reference/lipgloss`     | tag `v2.0.0`              |
| `charmbracelet/bubbles`      | `reference/bubbles`      | tag `v2.0.0`              |
| `charmbracelet/x`            | `reference/x`            | default branch (`main`)   |
| `charmbracelet/colorprofile` | `reference/colorprofile` | default branch (`main`)   |
| `charmbracelet/harmonica`    | `reference/harmonica`    | default branch (`main`)   |
| `charmbracelet/glamour`      | `reference/glamour`      | default branch (`main`)   |
| `muesli/reflow`              | `reference/reflow`       | default branch (`master`) |
| `muesli/cancelreader`        | `reference/cancelreader` | default branch (`master`) |


`x`, `colorprofile`, `harmonica`, `glamour`, `reflow`, and `cancelreader` do not
publish tags matching the consuming versions, so they track their default branch.

## Status legend


| Status      | Meaning                                                                    |
| ----------- | -------------------------------------------------------------------------- |
| **Present** | Implemented in Rust with v2-equivalent behavior.                           |
| **Partial** | Some equivalent exists, but it is v1-shaped, ad hoc, or incomplete for v2. |
| **Missing** | No Rust equivalent exists yet.                                             |


A status of Partial or Missing does not imply the file is unimportant; it records the
delta against v2 that a later tick must close.

## Target crate graph

```
ansi            (x/ansi)            -> no internal deps
cellbuf         (x/cellbuf)         -> ansi
colorprofile    (colorprofile)      -> ansi
term            (x/term)            -> no internal deps
harmonica       (harmonica)         -> no internal deps
lipgloss        (lipgloss)          -> ansi, colorprofile, cellbuf
lipgloss-{list,table,tree}          -> lipgloss
lipgloss-extras                     -> lipgloss-{list,table,tree}
bubble-t        (bubbletea core)    -> ansi, cellbuf, colorprofile, term, lipgloss
bubble-t-widgets(bubbles)           -> bubble-t, lipgloss, harmonica
```

As of tick 12 (2026-06-25) the foundation crates, v2 `View`/messages,
cursed renderer, Lip Gloss compositing, keyboard enhancements, harmonica progress, OSC
query parsers, hardscroll cost model, signal/suspend handling, terminal image rendering
(Sixel + Kitty), and quality gates are in place for **full v2 parity** with bubbletea
v2.0.7 + lipgloss v2 + bubbles + x foundation crates.

---

## crates/ansi (port of `x/ansi`)


| Upstream file                                              | Key symbols                                          | Target (`ansi::`)                               | Status  |
| ---------------------------------------------------------- | ---------------------------------------------------- | ----------------------------------------------- | ------- |
| `ansi.go`, `c0.go`, `c1.go`, `ctrl.go`                     | C0/C1 controls, control builders                     | `c0`, `c1`, `ctrl`                              | Present |
| `parser_decode.go`                                         | `DecodeSequence`, grapheme width                     | `parse::decode`                                 | Present |
| `parser.go`, `parser_handler.go`, `parser_sync.go`         | `Parser::advance`, handlers                          | `parse::parser`                                 | Present |
| `sgr.go`, `style.go`                                       | `Style`, SGR attribute builders                      | `sgr`, `style`                                  | Present |
| `color.go`                                                 | `Color`, `Convert256`, `Convert16`, `ReadStyleColor` | `color`                                         | Present |
| `palette.go`                                               | `SetPalette`, `ResetPalette`                         | `palette`                                       | Present |
| `mode.go`, `modes.go`, `mode_deprecated.go`                | DEC private modes, set/reset, DECRPM                 | `mode`                                          | Present |
| `cursor.go`                                                | cursor movement / shape sequences                    | `cursor`                                        | Present |
| `screen.go`, `reset.go`                                    | erase/scroll/clear, RIS                              | `screen`                                        | Present |
| `mouse.go`                                                 | SGR mouse encode/decode                              | `mouse`                                         | Present |
| `kitty.go`, `keypad.go`, `xterm.go`                        | Kitty keyboard, modifyOtherKeys, keypad              | `kitty`, `xterm`                                | Present |
| `clipboard.go`                                             | OSC 52 set/query                                     | `clipboard`                                     | Present |
| `hyperlink.go`                                             | OSC 8 hyperlinks                                     | `hyperlink`                                     | Present |
| `title.go`, `notification.go`, `iterm2.go`, `finalterm.go` | OSC titles/notifications                             | `title`, `notification`, `finalterm`            | Present |
| `background.go`, `cwd.go`, `progress.go`, `passthrough.go` | OSC 10–12, 7, 9;4, passthrough                       | `background`, `cwd`, `progress`, `passthrough`  | Present |
| `status.go`, `termcap.go`, `xterm.go`                      | DA, XTVERSION, XTGETTCAP, DECRPM, DSR                | `ctrl`, `status`, `query`                       | Present |
| `paste.go`                                                 | bracketed paste mode                                 | `paste`                                         | Present |
| `focus.go`                                                 | focus reporting mode                                 | `focus`                                         | Present |
| `graphics.go`, `inband.go`, `winop.go`                     | Sixel/Kitty graphics, in-band resize, window ops     | `graphics`, `graphics::kitty`, `sixel`, `winop` | Present |
| `charset.go`, `method.go`                                  | charset designation, width method                    | `width::Method`                                 | Present |
| `width.go`, `wrap.go`, `truncate.go`, `util.go`            | width-aware string ops                               | `width`, `wrap`, `truncate`                     | Present |
| `ascii.go`, `urxvt.go`                                     | ASCII helpers, URxvt OSC 777                         | `c0`, `urxvt`                                   | Present |


## crates/cellbuf (port of `x/cellbuf`)


| Upstream file                       | Key symbols                                | Target (`cellbuf::`) | Status                              |
| ----------------------------------- | ------------------------------------------ | -------------------- | ----------------------------------- |
| `cell.go`                           | `Cell`, content/width, wide placeholders   | `cell`               | Present                             |
| `style.go`, `pen.go`, `sequence.go` | `Style`, `ReadStyle`, SGR diff             | `style`              | Present                             |
| `buffer.go`                         | `Buffer`, grid storage, resize             | `buffer`             | Present                             |
| `screen.go`                         | `Screen`, cursor, redraw                   | `screen`             | Present                             |
| `sequence.go`                       | cell-to-ANSI emit (`Render`, `RenderLine`) | `render`             | Present                             |
| `hardscroll.go`, `hashmap.go`       | scroll-optimized diff                      | `hardscroll`, `diff` | Present (growHunks + costEffective) |
| `geom.go`                           | `Rectangle`, `Position`                    | `geom`               | Present                             |
| `link.go`                           | hyperlink cell metadata                    | `link`               | Present                             |
| `tabstop.go`                        | tab stops                                  | `tabstop`            | Present                             |
| `wrap.go`, `utils.go`               | wrapping, helpers                          | `wrap`, `util`       | Present                             |
| `writer.go`                         | `SetContent`, `printString`                | `writer`             | Present                             |
| `errors.go`                         | error types                                | `error`              | Present                             |
| (graphics placement)                | inline Sixel/Kitty in cell grid            | `graphics`           | Present                             |


## crates/colorprofile (port of `colorprofile`)


| Upstream file                              | Key symbols                      | Target (`colorprofile::`) | Status  |
| ------------------------------------------ | -------------------------------- | ------------------------- | ------- |
| `profile.go`                               | `Profile`, `Convert`, downsample | `profile`                 | Present |
| `env.go`, `env_windows.go`, `env_other.go` | `Detect`, env inspection         | `env`                     | Present |
| `writer.go`                                | downsampling `Writer`            | `writer`                  | Present |
| `doc.go`                                   | crate docs                       | crate root                | Present |
| (terminfo)                                 | `Terminfo`, `Tc`/`RGB` caps      | `terminfo`                | Present |
| (tmux)                                     | `Tmux`, `tmux info`              | `tmux`                    | Present |


## crates/term (port of `x/term`)


| Upstream file                                     | Key symbols                        | Target (`term::`) | Status  |
| ------------------------------------------------- | ---------------------------------- | ----------------- | ------- |
| `term.go`, `terminal.go`                          | `State`, raw-mode API              | `lib`             | Present |
| `term_windows.go`                                 | Windows console mode               | `sys::windows`    | Present |
| `term_unix*.go`, `term_plan9.go`, `term_other.go` | termios raw mode                   | `sys::unix`       | Present |
| `util.go`                                         | `GetSize`, `IsTerminal`, `OpenTTY` | `lib`             | Present |


## crates/harmonica (port of `harmonica`)


| Upstream file   | Key symbols                     | Target (`harmonica::`) | Status  |
| --------------- | ------------------------------- | ---------------------- | ------- |
| `spring.go`     | `Spring`, `NewSpring`, `Update` | `spring`               | Present |
| `harmonica.go`  | `FPS`, deltas                   | `lib`                  | Present |
| `projectile.go` | `Projectile`, `Point`, `Vector` | `projectile`           | Present |


---

## bubble-t core (port of `bubbletea`)


| Upstream file                                       | Key symbols                                           | Target (`bubble_t::`)     | Status                                                                    |
| --------------------------------------------------- | ----------------------------------------------------- | ------------------------- | ------------------------------------------------------------------------- |
| `tea.go`                                            | `Program`, `Model`, run loop                          | `program`, `model`        | Present                                                                   |
| `mod.go`                                            | `Model` trait                                         | `model`                   | Present                                                                   |
| `renderer.go`, `nil_renderer.go`                    | `renderer` interface                                  | `program` (renderer)      | Present                                                                   |
| `cursed_renderer.go`                                | cell-diff renderer over cellbuf                       | `renderer::cursed`        | Present                                                                   |
| `screen.go`                                         | alt-screen/clear sequences                            | `program`                 | Present (declarative `View`)                                              |
| `key.go`                                            | `KeyPressMsg`/`KeyReleaseMsg`, `Key`, `keystroke()`   | `event`, `input`, `key`   | Present                                                                   |
| `keyboard.go`                                       | keyboard enhancements (Kitty/modifyOtherKeys)         | `event`, `input`          | Present                                                                   |
| `mouse.go`                                          | `MouseClick/Release/Wheel/Motion`, `Mouse`            | `event`                   | Present                                                                   |
| `paste.go`                                          | `PasteMsg`/`PasteStart`/`PasteEnd`                    | `event`                   | Present                                                                   |
| `focus.go`                                          | `FocusMsg`/`BlurMsg`, report-focus                    | `event`                   | Present                                                                   |
| `cursor.go`                                         | `Cursor`, shape/blink/color                           | `view::Cursor`            | Present                                                                   |
| `clipboard.go`                                      | OSC 52 `SetClipboard`/`ReadClipboard`, `ClipboardMsg` | `command`, `event`        | Present                                                                   |
| `color.go`                                          | fg/bg/cursor color msgs, `HasDarkBackground`          | `command`, `event`        | Present                                                                   |
| `profile.go`                                        | `ColorProfileMsg`, profile wiring                     | `command`, `program`      | Present                                                                   |
| `environ.go`                                        | `EnvMsg`, `WithEnvironment`                           | `command`, `program`      | Present                                                                   |
| `xterm.go`, `termcap.go`                            | XTVERSION/XTGETTCAP queries + msgs                    | `command`, `event`        | Present                                                                   |
| `commands.go`                                       | `Tick`/`Every`/`Sequence`/`Batch`/`Quit` etc.         | `command`                 | Present                                                                   |
| `options.go`                                        | `with_`* options, `OpenTTY`                           | `program`                 | Present                                                                   |
| `exec.go`                                           | `ExecProcess`, `ExecCommand`                          | `command`                 | Present                                                                   |
| `tty.go`, `tty_unix.go`, `tty_windows.go`, `raw.go` | TTY open / raw mode                                   | `terminal` (-> `term`)    | Present                                                                   |
| `termios_*.go`                                      | termios per-platform                                  | (-> `term` + `crossterm`) | Present                                                                   |
| `signals_unix.go`, `signals_windows.go`             | signal handling                                       | `signals`, `program`      | Present (SIGINT→Interrupt, SIGTERM→Quit, SIGWINCH→resize, suspend/resume) |
| `logging.go`                                        | `LogToFile`                                           | `logging`                 | Present                                                                   |
| `environ.go`/`input.go`                             | input reader/parser                                   | `input`, `query_parser`   | Present                                                                   |


## lipgloss (port of `lipgloss` v2)


| Upstream file                              | Key symbols                                       | Target (`lipgloss::`)        | Status  |
| ------------------------------------------ | ------------------------------------------------- | ---------------------------- | ------- |
| `style.go`, `set.go`, `get.go`, `unset.go` | `Style`, setters/getters                          | `style`                      | Present |
| `color.go`                                 | `Color`, `RGBColor`, `LightDark`, `Complete`      | `color`                      | Present |
| `terminal.go`, `query.go`                  | terminal detection (moved to writer/colorprofile) | (-> `colorprofile`)          | Present |
| `writer.go`                                | output writer + downsampling                      | `output` (-> `colorprofile`) | Present |
| `borders.go`                               | `Border`, presets                                 | `border`                     | Present |
| `align.go`                                 | horizontal/vertical align                         | `align`                      | Present |
| `join.go`                                  | `JoinHorizontal`/`JoinVertical`                   | `join`                       | Present |
| `position.go`                              | `Position`, `Place`                               | `position`                   | Present |
| `size.go`                                  | `Width`/`Height`                                  | `size`                       | Present |
| `whitespace.go`                            | whitespace fill                                   | `whitespace`                 | Present |
| `wrap.go`, `runes.go`, `ranges.go`         | wrapping, rune/range styling                      | `utils`                      | Present |
| `blending.go`                              | `Blend1D`/`Blend2D`, border blend                 | `blending`                   | Present |
| `layer.go`                                 | `Layer` (X/Y/Z, nested)                           | `layer`                      | Present |
| `canvas.go`                                | `Canvas`, `Compositor`, `Compose`/`Render`, `Hit` | `canvas`, `layer`            | Present |
| (image inline)                             | Sixel/Kitty render helpers                        | `image`                      | Present |
| `ansi_unix.go`, `ansi_windows.go`          | platform ANSI enable                              | (-> `colorprofile`/`term`)   | Present |
| `lipgloss.go`                              | package entry, hyperlink (OSC 8)                  | `lib`                        | Present |
| `compat/`                                  | `AdaptiveColor` compatibility                     | `color::compat`              | Present |
| `list/`, `table/`, `tree/`                 | sub-packages                                      | `lipgloss-{list,table,tree}` | Present |


## bubbles (port of `bubbles` v2) -> bubble-t-widgets


| Upstream pkg | Target (`bubble_t_widgets::`) | Status  | v2 delta                           |
| ------------ | ----------------------------- | ------- | ---------------------------------- |
| `cursor`     | `cursor`                      | Present | integrates `tea::Cursor`           |
| `key`        | `key`                         | Present | `KeyPressMsg` matching             |
| `help`       | `help`                        | Present | v2 messages                        |
| `paginator`  | `paginator`                   | Present | v2 messages                        |
| `progress`   | `progress`                    | Present | harmonica spring for animated fill |
| `spinner`    | `spinner`                     | Present | v2 messages                        |
| `stopwatch`  | `stopwatch`                   | Present | v2 messages                        |
| `timer`      | `timer`                       | Present | v2 messages                        |
| `viewport`   | `viewport`                    | Present | v2 messages/mouse                  |
| `textinput`  | `textinput`                   | Present | integrates `tea::Cursor`           |
| `textarea`   | `textarea`                    | Present | integrates `tea::Cursor`           |
| `filepicker` | `filepicker`                  | Present | v2 messages                        |
| `list`       | `list`                        | Present | v2 messages/Lip Gloss              |
| `table`      | `table`                       | Present | v2 messages/Lip Gloss              |
| (none)       | `clipboard`                   | N/A     | replaced by core OSC 52            |


## Intentional remaining gaps (vs upstream v2.0.7)


| Area                             | Status            | Rationale                                                                                                                                                            |
| -------------------------------- | ----------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `cellbuf::wrap` styled grid walk | Partial by design | Style-aware line wrap for cell grids delegates plain text to `ansi::wrap`; full PenWriter-style styled space preservation matches lipgloss layer compositing instead |
| `cancelreader` exact API         | Folded            | Async cancel semantics provided by tokio + `Program` shutdown token                                                                                                  |


## Out-of-tree references


| Repo           | Purpose                       | Consuming crate                          |
| -------------- | ----------------------------- | ---------------------------------------- |
| `glamour`      | Markdown rendering (examples) | `examples/glamour`                       |
| `reflow`       | wrapping/indent algorithms    | folded into `ansi`/`lipgloss` width/wrap |
| `cancelreader` | cancelable stdin reads        | folded into `bubble-t::input` / `term`   |


