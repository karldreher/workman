/** A git repository registered in the global repo pool. */
export interface Repo {
  /** Human-readable name (derived from the folder name on registration). */
  name: string;
  /** Absolute path to the repo root on disk. */
  path: string;
}

/** A single git worktree checkout inside a {@link Project}. */
export interface ProjectWorktree {
  /** Name of the {@link Repo} this worktree belongs to. */
  repo_name: string;
  /** Absolute path to the worktree directory (`~/.workman/projects/<project>/<repo>/`). */
  path: string;
}

/**
 * A named grouping of worktrees, all on the same branch, across one or more repos.
 * Lives at `~/.workman/projects/<name>/`.
 */
export interface Project {
  /** Short display name (e.g. `"my-feature"`). */
  name: string;
  /** Git branch shared by all worktrees in this project (e.g. `"feat/my-feature"`). */
  branch: string;
  /** Worktree checkouts that belong to this project. */
  worktrees: ProjectWorktree[];
  /** Absolute path to the project folder (`~/.workman/projects/<name>/`). */
  folder: string;
}

/** Global application settings persisted to `~/.workman.config`. */
export interface Settings {
  /**
   * When `true`, the terminal action opens the system terminal app at the
   * worktree/project path instead of the built-in xterm pane.
   * When `false` (default), the built-in xterm pane is used, with tmux
   * session persistence if tmux is available on the system.
   */
  use_external_terminal: boolean;
}

/** Top-level configuration object stored at `~/.workman.config` (JSON). */
export interface Config {
  /** All registered repos available for use in projects. */
  repos: Repo[];
  /** All projects (each groups worktrees across one or more repos). */
  projects: Project[];
  /** Global settings. */
  settings: Settings;
}

/**
 * A filesystem path entry returned by {@link getRepoSuggestions}.
 * Known repos (already registered) are listed first.
 */
export interface RepoSuggestion {
  /** Absolute path to the directory. */
  path: string;
  /** `true` if this path is already a registered repo. */
  known: boolean;
  /** `true` if a `.git` directory exists at this path. */
  is_git_repo: boolean;
}

/** Aggregated result of a single-worktree push (add + commit + push). */
export interface PushResult {
  /** Whether all three git commands succeeded. */
  success: boolean;
  /** Combined stdout/stderr from all three commands. */
  output: string;
}

/** Per-repo result when pushing an entire project. */
export interface WorktreePushResult {
  /** Name of the repo this result belongs to. */
  repo_name: string;
  /** Whether the push succeeded for this repo. */
  success: boolean;
  /** Human-readable summary or error detail. */
  detail: string;
}
