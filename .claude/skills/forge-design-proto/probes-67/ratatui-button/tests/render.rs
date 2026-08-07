use probe_ratatui::forge::{Button, Theme, Variant};
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::Terminal;

#[test]
fn render_row() {
    let t = Theme::dark();
    let mut term = Terminal::new(TestBackend::new(60, 3)).unwrap();
    term.draw(|f| {
        let a = f.area();
        f.render_widget(
            Button::new("Deploy").variant(Variant::Primary).focused(true).theme(&t),
            Rect::new(a.x, a.y, 12, 1),
        );
        f.render_widget(
            Button::new("Dry run").theme(&t).disabled(true),
            Rect::new(a.x + 13, a.y, 12, 1),
        );
        f.render_widget(
            Button::new("Cancel deployment").variant(Variant::Danger).theme(&t),
            Rect::new(a.x + 26, a.y, 22, 1),
        );
    })
    .unwrap();
    let buf = term.backend().buffer().clone();
    for y in 0..buf.area.height {
        let mut line = String::new();
        for x in 0..buf.area.width {
            line.push_str(buf[(x, y)].symbol());
        }
        println!("|{}|", line);
    }
    for y in 0..1u16 {
        for x in 0..buf.area.width {
            let c = &buf[(x, y)];
            print!("{}", if c.modifier.contains(ratatui::style::Modifier::REVERSED) { 'R' } else if c.modifier.contains(ratatui::style::Modifier::BOLD) { 'B' } else { '.' });
        }
        println!();
    }
    panic!("show output");
}
