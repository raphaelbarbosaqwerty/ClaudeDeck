use crate::git;
use crate::model::{Workspace, WorkspaceKind};
use crate::state::AppState;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::State;
use uuid::Uuid;

/// On-disk schema. We bump `version` if we ever break the layout, so an old
/// install reading a newer file can refuse cleanly instead of mis-parsing.
#[derive(Debug, Serialize, Deserialize)]
struct WorkspacesFile {
    version: u32,
    workspaces: Vec<Workspace>,
}

const SCHEMA_VERSION: u32 = 1;

pub fn config_path() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("claudedeck").join("workspaces.json")
}

pub fn load(path: &Path) -> Result<Vec<Workspace>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let bytes = std::fs::read(path).context("read workspaces.json")?;
    let file: WorkspacesFile = serde_json::from_slice(&bytes).context("parse workspaces.json")?;
    Ok(file.workspaces)
}

pub fn save(path: &Path, workspaces: &[Workspace]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("create config dir")?;
    }
    let file = WorkspacesFile {
        version: SCHEMA_VERSION,
        workspaces: workspaces.to_vec(),
    };
    let json = serde_json::to_vec_pretty(&file).context("serialize workspaces")?;
    // Atomic-ish write: write to .tmp then rename, so a crash mid-write
    // doesn't leave a half-file that fails to parse on next launch.
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json).context("write workspaces.json.tmp")?;
    std::fs::rename(&tmp, path).context("rename workspaces.json")?;
    Ok(())
}

// ---------- Tauri commands ----------

#[tauri::command]
pub fn list_workspaces(state: State<AppState>) -> Vec<Workspace> {
    state.list_workspaces()
}

#[tauri::command]
pub fn add_workspace(state: State<AppState>, path: String) -> Result<Workspace, String> {
    let path = expand_path(&path);
    if !Path::new(&path).is_dir() {
        return Err(format!("Path is not a directory: {path}"));
    }
    // Reject duplicates on path — same folder twice would just confuse the UI.
    if state.list_workspaces().iter().any(|w| w.path == path) {
        return Err("Workspace already exists for that path".into());
    }

    let name = Path::new(&path)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.clone());

    let branch = git::current_branch(&path);

    let ws = Workspace {
        id: Uuid::new_v4(),
        name,
        path: path.clone(),
        kind: WorkspaceKind::Project,
        parent_id: None,
        branch,
    };

    state.add_workspace(ws.clone());
    save(&state.config_path, &state.list_workspaces()).map_err(|e| e.to_string())?;
    Ok(ws)
}

#[tauri::command]
pub fn remove_workspace(state: State<AppState>, id: String) -> Result<(), String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    state.remove_workspace(&uuid);
    save(&state.config_path, &state.list_workspaces()).map_err(|e| e.to_string())?;
    Ok(())
}

/// List worktrees of a workspace's git repo (excluding the workspace's own path).
/// Empty for non-git workspaces.
#[tauri::command]
pub fn list_worktrees_for(state: State<AppState>, id: String) -> Result<Vec<git::WorktreeEntry>, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let ws = state.get_workspace(&uuid).ok_or("Workspace not found")?;
    let Some(root) = git::repo_root(&ws.path) else {
        return Ok(Vec::new());
    };
    git::list_worktrees(&root).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_worktree(
    state: State<AppState>,
    workspace_id: String,
    name: String,
    base_branch: Option<String>,
) -> Result<Workspace, String> {
    let uuid = Uuid::parse_str(&workspace_id).map_err(|e| e.to_string())?;
    let parent = state.get_workspace(&uuid).ok_or("Workspace not found")?;
    let root = git::repo_root(&parent.path).ok_or("Parent workspace is not a git repo")?;

    let path = git::create_worktree(&root, &name, base_branch.as_deref())
        .map_err(|e| e.to_string())?;

    let ws = Workspace {
        id: Uuid::new_v4(),
        name: name.clone(),
        path,
        kind: WorkspaceKind::Worktree,
        parent_id: Some(parent.id),
        branch: Some(name),
    };
    state.add_workspace(ws.clone());
    save(&state.config_path, &state.list_workspaces()).map_err(|e| e.to_string())?;
    Ok(ws)
}

/// Expand a leading `~` to the user's home dir. We don't shell-expand env vars —
/// users picking a folder via dialog give us absolute paths anyway; this is just
/// for the typed-path entry case.
pub(crate) fn expand_path(p: &str) -> String {
    if let Some(rest) = p.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest).to_string_lossy().into_owned();
        }
    }
    p.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::WorkspaceKind;

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new(label: &str) -> Self {
            let p = std::env::temp_dir()
                .join(format!("claudedeck-test-{}-{}", label, Uuid::new_v4()));
            std::fs::create_dir_all(&p).unwrap();
            Self { path: p }
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    fn sample_workspaces() -> Vec<Workspace> {
        vec![Workspace {
            id: Uuid::new_v4(),
            name: "ws".into(),
            path: "/tmp/ws".into(),
            kind: WorkspaceKind::Project,
            parent_id: None,
            branch: Some("main".into()),
        }]
    }

    #[test]
    fn save_then_load_round_trips() {
        let tmp = TempDir::new("roundtrip");
        let path = tmp.path.join("workspaces.json");
        let ws = sample_workspaces();
        save(&path, &ws).unwrap();
        let loaded = load(&path).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, ws[0].name);
        assert_eq!(loaded[0].path, ws[0].path);
        assert_eq!(loaded[0].id, ws[0].id);
        assert_eq!(loaded[0].branch, ws[0].branch);
    }

    #[test]
    fn load_nonexistent_returns_empty() {
        let tmp = TempDir::new("missing");
        let path = tmp.path.join("does-not-exist.json");
        let loaded = load(&path).unwrap();
        assert!(loaded.is_empty());
    }

    #[test]
    fn load_corrupt_returns_err() {
        let tmp = TempDir::new("corrupt");
        let path = tmp.path.join("workspaces.json");
        std::fs::write(&path, b"{not valid json").unwrap();
        assert!(load(&path).is_err());
    }

    #[test]
    fn load_with_wrong_version_still_parses() {
        // We don't currently validate the version field on read.
        let tmp = TempDir::new("badversion");
        let path = tmp.path.join("workspaces.json");
        let body = serde_json::json!({
            "version": 999,
            "workspaces": []
        });
        std::fs::write(&path, serde_json::to_vec(&body).unwrap()).unwrap();
        let loaded = load(&path).unwrap();
        assert!(loaded.is_empty());
    }

    #[test]
    fn expand_path_tilde() {
        let expanded = expand_path("~/foo");
        let home = dirs::home_dir().unwrap();
        let expected = home.join("foo").to_string_lossy().into_owned();
        assert_eq!(expanded, expected);
    }

    #[test]
    fn expand_path_absolute_unchanged() {
        assert_eq!(expand_path("/abs"), "/abs");
    }
}
