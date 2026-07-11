//! Whole-document markdown import/export. JSON is the canonical interchange;
//! markdown is for clipboard interop and export. Lossy cases: columns
//! flatten sequentially, custom blocks travel as ```block:<kind> JSON fences.

use crate::schema::{Block, BlockKind, Document, ListStyle, Tone};

fn tone_tag(tone: Tone) -> &'static str {
    match tone {
        Tone::Info => "INFO",
        Tone::Success => "SUCCESS",
        Tone::Warning => "WARNING",
        Tone::Danger => "DANGER",
    }
}

fn tone_from_tag(tag: &str) -> Option<Tone> {
    match tag.to_ascii_uppercase().as_str() {
        "INFO" | "NOTE" | "TIP" => Some(Tone::Info),
        "SUCCESS" => Some(Tone::Success),
        "WARNING" | "IMPORTANT" => Some(Tone::Warning),
        "DANGER" | "CAUTION" => Some(Tone::Danger),
        _ => None,
    }
}

/// Render the document as markdown text.
pub fn to_markdown(doc: &Document) -> String {
    let mut out = String::new();
    let mut counter = 0usize; // ordered-list numbering per run
    let blocks = &doc.blocks;
    for (i, block) in blocks.iter().enumerate() {
        let prev_is_list = i > 0 && matches!(blocks[i - 1].kind, BlockKind::ListItem { .. });
        let this_is_list = matches!(block.kind, BlockKind::ListItem { .. });
        if i > 0 && !(prev_is_list && this_is_list) {
            out.push('\n');
        }
        if !this_is_list {
            counter = 0;
        }
        block_to_markdown(block, &mut out, &mut counter);
    }
    out
}

fn block_to_markdown(block: &Block, out: &mut String, counter: &mut usize) {
    match &block.kind {
        BlockKind::Paragraph { md } => {
            out.push_str(md);
            out.push('\n');
        }
        BlockKind::Heading { level, md } => {
            for _ in 0..*level {
                out.push('#');
            }
            out.push(' ');
            out.push_str(md);
            out.push('\n');
        }
        BlockKind::ListItem {
            style,
            checked,
            indent,
            md,
        } => {
            for _ in 0..(*indent as usize * 2) {
                out.push(' ');
            }
            match style {
                ListStyle::Bullet => out.push_str("- "),
                ListStyle::Number => {
                    *counter += 1;
                    out.push_str(&format!("{counter}. "));
                }
                ListStyle::Todo => {
                    out.push_str(if checked.unwrap_or(false) {
                        "- [x] "
                    } else {
                        "- [ ] "
                    });
                }
            }
            out.push_str(md);
            out.push('\n');
        }
        BlockKind::Quote { md } => {
            for line in md.split('\n') {
                out.push_str("> ");
                out.push_str(line);
                out.push('\n');
            }
        }
        BlockKind::Divider => out.push_str("---\n"),
        BlockKind::Code { lang, code } => {
            out.push_str("```");
            out.push_str(lang);
            out.push('\n');
            out.push_str(code);
            if !code.ends_with('\n') {
                out.push('\n');
            }
            out.push_str("```\n");
        }
        BlockKind::Table { header, rows } => {
            out.push('|');
            for cell in header {
                out.push(' ');
                out.push_str(cell);
                out.push_str(" |");
            }
            out.push('\n');
            out.push('|');
            for _ in header {
                out.push_str(" --- |");
            }
            out.push('\n');
            for row in rows {
                out.push('|');
                for cell in row {
                    out.push(' ');
                    out.push_str(cell);
                    out.push_str(" |");
                }
                out.push('\n');
            }
        }
        BlockKind::Admonition { tone, title, md } => {
            out.push_str("> [!");
            out.push_str(tone_tag(*tone));
            out.push(']');
            if !title.is_empty() {
                out.push(' ');
                out.push_str(title);
            }
            out.push('\n');
            for line in md.split('\n') {
                out.push_str("> ");
                out.push_str(line);
                out.push('\n');
            }
        }
        BlockKind::Columns { columns } => {
            // Lossy: columns flatten sequentially.
            let mut first = true;
            for col in columns {
                for b in &col.blocks {
                    if !first {
                        out.push('\n');
                    }
                    first = false;
                    block_to_markdown(b, out, counter);
                }
            }
        }
        BlockKind::Custom { kind, data } => {
            out.push_str("```block:");
            out.push_str(kind);
            out.push('\n');
            out.push_str(&serde_json::to_string_pretty(data).unwrap_or_default());
            out.push_str("\n```\n");
        }
    }
}

/// Parse markdown text into a document (line scanner keeping raw inline
/// source — the inverse of [`to_markdown`] for everything except columns).
pub fn from_markdown(text: &str) -> Document {
    let lines: Vec<&str> = text.lines().collect();
    let mut blocks: Vec<Block> = Vec::new();
    let mut i = 0usize;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim_start();
        if trimmed.is_empty() {
            i += 1;
            continue;
        }

        // Fenced code (plain or ```block:<kind> custom payload).
        if let Some(info) = trimmed.strip_prefix("```") {
            let info = info.trim();
            let mut body = Vec::new();
            i += 1;
            while i < lines.len() && !lines[i].trim_start().starts_with("```") {
                body.push(lines[i]);
                i += 1;
            }
            i += 1; // closing fence
            let body = body.join("\n");
            if let Some(kind) = info.strip_prefix("block:") {
                let data = serde_json::from_str(&body).unwrap_or(serde_json::Value::Null);
                blocks.push(Block::new(BlockKind::Custom {
                    kind: kind.to_string(),
                    data,
                }));
            } else {
                blocks.push(Block::new(BlockKind::Code {
                    lang: info.to_string(),
                    code: body,
                }));
            }
            continue;
        }

        // Heading.
        if let Some(rest) = heading(trimmed) {
            blocks.push(Block::new(rest));
            i += 1;
            continue;
        }

        // Divider.
        if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            blocks.push(Block::new(BlockKind::Divider));
            i += 1;
            continue;
        }

        // Blockquote group → admonition (first line `[!TONE] Title`) or quote.
        if trimmed.starts_with('>') {
            let mut body: Vec<String> = Vec::new();
            while i < lines.len() {
                let l = lines[i].trim_start();
                if let Some(rest) = l.strip_prefix("> ") {
                    body.push(rest.to_string());
                } else if let Some(rest) = l.strip_prefix('>') {
                    body.push(rest.to_string());
                } else {
                    break;
                }
                i += 1;
            }
            let first = body.first().cloned().unwrap_or_default();
            let alert = first.strip_prefix("[!").and_then(|r| {
                let (tag, title) = r.split_once(']')?;
                Some((tone_from_tag(tag)?, title.trim().to_string()))
            });
            if let Some((tone, title)) = alert {
                blocks.push(Block::new(BlockKind::Admonition {
                    tone,
                    title,
                    md: body[1..].join("\n"),
                }));
            } else {
                blocks.push(Block::new(BlockKind::Quote {
                    md: body.join("\n"),
                }));
            }
            continue;
        }

        // List item.
        if let Some(kind) = list_item(line) {
            blocks.push(Block::new(kind));
            i += 1;
            continue;
        }

        // Pipe table (header + separator line).
        if trimmed.starts_with('|')
            && i + 1 < lines.len()
            && is_table_separator(lines[i + 1].trim())
        {
            let header = split_row(trimmed);
            i += 2;
            let mut rows = Vec::new();
            while i < lines.len() && lines[i].trim_start().starts_with('|') {
                rows.push(split_row(lines[i].trim()));
                i += 1;
            }
            blocks.push(Block::new(BlockKind::Table { header, rows }));
            continue;
        }

        // Paragraph: consecutive plain lines join with soft breaks.
        let mut para = vec![line.trim_end()];
        i += 1;
        while i < lines.len() {
            let l = lines[i];
            let t = l.trim_start();
            if t.is_empty()
                || t.starts_with("```")
                || t.starts_with('>')
                || t.starts_with('|')
                || heading(t).is_some()
                || list_item(l).is_some()
                || t == "---"
            {
                break;
            }
            para.push(l.trim_end());
            i += 1;
        }
        blocks.push(Block::new(BlockKind::Paragraph {
            md: para.join("\n"),
        }));
    }

    Document::from_blocks(blocks)
}

fn heading(line: &str) -> Option<BlockKind> {
    let hashes = line.bytes().take_while(|b| *b == b'#').count();
    if (1..=4).contains(&hashes) && line.as_bytes().get(hashes) == Some(&b' ') {
        Some(BlockKind::Heading {
            level: hashes as u8,
            md: line[hashes + 1..].to_string(),
        })
    } else {
        None
    }
}

fn list_item(line: &str) -> Option<BlockKind> {
    let spaces = line.len() - line.trim_start_matches(' ').len();
    let indent = ((spaces / 2) as u8).min(5);
    let rest = &line[spaces..];
    for (p, checked) in [("- [ ] ", false), ("- [x] ", true), ("- [X] ", true)] {
        if let Some(md) = rest.strip_prefix(p) {
            return Some(BlockKind::ListItem {
                style: ListStyle::Todo,
                checked: Some(checked),
                indent,
                md: md.to_string(),
            });
        }
    }
    for p in ["- ", "* ", "+ "] {
        if let Some(md) = rest.strip_prefix(p) {
            return Some(BlockKind::ListItem {
                style: ListStyle::Bullet,
                checked: None,
                indent,
                md: md.to_string(),
            });
        }
    }
    let digits = rest.bytes().take_while(|b| b.is_ascii_digit()).count();
    if digits > 0 {
        for sep in [". ", ") "] {
            if let Some(md) = rest[digits..].strip_prefix(sep) {
                return Some(BlockKind::ListItem {
                    style: ListStyle::Number,
                    checked: None,
                    indent,
                    md: md.to_string(),
                });
            }
        }
    }
    None
}

fn is_table_separator(line: &str) -> bool {
    line.starts_with('|')
        && !line.is_empty()
        && line.chars().all(|c| matches!(c, '|' | '-' | ':' | ' '))
        && line.contains('-')
}

fn split_row(line: &str) -> Vec<String> {
    line.trim()
        .trim_start_matches('|')
        .trim_end_matches('|')
        .split('|')
        .map(|c| c.trim().to_string())
        .collect()
}
