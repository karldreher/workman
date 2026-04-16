use crate::models::{Config, Repo};
use crate::session::Session;
use ratatui::widgets::ListState;
use ratatui::style::{Color, Modifier, Style};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub enum Selection {
    Project(usize),
    Worktree(usize, usize), // (project_idx, worktree_idx)
    Repo(usize),
    Separator, // visual only — skipped during navigation
}

#[derive(PartialEq)]
pub enum InputMode {
    Normal,
    AddingRepoPath,      // adding a repo to the global pool
    AddingProjectName,   // step 1 of project creation: name
    AddingProjectBranch, // step 2 of project creation: branch
    SelectingRepos,      // step 3: multi-select repos (also used when adding to existing project)
    ViewingDiff,
    EditingCommitMessage,
    Terminal,
    Options,
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
    pub sessions: HashMap<Selection, Session>,
    pub terminal_warning: Option<String>,
    pub worktree_status: HashMap<(usize, usize), String>,
    // Project expand/collapse state
    pub expanded_projects: HashSet<usize>,
    // Project creation state
    pub pending_project_name: String,
    pub pending_project_branch: String,
    // Repo multi-select state (used in SelectingRepos mode)
    pub repo_selection: Vec<bool>, // parallel to config.repos or filtered_repos
    pub repo_cursor: usize,
    // When Some(p_idx): adding worktrees to existing project; when None: creating new project
    pub adding_to_project: Option<usize>,
    // Options overlay cursor
    pub options_cursor: usize,
}

impl App {
    pub fn new() -> App {
        let (config, migration_notice) = Config::load();
        let expanded_projects: HashSet<usize> = (0..config.projects.len()).collect();
        let has_items = !config.repos.is_empty() || !config.projects.is_empty();
        let mut app = App {
            config,
            tree_state: ListState::default(),
            input_mode: InputMode::Normal,
            input: String::new(),
            error_message: migration_notice,
            full_error_detail: None,
            command_output: Vec::new(),
            diff_scroll_offset: 0,
            path_completions: Vec::new(),
            completion_idx: None,
            sessions: HashMap::new(),
            terminal_warning: None,
            worktree_status: HashMap::new(),
            expanded_projects,
            pending_project_name: String::new(),
            pending_project_branch: String::new(),
            repo_selection: Vec::new(),
            repo_cursor: 0,
            adding_to_project: None,
            options_cursor: 0,
        };
        if has_items {
            app.tree_state.select(Some(0));
        }
        app.refresh_worktree_status();
        app
    }

    pub fn refresh_worktree_status(&mut self) {
        self.worktree_status.clear();
        for (p_idx, project) in self.config.projects.iter().enumerate() {
            for (w_idx, wt) in project.worktrees.iter().enumerate() {
                self.worktree_status.insert((p_idx, w_idx), wt.get_status());
            }
        }
    }

    pub fn save_config(&self) {
        let _ = self.config.save();
    }

    /// Returns the repos available for selection in SelectingRepos mode.
    /// When adding to an existing project, filters out repos already in that project.
    pub fn available_repos(&self) -> Vec<(usize, &Repo)> {
        self.config.repos.iter().enumerate().filter(|(_, repo)| {
            if let Some(p_idx) = self.adding_to_project {
                let project = &self.config.projects[p_idx];
                !project.worktrees.iter().any(|wt| wt.repo_name == repo.name)
            } else {
                true
            }
        }).collect()
    }

    /// Builds the flat list of items for the left-panel tree.
    pub fn get_tree_items(&self) -> Vec<(String, Selection, Style)> {
        let mut items = Vec::new();

        for (p_idx, project) in self.config.projects.iter().enumerate() {
            let is_expanded = self.expanded_projects.contains(&p_idx);
            let prefix = if is_expanded { "▼" } else { "▶" };
            items.push((
                format!("{} {}", prefix, project.name),
                Selection::Project(p_idx),
                Style::default().add_modifier(Modifier::BOLD),
            ));

            if is_expanded {
                let wt_count = project.worktrees.len();
                for (w_idx, wt) in project.worktrees.iter().enumerate() {
                    let tree_sym = if w_idx == wt_count - 1 { "└──" } else { "├──" };
                    let status_str = self.worktree_status
                        .get(&(p_idx, w_idx))
                        .map(|s| s.as_str())
                        .unwrap_or("...");
                    let style = if status_str == "clean" {
                        Style::default().fg(Color::Green)
                    } else if status_str == "..." {
                        Style::default().fg(Color::DarkGray)
                    } else {
                        Style::default().fg(Color::Red)
                    };
                    items.push((
                        format!("  {} [{}]  {}  {}", tree_sym, wt.repo_name, project.branch, status_str),
                        Selection::Worktree(p_idx, w_idx),
                        style,
                    ));
                }
            }
        }

        // Repos section
        if !self.config.repos.is_empty() {
            items.push((
                "── Repos ──".to_string(),
                Selection::Separator,
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::DIM),
            ));
            for (r_idx, repo) in self.config.repos.iter().enumerate() {
                items.push((
                    format!("  {} ({})", repo.name, repo.path.display()),
                    Selection::Repo(r_idx),
                    Style::default().fg(Color::DarkGray),
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
        if items.is_empty() {
            return;
        }
        let mut i = match self.tree_state.selected() {
            Some(i) => {
                if i >= items.len() - 1 { 0 } else { i + 1 }
            }
            None => 0,
        };
        // Skip over non-navigable separator items
        let mut guard = 0;
        while items.get(i).map(|item| item.1 == Selection::Separator).unwrap_or(false) {
            i = if i >= items.len() - 1 { 0 } else { i + 1 };
            guard += 1;
            if guard > items.len() {
                break;
            }
        }
        self.tree_state.select(Some(i));
        self.error_message = None;
        self.full_error_detail = None;
    }

    pub fn previous(&mut self) {
        let items = self.get_tree_items();
        if items.is_empty() {
            return;
        }
        let mut i = match self.tree_state.selected() {
            Some(i) => {
                if i == 0 { items.len() - 1 } else { i - 1 }
            }
            None => 0,
        };
        // Skip over non-navigable separator items
        let mut guard = 0;
        while items.get(i).map(|item| item.1 == Selection::Separator).unwrap_or(false) {
            i = if i == 0 { items.len() - 1 } else { i - 1 };
            guard += 1;
            if guard > items.len() {
                break;
            }
        }
        self.tree_state.select(Some(i));
        self.error_message = None;
        self.full_error_detail = None;
    }

    /// Toggles expand/collapse for a project.
    pub fn toggle_project_expand(&mut self, p_idx: usize) {
        if self.expanded_projects.contains(&p_idx) {
            self.expanded_projects.remove(&p_idx);
        } else {
            self.expanded_projects.insert(p_idx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Project, ProjectWorktree};
    use std::path::PathBuf;

    fn make_test_app() -> App {
        App {
            config: Config::default(),
            tree_state: ListState::default(),
            input_mode: InputMode::Normal,
            input: String::new(),
            error_message: None,
            full_error_detail: None,
            command_output: Vec::new(),
            diff_scroll_offset: 0,
            path_completions: Vec::new(),
            completion_idx: None,
            sessions: HashMap::new(),
            terminal_warning: None,
            worktree_status: HashMap::new(),
            expanded_projects: HashSet::new(),
            pending_project_name: String::new(),
            pending_project_branch: String::new(),
            repo_selection: Vec::new(),
            repo_cursor: 0,
            adding_to_project: None,
            options_cursor: 0,
        }
    }

    #[test]
    fn test_app_navigation() {
        let mut app = make_test_app();

        app.config.projects.push(Project {
            name: "p1".to_string(),
            branch: "feat/p1".to_string(),
            folder: PathBuf::from("/tmp/.workman/projects/p1"),
            worktrees: vec![
                ProjectWorktree { repo_name: "repo1".to_string(), path: PathBuf::from("/p1/wt") },
            ],
        });
        app.config.projects.push(Project {
            name: "p2".to_string(),
            branch: "feat/p2".to_string(),
            folder: PathBuf::from("/tmp/.workman/projects/p2"),
            worktrees: vec![],
        });
        app.expanded_projects.insert(0);
        app.expanded_projects.insert(1);

        app.tree_state.select(Some(0));
        let items = app.get_tree_items();
        // p1 + p1/wt + p2 = 3 items
        assert_eq!(items.len(), 3);
        assert_eq!(app.get_selected_selection(), Some(Selection::Project(0)));

        app.next();
        assert_eq!(app.get_selected_selection(), Some(Selection::Worktree(0, 0)));

        app.next();
        assert_eq!(app.get_selected_selection(), Some(Selection::Project(1)));

        app.next();
        assert_eq!(app.get_selected_selection(), Some(Selection::Project(0)));

        app.previous();
        assert_eq!(app.get_selected_selection(), Some(Selection::Project(1)));
    }

    #[test]
    fn test_separator_skipped_in_navigation() {
        let mut app = make_test_app();

        app.config.repos.push(Repo {
            name: "myrepo".to_string(),
            path: PathBuf::from("/myrepo"),
        });

        let items = app.get_tree_items();
        // Separator + Repo(0) = 2 items; no projects
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].1, Selection::Separator);
        assert_eq!(items[1].1, Selection::Repo(0));

        // Starting at separator, next should land on Repo
        app.tree_state.select(Some(0));
        app.next();
        // Should skip separator and land on Repo or wrap
        let sel = app.get_selected_selection();
        assert_ne!(sel, Some(Selection::Separator));
    }

    #[test]
    fn test_get_tree_items_with_repos_and_projects() {
        let mut app = make_test_app();

        app.config.repos.push(Repo {
            name: "frontend".to_string(),
            path: PathBuf::from("/frontend"),
        });
        app.config.projects.push(Project {
            name: "my-feature".to_string(),
            branch: "feat/my-feature".to_string(),
            folder: PathBuf::from("/tmp/.workman/projects/my-feature"),
            worktrees: vec![
                ProjectWorktree { repo_name: "frontend".to_string(), path: PathBuf::from("/frontend/.workman/feat-my-feature") },
            ],
        });
        app.expanded_projects.insert(0);

        let items = app.get_tree_items();
        // Project(0) + Worktree(0,0) + Separator + Repo(0) = 4
        assert_eq!(items.len(), 4);
        assert_eq!(items[0].1, Selection::Project(0));
        assert_eq!(items[1].1, Selection::Worktree(0, 0));
        assert_eq!(items[2].1, Selection::Separator);
        assert_eq!(items[3].1, Selection::Repo(0));
        // Worktree label should contain repo name and branch
        assert!(items[1].0.contains("frontend"));
        assert!(items[1].0.contains("feat/my-feature"));
    }

    #[test]
    fn test_toggle_project_expand() {
        let mut app = make_test_app();
        app.config.projects.push(Project {
            name: "p1".to_string(),
            branch: "main".to_string(),
            folder: PathBuf::from("/tmp"),
            worktrees: vec![],
        });

        assert!(!app.expanded_projects.contains(&0));
        app.toggle_project_expand(0);
        assert!(app.expanded_projects.contains(&0));
        app.toggle_project_expand(0);
        assert!(!app.expanded_projects.contains(&0));
    }

    #[test]
    fn test_update_completions() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path();

        fs::create_dir(path.join("dir1")).unwrap();
        std::fs::File::create(path.join("file1.txt")).unwrap();

        let mut app = make_test_app();
        app.input = path.to_str().unwrap().to_string() + "/";
        app.update_completions();

        assert!(app.path_completions.len() >= 2);
        assert!(app.path_completions.iter().any(|c| c.contains("dir1/")));
        assert!(app.path_completions.iter().any(|c| c.contains("file1.txt")));
    }
}
