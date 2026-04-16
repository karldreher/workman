use crate::terminal_handler;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::{fs, path::PathBuf};

use crate::app::{App, InputMode, Selection};
use crate::models::{Config, Project, ProjectWorktree};
use crate::session::Session;

pub enum AppState {
    Continue,
    Quit,
    /// Suspend workman, run a tmux session, then resume.
    TmuxSession { path: PathBuf, session_name: String },
}

pub async fn handle_key_event(
    key: KeyEvent,
    app: &mut App,
    current_width: u16,
    current_height: u16,
) -> Result<AppState> {
    // Global Ctrl+C
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        if app.input_mode != InputMode::Terminal {
            return Ok(AppState::Quit);
        }
    }

    // Global Ctrl+L: export log
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('l') {
        if let Some(detail) = &app.full_error_detail {
            let _ = fs::write("/tmp/workman.log", detail);
            app.error_message = Some("Log exported to /tmp/workman.log".to_string());
        } else if let Some(err) = &app.error_message {
            let _ = fs::write("/tmp/workman.log", err);
            app.error_message = Some("Status exported to /tmp/workman.log".to_string());
        }
        return Ok(AppState::Continue);
    }

    match app.input_mode {
        // ── Normal mode ──────────────────────────────────────────────────
        InputMode::Normal => match key.code {
            KeyCode::Char('q') => return Ok(AppState::Quit),

            // New project (3-step wizard)
            KeyCode::Char('n') => {
                app.input_mode = InputMode::AddingProjectName;
                app.input.clear();
                app.pending_project_name.clear();
                app.pending_project_branch.clear();
                app.error_message = None;
                app.full_error_detail = None;
            }

            // Add repo to global pool
            KeyCode::Char('a') => {
                app.input_mode = InputMode::AddingRepoPath;
                app.input.clear();
                app.error_message = None;
                app.full_error_detail = None;
            }

            // Add worktrees to existing project (or expand if no repos to add)
            KeyCode::Char('w') => {
                if let Some(Selection::Project(p_idx)) = app.get_selected_selection() {
                    let available: Vec<_> = app.config.repos.iter().filter(|repo| {
                        !app.config.projects[p_idx].worktrees.iter().any(|wt| wt.repo_name == repo.name)
                    }).collect();

                    if available.is_empty() {
                        app.error_message = Some("All repos are already in this project.".to_string());
                    } else {
                        app.adding_to_project = Some(p_idx);
                        app.pending_project_name = app.config.projects[p_idx].name.clone();
                        app.pending_project_branch = app.config.projects[p_idx].branch.clone();
                        app.repo_selection = vec![false; app.config.repos.len()];
                        app.repo_cursor = 0;
                        app.input_mode = InputMode::SelectingRepos;
                        app.error_message = None;
                        app.full_error_detail = None;
                    }
                }
            }

            // Remove project (all worktrees + folder) or remove single worktree
            KeyCode::Char('r') => {
                match app.get_selected_selection() {
                    Some(Selection::Project(p_idx)) => {
                        handle_remove_project(app, p_idx);
                    }
                    Some(Selection::Worktree(p_idx, w_idx)) => {
                        handle_remove_worktree(app, p_idx, w_idx);
                    }
                    _ => {}
                }
            }

            // Open terminal (worktree or project level)
            KeyCode::Char('c') => {
                match app.get_selected_selection() {
                    Some(sel @ Selection::Worktree(p_idx, w_idx)) => {
                        let wt_path = app.config.projects[p_idx].worktrees[w_idx].path.clone();
                        let repo_name = app.config.projects[p_idx].worktrees[w_idx].repo_name.clone();
                        let project_name = app.config.projects[p_idx].name.clone();

                        if app.config.settings.use_tmux {
                            let session_name = sanitize_tmux_name(&format!(
                                "workman-{}-{}", project_name, repo_name
                            ));
                            return Ok(AppState::TmuxSession { path: wt_path, session_name });
                        }

                        if !app.sessions.contains_key(&sel) {
                            match Session::new(wt_path, current_width, current_height) {
                                Ok(session) => { app.sessions.insert(sel, session); }
                                Err(e) => {
                                    app.error_message = Some(format!("Failed to start session: {}", e));
                                    return Ok(AppState::Continue);
                                }
                            }
                        }
                        app.input_mode = InputMode::Terminal;
                    }
                    Some(sel @ Selection::Project(p_idx)) => {
                        let folder = app.config.projects[p_idx].folder.clone();
                        let project_name = app.config.projects[p_idx].name.clone();

                        if app.config.settings.use_tmux {
                            let session_name = sanitize_tmux_name(&format!("workman-{}", project_name));
                            return Ok(AppState::TmuxSession { path: folder, session_name });
                        }

                        if !app.sessions.contains_key(&sel) {
                            match Session::new(folder, current_width, current_height) {
                                Ok(session) => { app.sessions.insert(sel, session); }
                                Err(e) => {
                                    app.error_message = Some(format!("Failed to start session: {}", e));
                                    return Ok(AppState::Continue);
                                }
                            }
                        }
                        app.input_mode = InputMode::Terminal;
                    }
                    _ => {}
                }
            }

            // Push: single worktree or all worktrees in project
            KeyCode::Char('p') => {
                match app.get_selected_selection() {
                    Some(Selection::Worktree(_, _)) | Some(Selection::Project(_)) => {
                        app.input_mode = InputMode::EditingCommitMessage;
                        app.input.clear();
                        app.error_message = None;
                        app.full_error_detail = None;
                    }
                    _ => {}
                }
            }

            // Diff (worktree only)
            KeyCode::Char('d') => {
                if let Some(sel @ Selection::Worktree(p_idx, w_idx)) = app.get_selected_selection() {
                    match app.config.projects[p_idx].worktrees[w_idx].get_diff() {
                        Ok(out) => {
                            let mut full_output = Vec::new();
                            full_output.extend_from_slice(&out.stdout);
                            full_output.extend_from_slice(&out.stderr);

                            if let Some(session) = app.sessions.get(&sel) {
                                session.parser.lock().unwrap().process(&full_output);
                                app.input_mode = InputMode::ViewingDiff;
                            } else {
                                app.command_output = String::from_utf8_lossy(&full_output)
                                    .lines().map(String::from).collect();
                                if !out.status.success() {
                                    app.error_message = Some("Failed to get diff".to_string());
                                    app.full_error_detail = Some(app.command_output.join("\n"));
                                } else if app.command_output.is_empty() {
                                    app.error_message = Some("No changes to diff.".to_string());
                                } else {
                                    app.input_mode = InputMode::ViewingDiff;
                                    app.error_message = None;
                                    app.full_error_detail = None;
                                    app.diff_scroll_offset = 0;
                                }
                            }
                        }
                        Err(e) => {
                            app.error_message = Some("System error getting diff".to_string());
                            app.full_error_detail = Some(e.to_string());
                        }
                    }
                }
            }

            // Options overlay
            KeyCode::Char('o') => {
                app.input_mode = InputMode::Options;
                app.options_cursor = 0;
                app.error_message = None;
            }

            // Help view
            KeyCode::Char('h') => {
                app.input_mode = InputMode::Help;
                app.error_message = None;
            }

            // Expand/collapse project
            KeyCode::Enter => {
                if let Some(Selection::Project(p_idx)) = app.get_selected_selection() {
                    app.toggle_project_expand(p_idx);
                }
            }

            KeyCode::Down => app.next(),
            KeyCode::Up => app.previous(),
            KeyCode::Esc => {
                app.error_message = None;
                app.full_error_detail = None;
                app.command_output.clear();
            }
            _ => {}
        },

        // ── Terminal mode ─────────────────────────────────────────────────
        InputMode::Terminal => terminal_handler::handle_terminal_key_event(key, app),

        // ── Viewing diff ──────────────────────────────────────────────────
        InputMode::ViewingDiff => match key.code {
            KeyCode::Char(' ') => {
                if app.diff_scroll_offset + 1 < app.command_output.len() {
                    app.diff_scroll_offset += 1;
                } else {
                    app.diff_scroll_offset = 0;
                }
            }
            KeyCode::Esc => {
                app.input_mode = InputMode::Normal;
                app.error_message = None;
                app.full_error_detail = None;
                app.input.clear();
                app.command_output.clear();
                app.diff_scroll_offset = 0;
            }
            _ => {}
        },

        // ── Options overlay ───────────────────────────────────────────────
        InputMode::Options => match key.code {
            KeyCode::Esc => {
                app.input_mode = InputMode::Normal;
            }
            KeyCode::Up => {
                if app.options_cursor > 0 {
                    app.options_cursor -= 1;
                }
            }
            KeyCode::Down => {
                // Extend upper bound as more settings are added
                let max_idx = 0usize; // currently 1 option
                if app.options_cursor < max_idx {
                    app.options_cursor += 1;
                }
            }
            KeyCode::Char(' ') | KeyCode::Enter => {
                match app.options_cursor {
                    0 => {
                        app.config.settings.use_tmux = !app.config.settings.use_tmux;
                        app.save_config();
                    }
                    _ => {}
                }
            }
            _ => {}
        },

        // ── Help view ─────────────────────────────────────────────────────
        InputMode::Help => {
            app.input_mode = InputMode::Normal;
        }

        // ── Adding repo path ──────────────────────────────────────────────
        InputMode::AddingRepoPath => match key.code {
            KeyCode::Enter => {
                let path_str = app.input.trim().to_string();
                let path = PathBuf::from(&path_str);
                match Config::validate_repo_path(&path) {
                    Ok(_) => {
                        let abs_path = fs::canonicalize(&path).unwrap();
                        let name = abs_path.file_name().unwrap().to_string_lossy().to_string();
                        // Don't add duplicates
                        if app.config.repos.iter().any(|r| r.path == abs_path) {
                            app.error_message = Some(format!("Repo '{}' already added.", name));
                        } else {
                            app.config.repos.push(crate::models::Repo { name, path: abs_path });
                            app.save_config();
                            app.input_mode = InputMode::Normal;
                            app.error_message = None;
                            app.full_error_detail = None;
                        }
                    }
                    Err(e) => {
                        app.error_message = Some(e.to_string());
                        app.full_error_detail = Some(e.to_string());
                    }
                }
            }
            KeyCode::Tab => {
                if app.path_completions.is_empty() {
                    app.update_completions();
                }
                if !app.path_completions.is_empty() {
                    let idx = match app.completion_idx {
                        Some(i) => (i + 1) % app.path_completions.len(),
                        None => 0,
                    };
                    app.completion_idx = Some(idx);
                    app.input = app.path_completions[idx].clone();
                }
            }
            KeyCode::Char(c) => {
                app.input.push(c);
                app.error_message = None;
                app.path_completions.clear();
            }
            KeyCode::Backspace => {
                app.input.pop();
                app.path_completions.clear();
            }
            KeyCode::Esc => {
                app.input_mode = InputMode::Normal;
                app.input.clear();
                app.error_message = None;
                app.full_error_detail = None;
            }
            _ => {}
        },

        // ── Project name (step 1) ─────────────────────────────────────────
        InputMode::AddingProjectName => match key.code {
            KeyCode::Enter => {
                let name = app.input.trim().to_string();
                if name.is_empty() {
                    app.error_message = Some("Project name cannot be empty.".to_string());
                } else if app.config.projects.iter().any(|p| p.name == name) {
                    app.error_message = Some(format!("Project '{}' already exists.", name));
                } else {
                    app.pending_project_name = name;
                    app.input.clear();
                    app.input_mode = InputMode::AddingProjectBranch;
                    app.error_message = None;
                }
            }
            KeyCode::Char(c) => { app.input.push(c); app.error_message = None; }
            KeyCode::Backspace => { app.input.pop(); }
            KeyCode::Esc => {
                app.input_mode = InputMode::Normal;
                app.input.clear();
                app.error_message = None;
            }
            _ => {}
        },

        // ── Branch name (step 2) ──────────────────────────────────────────
        InputMode::AddingProjectBranch => match key.code {
            KeyCode::Enter => {
                let branch = app.input.trim().to_string();
                if branch.is_empty() {
                    app.error_message = Some("Branch name cannot be empty.".to_string());
                } else {
                    app.pending_project_branch = branch;
                    app.input.clear();
                    if app.config.repos.is_empty() {
                        // No repos yet — create empty project immediately
                        let project_name = app.pending_project_name.clone();
                        let project_branch = app.pending_project_branch.clone();
                        let folder = Project::make_folder_path(&project_name);
                        let project = Project {
                            name: project_name.clone(),
                            branch: project_branch,
                            worktrees: Vec::new(),
                            folder: folder.clone(),
                        };
                        let _ = project.create_folder();
                        app.config.projects.push(project);
                        let new_p_idx = app.config.projects.len() - 1;
                        app.expanded_projects.insert(new_p_idx);
                        app.save_config();
                        let items = app.get_tree_items();
                        if let Some(idx) = items.iter().position(|(_, s, _)| *s == Selection::Project(new_p_idx)) {
                            app.tree_state.select(Some(idx));
                        }
                        app.input_mode = InputMode::Normal;
                        app.error_message = Some("Project created. Add repos with 'a', then add worktrees with 'w'.".to_string());
                    } else {
                        // Move to repo selection
                        app.adding_to_project = None;
                        app.repo_selection = vec![false; app.config.repos.len()];
                        app.repo_cursor = 0;
                        app.input_mode = InputMode::SelectingRepos;
                        app.error_message = None;
                    }
                }
            }
            KeyCode::Char(c) => { app.input.push(c); app.error_message = None; }
            KeyCode::Backspace => { app.input.pop(); }
            KeyCode::Esc => {
                app.input_mode = InputMode::Normal;
                app.input.clear();
                app.error_message = None;
            }
            _ => {}
        },

        // ── Repo multi-select (step 3 or add-to-project) ──────────────────
        InputMode::SelectingRepos => match key.code {
            KeyCode::Up => {
                if app.repo_cursor > 0 {
                    app.repo_cursor -= 1;
                }
            }
            KeyCode::Down => {
                let available = app.available_repos();
                if app.repo_cursor + 1 < available.len() {
                    app.repo_cursor += 1;
                }
            }
            KeyCode::Char(' ') => {
                let available = app.available_repos();
                if let Some((repo_idx, _)) = available.get(app.repo_cursor) {
                    let repo_idx = *repo_idx;
                    if app.repo_selection.len() <= repo_idx {
                        app.repo_selection.resize(repo_idx + 1, false);
                    }
                    app.repo_selection[repo_idx] = !app.repo_selection[repo_idx];
                }
            }
            KeyCode::Enter => {
                handle_confirm_repo_selection(app);
            }
            KeyCode::Esc => {
                app.input_mode = InputMode::Normal;
                app.repo_selection.clear();
                app.adding_to_project = None;
                app.error_message = None;
            }
            _ => {}
        },

        // ── Commit message → push ─────────────────────────────────────────
        InputMode::EditingCommitMessage => match key.code {
            KeyCode::Enter => {
                let commit_msg = if app.input.trim().is_empty() {
                    None
                } else {
                    Some(app.input.trim().to_string())
                };

                match app.get_selected_selection() {
                    Some(sel @ Selection::Worktree(p_idx, w_idx)) => {
                        handle_push_single(app, sel, p_idx, w_idx, commit_msg);
                    }
                    Some(Selection::Project(p_idx)) => {
                        handle_push_project(app, p_idx, commit_msg);
                    }
                    _ => {}
                }
                app.input_mode = InputMode::Normal;
                app.input.clear();
            }
            KeyCode::Char(c) => app.input.push(c),
            KeyCode::Backspace => { app.input.pop(); }
            KeyCode::Esc => {
                app.input_mode = InputMode::Normal;
                app.input.clear();
                app.error_message = None;
                app.full_error_detail = None;
            }
            _ => {}
        },
    }

    Ok(AppState::Continue)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Removes an entire project: all worktrees, project folder, config entry.
fn handle_remove_project(app: &mut App, p_idx: usize) {
    let project = app.config.projects[p_idx].clone();
    let mut errors: Vec<String> = Vec::new();

    // Remove each git worktree via its parent repo
    for wt in &project.worktrees {
        if let Some(repo) = app.config.repos.iter().find(|r| r.name == wt.repo_name) {
            if let Err(e) = repo.remove_worktree(&wt.path) {
                errors.push(format!("[{}] remove worktree error: {}", wt.repo_name, e));
            }
        }
    }

    // Remove project folder (symlinks)
    if let Err(e) = project.remove_folder() {
        errors.push(format!("remove project folder: {}", e));
    }

    // Remove session if open
    app.sessions.remove(&Selection::Project(p_idx));
    for w_idx in 0..project.worktrees.len() {
        app.sessions.remove(&Selection::Worktree(p_idx, w_idx));
    }

    app.config.projects.remove(p_idx);
    // Re-index expanded_projects
    let updated: std::collections::HashSet<usize> = app.expanded_projects.iter()
        .filter_map(|&i| if i == p_idx { None } else if i > p_idx { Some(i - 1) } else { Some(i) })
        .collect();
    app.expanded_projects = updated;

    app.save_config();
    app.refresh_worktree_status();

    if errors.is_empty() {
        app.error_message = None;
        app.command_output.clear();
    } else {
        app.command_output = errors;
        app.error_message = Some("Some errors during project removal (see output).".to_string());
    }

    let items = app.get_tree_items();
    if items.is_empty() {
        app.tree_state.select(None);
    } else {
        let idx = p_idx.min(items.len() - 1);
        app.tree_state.select(Some(idx));
    }
}

/// Removes a single worktree from a project.
fn handle_remove_worktree(app: &mut App, p_idx: usize, w_idx: usize) {
    let wt = app.config.projects[p_idx].worktrees[w_idx].clone();
    let project_folder = app.config.projects[p_idx].folder.clone();

    // Remove git worktree
    let git_result = app.config.repos.iter()
        .find(|r| r.name == wt.repo_name)
        .map(|repo| repo.remove_worktree(&wt.path));

    match git_result {
        Some(Ok(out)) if out.status.success() || !wt.path.exists() => {
            // Remove symlink from project folder
            let link = project_folder.join(&wt.repo_name);
            let _ = std::fs::remove_file(&link);

            app.sessions.remove(&Selection::Worktree(p_idx, w_idx));
            app.config.projects[p_idx].worktrees.remove(w_idx);
            app.save_config();
            app.refresh_worktree_status();
            app.error_message = None;
            app.full_error_detail = None;

            // Navigate to project row after removing worktree
            let items = app.get_tree_items();
            if let Some(proj_idx) = items.iter().position(|(_, s, _)| *s == Selection::Project(p_idx)) {
                app.tree_state.select(Some(proj_idx));
            }
        }
        Some(Ok(out)) => {
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            app.error_message = Some("Failed to remove worktree".to_string());
            app.full_error_detail = Some(stderr);
        }
        Some(Err(e)) => {
            app.error_message = Some("System error removing worktree".to_string());
            app.full_error_detail = Some(e.to_string());
        }
        None => {
            // Repo no longer in registry — remove from project anyway
            let link = project_folder.join(&wt.repo_name);
            let _ = std::fs::remove_file(&link);
            app.sessions.remove(&Selection::Worktree(p_idx, w_idx));
            app.config.projects[p_idx].worktrees.remove(w_idx);
            app.save_config();
            app.refresh_worktree_status();
            app.error_message = None;
        }
    }
}

/// Confirms repo selection and creates worktrees (new project or add to existing).
fn handle_confirm_repo_selection(app: &mut App) {
    let selected_repo_indices: Vec<usize> = app.available_repos()
        .iter()
        .filter(|(repo_idx, _)| app.repo_selection.get(*repo_idx).copied().unwrap_or(false))
        .map(|(repo_idx, _)| *repo_idx)
        .collect();

    if selected_repo_indices.is_empty() {
        if app.adding_to_project.is_some() {
            app.error_message = Some("No repos selected. Use Space to select.".to_string());
            return;
        }
        // Creating a new project with no repos selected — create empty project
        let project_name = app.pending_project_name.clone();
        let project_branch = app.pending_project_branch.clone();
        let folder = Project::make_folder_path(&project_name);
        let project = Project {
            name: project_name.clone(),
            branch: project_branch,
            worktrees: Vec::new(),
            folder: folder.clone(),
        };
        let _ = project.create_folder();
        app.config.projects.push(project);
        let new_p_idx = app.config.projects.len() - 1;
        app.expanded_projects.insert(new_p_idx);
        app.save_config();
        let items = app.get_tree_items();
        if let Some(idx) = items.iter().position(|(_, s, _)| *s == Selection::Project(new_p_idx)) {
            app.tree_state.select(Some(idx));
        }
        app.input_mode = InputMode::Normal;
        app.repo_selection.clear();
        app.adding_to_project = None;
        app.error_message = Some("Project created with no repos. Use 'w' to add worktrees.".to_string());
        return;
    }

    let branch = app.pending_project_branch.clone();
    let project_name = app.pending_project_name.clone();

    let mut created: Vec<ProjectWorktree> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    for &repo_idx in &selected_repo_indices {
        let repo = app.config.repos[repo_idx].clone();
        match repo.add_worktree(&branch) {
            Ok((out, wt_path)) if out.status.success() => {
                created.push(ProjectWorktree {
                    repo_name: repo.name.clone(),
                    path: wt_path,
                });
            }
            Ok((out, _)) => {
                let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                errors.push(format!("[{}] {}", repo.name, stderr.trim()));
            }
            Err(e) => {
                errors.push(format!("[{}] {}", repo.name, e));
            }
        }
    }

    if let Some(p_idx) = app.adding_to_project {
        // Adding to existing project
        for wt in &created {
            let _ = app.config.projects[p_idx].add_symlink(wt);
            app.config.projects[p_idx].worktrees.push(wt.clone());
        }
        app.save_config();
        app.refresh_worktree_status();

        // Navigate to the project
        let items = app.get_tree_items();
        if let Some(idx) = items.iter().position(|(_, s, _)| *s == Selection::Project(p_idx)) {
            app.tree_state.select(Some(idx));
        }
    } else {
        // Creating new project
        if !created.is_empty() {
            let folder = Project::make_folder_path(&project_name);
            let worktrees_empty = created.is_empty();
            let project = Project {
                name: project_name.clone(),
                branch: branch.clone(),
                worktrees: created,
                folder: folder.clone(),
            };
            let _ = project.create_folder();
            app.config.projects.push(project);
            let new_p_idx = app.config.projects.len() - 1;
            app.expanded_projects.insert(new_p_idx);
            app.save_config();
            app.refresh_worktree_status();

            let items = app.get_tree_items();
            if let Some(idx) = items.iter().position(|(_, s, _)| *s == Selection::Project(new_p_idx)) {
                app.tree_state.select(Some(idx));
            }
            let _ = worktrees_empty; // used below
        }
    }

    if errors.is_empty() {
        app.command_output.clear();
        app.error_message = None;
    } else {
        app.command_output = errors.clone();
        let new_project_failed = app.adding_to_project.is_none()
            && app.config.projects.last().map(|p| p.worktrees.is_empty()).unwrap_or(true);
        if new_project_failed {
            app.error_message = Some("All worktree creations failed.".to_string());
        } else {
            app.error_message = Some(format!("{} worktree(s) failed (see output).", errors.len()));
        }
    }

    app.input_mode = InputMode::Normal;
    app.repo_selection.clear();
    app.adding_to_project = None;
}

/// Push a single worktree.
fn handle_push_single(app: &mut App, sel: Selection, p_idx: usize, w_idx: usize, commit_msg: Option<String>) {
    match app.config.projects[p_idx].worktrees[w_idx].push(commit_msg) {
        Ok((add_out, commit_out, push_out)) => {
            let mut full_output = Vec::new();
            full_output.extend_from_slice(&add_out.stdout);
            full_output.extend_from_slice(&add_out.stderr);
            full_output.extend_from_slice(&commit_out.stdout);
            full_output.extend_from_slice(&commit_out.stderr);
            full_output.extend_from_slice(&push_out.stdout);
            full_output.extend_from_slice(&push_out.stderr);

            if let Some(session) = app.sessions.get(&sel) {
                session.parser.lock().unwrap().process(&full_output);
            } else {
                app.command_output = String::from_utf8_lossy(&full_output).lines().map(String::from).collect();
            }

            if push_out.status.success() {
                app.refresh_worktree_status();
                app.error_message = None;
                app.full_error_detail = None;
                if !app.sessions.contains_key(&sel) {
                    let mut success = b"Push successful!\n".to_vec();
                    success.extend(full_output);
                    app.command_output = String::from_utf8_lossy(&success).lines().map(String::from).collect();
                }
            } else {
                app.error_message = Some("Push failed (Ctrl+L to export log)".to_string());
                app.full_error_detail = Some(app.command_output.join("\n"));
            }
        }
        Err(e) => {
            app.error_message = Some("System error during push".to_string());
            app.full_error_detail = Some(e.to_string());
        }
    }
}

/// Push all worktrees in a project.
fn handle_push_project(app: &mut App, p_idx: usize, commit_msg: Option<String>) {
    let worktrees = app.config.projects[p_idx].worktrees.clone();
    let mut results: Vec<String> = Vec::new();
    let mut all_success = true;

    for wt in &worktrees {
        match wt.push(commit_msg.clone()) {
            Ok((_, commit_out, push_out)) => {
                let pushed = push_out.status.success();
                let committed = commit_out.status.success();
                if !pushed { all_success = false; }

                let status_icon = if pushed { "✓" } else { "✗" };
                let detail = if !committed {
                    let stderr = String::from_utf8_lossy(&commit_out.stderr).trim().to_string();
                    if stderr.contains("nothing to commit") {
                        "nothing to commit".to_string()
                    } else {
                        stderr
                    }
                } else if !pushed {
                    String::from_utf8_lossy(&push_out.stderr).trim().to_string()
                } else {
                    "pushed".to_string()
                };

                results.push(format!("{} [{}]  {}", status_icon, wt.repo_name, detail));
            }
            Err(e) => {
                all_success = false;
                results.push(format!("✗ [{}]  error: {}", wt.repo_name, e));
            }
        }
    }

    app.command_output = results;
    if all_success {
        app.refresh_worktree_status();
        app.error_message = None;
        app.full_error_detail = None;
    } else {
        app.error_message = Some("Some pushes failed (see output, Ctrl+L to export)".to_string());
        app.full_error_detail = Some(app.command_output.join("\n"));
    }
}

/// Sanitizes a string for use as a tmux session name.
fn sanitize_tmux_name(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect()
}
