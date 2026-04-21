use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

use crate::models::{Config, Repo, Settings};
use crate::state::WorkmanState;

#[derive(serde::Serialize)]
pub struct RepoSuggestion {
    pub path: String,
    pub known: bool,
}

#[tauri::command]
pub fn load_config(state: State<'_, Mutex<WorkmanState>>) -> Result<Config, String> {
    Ok(state.lock().unwrap().config.clone())
}

#[tauri::command]
pub fn update_settings(
    settings: Settings,
    state: State<'_, Mutex<WorkmanState>>,
) -> Result<Config, String> {
    let mut state = state.lock().unwrap();
    state.config.settings = settings;
    state.config.save().map_err(|e| e.to_string())?;
    Ok(state.config.clone())
}

#[tauri::command]
pub fn get_repo_suggestions(
    query: String,
    state: State<'_, Mutex<WorkmanState>>,
) -> Result<Vec<RepoSuggestion>, String> {
    let state = state.lock().unwrap();
    Ok(compute_suggestions(&query, &state.config.repos))
}

pub fn compute_suggestions(query: &str, repos: &[Repo]) -> Vec<RepoSuggestion> {
    let mut results: Vec<RepoSuggestion> = Vec::new();
    let mut known_paths: HashSet<PathBuf> = HashSet::new();

    let is_path_input = query.contains('/');
    let q = query.trim().to_lowercase();

    for repo in repos {
        let matches = if is_path_input || q.is_empty() {
            true
        } else {
            let path_str = repo.path.to_string_lossy().to_lowercase();
            let name_str = repo.name.to_lowercase();
            path_str.contains(&q) || name_str.contains(&q)
        };
        if matches {
            known_paths.insert(repo.path.clone());
            results.push(RepoSuggestion {
                path: repo.path.to_string_lossy().to_string(),
                known: true,
            });
        }
    }

    let input_path = PathBuf::from(if query.is_empty() { "." } else { query });
    let (dir, prefix): (PathBuf, String) = if query.is_empty() || query.ends_with('/') {
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
            if name.starts_with('.') { continue; }
            if prefix.is_empty() || name.starts_with(&prefix) {
                let canonical = fs::canonicalize(&p).unwrap_or_else(|_| p.clone());
                if !known_paths.contains(&canonical) {
                    fs_dirs.push(p);
                }
            }
        }
        fs_dirs.sort();
        for p in fs_dirs {
            results.push(RepoSuggestion {
                path: p.to_string_lossy().to_string(),
                known: false,
            });
        }
    }

    results
}

#[tauri::command]
pub fn validate_repo_path(path: String) -> Result<(), String> {
    Config::validate_repo_path(&PathBuf::from(path)).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_compute_suggestions_filesystem() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path();
        fs::create_dir(path.join("myrepo")).unwrap();
        fs::create_dir(path.join("other")).unwrap();
        std::fs::File::create(path.join("file.txt")).unwrap();

        let query = path.to_str().unwrap().to_string() + "/";
        let results = compute_suggestions(&query, &[]);

        assert!(results.iter().any(|e| e.path.ends_with("myrepo")));
        assert!(results.iter().any(|e| e.path.ends_with("other")));
        // files should not appear
        assert!(!results.iter().any(|e| e.path.ends_with("file.txt")));
    }

    #[test]
    fn test_compute_suggestions_known_promoted() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path();
        fs::create_dir(path.join("newrepo")).unwrap();

        let known_repo = Repo {
            name: "frontend".to_string(),
            path: PathBuf::from("/repos/frontend"),
        };
        let query = path.to_str().unwrap().to_string() + "/";
        let results = compute_suggestions(&query, &[known_repo]);

        let known: Vec<_> = results.iter().filter(|e| e.known).collect();
        assert_eq!(known.len(), 1);
        let first_known = results.iter().position(|e| e.known);
        let first_fs = results.iter().position(|e| !e.known);
        if let (Some(fk), Some(ffs)) = (first_known, first_fs) {
            assert!(fk < ffs, "known entries should precede filesystem entries");
        }
    }
}
