use std::sync::Mutex;
use tauri::State;

use crate::state::WorkmanState;

#[derive(serde::Serialize)]
pub struct PushResult {
    pub success: bool,
    pub output: String,
}

#[derive(serde::Serialize)]
pub struct WorktreePushResult {
    pub repo_name: String,
    pub success: bool,
    pub detail: String,
}

#[tauri::command]
pub fn get_diff(
    project_name: String,
    repo_name: String,
    state: State<'_, Mutex<WorkmanState>>,
) -> Result<String, String> {
    let state = state.lock().unwrap();
    let project = state.config.projects.iter().find(|p| p.name == project_name)
        .ok_or_else(|| format!("Project '{}' not found.", project_name))?;
    let wt = project.worktrees.iter().find(|w| w.repo_name == repo_name)
        .ok_or_else(|| format!("Repo '{}' not in project.", repo_name))?;
    let out = wt.get_diff().map_err(|e| e.to_string())?;
    let mut full = String::from_utf8_lossy(&out.stdout).to_string();
    full.push_str(&String::from_utf8_lossy(&out.stderr));
    Ok(full)
}

#[tauri::command]
pub fn push_worktree(
    project_name: String,
    repo_name: String,
    commit_message: Option<String>,
    state: State<'_, Mutex<WorkmanState>>,
) -> Result<PushResult, String> {
    let state = state.lock().unwrap();
    let project = state.config.projects.iter().find(|p| p.name == project_name)
        .ok_or_else(|| format!("Project '{}' not found.", project_name))?;
    let wt = project.worktrees.iter().find(|w| w.repo_name == repo_name)
        .ok_or_else(|| format!("Repo '{}' not in project.", repo_name))?;
    let (add_out, commit_out, push_out) = wt.push(commit_message).map_err(|e| e.to_string())?;
    let mut output = String::new();
    for o in [&add_out, &commit_out, &push_out] {
        output.push_str(&String::from_utf8_lossy(&o.stdout));
        output.push_str(&String::from_utf8_lossy(&o.stderr));
    }
    Ok(PushResult { success: push_out.status.success(), output })
}

#[tauri::command]
pub fn push_project(
    project_name: String,
    commit_message: Option<String>,
    state: State<'_, Mutex<WorkmanState>>,
) -> Result<Vec<WorktreePushResult>, String> {
    let state = state.lock().unwrap();
    let project = state.config.projects.iter().find(|p| p.name == project_name)
        .ok_or_else(|| format!("Project '{}' not found.", project_name))?;
    let worktrees = project.worktrees.clone();
    let mut results = Vec::new();

    for wt in &worktrees {
        match wt.push(commit_message.clone()) {
            Ok((_, commit_out, push_out)) => {
                let pushed = push_out.status.success();
                let committed = commit_out.status.success();
                let detail = if !committed {
                    let s = String::from_utf8_lossy(&commit_out.stderr).trim().to_string();
                    if s.contains("nothing to commit") { "nothing to commit".to_string() } else { s }
                } else if !pushed {
                    String::from_utf8_lossy(&push_out.stderr).trim().to_string()
                } else {
                    "pushed".to_string()
                };
                results.push(WorktreePushResult { repo_name: wt.repo_name.clone(), success: pushed, detail });
            }
            Err(e) => results.push(WorktreePushResult {
                repo_name: wt.repo_name.clone(),
                success: false,
                detail: e.to_string(),
            }),
        }
    }
    Ok(results)
}
