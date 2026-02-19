# workman

A minimalist Git worktree manager for staying focused in large repositories.

`workman` provides a TUI to manage multiple Git worktrees effortlessly, isolating them within a `.workman/` directory in your projects to keep your root directory clean.

## Overview

`workman` is designed for developers who juggle multiple branches and tasks simultaneously. Instead of switching branches in your main directory, `workman` encourages a "one worktree per task" workflow.

### Key Features

*   **Isolated Worktrees**: All worktrees are created under `.workman/` and automatically added to `.gitignore`.
*   **Automatic Branching**: Automatically detects if a branch exists or creates a new one with `-b`.
*   **TUI Navigation**: Fast, keyboard-driven interface with a single tree-like view for projects and worktrees.
*   **Git Status at a Glance**: Detailed status including diff stats (e.g., `0/0`, `10/-12`), untracked files (`U:1`), and unpushed commits (`↑3`). Color-coded (Green for clean, Red for dirty).
*   **Persistent Config**: Remembers your projects and worktrees across sessions (`~/.workman.config`).
*   **Quick Commands**: Run one-off shell commands directly from the TUI.

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
| `w` | Add new worktree (automatically derives branch name) |
| `r` | Remove worktree (output in right panel) |
| `c` | Run a single command in the selected worktree (output in right panel) |
| `p` | Push the current branch of the selected worktree (output in right panel) |
| `d` | Show diff for selected worktree (Space to scroll, Esc to exit) |
| `Ctrl+C` | Global quit |
| `Ctrl+L` | Export error logs to `/tmp/workman.log` |

## Technical Details

Built with **Rust**, **ratatui**, and **crossterm**. It relies on native `git` and `signal-hook` for safe terminal restoration and signal handling.

---
Built for efficiency.
