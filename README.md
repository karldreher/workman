# workman

A minimalist Git worktree manager for staying focused in large repositories.

`workman` provides a TUI to manage multiple Git worktrees effortlessly, isolating them within a `.workman/` directory in your projects to keep your root directory clean.

## Overview

`workman` is designed for developers who juggle multiple branches and tasks simultaneously. Instead of switching branches in your main directory, `workman` encourages a "one worktree per task" workflow.

### Key Features

*   **Isolated Worktrees**: All worktrees are created under `.workman/` and automatically added to `.gitignore`.
*   **Automatic Branching**: Automatically detects if a branch exists or creates a new one with `-b`.
*   **TUI Navigation**: Fast, keyboard-driven interface with a single tree-like view for projects and worktrees.
*   **Git Status at a Glance**: Detailed status including diff stats (e.g., `10/-12`), untracked files (`U:1`), and unpushed commits (`↑3`). Color-coded (Green for "clean", Red for dirty).
*   **Persistent Config**: Remembers your projects and worktrees across sessions (`~/.workman.config`).
*   **Quick Commands**: Run one-off shell commands directly from the TUI.

## Installation

### Manual

```bash
# Clone the repository
git clone https://github.com/your-username/workman
cd workman

# Build and install
cargo build --release
cp target/release/workman /usr/local/bin/
```

### macOS (GitHub Releases)

If you download the binary directly from GitHub, macOS will block it with a "Developer cannot be verified" message. To fix this, run the following command to remove the "quarantine" flag after extracting:

```bash
xattr -d com.apple.quarantine workman
```

Alternatively, you can right-click the binary in Finder, select **Open**, and then click **Open Anyway** in the dialog box.

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
| `r` | Remove worktree (output in right panel) |
| `c` | Run a single command in the selected worktree (output in right panel) |
| `p` | Auto-commit and Push (adds all, commits, pushes) |
| `d` | Show diff for selected worktree (Space to scroll, Esc to exit) |
| `Esc` | Cancel input / Clear output / Exit diff |
| `Ctrl+C` | Global quit |
| `Ctrl+L` | Export error logs to `/tmp/workman.log` |

## Technical Details

Built with **Rust**, **ratatui**, and **crossterm**. It relies on native `git` and `signal-hook` for safe terminal restoration and signal handling.

---
Built for efficiency.
