# workman

A Git worktree manager for staying focused in large repositories.

`workman` organizes worktrees from multiple repos into **Projects**. A Project groups worktrees — one per repo — all on the same branch, so you can work on a feature that spans several codebases without losing context.

## Concepts

| Term | Meaning |
| :--- | :--- |
| **Repo** | A git repository registered in the global pool. |
| **Project** | A named grouping of worktrees, all on the same branch (e.g. `feat/my-feature`). |
| **Worktree** | A checked-out branch for one repo inside a project, at `~/.workman/projects/<project>/<repo>/`. |

### Project folder

Each Project gets a folder at `~/.workman/projects/<name>/`. Worktrees are checked out **directly** inside it — no symlinks, works the same on Windows, Linux, and macOS:

```
~/.workman/projects/my-feature/
├── frontend/              ← git worktree on feat/my-feature
├── backend/               ← git worktree on feat/my-feature
├── infra/                 ← git worktree on feat/my-feature
└── my-feature.code-workspace
```

## Installation

Download the latest release for your platform from [GitHub Releases](https://github.com/karldreher/workman/releases).

**macOS**: Open the `.dmg`, drag the app to `/Applications`.

> If macOS shows a security warning, go to System Settings → Privacy & Security and click Open Anyway, or run:
> ```bash
> xattr -dr com.apple.quarantine /Applications/workman.app
> ```

**Linux**: Use the `.AppImage` (chmod +x, run directly) or install the `.deb` / `.rpm` package.

**Windows**: Run the NSIS `.exe` installer or the `.msi`.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for local development setup, testing instructions, and code style guidance.

## Keybindings

| Key | Context | Action |
| :--- | :--- | :--- |
| `n` | Anywhere | Create a new project |
| `a` | Project selected | Add a repo to the project |
| `t` | Project / worktree selected | Open terminal (or external terminal if "Use external terminal" is on) |
| `p` | Project / worktree selected | Push (prompts for commit message) |
| `d` | Worktree selected | Show diff |
| `x` | Project / worktree selected | Delete |
| `o` | Anywhere | Options |
| `h` | Anywhere | Help |
| `↑` / `↓` | Anywhere | Navigate |
| `Enter` | Project selected | Expand / collapse |
| `Esc` | Anywhere | Close panel / dismiss error |
| `q` | Anywhere | Quit |

### In the terminal pane

Click **✕ detach** in the header to return to the tree while keeping the shell session alive.

## Status indicators

| Indicator | Meaning |
| :--- | :--- |
| `clean` | No changes, no unpushed commits |
| `5/-3` | 5 insertions, 3 deletions (unstaged) |
| `U:2` | 2 untracked files |
| `↑1` | 1 unpushed commit |
| `N/A` | Worktree path no longer exists |

## Options

| Setting | Default | Description |
| :--- | :--- | :--- |
| Use external terminal | off | When on, `t` opens the system terminal app at the worktree/project path instead of the built-in xterm pane. |

## Configuration

Config is stored at `~/.workman.config` (JSON). Manual editing is not required.

**Migrating from an older version**: if `workman` detects the legacy format it automatically migrates your repos and displays a notice. Create your first project with `n`.

## Technical details

Built with **Rust** + **Tauri v2** (backend) and **React + TypeScript + Vite** (frontend). Terminal emulation via **xterm.js**; PTY management via **portable-pty**. Config uses **serde_json**.
