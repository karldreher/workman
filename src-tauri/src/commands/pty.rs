use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, State};

use crate::session::Session;
use crate::state::WorkmanState;

#[tauri::command]
pub fn open_pty_session(
    session_id: String,
    working_dir: String,
    cols: u16,
    rows: u16,
    app_handle: AppHandle,
    state: State<'_, Mutex<WorkmanState>>,
) -> Result<(), String> {
    let mut state = state.lock().unwrap();
    let session = Session::new(session_id.clone(), PathBuf::from(&working_dir), cols, rows, app_handle)
        .map_err(|e| e.to_string())?;
    state.sessions.insert(session_id, session);
    Ok(())
}

#[tauri::command]
pub fn close_pty_session(
    session_id: String,
    state: State<'_, Mutex<WorkmanState>>,
) -> Result<(), String> {
    state.lock().unwrap().sessions.remove(&session_id);
    Ok(())
}

#[tauri::command]
pub fn write_to_pty(
    session_id: String,
    data: Vec<u8>,
    state: State<'_, Mutex<WorkmanState>>,
) -> Result<(), String> {
    let mut state = state.lock().unwrap();
    let session = state.sessions.get_mut(&session_id)
        .ok_or_else(|| format!("Session '{}' not found.", session_id))?;
    session.write(&data).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn resize_pty(
    session_id: String,
    cols: u16,
    rows: u16,
    state: State<'_, Mutex<WorkmanState>>,
) -> Result<(), String> {
    let mut state = state.lock().unwrap();
    let session = state.sessions.get_mut(&session_id)
        .ok_or_else(|| format!("Session '{}' not found.", session_id))?;
    session.resize(cols, rows).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_external_terminal(path: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .args(["-a", "Terminal", &path])
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    #[cfg(target_os = "linux")]
    {
        for term in &["x-terminal-emulator", "xterm", "gnome-terminal", "konsole", "xfce4-terminal"] {
            if std::process::Command::new(term)
                .arg("--working-directory").arg(&path)
                .spawn().is_ok()
            {
                return Ok(());
            }
        }
        Err("No terminal emulator found".to_string())
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/c", "start", "cmd", "/k", &format!("cd /d {}", path)])
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}
