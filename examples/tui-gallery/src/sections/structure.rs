use forge_tui::event::{KeyCombo, Keymap};
use forge_tui::prelude::*;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::Frame;

const TABS: FocusId = FocusId::new("st-tabs");
const PAGES: FocusId = FocusId::new("st-pages");
const SPLIT: FocusId = FocusId::new("st-split");

pub struct StructureState {
    pub tabs: TabsState,
    pub pages: PaginationState,
    pub split: SplitState,
    pub keymap: Keymap,
}

impl Default for StructureState {
    fn default() -> StructureState {
        StructureState {
            tabs: TabsState::new(0),
            pages: PaginationState::new(6, 42),
            split: SplitState::new(0.45),
            keymap: Keymap::new()
                .bind("palette", KeyCombo::ctrl(KeyCode::Char('k')), "palette")
                .bind("help", KeyCombo::char('?'), "help")
                .bind("theme", KeyCombo::char('t'), "theme")
                .bind("quit", KeyCombo::char('q'), "quit"),
        }
    }
}

impl StructureState {
    pub fn handle_key(&mut self, focused: Option<FocusId>, key: KeyEvent) -> Outcome {
        match focused {
            Some(id) if id == TABS => self.tabs.handle_key(key),
            Some(id) if id == PAGES => self.pages.handle_key(key),
            Some(id) if id == SPLIT => self.split.handle_key(key),
            _ => Outcome::Ignored,
        }
    }
}

pub fn draw(frame: &mut Frame, area: Rect, ctx: &mut Ctx, t: &Theme, state: &mut StructureState) {
    let mut y = area.y;
    let bottom = area.y + area.height;
    let row = |h: u16, gap: u16, y: &mut u16| -> Option<Rect> {
        if *y + h > bottom {
            return None;
        }
        let r = Rect::new(area.x, *y, area.width, h);
        *y += h + gap;
        Some(r)
    };

    if let Some(r) = row(1, 0, &mut y) {
        frame.render_widget(Crumbs::new(&["forge", "tui", "structure"]).theme(t), r);
    }
    if let Some(r) = row(2, 1, &mut y) {
        frame.render_widget(
            PageHead::new("Structure")
                .description("Crumbs, page head, tabs, pagination, split pane, settings rows")
                .theme(t),
            r,
        );
    }

    if let Some(r) = row(2, 1, &mut y) {
        let focused = ctx.focus.register(TABS);
        frame.render_stateful_widget(
            Tabs::new(&["Overview", "Metrics", "Logs", "Config"]).focused(focused).theme(t),
            r,
            &mut state.tabs,
        );
    }

    if let Some(r) = row(1, 1, &mut y) {
        let focused = ctx.focus.register(PAGES);
        frame.render_stateful_widget(
            Pagination::new().focused(focused).theme(t),
            Rect::new(r.x, r.y, r.width.min(40), 1),
            &mut state.pages,
        );
    }

    if let Some(r) = row(6, 1, &mut y) {
        let focused = ctx.focus.register(SPLIT);
        let pane = SplitPane::new().min(10).focused(focused).theme(t);
        let (left, right) = pane.areas(r, &mut state.split);
        frame.render_stateful_widget(pane, r, &mut state.split);
        let left_card = Card::new().title(" Nodes ").theme(t);
        let right_card = Card::new().title(" Detail ").theme(t);
        let li = left_card.inner(left);
        frame.render_widget(left_card, left);
        frame.render_widget(right_card, right);
        frame.render_widget(
            Empty::new("←/→ resize when focused").glyph(Glyph::ChevronRight).theme(t),
            li,
        );
    }

    if let Some(r) = row(2, 0, &mut y) {
        frame.render_widget(SettingsSection::new("Cluster").theme(t), r);
    }
    if let Some(r) = row(1, 0, &mut y) {
        let sr = SettingsRow::new("Auto-heal").theme(t);
        let control = sr.control_area(r);
        frame.render_widget(sr, r);
        frame.render_widget(Badge::new("on").severity(Severity::Success).theme(t), control);
    }
    if let Some(r) = row(2, 1, &mut y) {
        let sr = SettingsRow::new("Replicas").help("Rolling restarts keep quorum").theme(t);
        let control = sr.control_area(r);
        frame.render_widget(sr, r);
        frame.render_widget(
            Progress::new(0.6).label("3/5").show_percent(false).theme(t),
            Rect::new(control.x, control.y, control.width.min(20), 1),
        );
    }

    if let Some(r) = row(1, 0, &mut y) {
        frame.render_widget(HelpBar::new(&state.keymap).theme(t), r);
    }
}
