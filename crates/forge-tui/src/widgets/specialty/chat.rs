//! Chat kit (cargo feature `chat`, implies `markdown`): transcript view with
//! role gutters and tool-call boxes, a composer, and interactive prompts —
//! the terminal mirror of `@forge/chat`.

use crate::event::{clicked, in_area, is_press, scroll_delta, Outcome};
use crate::text;
use crate::theme::{default_theme, Theme};
use crate::widgets::forms::{Textarea, TextareaState};
use crate::widgets::specialty::markdown::markdown_lines;
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::StatefulWidget;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
    System,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolStatus {
    Running,
    Ok,
    Error,
}

/// One transcript entry.
#[derive(Clone, Debug)]
pub enum ChatItem {
    Message {
        role: Role,
        author: Option<String>,
        ts: Option<String>,
        body: String,
    },
    ToolCall {
        name: String,
        status: ToolStatus,
        detail: Option<String>,
        open: bool,
    },
    Divider(String),
    Typing(String),
}

impl ChatItem {
    pub fn user(body: impl Into<String>) -> ChatItem {
        ChatItem::Message {
            role: Role::User,
            author: None,
            ts: None,
            body: body.into(),
        }
    }

    pub fn assistant(body: impl Into<String>) -> ChatItem {
        ChatItem::Message {
            role: Role::Assistant,
            author: None,
            ts: None,
            body: body.into(),
        }
    }

    pub fn system(body: impl Into<String>) -> ChatItem {
        ChatItem::Message {
            role: Role::System,
            author: None,
            ts: None,
            body: body.into(),
        }
    }

    pub fn tool(name: impl Into<String>, status: ToolStatus) -> ChatItem {
        ChatItem::ToolCall {
            name: name.into(),
            status,
            detail: None,
            open: false,
        }
    }
}

/// Scroll state for the transcript; follows the tail like `Logs`.
#[derive(Clone, Debug)]
pub struct ChatViewState {
    pub follow: bool,
    offset: usize,
    total: usize,
    view_h: usize,
    area: Rect,
}

impl Default for ChatViewState {
    fn default() -> ChatViewState {
        ChatViewState {
            follow: true,
            offset: 0,
            total: 0,
            view_h: 0,
            area: Rect::default(),
        }
    }
}

impl ChatViewState {
    pub fn new() -> ChatViewState {
        ChatViewState::default()
    }

    /// Wheel scrolls the transcript (scrolling up unpins follow mode).
    pub fn handle_mouse(&mut self, ev: &MouseEvent) -> Outcome {
        let delta = scroll_delta(ev);
        if delta == 0 || !in_area(ev, self.area) {
            return Outcome::Ignored;
        }
        if delta < 0 {
            self.follow = false;
            self.offset = self.offset.saturating_sub(3);
        } else {
            self.offset = (self.offset + 3).min(self.total.saturating_sub(self.view_h));
        }
        Outcome::Consumed
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Outcome {
        if !is_press(&key) {
            return Outcome::Ignored;
        }
        let page = self.view_h.max(1);
        match key.code {
            KeyCode::Up => {
                self.follow = false;
                self.offset = self.offset.saturating_sub(1);
                Outcome::Consumed
            }
            KeyCode::Down => {
                self.offset = (self.offset + 1).min(self.total.saturating_sub(self.view_h));
                Outcome::Consumed
            }
            KeyCode::PageUp => {
                self.follow = false;
                self.offset = self.offset.saturating_sub(page);
                Outcome::Consumed
            }
            KeyCode::PageDown => {
                self.offset = (self.offset + page).min(self.total.saturating_sub(self.view_h));
                Outcome::Consumed
            }
            KeyCode::End => {
                self.follow = true;
                Outcome::Changed
            }
            _ => Outcome::Ignored,
        }
    }
}

/// The transcript. Bodies render as markdown; tool calls as status rows with
/// optional detail; a typing entry animates with the runtime frame.
#[derive(Clone, Debug)]
pub struct ChatView<'a> {
    items: &'a [ChatItem],
    frame: u64,
    focused: bool,
    theme: Option<&'a Theme>,
}

impl<'a> ChatView<'a> {
    pub fn new(items: &'a [ChatItem]) -> ChatView<'a> {
        ChatView {
            items,
            frame: 0,
            focused: false,
            theme: None,
        }
    }

    pub fn frame(mut self, frame: u64) -> Self {
        self.frame = frame;
        self
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }

    fn role_header(role: Role, author: Option<&str>, ts: Option<&str>, t: &Theme) -> Line<'static> {
        let (glyph, name, color) = match role {
            Role::User => ("▸", author.unwrap_or("you"), t.accent.base),
            Role::Assistant => ("◆", author.unwrap_or("assistant"), t.success.base),
            Role::System => ("·", author.unwrap_or("system"), t.fg[2]),
        };
        let mut spans = vec![
            Span::styled(format!("{glyph} "), Style::new().fg(color)),
            Span::styled(
                name.to_owned(),
                Style::new().fg(t.fg[0]).add_modifier(Modifier::BOLD),
            ),
        ];
        if let Some(ts) = ts {
            spans.push(Span::styled(format!("  {ts}"), Style::new().fg(t.fg[3])));
        }
        Line::from(spans)
    }

    fn item_lines(&self, item: &ChatItem, width: usize, t: &Theme) -> Vec<Line<'static>> {
        match item {
            ChatItem::Message {
                role,
                author,
                ts,
                body,
            } => {
                let mut lines = vec![ChatView::role_header(
                    *role,
                    author.as_deref(),
                    ts.as_deref(),
                    t,
                )];
                let body_w = width.saturating_sub(2).max(8);
                for line in markdown_lines(body, body_w, t) {
                    let mut spans = vec![Span::raw("  ")];
                    spans.extend(line.spans);
                    lines.push(Line::from(spans));
                }
                lines.push(Line::default());
                lines
            }
            ChatItem::ToolCall {
                name,
                status,
                detail,
                open,
            } => {
                let (dot, color): (&str, Color) = match status {
                    ToolStatus::Running => ("◌", t.info.base),
                    ToolStatus::Ok => ("●", t.success.base),
                    ToolStatus::Error => ("●", t.danger.base),
                };
                let mut lines = vec![Line::from(vec![
                    Span::styled("  ⚙ ", Style::new().fg(t.fg[2])),
                    Span::styled(name.clone(), Style::new().fg(t.fg[1]).bg(t.bg[2])),
                    Span::styled(format!(" {dot}"), Style::new().fg(color)),
                ])];
                if *open {
                    if let Some(detail) = detail {
                        for l in text::wrap(detail, width.saturating_sub(6).max(8)) {
                            lines.push(Line::from(Span::styled(
                                format!("      {l}"),
                                Style::new().fg(t.fg[2]),
                            )));
                        }
                    }
                }
                lines.push(Line::default());
                lines
            }
            ChatItem::Divider(label) => {
                let pad = width.saturating_sub(text::width(label) + 2) / 2;
                vec![
                    Line::from(Span::styled(
                        format!("{} {} {}", "─".repeat(pad), label, "─".repeat(pad)),
                        Style::new().fg(t.fg[3]),
                    )),
                    Line::default(),
                ]
            }
            ChatItem::Typing(name) => {
                let dots = ["·  ", "·· ", "···", " ··", "  ·", "   "];
                let d = dots[(self.frame as usize / 2) % dots.len()];
                vec![Line::from(vec![
                    Span::styled("✳ ", Style::new().fg(t.accent.base)),
                    Span::styled(format!("{name} is typing "), Style::new().fg(t.fg[2])),
                    Span::styled(d.to_owned(), Style::new().fg(t.fg[2])),
                ])]
            }
        }
    }
}

impl<'a> StatefulWidget for ChatView<'a> {
    type State = ChatViewState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut ChatViewState) {
        state.view_h = area.height as usize;
        state.area = area;
        if area.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        let width = area.width as usize;
        let mut lines: Vec<Line<'static>> = Vec::new();
        for item in self.items {
            lines.extend(self.item_lines(item, width, t));
        }
        state.total = lines.len();
        let max_offset = state.total.saturating_sub(state.view_h);
        if state.follow {
            state.offset = max_offset;
        } else {
            state.offset = state.offset.min(max_offset);
        }
        for (i, line) in lines.iter().skip(state.offset).enumerate() {
            if i as u16 >= area.height {
                break;
            }
            buf.set_line(area.x, area.y + i as u16, line, area.width);
        }
        let _ = self.focused;
    }
}

/// Message composer: a Textarea where Enter sends (`Submitted`) and
/// Alt+Enter inserts a newline. Read/clear the draft via `state.input`.
#[derive(Clone, Debug, Default)]
pub struct ComposerState {
    pub input: TextareaState,
}

impl ComposerState {
    pub fn new() -> ComposerState {
        ComposerState::default()
    }

    pub fn take_message(&mut self) -> String {
        let msg = self.input.value();
        self.input.set_value("");
        msg
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Outcome {
        if !is_press(&key) {
            return Outcome::Ignored;
        }
        match key.code {
            KeyCode::Enter if key.modifiers.contains(KeyModifiers::ALT) => {
                self.input.insert_str("\n");
                Outcome::Changed
            }
            KeyCode::Enter if key.modifiers.is_empty() => {
                if self.input.value().trim().is_empty() {
                    Outcome::Consumed
                } else {
                    Outcome::Submitted
                }
            }
            _ => self.input.handle_key(key),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Composer<'a> {
    placeholder: &'a str,
    focused: bool,
    theme: Option<&'a Theme>,
}

impl<'a> Composer<'a> {
    pub fn new() -> Composer<'a> {
        Composer {
            placeholder: "Message… (Enter send · Alt+Enter newline)",
            ..Default::default()
        }
    }

    pub fn placeholder(mut self, placeholder: &'a str) -> Self {
        self.placeholder = placeholder;
        self
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }
}

impl<'a> StatefulWidget for Composer<'a> {
    type State = ComposerState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut ComposerState) {
        let t = self.theme.unwrap_or_else(|| default_theme());
        Textarea::new()
            .placeholder(self.placeholder)
            .focused(self.focused)
            .theme(t)
            .render(area, buf, &mut state.input);
    }
}

/// Interactive question with option chips (the chat kit's `ChatPrompt`):
/// ←/→ move, Enter submits the selected option.
#[derive(Clone, Debug, Default)]
pub struct ChatPromptState {
    pub selected: usize,
    len: usize,
    chip_rects: Vec<Rect>,
}

impl ChatPromptState {
    pub fn new() -> ChatPromptState {
        ChatPromptState::default()
    }

    /// Click an option chip to choose it (submits).
    pub fn handle_mouse(&mut self, ev: &MouseEvent) -> Outcome {
        for (i, rect) in self.chip_rects.clone().into_iter().enumerate() {
            if clicked(ev, rect) {
                self.selected = i;
                return Outcome::Submitted;
            }
        }
        Outcome::Ignored
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Outcome {
        if !is_press(&key) {
            return Outcome::Ignored;
        }
        match key.code {
            KeyCode::Left | KeyCode::Up => {
                self.selected = self.selected.saturating_sub(1);
                Outcome::Consumed
            }
            KeyCode::Right | KeyCode::Down => {
                if self.len > 0 && self.selected + 1 < self.len {
                    self.selected += 1;
                }
                Outcome::Consumed
            }
            KeyCode::Enter => Outcome::Submitted,
            KeyCode::Esc => Outcome::Cancelled,
            _ => Outcome::Ignored,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ChatPrompt<'a> {
    question: &'a str,
    options: &'a [&'a str],
    focused: bool,
    theme: Option<&'a Theme>,
}

impl<'a> ChatPrompt<'a> {
    pub fn new(question: &'a str, options: &'a [&'a str]) -> ChatPrompt<'a> {
        ChatPrompt {
            question,
            options,
            focused: false,
            theme: None,
        }
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }
}

impl<'a> StatefulWidget for ChatPrompt<'a> {
    type State = ChatPromptState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut ChatPromptState) {
        state.len = self.options.len();
        state.chip_rects.clear();
        if area.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        buf.set_string(
            area.x,
            area.y,
            text::truncate(self.question, area.width as usize),
            Style::new().fg(t.fg[0]),
        );
        if area.height < 2 {
            return;
        }
        let mut x = area.x;
        for (i, option) in self.options.iter().enumerate() {
            let w = text::width(option) as u16 + 2;
            if x + w > area.x + area.width {
                break;
            }
            let active = i == state.selected;
            let mut style = if active {
                Style::new().fg(t.accent.contrast).bg(t.accent.base)
            } else {
                Style::new().fg(t.fg[1]).bg(t.bg[3])
            };
            if active && self.focused {
                style = style.add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
            }
            state.chip_rects.push(Rect::new(x, area.y + 1, w, 1));
            buf.set_style(Rect::new(x, area.y + 1, w, 1), style);
            buf.set_string(x + 1, area.y + 1, *option, style);
            x += w + 1;
        }
    }
}
