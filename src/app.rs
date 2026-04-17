use crate::models::Config;
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
}

#[derive(PartialEq)]
pub enum InputMode {
    Normal,
    AddingProjectName,   // step 1 of project creation: name (branch derived automatically)
    AddingRepo,          // path input + fuzzy suggestions for adding a repo to a project
    ViewingDiff,
    EditingCommitMessage,
    Terminal,
    Options,
    Help,
}

/// A single entry in the fuzzy suggestion list shown in AddingRepo mode.
/// Derives a git branch name from a human-readable project name.
pub fn branch_from_name(name: &str) -> String {
    let raw: String = name.trim().to_lowercase().chars()
        .map(|c| if c.is_alphanumeric() || c == '/' || c == '.' { c } else { '-' })
        .collect();
    raw.split('-').filter(|s| !s.is_empty()).collect::<Vec<_>>().join("-")
}

pub struct FuzzyEntry {
    pub path: PathBuf,
    pub known: bool, // true = previously used in another project
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
    pub sessions: HashMap<Selection, Session>,
    pub terminal_warning: Option<String>,
    pub worktree_status: HashMap<(usize, usize), String>,
    // Project expand/collapse state
    pub expanded_projects: HashSet<usize>,
    // Project creation state
    pub pending_project_name: String,
    // Fuzzy repo picker state (AddingRepo mode)
    pub fuzzy_results: Vec<FuzzyEntry>,
    pub fuzzy_cursor: Option<usize>, // None = cursor at text input; Some(i) = suggestion highlighted
    // Which project we are currently adding a repo to
    pub adding_to_project: Option<usize>,
    // Options overlay cursor
    pub options_cursor: usize,
}

impl App {
    pub fn new() -> App {
        let (config, migration_notice) = Config::load();
        let expanded_projects: HashSet<usize> = (0..config.projects.len()).collect();
        let has_items = !config.projects.is_empty();
        let mut app = App {
            config,
            tree_state: ListState::default(),
            input_mode: InputMode::Normal,
            input: String::new(),
            error_message: migration_notice,
            full_error_detail: None,
            command_output: Vec::new(),
            diff_scroll_offset: 0,
            sessions: HashMap::new(),
            terminal_warning: None,
            worktree_status: HashMap::new(),
            expanded_projects,
            pending_project_name: String::new(),
            fuzzy_results: Vec::new(),
            fuzzy_cursor: None,
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

    /// Recomputes fuzzy suggestions from previously used repos + filesystem directories.
    /// Call whenever `self.input` changes while in AddingRepo mode.
    pub fn update_fuzzy_results(&mut self) {
        let query = self.input.trim().to_lowercase();
        let mut results: Vec<FuzzyEntry> = Vec::new();
        let mut known_paths: HashSet<PathBuf> = HashSet::new();

        // 1. Previously used repos promoted to the top.
        //    When the input looks like a filesystem path (contains '/'), show all known repos
        //    so they stay visible while the user browses dirs. Otherwise filter by substring.
        let is_path_input = self.input.contains('/');
        for repo in &self.config.repos {
            // Skip repos already wired into the target project
            if let Some(p_idx) = self.adding_to_project {
                if p_idx < self.config.projects.len()
                    && self.config.projects[p_idx].worktrees.iter().any(|wt| wt.repo_name == repo.name)
                {
                    continue;
                }
            }
            let matches = if is_path_input || query.is_empty() {
                true // always show when navigating filesystem or nothing typed
            } else {
                let path_str = repo.path.to_string_lossy().to_lowercase();
                let name_str = repo.name.to_lowercase();
                path_str.contains(&query) || name_str.contains(&query)
            };
            if matches {
                known_paths.insert(repo.path.clone());
                results.push(FuzzyEntry { path: repo.path.clone(), known: true });
            }
        }

        // 2. Filesystem directories that match the typed prefix
        let input_path = PathBuf::from(if self.input.is_empty() { "." } else { &self.input });
        let (dir, prefix): (PathBuf, String) = if self.input.is_empty() || self.input.ends_with('/') {
            (input_path, String::new())
        } else {
            let p = input_path.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| PathBuf::from("."));
            let f = input_path.file_name().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
            (p, f)
        };

        if let Ok(entries) = fs::read_dir(&dir) {
            let mut fs_dirs: Vec<PathBuf> = Vec::new();
            for entry in entries.flatten() {
                let p = entry.path();
                if !p.is_dir() { continue; }
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
                if name.starts_with('.') { continue; } // skip hidden
                if prefix.is_empty() || name.starts_with(&prefix) {
                    let canonical = fs::canonicalize(&p).unwrap_or_else(|_| p.clone());
                    if !known_paths.contains(&canonical) {
                        fs_dirs.push(p);
                    }
                }
            }
            fs_dirs.sort();
            for p in fs_dirs {
                results.push(FuzzyEntry { path: p, known: false });
            }
        }

        // Cap cursor to valid range
        if let Some(c) = self.fuzzy_cursor {
            if results.is_empty() {
                self.fuzzy_cursor = None;
            } else if c >= results.len() {
                self.fuzzy_cursor = Some(results.len() - 1);
            }
        }
        self.fuzzy_results = results;
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

        items
    }

    pub fn get_selected_selection(&self) -> Option<Selection> {
        let items = self.get_tree_items();
        self.tree_state.selected().and_then(|idx| items.get(idx).map(|item| item.1))
    }

    pub fn next(&mut self) {
        let items = self.get_tree_items();
        if items.is_empty() {
            return;
        }
        let i = match self.tree_state.selected() {
            Some(i) => if i >= items.len() - 1 { 0 } else { i + 1 },
            None => 0,
        };
        self.tree_state.select(Some(i));
        self.error_message = None;
        self.full_error_detail = None;
    }

    pub fn previous(&mut self) {
        let items = self.get_tree_items();
        if items.is_empty() {
            return;
        }
        let i = match self.tree_state.selected() {
            Some(i) => if i == 0 { items.len() - 1 } else { i - 1 },
            None => 0,
        };
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
    use crate::models::{Project, ProjectWorktree, Repo};
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
            sessions: HashMap::new(),
            terminal_warning: None,
            worktree_status: HashMap::new(),
            expanded_projects: HashSet::new(),
            pending_project_name: String::new(),
            fuzzy_results: Vec::new(),
            fuzzy_cursor: None,
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
    fn test_get_tree_items_with_repos_and_projects() {
        let mut app = make_test_app();

        // Repos are in the background cache but not rendered in the tree
        app.config.repos.push(Repo { name: "frontend".to_string(), path: PathBuf::from("/frontend") });
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
        // Project(0) + Worktree(0,0) = 2 items (repos are not shown in main tree)
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].1, Selection::Project(0));
        assert_eq!(items[1].1, Selection::Worktree(0, 0));
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
    fn test_update_fuzzy_results_filesystem() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path();

        fs::create_dir(path.join("myrepo")).unwrap();
        fs::create_dir(path.join("other")).unwrap();
        std::fs::File::create(path.join("file.txt")).unwrap(); // files excluded

        let mut app = make_test_app();
        app.input = path.to_str().unwrap().to_string() + "/";
        app.update_fuzzy_results();

        // Both dirs should appear; the file should not
        assert_eq!(app.fuzzy_results.iter().filter(|e| !e.known).count(), 2);
        assert!(app.fuzzy_results.iter().any(|e| e.path.ends_with("myrepo")));
        assert!(app.fuzzy_results.iter().any(|e| e.path.ends_with("other")));
    }

    #[test]
    fn test_update_fuzzy_results_known_promoted() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path();
        fs::create_dir(path.join("newrepo")).unwrap();

        let mut app = make_test_app();
        // Add a known repo
        app.config.repos.push(Repo {
            name: "frontend".to_string(),
            path: PathBuf::from("/repos/frontend"),
        });
        app.input = path.to_str().unwrap().to_string() + "/";
        app.update_fuzzy_results();

        // Known repo should appear first (even if path differs from typed prefix)
        let known: Vec<_> = app.fuzzy_results.iter().filter(|e| e.known).collect();
        assert_eq!(known.len(), 1);
        assert_eq!(known[0].path, PathBuf::from("/repos/frontend"));
        // known entries precede filesystem entries in results
        let first_fs = app.fuzzy_results.iter().position(|e| !e.known);
        let first_known = app.fuzzy_results.iter().position(|e| e.known);
        if let (Some(fk), Some(ffs)) = (first_known, first_fs) {
            assert!(fk < ffs, "known entries should precede filesystem entries");
        }
    }
}
