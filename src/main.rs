use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Terminal,
};
use serde::{Deserialize, Serialize};
use std::{fs, io, path::PathBuf, sync::atomic::{AtomicBool, Ordering}, sync::Arc};
// No longer need regex
// use regex::Regex;

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Worktree {
    name: String,
    path: PathBuf,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Project {
    name: String,
    path: PathBuf,
    worktrees: Vec<Worktree>,
}

#[derive(PartialEq)]
enum InputMode {
    Normal,
    AddingProjectPath,
    AddingWorktreeName,
    AddingWorktreeBranch,
    RunningCommand,
    ViewingDiff, // New input mode
}

struct TerminalRestorer;

impl Drop for TerminalRestorer {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let mut stdout = io::stdout();
        let _ = execute!(stdout, LeaveAlternateScreen);
    }
}

impl Worktree {
    fn get_status(&self) -> String {
        if !self.path.exists() {
            return "N/A".to_string(); // Not available if path doesn't exist
        }

        let git_dir_arg = format!("--git-dir={}/.git", self.path.display());
        let work_tree_arg = format!("--work-tree={}", self.path.display());

        // Get diff stats using --numstat
        let diff_numstat_output = std::process::Command::new("git")
            .arg(&git_dir_arg)
            .arg(&work_tree_arg)
            .arg("diff")
            .arg("--numstat")
            .output();
        
        let mut total_insertions = 0;
        let mut total_deletions = 0;
        let mut status_indicators = Vec::new();

        if let Ok(output) = diff_numstat_output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() == 3 {
                    // Ignore binary files which output "-" for added/deleted lines
                    if parts[0] != "-" {
                        if let Ok(added) = parts[0].parse::<i32>() {
                            total_insertions += added;
                        }
                    }
                    if parts[1] != "-" {
                        if let Ok(deleted) = parts[1].parse::<i32>() {
                            total_deletions += deleted;
                        }
                    }
                }
            }
        }
        status_indicators.push(format!("{}/-{}", total_insertions, total_deletions));

        // Check for untracked files using --porcelain=v1 to count '??' lines
        let untracked_status_output = std::process::Command::new("git")
            .arg(&git_dir_arg)
            .arg(&work_tree_arg)
            .arg("status")
            .arg("--porcelain=v1")
            .output();

        if let Ok(output) = untracked_status_output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let untracked_count = stdout.lines().filter(|line| line.starts_with("??")).count();
            if untracked_count > 0 {
                status_indicators.push(format!("U:{}", untracked_count));
            }
        }

        // Check for unpushed commits
        let unpushed_output = std::process::Command::new("git")
            .arg(&git_dir_arg)
            .arg(&work_tree_arg)
            .arg("cherry")
            .arg("-v")
            .output();

        if let Ok(output) = unpushed_output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let unpushed_count = stdout.lines().count();
            if unpushed_count > 0 {
                status_indicators.push(format!("↑{}", unpushed_count));
            }
        }
        
        // Check if all indicators are clean, considering "0/-0" as clean
        if status_indicators.len() == 1 && status_indicators[0] == "0/-0" {
            "clean".to_string()
        } else {
            status_indicators.join(" ")
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
struct Config {
    projects: Vec<Project>,
}

impl Config {
    fn get_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".workman.config")
    }

    fn load() -> Self {
        let path = Self::get_path();
        if path.exists() {
            let content = fs::read_to_string(path).unwrap_or_default();
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    fn save(&self) -> Result<()> {
        let path = Self::get_path();
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    fn validate_project_path(path: &PathBuf) -> Result<()> {
        if !path.exists() {
            return Err(anyhow::anyhow!("Path does not exist: {:?}", path));
        }
        let absolute_path = fs::canonicalize(path)?;
        if !absolute_path.is_dir() {
            return Err(anyhow::anyhow!("Path is not a directory: {:?}", absolute_path));
        }
        let git_dir = absolute_path.join(".git");
        if !git_dir.exists() {
            return Err(anyhow::anyhow!("Path is not a Git repository (no .git folder): {:?}", absolute_path));
        }
        Ok(())
    }
}

#[derive(PartialEq, Debug, Clone, Copy)]
enum Selection {
    Project(usize),
    Worktree(usize, usize),
}

struct App {
    config: Config,
    tree_state: ListState,
    input_mode: InputMode,
    input: String,
    temp_worktree_name: String,
    error_message: Option<String>,
    full_error_detail: Option<String>,
    command_output: Vec<String>,
    diff_scroll_offset: usize, // New field for diff scrolling
    path_completions: Vec<String>,
    completion_idx: Option<usize>,
}

impl App {
    fn new() -> App {
        let config = Config::load();
        let mut app = App {
            config,
            tree_state: ListState::default(),
            input_mode: InputMode::Normal,
            input: String::new(),
            temp_worktree_name: String::new(),
            error_message: None,
            full_error_detail: None,
            command_output: Vec::new(), // Initialize new field
            diff_scroll_offset: 0, // Initialize diff scroll offset
            path_completions: Vec::new(),
            completion_idx: None,
        };
        if !app.config.projects.is_empty() {
            app.tree_state.select(Some(0));
        }
        app
    }

    fn save_config(&self) {
        let _ = self.config.save();
    }

    fn get_tree_items(&self) -> Vec<(String, Selection, Style)> {
        let mut items = Vec::new();
        for (p_idx, project) in self.config.projects.iter().enumerate() {
            items.push((
                project.name.clone(),
                Selection::Project(p_idx),
                Style::default(),
            ));
            let wt_count = project.worktrees.len();
            for (w_idx, wt) in project.worktrees.iter().enumerate() {
                let prefix = if w_idx == wt_count - 1 {
                    "└── "
                } else {
                    "├── "
                };
                let status_str = wt.get_status();
                let style = if status_str == "clean" { Style::default().fg(Color::Green) } else { Style::default().fg(Color::Red) };
                items.push((
                    format!("{} {} ({})", prefix, wt.name, status_str),
                    Selection::Worktree(p_idx, w_idx),
                    style,
                ));
            }
        }
        items
    }

    fn get_selected_selection(&self) -> Option<Selection> {
        let items = self.get_tree_items();
        self.tree_state.selected().and_then(|idx| items.get(idx).map(|item| item.1))
    }

    fn update_completions(&mut self) {
        let input_path = if self.input.is_empty() {
            PathBuf::from(".")
        } else {
            PathBuf::from(&self.input)
        };

        let (dir, prefix) = if self.input.ends_with('/') || self.input.is_empty() {
            (input_path, "")
        } else {
            let p = input_path.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| PathBuf::from("."));
            let f = input_path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            (p, f)
        };

        let mut completions = Vec::new();
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.starts_with(prefix) {
                    let p = entry.path();
                    let mut s = p.to_string_lossy().to_string();
                    if p.is_dir() && !s.ends_with('/') {
                        s.push('/');
                    }
                    completions.push(s);
                }
            }
        }
        completions.sort();
        self.path_completions = completions;
        self.completion_idx = None;
    }

    fn next(&mut self) {
        let items = self.get_tree_items();
        if items.is_empty() { return; }
        let i = match self.tree_state.selected() {
            Some(i) => {
                if i >= items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.tree_state.select(Some(i));
    }

    fn previous(&mut self) {
        let items = self.get_tree_items();
        if items.is_empty() { return; }
        let i = match self.tree_state.selected() {
            Some(i) => {
                if i == 0 {
                    items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.tree_state.select(Some(i));
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
                            app.full_error_detail = None; // Clear full error detail
                            app.command_output.clear(); // Clear command output when starting new input
                        }
                        KeyCode::Char('x') => { // Changed from 'd' to 'x'
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
                            app.error_message = None; // Clear error messages after action
                            app.full_error_detail = None; // Clear full error detail
                            app.command_output.clear(); // Clear command output after action
                        }
                        KeyCode::Char('w') => {
                            if let Some(Selection::Project(_)) = app.get_selected_selection() {
                                app.input_mode = InputMode::AddingWorktreeName;
                                app.input.clear();
                                app.error_message = None;
                                app.full_error_detail = None; // Clear full error detail
                                app.command_output.clear(); // Clear command output when starting new input
                            }
                        }
                        KeyCode::Char('r') => {
                            if let Some(Selection::Worktree(p_idx, w_idx)) = app.get_selected_selection() {
                                let wt = &app.config.projects[p_idx].worktrees[w_idx];
                                let output = std::process::Command::new("git")
                                    .arg("-C")
                                    .arg(&app.config.projects[p_idx].path)
                                    .arg("worktree")
                                    .arg("remove")
                                    .arg(&wt.name)
                                    .output();
                                
                                match output {
                                    Ok(out) => {
                                        app.command_output = String::from_utf8_lossy(&out.stdout).lines().map(String::from).collect();
                                        if !out.stderr.is_empty() {
                                            app.command_output.extend(String::from_utf8_lossy(&out.stderr).lines().map(String::from));
                                        }
                                        if out.status.success() {
                                            app.config.projects[p_idx].worktrees.remove(w_idx);
                                            app.save_config();
                                            app.error_message = None; // Clear error messages on success
                                            app.full_error_detail = None; // Clear full error detail
                                            app.command_output.clear(); // Clear command output on success
                                            if app.config.projects[p_idx].worktrees.is_empty() {
                                                app.tree_state.select(None); // Nothing to select in this project
                                            } else {
                                                let new_idx = if w_idx >= app.config.projects[p_idx].worktrees.len() { app.config.projects[p_idx].worktrees.len() - 1 } else { w_idx };
                                                if let Some(_current_selection_idx) = app.tree_state.selected() {
                                                    let items = app.get_tree_items();
                                                    if let Some(new_sel_idx) = items.iter().position(|(_, sel, _)| *sel == Selection::Worktree(p_idx, new_idx)) {
                                                        app.tree_state.select(Some(new_sel_idx));
                                                    } else if let Some(proj_sel_idx) = items.iter().position(|(_, sel, _)| *sel == Selection::Project(p_idx)) {
                                                        app.tree_state.select(Some(proj_sel_idx));
                                                    } else {
                                                        app.tree_state.select(None);
                                                    }
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
                                        app.command_output.clear(); // Clear command output on system error
                                    }
                                }
                            }
                        }
                        KeyCode::Char('c') => {
                            if let Some(Selection::Worktree(_p_idx, _w_idx)) = app.get_selected_selection() {
                                app.input_mode = InputMode::RunningCommand;
                                app.input.clear();
                                app.error_message = None; // Clear error messages
                                app.full_error_detail = None; // Clear full error detail
                                app.command_output.clear(); // Clear previous command output
                            }
                        }
                        KeyCode::Char('p') => {
                            if let Some(Selection::Worktree(p_idx, w_idx)) = app.get_selected_selection() {
                                let wt_path = &app.config.projects[p_idx].worktrees[w_idx].path;
                                let output = std::process::Command::new("git")
                                    .arg("-C")
                                    .arg(wt_path)
                                    .arg("push")
                                    .output();

                                match output {
                                    Ok(out) => {
                                        app.command_output = String::from_utf8_lossy(&out.stdout).lines().map(String::from).collect();
                                        if !out.stderr.is_empty() {
                                            app.command_output.extend(String::from_utf8_lossy(&out.stderr).lines().map(String::from));
                                        }
                                        if !out.status.success() {
                                            app.error_message = Some("Push failed".to_string());
                                            app.full_error_detail = Some(app.command_output.join("\n"));
                                        } else {
                                            app.error_message = None; // Clear any previous error
                                            app.full_error_detail = None; // Clear full error detail
                                            app.command_output.clear(); // Clear command output on success
                                        }
                                    },
                                    Err(e) => {
                                        app.error_message = Some("System error occurred during push".to_string());
                                        app.full_error_detail = Some(e.to_string());
                                        app.command_output.clear(); // Clear command output on system error
                                    }
                                }
                            }
                        }
                        KeyCode::Char('d') => {
                            if let Some(Selection::Worktree(p_idx, w_idx)) = app.get_selected_selection() {
                                let wt_path = &app.config.projects[p_idx].worktrees[w_idx].path;
                                let output = std::process::Command::new("git")
                                    .arg("-C")
                                    .arg(wt_path)
                                    .arg("diff")
                                    .output();
                                
                                match output {
                                    Ok(out) => {
                                        app.command_output = String::from_utf8_lossy(&out.stdout).lines().map(String::from).collect();
                                        if !out.stderr.is_empty() {
                                            app.command_output.extend(String::from_utf8_lossy(&out.stderr).lines().map(String::from));
                                        }
                                        if !out.status.success() {
                                            app.error_message = Some("Failed to get diff".to_string());
                                            app.full_error_detail = Some(app.command_output.join("\n"));
                                            app.input_mode = InputMode::Normal; // Stay in Normal if diff fails
                                            app.diff_scroll_offset = 0; // Reset scroll offset
                                        } else {
                                            if app.command_output.is_empty() {
                                                app.error_message = Some("No changes to display diff for.".to_string());
                                                app.full_error_detail = None;
                                                app.command_output.clear();
                                                app.input_mode = InputMode::Normal;
                                                app.diff_scroll_offset = 0; // Reset scroll offset
                                            } else {
                                                app.input_mode = InputMode::ViewingDiff;
                                                app.error_message = None;
                                                app.full_error_detail = None;
                                                app.diff_scroll_offset = 0; // Reset scroll offset when entering diff view
                                            }
                                        }
                                    },
                                    Err(e) => {
                                        app.error_message = Some("System error occurred while getting diff".to_string());
                                        app.full_error_detail = Some(e.to_string());
                                        app.input_mode = InputMode::Normal;
                                        app.diff_scroll_offset = 0; // Reset scroll offset on error
                                    }
                                }
                            }
                        }
                        KeyCode::Down => app.next(),
                        KeyCode::Up => app.previous(),
                        _ => {}
                    },
                    InputMode::ViewingDiff => match key.code {
                        KeyCode::Char(' ') => {
                            // Implement scrolling here. For simplicity, we'll just clear the output for now.
                            // Real scrolling would require tracking a scroll position and rendering a subset of lines.
                            if app.diff_scroll_offset + 1 < app.command_output.len() {
                                app.diff_scroll_offset += 1;
                            } else {
                                app.diff_scroll_offset = 0; // Wrap around to the beginning
                            }
                        }
                        KeyCode::Esc => {
                            app.input_mode = InputMode::Normal;
                            app.error_message = None;
                            app.full_error_detail = None;
                            app.input.clear();
                            app.command_output.clear();
                            app.diff_scroll_offset = 0; // Reset scroll offset on exit
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
                                    // Select the newly added project
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
                                    app.command_output.clear(); // Clear command output on success
                                }
                                Err(e) => {
                                    app.error_message = Some(e.to_string());
                                    app.full_error_detail = Some(e.to_string()); // Capture full error detail
                                    app.command_output.clear(); // Clear command output on failure
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
                            let name = app.input.drain(..).collect::<String>();
                            app.temp_worktree_name = name.clone();
                            app.input = name; // Default branch to worktree name
                            app.input_mode = InputMode::AddingWorktreeBranch;
                            app.error_message = None;
                            app.full_error_detail = None;
                            app.command_output.clear();
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
                    InputMode::AddingWorktreeBranch => match key.code {
                        KeyCode::Enter => {
                            let branch = app.input.trim().to_string();
                            if branch.is_empty() {
                                app.error_message = Some("Branch name cannot be empty".to_string());
                                app.full_error_detail = Some("Branch name cannot be empty".to_string());
                                app.command_output.clear();
                                return Ok(());
                            }
                            
                            if let Some(Selection::Project(p_idx)) = app.get_selected_selection() { // Ensure a project is selected
                                let p_path = app.config.projects[p_idx].path.clone();
                                let wt_name = app.temp_worktree_name.clone();
                                
                                // 0. Handle .workman/ directory and .gitignore
                                let workman_dir = p_path.join(".workman");
                                if !workman_dir.exists() {
                                    let _ = fs::create_dir_all(&workman_dir);
                                }

                                let gitignore_path = p_path.join(".gitignore");
                                let mut needs_append = true;
                                if let Ok(content) = fs::read_to_string(&gitignore_path) {
                                    if content.lines().any(|l| l.trim() == ".workman/" || l.trim() == ".workman") {
                                        needs_append = false;
                                    }
                                }
                                if needs_append {
                                    use std::io::Write;
                                    if let Ok(mut file) = fs::OpenOptions::new()
                                        .append(true)
                                        .create(true)
                                        .open(&gitignore_path) 
                                    {
                                        let _ = writeln!(file, "\n# workman worktrees\n.workman/");
                                    }
                                }

                                let wt_path = workman_dir.join(&wt_name);

                                // 1. Validate branch name format
                                let valid_format = std::process::Command::new("git")
                                    .arg("-C").arg(&p_path)
                                    .arg("check-ref-format")
                                    .arg("--normalize")
                                    .arg(format!("refs/heads/{}", branch))
                                    .output();

                                match valid_format {
                                    Ok(out) if !out.status.success() => {
                                        let err_output = String::from_utf8_lossy(&out.stderr).trim().to_string();
                                        app.command_output = String::from_utf8_lossy(&out.stdout).lines().map(String::from).collect();
                                        app.command_output.extend(err_output.lines().map(String::from));
                                        app.error_message = Some("Invalid branch name format".to_string());
                                        app.full_error_detail = Some(format!("Git check-ref-format says:\n{}", err_output));
                                        app.input = branch;
                                        return Ok(());
                                    }
                                    Err(e) => {
                                        app.error_message = Some("Failed to run git check-ref-format".to_string());
                                        app.full_error_detail = Some(e.to_string());
                                        app.command_output.clear(); // Clear command output on system error
                                        return Ok(());
                                    }
                                    _ => {}
                                }
                                
                                // 2. Check if branch exists
                                let branch_exists = std::process::Command::new("git")
                                    .arg("-C").arg(&p_path)
                                    .arg("show-ref")
                                    .arg("--verify")
                                    .arg(format!("refs/heads/{}", branch))
                                    .output()
                                    .map(|o| o.status.success())
                                    .unwrap_or(false);

                                let mut cmd = std::process::Command::new("git");
                                cmd.arg("-C").arg(&p_path).arg("worktree").arg("add");
                                
                                if !branch_exists {
                                    cmd.arg("-b").arg(&branch).arg(&wt_path);
                                } else {
                                    cmd.arg(&wt_path).arg(&branch);
                                }
                                
                                let output = cmd.output();
                                
                                match output {
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
                                        app.command_output.clear(); // Clear command output on success
                                        // Select the newly added worktree
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
                                        app.command_output.clear(); // Clear command output on system error
                                        app.input = branch;
                                    }
                                }
                            } else {
                                app.error_message = Some("No project selected to add worktree to.".to_string());
                                app.full_error_detail = Some("No project selected to add worktree to.".to_string());
                                app.command_output.clear();
                            }
                        }
                        KeyCode::Char(c) => {
                            app.input.push(c);
                            app.error_message = None;
                        }
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
                            if let Some(Selection::Worktree(p_idx, w_idx)) = app.get_selected_selection() { // Ensure a worktree is selected
                                let wt_path = app.config.projects[p_idx].worktrees[w_idx].path.clone();
                                
                                disable_raw_mode()?;
                                execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
                                
                                let shell = std::env::var("SHELL").unwrap_or_else(|_| "sh".to_string());
                                let output = std::process::Command::new(shell)
                                    .arg("-c")
                                    .arg(&cmd)
                                    .current_dir(wt_path)
                                    .output();
                                
                                execute!(terminal.backend_mut(), EnterAlternateScreen, EnableMouseCapture)?;
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
                }
            }
        }
    }
    Ok(())
}

fn ui(f: &mut ratatui::Frame, app: &mut App) {
    let main_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage(25), // Thinner column for the tree view
                Constraint::Percentage(75), // Wider column for actions and help
            ]
            .as_ref(),
        )
        .split(f.area());

    // Tree Panel
    let items_with_data = app.get_tree_items();
    let tree_items: Vec<ListItem> = items_with_data
        .iter()
        .map(|(text, _, style)| ListItem::new(text.as_str()).style(*style))
        .collect();

    let tree_title = match app.get_selected_selection() {
        Some(Selection::Project(p_idx)) => format!("Projects & Worktrees: {}", app.config.projects[p_idx].name),
        Some(Selection::Worktree(p_idx, w_idx)) => format!("Projects & Worktrees: {} -> {}", app.config.projects[p_idx].name, app.config.projects[p_idx].worktrees[w_idx].name),
        None => "Projects & Worktrees".to_string(),
    };

    let tree_block = Block::default()
        .borders(Borders::ALL)
        .title(tree_title.as_str())
        .border_style(if app.input_mode == InputMode::Normal { Style::default().fg(Color::Yellow) } else { Style::default() }); // Highlight if in normal mode

    let tree_list = List::new(tree_items)
        .block(tree_block)
        .highlight_style(Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan))
        .highlight_symbol("> ");
    f.render_stateful_widget(tree_list, main_layout[0], &mut app.tree_state);

    // Right Panel: Split into Help Bar and Output/Command Pane
    let right_panel_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Thin help bar (3 lines)
            Constraint::Min(0),    // Large output/command pane
        ].as_ref())
        .split(main_layout[1]);

    // TOP-RIGHT: Help Bar
    let help_block = Block::default()
        .borders(Borders::ALL)
        .title("Help")
        .border_style(Style::default().fg(Color::LightBlue)); // Always active and visible

    let mut help_text_lines = Vec::new();
    match app.input_mode {
        InputMode::Normal => {
            match app.get_selected_selection() {
                Some(Selection::Project(_)) => {
                    help_text_lines.push(" 'a': Add Project, 'x': Del Project, 'w': Add Worktree".to_string());
                },
                Some(Selection::Worktree(_, _)) => {
                    help_text_lines.push(" 'c': Run Cmd, 'p': Push, 'r': Rm Worktree, 'd': Show Diff".to_string());
                },
                None => {
                    help_text_lines.push(" 'a': Add Project".to_string());
                }
            }
            help_text_lines.push(" Arrows: Navigate, 'q': Quit, Ctrl+L: Export log".to_string());
        },
        InputMode::AddingProjectPath => {
            help_text_lines.push(" Enter Project Path (Tab: autocomplete, Esc: cancel)".to_string());
        },
        InputMode::AddingWorktreeName => {
            help_text_lines.push(" Enter Worktree Name (Esc: cancel)".to_string());
        },
        InputMode::AddingWorktreeBranch => {
            help_text_lines.push(" Enter Branch Name (Esc: cancel)".to_string());
        },
        InputMode::RunningCommand => {
            help_text_lines.push(" Enter Command (Esc: cancel)".to_string());
        },
        InputMode::ViewingDiff => {
            help_text_lines.push(" Viewing Diff (Space: scroll, Esc: exit)".to_string());
        },
    }

    let help_paragraph = Paragraph::new(help_text_lines.join("\n"))
        .block(help_block)
        .wrap(ratatui::widgets::Wrap { trim: true });
    f.render_widget(help_paragraph, right_panel_chunks[0]);


    // BOTTOM-RIGHT: Output / Command Pane
    let output_pane_block = Block::default()
        .borders(Borders::ALL)
        .title("Output / Command")
        .border_style(if app.input_mode != InputMode::Normal { Style::default().fg(Color::Yellow) } else { Style::default() }); // Highlight if active input mode

    let mut output_content_lines = Vec::new();

    // Errors always prepend
    if let Some(err) = &app.error_message {
        output_content_lines.push(format!("ERROR: {}", err));
        if let Some(detail) = &app.full_error_detail {
            output_content_lines.push(format!("DETAIL: {}", detail));
        }
    }

    // Command output or diff
    if !app.command_output.is_empty() {
        if app.input_mode == InputMode::ViewingDiff {
            // Apply scrolling for diff output
            let num_display_lines = (right_panel_chunks[1].height as usize) - 2; // Account for borders
            let start_index = app.diff_scroll_offset;
            let end_index = (start_index + num_display_lines).min(app.command_output.len());
            output_content_lines.extend(
                app.command_output[start_index..end_index].iter().cloned()
            );
        } else {
            output_content_lines.extend(app.command_output.iter().cloned());
        }
    }

    // Input prompt and current input if active
    if app.input_mode != InputMode::Normal && app.input_mode != InputMode::ViewingDiff {
        let prompt = match app.input_mode {
            InputMode::AddingProjectPath => "Path> ".to_string(),
            InputMode::AddingWorktreeName => "Name> ".to_string(),
            InputMode::AddingWorktreeBranch => "Branch> ".to_string(),
            InputMode::RunningCommand => "Cmd> ".to_string(),
            _ => "> ".to_string(), // Should not happen for these modes
        };
        output_content_lines.push(format!("{}{}", prompt, app.input));
    } else if app.input_mode == InputMode::Normal && app.input.len() > 0 {
         // Show pending input even in normal mode if something was typed and not submitted/cleared
         output_content_lines.push(format!("> {}", app.input));
    }
    
    let output_paragraph = Paragraph::new(output_content_lines.join("\n"))
        .block(output_pane_block)
        .wrap(ratatui::widgets::Wrap { trim: false }); // Do not trim for diff/command output
    f.render_widget(output_paragraph, right_panel_chunks[1]);
}
