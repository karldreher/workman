mod app;
mod models;
mod ui;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};
use std::{fs, io, path::PathBuf, sync::atomic::{AtomicBool, Ordering}, sync::Arc};
use crate::app::{App, InputMode, Selection};
use crate::models::{Config, Project, Worktree};
use crate::ui::ui;

struct TerminalRestorer;

impl Drop for TerminalRestorer {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let mut stdout = io::stdout();
        let _ = execute!(stdout, LeaveAlternateScreen);
    }
}

fn main() -> Result<()> {
    let term_restorer = TerminalRestorer;
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    
    // Handle SIGTERM
    let mut signals = signal_hook::iterator::Signals::new(&[signal_hook::consts::SIGTERM, signal_hook::consts::SIGINT])?;
    std::thread::spawn(move || {
        for _ in signals.forever() {
            r.store(false, Ordering::SeqCst);
            break;
        }
    });

    let app = App::new();
    let res = run_app(&mut terminal, app, running);

    drop(term_restorer);

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend + io::Write>(terminal: &mut Terminal<B>, mut app: App, running: Arc<AtomicBool>) -> Result<()> {
    while running.load(Ordering::SeqCst) {
        terminal.draw(|f| ui(f, &mut app)).map_err(|e| anyhow::anyhow!(e.to_string()))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                        // Global Ctrl+C handler
                        if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                            return Ok(());
                        }

                        // Global Ctrl+L to export current error log for copy-paste
                        if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) && key.code == KeyCode::Char('l') {
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
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Char('a') => {
                            app.input_mode = InputMode::AddingProjectPath;
                            app.input.clear();
                            app.error_message = None;
                            app.full_error_detail = None;
                            app.command_output.clear();
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
                            app.command_output.clear();
                        }
                        KeyCode::Char('w') => {
                            if let Some(Selection::Project(_)) = app.get_selected_selection() {
                                app.input_mode = InputMode::AddingWorktreeName;
                                app.input.clear();
                                app.error_message = None;
                                app.full_error_detail = None;
                                app.command_output.clear();
                            }
                        }
                        KeyCode::Char('r') => {
                            if let Some(Selection::Worktree(p_idx, w_idx)) = app.get_selected_selection() {
                                match app.config.projects[p_idx].remove_worktree(w_idx) {
                                    Ok(out) => {
                                        app.command_output = String::from_utf8_lossy(&out.stdout).lines().map(String::from).collect();
                                        if !out.stderr.is_empty() {
                                            app.command_output.extend(String::from_utf8_lossy(&out.stderr).lines().map(String::from));
                                        }
                                        if out.status.success() {
                                            app.config.projects[p_idx].worktrees.remove(w_idx);
                                            app.save_config();
                                            app.error_message = None;
                                            app.full_error_detail = None;
                                            app.command_output.clear();
                                            if app.config.projects[p_idx].worktrees.is_empty() {
                                                app.tree_state.select(None);
                                            } else {
                                                let new_idx = if w_idx >= app.config.projects[p_idx].worktrees.len() { app.config.projects[p_idx].worktrees.len() - 1 } else { w_idx };
                                                let items = app.get_tree_items();
                                                if let Some(new_sel_idx) = items.iter().position(|(_, sel, _)| *sel == Selection::Worktree(p_idx, new_idx)) {
                                                    app.tree_state.select(Some(new_sel_idx));
                                                } else if let Some(proj_sel_idx) = items.iter().position(|(_, sel, _)| *sel == Selection::Project(p_idx)) {
                                                    app.tree_state.select(Some(proj_sel_idx));
                                                }
                                            }
                                        } else {
                                            app.error_message = Some("Failed to remove worktree".to_string());
                                            app.full_error_detail = Some(app.command_output.join("\n"));
                                        }
                                    },
                                    Err(e) => {
                                        app.error_message = Some("System error occurred".to_string());
                                        app.full_error_detail = Some(e.to_string());
                                        app.command_output.clear();
                                    }
                                }
                            }
                        }
                        KeyCode::Char('c') => {
                            if let Some(Selection::Worktree(_p_idx, _w_idx)) = app.get_selected_selection() {
                                app.input_mode = InputMode::RunningCommand;
                                app.input.clear();
                                app.error_message = None;
                                app.full_error_detail = None;
                                app.command_output.clear();
                            }
                        }
                        KeyCode::Char('p') => {
                            if let Some(Selection::Worktree(_p_idx, _w_idx)) = app.get_selected_selection() {
                                app.input_mode = InputMode::EditingCommitMessage;
                                app.input.clear();
                                app.error_message = None;
                                app.full_error_detail = None;
                                app.command_output.clear();
                            }
                        }
                        KeyCode::Char('d') => {
                            if let Some(Selection::Worktree(p_idx, w_idx)) = app.get_selected_selection() {
                                match app.config.projects[p_idx].worktrees[w_idx].get_diff() {
                                    Ok(out) => {
                                        app.command_output = String::from_utf8_lossy(&out.stdout).lines().map(String::from).collect();
                                        if !out.stderr.is_empty() {
                                            app.command_output.extend(String::from_utf8_lossy(&out.stderr).lines().map(String::from));
                                        }
                                        if !out.status.success() {
                                            app.error_message = Some("Failed to get diff".to_string());
                                            app.full_error_detail = Some(app.command_output.join("\n"));
                                            app.input_mode = InputMode::Normal;
                                            app.diff_scroll_offset = 0;
                                        } else {
                                            if app.command_output.is_empty() {
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
                            app.command_output.clear();
                            app.error_message = None;
                            app.full_error_detail = None;
                        }
                        _ => {}
                    },
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
                                    app.command_output.clear();
                                }
                                Err(e) => {
                                    app.error_message = Some(e.to_string());
                                    app.full_error_detail = Some(e.to_string());
                                    app.command_output.clear();
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
                            app.command_output.clear();
                        }
                        _ => {}
                    },
                    InputMode::AddingWorktreeName => match key.code {
                        KeyCode::Enter => {
                            let name = app.input.trim().to_string();
                            if name.is_empty() {
                                app.error_message = Some("Worktree name cannot be empty".to_string());
                                app.full_error_detail = Some("Worktree name cannot be empty".to_string());
                                app.command_output.clear();
                                return Ok(());
                            }
                            
                            if let Some(Selection::Project(p_idx)) = app.get_selected_selection() {
                                let wt_name = name.clone();
                                let branch = name;
                                let workman_dir = app.config.projects[p_idx].path.join(".workman");
                                let wt_path = workman_dir.join(&wt_name);

                                match app.config.projects[p_idx].add_worktree(&wt_name, wt_path.clone(), &branch) {
                                    Ok(out) if out.status.success() => {
                                        app.command_output = String::from_utf8_lossy(&out.stdout).lines().map(String::from).collect();
                                        if !out.stderr.is_empty() {
                                            app.command_output.extend(String::from_utf8_lossy(&out.stderr).lines().map(String::from));
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
                                        app.command_output.clear();
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
                                        let err_output = String::from_utf8_lossy(&out.stderr).trim().to_string();
                                        app.command_output = String::from_utf8_lossy(&out.stdout).lines().map(String::from).collect();
                                        app.command_output.extend(err_output.lines().map(String::from));

                                        app.error_message = Some("Worktree creation failed (Ctrl+L to export log)".to_string());
                                        app.full_error_detail = Some(app.command_output.join("\n"));
                                        app.input = branch;
                                    }
                                    Err(e) => {
                                        app.error_message = Some("System error occurred".to_string());
                                        app.full_error_detail = Some(e.to_string());
                                        app.command_output.clear();
                                        app.input = branch;
                                    }
                                }
                            } else {
                                app.error_message = Some("No project selected to add worktree to.".to_string());
                                app.full_error_detail = Some("No project selected to add worktree to.".to_string());
                                app.command_output.clear();
                            }
                        }
                        KeyCode::Char(c) => app.input.push(c),
                        KeyCode::Backspace => { app.input.pop(); }
                        KeyCode::Esc => {
                            app.input_mode = InputMode::Normal;
                            app.error_message = None;
                            app.full_error_detail = None;
                            app.input.clear();
                            app.command_output.clear();
                        }
                        _ => {}
                    },

                    InputMode::RunningCommand => match key.code {
                        KeyCode::Enter => {
                            let cmd = app.input.drain(..).collect::<String>();
                            if let Some(Selection::Worktree(p_idx, w_idx)) = app.get_selected_selection() {
                                let wt_path = app.config.projects[p_idx].worktrees[w_idx].path.clone();
                                
                                disable_raw_mode()?;
                                execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                                
                                let shell = std::env::var("SHELL").unwrap_or_else(|_| "sh".to_string());
                                let output = std::process::Command::new(shell)
                                    .arg("-c")
                                    .arg(&cmd)
                                    .current_dir(wt_path)
                                    .output();
                                
                                execute!(terminal.backend_mut(), EnterAlternateScreen)?;
                                enable_raw_mode()?;
                                terminal.clear().map_err(|e| anyhow::anyhow!(e.to_string()))?;

                                match output {
                                    Ok(out) => {
                                        app.command_output = String::from_utf8_lossy(&out.stdout).lines().map(String::from).collect();
                                        if !out.stderr.is_empty() {
                                            app.command_output.extend(String::from_utf8_lossy(&out.stderr).lines().map(String::from));
                                        }
                                        if !out.status.success() {
                                            app.error_message = Some(format!("Command failed: {}", cmd));
                                            app.full_error_detail = Some(app.command_output.join("\n"));
                                        } else {
                                            app.error_message = None;
                                        }
                                    },
                                    Err(e) => {
                                        app.error_message = Some(format!("Failed to execute command: {}", cmd));
                                        app.full_error_detail = Some(e.to_string());
                                    }
                                }

                            } else {
                                app.error_message = Some("No worktree selected to run command in.".to_string());
                                app.full_error_detail = Some("No worktree selected to run command in.".to_string());
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
                            app.command_output.clear();
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
                            
                            if let Some(Selection::Worktree(p_idx, w_idx)) = app.get_selected_selection() {
                                match app.config.projects[p_idx].worktrees[w_idx].push(commit_msg) {
                                    Ok((add_out, commit_out, push_out)) => {
                                        let mut output = Vec::new();
                                        
                                        // Collect all outputs
                                        if !add_out.stdout.is_empty() {
                                            output.extend(String::from_utf8_lossy(&add_out.stdout).lines().map(String::from));
                                        }
                                        if !commit_out.stdout.is_empty() {
                                            output.extend(String::from_utf8_lossy(&commit_out.stdout).lines().map(String::from));
                                        }
                                        if !push_out.stdout.is_empty() {
                                            output.extend(String::from_utf8_lossy(&push_out.stdout).lines().map(String::from));
                                        }
                                        
                                        // Collect all errors
                                        if !add_out.stderr.is_empty() {
                                            output.extend(String::from_utf8_lossy(&add_out.stderr).lines().map(String::from));
                                        }
                                        if !commit_out.stderr.is_empty() {
                                            output.extend(String::from_utf8_lossy(&commit_out.stderr).lines().map(String::from));
                                        }
                                        if !push_out.stderr.is_empty() {
                                            output.extend(String::from_utf8_lossy(&push_out.stderr).lines().map(String::from));
                                        }
                                        
                                        app.command_output = output;
                                        
                                        // Check if push succeeded
                                        if !push_out.status.success() {
                                            app.error_message = Some("Push failed".to_string());
                                            app.full_error_detail = Some(app.command_output.join("\n"));
                                        } else {
                                            // Success: prepend success message to output
                                            let mut success_output = vec!["Push successful!".to_string()];
                                            success_output.extend(app.command_output.clone());
                                            app.command_output = success_output;
                                            app.error_message = None;
                                            app.full_error_detail = None;
                                        }
                                    },
                                    Err(e) => {
                                        app.error_message = Some("System error occurred during push".to_string());
                                        app.full_error_detail = Some(e.to_string());
                                        app.command_output.clear();
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
                            app.command_output.clear();
                        }
                        _ => {}
                    },
                }
            }
        }
    }
    Ok(())
}
