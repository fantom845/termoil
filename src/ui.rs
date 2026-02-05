use crate::App;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

const LOGO: &str = r#" _____                             ___________
  __  /___________________ ____________(_)__  /
 _  __/  _ \_  ___/_  __ `__ \  __ \_  /__  /
/ /_ /  __/  /   _  / / / / / /_/ /  / _  /
\__/ \___//_/    /_/ /_/ /_/\____//_/  /_/"#;

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(7), Constraint::Min(0)])
        .split(area);

    let header_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(147, 112, 219)))
        .style(Style::default().bg(Color::Rgb(20, 15, 30)));

    let logo = Paragraph::new(LOGO)
        .style(Style::default().fg(Color::Rgb(147, 112, 219)))
        .alignment(Alignment::Center)
        .block(header_block);

    frame.render_widget(logo, chunks[0]);

    let pane_block = Block::default()
        .title(format!(" {} ", app.target))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(100, 80, 140)))
        .style(Style::default().bg(Color::Rgb(20, 15, 30)));

    let pane_content = Paragraph::new(app.content.as_str())
        .style(Style::default().fg(Color::White))
        .block(pane_block);

    frame.render_widget(pane_content, chunks[1]);
}
