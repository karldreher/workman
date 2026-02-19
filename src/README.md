# Source Structure

- `main.rs`: Entry point, event loop, and terminal management.
- `app.rs`: Application state (`App` struct), selection logic, and input mode definitions.
- `models.rs`: Data models for `Project`, `Worktree`, and `Config`, including persistence and git status logic.
- `ui.rs`: TUI rendering logic using `ratatui`.
