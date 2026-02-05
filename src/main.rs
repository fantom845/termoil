mod pty;
mod ui;
mod watchdog;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use pty::Pane;
use ratatui::prelude::*;
use std::io;
use std::time::Duration;

pub struct App {
    pub panes: Vec<Pane>,
    pub selected: usize,
    pub zoomed: bool,
    pub watchdog: watchdog::Watchdog,
    pub attention: Vec<bool>,
    pub tick: u64,
}

impl App {
    fn new() -> Self {
        Self {
            panes: Vec::new(),
            selected: 0,
            zoomed: false,
            watchdog: watchdog::Watchdog::new(),
            attention: Vec::new(),
            tick: 0,
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
        for (i, pane) in self.panes.iter().enumerate() {
            self.attention[i] = self.watchdog.needs_attention(
                &pane.cursor_line(),
                &pane.lines_near_cursor(),
            );
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
            _ => return,
        };
        let _ = pane.write_bytes(&bytes);
    }
}

fn main() -> Result<()> {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(panic_info);
    }));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let result = run(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    loop {
        app.read_pty_output();
        terminal.draw(|f| ui::draw(f, app))?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                if app.zoomed {
                    if key.code == KeyCode::Char(' ') && key.modifiers.contains(KeyModifiers::CONTROL) {
                        app.zoomed = false;
                    } else {
                        app.send_key(key.code, key.modifiers);
                    }
                } else {
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Char('n') => {
                            let size = terminal.size()?;
                            let _ = app.spawn_shell(size.height - 9, size.width - 2);
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if app.selected > 0 {
                                app.selected -= 1;
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if app.selected + 1 < app.panes.len() {
                                app.selected += 1;
                            }
                        }
                        KeyCode::Enter => {
                            if !app.panes.is_empty() {
                                app.zoomed = true;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
