import { invoke } from "@tauri-apps/api/core";
import { Config, RepoSuggestion, PushResult, Settings, WorktreePushResult } from "./types";

export const loadConfig = () =>
  invoke<Config>("load_config");

export const updateSettings = (settings: Settings) =>
  invoke<Config>("update_settings", { settings });

export const getRepoSuggestions = (query: string) =>
  invoke<RepoSuggestion[]>("get_repo_suggestions", { query });

export const validateRepoPath = (path: string) =>
  invoke<void>("validate_repo_path", { path });

export const branchFromName = (name: string) =>
  invoke<string>("branch_from_name", { name });

export const createProject = (name: string) =>
  invoke<Config>("create_project", { name });

export const removeProject = (projectName: string) =>
  invoke<Config>("remove_project", { projectName });

export const addRepoToProject = (projectName: string, repoPath: string) =>
  invoke<Config>("add_repo_to_project", { projectName, repoPath });

export const removeWorktree = (projectName: string, repoName: string) =>
  invoke<Config>("remove_worktree", { projectName, repoName });

export const getAllStatuses = () =>
  invoke<Record<string, string>>("get_all_statuses");

export const getDiff = (projectName: string, repoName: string) =>
  invoke<string>("get_diff", { projectName, repoName });

export const pushWorktree = (projectName: string, repoName: string, commitMessage?: string) =>
  invoke<PushResult>("push_worktree", { projectName, repoName, commitMessage });

export const pushProject = (projectName: string, commitMessage?: string) =>
  invoke<WorktreePushResult[]>("push_project", { projectName, commitMessage });

export const openPtySession = (sessionId: string, workingDir: string, cols: number, rows: number) =>
  invoke<void>("open_pty_session", { sessionId, workingDir, cols, rows });

export const closePtySession = (sessionId: string) =>
  invoke<void>("close_pty_session", { sessionId });

export const writeToPty = (sessionId: string, data: number[]) =>
  invoke<void>("write_to_pty", { sessionId, data });

export const resizePty = (sessionId: string, cols: number, rows: number) =>
  invoke<void>("resize_pty", { sessionId, cols, rows });

export const openExternalTerminal = (path: string) =>
  invoke<void>("open_external_terminal", { path });

export const openInVscode = (projectName: string) =>
  invoke<void>("open_in_vscode", { projectName });
