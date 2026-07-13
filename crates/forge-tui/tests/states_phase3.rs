use forge_tui::event::Outcome;
use forge_tui::theme::Theme;
use forge_tui::widgets::*;
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::widgets::StatefulWidget;
use serde_json::json;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn shift(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::SHIFT)
}

#[test]
fn table_cursor_selection_and_sort_cycle() {
    let t = Theme::dark();
    let columns = [Column::new("a"), Column::new("b")];
    let rows: Vec<Vec<&str>> = vec![vec!["r0", "x"], vec!["r1", "y"], vec!["r2", "z"]];
    let mut state = TableState::new();
    let mut buf = Buffer::empty(Rect::new(0, 0, 30, 5));
    Table::new(&columns, &rows).render(Rect::new(0, 0, 30, 5), &mut buf, &mut state);

    let _ = state.handle_key(key(KeyCode::Down));
    assert_eq!(state.cursor, 1);
    assert_eq!(state.handle_key(key(KeyCode::Char(' '))), Outcome::Changed);
    assert!(state.is_selected(1));
    assert_eq!(state.handle_key(key(KeyCode::Enter)), Outcome::Submitted);

    // Sort cycles col0↑ → col0↓ → col1↑ → col1↓ → off.
    assert_eq!(state.handle_key(key(KeyCode::Char('s'))), Outcome::Changed);
    assert_eq!(state.sort, Some((0, true)));
    let _ = state.handle_key(key(KeyCode::Char('s')));
    assert_eq!(state.sort, Some((0, false)));
    let _ = state.handle_key(key(KeyCode::Char('s')));
    let _ = state.handle_key(key(KeyCode::Char('s')));
    let _ = state.handle_key(key(KeyCode::Char('s')));
    assert_eq!(state.sort, None);
}

#[test]
fn logs_follow_pins_to_tail() {
    let t = Theme::dark();
    let lines: Vec<LogLine> = (0..30)
        .map(|i| LogLine::new(Level::Info, format!("line {i}")))
        .collect();
    let mut state = LogsState::new();
    let area = Rect::new(0, 0, 30, 5);
    let mut buf = Buffer::empty(area);
    Logs::new(&lines)
        .theme(&t)
        .render(area, &mut buf, &mut state);
    let bottom_row: String = (0..30u16).map(|x| buf[(x, 4)].symbol()).collect();
    assert!(
        bottom_row.contains("line 29"),
        "follow should pin tail: {bottom_row}"
    );

    // Scrolling up unpins; f re-pins.
    assert_eq!(state.handle_key(key(KeyCode::Up)), Outcome::Consumed);
    assert!(!state.follow);
    assert_eq!(state.handle_key(key(KeyCode::Char('f'))), Outcome::Changed);
    assert!(state.follow);
}

#[test]
fn tree_expand_collapse_and_parent_jump() {
    const KIDS: [TreeNode<'static>; 2] = [
        TreeNode {
            label: "a",
            children: &[],
        },
        TreeNode {
            label: "b",
            children: &[],
        },
    ];
    let roots = [TreeNode::branch("root", &KIDS), TreeNode::leaf("sibling")];
    let mut state = TreeState::new();
    // Expand root.
    assert_eq!(
        state.handle_key(key(KeyCode::Right), &roots),
        Outcome::Changed
    );
    assert!(state.is_expanded(&[0]));
    // Step into first child, then jump back to parent.
    let _ = state.handle_key(key(KeyCode::Right), &roots);
    assert_eq!(state.cursor, 1);
    let _ = state.handle_key(key(KeyCode::Left), &roots);
    assert_eq!(state.cursor, 0);
    // Collapse.
    assert_eq!(
        state.handle_key(key(KeyCode::Left), &roots),
        Outcome::Changed
    );
    assert!(!state.is_expanded(&[0]));
    assert_eq!(state.cursor_path(&roots), Some(vec![0]));
}

#[test]
fn json_viewer_paths_and_expansion() {
    let value = json!({"a": [1, 2], "b": "text"});
    let mut state = JsonViewerState::new();
    let t = Theme::dark();
    let mut buf = Buffer::empty(Rect::new(0, 0, 40, 8));
    JsonViewer::new()
        .theme(&t)
        .render_value(&value, Rect::new(0, 0, 40, 8), &mut buf, &mut state);
    assert_eq!(state.cursor_path(), "$");
    let _ = state.handle_key(key(KeyCode::Down), &value);
    // Expand $.a and verify the array children appear.
    assert_eq!(
        state.handle_key(key(KeyCode::Right), &value),
        Outcome::Changed
    );
    JsonViewer::new()
        .theme(&t)
        .render_value(&value, Rect::new(0, 0, 40, 8), &mut buf, &mut state);
    let _ = state.handle_key(key(KeyCode::Down), &value);
    JsonViewer::new()
        .theme(&t)
        .render_value(&value, Rect::new(0, 0, 40, 8), &mut buf, &mut state);
    assert_eq!(state.cursor_path(), "$.a[0]");
}

#[test]
fn kanban_moves_are_requested_not_applied() {
    let cards_a = ["one", "two"];
    let cards_b = ["three"];
    let cols = [
        KanbanColumn::new("A", &cards_a),
        KanbanColumn::new("B", &cards_b),
    ];
    let t = Theme::dark();
    let mut state = KanbanState::new();
    let mut buf = Buffer::empty(Rect::new(0, 0, 40, 10));
    Kanban::new(&cols)
        .theme(&t)
        .render(Rect::new(0, 0, 40, 10), &mut buf, &mut state);

    assert_eq!(state.handle_key(shift(KeyCode::Right)), Outcome::Changed);
    let mv = state.take_move().expect("move requested");
    assert_eq!(mv.from, (0, 0));
    assert_eq!(mv.to, (1, 1)); // appended at the target column's end
    assert_eq!(state.take_move(), None, "take_move must clear");
    // Plain arrows only move the cursor.
    let _ = state.handle_key(key(KeyCode::Left));
    assert_eq!(state.take_move(), None);
}

#[test]
fn block_grid_wraps_and_clips() {
    let grid = BlockGrid::new(2).gap(1);
    let rects = grid.split(
        Rect::new(0, 0, 21, 10),
        &[
            BlockSpec::new(1, 4),
            BlockSpec::new(1, 4),
            BlockSpec::new(2, 3),
        ],
    );
    assert_eq!(rects.len(), 3);
    assert_eq!(rects[0].y, rects[1].y);
    assert!(rects[2].y > rects[0].y, "third block wraps to next row");
    assert!(rects[2].width > rects[0].width, "span-2 block is wider");
}

#[cfg(feature = "calendar")]
#[test]
fn calendar_navigation() {
    use time::{Date, Month};
    let start = Date::from_calendar_date(2026, Month::July, 10).unwrap();
    let mut state = CalendarState::new(start);
    assert_eq!(state.handle_key(key(KeyCode::Right)), Outcome::Changed);
    assert_eq!(state.selected.day(), 11);
    let _ = state.handle_key(key(KeyCode::Up));
    assert_eq!(state.selected.day(), 4);
    assert_eq!(state.handle_key(key(KeyCode::PageDown)), Outcome::Changed);
    assert_eq!(state.selected.month(), Month::August);
    assert_eq!(state.handle_key(key(KeyCode::Enter)), Outcome::Submitted);

    // Month-end clamping: Jan 31 → Feb 28/29.
    let eom = Date::from_calendar_date(2026, Month::January, 31).unwrap();
    let mut state = CalendarState::new(eom);
    let _ = state.handle_key(key(KeyCode::PageDown));
    assert_eq!(state.selected.month(), Month::February);
    assert_eq!(state.selected.day(), 28);
}

#[test]
fn file_picker_navigates_directories() {
    let dir = std::env::temp_dir().join(format!("forge-tui-test-{}", std::process::id()));
    let sub = dir.join("subdir");
    std::fs::create_dir_all(&sub).unwrap();
    std::fs::write(dir.join("file.txt"), b"x").unwrap();
    std::fs::write(sub.join("inner.txt"), b"y").unwrap();

    let mut state = FilePickerState::new(&dir);
    // Entries sort dirs first: cursor 0 = subdir.
    assert_eq!(state.handle_key(key(KeyCode::Enter)), Outcome::Changed);
    assert!(state.cwd().ends_with("subdir"));
    // Pick the file inside.
    assert_eq!(state.handle_key(key(KeyCode::Enter)), Outcome::Submitted);
    assert!(state.take_selected().unwrap().ends_with("inner.txt"));
    // Go back up.
    assert_eq!(state.handle_key(key(KeyCode::Backspace)), Outcome::Changed);
    assert!(!state.cwd().ends_with("subdir"));

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn accordion_is_exclusive() {
    let t = Theme::dark();
    let items = [("one", "body1"), ("two", "body2")];
    let mut state = AccordionState::new();
    let mut buf = Buffer::empty(Rect::new(0, 0, 30, 8));
    Accordion::new(&items)
        .theme(&t)
        .render(Rect::new(0, 0, 30, 8), &mut buf, &mut state);
    let _ = state.handle_key(key(KeyCode::Enter));
    assert_eq!(state.open, Some(0));
    let _ = state.handle_key(key(KeyCode::Down));
    let _ = state.handle_key(key(KeyCode::Enter));
    assert_eq!(
        state.open,
        Some(1),
        "opening another panel closes the first"
    );
    let _ = state.handle_key(key(KeyCode::Enter));
    assert_eq!(state.open, None, "re-toggling closes");
}
