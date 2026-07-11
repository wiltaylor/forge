//! Inline-markdown rendering for unfocused text blocks: `parse_inline` spans
//! mapped onto one [`egui::text::LayoutJob`] with Forge tokens (strong →
//! SemiBold, emphasis → italics, strike, code chips on `bg[3]`, links in the
//! accent tone). Emoji shortcodes are already resolved by the parser.

use crate::theme::{FontWeight, Theme};
use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, Stroke, Ui};
use forge_blocks::{parse_inline, BlockKind};

/// Base text style of a block kind — the same values feed the focused
/// `TextEdit`, so entering/leaving edit mode never changes metrics.
#[derive(Clone, Copy, Debug)]
pub(super) struct InlineStyle {
    pub size: f32,
    pub weight: FontWeight,
    pub color: Color32,
    pub italics: bool,
}

/// The style a text-bearing kind renders (and edits) in.
pub(super) fn text_style(t: &Theme, kind: &BlockKind) -> InlineStyle {
    match kind {
        BlockKind::Heading { level, .. } => InlineStyle {
            size: match level {
                1 => t.type_scale.h1,
                2 => t.type_scale.h2,
                3 => t.type_scale.h3,
                _ => t.type_scale.lg,
            },
            weight: FontWeight::SemiBold,
            color: t.fg[0],
            italics: false,
        },
        BlockKind::Quote { .. } => InlineStyle {
            size: t.type_scale.base,
            weight: FontWeight::Regular,
            color: t.fg[2],
            italics: true,
        },
        _ => InlineStyle {
            size: t.type_scale.base,
            weight: FontWeight::Regular,
            color: t.fg[1],
            italics: false,
        },
    }
}

/// Lay one block's inline markdown out as a wrapped [`LayoutJob`].
pub(super) fn inline_job(
    ui: &Ui,
    t: &Theme,
    md: &str,
    base: InlineStyle,
    wrap_width: f32,
) -> LayoutJob {
    let mut job = LayoutJob {
        wrap: egui::text::TextWrapping::wrap_at_width(wrap_width),
        ..Default::default()
    };
    let base_format = |weight: FontWeight, color: Color32, italics: bool| TextFormat {
        font_id: t.font(ui.ctx(), weight, base.size),
        color,
        italics,
        ..Default::default()
    };
    if md.is_empty() {
        // Keep empty blocks one row tall (and clickable).
        job.append(" ", 0.0, base_format(base.weight, base.color, base.italics));
        return job;
    }
    for span in parse_inline(md) {
        if span.code {
            job.append(
                &format!(" {} ", span.text),
                0.0,
                TextFormat {
                    font_id: t.mono(base.size - 1.0),
                    color: t.accent.fg,
                    background: t.bg[3],
                    ..Default::default()
                },
            );
            continue;
        }
        let weight = if span.strong {
            FontWeight::SemiBold
        } else {
            base.weight
        };
        let color = if span.link.is_some() {
            t.accent.base
        } else if span.strong {
            t.fg[0]
        } else {
            base.color
        };
        let mut format = base_format(weight, color, span.emphasis || base.italics);
        if span.strike {
            format.strikethrough = Stroke::new(1.0, color);
        }
        if span.link.is_some() {
            format.underline = Stroke::new(1.0, t.accent.base);
        }
        job.append(&span.text, 0.0, format);
    }
    job
}
