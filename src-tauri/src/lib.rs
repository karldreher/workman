use std::collections::HashMap;
use std::sync::Mutex;

mod commands;
mod models;
mod session;
mod state;

use state::WorkmanState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let (config, _migration_notice) = models::Config::load();

    tauri::Builder::default()
        .manage(Mutex::new(WorkmanState {
            config,
            sessions: HashMap::new(),
        }))
        .invoke_handler(tauri::generate_handler![
            commands::config::load_config,
            commands::config::update_settings,
            commands::config::get_repo_suggestions,
            commands::config::validate_repo_path,
            commands::projects::branch_from_name,
            commands::projects::create_project,
            commands::projects::remove_project,
            commands::worktrees::add_repo_to_project,
            commands::worktrees::remove_worktree,
            commands::worktrees::get_all_statuses,
            commands::git::get_diff,
            commands::git::push_worktree,
            commands::git::push_project,
            commands::pty::open_pty_session,
            commands::pty::close_pty_session,
            commands::pty::write_to_pty,
            commands::pty::resize_pty,
            commands::pty::open_external_terminal,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application")
}
