# workman

A minimalist Git worktree manager for staying focused in large repositories.

`workman` provides a TUI to manage multiple Git worktrees effortlessly, isolating them within a `.workman/` directory in your projects to keep your root directory clean. It now includes **persistent, interactive terminal sessions** for each worktree, akin to `tmux`.

## Overview

`workman` is designed for developers who juggle multiple branches and tasks simultaneously. Instead of switching branches in your main directory, `workman` encourages a "one worktree per task" workflow.

### Key Features

*   **Isolated Worktrees**: All worktrees are created under `.workman/` and automatically added to `.gitignore`.
*   **Automatic Branching**: Automatically detects if a branch exists or creates a new one with `-b`.
*   **TUI Navigation**: Fast, keyboard-driven interface with a single tree-like view for projects and worktrees.
*   **Git Status at a Glance**: Detailed status including diff stats (e.g., `10/-12`), untracked files (`U:1`), and unpushed commits (`↑3`). Color-coded (Green for "clean", Red for dirty).
*   **Persistent Sessions**: Offers `tmux`-style attach/detach for worktree-specific terminal sessions, preserving command output and history.
*   **Integrated Terminal**: Attach to a live terminal session for any worktree, enabling continuous interaction and persistent output.

## Installation

```bash
# Clone the repository
git clone https://github.com/your-username/workman
cd workman

# Build and install
cargo build --release
cp target/release/workman /usr/local/bin/
```

## Usage

Run `workman` from any terminal.

### Keybindings

| Key | Action |
| :--- | :--- |
| `q` | Quit |
| `Arrows` | Navigate the combined projects/worktrees list |
| `a` | Add a new project (supports path autocomplete with Tab) |
| `x` | Delete project from workman |
| `w` | Add new worktree (uses name for branch) |
| `r` | Remove worktree |
| `c` | Attach to an existing or create a new persistent terminal session for the selected worktree. |
| `p` | Auto-commit and Push (adds all, commits, pushes) |
| `d` | Show diff for selected worktree (Space to scroll, Esc to exit) |
| `Esc` | Cancel input / Clear output / Exit diff. In Terminal mode, detaches from the current session. |
| `Ctrl+C` | Global quit |
| `Ctrl+L` | Export error logs to `/tmp/workman.log` |

## Technical Details

Built with **Rust**, **tokio** (for async runtime), **ratatui**, **crossterm**, **portable-pty** (for pseudo-terminal management), and **vt100** (for terminal emulation). It relies on native `git` and `signal-hook` for safe terminal restoration and signal handling.

---
Built for efficiency.
