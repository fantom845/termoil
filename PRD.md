# termoil (v1)
## Less friction for your multi-agent workflow

## 1. Executive Summary

termoil is a terminal-based dashboard that solves the "Silent Hang" problem. When running multiple AI coding agents (Claude Code, Aider, Cline, Roo, Codex, Cursor) in parallel, agents frequently halt waiting for user permission. termoil monitors those sessions and alerts you the moment an agent needs attention.

## 2. Problem Statement

Managing parallel agentic workflows is a "Whack-a-Mole" game:

- **Visibility Loss**: Can't see 10 terminal outputs at once
- **Process Stalling**: Agents sit idle for hours because a `[Y/n]` prompt went unnoticed
- **Context Switching**: Manually cycling through terminal tabs is exhausting

## 3. Technical Decisions

| Component | Decision | Why |
|-----------|----------|-----|
| Language | Rust | Single binary, fast, K9s-like aesthetic |
| TUI | ratatui | Battle-tested, supports animations |
| Terminal | Direct PTY (nix + vt100) | Full terminal emulation, no external deps |
| Detection | Regex + cursor awareness | Instant, zero-cost, avoids false positives |
| Alerting | Blinking border | Simple, visible, no external deps |

## 4. Core Features (v1)

### 4.1. Dashboard

- Adaptive grid layout: 1 pane full, 2 side-by-side, 3-4 as 2x2, 5-6 as 2x3, 7-9 as 3x3
- Cell-accurate rendering with full color/attribute support (bold, italic, underline, inverse)
- Dull purple theme with ASCII art logo
- Max 9 panes
- Terminal resize events propagate to all panes

### 4.2. Watchdog

- Scans lines near cursor position each tick (skipped while zoomed for performance)
- Suppresses alerts when cursor is on a shell prompt (avoids false positives from old output)
- Default trigger patterns: `[y/n]`, `[Y/n]`, `confirm`, `password:`, `(y/n)?`, `allow?`, `proceed?`, `continue?`, `do you want to`, `are you sure`, `Esc to cancel`

### 4.3. Visual Alerting

- **Normal**: Subtle purple border
- **Selected**: Bright purple border
- **Attention needed**: Blinking red border with `[!]` in title
- **Selected + attention**: Alternates red/purple

### 4.4. Navigation & Interaction

- **Arrow keys**: 2D grid navigation between panes (wraps to last pane on short rows)
- **Enter**: Zoom into selected pane (full screen)
- **Ctrl+Space**: Exit zoom back to grid
- **n**: Spawn a new shell pane
- **q**: Quit
- **F2**: Toggle mouse capture (mouse:on / mouse:off shown in zoom title)
- **Keystroke passthrough**: When zoomed, all input goes to the child process
- **Mouse passthrough**: Full xterm mouse protocol forwarding (click, drag, scroll) respecting child app's mouse mode/encoding
- **DSR support**: Responds to cursor position queries with real cursor position; handles split sequences across reads

### 4.5. Terminal Emulation

- vt100-based screen buffer with 1000 lines scrollback
- Cursor visibility follows child app (hidden when app requests it)
- PTY sizes match actual rendered cell area per pane (including uneven last rows)
- Zombie child processes reaped automatically

## 5. Architecture

```
┌───────────────────────────────────────────┐
│              termoil TUI (ratatui)         │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐  │
│  │ PTY+vt100│ │ PTY+vt100│ │ PTY+vt100│  │
│  │  shell 1 │ │  shell 2 │ │  shell 3 │  │
│  └──────────┘ └──────────┘ └──────────┘  │
│       ▲              ▲             ▲       │
│   fork+exec      fork+exec    fork+exec   │
└───────────────────────────────────────────┘

Rendering: vt100 cells -> ratatui buffer (cell-by-cell with color/attrs)
Input:     crossterm events -> xterm escape sequences -> PTY master fd
Mouse:     crossterm mouse -> xterm mouse protocol -> PTY master fd
DSR:       child sends ESC[6n -> detected in output -> real cursor pos response
```

- termoil spawns shells via PTY (fork + exec)
- Reads output via non-blocking read on master fd in background thread
- Parses terminal output through vt100 emulator
- Renders cell-by-cell to ratatui buffer preserving colors and attributes
- Sends input/mouse via write to master fd
- PTY resizes on zoom/unzoom/terminal resize to match display area
- Mouse capture toggleable at runtime for copy/paste usability

## 6. User Flow

1. User runs `termoil`
2. Presses `n` to spawn shells, runs agents in each
3. Agent in pane 2 asks for file-write permission
4. Pane 2 border blinks red
5. User arrows to pane 2, presses Enter to zoom
6. User interacts with the agent (full mouse + keyboard support)
7. User presses Ctrl+Space to return to grid
8. Border returns to normal

## 7. Scope Cuts (v1)

- No config file (sensible defaults only)
- No OS-level push notifications
- No custom theming (ships with purple theme)
- No CLI args for pre-spawning commands
- No closing individual panes
- No pane renaming
- No session persistence

## 8. Dependencies

- **Runtime**: None (single binary)
- **Build**: Rust toolchain
- **Crates**: ratatui, crossterm, nix, libc, vt100, regex, anyhow

## 9. Roadmap (v2+)

### 9.1. Pane Lifecycle
- Close/restart individual panes
- Rename panes
- Command-at-spawn (`n` with preset or CLI arg, e.g. `termoil "claude" "aider"`)

### 9.2. Operator Navigation
- Jump to next pane needing attention (hotkey)
- Attention queue / priority ordering
- Unread/attention count in status bar

### 9.3. Config & Persistence
- `config.toml` for keybindings, trigger patterns, theme
- Optional session restore (reopen last pane layout)

### 9.4. Notification Hooks
- Optional bell / desktop notification on attention events
- Webhook support for external integrations

### 9.5. Observability
- Per-pane status indicator (running / idle / waiting)
- Last activity timestamp
- Error banner for spawn/write failures

## 10. Success Criteria

- Every "waiting" state detected within 3 seconds
- User stays in termoil for entire multi-agent session
- Zero lost work from unnoticed prompts
- TUI agents (Claude Code, Codex) work correctly inside panes
