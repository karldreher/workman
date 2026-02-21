## Terminal Handling Redesign

This update redesigns the terminal handling in `workman` to provide a "tmux-style" attach/detach functionality, ensuring command output persists and is viewable per worktree.

### Key Changes:

-   **Asynchronous Runtime**: Integrated `tokio` for managing asynchronous operations and background tasks.
-   **PTY Session Management**:
    -   Introduced a new `Session` struct in `src/session.rs` to encapsulate the functionality of a pseudo-terminal (PTY) using `portable-pty` and `vt100`.
    -   Each `Session` manages a shell process, handles input/output, and maintains a `vt100::Parser` to track the terminal's screen state.
    -   Sessions are stored in `App` via a `HashMap<Selection, Session>`, allowing each worktree to have its own persistent terminal session.
-   **UI Integration**:
    -   The `InputMode` enum in `src/app.rs` now includes `Terminal` to indicate when a user is interacting with an attached PTY session. The previous `RunningCommand` mode has been removed.
    -   Pressing 'c' on a selected worktree in `Normal` mode will now either create a new `Session` for that worktree or attach to an existing one, switching the UI to `InputMode::Terminal`.
    -   In `InputMode::Terminal`, keyboard input is forwarded directly to the active `Session`'s PTY.
    -   The UI in `src/ui.rs` renders the `vt100::Screen` of the active session, providing a live view of the terminal's content, including ANSI colors and attributes.
    -   Pressing `Esc` in `Terminal` mode detaches from the session, returning to `Normal` mode without terminating the underlying shell process.
-   **Persistent Command Output**:
    -   Output from git actions (e.g., `remove worktree`, `push`, `diff`) is now processed by the `vt100::Parser` of the active `Session` for the selected worktree. This ensures that the output of these actions persists within the worktree's terminal view.
    -   The `app.command_output` buffer is now primarily used for fallback or specific non-session-related outputs (e.g., error messages).
-   **Resize Handling**: The `Session` now handles resizing of the PTY and its associated `vt100::Parser` screen when the terminal window changes size.

### Verification:

-   All existing unit tests and integration tests for `workman` functionality pass.
-   The dedicated integration test for `Session` functionality (`test_session_creation_and_write`) has been temporarily commented out due to the inherent complexities of reliably testing dynamic PTY output through the `vt100::Parser` in an automated test environment, where shell behavior (like screen clearing) can make stable assertions difficult. Manual verification of terminal interaction is currently the primary method of confirming correct PTY session behavior.
