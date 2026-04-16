# workman

A minimalist Git worktree manager for staying focused in large repositories.

`workman` provides a TUI to manage multiple Git worktrees across multiple repos, organized into **Projects**. A Project groups worktrees from different repos — all on the same branch — so you can work on a feature that spans several codebases without losing context.

## Concepts

| Term | Meaning |
| :--- | :--- |
| **Repo** | A git repository registered in the global pool. Equivalent to what was previously called a "project." |
| **Project** | A named grouping of worktrees, one per selected repo, all on the same branch (e.g., `feat/my-feature`). Represents a unit of work across multiple repos. |
| **Worktree** | A checked-out branch inside a repo, isolated under `<repo>/.workman/<branch>/`. Belongs to one Project. |

### Workflow

1. Register your repos globally (`a`).
2. Create a Project (`n`): give it a name, a branch, and select which repos to include. `workman` creates a worktree in each repo on that branch.
3. Open a terminal (`c`) in any worktree or at the project root.
4. Push changes (`p`) for a single worktree or all worktrees in a project at once.

### Project Folder

Each Project gets a folder at `~/.workman/projects/<name>/` containing symlinks to each of its worktrees. Opening a terminal at the Project level lands you here, giving you a single place to navigate across all repos involved in the project.

```
~/.workman/projects/my-feature/
├── frontend -> /path/to/frontend/.workman/feat-my-feature/
├── backend  -> /path/to/backend/.workman/feat-my-feature/
└── infra    -> /path/to/infra/.workman/feat-my-feature/
```

## Installation

### Manual

```bash
git clone https://github.com/your-username/workman
cd workman
cargo build --release
cp target/release/workman /usr/local/bin/
```

### macOS (GitHub Releases)

If you download the binary from GitHub Releases, macOS will quarantine it. Run:

```bash
xattr -d com.apple.quarantine workman
```

Or right-click → Open → Open Anyway in Finder.

## Usage

Run `workman` from any terminal.

### UI Layout

```
┌─ Projects & Repos ───┐  ┌─ Help ────────────────────────────────────────┐
│ ▼ my-feature         │  │ [Enter] expand/collapse  [w] add worktrees ... │
│   ├── [frontend]     │  └───────────────────────────────────────────────-┘
│   │   feat/my-feat.. │  ┌─ Output ───────────────────────────────────────┐
│   └── [backend]      │  │                                                 │
│       feat/my-feat.. │  │  Push successful!                               │
│ ▶ another-project    │  │  ✓ [frontend]  pushed                           │
│ ── Repos ──          │  │  ✓ [backend]   pushed                           │
│   frontend (/repos/…)│  │                                                 │
│   backend  (/repos/…)│  │                                                 │
└──────────────────────┘  └─────────────────────────────────────────────────┘
```

- **Left panel**: Projects (expandable) with their worktrees, then the global Repo list.
- **Right panel**: Context-sensitive help bar + output/terminal pane.
- Worktree status is color-coded: **green** = clean, **red** = dirty.

### Keybindings

#### Global

| Key | Action |
| :--- | :--- |
| `q` / `Ctrl+C` | Quit |
| `↑` / `↓` | Navigate |
| `Ctrl+L` | Export error log to `/tmp/workman.log` |

#### Normal mode

| Key | Context | Action |
| :--- | :--- | :--- |
| `n` | Anywhere | Create a new Project (3-step wizard: name → branch → select repos) |
| `a` | Anywhere | Add a Repo to the global pool (Tab for path autocomplete) |
| `x` | Repo selected | Remove repo from global pool |
| `Enter` | Project selected | Expand / collapse project worktrees |
| `w` | Project selected | Add more worktrees to the project (select from remaining repos) |
| `r` | Project selected | Delete project (removes all worktrees + project folder) |
| `r` | Worktree selected | Remove that worktree |
| `c` | Project selected | Open terminal at project folder |
| `c` | Worktree selected | Open terminal in that worktree |
| `p` | Project selected | Push all worktrees (prompts for commit message) |
| `p` | Worktree selected | Push that worktree |
| `d` | Worktree selected | Show diff (Space to scroll, Esc to exit) |
| `o` | Anywhere | Open Options |
| `Esc` | Anywhere | Cancel / clear output |

#### Terminal mode (in-app PTY)

| Key | Action |
| :--- | :--- |
| `Esc` | Detach from session (session stays alive) |
| `Ctrl+C` | Send interrupt to shell |

#### Tmux mode (when Use Tmux is enabled)

| Key | Action |
| :--- | :--- |
| `Ctrl-B D` | Detach from tmux session (workman resumes) |

### Options (`o`)

| Setting | Default | Description |
| :--- | :--- | :--- |
| Use Tmux | Off | When enabled, `c` opens a named `tmux` session instead of the built-in PTY. Session names follow the pattern `workman-<project>-<repo>`. `tmux` must be installed and on `$PATH`. |

## Status Indicators

Each worktree row shows a git status summary:

| Indicator | Meaning |
| :--- | :--- |
| `clean` | No uncommitted changes, no unpushed commits |
| `5/-3` | 5 insertions, 3 deletions (unstaged) |
| `U:2` | 2 untracked files |
| `↑1` | 1 unpushed commit |
| `N/A` | Worktree path no longer exists |

## Configuration

`workman` stores its config at `~/.workman.config` (JSON). You should not need to edit this manually.

**Upgrading from an older version**: If `workman` detects the legacy format (repos stored as "projects"), it automatically migrates them to the new `repos` list and displays a notice. Your data is preserved — just create your first Project with `n`.

## Technical Details

Built with **Rust**, **tokio** (async runtime), **ratatui**, **crossterm**, **portable-pty** (PTY management), and **vt100** (terminal emulation). Relies on native `git` and `signal-hook` for safe terminal restoration.

When **Use Tmux** is enabled, `portable-pty` is bypassed entirely. `workman` restores the terminal, hands off to `tmux new-session -A -s <name> -c <path>`, then re-enters raw mode when you detach.

---
Built for efficiency.
