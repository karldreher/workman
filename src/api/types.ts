export interface Repo {
  name: string;
  path: string;
}

export interface ProjectWorktree {
  repo_name: string;
  path: string;
}

export interface Project {
  name: string;
  branch: string;
  worktrees: ProjectWorktree[];
  folder: string;
}

export interface Settings {
  use_tmux: boolean;
}

export interface Config {
  repos: Repo[];
  projects: Project[];
  settings: Settings;
}

export interface RepoSuggestion {
  path: string;
  known: boolean;
}

export interface PushResult {
  success: boolean;
  output: string;
}

export interface WorktreePushResult {
  repo_name: string;
  success: boolean;
  detail: string;
}
