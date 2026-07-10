use forge_tui::event::Outcome;
use forge_tui::runtime::{NavSection, ShellState};
use forge_tui::text::fuzzy_score;
use forge_tui::theme::Theme;
use forge_tui::widgets::*;
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::widgets::StatefulWidget;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn render_stateful<W: StatefulWidget>(widget: W, w: u16, h: u16, state: &mut W::State) -> Buffer {
    let mut buf = Buffer::empty(Rect::new(0, 0, w, h));
    widget.render(Rect::new(0, 0, w, h), &mut buf, state);
    buf
}

#[test]
fn tabs_navigate_and_number_jump() {
    let mut state = TabsState::new(0);
    let t = Theme::dark();
    render_stateful(Tabs::new(&["a", "b", "c"]).theme(&t), 30, 2, &mut state);
    assert_eq!(state.handle_key(key(KeyCode::Right)), Outcome::Changed);
    assert_eq!(state.selected, 1);
    assert_eq!(state.handle_key(key(KeyCode::Char('3'))), Outcome::Changed);
    assert_eq!(state.selected, 2);
    assert_eq!(state.handle_key(key(KeyCode::Right)), Outcome::Consumed); // clamped
    assert_eq!(state.handle_key(key(KeyCode::Char('9'))), Outcome::Consumed); // out of range
}

#[test]
fn pagination_clamps_and_jumps() {
    let mut p = PaginationState::new(0, 10);
    assert_eq!(p.handle_key(key(KeyCode::Left)), Outcome::Consumed);
    assert_eq!(p.handle_key(key(KeyCode::Right)), Outcome::Changed);
    assert_eq!(p.page, 1);
    assert_eq!(p.handle_key(key(KeyCode::End)), Outcome::Changed);
    assert_eq!(p.page, 9);
    assert_eq!(p.handle_key(key(KeyCode::Home)), Outcome::Changed);
    assert_eq!(p.page, 0);
}

#[test]
fn split_resizes_within_bounds() {
    let mut s = SplitState::new(0.5);
    assert_eq!(s.handle_key(key(KeyCode::Left)), Outcome::Changed);
    assert!((s.ratio - 0.45).abs() < 1e-9);
    for _ in 0..20 {
        let _ = s.handle_key(key(KeyCode::Left));
    }
    assert!(s.ratio >= 0.05 - 1e-9);
    assert_eq!(s.handle_key(key(KeyCode::Left)), Outcome::Consumed); // pinned at min
}

#[test]
fn listbox_selection_modes() {
    let t = Theme::dark();
    let items = ["a", "b", "c", "d"];
    let mut single = ListBoxState::new();
    render_stateful(ListBox::new(&items).theme(&t), 20, 4, &mut single);
    let _ = single.handle_key(key(KeyCode::Down));
    assert_eq!(single.handle_key(key(KeyCode::Char(' '))), Outcome::Changed);
    let _ = single.handle_key(key(KeyCode::Down));
    let _ = single.handle_key(key(KeyCode::Char(' ')));
    assert_eq!(single.selected().len(), 1, "single-select must replace");
    assert_eq!(single.selected_one(), Some(2));

    let mut multi = ListBoxState::multi();
    render_stateful(ListBox::new(&items).theme(&t), 20, 4, &mut multi);
    let _ = multi.handle_key(key(KeyCode::Char(' ')));
    let _ = multi.handle_key(key(KeyCode::Down));
    let _ = multi.handle_key(key(KeyCode::Char(' ')));
    assert_eq!(multi.selected().len(), 2);
    // Toggle off.
    let _ = multi.handle_key(key(KeyCode::Char(' ')));
    assert_eq!(multi.selected().len(), 1);
}

#[test]
fn listbox_scrolls_to_cursor() {
    let t = Theme::dark();
    let items: Vec<String> = (0..20).map(|i| format!("item-{i}")).collect();
    let refs: Vec<&str> = items.iter().map(String::as_str).collect();
    let mut state = ListBoxState::new();
    render_stateful(ListBox::new(&refs).theme(&t), 20, 5, &mut state);
    let _ = state.handle_key(key(KeyCode::End));
    assert_eq!(state.highlight, 19);
    let buf = render_stateful(ListBox::new(&refs).theme(&t), 20, 5, &mut state);
    let last_row: String = (0..20u16).map(|x| buf[(x, 4)].symbol()).collect();
    assert!(last_row.contains("item-19"), "viewport did not follow: {last_row:?}");
}

#[test]
fn select_open_choose_commit() {
    let mut s = SelectState::new();
    assert_eq!(s.handle_key(key(KeyCode::Enter)), Outcome::Consumed);
    assert!(s.open);
    // Needs a render for the list to learn its length.
    let t = Theme::dark();
    let mut buf = Buffer::empty(Rect::new(0, 0, 24, 10));
    Select::new(&["dev", "staging", "prod"]).theme(&t).render(
        Rect::new(0, 0, 24, 1),
        &mut buf,
        &mut s,
    );
    let _ = s.handle_key(key(KeyCode::Down));
    assert_eq!(s.handle_key(key(KeyCode::Enter)), Outcome::Changed);
    assert!(!s.open);
    assert_eq!(s.value, Some(1));
    // Esc while open just closes.
    let _ = s.handle_key(key(KeyCode::Char(' ')));
    assert!(s.open);
    assert_eq!(s.handle_key(key(KeyCode::Esc)), Outcome::Consumed);
    assert!(!s.open);
    assert_eq!(s.value, Some(1));
}

#[test]
fn fuzzy_scoring_prefers_word_starts_and_runs() {
    assert!(fuzzy_score("fc", "forge-core").is_some());
    assert!(fuzzy_score("xyz", "forge-core").is_none());
    // Word-start match beats scattered match.
    let a = fuzzy_score("fs", "forge-server").unwrap();
    let b = fuzzy_score("fs", "puffiest").unwrap();
    assert!(a > b, "word-start should score higher: {a} vs {b}");
    // Empty needle matches everything.
    assert_eq!(fuzzy_score("", "anything"), Some(0));
}

#[test]
fn combobox_filters_and_submits() {
    let items = ["nixos/24.11", "debian/12", "arch/rolling"];
    let mut s = ComboboxState::new();
    for c in "deb".chars() {
        let _ = s.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE), &items);
    }
    assert!(s.open);
    assert_eq!(s.matches(), &[1]);
    assert_eq!(s.handle_key(key(KeyCode::Enter), &items), Outcome::Submitted);
    assert_eq!(s.input.value(), "debian/12");
    assert!(!s.open);
}

#[test]
fn textarea_editing_and_line_joins() {
    let mut s = TextareaState::new();
    s.insert_str("hello");
    assert_eq!(s.handle_key(key(KeyCode::Enter)), Outcome::Changed);
    s.insert_str("world");
    assert_eq!(s.value(), "hello\nworld");
    assert_eq!(s.line_count(), 2);
    // Join lines with backspace at column 0.
    let _ = s.handle_key(key(KeyCode::Home));
    assert_eq!(s.handle_key(key(KeyCode::Backspace)), Outcome::Changed);
    assert_eq!(s.value(), "helloworld");
    // Up/down keeps the desired column.
    s.set_value("long line here\nab\nanother long line");
    let _ = s.handle_key(KeyEvent::new(KeyCode::Home, KeyModifiers::CONTROL));
    let _ = s.handle_key(key(KeyCode::End)); // col 14 cells
    let _ = s.handle_key(key(KeyCode::Down)); // clamps to "ab"
    assert_eq!(s.cursor(), (1, 2));
    let _ = s.handle_key(key(KeyCode::Down)); // restores toward desired
    assert_eq!(s.cursor().0, 2);
    assert!(s.cursor().1 >= 14);
}

#[test]
fn menu_state_skips_and_submits() {
    let entries = [
        MenuEntry::Section("Node"),
        MenuEntry::item("Restart"),
        MenuEntry::item("Drain"),
        MenuEntry::Separator,
        MenuEntry::danger("Delete"),
    ];
    let t = Theme::dark();
    let mut state = MenuState::new();
    let mut buf = Buffer::empty(Rect::new(0, 0, 30, 10));
    DropdownMenu::new(&entries, Rect::new(0, 0, 1, 1)).theme(&t).render(
        Rect::new(0, 0, 30, 10),
        &mut buf,
        &mut state,
    );
    let _ = state.handle_key(key(KeyCode::Down));
    let _ = state.handle_key(key(KeyCode::Down));
    assert_eq!(state.highlight, 2); // "Delete" (selectable index)
    assert_eq!(state.handle_key(key(KeyCode::Down)), Outcome::Consumed); // clamped
    assert_eq!(state.handle_key(key(KeyCode::Enter)), Outcome::Submitted);
    assert_eq!(state.handle_key(key(KeyCode::Esc)), Outcome::Cancelled);
    // Mnemonic jumps + submits.
    assert_eq!(state.mnemonic(&entries, 'r'), Outcome::Submitted);
    assert_eq!(state.highlight, 0);
}

#[test]
fn palette_filters_and_navigates() {
    let commands = [
        Command::new("deploy", "Deploy to production"),
        Command::new("restart", "Restart event bus"),
        Command::new("logs", "Open logs"),
    ];
    let mut state = PaletteState::new();
    state.filter(&commands);
    assert_eq!(state.matches().len(), 3);
    for c in "restart".chars() {
        let _ = state.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE), &commands);
    }
    assert_eq!(state.highlighted(), Some(1));
    assert_eq!(state.handle_key(key(KeyCode::Enter), &commands), Outcome::Submitted);
    assert_eq!(state.handle_key(key(KeyCode::Esc), &commands), Outcome::Cancelled);
}

#[test]
fn shell_nav_and_collapse() {
    let t = Theme::dark();
    let sections = [
        NavSection::new(Some("A"), &["one", "two"]),
        NavSection::new(None, &["three"]),
    ];
    let mut state = ShellState::new();
    let mut buf = Buffer::empty(Rect::new(0, 0, 80, 20));
    forge_tui::runtime::AppShell::new("T", &sections)
        .status("hints")
        .theme(&t)
        .render(Rect::new(0, 0, 80, 20), &mut buf, &mut state);
    assert!(!state.content().is_empty());
    let _ = state.handle_key(key(KeyCode::Down));
    let _ = state.handle_key(key(KeyCode::Down));
    assert_eq!(state.selected, 2);
    assert_eq!(state.handle_key(key(KeyCode::Down)), Outcome::Consumed); // clamped across sections
    assert_eq!(state.handle_key(key(KeyCode::Enter)), Outcome::Submitted);
    // Ctrl+B toggles collapse.
    let ctrl_b = KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL);
    assert_eq!(state.handle_key(ctrl_b), Outcome::Changed);
    assert_eq!(state.collapsed, Some(true));
}

#[test]
fn toggle_group_and_slider() {
    let t = Theme::dark();
    let mut tg = ToggleGroupState::new(0);
    render_stateful(ToggleGroup::new(&["a", "b"]).theme(&t), 20, 1, &mut tg);
    assert_eq!(tg.handle_key(key(KeyCode::Right)), Outcome::Changed);
    assert_eq!(tg.handle_key(key(KeyCode::Right)), Outcome::Consumed);

    let mut sl = SliderState::new(5.0, 0.0, 10.0, 1.0);
    assert_eq!(sl.handle_key(key(KeyCode::Right)), Outcome::Changed);
    assert_eq!(sl.value, 6.0);
    let big = KeyEvent::new(KeyCode::Right, KeyModifiers::SHIFT);
    assert_eq!(sl.handle_key(big), Outcome::Changed);
    assert_eq!(sl.value, 10.0); // clamped to max
    assert_eq!(sl.handle_key(key(KeyCode::Right)), Outcome::Consumed);
}
