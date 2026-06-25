//! Text Input Example (Rust, using bubble-t-widgets)
//!
//! Port of Bubble Tea's `textinput` example using `bubble-t-widgets::textinput`.

use bubble_t::{Cmd, KeyMsg, Model, Msg, Program, View, quit};
use bubble_t_widgets::key::{Binding, new_binding, with_help, with_keys_str};
use bubble_t_widgets::textinput;

/// Key bindings for the textinput example
#[derive(Debug)]
pub struct KeyBindings {
    pub quit: Binding,
    pub quit_alt: Binding,
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            quit: new_binding(vec![
                with_keys_str(&["enter", "esc"]),
                with_help("enter/esc", "quit"),
            ]),
            quit_alt: new_binding(vec![
                with_keys_str(&["ctrl+c"]),
                with_help("ctrl+c", "quit"),
            ]),
        }
    }
}

pub struct TextInputModel {
    text_input: textinput::Model,
    quitting: bool,
    keys: KeyBindings,
}

impl TextInputModel {
    fn new() -> Self {
        let mut ti = textinput::new();
        ti.set_placeholder("Pikachu");
        ti.set_char_limit(156);
        ti.set_width(20);
        Self {
            text_input: ti,
            quitting: false,
            keys: KeyBindings::default(),
        }
    }
}

impl Model for TextInputModel {
    fn init() -> (Self, Option<Cmd>) {
        let mut model = Self::new();
        let cmd = model.text_input.focus();
        (model, Some(cmd))
    }

    fn update(&mut self, msg: Msg) -> Option<Cmd> {
        // Handle quit keys first
        if let Some(key_msg) = msg.downcast_ref::<KeyMsg>()
            && (self.keys.quit.matches(key_msg) || self.keys.quit_alt.matches(key_msg))
        {
            self.quitting = true;
            return Some(quit());
        }

        self.text_input.update(msg)
    }

    fn view(&self) -> View {
        if self.quitting {
            return View::new("");
        }

        View::new(format!(
            "What’s your favorite Pokémon?\n\n{}\n\n(esc to quit)\n",
            self.text_input.view()
        ))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let program = Program::<TextInputModel>::builder()
        .signal_handler(true)
        .build()?;

    let _ = program.run().await?;
    Ok(())
}
