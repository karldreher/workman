use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

use crate::models::{Config, ProjectWorktree, Repo};
use crate::state::WorkmanState;

#[tauri::command]
pub fn add_repo_to_project(
    project_name: String,
    repo_path: String,
    state: State<'_, Mutex<WorkmanState>>,
) -> Result<Config, String> {
    let mut state = state.lock().unwrap();
    let path = PathBuf::from(&repo_path);

    Config::validate_repo_path(&path).map_err(|e| e.to_string())?;

    let abs_path = fs::canonicalize(&path)
        .map_err(|e| format!("Cannot resolve path: {}", e))?;
    let name = abs_path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let p_idx = state.config.projects.iter().position(|p| p.name == project_name)
        .ok_or_else(|| format!("Project '{}' not found.", project_name))?;

    if state.config.projects[p_idx].worktrees.iter().any(|wt| wt.repo_name == name) {
        return Err(format!("'{}' is already in this project.", name));
    }

    let repo = if let Some(existing) = state.config.repos.iter().find(|r| r.path == abs_path).cloned() {
        existing
    } else {
        let r = Repo { name: name.clone(), path: abs_path };
        state.config.repos.push(r.clone());
        r
    };

    let branch = state.config.projects[p_idx].branch.clone();
    let (out, wt_path) = repo.add_worktree(&branch).map_err(|e| e.to_string())?;

    if !out.status.success() {
        state.config.save().ok();
        return Err(format!("Worktree error: {}", String::from_utf8_lossy(&out.stderr).trim()));
    }

    let wt = ProjectWorktree { repo_name: repo.name.clone(), path: wt_path };
    let _ = state.config.projects[p_idx].add_symlink(&wt);
    state.config.projects[p_idx].worktrees.push(wt);
    state.config.save().map_err(|e| e.to_string())?;
    Ok(state.config.clone())
}

#[tauri::command]
pub fn remove_worktree(
    project_name: String,
    repo_name: String,
    state: State<'_, Mutex<WorkmanState>>,
) -> Result<Config, String> {
    let mut state = state.lock().unwrap();
    let p_idx = state.config.projects.iter().position(|p| p.name == project_name)
        .ok_or_else(|| format!("Project '{}' not found.", project_name))?;
    let w_idx = state.config.projects[p_idx].worktrees.iter().position(|wt| wt.repo_name == repo_name)
        .ok_or_else(|| format!("Repo '{}' not in project.", repo_name))?;

    let wt = state.config.projects[p_idx].worktrees[w_idx].clone();
    let project_folder = state.config.projects[p_idx].folder.clone();

    let git_result = state.config.repos.iter()
        .find(|r| r.name == wt.repo_name)
        .map(|repo| repo.remove_worktree(&wt.path));

    match git_result {
        Some(Ok(out)) if !out.status.success() && wt.path.exists() => {
            return Err(String::from_utf8_lossy(&out.stderr).to_string());
        }
        Some(Err(e)) => return Err(e.to_string()),
        _ => {}
    }

    let _ = std::fs::remove_file(project_folder.join(&wt.repo_name));
    state.sessions.remove(&format!("{}/{}", project_name, repo_name));
    state.config.projects[p_idx].worktrees.remove(w_idx);
    state.config.save().map_err(|e| e.to_string())?;
    Ok(state.config.clone())
}

#[tauri::command]
pub fn get_all_statuses(
    state: State<'_, Mutex<WorkmanState>>,
) -> Result<HashMap<String, String>, String> {
    let state = state.lock().unwrap();
    let mut map = HashMap::new();
    for project in &state.config.projects {
        for wt in &project.worktrees {
            map.insert(format!("{}/{}", project.name, wt.repo_name), wt.get_status());
        }
    }
    Ok(map)
}
