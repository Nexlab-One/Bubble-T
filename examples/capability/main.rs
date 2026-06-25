//! Terminal capability query demo (XTGETTCAP).

use bubble_t::{
    CapabilityMsg, Cmd, Model, Msg, Program, View, legacy_key_msg, quit, request_capability,
};
use bubble_t_widgets::textinput::{Model as TextInput, new as new_textinput};
use lipgloss_extras::lipgloss::Style;

struct App {
    input: TextInput,
    width: u16,
}

impl Model for App {
    fn init() -> (Self, Option<Cmd>) {
        let mut input = new_textinput();
        input.set_placeholder("Enter capability name (TN, RGB, cols, …)");
        let focus_cmd = input.focus();
        (Self { input, width: 60 }, Some(focus_cmd))
    }

    fn update(&mut self, msg: Msg) -> Option<Cmd> {
        if let Some(size) = msg.downcast_ref::<bubble_t::WindowSizeMsg>() {
            self.width = size.width;
        }

        if let Some(cap) = msg.downcast_ref::<CapabilityMsg>() {
            eprintln!("Got capability: {}", cap.content);
        }

        if let Some(key) = legacy_key_msg(&msg) {
            match key.key {
                crossterm::event::KeyCode::Esc | crossterm::event::KeyCode::Char('c')
                    if key
                        .modifiers
                        .contains(crossterm::event::KeyModifiers::CONTROL) =>
                {
                    return Some(quit());
                }
                crossterm::event::KeyCode::Enter => {
                    let value = self.input.value().to_string();
                    self.input.reset();
                    if !value.is_empty() {
                        return Some(request_capability(value));
                    }
                }
                _ => {}
            }
        }

        self.input.update(msg);
        None
    }

    fn view(&self) -> View {
        let w = self.width.min(60) as i32;
        let instructions = Style::new().width(w).render(
            "Query terminal capabilities. Try TN, RGB, or cols.\n\
             Press Enter to request, ctrl+c to quit.",
        );

        View::new(format!("\n{instructions}\n\n{}\n", self.input.view()))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let program = Program::<App>::builder().build()?;
    program.run().await?;
    Ok(())
}
