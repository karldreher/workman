use crate::models::Config;
use ratatui::widgets::ListState;
use ratatui::style::{Color, Style};
use std::fs;
use std::path::PathBuf;

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum Selection {
    Project(usize),
    Worktree(usize, usize),
}

#[derive(PartialEq)]
pub enum InputMode {
    Normal,
    AddingProjectPath,
    AddingWorktreeName,
    RunningCommand,
    ViewingDiff,
}

pub struct App {
    pub config: Config,
    pub tree_state: ListState,
    pub input_mode: InputMode,
    pub input: String,
    pub error_message: Option<String>,
    pub full_error_detail: Option<String>,
    pub command_output: Vec<String>,
    pub diff_scroll_offset: usize,
    pub path_completions: Vec<String>,
    pub completion_idx: Option<usize>,
}

impl App {
    pub fn new() -> App {
        let config = Config::load();
        let mut app = App {
            config,
            tree_state: ListState::default(),
            input_mode: InputMode::Normal,
            input: String::new(),
            error_message: None,
            full_error_detail: None,
            command_output: Vec::new(),
            diff_scroll_offset: 0,
            path_completions: Vec::new(),
            completion_idx: None,
        };
        if !app.config.projects.is_empty() {
            app.tree_state.select(Some(0));
        }
        app
    }

    pub fn save_config(&self) {
        let _ = self.config.save();
    }

    pub fn get_tree_items(&self) -> Vec<(String, Selection, Style)> {
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
                let style = if status_str == "clean" {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::Red)
                };
                items.push((
                    format!("{} {} ({})", prefix, wt.name, status_str),
                    Selection::Worktree(p_idx, w_idx),
                    style,
                ));
            }
        }
        items
    }

    pub fn get_selected_selection(&self) -> Option<Selection> {
        let items = self.get_tree_items();
        self.tree_state.selected().and_then(|idx| items.get(idx).map(|item| item.1))
    }

    pub fn update_completions(&mut self) {
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

    pub fn next(&mut self) {
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

    pub fn previous(&mut self) {
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
