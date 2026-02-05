mod pty;
mod ui;
mod watchdog;

use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
        MouseEvent, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use pty::Pane;
use ratatui::prelude::*;
use std::io;
use std::time::Duration;

const UI_POLL_MS: u64 = 16;

#[derive(Parser)]
#[command(
    name = "termoil",
    version,
    about = "Less friction for your multi-agent workflow"
)]
struct Cli {}

fn mouse_modifier_bits(modifiers: KeyModifiers) -> u8 {
    let mut bits = 0;
    if modifiers.contains(KeyModifiers::SHIFT) {
        bits |= 4;
    }
    if modifiers.contains(KeyModifiers::ALT) {
        bits |= 8;
    }
    if modifiers.contains(KeyModifiers::CONTROL) {
        bits |= 16;
    }
    bits
}

fn encode_xterm_mouse(
    encoding: vt100::MouseProtocolEncoding,
    cb: u8,
    x: u16,
    y: u16,
    release: bool,
) -> Option<Vec<u8>> {
    match encoding {
        vt100::MouseProtocolEncoding::Sgr => {
            let suffix = if release { 'm' } else { 'M' };
            Some(format!("\x1b[<{};{};{}{}", cb, x, y, suffix).into_bytes())
        }
        vt100::MouseProtocolEncoding::Default | vt100::MouseProtocolEncoding::Utf8 => {
            let cb_enc = cb.checked_add(32)?;
            let x_enc = u8::try_from(x).ok()?.checked_add(32)?;
            let y_enc = u8::try_from(y).ok()?.checked_add(32)?;
            Some(vec![0x1b, b'[', b'M', cb_enc, x_enc, y_enc])
        }
    }
}

pub struct App {
    pub panes: Vec<Pane>,
    pub selected: usize,
    pub zoomed: bool,
    pub mouse_capture_enabled: bool,
    pub watchdog: watchdog::Watchdog,
    pub attention: Vec<bool>,
    pub tick: u64,
    pub scroll_offset: u16,
}

impl App {
    fn new() -> Self {
        Self {
            panes: Vec::new(),
            selected: 0,
            zoomed: false,
            mouse_capture_enabled: true,
            watchdog: watchdog::Watchdog::new(),
            attention: Vec::new(),
            tick: 0,
            scroll_offset: 0,
        }
    }

    fn spawn_shell(&mut self, rows: u16, cols: u16) -> Result<()> {
        let pane = Pane::spawn_shell(rows, cols)?;
        self.panes.push(pane);
        self.attention.push(false);
        self.selected = self.panes.len() - 1;
        Ok(())
    }

    fn read_pty_output(&mut self) {
        self.tick = self.tick.wrapping_add(1);
        for pane in &mut self.panes {
            pane.read_available();
        }
        if self.zoomed {
            return;
        }
        for (i, pane) in self.panes.iter().enumerate() {
            self.attention[i] = self
                .watchdog
                .needs_attention(&pane.cursor_line(), &pane.lines_near_cursor());
        }
    }

    fn resize_all_to_grid(&mut self, term_h: u16, term_w: u16) {
        if self.panes.is_empty() {
            return;
        }
        let areas = ui::compute_pane_areas(
            Rect::new(0, 7, term_w, term_h.saturating_sub(7)),
            self.panes.len(),
        );
        for (i, pane) in self.panes.iter_mut().enumerate() {
            if let Some(area) = areas.get(i) {
                pane.resize(area.height.saturating_sub(2), area.width.saturating_sub(2));
            }
        }
    }

    fn selected_grid_inner_size(&self, term_h: u16, term_w: u16) -> (u16, u16) {
        if self.panes.is_empty() {
            return (24, 80);
        }
        let areas = ui::compute_pane_areas(
            Rect::new(0, 7, term_w, term_h.saturating_sub(7)),
            self.panes.len(),
        );
        if let Some(area) = areas.get(self.selected) {
            (
                area.height.saturating_sub(2).max(1),
                area.width.saturating_sub(2).max(1),
            )
        } else {
            (24, 80)
        }
    }

    fn close_selected_pane(&mut self) {
        if self.panes.is_empty() {
            return;
        }

        if let Some(pane) = self.panes.get(self.selected) {
            pane.terminate();
        }

        self.panes.remove(self.selected);
        self.attention.remove(self.selected);

        if self.panes.is_empty() {
            self.selected = 0;
            self.zoomed = false;
            self.scroll_offset = 0;
        } else if self.selected >= self.panes.len() {
            self.selected = self.panes.len() - 1;
        }
    }

    fn restart_selected_pane(&mut self, term_h: u16, term_w: u16) {
        if self.panes.is_empty() {
            return;
        }

        let idx = self.selected;
        let (rows, cols) = self.selected_grid_inner_size(term_h, term_w);
        if let Ok(new_pane) = Pane::spawn_shell(rows, cols) {
            if let Some(old_pane) = self.panes.get(idx) {
                old_pane.terminate();
            }
            self.panes[idx] = new_pane;
            self.attention[idx] = false;
        }
    }

    fn navigate(&mut self, direction: KeyCode) {
        if self.panes.is_empty() {
            return;
        }
        let count = self.panes.len();
        let (_rows, cols) = ui::grid_dimensions(count);
        let cur_row = self.selected / cols;
        let cur_col = self.selected % cols;

        let (new_row, new_col) = match direction {
            KeyCode::Up => (cur_row.saturating_sub(1), cur_col),
            KeyCode::Down => (cur_row + 1, cur_col),
            KeyCode::Left => (cur_row, cur_col.saturating_sub(1)),
            KeyCode::Right => (cur_row, cur_col + 1),
            _ => return,
        };

        let mut new_idx = new_row * cols + new_col;
        // Clamp to last pane on the target row if column doesn't exist
        if new_idx >= count && new_row * cols < count {
            new_idx = count - 1;
        }
        if new_idx < count {
            self.selected = new_idx;
        }
    }

    fn send_key(&mut self, key: KeyCode, modifiers: KeyModifiers) {
        if self.panes.is_empty() || !self.zoomed {
            return;
        }
        let pane = &mut self.panes[self.selected];
        let bytes: Vec<u8> = match key {
            KeyCode::Char(c) => {
                if modifiers.contains(KeyModifiers::CONTROL) {
                    vec![(c as u8) & 0x1f]
                } else {
                    c.to_string().into_bytes()
                }
            }
            KeyCode::Enter => vec![b'\r'],
            KeyCode::Backspace => vec![127],
            KeyCode::Tab => vec![b'\t'],
            KeyCode::Up => vec![27, 91, 65],
            KeyCode::Down => vec![27, 91, 66],
            KeyCode::Right => vec![27, 91, 67],
            KeyCode::Left => vec![27, 91, 68],
            KeyCode::Esc => vec![27],
            KeyCode::PageUp => vec![27, 91, 53, 126],
            KeyCode::PageDown => vec![27, 91, 54, 126],
            KeyCode::Home => vec![27, 91, 72],
            KeyCode::End => vec![27, 91, 70],
            KeyCode::Delete => vec![27, 91, 51, 126],
            _ => return,
        };
        let _ = pane.write_bytes(&bytes);
    }
}

fn main() -> Result<()> {
    Cli::parse();

    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(panic_info);
    }));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut app = App::new();
    set_mouse_capture(&mut terminal, app.mouse_capture_enabled)?;
    let result = run(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

fn set_mouse_capture<B: Backend + io::Write>(
    terminal: &mut Terminal<B>,
    enabled: bool,
) -> Result<()> {
    if enabled {
        execute!(terminal.backend_mut(), EnableMouseCapture)?;
    } else {
        execute!(terminal.backend_mut(), DisableMouseCapture)?;
    }
    Ok(())
}

fn run<B: Backend + io::Write>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    let ts = terminal.size()?;
    let mut last_size = Rect::new(0, 0, ts.width, ts.height);

    loop {
        app.read_pty_output();
        terminal.draw(|f| ui::draw(f, app))?;

        if event::poll(Duration::from_millis(UI_POLL_MS))? {
            match event::read()? {
                Event::Resize(w, h) => {
                    let size = Rect::new(0, 0, w, h);
                    last_size = size;
                    if app.zoomed {
                        if let Some(pane) = app.panes.get_mut(app.selected) {
                            pane.resize(h.saturating_sub(2), w.saturating_sub(2));
                        }
                    } else {
                        app.resize_all_to_grid(h, w);
                    }
                }
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    let size = last_size;
                    if key.code == KeyCode::F(2) {
                        app.mouse_capture_enabled = !app.mouse_capture_enabled;
                        set_mouse_capture(terminal, app.mouse_capture_enabled)?;
                        continue;
                    }
                    if app.zoomed {
                        if key.code == KeyCode::Char(' ')
                            && key.modifiers.contains(KeyModifiers::CONTROL)
                        {
                            app.zoomed = false;
                            app.scroll_offset = 0;
                            app.resize_all_to_grid(size.height, size.width);
                        } else {
                            app.send_key(key.code, key.modifiers);
                        }
                    } else {
                        match key.code {
                            KeyCode::Char('q') => return Ok(()),
                            KeyCode::Char('n') if app.panes.len() < 9 => {
                                let _ = app.spawn_shell(24, 80);
                                app.resize_all_to_grid(size.height, size.width);
                            }
                            KeyCode::Char('x') => {
                                app.close_selected_pane();
                            }
                            KeyCode::Char('r') => {
                                app.restart_selected_pane(size.height, size.width);
                            }
                            KeyCode::Char(c) if ('1'..='9').contains(&c) => {
                                let idx = (c as u8 - b'1') as usize;
                                if idx < app.panes.len() {
                                    app.selected = idx;
                                    app.zoomed = true;
                                    app.scroll_offset = 0;
                                    if let Some(pane) = app.panes.get_mut(app.selected) {
                                        pane.resize(
                                            size.height.saturating_sub(2),
                                            size.width.saturating_sub(2),
                                        );
                                    }
                                }
                            }
                            KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right => {
                                app.navigate(key.code);
                            }
                            KeyCode::Enter => {
                                if !app.panes.is_empty() {
                                    app.zoomed = true;
                                    app.scroll_offset = 0;
                                    if let Some(pane) = app.panes.get_mut(app.selected) {
                                        pane.resize(
                                            size.height.saturating_sub(2),
                                            size.width.saturating_sub(2),
                                        );
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Event::Mouse(mouse) if app.zoomed && app.mouse_capture_enabled => {
                    app.handle_mouse(mouse, last_size);
                }
                _ => {}
            }
        }
    }
}

impl App {
    fn handle_mouse(&mut self, mouse: MouseEvent, term_area: Rect) {
        if let Some(pane) = self.panes.get_mut(self.selected) {
            let mode = pane.mouse_protocol_mode();
            if mode == vt100::MouseProtocolMode::None {
                return;
            }

            if term_area.width <= 2 || term_area.height <= 2 {
                return;
            }

            // Zoomed mode uses a full-frame block with 1-cell border.
            if mouse.column == 0
                || mouse.row == 0
                || mouse.column >= term_area.width.saturating_sub(1)
                || mouse.row >= term_area.height.saturating_sub(1)
            {
                return;
            }

            let x = mouse.column;
            let y = mouse.row;
            let modifier_bits = mouse_modifier_bits(mouse.modifiers);

            let event = match mouse.kind {
                MouseEventKind::Down(button) => {
                    let button_code = match button {
                        crossterm::event::MouseButton::Left => 0,
                        crossterm::event::MouseButton::Middle => 1,
                        crossterm::event::MouseButton::Right => 2,
                    };
                    Some((button_code | modifier_bits, false))
                }
                MouseEventKind::Up(_) => Some((3 | modifier_bits, true)),
                MouseEventKind::Drag(button) => {
                    if matches!(
                        mode,
                        vt100::MouseProtocolMode::ButtonMotion
                            | vt100::MouseProtocolMode::AnyMotion
                    ) {
                        let button_code = match button {
                            crossterm::event::MouseButton::Left => 0,
                            crossterm::event::MouseButton::Middle => 1,
                            crossterm::event::MouseButton::Right => 2,
                        };
                        Some((button_code | 32 | modifier_bits, false))
                    } else {
                        None
                    }
                }
                MouseEventKind::Moved => {
                    if mode == vt100::MouseProtocolMode::AnyMotion {
                        Some((35 | modifier_bits, false))
                    } else {
                        None
                    }
                }
                MouseEventKind::ScrollUp => Some((64 | modifier_bits, false)),
                MouseEventKind::ScrollDown => Some((65 | modifier_bits, false)),
                MouseEventKind::ScrollLeft => Some((66 | modifier_bits, false)),
                MouseEventKind::ScrollRight => Some((67 | modifier_bits, false)),
            };

            if let Some((cb, release)) = event {
                if let Some(bytes) =
                    encode_xterm_mouse(pane.mouse_protocol_encoding(), cb, x, y, release)
                {
                    let _ = pane.write_bytes(&bytes);
                }
            }
        }
    }
}
