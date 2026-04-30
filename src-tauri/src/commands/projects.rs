use std::sync::Mutex;
use tauri::State;

use crate::models::{Config, Project};
use crate::state::WorkmanState;

pub fn branch_from_name_impl(name: &str) -> String {
    let raw: String = name.trim().to_lowercase().chars()
        .map(|c| if c.is_alphanumeric() || c == '/' || c == '.' { c } else { '-' })
        .collect();
    raw.split('-').filter(|s| !s.is_empty()).collect::<Vec<_>>().join("-")
}

#[tauri::command]
pub fn branch_from_name(name: String) -> String {
    branch_from_name_impl(&name)
}

#[tauri::command]
pub fn create_project(
    name: String,
    state: State<'_, Mutex<WorkmanState>>,
) -> Result<Config, String> {
    let mut state = state.lock().unwrap();
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("Project name cannot be empty.".to_string());
    }
    if state.config.projects.iter().any(|p| p.name == name) {
        return Err(format!("Project '{}' already exists.", name));
    }
    let branch = branch_from_name_impl(&name);
    let folder = Project::make_folder_path(&name);
    let project = Project { name, branch, worktrees: Vec::new(), folder };
    project.create_folder().map_err(|e| e.to_string())?;
    state.config.projects.push(project);
    state.config.save().map_err(|e| e.to_string())?;
    Ok(state.config.clone())
}

#[tauri::command]
pub fn remove_project(
    project_name: String,
    state: State<'_, Mutex<WorkmanState>>,
) -> Result<Config, String> {
    let mut state = state.lock().unwrap();
    let p_idx = state.config.projects.iter().position(|p| p.name == project_name)
        .ok_or_else(|| format!("Project '{}' not found.", project_name))?;

    let project = state.config.projects[p_idx].clone();
    let mut errors: Vec<String> = Vec::new();

    for wt in &project.worktrees {
        if let Some(repo) = state.config.repos.iter().find(|r| r.name == wt.repo_name) {
            if let Err(e) = repo.remove_worktree(&wt.path) {
                errors.push(format!("[{}] {}", wt.repo_name, e));
            }
        }
    }
    if let Err(e) = project.remove_folder() {
        errors.push(format!("remove folder: {}", e));
    }

    let session_prefix = format!("{}/", project_name);
    let keys: Vec<String> = state.sessions.keys()
        .filter(|k| k.starts_with(&session_prefix) || k.as_str() == project_name)
        .cloned()
        .collect();
    for k in keys {
        state.sessions.remove(&k);
    }

    state.config.projects.remove(p_idx);
    state.config.save().map_err(|e| e.to_string())?;

    if errors.is_empty() {
        Ok(state.config.clone())
    } else {
        // Save succeeded; return partial-error message alongside updated config
        Err(errors.join("; "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_branch_from_name() {
        assert_eq!(branch_from_name_impl("My Feature"), "my-feature");
        assert_eq!(branch_from_name_impl("fix: some bug"), "fix-some-bug");
        assert_eq!(branch_from_name_impl("feat/my-feature"), "feat/my-feature");
        assert_eq!(branch_from_name_impl("  hello world  "), "hello-world");
    }
}
