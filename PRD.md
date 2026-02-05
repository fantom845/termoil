# termoil (v1)
## Less friction for your multi-agent workflow

## 1. Executive Summary

termoil is a terminal-based dashboard that solves the "Silent Hang" problem. When running multiple AI coding agents (Claude Code, Aider, Cline, Roo, Codex, Cursor) across different tmux sessions, agents frequently halt waiting for user permission. termoil monitors those sessions and alerts you the moment an agent needs attention.

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
| Backend | tmux | Sessions persist if termoil crashes |
| Detection | Regex | Instant, zero-cost, reliable |
| Alerting | Border color change | Simple, visible, no external deps |

## 4. Core Features (v1)

### 4.1. Dashboard

- Flexible grid layout based on number of attached sessions
- Real-time tailing of each tmux pane
- Dull purple theme with ASCII art logo

### 4.2. Watchdog

- Scans last N lines of each pane every ~2 seconds
- Default trigger patterns: `[y/n]`, `[Y/n]`, `confirm`, `password:`, `(y/n)?`, `allow?`, `proceed?`, `continue?`
- Extensible for agent-specific patterns

### 4.3. Visual Alerting

- **Normal**: Subtle border
- **Attention needed**: Pane border lights up (distinct color)
- Animated outline on currently selected pane

### 4.4. Navigation & Interaction

- **Arrow keys**: Move selection between panes
- **Double-enter**: Zoom into selected pane (full screen), especially useful when pane is alerting
- **Esc**: Return to grid view from zoomed state
- **Keystroke passthrough**: When zoomed, all input goes to the underlying tmux pane

## 5. Architecture

```
┌─────────────────────────────────────────┐
│              termoil TUI                │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐   │
│  │ pane 1  │ │ pane 2  │ │ pane 3  │   │
│  │ (tmux)  │ │ (tmux)  │ │ (tmux)  │   │
│  └─────────┘ └─────────┘ └─────────┘   │
│         ▲           ▲           ▲       │
│         │           │           │       │
│    capture-pane  send-keys  (tmux CLI)  │
└─────────────────────────────────────────┘
```

- termoil attaches to existing tmux sessions/panes
- Reads output via `tmux capture-pane`
- Sends input via `tmux send-keys`
- If termoil dies, tmux sessions continue running

## 6. User Flow

1. User has tmux sessions running with coding agents
2. User runs `termoil session1:0 session2:0 session3:0` (tmux target syntax)
3. Dashboard opens showing all panes in a grid
4. Agent in pane 2 asks for file-write permission
5. Pane 2 border lights up
6. User arrows to pane 2, double-enters to zoom
7. User types `y`, hits enter
8. User hits `Esc` to return to grid
9. Border returns to normal

## 7. Scope Cuts (v1)

- No config file (sensible defaults only)
- No OS-level push notifications
- No custom theming (ships with purple theme)
- No mouse support (keyboard only)
- No spawning sessions (attach-only)

## 8. Dependencies

- **Runtime**: tmux (must be installed)
- **Build**: Rust toolchain

## 9. Success Criteria

- Every "waiting" state detected within 3 seconds
- User stays in termoil for entire multi-agent session
- Zero lost work from unnoticed prompts
