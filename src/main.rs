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
    fn get_status(&self) -> bool {
        if !self.path.exists() {
            return false;
        }
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(&self.path)
            .arg("status")
            .arg("--porcelain")
            .output();
        
        let mut clean = true;
        if let Ok(out) = status {
            if !out.stdout.is_empty() {
                clean = false;
            }
        }

        if clean {
            let unpushed = std::process::Command::new("git")
                .arg("-C")
                .arg(&self.path)
                .arg("cherry")
                .arg("-v")
                .output();
            
            if let Ok(out) = unpushed {
                if !out.stdout.is_empty() {
                    clean = false;
                }
            }
        }
        clean
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

struct App {
    config: Config,
    project_list_state: ListState,
    worktree_list_state: ListState,
    selected_project_idx: Option<usize>,
    selected_worktree_idx: Option<usize>,
    active_panel: usize,
    input_mode: InputMode,
    input: String,
    temp_worktree_name: String,
    error_message: Option<String>,
    full_error_detail: Option<String>,
    path_completions: Vec<String>,
    completion_idx: Option<usize>,
}

impl App {
    fn new() -> App {
        let config = Config::load();
        let mut app = App {
            config,
            project_list_state: ListState::default(),
            worktree_list_state: ListState::default(),
            selected_project_idx: None,
            selected_worktree_idx: None,
            active_panel: 0,
            input_mode: InputMode::Normal,
            input: String::new(),
            temp_worktree_name: String::new(),
            error_message: None,
            full_error_detail: None,
            path_completions: Vec::new(),
            completion_idx: None,
        };
        if !app.config.projects.is_empty() {
            app.project_list_state.select(Some(0));
            app.selected_project_idx = Some(0);
        }
        app
    }

    fn save_config(&self) {
        let _ = self.config.save();
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

    fn next_project(&mut self) {
        let i = match self.project_list_state.selected() {
            Some(i) => {
                if i >= self.config.projects.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        if !self.config.projects.is_empty() {
            self.project_list_state.select(Some(i));
            self.selected_project_idx = Some(i);
            self.selected_worktree_idx = None;
            self.worktree_list_state.select(None);
        }
    }

    fn previous_project(&mut self) {
        let i = match self.project_list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.config.projects.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        if !self.config.projects.is_empty() {
            self.project_list_state.select(Some(i));
            self.selected_project_idx = Some(i);
            self.selected_worktree_idx = None;
            self.worktree_list_state.select(None);
        }
    }

    fn next_worktree(&mut self) {
        if let Some(p_idx) = self.selected_project_idx {
            let worktrees = &self.config.projects[p_idx].worktrees;
            if worktrees.is_empty() { return; }
            let i = match self.worktree_list_state.selected() {
                Some(i) => {
                    if i >= worktrees.len() - 1 {
                        0
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            self.worktree_list_state.select(Some(i));
            self.selected_worktree_idx = Some(i);
        }
    }

    fn previous_worktree(&mut self) {
        if let Some(p_idx) = self.selected_project_idx {
            let worktrees = &self.config.projects[p_idx].worktrees;
            if worktrees.is_empty() { return; }
            let i = match self.worktree_list_state.selected() {
                Some(i) => {
                    if i == 0 {
                        worktrees.len() - 1
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            self.worktree_list_state.select(Some(i));
            self.selected_worktree_idx = Some(i);
        }
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
                        }
                        KeyCode::Char('d') => {
                            if let Some(idx) = app.selected_project_idx {
                                app.config.projects.remove(idx);
                                app.save_config();
                                if app.config.projects.is_empty() {
                                    app.selected_project_idx = None;
                                    app.project_list_state.select(None);
                                } else {
                                    let new_idx = if idx >= app.config.projects.len() { app.config.projects.len() - 1 } else { idx };
                                    app.selected_project_idx = Some(new_idx);
                                    app.project_list_state.select(Some(new_idx));
                                }
                            }
                        }
                        KeyCode::Char('w') => {
                            if app.selected_project_idx.is_some() {
                                app.input_mode = InputMode::AddingWorktreeName;
                                app.input.clear();
                                app.error_message = None;
                            }
                        }
                        KeyCode::Char('r') => {
                            if let Some(p_idx) = app.selected_project_idx {
                                if let Some(w_idx) = app.selected_worktree_idx {
                                    let wt = &app.config.projects[p_idx].worktrees[w_idx];
                                    let _ = std::process::Command::new("git")
                                        .arg("-C")
                                        .arg(&app.config.projects[p_idx].path)
                                        .arg("worktree")
                                        .arg("remove")
                                        .arg(&wt.name)
                                        .status();
                                    
                                    app.config.projects[p_idx].worktrees.remove(w_idx);
                                    app.save_config();
                                    if app.config.projects[p_idx].worktrees.is_empty() {
                                        app.selected_worktree_idx = None;
                                        app.worktree_list_state.select(None);
                                    } else {
                                        let new_idx = if w_idx >= app.config.projects[p_idx].worktrees.len() { app.config.projects[p_idx].worktrees.len() - 1 } else { w_idx };
                                        app.selected_worktree_idx = Some(new_idx);
                                        app.worktree_list_state.select(Some(new_idx));
                                    }
                                }
                            }
                        }
                        KeyCode::Tab => {
                            app.active_panel = (app.active_panel + 1) % 3;
                        }
                        KeyCode::BackTab => {
                            app.active_panel = if app.active_panel == 0 { 2 } else { app.active_panel - 1 };
                        }
                        KeyCode::Char('c') => {
                            if app.selected_worktree_idx.is_some() {
                                app.input_mode = InputMode::RunningCommand;
                                app.input.clear();
                            }
                        }
                        KeyCode::Char('p') => {
                            if let Some(p_idx) = app.selected_project_idx {
                                if let Some(w_idx) = app.selected_worktree_idx {
                                    let wt_path = &app.config.projects[p_idx].worktrees[w_idx].path;
                                    let _ = std::process::Command::new("git")
                                        .arg("-C")
                                        .arg(wt_path)
                                        .arg("push")
                                        .status();
                                }
                            }
                        }
                        KeyCode::Down => {
                            if app.active_panel == 0 {
                                app.next_project();
                            } else if app.active_panel == 1 {
                                app.next_worktree();
                            }
                        }
                        KeyCode::Up => {
                            if app.active_panel == 0 {
                                app.previous_project();
                            } else if app.active_panel == 1 {
                                app.previous_worktree();
                            }
                        }
                        KeyCode::Left => {
                            if app.active_panel > 0 {
                                app.active_panel -= 1;
                            }
                        }
                        KeyCode::Right => {
                            if app.active_panel < 2 {
                                app.active_panel += 1;
                            }
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
                                    if app.selected_project_idx.is_none() {
                                        app.selected_project_idx = Some(0);
                                        app.project_list_state.select(Some(0));
                                    }
                                }
                                Err(e) => {
                                    app.error_message = Some(e.to_string());
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
                        KeyCode::Esc => app.input_mode = InputMode::Normal,
                        _ => {}
                    },
                    InputMode::AddingWorktreeName => match key.code {
                        KeyCode::Enter => {
                            let name = app.input.drain(..).collect::<String>();
                            app.temp_worktree_name = name.clone();
                            app.input = name; // Default branch to worktree name
                            app.input_mode = InputMode::AddingWorktreeBranch;
                        }
                        KeyCode::Char(c) => app.input.push(c),
                        KeyCode::Backspace => { app.input.pop(); }
                        KeyCode::Esc => app.input_mode = InputMode::Normal,
                        _ => {}
                    },
                    InputMode::AddingWorktreeBranch => match key.code {
                        KeyCode::Enter => {
                            let branch = app.input.trim().to_string();
                            if branch.is_empty() {
                                app.error_message = Some("Branch name cannot be empty".to_string());
                                return Ok(());
                            }
                            
                            if let Some(p_idx) = app.selected_project_idx {
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
                                        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
                                        app.error_message = Some("Invalid branch name format".to_string());
                                        app.full_error_detail = Some(format!("Git check-ref-format says:\n{}", err));
                                        app.input = branch;
                                        return Ok(());
                                    }
                                    Err(e) => {
                                        app.error_message = Some("Failed to run git check-ref-format".to_string());
                                        app.full_error_detail = Some(e.to_string());
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
                                        app.config.projects[p_idx].worktrees.push(Worktree {
                                            name: wt_name,
                                            path: wt_path,
                                        });
                                        app.save_config();
                                        app.input_mode = InputMode::Normal;
                                        app.error_message = None;
                                        app.full_error_detail = None;
                                        app.input.clear();
                                    }
                                    Ok(out) => {
                                        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
                                        app.error_message = Some("Worktree creation failed (Ctrl+L to export log)".to_string());
                                        app.full_error_detail = Some(err);
                                        app.input = branch;
                                    }
                                    Err(e) => {
                                        app.error_message = Some("System error occurred".to_string());
                                        app.full_error_detail = Some(e.to_string());
                                        app.input = branch;
                                    }
                                }
                            }
                        }
                        KeyCode::Char(c) => {
                            app.input.push(c);
                            app.error_message = None;
                        }
                        KeyCode::Backspace => { app.input.pop(); }
                        KeyCode::Esc => {
                            app.input_mode = InputMode::Normal;
                        }
                        _ => {}
                    },
                    InputMode::RunningCommand => match key.code {
                        KeyCode::Enter => {
                            let cmd = app.input.drain(..).collect::<String>();
                            if let Some(p_idx) = app.selected_project_idx {
                                if let Some(w_idx) = app.selected_worktree_idx {
                                    let wt_path = app.config.projects[p_idx].worktrees[w_idx].path.clone();
                                    
                                    disable_raw_mode()?;
                                    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
                                    
                                    let shell = std::env::var("SHELL").unwrap_or_else(|_| "sh".to_string());
                                    let _ = std::process::Command::new(shell)
                                        .arg("-c")
                                        .arg(&cmd)
                                        .current_dir(wt_path)
                                        .status();
                                    
                                    println!("\nPress Enter to return to workman...");
                                    let mut dummy = String::new();
                                    let _ = std::io::stdin().read_line(&mut dummy);

                                    execute!(terminal.backend_mut(), EnterAlternateScreen, EnableMouseCapture)?;
                                    enable_raw_mode()?;
                                    terminal.clear().map_err(|e| anyhow::anyhow!(e.to_string()))?;
                                }
                            }
                            app.input_mode = InputMode::Normal;
                        }
                        KeyCode::Char(c) => app.input.push(c),
                        KeyCode::Backspace => { app.input.pop(); }
                        KeyCode::Esc => app.input_mode = InputMode::Normal,
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
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(3),
        ].as_ref())
        .split(f.area());

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage(30),
                Constraint::Percentage(30),
                Constraint::Percentage(40),
            ]
            .as_ref(),
        )
        .split(main_layout[0]);

    // Projects Panel
    let projects: Vec<ListItem> = app.config.projects.iter().map(|p| ListItem::new(p.name.as_str())).collect();
    let projects_block = Block::default()
        .borders(Borders::ALL)
        .title("Projects (a: add, d: delete)")
        .border_style(if app.active_panel == 0 { Style::default().fg(Color::Yellow) } else { Style::default() });
    let projects_list = List::new(projects)
        .block(projects_block)
        .highlight_style(Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan))
        .highlight_symbol("> ");
    f.render_stateful_widget(projects_list, chunks[0], &mut app.project_list_state);

    // Worktrees Panel
    let worktrees: Vec<ListItem> = if let Some(p_idx) = app.selected_project_idx {
        app.config.projects[p_idx].worktrees.iter().map(|w| {
            let is_clean = w.get_status();
            let style = if is_clean { Style::default().fg(Color::Green) } else { Style::default().fg(Color::Red) };
            ListItem::new(w.name.as_str()).style(style)
        }).collect()
    } else { vec![] };
    let worktrees_block = Block::default()
        .borders(Borders::ALL)
        .title("Worktrees (w: add, r: remove)")
        .border_style(if app.active_panel == 1 { Style::default().fg(Color::Yellow) } else { Style::default() });
    let worktrees_list = List::new(worktrees)
        .block(worktrees_block)
        .highlight_style(Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan))
        .highlight_symbol("> ");
    f.render_stateful_widget(worktrees_list, chunks[1], &mut app.worktree_list_state);

    // Actions Panel
    let action_title = if let Some(w_idx) = app.selected_worktree_idx {
        if let Some(p_idx) = app.selected_project_idx {
            format!("Actions: {}", app.config.projects[p_idx].worktrees[w_idx].name)
        } else { "Actions".to_string() }
    } else { "Actions".to_string() };

    let actions_block = Block::default()
        .borders(Borders::ALL)
        .title(action_title.as_str())
        .border_style(if app.active_panel == 2 { Style::default().fg(Color::Yellow) } else { Style::default() });
    
    let display_text = if let Some(detail) = &app.full_error_detail {
        format!("ERROR DETAIL:\n\n{}", detail)
    } else {
        "HELP:\n\n'c': Run single command\n'p': Push branch\n'r': Remove worktree".to_string()
    };
    let actions_paragraph = Paragraph::new(display_text)
        .block(actions_block)
        .wrap(ratatui::widgets::Wrap { trim: true });
    f.render_widget(actions_paragraph, chunks[2]);

    // Status Bar
    let input_title = if let Some(err) = &app.error_message { format!("Error: {} (Ctrl+L to export log)", err) } else {
        match app.input_mode {
            InputMode::Normal => "Status (q: quit, arrows: navigate, Tab: switch panel)".to_string(),
            InputMode::AddingProjectPath => "Enter Project Path (Tab to autocomplete):".to_string(),
            InputMode::AddingWorktreeName => "Enter Worktree Name:".to_string(),
            InputMode::AddingWorktreeBranch => "Enter Branch Name (auto-creates if missing):".to_string(),
            InputMode::RunningCommand => "Enter Command:".to_string(),
        }
    };
    let input_block = Block::default().borders(Borders::ALL).title(input_title.as_str()).border_style(if app.error_message.is_some() { Style::default().fg(Color::Red) } else { Style::default() });
    let input_paragraph = Paragraph::new(app.input.as_str()).block(input_block);
    f.render_widget(input_paragraph, main_layout[1]);
}
