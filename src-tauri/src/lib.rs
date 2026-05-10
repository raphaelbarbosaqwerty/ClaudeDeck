mod discovery;
mod git;
mod jsonl_watcher;
mod model;
mod pty;
mod sessions;
mod state;
mod status_watcher;
mod toolkit;
mod workspaces;

use sessions::SessionRuntime;
use state::AppState;
use std::sync::Arc;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let config_path = workspaces::config_path();
    let app_state = AppState::new(config_path.clone());

    match workspaces::load(&config_path) {
        Ok(items) => app_state.replace_workspaces(items),
        Err(e) => eprintln!("[claudedeck] failed to load workspaces: {e}"),
    }

    let runtime: Arc<SessionRuntime> = Arc::new(SessionRuntime::new());

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .manage(app_state)
        .manage(runtime)
        .invoke_handler(tauri::generate_handler![
            workspaces::list_workspaces,
            workspaces::add_workspace,
            workspaces::remove_workspace,
            workspaces::list_worktrees_for,
            workspaces::create_worktree,
            sessions::list_sessions,
            sessions::create_session,
            sessions::latest_claude_session,
            sessions::write_pty,
            sessions::resize_pty,
            sessions::close_session,
            toolkit::get_toolkit,
            toolkit::get_toolkit_view,
            toolkit::add_command,
            toolkit::remove_command,
            toolkit::run_command,
            toolkit::dispatch_agent,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
