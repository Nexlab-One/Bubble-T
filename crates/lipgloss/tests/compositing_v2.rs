//! Compositing and v2 style tests.

use lipgloss::{Canvas, Compositor, Layer, Style};

#[test]
fn compositor_nested_z_order() {
    let back = Layer::new("aaa", vec![]).id("back").z(0);
    let mid = Layer::new(" b ", vec![]).id("mid").x(1).z(1);
    let front = Layer::new(" c", vec![]).id("front").x(2).y(0).z(2);
    let comp = Compositor::new(vec![back, mid, front]);
    let mut canvas = Canvas::new(6, 1);
    canvas.compose(&comp);
    let out = canvas.render();
    assert!(out.contains('a'));
    assert!(out.contains('c'));
}

#[test]
fn compositor_hit_respects_z_order() {
    let back = Layer::new("background", vec![]).id("back").z(0);
    let front = Layer::new("front", vec![]).id("front").x(2).y(1).z(1);
    let comp = Compositor::new(vec![back, front]);
    let hit = comp.hit(2, 1);
    assert_eq!(hit.id(), "front");
}

#[test]
fn canvas_compose_and_render() {
    let layer = Layer::new("hi", vec![]);
    let comp = Compositor::new(vec![layer]);
    let mut canvas = Canvas::new(10, 3);
    canvas.compose(&comp);
    let out = canvas.render();
    assert!(out.contains('h'));
}

#[test]
fn hyperlink_wraps_render_output() {
    let style = Style::new()
        .foreground(lipgloss::Color::from("9"))
        .hyperlink("https://example.com");
    let out = style.render("click me");
    assert!(out.contains("\x1b]8;"));
    assert!(out.contains("https://example.com"));
}

#[test]
fn underline_style_emits_sgr_variant() {
    let style = Style::new()
        .underline_style(ansi::style::Underline::Curly)
        .render("x");
    assert!(style.contains("4:3"));
}
