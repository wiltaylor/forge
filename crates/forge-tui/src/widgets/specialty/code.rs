//! Code viewing (cargo feature `code`, syntect with the pure-Rust regex
//! engine): syntax-highlighted read-only source with line numbers and gutter
//! marks, plus a unified diff view with add/del tints.
//!
//! Colors come from a syntect theme generated out of the Forge tokens
//! (mirroring `packages/code/src/theme.ts`), so highlighting needs RGB
//! tokens — pass the truecolor theme (quantized themes fall back to plain
//! text colors).

use crate::event::{in_area, is_press, scroll_delta, Outcome};
use crate::text;
use crate::theme::{default_theme, Theme};
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::StatefulWidget;
use std::sync::OnceLock;
use syntect::easy::HighlightLines;
use syntect::highlighting::{
    Color as SynColor, FontStyle, ScopeSelectors, StyleModifier, Theme as SynTheme, ThemeItem,
    ThemeSettings,
};
use syntect::parsing::SyntaxSet;

fn syntax_set() -> &'static SyntaxSet {
    static SET: OnceLock<SyntaxSet> = OnceLock::new();
    SET.get_or_init(SyntaxSet::load_defaults_newlines)
}

fn syn_color(c: Color) -> Option<SynColor> {
    match c {
        Color::Rgb(r, g, b) => Some(SynColor { r, g, b, a: 255 }),
        _ => None,
    }
}

/// Build a syntect theme from Forge tokens — the terminal mirror of the web
/// CodeMirror theme: keyword→accent-fg, string→success-fg, number/atom→
/// info-fg, comment→fg3, type→warning-fg, property→info-fg, punctuation→fg2.
fn forge_syn_theme(t: &Theme) -> Option<SynTheme> {
    let fg = syn_color(t.fg[0])?;
    let item = |scopes: &str, color: Color, bold: bool| -> Option<ThemeItem> {
        let selectors: ScopeSelectors = scopes.parse().ok()?;
        Some(ThemeItem {
            scope: selectors,
            style: StyleModifier {
                foreground: syn_color(color),
                background: None,
                font_style: bold.then_some(FontStyle::BOLD),
            },
        })
    };
    let scopes = [
        ("keyword, storage.modifier, storage.type.function, storage.type.class", t.accent.fg, false),
        ("string, punctuation.definition.string", t.success.fg, false),
        ("constant.numeric, constant.language, constant.character", t.info.fg, false),
        ("comment, punctuation.definition.comment", t.fg[3], false),
        ("entity.name.type, entity.name.class, support.type, support.class, storage.type", t.warning.fg, false),
        ("entity.other.attribute-name, support.function, meta.property-name, variable.other.member", t.info.fg, false),
        ("entity.name.function", t.fg[0], true),
        ("punctuation, keyword.operator", t.fg[2], false),
        ("variable", t.fg[0], false),
    ];
    let mut theme = SynTheme {
        settings: ThemeSettings {
            foreground: Some(fg),
            ..Default::default()
        },
        ..Default::default()
    };
    for (sel, color, bold) in scopes {
        theme.scopes.push(item(sel, color, bold)?);
    }
    Some(theme)
}

/// Scroll state shared by CodeView and DiffView.
#[derive(Clone, Copy, Debug, Default)]
pub struct CodeViewState {
    pub row: usize,
    pub col: usize,
    total: usize,
    view_h: usize,
    area: Rect,
}

impl CodeViewState {
    pub fn new() -> CodeViewState {
        CodeViewState::default()
    }

    /// Wheel scrolls three lines.
    pub fn handle_mouse(&mut self, ev: &MouseEvent) -> Outcome {
        let delta = scroll_delta(ev);
        if delta == 0 || !in_area(ev, self.area) {
            return Outcome::Ignored;
        }
        let max = self.total.saturating_sub(self.view_h);
        self.row = if delta < 0 {
            self.row.saturating_sub(3)
        } else {
            (self.row + 3).min(max)
        };
        Outcome::Consumed
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Outcome {
        if !is_press(&key) {
            return Outcome::Ignored;
        }
        let page = self.view_h.max(1);
        let max = self.total.saturating_sub(self.view_h);
        match key.code {
            KeyCode::Up => self.row = self.row.saturating_sub(1),
            KeyCode::Down => self.row = (self.row + 1).min(max),
            KeyCode::PageUp => self.row = self.row.saturating_sub(page),
            KeyCode::PageDown => self.row = (self.row + page).min(max),
            KeyCode::Home => self.row = 0,
            KeyCode::End => self.row = max,
            KeyCode::Left => self.col = self.col.saturating_sub(4),
            KeyCode::Right => self.col += 4,
            _ => return Outcome::Ignored,
        }
        Outcome::Consumed
    }
}

/// Syntax-highlighted read-only code with a line-number gutter and optional
/// severity marks per line (LSP-style annotations).
#[derive(Clone, Debug)]
pub struct CodeView<'a> {
    source: &'a str,
    /// A file extension or syntax token (`rs`, `py`, `json`, …).
    language: &'a str,
    line_numbers: bool,
    marks: &'a [(usize, crate::theme::Severity)],
    focused: bool,
    theme: Option<&'a Theme>,
}

impl<'a> CodeView<'a> {
    pub fn new(source: &'a str, language: &'a str) -> CodeView<'a> {
        CodeView {
            source,
            language,
            line_numbers: true,
            marks: &[],
            focused: false,
            theme: None,
        }
    }

    pub fn line_numbers(mut self, on: bool) -> Self {
        self.line_numbers = on;
        self
    }

    /// `(0-based line, severity)` gutter marks.
    pub fn marks(mut self, marks: &'a [(usize, crate::theme::Severity)]) -> Self {
        self.marks = marks;
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

impl<'a> StatefulWidget for CodeView<'a> {
    type State = CodeViewState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut CodeViewState) {
        if area.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        buf.set_style(area, Style::new().bg(t.bg[1]));
        let lines: Vec<&str> = self.source.lines().collect();
        state.total = lines.len();
        state.view_h = area.height as usize;
        state.area = area;
        let max = state.total.saturating_sub(state.view_h);
        state.row = state.row.min(max);

        let gutter_w = if self.line_numbers {
            (state.total.max(1).ilog10() as u16 + 1).max(2) + 2
        } else {
            0
        };
        let code_x = area.x + gutter_w + 1;
        let code_w = area.width.saturating_sub(gutter_w + 1) as usize;

        // Highlight the whole file (correct state needs a top-down pass; fine
        // for viewer-sized sources).
        let ss = syntax_set();
        let syntax = ss
            .find_syntax_by_token(self.language)
            .unwrap_or_else(|| ss.find_syntax_plain_text());
        let syn_theme = forge_syn_theme(t);
        let mut highlighter = syn_theme.as_ref().map(|th| HighlightLines::new(syntax, th));

        let mut styled: Vec<Vec<(Style, String)>> = Vec::with_capacity(lines.len());
        for line in &lines {
            match &mut highlighter {
                Some(h) => {
                    let spans = h
                        .highlight_line(line, ss)
                        .unwrap_or_default()
                        .into_iter()
                        .map(|(s, txt)| {
                            let mut style = Style::new().fg(Color::Rgb(
                                s.foreground.r,
                                s.foreground.g,
                                s.foreground.b,
                            ));
                            if s.font_style.contains(FontStyle::BOLD) {
                                style = style.add_modifier(ratatui::style::Modifier::BOLD);
                            }
                            (style.bg(t.bg[1]), txt.to_owned())
                        })
                        .collect();
                    styled.push(spans);
                }
                None => styled.push(vec![(
                    Style::new().fg(t.fg[1]).bg(t.bg[1]),
                    (*line).to_owned(),
                )]),
            }
        }

        for vis in 0..state.view_h {
            let li = state.row + vis;
            if li >= styled.len() {
                break;
            }
            let y = area.y + vis as u16;
            // Gutter.
            if self.line_numbers {
                buf.set_string(
                    area.x,
                    y,
                    format!("{:>w$} ", li + 1, w = gutter_w as usize - 2),
                    Style::new().fg(t.fg[3]).bg(t.bg[1]),
                );
            }
            if let Some((_, sev)) = self.marks.iter().find(|(l, _)| *l == li) {
                buf.set_string(
                    area.x + gutter_w.saturating_sub(1),
                    y,
                    "▎",
                    Style::new().fg(t.severity(*sev).base).bg(t.bg[1]),
                );
            }
            // Code with horizontal scroll.
            let mut cell = 0usize;
            let mut x = code_x;
            for (style, txt) in &styled[li] {
                for g in unicode_segmentation::UnicodeSegmentation::graphemes(txt.as_str(), true) {
                    let gw = text::width(g);
                    if cell + gw > state.col + code_w {
                        break;
                    }
                    if cell >= state.col {
                        buf.set_string(x, y, g, *style);
                        x += gw as u16;
                    }
                    cell += gw;
                }
            }
        }
        let _ = self.focused;
    }
}

enum DiffRow<'a> {
    Same(&'a str),
    Del(&'a str),
    Add(&'a str),
}

/// Plain LCS line diff — quadratic, so very large inputs degrade to full
/// replace rather than stalling the UI thread.
fn diff_lines<'a>(old: &'a str, new: &'a str) -> Vec<DiffRow<'a>> {
    let a: Vec<&str> = old.lines().collect();
    let b: Vec<&str> = new.lines().collect();
    if a.len().saturating_mul(b.len()) > 1_000_000 {
        let mut out: Vec<DiffRow> = a.into_iter().map(DiffRow::Del).collect();
        out.extend(b.into_iter().map(DiffRow::Add));
        return out;
    }
    let n = a.len();
    let m = b.len();
    let mut dp = vec![0u32; (n + 1) * (m + 1)];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            dp[i * (m + 1) + j] = if a[i] == b[j] {
                dp[(i + 1) * (m + 1) + j + 1] + 1
            } else {
                dp[(i + 1) * (m + 1) + j].max(dp[i * (m + 1) + j + 1])
            };
        }
    }
    let mut out = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < n && j < m {
        if a[i] == b[j] {
            out.push(DiffRow::Same(a[i]));
            i += 1;
            j += 1;
        } else if dp[(i + 1) * (m + 1) + j] >= dp[i * (m + 1) + j + 1] {
            out.push(DiffRow::Del(a[i]));
            i += 1;
        } else {
            out.push(DiffRow::Add(b[j]));
            j += 1;
        }
    }
    out.extend(a[i..].iter().map(|l| DiffRow::Del(l)));
    out.extend(b[j..].iter().map(|l| DiffRow::Add(l)));
    out
}

/// Unified diff of two sources with add/del tint rows.
#[derive(Clone, Debug)]
pub struct DiffView<'a> {
    old: &'a str,
    new: &'a str,
    theme: Option<&'a Theme>,
}

impl<'a> DiffView<'a> {
    pub fn new(old: &'a str, new: &'a str) -> DiffView<'a> {
        DiffView {
            old,
            new,
            theme: None,
        }
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }
}

impl<'a> StatefulWidget for DiffView<'a> {
    type State = CodeViewState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut CodeViewState) {
        if area.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        buf.set_style(area, Style::new().bg(t.bg[1]));
        let rows = diff_lines(self.old, self.new);
        state.total = rows.len();
        state.view_h = area.height as usize;
        state.area = area;
        state.row = state.row.min(state.total.saturating_sub(state.view_h));
        for vis in 0..state.view_h {
            let ri = state.row + vis;
            let Some(row) = rows.get(ri) else { break };
            let y = area.y + vis as u16;
            let (marker, line, style) = match row {
                DiffRow::Same(l) => (" ", l, Style::new().fg(t.fg[2]).bg(t.bg[1])),
                DiffRow::Del(l) => ("-", l, Style::new().fg(t.danger.fg).bg(t.danger.bg)),
                DiffRow::Add(l) => ("+", l, Style::new().fg(t.success.fg).bg(t.success.bg)),
            };
            if !matches!(row, DiffRow::Same(_)) {
                buf.set_style(Rect::new(area.x, y, area.width, 1), style);
            }
            buf.set_string(area.x, y, marker, style);
            buf.set_string(
                area.x + 2,
                y,
                text::truncate(line, area.width.saturating_sub(2) as usize),
                style,
            );
        }
    }
}
