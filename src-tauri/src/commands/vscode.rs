use std::sync::Mutex;
use tauri::State;

use crate::state::WorkmanState;

#[tauri::command]
pub fn open_in_vscode(project_name: String, state: State<'_, Mutex<WorkmanState>>) -> Result<(), String> {
    let state = state.lock().unwrap();
    let project = state
        .config
        .projects
        .iter()
        .find(|p| p.name == project_name)
        .ok_or_else(|| format!("Project '{}' not found", project_name))?;

    let folders: Vec<_> = project
        .worktrees
        .iter()
        .map(|wt| serde_json::json!({ "path": wt.path }))
        .collect();

    let workspace_json = serde_json::to_string_pretty(&serde_json::json!({
        "folders": folders,
        "settings": {}
    }))
    .map_err(|e| e.to_string())?;

    let workspace_path = project.folder.join(format!("{}.code-workspace", project.name));

    std::fs::create_dir_all(&project.folder).map_err(|e| e.to_string())?;
    std::fs::write(&workspace_path, workspace_json).map_err(|e| e.to_string())?;

    #[cfg(target_os = "windows")]
    std::process::Command::new("cmd")
        .args(["/C", "code", workspace_path.to_str().unwrap_or("")])
        .spawn()
        .map_err(|e| format!("Failed to launch VS Code: {e}"))?;

    #[cfg(not(target_os = "windows"))]
    std::process::Command::new("code")
        .arg(&workspace_path)
        .spawn()
        .map_err(|e| format!(
            "Failed to launch VS Code — ensure 'code' is in your PATH \
             (VS Code → Command Palette → 'Shell Command: Install code command'). {e}"
        ))?;

    Ok(())
}
