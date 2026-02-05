use crate::{pty::Pane, App};
use ratatui::{
    prelude::*,
    style::Modifier,
    widgets::{Block, Borders, Paragraph},
};

const LOGO: &str = r#" _____                             ___________
  __  /___________________ ____________(_)__  /
 _  __/  _ \_  ___/_  __ `__ \  __ \_  /__  /
/ /_ /  __/  /   _  / / / / / /_/ /  / _  /
\__/ \___//_/    /_/ /_/ /_/\____//_/  /_/"#;
const PURPLE: Color = Color::Rgb(147, 112, 219);
const CYAN: Color = Color::Rgb(80, 210, 255);
const BG: Color = Color::Rgb(20, 15, 30);
const DIM: Color = Color::Rgb(50, 45, 70);
const ALERT: Color = Color::Rgb(255, 80, 80);
const ALERT_DIM: Color = Color::Rgb(80, 30, 30);

pub fn grid_dimensions(count: usize) -> (usize, usize) {
    match count {
        0 | 1 => (1, 1),
        2 => (1, 2),
        3 | 4 => (2, 2),
        5 | 6 => (2, 3),
        _ => (3, 3),
    }
}

pub fn compute_pane_areas(area: Rect, count: usize) -> Vec<Rect> {
    if count == 0 {
        return vec![];
    }
    let (rows, cols) = grid_dimensions(count);
    let row_constraints: Vec<Constraint> = (0..rows)
        .map(|_| Constraint::Ratio(1, rows as u32))
        .collect();
    let row_areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints(row_constraints)
        .split(area);

    let mut areas = Vec::new();
    let mut idx = 0;
    for r in 0..rows {
        let remaining = count - idx;
        let items_this_row = if r == rows - 1 {
            remaining
        } else {
            cols.min(remaining)
        };

        let col_constraints: Vec<Constraint> = (0..items_this_row)
            .map(|_| Constraint::Ratio(1, items_this_row as u32))
            .collect();

        let col_areas = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(col_constraints)
            .split(row_areas[r]);

        for c in 0..items_this_row {
            areas.push(col_areas[c]);
            idx += 1;
        }
    }
    areas
}

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();

    if app.zoomed && !app.panes.is_empty() {
        draw_zoomed(frame, app, area);
    } else {
        draw_grid(frame, app, area);
    }
}

fn pane_inner_area(area: Rect) -> Rect {
    Rect::new(
        area.x.saturating_add(1),
        area.y.saturating_add(1),
        area.width.saturating_sub(2),
        area.height.saturating_sub(2),
    )
}

fn vt_color_to_tui(color: vt100::Color) -> Color {
    match color {
        vt100::Color::Default => Color::Reset,
        vt100::Color::Idx(idx) => match idx {
            0 => Color::Black,
            1 => Color::Red,
            2 => Color::Green,
            3 => Color::Yellow,
            4 => Color::Blue,
            5 => Color::Magenta,
            6 => Color::Cyan,
            7 => Color::Gray,
            8 => Color::DarkGray,
            9 => Color::LightRed,
            10 => Color::LightGreen,
            11 => Color::LightYellow,
            12 => Color::LightBlue,
            13 => Color::LightMagenta,
            14 => Color::LightCyan,
            15 => Color::White,
            _ => Color::Indexed(idx),
        },
        vt100::Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}

fn vt_cell_style(cell: &vt100::Cell) -> Style {
    let mut style = Style::default()
        .fg(vt_color_to_tui(cell.fgcolor()))
        .bg(vt_color_to_tui(cell.bgcolor()));

    let mut modifier = Modifier::empty();
    if cell.bold() {
        modifier.insert(Modifier::BOLD);
    }
    if cell.italic() {
        modifier.insert(Modifier::ITALIC);
    }
    if cell.underline() {
        modifier.insert(Modifier::UNDERLINED);
    }
    if cell.inverse() {
        modifier.insert(Modifier::REVERSED);
    }
    if !modifier.is_empty() {
        style = style.add_modifier(modifier);
    }

    style
}

fn render_pane_cells(frame: &mut Frame, pane: &Pane, area: Rect) {
    let inner = pane_inner_area(area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let buffer = frame.buffer_mut();
    for y in 0..inner.height {
        for x in 0..inner.width {
            let cell = &mut buffer[(inner.x + x, inner.y + y)];
            if let Some(src) = pane.cell(y, x) {
                if src.is_wide_continuation() {
                    cell.set_symbol(" ");
                } else {
                    let symbol = src.contents();
                    if symbol.is_empty() {
                        cell.set_symbol(" ");
                    } else {
                        cell.set_symbol(&symbol);
                    }
                }
                cell.set_style(vt_cell_style(src));
            } else {
                cell.reset();
            }
        }
    }
}

fn draw_grid(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(7), Constraint::Min(0)])
        .split(area);

    let header_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CYAN))
        .style(Style::default().bg(BG));

    let logo = Paragraph::new(LOGO)
        .style(Style::default().fg(CYAN))
        .alignment(Alignment::Center)
        .block(header_block);

    frame.render_widget(logo, chunks[0]);

    if app.panes.is_empty() {
        let empty_block = Block::default()
            .title(" No panes - press 'n' to spawn a shell ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(DIM))
            .style(Style::default().bg(BG));

        let hint = Paragraph::new("n: new shell  |  q: quit")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .block(empty_block);

        frame.render_widget(hint, chunks[1]);
        return;
    }

    let pane_areas = compute_pane_areas(chunks[1], app.panes.len());

    for (i, pane) in app.panes.iter().enumerate() {
        let is_selected = i == app.selected;
        let needs_attention = app.attention.get(i).copied().unwrap_or(false);
        let blink_on = (app.tick / 5) % 2 == 0;

        let border_color = if needs_attention && is_selected {
            if blink_on {
                ALERT
            } else {
                PURPLE
            }
        } else if needs_attention {
            if blink_on {
                ALERT
            } else {
                ALERT_DIM
            }
        } else if is_selected {
            PURPLE
        } else {
            DIM
        };

        let title = if needs_attention {
            format!(" shell {} [!] ", i + 1)
        } else {
            format!(" shell {} ", i + 1)
        };

        let block = Block::default()
            .title(Line::from(title).style(Style::default().fg(CYAN)))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(BG));

        frame.render_widget(block, pane_areas[i]);
        render_pane_cells(frame, pane, pane_areas[i]);
    }
}

fn draw_zoomed(frame: &mut Frame, app: &App, area: Rect) {
    let pane = &app.panes[app.selected];
    let mouse_mode = if app.mouse_capture_enabled {
        "mouse:on"
    } else {
        "mouse:off"
    };

    let title = format!(
        " shell {} (Ctrl+Space exit | F2 {} ) ",
        app.selected + 1,
        mouse_mode
    );
    let block = Block::default()
        .title(Line::from(title).style(Style::default().fg(CYAN)))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(PURPLE))
        .style(Style::default().bg(BG));

    frame.render_widget(block, area);
    render_pane_cells(frame, pane, area);

    let (row, col) = pane.cursor_position();
    let inner = pane_inner_area(area);
    if !pane.hide_cursor() && inner.width > 0 && inner.height > 0 {
        let cx = inner.x + col.min(inner.width - 1);
        let cy = inner.y + row.min(inner.height - 1);
        frame.set_cursor_position((cx, cy));
    }
}
