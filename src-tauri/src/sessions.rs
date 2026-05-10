//! Session orchestration: ties PTY + status watcher + JSONL watcher into a
//! single lifecycle, and exposes Tauri commands the frontend can call.

use crate::jsonl_watcher::JsonlWatcher;
use crate::model::Session;
use crate::pty::{PtyManager, SpawnCommand};
use crate::state::AppState;
use crate::status_watcher::StatusWatcher;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, State};
use uuid::Uuid;

/// Per-session bag of watchers. The PTY itself lives in PtyManager keyed by id.
struct SessionWatchers {
    status: StatusWatcher,
    jsonl: JsonlWatcher,
}

#[derive(Default)]
pub struct SessionRuntime {
    pub pty: PtyManager,
    watchers: Mutex<HashMap<Uuid, SessionWatchers>>,
}

impl SessionRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    fn attach_watchers(&self, app: AppHandle, session_id: Uuid, cwd: String) {
        let status = StatusWatcher::spawn(app.clone(), session_id, cwd.clone());
        let jsonl = JsonlWatcher::spawn(app, session_id, cwd);
        self.watchers
            .lock()
            .insert(session_id, SessionWatchers { status, jsonl });
    }

    fn detach_watchers(&self, session_id: &Uuid) {
        if let Some(w) = self.watchers.lock().remove(session_id) {
            w.status.stop();
            w.jsonl.stop();
        }
    }
}

#[tauri::command]
pub fn list_sessions(state: State<AppState>) -> Vec<Session> {
    state.list_sessions()
}

#[tauri::command]
pub fn create_session(
    app: AppHandle,
    state: State<AppState>,
    runtime: State<Arc<SessionRuntime>>,
    workspace_id: String,
    cols: u16,
    rows: u16,
    use_shell: Option<bool>,
    resume_id: Option<String>,
    is_aux: Option<bool>,
) -> Result<Session, String> {
    let ws_uuid = Uuid::parse_str(&workspace_id).map_err(|e| e.to_string())?;
    let ws = state.get_workspace(&ws_uuid).ok_or("Workspace not found")?;

    let mut session = Session::new(ws.id, ws.name.clone());
    let aux = is_aux.unwrap_or(false);
    // Aux panes are always shells. We force the flag here so a caller can't
    // accidentally request `is_aux=true` with `use_shell=false` and end up
    // with an aux pane that's somehow running claude — that would defeat
    // the entire purpose of the secondary pane.
    let shell = aux || use_shell.unwrap_or(false);
    let cmd = if shell {
        SpawnCommand::Shell
    } else {
        SpawnCommand::Claude { resume: resume_id }
    };

    runtime
        .pty
        .spawn(app.clone(), session.id, &ws.path, cols, rows, cmd)
        .map_err(|e| e.to_string())?;

    // Aux panes don't need JSONL/status watchers — those track Claude's
    // state and tokens, neither of which apply to a plain shell. Skip
    // them to avoid unnecessary background work.
    if !aux {
        runtime.attach_watchers(app, session.id, ws.path.clone());
    }

    // Stamp default model from preference. Real value will be overwritten by
    // the JSONL watcher's first usage event. Aux panes have no model — we
    // leave the default "sonnet" stamp but no JSONL will ever overwrite it,
    // and the UI doesn't render the model badge on aux sessions anyway.
    session.model = if aux { "shell".into() } else { "sonnet".into() };
    session.is_aux = aux;
    state.add_session(session.clone());
    Ok(session)
}

/// Look up the most recent Claude session id for a workspace's cwd.
/// Returns null if there's no `~/.claude/projects/<encoded-cwd>/` directory
/// or no JSONL file in it. The frontend uses this to decide whether to
/// show "Resume previous" alongside "+ New session".
#[tauri::command]
pub fn latest_claude_session(
    state: State<AppState>,
    workspace_id: String,
) -> Result<Option<ResumableSession>, String> {
    let uuid = Uuid::parse_str(&workspace_id).map_err(|e| e.to_string())?;
    let ws = state.get_workspace(&uuid).ok_or("Workspace not found")?;
    Ok(crate::jsonl_watcher::latest_session_for(&ws.path).map(|(id, modified_at_ms)| {
        ResumableSession { id, modified_at_ms }
    }))
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumableSession {
    pub id: String,
    pub modified_at_ms: i64,
}

#[tauri::command]
pub fn write_pty(
    runtime: State<Arc<SessionRuntime>>,
    session_id: String,
    data: String,
) -> Result<(), String> {
    let id = Uuid::parse_str(&session_id).map_err(|e| e.to_string())?;
    runtime.pty.write(&id, data.as_bytes()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn resize_pty(
    runtime: State<Arc<SessionRuntime>>,
    session_id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let id = Uuid::parse_str(&session_id).map_err(|e| e.to_string())?;
    runtime.pty.resize(&id, cols, rows).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn close_session(
    state: State<AppState>,
    runtime: State<Arc<SessionRuntime>>,
    session_id: String,
) -> Result<(), String> {
    let id = Uuid::parse_str(&session_id).map_err(|e| e.to_string())?;
    runtime.detach_watchers(&id);
    runtime.pty.close(&id).map_err(|e| e.to_string())?;
    state.remove_session(&id);
    Ok(())
}

