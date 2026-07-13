use forge_tui::event::Outcome;
use forge_tui::runtime::{AppShell, NavSection, ShellState};
use forge_tui::theme::Theme;
use forge_tui::widgets::*;
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;
use ratatui::widgets::StatefulWidget;

fn click(x: u16, y: u16) -> MouseEvent {
    MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: x,
        row: y,
        modifiers: KeyModifiers::NONE,
    }
}

fn wheel(x: u16, y: u16, down: bool) -> MouseEvent {
    MouseEvent {
        kind: if down {
            MouseEventKind::ScrollDown
        } else {
            MouseEventKind::ScrollUp
        },
        column: x,
        row: y,
        modifiers: KeyModifiers::NONE,
    }
}

fn hover(x: u16, y: u16) -> MouseEvent {
    MouseEvent {
        kind: MouseEventKind::Moved,
        column: x,
        row: y,
        modifiers: KeyModifiers::NONE,
    }
}

#[test]
fn checkbox_and_toggle_click() {
    let t = Theme::dark();
    let mut cb = CheckboxState::new(false);
    let mut buf = Buffer::empty(Rect::new(0, 0, 20, 1));
    Checkbox::new("agree")
        .theme(&t)
        .render(Rect::new(0, 0, 20, 1), &mut buf, &mut cb);
    assert_eq!(cb.handle_mouse(&click(2, 0)), Outcome::Changed);
    assert!(cb.checked);
    // Outside the control: ignored.
    assert_eq!(cb.handle_mouse(&click(5, 3)), Outcome::Ignored);
}

#[test]
fn input_click_places_cursor() {
    let t = Theme::dark();
    let mut s = InputState::with_value("hello world");
    let mut buf = Buffer::empty(Rect::new(0, 0, 20, 1));
    Input::new()
        .focused(true)
        .theme(&t)
        .render(Rect::new(0, 0, 20, 1), &mut buf, &mut s);
    // Text region starts at x=1 (after the edge bar); click on the 'w'.
    assert_eq!(s.handle_mouse(&click(7, 0)), Outcome::Consumed);
    assert_eq!(s.cursor(), 6);
}

#[test]
fn listbox_click_selects_and_wheel_moves() {
    let t = Theme::dark();
    let items = ["a", "b", "c", "d"];
    let mut s = ListBoxState::new();
    let mut buf = Buffer::empty(Rect::new(0, 0, 20, 4));
    ListBox::new(&items)
        .theme(&t)
        .render(Rect::new(0, 0, 20, 4), &mut buf, &mut s);
    assert_eq!(s.handle_mouse(&click(3, 2)), Outcome::Changed);
    assert_eq!(s.selected_one(), Some(2));
    assert_eq!(s.highlight, 2);
    let _ = s.handle_mouse(&wheel(3, 1, true));
    assert_eq!(s.highlight, 3);
}

#[test]
fn table_header_click_sorts_row_click_moves_cursor() {
    let t = Theme::dark();
    let columns = [Column::new("name"), Column::new("cpu").width(5)];
    let rows: Vec<Vec<&str>> = vec![vec!["a", "1"], vec!["b", "2"], vec!["c", "3"]];
    let mut s = TableState::new();
    let mut buf = Buffer::empty(Rect::new(0, 0, 30, 5));
    Table::new(&columns, &rows)
        .theme(&t)
        .render(Rect::new(0, 0, 30, 5), &mut buf, &mut s);
    // Header click on column 0: asc, then desc.
    assert_eq!(s.handle_mouse(&click(2, 0)), Outcome::Changed);
    assert_eq!(s.sort, Some((0, true)));
    let _ = s.handle_mouse(&click(2, 0));
    assert_eq!(s.sort, Some((0, false)));
    // Row click.
    assert_eq!(s.handle_mouse(&click(2, 3)), Outcome::Consumed);
    assert_eq!(s.cursor, 2);
}

#[test]
fn tabs_and_pagination_click() {
    let t = Theme::dark();
    let mut tabs = TabsState::new(0);
    let mut buf = Buffer::empty(Rect::new(0, 0, 40, 2));
    Tabs::new(&["one", "two", "three"]).theme(&t).render(
        Rect::new(0, 0, 40, 2),
        &mut buf,
        &mut tabs,
    );
    // "two" starts at x = 3 + 3 = 6.
    assert_eq!(tabs.handle_mouse(&click(7, 0)), Outcome::Changed);
    assert_eq!(tabs.selected, 1);

    let mut pages = PaginationState::new(0, 5);
    let mut buf = Buffer::empty(Rect::new(0, 0, 40, 1));
    Pagination::new()
        .theme(&t)
        .render(Rect::new(0, 0, 40, 1), &mut buf, &mut pages);
    // '›' next arrow.
    let text: String = (0..40u16).map(|x| buf[(x, 0)].symbol()).collect();
    // chars().position — the buffer is one symbol per cell (byte offsets lie).
    let next_x = text.chars().position(|c| c == '›').unwrap() as u16;
    assert_eq!(pages.handle_mouse(&click(next_x, 0)), Outcome::Changed);
    assert_eq!(pages.page, 1);
}

#[test]
fn slider_click_snaps_to_step() {
    let t = Theme::dark();
    let mut s = SliderState::new(0.0, 0.0, 10.0, 1.0);
    let mut buf = Buffer::empty(Rect::new(0, 0, 25, 1));
    Slider::new()
        .show_value(false)
        .theme(&t)
        .render(Rect::new(0, 0, 25, 1), &mut buf, &mut s);
    // Click the far right end of the track.
    assert_eq!(s.handle_mouse(&click(24, 0)), Outcome::Changed);
    assert_eq!(s.value, 10.0);
    // Click the middle.
    let _ = s.handle_mouse(&click(12, 0));
    assert_eq!(s.value, 5.0);
}

#[test]
fn select_click_open_choose_and_click_away() {
    let t = Theme::dark();
    let items = ["dev", "staging", "prod"];
    let mut s = SelectState::new();
    let area = Rect::new(0, 0, 24, 12);
    let field = Rect::new(0, 0, 24, 1);
    let mut buf = Buffer::empty(area);
    Select::new(&items)
        .theme(&t)
        .render(field, &mut buf, &mut s);
    // Click the field: opens.
    assert_eq!(s.handle_mouse(&click(3, 0)), Outcome::Consumed);
    assert!(s.open);
    // Render the open popup so the list learns its rects (rows at y=2..).
    Select::new(&items)
        .theme(&t)
        .render(field, &mut buf, &mut s);
    // Click "staging" (row 1 of the list, at popup inner y = 2 + 1).
    assert_eq!(s.handle_mouse(&click(3, 3)), Outcome::Changed);
    assert!(!s.open);
    assert_eq!(s.value, Some(1));
    // Reopen and click away.
    let _ = s.handle_mouse(&click(3, 0));
    Select::new(&items)
        .theme(&t)
        .render(field, &mut buf, &mut s);
    let _ = s.handle_mouse(&click(23, 11));
    assert!(!s.open);
    assert_eq!(s.value, Some(1), "click-away must not change the value");
}

#[test]
fn tree_click_moves_then_toggles() {
    const KIDS: [TreeNode<'static>; 1] = [TreeNode {
        label: "leaf",
        children: &[],
    }];
    let roots = [TreeNode::branch("root", &KIDS)];
    let t = Theme::dark();
    let mut s = TreeState::new();
    let mut buf = Buffer::empty(Rect::new(0, 0, 20, 5));
    Tree::new(&roots)
        .theme(&t)
        .render(Rect::new(0, 0, 20, 5), &mut buf, &mut s);
    // First click on the root row: cursor is already there → toggles open.
    assert_eq!(s.handle_mouse(&click(3, 0), &roots), Outcome::Changed);
    assert!(s.is_expanded(&[0]));
    Tree::new(&roots)
        .theme(&t)
        .render(Rect::new(0, 0, 20, 5), &mut buf, &mut s);
    // Click the leaf row: moves cursor. Second click: submits.
    assert_eq!(s.handle_mouse(&click(4, 1), &roots), Outcome::Consumed);
    assert_eq!(s.cursor, 1);
    assert_eq!(s.handle_mouse(&click(4, 1), &roots), Outcome::Submitted);
}

#[test]
fn menu_hover_highlights_click_submits_away_cancels() {
    let entries = [
        MenuEntry::item("Restart"),
        MenuEntry::item("Drain"),
        MenuEntry::danger("Delete"),
    ];
    let t = Theme::dark();
    let mut s = MenuState::new();
    let area = Rect::new(0, 0, 40, 12);
    let mut buf = Buffer::empty(area);
    DropdownMenu::new(&entries, Rect::new(0, 0, 1, 1))
        .theme(&t)
        .render(area, &mut buf, &mut s);
    // Panel sits below the anchor at y=1; items at inner y = 2,3,4.
    assert_eq!(s.handle_mouse(&hover(4, 3)), Outcome::Consumed);
    assert_eq!(s.highlight, 1);
    assert_eq!(s.handle_mouse(&click(4, 4)), Outcome::Submitted);
    assert_eq!(s.highlight, 2);
    assert_eq!(s.handle_mouse(&click(39, 11)), Outcome::Cancelled);
}

#[test]
fn shell_nav_click_switches_section() {
    let t = Theme::dark();
    let sections = [NavSection::new(Some("A"), &["one", "two", "three"])];
    let mut s = ShellState::new();
    let mut buf = Buffer::empty(Rect::new(0, 0, 80, 20));
    AppShell::new("T", &sections)
        .subtitle("sub")
        .theme(&t)
        .render(Rect::new(0, 0, 80, 20), &mut buf, &mut s);
    // Items start after brand(1) + subtitle(2) + gap(3) + section title(4): rows 5..7.
    assert_eq!(s.handle_mouse(&click(3, 6)), Outcome::Changed);
    assert_eq!(s.selected, 1);
    // Clicking the already-active item is consumed, not changed.
    assert_eq!(s.handle_mouse(&click(3, 6)), Outcome::Consumed);
}

#[test]
fn logs_wheel_unpins_follow() {
    let t = Theme::dark();
    let lines: Vec<LogLine> = (0..30)
        .map(|i| LogLine::new(Level::Info, format!("l{i}")))
        .collect();
    let mut s = LogsState::new();
    let area = Rect::new(0, 0, 30, 5);
    let mut buf = Buffer::empty(area);
    Logs::new(&lines).theme(&t).render(area, &mut buf, &mut s);
    assert!(s.follow);
    assert_eq!(s.handle_mouse(&wheel(5, 2, false)), Outcome::Consumed);
    assert!(!s.follow, "wheel-up must unpin follow");
    // Wheel outside the area is ignored.
    assert_eq!(s.handle_mouse(&wheel(5, 10, false)), Outcome::Ignored);
}

#[test]
fn kanban_click_focuses_then_submits() {
    let cards_a = ["one", "two"];
    let cols = [KanbanColumn::new("A", &cards_a)];
    let t = Theme::dark();
    let mut s = KanbanState::new();
    let mut buf = Buffer::empty(Rect::new(0, 0, 30, 10));
    Kanban::new(&cols)
        .theme(&t)
        .render(Rect::new(0, 0, 30, 10), &mut buf, &mut s);
    // Card 1 occupies rows 4..7 (header row + card0 rows 1..4).
    assert_eq!(s.handle_mouse(&click(3, 5)), Outcome::Consumed);
    assert_eq!((s.col, s.card), (0, 1));
    assert_eq!(s.handle_mouse(&click(3, 5)), Outcome::Submitted);
}
