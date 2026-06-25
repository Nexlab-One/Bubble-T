//! Applies declarative [`View`] fields to the terminal each frame.

use crate::Error;
use crate::renderer::{RenderFrameOptions, progress_bar_sequence};
use crate::terminal::TerminalInterface;
use crate::view::{AppliedViewState, MouseMode};

/// Diffs `next` against `applied` and updates the terminal to match.
pub(crate) async fn apply_view(
    terminal: &mut dyn TerminalInterface,
    applied: &mut AppliedViewState,
    next: AppliedViewState,
) -> Result<(), Error> {
    if next.alt_screen != applied.alt_screen {
        if next.alt_screen {
            terminal.enter_alt_screen().await?;
        } else {
            terminal.exit_alt_screen().await?;
        }
    }

    if next.mouse_mode != applied.mouse_mode {
        apply_mouse_mode(terminal, next.mouse_mode).await?;
    }

    if next.report_focus != applied.report_focus {
        if next.report_focus {
            terminal.enable_focus_reporting().await?;
        } else {
            terminal.disable_focus_reporting().await?;
        }
    }

    if next.bracketed_paste != applied.bracketed_paste {
        if next.bracketed_paste {
            terminal.enable_bracketed_paste().await?;
        } else {
            terminal.disable_bracketed_paste().await?;
        }
    }

    if next.window_title != applied.window_title {
        terminal.set_window_title(&next.window_title).await?;
    }

    if next.cursor != applied.cursor {
        apply_cursor(terminal, next.cursor.as_ref()).await?;
    }

    if next.foreground_color != applied.foreground_color {
        apply_foreground_color(terminal, next.foreground_color).await?;
    }

    if next.background_color != applied.background_color {
        apply_background_color(terminal, next.background_color).await?;
    }

    if next.progress_bar != applied.progress_bar {
        let seq = progress_bar_sequence(next.progress_bar.as_ref());
        terminal.write_raw(seq.as_bytes()).await?;
    }

    if next.keyboard_enhancements != applied.keyboard_enhancements {
        apply_keyboard_enhancements(terminal, next.keyboard_enhancements).await?;
    }

    *applied = next;
    Ok(())
}

/// Builds OutputContext options from a [`View`] for the cell diff pass.
pub(crate) fn render_options_from_view(view: &crate::view::View) -> RenderFrameOptions {
    RenderFrameOptions {
        foreground_color: view.foreground_color,
        background_color: view.background_color,
        progress_bar: view.progress_bar.clone(),
    }
}

async fn apply_mouse_mode(
    terminal: &mut dyn TerminalInterface,
    mode: MouseMode,
) -> Result<(), Error> {
    terminal.disable_mouse().await?;
    match mode {
        MouseMode::None => {}
        MouseMode::CellMotion => terminal.enable_mouse_cell_motion().await?,
        MouseMode::AllMotion => terminal.enable_mouse_all_motion().await?,
    }
    Ok(())
}

async fn apply_cursor(
    terminal: &mut dyn TerminalInterface,
    cursor: Option<&crate::view::Cursor>,
) -> Result<(), Error> {
    match cursor {
        Some(c) => {
            terminal.show_cursor().await?;
            terminal.set_cursor_style(c).await?;
            terminal
                .set_cursor_position(c.position.x, c.position.y)
                .await?;
        }
        None => terminal.hide_cursor().await?,
    }
    Ok(())
}

async fn apply_foreground_color(
    terminal: &mut dyn TerminalInterface,
    color: Option<ansi::color::Color>,
) -> Result<(), Error> {
    use ansi::background::{RESET_FOREGROUND_COLOR, set_foreground_color};
    let seq = match color {
        Some(c) => set_foreground_color(&color_to_hex(c)),
        None => RESET_FOREGROUND_COLOR.to_string(),
    };
    terminal.write_raw(seq.as_bytes()).await
}

async fn apply_background_color(
    terminal: &mut dyn TerminalInterface,
    color: Option<ansi::color::Color>,
) -> Result<(), Error> {
    use ansi::background::{RESET_BACKGROUND_COLOR, set_background_color};
    let seq = match color {
        Some(c) => set_background_color(&color_to_hex(c)),
        None => RESET_BACKGROUND_COLOR.to_string(),
    };
    terminal.write_raw(seq.as_bytes()).await
}

async fn apply_keyboard_enhancements(
    terminal: &mut dyn TerminalInterface,
    enhancements: crate::view::KeyboardEnhancements,
) -> Result<(), Error> {
    use ansi::kitty::{REQUEST_KITTY_KEYBOARD, kitty_keyboard};
    use ansi::xterm::SET_MODIFY_OTHER_KEYS2;

    terminal
        .write_raw(SET_MODIFY_OTHER_KEYS2.as_bytes())
        .await?;
    let flags = enhancements.kitty_flags();
    let seq = kitty_keyboard(flags, 1);
    terminal.write_raw(seq.as_bytes()).await?;
    terminal
        .write_raw(REQUEST_KITTY_KEYBOARD.as_bytes())
        .await?;
    Ok(())
}

fn color_to_hex(color: ansi::color::Color) -> String {
    use ansi::color::{Color, indexed_to_rgb};
    let rgb = match color {
        Color::Rgb(c) => c,
        Color::Indexed(i) => indexed_to_rgb(i.0),
        Color::Basic(c) => indexed_to_rgb(c as u8),
    };
    format!("#{:02x}{:02x}{:02x}", rgb.r, rgb.g, rgb.b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::{Cursor, Position, ProgressBar, ProgressBarState, View};
    use std::sync::{Arc, Mutex};

    struct RecordingTerminal {
        ops: Arc<Mutex<Vec<String>>>,
    }

    #[async_trait::async_trait]
    impl TerminalInterface for RecordingTerminal {
        fn new(
            _: Option<std::sync::Arc<tokio::sync::Mutex<dyn tokio::io::AsyncWrite + Send + Unpin>>>,
        ) -> Result<Self, Error>
        where
            Self: Sized,
        {
            Ok(Self {
                ops: Arc::new(Mutex::new(Vec::new())),
            })
        }

        async fn enter_raw_mode(&mut self) -> Result<(), Error> {
            Ok(())
        }
        async fn exit_raw_mode(&mut self) -> Result<(), Error> {
            Ok(())
        }
        async fn enter_alt_screen(&mut self) -> Result<(), Error> {
            self.ops.lock().unwrap().push("alt_on".into());
            Ok(())
        }
        async fn exit_alt_screen(&mut self) -> Result<(), Error> {
            self.ops.lock().unwrap().push("alt_off".into());
            Ok(())
        }
        async fn enable_mouse(&mut self) -> Result<(), Error> {
            Ok(())
        }
        async fn enable_mouse_cell_motion(&mut self) -> Result<(), Error> {
            self.ops.lock().unwrap().push("mouse_cell".into());
            Ok(())
        }
        async fn enable_mouse_all_motion(&mut self) -> Result<(), Error> {
            self.ops.lock().unwrap().push("mouse_all".into());
            Ok(())
        }
        async fn disable_mouse(&mut self) -> Result<(), Error> {
            self.ops.lock().unwrap().push("mouse_off".into());
            Ok(())
        }
        async fn enable_focus_reporting(&mut self) -> Result<(), Error> {
            self.ops.lock().unwrap().push("focus_on".into());
            Ok(())
        }
        async fn disable_focus_reporting(&mut self) -> Result<(), Error> {
            self.ops.lock().unwrap().push("focus_off".into());
            Ok(())
        }
        async fn enable_bracketed_paste(&mut self) -> Result<(), Error> {
            self.ops.lock().unwrap().push("paste_on".into());
            Ok(())
        }
        async fn disable_bracketed_paste(&mut self) -> Result<(), Error> {
            self.ops.lock().unwrap().push("paste_off".into());
            Ok(())
        }
        async fn show_cursor(&mut self) -> Result<(), Error> {
            self.ops.lock().unwrap().push("cursor_show".into());
            Ok(())
        }
        async fn hide_cursor(&mut self) -> Result<(), Error> {
            self.ops.lock().unwrap().push("cursor_hide".into());
            Ok(())
        }
        async fn clear(&mut self) -> Result<(), Error> {
            Ok(())
        }
        async fn render(&mut self, _: &str) -> Result<(), Error> {
            Ok(())
        }
        async fn write_raw(&mut self, data: &[u8]) -> Result<(), Error> {
            self.ops
                .lock()
                .unwrap()
                .push(String::from_utf8_lossy(data).into_owned());
            Ok(())
        }
        async fn set_window_title(&mut self, title: &str) -> Result<(), Error> {
            self.ops.lock().unwrap().push(format!("title:{title}"));
            Ok(())
        }
        async fn set_cursor_position(&mut self, x: u16, y: u16) -> Result<(), Error> {
            self.ops.lock().unwrap().push(format!("cursor_pos:{x},{y}"));
            Ok(())
        }
        async fn set_cursor_style(&mut self, _: &crate::view::Cursor) -> Result<(), Error> {
            Ok(())
        }
        fn size(&self) -> Result<(u16, u16), Error> {
            Ok((80, 24))
        }
    }

    #[tokio::test]
    async fn apply_view_toggles_alt_screen() {
        let mut terminal = RecordingTerminal::new(None).unwrap();
        let ops = terminal.ops.clone();
        let mut applied = AppliedViewState::default();

        let mut view = View::new("");
        view.alt_screen = true;
        apply_view(
            &mut terminal,
            &mut applied,
            AppliedViewState::from_view(&view),
        )
        .await
        .unwrap();
        assert!(applied.alt_screen);
        assert!(ops.lock().unwrap().contains(&"alt_on".to_string()));

        view.alt_screen = false;
        apply_view(
            &mut terminal,
            &mut applied,
            AppliedViewState::from_view(&view),
        )
        .await
        .unwrap();
        assert!(!applied.alt_screen);
        assert!(ops.lock().unwrap().contains(&"alt_off".to_string()));
    }

    #[tokio::test]
    async fn apply_view_sets_cursor() {
        let mut terminal = RecordingTerminal::new(None).unwrap();
        let ops = terminal.ops.clone();
        let mut applied = AppliedViewState::default();

        let mut view = View::new("");
        view.cursor = Some(Cursor::new(Position::new(3, 4)));
        apply_view(
            &mut terminal,
            &mut applied,
            AppliedViewState::from_view(&view),
        )
        .await
        .unwrap();
        assert!(ops.lock().unwrap().contains(&"cursor_show".to_string()));
        assert!(ops.lock().unwrap().contains(&"cursor_pos:3,4".to_string()));
    }

    #[tokio::test]
    async fn apply_view_sets_progress_bar() {
        let mut terminal = RecordingTerminal::new(None).unwrap();
        let ops = terminal.ops.clone();
        let mut applied = AppliedViewState::default();

        let mut view = View::new("");
        view.progress_bar = Some(ProgressBar::default_bar(25));
        apply_view(
            &mut terminal,
            &mut applied,
            AppliedViewState::from_view(&view),
        )
        .await
        .unwrap();
        assert!(ops.lock().unwrap().iter().any(|s| s.contains("9;4;1;25")));
    }

    #[test]
    fn progress_bar_none_is_reset() {
        let seq = progress_bar_sequence(Some(&ProgressBar {
            state: ProgressBarState::None,
            value: 0,
        }));
        assert!(seq.contains("9;4;0"));
    }
}
