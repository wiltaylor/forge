//! Inline markdown parsing shared by the TUI and egui renderers: one flat
//! span list per `md` string. Kits map [`InlineSpan`] onto their own text
//! types (ratatui `Span` / egui `LayoutJob`) with small adapters.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::emoji::resolve_shortcodes;

/// A styled run of text. `link` carries a safe (http/https/mailto) URL;
/// unsafe URLs degrade to plain text. Newlines appear inside `text` for
/// soft/hard breaks.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InlineSpan {
    pub text: String,
    pub strong: bool,
    pub emphasis: bool,
    pub strike: bool,
    pub code: bool,
    pub link: Option<String>,
}

/// Allow http/https/mailto only — parity with the web kit's `safeUrl`.
pub fn safe_url(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("mailto:")
}

/// Parse one block's inline markdown into styled spans. `:shortcode:` emoji
/// resolve inside plain text runs (never inside code spans). Raw HTML stays
/// literal text.
pub fn parse_inline(md: &str) -> Vec<InlineSpan> {
    let mut spans: Vec<InlineSpan> = Vec::new();
    let mut style = InlineSpan::default();
    let mut depth = (0u32, 0u32, 0u32); // strong, em, strike
    let parser = Parser::new_ext(md, Options::ENABLE_STRIKETHROUGH);

    let push = |spans: &mut Vec<InlineSpan>, text: &str, style: &InlineSpan, code: bool| {
        if text.is_empty() {
            return;
        }
        let text = if code {
            text.to_string()
        } else {
            resolve_shortcodes(text).into_owned()
        };
        // Merge with the previous span when styling is identical.
        if let Some(last) = spans.last_mut() {
            if last.strong == style.strong
                && last.emphasis == style.emphasis
                && last.strike == style.strike
                && last.code == code
                && last.link == style.link
            {
                last.text.push_str(&text);
                return;
            }
        }
        spans.push(InlineSpan {
            text,
            code,
            ..style.clone()
        });
    };

    for event in parser {
        match event {
            Event::Start(Tag::Strong) => depth.0 += 1,
            Event::End(TagEnd::Strong) => depth.0 = depth.0.saturating_sub(1),
            Event::Start(Tag::Emphasis) => depth.1 += 1,
            Event::End(TagEnd::Emphasis) => depth.1 = depth.1.saturating_sub(1),
            Event::Start(Tag::Strikethrough) => depth.2 += 1,
            Event::End(TagEnd::Strikethrough) => depth.2 = depth.2.saturating_sub(1),
            Event::Start(Tag::Link { dest_url, .. }) => {
                style.link = safe_url(&dest_url).then(|| dest_url.to_string());
            }
            Event::End(TagEnd::Link) => style.link = None,
            Event::Start(Tag::Image { dest_url, .. }) => {
                // No image rendering in text kits — show as a link-styled tag.
                style.link = safe_url(&dest_url).then(|| dest_url.to_string());
            }
            Event::End(TagEnd::Image) => style.link = None,
            Event::Text(t) | Event::Html(t) | Event::InlineHtml(t) => {
                style.strong = depth.0 > 0;
                style.emphasis = depth.1 > 0;
                style.strike = depth.2 > 0;
                push(&mut spans, &t, &style, false);
            }
            Event::Code(t) => {
                style.strong = depth.0 > 0;
                style.emphasis = depth.1 > 0;
                style.strike = depth.2 > 0;
                push(&mut spans, &t, &style, true);
            }
            Event::SoftBreak | Event::HardBreak => {
                push(&mut spans, "\n", &style, false);
            }
            _ => {}
        }
    }
    spans
}

/// Greedy word-wrap of styled spans into display lines no wider than `width`
/// terminal cells. Explicit `\n` in span text forces a break; overlong words
/// split at grapheme boundaries. `width == 0` yields one line per `\n` run.
pub fn wrap_spans(spans: &[InlineSpan], width: usize) -> Vec<Vec<InlineSpan>> {
    let mut lines: Vec<Vec<InlineSpan>> = vec![Vec::new()];
    let mut used = 0usize;

    let emit =
        |lines: &mut Vec<Vec<InlineSpan>>, used: &mut usize, piece: &str, style: &InlineSpan| {
            let w = piece.width();
            // Trailing spaces may hang past the edge — they don't count.
            let w_visible = piece.trim_end_matches(' ').width();
            if width > 0 && *used + w_visible > width && *used > 0 {
                lines.push(Vec::new());
                *used = 0;
            }
            // A single token wider than the whole line: hard-split by grapheme.
            if width > 0 && w_visible > width {
                for g in piece.graphemes(true) {
                    let gw = g.width();
                    if *used + gw > width && *used > 0 {
                        lines.push(Vec::new());
                        *used = 0;
                    }
                    append(lines.last_mut().unwrap(), g, style);
                    *used += gw;
                }
                return;
            }
            // Skip leading spaces on a fresh wrapped line.
            if *used == 0 && piece.chars().all(|c| c == ' ') {
                return;
            }
            append(lines.last_mut().unwrap(), piece, style);
            *used += w;
        };

    fn append(line: &mut Vec<InlineSpan>, text: &str, style: &InlineSpan) {
        if let Some(last) = line.last_mut() {
            if last.strong == style.strong
                && last.emphasis == style.emphasis
                && last.strike == style.strike
                && last.code == style.code
                && last.link == style.link
            {
                last.text.push_str(text);
                return;
            }
        }
        line.push(InlineSpan {
            text: text.to_string(),
            ..style.clone()
        });
    }

    for span in spans {
        for (i, seg) in span.text.split('\n').enumerate() {
            if i > 0 {
                lines.push(Vec::new());
                used = 0;
            }
            // Word tokens keep their trailing spaces so widths stay honest.
            let mut word = String::new();
            for ch in seg.chars() {
                word.push(ch);
                if ch == ' ' {
                    emit(&mut lines, &mut used, &word, span);
                    word.clear();
                }
            }
            if !word.is_empty() {
                emit(&mut lines, &mut used, &word, span);
            }
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(spans: &[InlineSpan]) -> String {
        spans.iter().map(|s| s.text.as_str()).collect()
    }

    #[test]
    fn parses_styles() {
        let spans = parse_inline("a **b** *c* ~~d~~ `e` [f](https://x.dev)");
        assert_eq!(text(&spans), "a b c d e f");
        assert!(spans.iter().any(|s| s.strong && s.text == "b"));
        assert!(spans.iter().any(|s| s.emphasis && s.text == "c"));
        assert!(spans.iter().any(|s| s.strike && s.text == "d"));
        assert!(spans.iter().any(|s| s.code && s.text == "e"));
        assert!(spans
            .iter()
            .any(|s| s.link.as_deref() == Some("https://x.dev") && s.text == "f"));
    }

    #[test]
    fn unsafe_links_degrade_to_text() {
        let spans = parse_inline("[x](javascript:alert(1))");
        assert!(spans.iter().all(|s| s.link.is_none()));
        assert_eq!(text(&spans), "x");
    }

    #[test]
    fn html_stays_literal() {
        let spans = parse_inline("a <script>alert(1)</script> b");
        assert!(text(&spans).contains("<script>"));
    }

    #[test]
    fn emoji_resolve_outside_code() {
        let spans = parse_inline("go :rocket: `:rocket:`");
        assert!(spans
            .iter()
            .any(|s| !s.code && s.text.contains('\u{1F680}')));
        assert!(spans.iter().any(|s| s.code && s.text == ":rocket:"));
    }

    #[test]
    fn wraps_by_cells() {
        let spans = parse_inline("aaa bbb ccc");
        let lines = wrap_spans(&spans, 7);
        assert_eq!(lines.len(), 2);
        assert_eq!(text(&lines[0]).trim_end(), "aaa bbb");
        assert_eq!(text(&lines[1]), "ccc");
    }

    #[test]
    fn wraps_wide_graphemes() {
        let spans = vec![InlineSpan {
            text: "你好世界".into(),
            ..Default::default()
        }];
        let lines = wrap_spans(&spans, 4);
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn newline_forces_break() {
        let spans = parse_inline("a\nb");
        let lines = wrap_spans(&spans, 80);
        assert_eq!(lines.len(), 2);
    }
}
