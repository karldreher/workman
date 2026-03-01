use crate::terminal_handler;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::{fs, path::PathBuf};

use crate::app::{App, InputMode, Selection};
use crate::models::{Config, Project, Worktree};
use crate::session::Session;

pub enum AppState {
    Continue,
    Quit,
}

pub async fn handle_key_event(
    key: KeyEvent,
    app: &mut App,
    current_width: u16,
    current_height: u16,
) -> Result<AppState> {
    // Global Ctrl+C handler (except in Terminal mode where it might be sent to PTY)
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        if app.input_mode != InputMode::Terminal {
            return Ok(AppState::Quit);
        }
    }

    // Global Ctrl+L
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('l') {
        if let Some(detail) = &app.full_error_detail {
            let _ = fs::write("/tmp/workman.log", detail);
            app.error_message = Some("Log exported to /tmp/workman.log".to_string());
        } else if let Some(err) = &app.error_message {
            let _ = fs::write("/tmp/workman.log", err);
            app.error_message = Some("Status exported to /tmp/workman.log".to_string());
        }
    }

    match app.input_mode {
        InputMode::Normal => match key.code {
            KeyCode::Char('q') => return Ok(AppState::Quit),
            KeyCode::Char('a') => {
                app.input_mode = InputMode::AddingProjectPath;
                app.input.clear();
                app.error_message = None;
                app.full_error_detail = None;
            }
            KeyCode::Char('x') => {
                if let Some(Selection::Project(p_idx)) = app.get_selected_selection() {
                    app.config.projects.remove(p_idx);
                    app.save_config();
                    if app.config.projects.is_empty() {
                        app.tree_state.select(None);
                    } else {
                        let new_idx = if p_idx >= app.config.projects.len() { app.config.projects.len() - 1 } else { p_idx };
                        app.tree_state.select(Some(new_idx));
                    }
                }
                app.error_message = None;
                app.full_error_detail = None;
            }
            KeyCode::Char('w') => {
                if let Some(Selection::Project(_)) = app.get_selected_selection() {
                    app.input_mode = InputMode::AddingWorktreeName;
                    app.input.clear();
                    app.error_message = None;
                    app.full_error_detail = None;
                }
            }
            KeyCode::Char('r') => {
                if let Some(sel @ Selection::Worktree(_p_idx, _w_idx)) = app.get_selected_selection() {
                    let p_idx = match sel {
                        Selection::Worktree(p, _) => p,
                        _ => unreachable!(), // This case is prevented by the if let Some
                    };
                    let w_idx = match sel {
                        Selection::Worktree(_, w) => w,
                        _ => unreachable!(), // This case is prevented by the if let Some
                    };

                    match app.config.projects[p_idx].remove_worktree(w_idx) {
                        Ok(out) => {
                            let mut full_output = Vec::new();
                            full_output.extend_from_slice(&out.stdout);
                            full_output.extend_from_slice(&out.stderr);

                            if let Some(session) = app.sessions.get(&sel) {
                                session.parser.lock().unwrap().process(&full_output);
                            } else {
                                app.command_output = String::from_utf8_lossy(&full_output).lines().map(String::from).collect();
                            }

                            if out.status.success() {
                                app.config.projects[p_idx].worktrees.remove(w_idx);
                                app.save_config();
                                app.error_message = None;
                                app.full_error_detail = None;
                                if app.config.projects[p_idx].worktrees.is_empty() {
                                    app.tree_state.select(None);
                                } else {
                                    let new_idx = if w_idx >= app.config.projects[p_idx].worktrees.len() { app.config.projects[p_idx].worktrees.len() - 1 } else { w_idx };
                                    let items = app.get_tree_items();
                                    if let Some(new_sel_idx) = items.iter().position(|(_, s, _)| *s == Selection::Worktree(p_idx, new_idx)) {
                                        app.tree_state.select(Some(new_sel_idx));
                                    } else if let Some(proj_sel_idx) = items.iter().position(|(_, s, _)| *s == Selection::Project(p_idx)) {
                                        app.tree_state.select(Some(proj_sel_idx));
                                    }
                                }
                            } else {
                                app.error_message = Some("Failed to remove worktree".to_string());
                                if !app.sessions.contains_key(&sel) {
                                    app.full_error_detail = Some(app.command_output.join("\n"));
                                }
                            }
                        },
                        Err(e) => {
                            app.error_message = Some("System error occurred".to_string());
                            app.full_error_detail = Some(e.to_string());
                        }
                    }
                }
            }
            KeyCode::Char('c') => {
                if let Some(sel) = app.get_selected_selection() {
                    if let Selection::Worktree(p_idx, w_idx) = sel {
                        if !app.sessions.contains_key(&sel) {
                            let path = app.config.projects[p_idx].worktrees[w_idx].path.clone();
                            match Session::new(path, current_width, current_height) {
                                Ok(session) => {
                                    app.sessions.insert(sel, session);
                                }
                                Err(e) => {
                                    app.error_message = Some(format!("Failed to start session: {}", e));
                                }
                            }
                        }
                        if app.sessions.contains_key(&sel) {
                            app.input_mode = InputMode::Terminal;
                        }
                    }
                }
            }
            KeyCode::Char('p') => {
                if let Some(_sel @ Selection::Worktree(_p_idx, _w_idx)) = app.get_selected_selection() {
                    app.input_mode = InputMode::EditingCommitMessage;
                    app.input.clear();
                    app.error_message = None;
                    app.full_error_detail = None;
                }
            }
            KeyCode::Char('d') => {
                if let Some(sel @ Selection::Worktree(_p_idx, _w_idx)) = app.get_selected_selection() {
                    match app.config.projects[_p_idx].worktrees[_w_idx].get_diff() {
                        Ok(out) => {
                            let mut full_output = Vec::new();
                            full_output.extend_from_slice(&out.stdout);
                            full_output.extend_from_slice(&out.stderr);

                            if let Some(session) = app.sessions.get(&sel) {
                                session.parser.lock().unwrap().process(&full_output);
                            } else {
                                app.command_output = String::from_utf8_lossy(&full_output).lines().map(String::from).collect();
                            }

                            if !out.status.success() {
                                app.error_message = Some("Failed to get diff".to_string());
                                if !app.sessions.contains_key(&sel) {
                                    app.full_error_detail = Some(app.command_output.join("\n"));
                                }
                                app.input_mode = InputMode::Normal;
                                app.diff_scroll_offset = 0;
                            } else {
                                if app.command_output.is_empty() && !app.sessions.contains_key(&sel) {
                                    app.error_message = Some("No changes to display diff for.".to_string());
                                    app.full_error_detail = None;
                                    app.command_output.clear();
                                    app.input_mode = InputMode::Normal;
                                    app.diff_scroll_offset = 0;
                                } else {
                                    app.input_mode = InputMode::ViewingDiff;
                                    app.error_message = None;
                                    app.full_error_detail = None;
                                    app.diff_scroll_offset = 0;
                                }
                            }
                        },
                        Err(e) => {
                            app.error_message = Some("System error occurred while getting diff".to_string());
                            app.full_error_detail = Some(e.to_string());
                            app.input_mode = InputMode::Normal;
                            app.diff_scroll_offset = 0;
                        }
                    }
                }
            }
            KeyCode::Down => app.next(),
            KeyCode::Up => app.previous(),
            KeyCode::Esc => {
                app.error_message = None;
                app.full_error_detail = None;
            }
            _ => {}
        },
        InputMode::Terminal => terminal_handler::handle_terminal_key_event(key, app),
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
                app.command_output.clear(); // Clear traditional output, session output remains
                app.diff_scroll_offset = 0;
            }
            _ => {}
        },
        InputMode::AddingProjectPath => match key.code {
            KeyCode::Enter => {
                let path_str = app.input.trim().to_string();
                let path = PathBuf::from(&path_str);
                match Config::validate_project_path(&path) {
                    Ok(_) => {
                        let abs_path = fs::canonicalize(&path).unwrap();
                        let name = abs_path.file_name().unwrap().to_string_lossy().to_string();
                        app.config.projects.push(Project {
                            name,
                            path: abs_path,
                            worktrees: Vec::new(),
                        });
                        app.save_config();
                        app.input_mode = InputMode::Normal;
                        let items = app.get_tree_items();
                        if let Some(new_sel_idx) = items.iter().position(|(_, sel, _)| {
                            if let Selection::Project(p_idx) = sel {
                                *p_idx == app.config.projects.len() - 1
                            } else { false }
                        }) {
                            app.tree_state.select(Some(new_sel_idx));
                        }
                        app.error_message = None;
                        app.full_error_detail = None;
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
                app.error_message = None;
                app.full_error_detail = None;
                app.input.clear();
            }
            _ => {}
        },
        InputMode::AddingWorktreeName => match key.code {
            KeyCode::Enter => {
                let name = app.input.trim().to_string();
                if name.is_empty() {
                    app.error_message = Some("Worktree name cannot be empty".to_string());
                    app.full_error_detail = Some("Worktree name cannot be empty".to_string());
                    return Ok(AppState::Continue);
                }

                if let Some(Selection::Project(p_idx)) = app.get_selected_selection() {
                    let wt_name = name.clone();
                    let branch = name;
                    let workman_dir = app.config.projects[p_idx].path.join(".workman");
                    let wt_path = workman_dir.join(&wt_name);

                    match app.config.projects[p_idx].add_worktree(&wt_name, wt_path.clone(), &branch) {
                        Ok(out) if out.status.success() => {
                            let mut full_output = Vec::new();
                            full_output.extend_from_slice(&out.stdout);
                            full_output.extend_from_slice(&out.stderr);

                            if let Some(session) = app.sessions.get(&Selection::Worktree(p_idx, app.config.projects[p_idx].worktrees.len())) { // Predicting the selection for the new worktree
                                session.parser.lock().unwrap().process(&full_output);
                            } else {
                                app.command_output = String::from_utf8_lossy(&full_output).lines().map(String::from).collect();
                            }

                            app.config.projects[p_idx].worktrees.push(Worktree {
                                name: wt_name,
                                path: wt_path,
                            });
                            app.save_config();
                            app.input_mode = InputMode::Normal;
                            app.error_message = None;
                            app.full_error_detail = None;
                            app.input.clear();
                            let items = app.get_tree_items();
                            if let Some(new_sel_idx) = items.iter().position(|(_, sel, _)| {
                                if let Selection::Worktree(proj_idx, wt_idx) = sel {
                                    *proj_idx == p_idx && *wt_idx == app.config.projects[p_idx].worktrees.len() - 1
                                } else { false }
                            }) {
                                app.tree_state.select(Some(new_sel_idx));
                            }
                        }
                        Ok(out) => {
                            let mut full_output = Vec::new();
                            full_output.extend_from_slice(&out.stdout);
                            full_output.extend_from_slice(&out.stderr);
                            if let Some(session) = app.sessions.get(&Selection::Project(p_idx)) { // Fallback if no worktree selected
                                session.parser.lock().unwrap().process(&full_output);
                            } else {
                                app.command_output = String::from_utf8_lossy(&full_output).lines().map(String::from).collect();
                            }
                            app.error_message = Some("Worktree creation failed (Ctrl+L to export log)".to_string());
                            if !app.sessions.contains_key(&Selection::Project(p_idx)) {
                                app.full_error_detail = Some(app.command_output.join("\n"));
                            }
                            app.input = branch;
                        }
                        Err(e) => {
                            app.error_message = Some("System error occurred".to_string());
                            app.full_error_detail = Some(e.to_string());
                            app.input = branch;
                        }
                    }
                } else {
                    app.error_message = Some("No project selected to add worktree to.".to_string());
                    app.full_error_detail = Some("No project selected to add worktree to.".to_string());
                }
            }
            KeyCode::Char(c) => app.input.push(c),
            KeyCode::Backspace => { app.input.pop(); }
            KeyCode::Esc => {
                app.input_mode = InputMode::Normal;
                app.error_message = None;
                app.full_error_detail = None;
                app.input.clear();
            }
            _ => {}
        },

        InputMode::EditingCommitMessage => match key.code {
            KeyCode::Enter => {
                let commit_msg = if app.input.trim().is_empty() {
                    None
                } else {
                    Some(app.input.trim().to_string())
                };

                if let Some(sel @ Selection::Worktree(p_idx, w_idx)) = app.get_selected_selection() {
                    match app.config.projects[p_idx].worktrees[w_idx].push(commit_msg) {
                        Ok((add_out, commit_out, push_out)) => {
                            let mut full_output = Vec::new();

                            // Collect all outputs
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

                            // Check if push succeeded
                            if !push_out.status.success() {
                                app.error_message = Some("Push failed".to_string());
                                if !app.sessions.contains_key(&sel) {
                                    app.full_error_detail = Some(app.command_output.join("\n"));
                                }
                            } else {
                                // Success: prepend success message to output
                                let mut success_output = "Push successful!\n".to_string().into_bytes();
                                success_output.extend(full_output.clone());
                                if let Some(session) = app.sessions.get(&sel) {
                                    session.parser.lock().unwrap().process(&success_output);
                                } else {
                                    app.command_output = String::from_utf8_lossy(&success_output).lines().map(String::from).collect();
                                }
                                app.error_message = None;
                                app.full_error_detail = None;
                            }
                        },
                        Err(e) => {
                            app.error_message = Some("System error occurred during push".to_string());
                            app.full_error_detail = Some(e.to_string());
                        }
                    }
                }
                app.input_mode = InputMode::Normal;
                app.input.clear();
            }
            KeyCode::Char(c) => app.input.push(c),
            KeyCode::Backspace => { app.input.pop(); }
            KeyCode::Esc => {
                app.input_mode = InputMode::Normal;
                app.error_message = None;
                app.full_error_detail = None;
                app.input.clear();
            }
            _ => {}
        },
    }

    Ok(AppState::Continue)
}
