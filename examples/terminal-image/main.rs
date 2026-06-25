//! Terminal image demo — Sixel and Kitty inline graphics in a Bubble-T view.
//!
//! Environment:
//! - `IMAGE_PROTOCOL=kitty` — use Kitty virtual placement instead of Sixel
//! - `IMAGE_TRANSMISSION=shm` — use Kitty shared-memory transmission (`t=s`);
//!   falls back to direct when SHM is unavailable (remote SSH, unsupported OS)

use ansi::sixel::Encoder as SixelEncoder;
use bubble_t::{Cmd, KeyMsg, Model, Msg, Program, View, quit};
use cellbuf::{
    Buffer, ImageArea, ImagePlacementConfig, ImageProtocol, place_image_with_config,
    render_from_home,
};
use crossterm::event::{KeyCode, KeyModifiers};
use image::{Rgba, RgbaImage};

struct ImageModel {
    protocol: ImageProtocol,
    placement: ImagePlacementConfig,
}

impl Model for ImageModel {
    fn init() -> (Self, Option<Cmd>) {
        let protocol = match std::env::var("IMAGE_PROTOCOL").as_deref() {
            Ok("kitty") => ImageProtocol::Kitty,
            _ => ImageProtocol::Sixel,
        };
        let placement = ImagePlacementConfig::from_env();
        (
            Self {
                protocol,
                placement,
            },
            None,
        )
    }

    fn update(&mut self, msg: Msg) -> Option<Cmd> {
        if let Some(key) = msg.downcast_ref::<KeyMsg>() {
            match key.key {
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    return Some(quit());
                }
                KeyCode::Char('q') | KeyCode::Esc => return Some(quit()),
                _ => {}
            }
        }
        None
    }

    fn view(&self) -> View {
        let (cols, rows) = (40, 12);
        let img = demo_image();
        let mut buf = Buffer::new(cols, rows);
        let area = ImageArea::new(4, 2, 20, 8);
        let prefix =
            place_image_with_config(&mut buf, &img, area, self.protocol, 31, self.placement)
                .unwrap_or_default();
        let body = render_from_home(&Buffer::new(cols, rows), &buf);
        let transmission = match std::env::var("IMAGE_TRANSMISSION").as_deref() {
            Ok(v) if !v.is_empty() => v.to_string(),
            _ => "direct".to_string(),
        };
        View::new(format!(
            "{prefix}{body}\n  terminal-image ({:?}, tx={transmission}) — q to quit",
            self.protocol
        ))
    }
}

fn demo_image() -> RgbaImage {
    let mut img = RgbaImage::new(40, 40);
    for (x, y, px) in img.enumerate_pixels_mut() {
        *px = Rgba([(x * 6) as u8, (y * 6) as u8, 128, 255]);
    }
    img
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!(
        "sixel payload bytes: {}",
        SixelEncoder.encode(&demo_image()).len()
    );
    if std::env::var("IMAGE_PROTOCOL").as_deref() == Ok("kitty") {
        eprintln!(
            "kitty transmission: {}",
            std::env::var("IMAGE_TRANSMISSION").unwrap_or_else(|_| "direct".into())
        );
    }
    let program = Program::<ImageModel>::builder().build()?;
    program.run().await?;
    Ok(())
}
