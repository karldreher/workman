# Source Structure

- `main.rs`: Entry point, event loop, and terminal management.
- `app.rs`: Application state (`App` struct), selection logic, and input mode definitions.
- `models.rs`: Data models for `Project`, `Worktree`, and `Config`, including persistence and git status logic.
- `session.rs`: Encapsulates pseudo-terminal (PTY) functionality and manages shell processes.
- `event_handler.rs`: Handles keyboard input and dispatches events to update application state or forward to the terminal session.
- `terminal_handler.rs`: Manages pseudo-terminal (PTY) input/output and rendering for active terminal sessions.
- `ui.rs`: TUI rendering logic using `ratatui`.
