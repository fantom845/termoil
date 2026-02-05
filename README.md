<h1 align="center">termoil</h1>

<p align="center">
<code>brew install fantom845/tap/termoil</code> &middot; <code>cargo install termoil</code>
</p>
<img width="1907" height="957" alt="image" src="https://github.com/user-attachments/assets/645957a3-8c9e-49a8-92ea-aab39945c4e5" />
<h3 align="center">the lubricant for multi-agent workflows.<br/>end the turmoil.</h3>

<p align="center">
terminal dashboard for running multiple AI coding agents in parallel.<br/>
monitors your shells and alerts you the moment an agent needs input.<br/>
no more silent hangs. no more tab-hopping.
</p>

<p align="center">
works with <b>Claude Code</b> &middot; <b>Codex</b> ;  or any TUI-based tool
</p>


<p align="center">
  <a href="https://github.com/fantom845/termoil/releases">Releases</a> &middot;
  <a href="#install">Install</a> &middot;
  <a href="#keybindings">Keybindings</a>
</p>

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
