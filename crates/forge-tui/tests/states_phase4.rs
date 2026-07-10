#![cfg(feature = "full")]

use forge_tui::event::Outcome;
use forge_tui::theme::Theme;
use forge_tui::widgets::*;
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::widgets::{StatefulWidget, Widget};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn buffer_text(buf: &Buffer) -> String {
    let area = buf.area;
    (area.y..area.y + area.height)
        .map(|y| {
            (area.x..area.x + area.width)
                .map(|x| buf[(x, y)].symbol().to_string())
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn markdown_renders_structure() {
    let t = Theme::dark();
    let src = "# Title\n\nSome **bold** and `code`.\n\n- one\n- two\n\n> quoted";
    let lines = markdown_lines(src, 40, &t);
    let text: Vec<String> = lines
        .iter()
        .map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect::<String>())
        .collect();
    let joined = text.join("\n");
    assert!(joined.contains("Title"));
    assert!(joined.contains("• one"));
    assert!(joined.contains("▎ quoted"));
    // Heading is bold+underlined.
    let title_line = lines.iter().find(|l| {
        l.spans.iter().any(|s| s.content.contains("Title"))
    }).unwrap();
    let style = title_line.spans.iter().find(|s| s.content.contains("Title")).unwrap().style;
    assert!(style.add_modifier.contains(Modifier::BOLD | Modifier::UNDERLINED));
}

#[test]
fn markdown_wraps_to_width() {
    let t = Theme::dark();
    let src = "one two three four five six seven eight nine ten";
    let lines = markdown_lines(src, 20, &t);
    assert!(lines.len() > 2, "long paragraph must wrap");
    for l in &lines {
        let w: usize = l.spans.iter().map(|s| forge_tui::text::width(&s.content)).sum();
        assert!(w <= 20, "line overflows: {w}");
    }
}

#[test]
fn composer_enter_sends_alt_enter_newlines() {
    let mut c = ComposerState::new();
    for ch in "hi".chars() {
        let _ = c.handle_key(key(KeyCode::Char(ch)));
    }
    let alt_enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::ALT);
    assert_eq!(c.handle_key(alt_enter), Outcome::Changed);
    for ch in "there".chars() {
        let _ = c.handle_key(key(KeyCode::Char(ch)));
    }
    assert_eq!(c.handle_key(key(KeyCode::Enter)), Outcome::Submitted);
    assert_eq!(c.take_message(), "hi\nthere");
    assert_eq!(c.input.value(), "");
    // Empty drafts don't submit.
    assert_eq!(c.handle_key(key(KeyCode::Enter)), Outcome::Consumed);
}

#[test]
fn chat_view_follows_tail() {
    let t = Theme::dark();
    let items: Vec<ChatItem> = (0..10)
        .map(|i| ChatItem::user(format!("message number {i}")))
        .collect();
    let mut state = ChatViewState::new();
    let area = Rect::new(0, 0, 40, 6);
    let mut buf = Buffer::empty(area);
    ChatView::new(&items).theme(&t).render(area, &mut buf, &mut state);
    assert!(buffer_text(&buf).contains("message number 9"));
    let _ = state.handle_key(key(KeyCode::Up));
    assert!(!state.follow);
}

#[test]
fn chat_prompt_selects_and_submits() {
    let t = Theme::dark();
    let mut state = ChatPromptState::new();
    let mut buf = Buffer::empty(Rect::new(0, 0, 40, 2));
    ChatPrompt::new("Q?", &["a", "b", "c"]).theme(&t).render(
        Rect::new(0, 0, 40, 2),
        &mut buf,
        &mut state,
    );
    let _ = state.handle_key(key(KeyCode::Right));
    let _ = state.handle_key(key(KeyCode::Right));
    let _ = state.handle_key(key(KeyCode::Right));
    assert_eq!(state.selected, 2, "clamped at the last option");
    assert_eq!(state.handle_key(key(KeyCode::Enter)), Outcome::Submitted);
}

#[test]
fn code_view_highlights_and_marks() {
    let t = Theme::dark();
    let src = "fn main() {\n    let x = \"hi\";\n}\n";
    let mut state = CodeViewState::new();
    let area = Rect::new(0, 0, 40, 5);
    let mut buf = Buffer::empty(area);
    let marks = [(1usize, forge_tui::theme::Severity::Warning)];
    CodeView::new(src, "rs").marks(&marks).theme(&t).render(area, &mut buf, &mut state);
    let text = buffer_text(&buf);
    assert!(text.contains("fn main"));
    assert!(text.contains("1"), "line numbers present");
    assert!(text.contains("▎"), "gutter mark present");
    // The `fn` keyword picks up the accent-fg mapping (truecolor theme).
    let mut found_keyword_color = false;
    for x in 0..40u16 {
        for y in 0..5u16 {
            if buf[(x, y)].style().fg == Some(t.accent.fg) {
                found_keyword_color = true;
            }
        }
    }
    assert!(found_keyword_color, "syntax highlighting produced accent keywords");
}

#[test]
fn diff_view_marks_adds_and_dels() {
    let t = Theme::dark();
    let old = "a\nb\nc\n";
    let new = "a\nX\nc\nd\n";
    let mut state = CodeViewState::new();
    let area = Rect::new(0, 0, 20, 8);
    let mut buf = Buffer::empty(area);
    DiffView::new(old, new).theme(&t).render(area, &mut buf, &mut state);
    let text = buffer_text(&buf);
    assert!(text.contains("- b"));
    assert!(text.contains("+ X"));
    assert!(text.contains("+ d"));
    assert!(text.contains("  a"), "context rows kept");
}

#[test]
fn flowchart_layers_and_arrows() {
    let t = Theme::dark();
    let nodes = [
        FlowNode::new("a", "alpha"),
        FlowNode::new("b", "beta"),
        FlowNode::new("c", "gamma"),
    ];
    let edges = [FlowEdge::new("a", "b"), FlowEdge::new("b", "c")];
    let area = Rect::new(0, 0, 60, 8);
    let mut buf = Buffer::empty(area);
    Flowchart::new(&nodes, &edges).theme(&t).render(area, &mut buf);
    let text = buffer_text(&buf);
    assert!(text.contains("alpha") && text.contains("beta") && text.contains("gamma"));
    assert!(text.contains("▸"), "edges end in arrowheads");
    // beta sits to the right of alpha (layered layout).
    let alpha_x = text.lines().find(|l| l.contains("alpha")).unwrap().find("alpha").unwrap();
    let beta_x = text.lines().find(|l| l.contains("beta")).unwrap().find("beta").unwrap();
    assert!(beta_x > alpha_x);
}

#[cfg(unix)]
#[test]
fn terminal_runs_a_real_pty() {
    use forge_tui::widgets::specialty::CommandBuilder;
    let mut cmd = CommandBuilder::new("/bin/sh");
    cmd.args(["-c", "printf forge-tui-pty-ok"]);
    let mut state = TerminalState::spawn(cmd, 6, 40).expect("pty spawn");
    // Wait for the output to arrive (drain on a simulated tick).
    let mut seen = false;
    for _ in 0..100 {
        state.drain();
        let t = Theme::dark();
        let area = Rect::new(0, 0, 40, 6);
        let mut buf = Buffer::empty(area);
        Terminal::new().theme(&t).render(area, &mut buf, &mut state);
        if buffer_text(&buf).contains("forge-tui-pty-ok") {
            seen = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(seen, "PTY output did not appear");
}
