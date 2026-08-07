use crate::forge::Theme;
use ratatui::{
    layout::{Constraint, Layout},
    style::Style,
    widgets::{Block, Borders, Paragraph, Row, Table},
    DefaultTerminal,
};

pub fn run(terminal: &mut DefaultTerminal) -> std::io::Result<()> {
    let theme = Theme::dark();
    terminal.draw(|f| {
        let [bar, body] =
            Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(f.area());

        f.render_widget(
            Paragraph::new(" opsview ").style(Style::new().bg(theme.bg_1).fg(theme.fg_2)),
            bar,
        );

        let rows = [Row::new(vec!["ingest", "us-east-1", "ok"])];
        f.render_widget(
            Table::new(rows, [Constraint::Fill(1); 3])
                .block(Block::new().borders(Borders::ALL).border_style(Style::new().fg(theme.border)))
                .style(Style::new().bg(theme.bg_0).fg(theme.fg_0)),
            body,
        );
    })?;
    Ok(())
}
