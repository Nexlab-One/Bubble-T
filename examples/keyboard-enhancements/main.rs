//! Keyboard enhancements demo (Kitty protocol + modifyOtherKeys).

use bubble_t::{Cmd, KeyboardEnhancementsMsg, Model, Msg, Program, View, quit};
use bubble_t_widgets::key::{Binding, new_binding, with_help, with_keys_str};
use lipgloss_extras::lipgloss::{Color, Style};

#[derive(Debug)]
struct App {
    supports_disambiguation: bool,
    supports_event_types: bool,
    keys: Binding,
}

impl Model for App {
    fn init() -> (Self, Option<Cmd>) {
        (
            Self {
                supports_disambiguation: false,
                supports_event_types: false,
                keys: new_binding(vec![
                    with_keys_str(&["ctrl+c", "q"]),
                    with_help("ctrl+c/q", "quit"),
                ]),
            },
            None,
        )
    }

    fn update(&mut self, msg: Msg) -> Option<Cmd> {
        if let Some(k) = msg.downcast_ref::<KeyboardEnhancementsMsg>() {
            self.supports_disambiguation = k.supports_key_disambiguation();
            self.supports_event_types = k.supports_event_types();
            return None;
        }

        if let Some(k) = msg.downcast_ref::<bubble_t::KeyPressMsg>() {
            if self.keys.matches(&k.to_legacy()) {
                return Some(quit());
            }
            eprintln!("  press: {}", k.string());
        }

        if let Some(k) = msg.downcast_ref::<bubble_t::KeyReleaseMsg>() {
            eprintln!("release: {}", k.string());
        }

        None
    }

    fn view(&self) -> View {
        let mut content = format!(
            "Terminal supports key disambiguation: {}\n\
             Terminal supports key releases: {}\n\n\
             This demo logs key events to stderr. Press ctrl+c to quit.\n",
            self.supports_disambiguation, self.supports_event_types
        );

        let style = Style::new().foreground(Color::from("245"));
        content = style.render(&content);

        let mut view = View::new(content);
        view.keyboard_enhancements.report_event_types = true;
        view
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let program = Program::<App>::builder().build()?;
    program.run().await?;
    Ok(())
}
