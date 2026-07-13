//! Markdown rendering (cargo feature `markdown`, pulldown-cmark). Maps the
//! Forge web styles onto terminal text: bold headings, tinted code, `▎`
//! blockquotes, accent links (text only — cell buffers can't carry OSC-8
//! hyperlinks).

use crate::text;
use crate::theme::{default_theme, Theme};
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

struct Builder<'t> {
    t: &'t Theme,
    width: usize,
    lines: Vec<Line<'static>>,
    cur: Vec<Span<'static>>,
    cur_w: usize,
    /// (ordered counter) per open list; None = bullet.
    lists: Vec<Option<u64>>,
    quote: usize,
    code: bool,
    styles: Vec<Style>,
}

impl<'t> Builder<'t> {
    fn new(t: &'t Theme, width: usize) -> Builder<'t> {
        Builder {
            t,
            width: width.max(8),
            lines: Vec::new(),
            cur: Vec::new(),
            cur_w: 0,
            lists: Vec::new(),
            quote: 0,
            code: false,
            styles: vec![Style::new().fg(t.fg[1])],
        }
    }

    fn style(&self) -> Style {
        *self.styles.last().unwrap()
    }

    fn push_style(&mut self, f: impl Fn(Style) -> Style) {
        self.styles.push(f(self.style()));
    }

    fn pop_style(&mut self) {
        if self.styles.len() > 1 {
            self.styles.pop();
        }
    }

    fn prefix(&self) -> (String, Style) {
        let mut p = String::new();
        for _ in 0..self.quote {
            p.push_str("▎ ");
        }
        p.push_str(&"  ".repeat(self.lists.len().saturating_sub(1)));
        (p, Style::new().fg(self.t.fg[3]))
    }

    fn flush(&mut self) {
        let spans = std::mem::take(&mut self.cur);
        self.cur_w = 0;
        if spans.is_empty() {
            self.lines.push(Line::default());
        } else {
            self.lines.push(Line::from(spans));
        }
    }

    fn blank(&mut self) {
        if !self.cur.is_empty() {
            self.flush();
        }
        if !matches!(self.lines.last(), Some(l) if l.spans.is_empty()) && !self.lines.is_empty() {
            self.lines.push(Line::default());
        }
    }

    fn start_line(&mut self) {
        let (p, style) = self.prefix();
        if !p.is_empty() {
            self.cur_w = text::width(&p);
            self.cur.push(Span::styled(p, style));
        }
    }

    /// Word-wrapping styled text append.
    fn push_text(&mut self, s: &str, style: Style) {
        for token in s.split_inclusive(char::is_whitespace) {
            let (word, ws) = match token.strip_suffix(char::is_whitespace) {
                Some(w) => (w, true),
                None => (token, false),
            };
            let ww = text::width(word);
            if self.cur_w > 0 && self.cur_w + ww > self.width {
                self.flush();
                self.start_line();
            }
            if self.cur.is_empty() && self.cur_w == 0 {
                self.start_line();
            }
            if !word.is_empty() {
                self.cur.push(Span::styled(word.to_owned(), style));
                self.cur_w += ww;
            }
            if ws && self.cur_w < self.width {
                self.cur.push(Span::styled(" ".to_owned(), style));
                self.cur_w += 1;
            }
        }
    }

    fn code_line(&mut self, line: &str) {
        let style = Style::new().fg(self.t.fg[1]).bg(self.t.bg[2]);
        let padded = format!("  {}", line);
        let padded = text::fit(&padded, self.width);
        self.lines.push(Line::from(Span::styled(padded, style)));
    }
}

/// Build styled lines from markdown source (also used by the chat kit).
pub fn markdown_lines(source: &str, width: usize, t: &Theme) -> Vec<Line<'static>> {
    let mut b = Builder::new(t, width);
    let parser = Parser::new_ext(
        source,
        Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES,
    );
    let mut code_buf = String::new();
    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                b.blank();
                let style = match level {
                    HeadingLevel::H1 => Style::new()
                        .fg(t.fg[0])
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                    HeadingLevel::H2 => Style::new().fg(t.fg[0]).add_modifier(Modifier::BOLD),
                    _ => Style::new().fg(t.fg[1]).add_modifier(Modifier::BOLD),
                };
                b.styles.push(style);
            }
            Event::End(TagEnd::Heading(_)) => {
                b.pop_style();
                b.flush();
            }
            Event::Start(Tag::Paragraph) => {
                if b.lists.is_empty() {
                    b.blank();
                }
            }
            Event::End(TagEnd::Paragraph) => {
                if !b.cur.is_empty() {
                    b.flush();
                }
            }
            Event::Start(Tag::BlockQuote(_)) => {
                b.blank();
                b.quote += 1;
                b.push_style(|s| s.fg(t.fg[2]));
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                b.quote = b.quote.saturating_sub(1);
                b.pop_style();
            }
            Event::Start(Tag::List(start)) => {
                if b.lists.is_empty() {
                    b.blank();
                }
                b.lists.push(start);
            }
            Event::End(TagEnd::List(_)) => {
                b.lists.pop();
            }
            Event::Start(Tag::Item) => {
                if !b.cur.is_empty() {
                    b.flush();
                }
                b.start_line();
                let marker = match b.lists.last_mut() {
                    Some(Some(n)) => {
                        let m = format!("{n}. ");
                        *n += 1;
                        m
                    }
                    _ => "• ".to_string(),
                };
                b.cur_w += text::width(&marker);
                b.cur.push(Span::styled(marker, Style::new().fg(t.fg[2])));
            }
            Event::End(TagEnd::Item) => {
                if !b.cur.is_empty() {
                    b.flush();
                }
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                b.blank();
                b.code = true;
                code_buf.clear();
                if let CodeBlockKind::Fenced(lang) = kind {
                    if !lang.is_empty() {
                        b.lines.push(Line::from(Span::styled(
                            format!("  {lang}"),
                            Style::new().fg(t.fg[3]),
                        )));
                    }
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                for line in code_buf.trim_end_matches('\n').split('\n') {
                    b.code_line(line);
                }
                b.code = false;
            }
            Event::Start(Tag::Emphasis) => b.push_style(|s| s.add_modifier(Modifier::ITALIC)),
            Event::End(TagEnd::Emphasis) => b.pop_style(),
            Event::Start(Tag::Strong) => {
                b.push_style(|s| s.add_modifier(Modifier::BOLD).fg(t.fg[0]))
            }
            Event::End(TagEnd::Strong) => b.pop_style(),
            Event::Start(Tag::Strikethrough) => {
                b.push_style(|s| s.add_modifier(Modifier::CROSSED_OUT))
            }
            Event::End(TagEnd::Strikethrough) => b.pop_style(),
            Event::Start(Tag::Link { .. }) => {
                b.push_style(|s| s.fg(t.accent.fg).add_modifier(Modifier::UNDERLINED))
            }
            Event::End(TagEnd::Link) => b.pop_style(),
            Event::Rule => {
                b.blank();
                b.lines.push(Line::from(Span::styled(
                    "─".repeat(b.width),
                    Style::new().fg(t.border.default),
                )));
            }
            Event::Text(s) => {
                if b.code {
                    code_buf.push_str(&s);
                } else {
                    let style = b.style();
                    b.push_text(&s, style);
                }
            }
            Event::Code(s) => {
                let style = Style::new().fg(t.accent.fg).bg(t.bg[3]);
                b.push_text(&s, style);
            }
            Event::SoftBreak => {
                let style = b.style();
                b.push_text(" ", style);
            }
            Event::HardBreak => {
                b.flush();
            }
            _ => {}
        }
    }
    if !b.cur.is_empty() {
        b.flush();
    }
    // Trim leading blank line.
    if matches!(b.lines.first(), Some(l) if l.spans.is_empty()) {
        b.lines.remove(0);
    }
    b.lines
}

/// Markdown block widget with a scroll offset. Measure with
/// [`Markdown::height`] when stacking.
#[derive(Clone, Debug)]
pub struct Markdown<'a> {
    source: &'a str,
    scroll: u16,
    theme: Option<&'a Theme>,
}

impl<'a> Markdown<'a> {
    pub fn new(source: &'a str) -> Markdown<'a> {
        Markdown {
            source,
            scroll: 0,
            theme: None,
        }
    }

    pub fn scroll(mut self, scroll: u16) -> Self {
        self.scroll = scroll;
        self
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Total rows at `width` cells.
    pub fn height(&self, width: u16) -> u16 {
        let t = self.theme.unwrap_or_else(|| default_theme());
        markdown_lines(self.source, width as usize, t).len() as u16
    }
}

impl Widget for Markdown<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        let lines = markdown_lines(self.source, area.width as usize, t);
        for (i, line) in lines.iter().skip(self.scroll as usize).enumerate() {
            if i as u16 >= area.height {
                break;
            }
            buf.set_line(area.x, area.y + i as u16, line, area.width);
        }
    }
}
