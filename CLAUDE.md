# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run

```bash
cargo build           # dev build
cargo run             # run termoil
cargo build --release # release binary at target/release/termoil
```

No test suite yet (`cargo test` runs 0 tests).

## Architecture

termoil is a terminal multiplexer/dashboard for monitoring multiple AI coding agents. It spawns shell subprocesses via PTY, emulates their terminal output, and alerts when agents need user input.

**Data flow:**
```
Input:     crossterm events → xterm escape sequences → PTY master fd
Output:    PTY read thread → mpsc channel → vt100 parser → ratatui frame buffer
Detection: vt100 screen → watchdog regex near cursor → attention flag → blinking border
```

### src/main.rs
Entry point. Owns `App` struct which holds all state: panes, selection, zoom, watchdog, attention flags, tick counter. Runs the 16ms event loop that drains PTY output, draws UI, and routes keyboard/mouse input. Handles terminal resize events by propagating new dimensions to all PTY panes.

### src/pty.rs
`Pane` struct wraps a forked child process with a PTY master fd. A background thread reads from the master fd and sends chunks via `mpsc::channel`. `read_available()` feeds chunks to the `vt100::Parser`, detects DSR cursor queries (`\x1b[6n`) across chunk boundaries using a 3-byte tail buffer, and responds with real cursor position. Also reaps zombie children via `waitpid(WNOHANG)`.

### src/ui.rs
Rendering layer. `draw()` dispatches to `draw_grid()` or `draw_zoomed()`. Grid layout is computed by `grid_dimensions()` (1→1x1, 2→1x2, 3-4→2x2, 5-6→2x3, 7-9→3x3). Pane content is rendered cell-by-cell from vt100 to ratatui buffer preserving colors/attributes via `render_pane_cells()`. Color constants: `PURPLE` (main), `CYAN` (accent/titles), `ALERT`/`ALERT_DIM` (blinking red), `DIM` (unselected), `BG` (background).

### src/watchdog.rs
`Watchdog` holds compiled regex patterns for attention triggers (`[y/n]`, `password:`, `do you want to`, etc.) and shell prompt patterns (`➜`, `$`, `%`, `user@`). `needs_attention()` checks lines near the cursor but suppresses if the cursor line itself is a shell prompt (avoids false positives from old output).

## Key Design Decisions

- **PTY via nix/libc, not portable-pty** - portable-pty conflicted with crossterm's raw mode. Direct `fork()`+`openpty()` gives full control.
- **vt100 crate for terminal emulation** - handles cursor positioning, colors, scrollback. Raw string appending produced garbled output.
- **Echo disabled on PTY slave** - prevents DSR responses from looping back through line discipline.
- **Watchdog skipped while zoomed** - performance optimization; user is already looking at the pane.
- **Cell-by-cell rendering** - copies vt100 cells directly to ratatui buffer instead of using `Paragraph` widget. Required for accurate color/attribute reproduction.

## Keybindings

Grid view: `n` spawn, `q` quit, arrows navigate, `Enter` zoom.
Zoomed: `Ctrl+Space` exit zoom, `F2` toggle mouse capture. All other keys pass through to child PTY.
