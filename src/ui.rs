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

    if app.zoomed && !app.panes.is_empty() {
        draw_zoomed(frame, app, area);
    } else {
        draw_grid(frame, app, area);
    }
}

fn draw_grid(frame: &mut Frame, app: &App, area: Rect) {
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

    if app.panes.is_empty() {
        let empty_block = Block::default()
            .title(" No panes - press 'n' to spawn a shell ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(100, 80, 140)))
            .style(Style::default().bg(Color::Rgb(20, 15, 30)));

        let hint = Paragraph::new("n: new shell  |  q: quit")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .block(empty_block);

        frame.render_widget(hint, chunks[1]);
        return;
    }

    let pane_constraints: Vec<Constraint> = app
        .panes
        .iter()
        .map(|_| Constraint::Ratio(1, app.panes.len() as u32))
        .collect();

    let pane_areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints(pane_constraints)
        .split(chunks[1]);

    for (i, pane) in app.panes.iter().enumerate() {
        let is_selected = i == app.selected;
        let needs_attention = app.attention.get(i).copied().unwrap_or(false);
        let blink_on = (app.tick / 5) % 2 == 0;

        let border_color = if needs_attention && is_selected {
            if blink_on { Color::Rgb(255, 80, 80) } else { Color::Rgb(147, 112, 219) }
        } else if needs_attention {
            if blink_on { Color::Rgb(255, 80, 80) } else { Color::Rgb(80, 30, 30) }
        } else if is_selected {
            Color::Rgb(147, 112, 219)
        } else {
            Color::Rgb(60, 50, 80)
        };

        let title = if needs_attention {
            format!(" shell {} [!] ", i + 1)
        } else {
            format!(" shell {} ", i + 1)
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(Color::Rgb(20, 15, 30)));

        let content = Paragraph::new(pane.screen_contents())
            .style(Style::default().fg(Color::White))
            .block(block);

        frame.render_widget(content, pane_areas[i]);
    }
}

fn draw_zoomed(frame: &mut Frame, app: &App, area: Rect) {
    let pane = &app.panes[app.selected];

    let block = Block::default()
        .title(format!(" shell {} (Ctrl+Space to exit) ", app.selected + 1))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(147, 112, 219)))
        .style(Style::default().bg(Color::Rgb(20, 15, 30)));

    let content = Paragraph::new(pane.screen_contents())
        .style(Style::default().fg(Color::White))
        .block(block);

    frame.render_widget(content, area);

    let (row, col) = pane.cursor_position();
    frame.set_cursor_position((area.x + 1 + col, area.y + 1 + row));
}
