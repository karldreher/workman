import { invoke } from "@tauri-apps/api/core";
import { Config, RepoSuggestion, PushResult, Settings, WorktreePushResult } from "./types";

/** Loads the full application config from disk. */
export const loadConfig = () =>
  invoke<Config>("load_config");

/** Persists updated {@link Settings} and returns the new {@link Config}. */
export const updateSettings = (settings: Settings) =>
  invoke<Config>("update_settings", { settings });

/**
 * Returns path suggestions for the repo picker.
 * When `query` is empty, suggestions start at the user's home directory.
 * Known (already-registered) repos are listed before filesystem entries.
 */
export const getRepoSuggestions = (query: string) =>
  invoke<RepoSuggestion[]>("get_repo_suggestions", { query });

/** Validates that `path` is a directory containing a `.git` folder. Rejects with an error message if not. */
export const validateRepoPath = (path: string) =>
  invoke<void>("validate_repo_path", { path });

/** Converts a freeform project name into a valid git branch name (e.g. `"my feature"` → `"feat/my-feature"`). */
export const branchFromName = (name: string) =>
  invoke<string>("branch_from_name", { name });

/** Creates a new project (and its folder at `~/.workman/projects/<name>/`) and returns the updated config. */
export const createProject = (name: string) =>
  invoke<Config>("create_project", { name });

/** Removes a project and all its worktree checkouts from disk. Returns the updated config. */
export const removeProject = (projectName: string) =>
  invoke<Config>("remove_project", { projectName });

/**
 * Adds a repo (by path) to an existing project: creates a worktree at
 * `~/.workman/projects/<project>/<repo>/` and returns the updated config.
 */
export const addRepoToProject = (projectName: string, repoPath: string) =>
  invoke<Config>("add_repo_to_project", { projectName, repoPath });

/** Removes a single worktree from a project (runs `git worktree remove --force`). Returns the updated config. */
export const removeWorktree = (projectName: string, repoName: string) =>
  invoke<Config>("remove_worktree", { projectName, repoName });

/** Returns a map of `"<project>/<repo>"` → status string for all known worktrees. */
export const getAllStatuses = () =>
  invoke<Record<string, string>>("get_all_statuses");

/** Returns the raw `git diff` output for a specific worktree. */
export const getDiff = (projectName: string, repoName: string) =>
  invoke<string>("get_diff", { projectName, repoName });

/** Stages all changes, commits with `commitMessage` (or the default), and pushes a single worktree. */
export const pushWorktree = (projectName: string, repoName: string, commitMessage?: string) =>
  invoke<PushResult>("push_worktree", { projectName, repoName, commitMessage });

/** Stages, commits, and pushes every worktree in a project. Returns per-repo results. */
export const pushProject = (projectName: string, commitMessage?: string) =>
  invoke<WorktreePushResult[]>("push_project", { projectName, commitMessage });

/** Opens a PTY session for the given `sessionId` at `workingDir` with the given terminal dimensions. */
export const openPtySession = (sessionId: string, workingDir: string, cols: number, rows: number) =>
  invoke<void>("open_pty_session", { sessionId, workingDir, cols, rows });

/** Closes an active PTY session and frees its resources. */
export const closePtySession = (sessionId: string) =>
  invoke<void>("close_pty_session", { sessionId });

/** Sends raw bytes to a PTY session's stdin. */
export const writeToPty = (sessionId: string, data: number[]) =>
  invoke<void>("write_to_pty", { sessionId, data });

/** Notifies the PTY of a terminal resize (triggers `SIGWINCH` on Unix). */
export const resizePty = (sessionId: string, cols: number, rows: number) =>
  invoke<void>("resize_pty", { sessionId, cols, rows });

/** Opens the system terminal application at `path` (used when `use_tmux` is enabled). */
export const openExternalTerminal = (path: string) =>
  invoke<void>("open_external_terminal", { path });

/**
 * Generates (or updates) a `.code-workspace` file for the project and opens it with the `code` CLI.
 * Workspace folders are the real worktree paths — no symlinks.
 */
export const openInVscode = (projectName: string) =>
  invoke<void>("open_in_vscode", { projectName });
