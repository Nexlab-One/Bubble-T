//! Cursor color and terminal color query demo.

use bubble_t::{
    Cmd, CursorColorMsg, ForegroundColorMsg, Model, Msg, Program, View, legacy_key_msg, quit,
    request_cursor_color, request_foreground_color,
};
use bubble_t_widgets::key::{Binding, new_binding, with_help, with_keys_str};
use lipgloss_extras::lipgloss::{Color, Style};

#[derive(Debug)]
struct App {
    cursor_color: Option<String>,
    fg_color: Option<String>,
    keys: Binding,
}

impl Model for App {
    fn init() -> (Self, Option<Cmd>) {
        (
            Self {
                cursor_color: None,
                fg_color: None,
                keys: new_binding(vec![
                    with_keys_str(&["ctrl+c", "q"]),
                    with_help("ctrl+c/q", "quit"),
                ]),
            },
            Some(request_foreground_color()),
        )
    }

    fn update(&mut self, msg: Msg) -> Option<Cmd> {
        if let Some(c) = msg.downcast_ref::<ForegroundColorMsg>() {
            self.fg_color = Some(format!("#{:02x}{:02x}{:02x}", c.0.r, c.0.g, c.0.b));
            return Some(request_cursor_color());
        }

        if let Some(c) = msg.downcast_ref::<CursorColorMsg>() {
            self.cursor_color = Some(format!("#{:02x}{:02x}{:02x}", c.0.r, c.0.g, c.0.b));
        }

        if let Some(key) = legacy_key_msg(&msg)
            && self.keys.matches(&key)
        {
            return Some(quit());
        }

        None
    }

    fn view(&self) -> View {
        let fg = self.fg_color.as_deref().unwrap_or("unknown");
        let cursor = self.cursor_color.as_deref().unwrap_or("unknown");

        let body = Style::new().foreground(Color::from("86")).render(&format!(
            "Terminal foreground: {fg}\nTerminal cursor color: {cursor}\n\n\
                 Move the cursor below and press ctrl+c to quit."
        ));

        let mut view = View::new(body);
        view.cursor = Some(bubble_t::Cursor::new(bubble_t::Position::new(0, 3)));
        view
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let program = Program::<App>::builder().build()?;
    program.run().await?;
    Ok(())
}
