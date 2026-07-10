//! Unicode-aware text measurement. ALL width math in forge-tui goes through
//! here — mixing `str::len` with display columns is how CJK/emoji shear box
//! borders. Terminals still disagree on emoji width; that is documented, not
//! chased.

use std::borrow::Cow;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// Display width of a string in terminal cells.
pub fn width(s: &str) -> usize {
    UnicodeWidthStr::width(s)
}

/// Truncate to at most `max` display cells, appending `…` when truncated.
pub fn truncate(s: &str, max: usize) -> Cow<'_, str> {
    if width(s) <= max {
        return Cow::Borrowed(s);
    }
    if max == 0 {
        return Cow::Borrowed("");
    }
    let mut out = String::new();
    let mut w = 0;
    for g in s.graphemes(true) {
        let gw = width(g);
        if w + gw > max.saturating_sub(1) {
            break;
        }
        out.push_str(g);
        w += gw;
    }
    out.push('…');
    Cow::Owned(out)
}

/// Word-wrap into lines of at most `max` display cells. Existing newlines are
/// respected; words longer than a line hard-break on grapheme boundaries.
pub fn wrap(s: &str, max: usize) -> Vec<String> {
    if max == 0 {
        return vec![String::new()];
    }
    let mut lines = Vec::new();
    for raw in s.split('\n') {
        let mut line = String::new();
        let mut line_w = 0;
        for word in raw.split_word_bounds() {
            let ww = width(word);
            if line_w + ww <= max {
                line.push_str(word);
                line_w += ww;
                continue;
            }
            // Flush the current line (dropping a trailing space-only word).
            if !line.is_empty() {
                lines.push(std::mem::take(&mut line));
                line_w = 0;
            }
            if word.trim().is_empty() {
                continue; // don't carry whitespace onto the next line
            }
            if ww <= max {
                line.push_str(word);
                line_w = ww;
            } else {
                // Hard-break an over-long word on grapheme boundaries.
                for g in word.graphemes(true) {
                    let gw = width(g);
                    if line_w + gw > max {
                        lines.push(std::mem::take(&mut line));
                        line_w = 0;
                    }
                    line.push_str(g);
                    line_w += gw;
                }
            }
        }
        lines.push(line);
    }
    lines
}

/// Pad or truncate to exactly `w` display cells (left-aligned).
pub fn fit(s: &str, w: usize) -> String {
    let t = truncate(s, w);
    let tw = width(&t);
    let mut out = t.into_owned();
    out.extend(std::iter::repeat(' ').take(w.saturating_sub(tw)));
    out
}
