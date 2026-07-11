//! Code viewing (cargo feature `code`, syntect with the pure-Rust regex
//! engine): syntax-highlighted read-only source with a line-number gutter and
//! LSP-style annotations, plus a line diff with add/del tints.
//!
//! Colors come from a syntect theme generated out of the Forge tokens —
//! the same scope→token mapping as forge-tui's code widget (which mirrors
//! `packages/code/src/theme.ts`): keyword→accent-fg, string→success-fg,
//! number/atom→info-fg, comment→fg3, type→warning-fg, property→info-fg,
//! punctuation→fg2.

use crate::theme::{Severity, Theme};
use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, CornerRadius, FontId, Frame, Margin, Pos2, Rect, Sense, Stroke, Ui, Vec2};
use std::sync::OnceLock;
use syntect::easy::HighlightLines;
use syntect::highlighting::{
    Color as SynColor, FontStyle, ScopeSelectors, StyleModifier, Theme as SynTheme, ThemeItem,
    ThemeSettings,
};
use syntect::parsing::SyntaxSet;

/// Named family carrying JetBrains Mono Bold (see `theme::fonts`); falls back
/// to regular mono when the `fonts` feature is off or fonts aren't bound yet.
const MONO_BOLD_FAMILY: &str = "jetbrains-mono-bold";

fn syntax_set() -> &'static SyntaxSet {
    static SET: OnceLock<SyntaxSet> = OnceLock::new();
    SET.get_or_init(SyntaxSet::load_defaults_newlines)
}

fn syn_color(c: Color32) -> SynColor {
    SynColor {
        r: c.r(),
        g: c.g(),
        b: c.b(),
        a: 255,
    }
}

/// Build a syntect theme from Forge tokens (ported from forge-tui).
fn forge_syn_theme(t: &Theme) -> SynTheme {
    let item = |scopes: &str, color: Color32, bold: bool| -> Option<ThemeItem> {
        let selectors: ScopeSelectors = scopes.parse().ok()?;
        Some(ThemeItem {
            scope: selectors,
            style: StyleModifier {
                foreground: Some(syn_color(color)),
                background: None,
                font_style: bold.then_some(FontStyle::BOLD),
            },
        })
    };
    #[rustfmt::skip]
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
            foreground: Some(syn_color(t.fg[0])),
            ..Default::default()
        },
        ..Default::default()
    };
    for (sel, color, bold) in scopes {
        if let Some(item) = item(sel, color, bold) {
            theme.scopes.push(item);
        }
    }
    theme
}

/// The mono font pair for code: `(regular, bold-if-bound)`.
fn mono_fonts(ui: &Ui, size: f32) -> (FontId, FontId) {
    let regular = FontId::monospace(size);
    let bold_family = egui::FontFamily::Name(MONO_BOLD_FAMILY.into());
    let bold = if ui
        .ctx()
        .fonts(|f| f.definitions().families.contains_key(&bold_family))
    {
        FontId::new(size, bold_family)
    } else {
        regular.clone()
    };
    (regular, bold)
}

/// Highlight `code` into styled `(format, text)` runs per line. Unknown
/// languages (or unparsable scope selectors) fall back to plain `fg[1]` text.
fn highlight_runs(
    ui: &Ui,
    t: &Theme,
    code: &str,
    lang: &str,
    size: f32,
) -> Vec<Vec<(TextFormat, String)>> {
    let (mono, mono_bold) = mono_fonts(ui, size);
    let ss = syntax_set();
    let syntax = ss
        .find_syntax_by_token(lang)
        .unwrap_or_else(|| ss.find_syntax_plain_text());
    let theme = forge_syn_theme(t);
    let mut highlighter = HighlightLines::new(syntax, &theme);

    let plain = TextFormat {
        font_id: mono.clone(),
        color: t.fg[1],
        ..Default::default()
    };
    code.lines()
        .map(|line| {
            let mut runs = Vec::new();
            match highlighter.highlight_line(line, ss) {
                Ok(spans) if !spans.is_empty() => {
                    for (style, text) in spans {
                        let bold = style.font_style.contains(FontStyle::BOLD);
                        runs.push((
                            TextFormat {
                                font_id: if bold {
                                    mono_bold.clone()
                                } else {
                                    mono.clone()
                                },
                                color: Color32::from_rgb(
                                    style.foreground.r,
                                    style.foreground.g,
                                    style.foreground.b,
                                ),
                                italics: style.font_style.contains(FontStyle::ITALIC),
                                ..Default::default()
                            },
                            text.to_owned(),
                        ));
                    }
                }
                _ => runs.push((plain.clone(), line.to_owned())),
            }
            if runs.iter().all(|(_, s)| s.is_empty()) {
                // Keep empty lines one row tall.
                runs.push((plain.clone(), " ".to_owned()));
            }
            runs
        })
        .collect()
}

/// Highlight `code` into one [`LayoutJob`] per line.
pub(crate) fn highlight_lines_jobs(
    ui: &Ui,
    t: &Theme,
    code: &str,
    lang: &str,
    size: f32,
) -> Vec<LayoutJob> {
    highlight_runs(ui, t, code, lang, size)
        .into_iter()
        .map(|runs| {
            let mut job = LayoutJob::default();
            for (format, text) in runs {
                job.append(&text, 0.0, format);
            }
            job
        })
        .collect()
}

/// Highlight a whole block into a single [`LayoutJob`] (used by Markdown's
/// fenced code blocks — hence unused when `code` is on without `markdown`).
#[cfg_attr(not(feature = "markdown"), allow(dead_code))]
pub(crate) fn highlight_job(ui: &Ui, t: &Theme, code: &str, lang: &str, size: f32) -> LayoutJob {
    let (mono, _) = mono_fonts(ui, size);
    let newline = TextFormat {
        font_id: mono,
        color: t.fg[1],
        ..Default::default()
    };
    let lines = highlight_runs(ui, t, code, lang, size);
    let n = lines.len();
    let mut out = LayoutJob::default();
    for (i, runs) in lines.into_iter().enumerate() {
        for (format, text) in runs {
            out.append(&text, 0.0, format);
        }
        if i + 1 < n {
            out.append("\n", 0.0, newline.clone());
        }
    }
    out
}

/// An LSP-style annotation: squiggle + gutter dot on `line` (1-based), with
/// the message as a hover tooltip.
#[derive(Clone, Debug, PartialEq)]
pub struct CodeAnnotation {
    /// 1-based line number.
    pub line: usize,
    pub severity: Severity,
    pub message: String,
}

impl CodeAnnotation {
    pub fn new(line: usize, severity: Severity, message: impl Into<String>) -> CodeAnnotation {
        CodeAnnotation {
            line,
            severity,
            message: message.into(),
        }
    }
}

/// Optional presentation state for [`CodeView`].
#[derive(Clone, Copy, Debug, Default)]
pub struct CodeViewState {
    /// Soft-wrap long lines instead of scrolling horizontally.
    pub wrap: bool,
}

/// Syntax-highlighted read-only code with a line-number gutter and
/// annotations: `CodeView::new(source, "rs").show(ui)`.
#[derive(Clone, Debug)]
pub struct CodeView<'a> {
    source: &'a str,
    /// A file extension or syntax token (`rs`, `py`, `json`, …).
    language: &'a str,
    line_numbers: bool,
    annotations: &'a [CodeAnnotation],
}

impl<'a> CodeView<'a> {
    pub fn new(source: &'a str, language: &'a str) -> CodeView<'a> {
        CodeView {
            source,
            language,
            line_numbers: true,
            annotations: &[],
        }
    }

    pub fn line_numbers(mut self, on: bool) -> Self {
        self.line_numbers = on;
        self
    }

    pub fn annotations(mut self, annotations: &'a [CodeAnnotation]) -> Self {
        self.annotations = annotations;
        self
    }

    pub fn show(self, ui: &mut Ui) -> egui::Response {
        let mut state = CodeViewState::default();
        self.show_state(ui, &mut state)
    }

    pub fn show_state(self, ui: &mut Ui, state: &mut CodeViewState) -> egui::Response {
        let t = Theme::of(ui.ctx());
        let wrap = state.wrap;
        Frame::new()
            .fill(t.bg[1])
            .stroke(Stroke::new(1.0, t.border.subtle))
            .corner_radius(CornerRadius::same(t.radius.md as u8))
            .inner_margin(Margin::same(8))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width() - 16.0);
                if wrap {
                    self.rows(ui, &t, true);
                } else {
                    egui::ScrollArea::horizontal()
                        .id_salt("forge-code-view")
                        .show(ui, |ui| self.rows(ui, &t, false));
                }
            })
            .response
    }

    fn rows(&self, ui: &mut Ui, t: &Theme, wrap: bool) {
        let jobs = highlight_lines_jobs(ui, t, self.source, self.language, t.type_scale.sm);
        let gutter_font = t.mono(t.type_scale.xs);
        let digits = jobs.len().max(1).ilog10() as usize + 1;
        let char_w = ui
            .painter()
            .layout_no_wrap("0".to_owned(), gutter_font.clone(), t.fg[3])
            .size()
            .x;
        let gutter_w = if self.line_numbers {
            digits.max(2) as f32 * char_w + 10.0
        } else {
            0.0
        };
        let row_h = ui
            .ctx()
            .fonts_mut(|f| f.row_height(&t.mono(t.type_scale.sm)));

        ui.spacing_mut().item_spacing.y = 2.0;
        for (i, job) in jobs.into_iter().enumerate() {
            let line_no = i + 1;
            let ann = self.annotations.iter().find(|a| a.line == line_no);
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                if self.line_numbers {
                    let (grect, _) =
                        ui.allocate_exact_size(Vec2::new(gutter_w, row_h), Sense::hover());
                    // Right-aligned, painted (never selectable).
                    let g = ui.painter().layout_no_wrap(
                        line_no.to_string(),
                        gutter_font.clone(),
                        t.fg[3],
                    );
                    ui.painter().galley(
                        Pos2::new(
                            grect.max.x - 4.0 - g.size().x,
                            grect.center().y - g.size().y / 2.0,
                        ),
                        g,
                        t.fg[3],
                    );
                    if let Some(ann) = ann {
                        ui.painter().circle_filled(
                            Pos2::new(grect.min.x + 3.0, grect.center().y),
                            2.5,
                            t.severity(ann.severity).base,
                        );
                    }
                }
                let mut job = job;
                if !wrap {
                    job.wrap.max_width = f32::INFINITY;
                }
                let label = egui::Label::new(job).wrap_mode(if wrap {
                    egui::TextWrapMode::Wrap
                } else {
                    egui::TextWrapMode::Extend
                });
                let response = ui.add(label);
                if let Some(ann) = ann {
                    squiggle(ui, response.rect, t.severity(ann.severity).base);
                    response.on_hover_text(
                        egui::RichText::new(&ann.message)
                            .font(t.font(
                                ui.ctx(),
                                crate::theme::FontWeight::Regular,
                                t.type_scale.sm,
                            ))
                            .color(t.fg[0]),
                    );
                }
            });
        }
    }
}

/// A wavy severity underline beneath a line of text.
fn squiggle(ui: &Ui, rect: Rect, color: Color32) {
    let y = rect.max.y - 1.0;
    let amp = 1.4;
    let period = 6.0;
    let mut points = Vec::new();
    let mut x = rect.min.x;
    while x <= rect.max.x {
        let phase = (x - rect.min.x) / period * std::f32::consts::TAU;
        points.push(Pos2::new(x, y + phase.sin() * amp));
        x += 1.5;
    }
    if points.len() > 1 {
        ui.painter()
            .add(egui::Shape::line(points, Stroke::new(1.0, color)));
    }
}

/* ---------------- diff ---------------- */

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DiffRow<'a> {
    Same(&'a str),
    Del(&'a str),
    Add(&'a str),
}

/// Plain LCS line diff (ported from forge-tui) — quadratic, so very large
/// inputs degrade to full replace rather than stalling the UI thread.
pub(crate) fn diff_lines<'a>(old: &'a str, new: &'a str) -> Vec<DiffRow<'a>> {
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

/// Unified line diff of two sources: added lines get a success tint and `+`
/// gutter, removed a danger tint and `-`.
#[derive(Clone, Debug)]
pub struct DiffView<'a> {
    old: &'a str,
    new: &'a str,
}

impl<'a> DiffView<'a> {
    pub fn new(old: &'a str, new: &'a str) -> DiffView<'a> {
        DiffView { old, new }
    }

    pub fn show(self, ui: &mut Ui) -> egui::Response {
        let t = Theme::of(ui.ctx());
        Frame::new()
            .fill(t.bg[1])
            .stroke(Stroke::new(1.0, t.border.subtle))
            .corner_radius(CornerRadius::same(t.radius.md as u8))
            .inner_margin(Margin::same(8))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width() - 16.0);
                egui::ScrollArea::horizontal()
                    .id_salt("forge-diff-view")
                    .show(ui, |ui| self.rows(ui, &t));
            })
            .response
    }

    fn rows(&self, ui: &mut Ui, t: &Theme) {
        let font = t.mono(t.type_scale.sm);
        let rows = diff_lines(self.old, self.new);
        let galleys: Vec<_> = rows
            .iter()
            .map(|row| {
                let (marker, line, color) = match row {
                    DiffRow::Same(l) => (' ', *l, t.fg[2]),
                    DiffRow::Del(l) => ('-', *l, t.danger.fg),
                    DiffRow::Add(l) => ('+', *l, t.success.fg),
                };
                let galley =
                    ui.painter()
                        .layout_no_wrap(format!("{marker} {line}"), font.clone(), color);
                (galley, color)
            })
            .collect();
        let row_h = ui.ctx().fonts_mut(|f| f.row_height(&font)) + 3.0;
        let max_w = galleys
            .iter()
            .map(|(g, _)| g.size().x)
            .fold(ui.available_width(), f32::max);
        let (rect, _) =
            ui.allocate_exact_size(Vec2::new(max_w, rows.len() as f32 * row_h), Sense::hover());
        if !ui.is_rect_visible(rect) {
            return;
        }
        for (i, (row, (galley, color))) in rows.iter().zip(galleys).enumerate() {
            let y = rect.min.y + i as f32 * row_h;
            let line_rect = Rect::from_min_size(Pos2::new(rect.min.x, y), Vec2::new(max_w, row_h));
            match row {
                DiffRow::Del(_) => {
                    ui.painter()
                        .rect_filled(line_rect, CornerRadius::ZERO, t.danger.bg);
                }
                DiffRow::Add(_) => {
                    ui.painter()
                        .rect_filled(line_rect, CornerRadius::ZERO, t.success.bg);
                }
                DiffRow::Same(_) => {}
            }
            ui.painter()
                .galley(Pos2::new(rect.min.x + 2.0, y + 1.5), galley, color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_insert() {
        let rows = diff_lines("a\nb", "a\nx\nb");
        assert_eq!(
            rows,
            vec![DiffRow::Same("a"), DiffRow::Add("x"), DiffRow::Same("b")]
        );
    }

    #[test]
    fn diff_delete() {
        let rows = diff_lines("a\nb\nc", "a\nc");
        assert_eq!(
            rows,
            vec![DiffRow::Same("a"), DiffRow::Del("b"), DiffRow::Same("c")]
        );
    }

    #[test]
    fn diff_replace() {
        let rows = diff_lines("a\nold\nz", "a\nnew\nz");
        assert_eq!(
            rows,
            vec![
                DiffRow::Same("a"),
                DiffRow::Del("old"),
                DiffRow::Add("new"),
                DiffRow::Same("z"),
            ]
        );
    }

    #[test]
    fn diff_identical_and_empty() {
        assert_eq!(
            diff_lines("a\nb", "a\nb"),
            vec![DiffRow::Same("a"), DiffRow::Same("b")]
        );
        assert_eq!(diff_lines("", ""), Vec::<DiffRow>::new());
        assert_eq!(diff_lines("", "a"), vec![DiffRow::Add("a")]);
        assert_eq!(diff_lines("a", ""), vec![DiffRow::Del("a")]);
    }

    #[test]
    fn forge_theme_builds_all_scopes() {
        let theme = forge_syn_theme(&Theme::dark());
        assert_eq!(theme.scopes.len(), 9);
    }
}
