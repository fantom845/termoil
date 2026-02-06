<h1 align="center">termoil</h1>

<p align="center">
<code>brew install fantom845/tap/termoil</code> &middot; <code>cargo install termoil</code>
</p>

<https://github.com/user-attachments/assets/35a23dc0-cec3-49a9-9f87-920a13c4017f>

<h3 align="center">the lubricant for multi-agent workflows.<br/>end the turmoil.</h3>

<p align="center">
works with <b>Claude Code</b> &middot; <b>Codex</b> &middot; <b>Aider</b> &middot; or any TUI-based tool
</p>

<p align="center">
  <a href="https://github.com/fantom845/termoil/releases">Releases</a> &middot;
  <a href="#install">Install</a> &middot;
  <a href="#keybindings">Keybindings</a>
</p>

---

Rant time: You're running Claude Code in 6 different worktrees. Aider in two more. Maybe a Codex session for good measure. You're in the zone, mass-parallelizing your entire sprint. Life is good.

Then you check back 45 minutes later and realize half your agents have been sitting there, patiently waiting for you to type `y` on a file-write permission. For forty-five minutes. The work you thought was happening? Wasn't. The agent didn't crash. It didn't error. It just... waited. In a tab you forgot existed.

You scramble through your terminals. Cmd+Tab. Cmd+Tab. Cmd+Tab. Which tab was the auth service? Was Aider on tab 4 or 7? You find one stuck agent, approve it, switch to the next tab -- wrong one. Switch again. There it is. Approve. Switch. Where's the third one? Was it in iTerm or the VS Code terminal?

If you're tired of playing whack-a-mole with your AI agents, termoil is for you.

One dashboard. All your agents. It blinks red when one needs you. That's it.

## Install

**Homebrew (macOS)**
```bash
brew install fantom845/tap/termoil
```

**Cargo (any platform)**
```bash
cargo install termoil
```

**From source**
```bash
git clone https://github.com/fantom845/termoil
cd termoil
cargo build --release
# binary at target/release/termoil
```

## Usage

```bash
termoil
```

Press `n` to spawn shells. Run your agents inside them.

## Keybindings

### Grid view

| Key | Action |
|-----|--------|
| `n` | Spawn a new shell (max 9) |
| Arrow keys | Navigate between panes |
| `Enter` | Zoom into selected pane |
| `x` | Close selected pane |
| `r` | Restart selected pane |
| `q` | Quit |

### Zoomed view

| Key | Action |
|-----|--------|
| `Ctrl+Space` | Exit zoom, return to grid |
| `F2` | Toggle mouse capture on/off |
| Everything else | Passes through to the shell |

## How it works

When an agent asks for permission -- `[Y/n]`, `Allow?`, `Do you want to proceed?` -- the pane border blinks red. Navigate to it, zoom in, respond, zoom out.

termoil spawns real PTY shells with full terminal emulation (colors, cursor positioning, mouse support). TUI apps like Claude Code and Codex work correctly inside panes.

## Layout

Panes arrange automatically based on count:

```
1: full        2: side-by-side     3-4: 2x2
5-6: 2x3       7-9: 3x3
```

## Requirements

- macOS or Linux
- Rust toolchain (for building)

## License

MIT
