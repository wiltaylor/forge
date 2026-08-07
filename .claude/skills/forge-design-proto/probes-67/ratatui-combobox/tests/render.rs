use probe_ratatui::combobox::{ComboBox, ComboBoxItem, ComboBoxState};
use probe_ratatui::theme::Theme;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::Terminal;

fn items() -> Vec<ComboBoxItem> {
    vec![
        ComboBoxItem::new("us-east-1", "us-east-1 · N. Virginia"),
        ComboBoxItem::new("us-west-2", "us-west-2 · Oregon"),
        ComboBoxItem::new("eu-west-2", "eu-west-2 · London"),
        ComboBoxItem::new("ap-southeast-2", "ap-southeast-2 · Sydney"),
        ComboBoxItem::new("sa-east-1", "sa-east-1 · Sao Paulo").disabled(true),
    ]
}

fn dump(label: &str, state: &mut ComboBoxState, it: &[ComboBoxItem]) {
    let t = Theme::dark();
    let mut term = Terminal::new(TestBackend::new(46, 9)).unwrap();
    term.draw(|f| {
        let a = Rect::new(0, 0, 44, 8);
        f.render_stateful_widget(ComboBox::new(it, &t).focused(true), a, state);
    })
    .unwrap();
    let buf = term.backend().buffer().clone();
    println!("--- {label}");
    for y in 0..buf.area.height {
        let mut line = String::new();
        for x in 0..buf.area.width {
            line.push_str(buf[(x, y)].symbol());
        }
        println!("|{}|", line);
    }
}

#[test]
fn shots() {
    let it = items();
    let mut s = ComboBoxState::new();
    dump("closed at open", &mut s, &it);
    s.focus(&it);
    dump("focused/open", &mut s, &it);
    for c in "sy".chars() {
        s.handle_key(KeyEvent::from(KeyCode::Char(c)), &it);
    }
    dump("typed 'sy'", &mut s, &it);
    for c in "zzz".chars() {
        s.handle_key(KeyEvent::from(KeyCode::Char(c)), &it);
    }
    dump("no match", &mut s, &it);
    panic!("show");
}
