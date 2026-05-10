//! Per-workspace toolkit: a list of user-defined commands the UI can run
//! against a workspace folder or its active Claude session.

use crate::discovery::{discover, WorkspaceDiscovery};
use crate::model::{Command, CommandKind};
use crate::sessions::SessionRuntime;
use crate::state::AppState;
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use tauri::{State, Emitter, AppHandle};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
struct ToolkitFile {
    version: u32,
    commands: Vec<Command>,
}

const SCHEMA_VERSION: u32 = 1;

fn toolkits_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("claudedeck")
        .join("toolkits")
}

fn toolkit_path(workspace_id: &Uuid) -> PathBuf {
    toolkits_dir().join(format!("{workspace_id}.json"))
}

/// Default commands seeded into every brand-new workspace. Keep this list
/// short — it's the user's first impression of what the toolkit is for.
pub(crate) fn default_commands() -> Vec<Command> {
    vec![
        Command::new("Show diff", "gitDiff", CommandKind::Shell, "git diff --stat"),
        Command::new(
            "Commit all",
            "gitCommit",
            CommandKind::Prompt,
            "Review the diff and create a single conventional commit summarizing the changes. Run the tests first if available.",
        ),
        Command::new(
            "Create PR",
            "gitPullRequest",
            CommandKind::Prompt,
            "Push the current branch and open a pull request via gh. Title and body should follow conventional commit conventions.",
        ),
        Command::new("Open in Finder", "folderOpen", CommandKind::Shell, "open ."),
    ]
}

fn load_or_seed(workspace_id: &Uuid) -> Vec<Command> {
    let path = toolkit_path(workspace_id);
    if let Ok(bytes) = std::fs::read(&path) {
        if let Ok(file) = serde_json::from_slice::<ToolkitFile>(&bytes) {
            return file.commands;
        }
    }
    default_commands()
}

fn save(workspace_id: &Uuid, commands: &[Command]) -> Result<()> {
    let dir = toolkits_dir();
    std::fs::create_dir_all(&dir).context("create toolkits dir")?;
    let path = toolkit_path(workspace_id);
    let file = ToolkitFile {
        version: SCHEMA_VERSION,
        commands: commands.to_vec(),
    };
    let json = serde_json::to_vec_pretty(&file).context("serialize toolkit")?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json).context("write toolkit tmp")?;
    std::fs::rename(&tmp, &path).context("rename toolkit")?;
    Ok(())
}

// ---------- Tauri commands ----------

#[tauri::command]
pub fn get_toolkit(
    state: State<AppState>,
    workspace_id: String,
) -> Result<Vec<Command>, String> {
    let uuid = Uuid::parse_str(&workspace_id).map_err(|e| e.to_string())?;
    let _ = state.get_workspace(&uuid).ok_or("Workspace not found")?;
    Ok(load_or_seed(&uuid))
}

/// Composite payload returned to the frontend: user-defined custom commands
/// PLUS auto-discovered agents from the workspace's `.claude/` folder PLUS
/// state-aware pipeline signals. All sections live behind one round-trip so
/// the UI can render the panel atomically.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolkitView {
    pub commands: Vec<Command>,
    pub discovery: WorkspaceDiscovery,
}

#[tauri::command]
pub fn get_toolkit_view(
    state: State<AppState>,
    workspace_id: String,
) -> Result<ToolkitView, String> {
    let uuid = Uuid::parse_str(&workspace_id).map_err(|e| e.to_string())?;
    let ws = state.get_workspace(&uuid).ok_or("Workspace not found")?;
    Ok(ToolkitView {
        commands: load_or_seed(&uuid),
        discovery: discover(&ws.path),
    })
}

#[tauri::command]
pub fn add_command(
    state: State<AppState>,
    workspace_id: String,
    name: String,
    icon: String,
    kind: String,
    body: String,
) -> Result<Command, String> {
    let uuid = Uuid::parse_str(&workspace_id).map_err(|e| e.to_string())?;
    let _ = state.get_workspace(&uuid).ok_or("Workspace not found")?;
    let kind = parse_kind(&kind)?;
    let cmd = Command::new(name, icon, kind, body);
    let mut current = load_or_seed(&uuid);
    current.push(cmd.clone());
    save(&uuid, &current).map_err(|e| e.to_string())?;
    Ok(cmd)
}

#[tauri::command]
pub fn remove_command(
    state: State<AppState>,
    workspace_id: String,
    command_id: String,
) -> Result<(), String> {
    let ws_uuid = Uuid::parse_str(&workspace_id).map_err(|e| e.to_string())?;
    let cmd_uuid = Uuid::parse_str(&command_id).map_err(|e| e.to_string())?;
    let _ = state.get_workspace(&ws_uuid).ok_or("Workspace not found")?;
    let mut current = load_or_seed(&ws_uuid);
    current.retain(|c| c.id != cmd_uuid);
    save(&ws_uuid, &current).map_err(|e| e.to_string())?;
    Ok(())
}

/// Run a command. Returns Ok(()) on success; the actual side-effect goes
/// through one of three paths depending on `kind`. We don't surface stdout
/// for shell kinds yet — fire-and-forget keeps the UX simple, and tools
/// that need output (lint, tests) belong as `prompt` so Claude reads them.
#[tauri::command]
pub fn run_command(
    app: AppHandle,
    state: State<AppState>,
    runtime: State<Arc<SessionRuntime>>,
    workspace_id: String,
    command_id: String,
    // When kind == prompt, the frontend tells us which session to write to.
    target_session_id: Option<String>,
) -> Result<(), String> {
    let ws_uuid = Uuid::parse_str(&workspace_id).map_err(|e| e.to_string())?;
    let cmd_uuid = Uuid::parse_str(&command_id).map_err(|e| e.to_string())?;
    let ws = state.get_workspace(&ws_uuid).ok_or("Workspace not found")?;
    let commands = load_or_seed(&ws_uuid);
    let cmd = commands
        .iter()
        .find(|c| c.id == cmd_uuid)
        .ok_or("Command not found")?
        .clone();

    match cmd.kind {
        CommandKind::Shell => run_shell(&ws.path, &cmd.body)
            .map_err(|e| e.to_string())?,
        CommandKind::Url => {
            // Delegate to the opener plugin so we get OS-correct browser handling.
            tauri_plugin_opener::open_url(cmd.body.clone(), None::<&str>)
                .map_err(|e| e.to_string())?;
        }
        CommandKind::Prompt => {
            let session_id = target_session_id
                .ok_or("A prompt command requires an active session")?;
            let sid = Uuid::parse_str(&session_id).map_err(|e| e.to_string())?;
            let mut text = cmd.body.clone();
            if !text.ends_with('\n') {
                text.push('\n');
            }
            runtime
                .pty
                .write(&sid, text.as_bytes())
                .map_err(|e| e.to_string())?;
        }
        CommandKind::Agent => {
            // Agent commands are dispatched by writing a canned line into the
            // active session — Claude's own agent system picks it up via the
            // `@<name>` mention. The frontend MAY pass a task description
            // appended to the body separated by `\n---\n`; we split here.
            let session_id = target_session_id
                .ok_or("An agent command requires an active session")?;
            let sid = Uuid::parse_str(&session_id).map_err(|e| e.to_string())?;
            let (agent, task) = match cmd.body.split_once("\n---\n") {
                Some((a, t)) => (a.trim(), t.trim()),
                None => (cmd.body.trim(), ""),
            };
            let line = if task.is_empty() {
                format!("Use the @{agent} agent.\n")
            } else {
                format!("Use the @{agent} agent to: {task}\n")
            };
            runtime
                .pty
                .write(&sid, line.as_bytes())
                .map_err(|e| e.to_string())?;
        }
    }
    // Fire a "command-ran" event mostly so the UI can flash a confirmation.
    let _ = app.emit("toolkit:ran", cmd.id.to_string());
    Ok(())
}

pub(crate) fn parse_kind(s: &str) -> Result<CommandKind, String> {
    match s.to_ascii_lowercase().as_str() {
        "shell" => Ok(CommandKind::Shell),
        "prompt" => Ok(CommandKind::Prompt),
        "url" => Ok(CommandKind::Url),
        "agent" => Ok(CommandKind::Agent),
        _ => Err(format!("Unknown command kind: {s}")),
    }
}

/// Dispatch an auto-discovered agent without persisting it as a custom
/// command. Used by the "AGENTS" and "PIPELINE" sections of the panel —
/// they're computed each render, not stored.
#[tauri::command]
pub fn dispatch_agent(
    runtime: State<Arc<SessionRuntime>>,
    session_id: String,
    agent: String,
    task: Option<String>,
) -> Result<(), String> {
    let sid = Uuid::parse_str(&session_id).map_err(|e| e.to_string())?;
    let line = match task.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
        Some(t) => format!("Use the @{agent} agent to: {t}\n"),
        None => format!("Use the @{agent} agent.\n"),
    };
    runtime
        .pty
        .write(&sid, line.as_bytes())
        .map_err(|e| e.to_string())
}

/// Spawn a shell command in `cwd`. We run it through the user's shell so
/// expansions (`$HOME`, `~`, glob patterns), aliases, and PATH-mutating
/// version managers (nvm, fvm, asdf) all work as expected.
///
/// Important: `-l -i` is intentional. `-l` (login) sources `.zprofile`,
/// while `-i` (interactive) sources `.zshrc` — and most tools developers
/// use day-to-day (npm via nvm, fvm-managed Flutter, etc.) only register
/// themselves in `.zshrc`. Skipping `-i` was the cause of bogus
/// "command not found" errors on commands that worked fine in Terminal.
///
/// We also capture stderr and surface it to the caller. A toolkit run
/// that fails silently is worse than one that fails loud — the user has
/// no other channel to discover what broke.
fn run_shell(cwd: &str, body: &str) -> Result<()> {
    if !Path::new(cwd).is_dir() {
        return Err(anyhow!("workspace path is not a directory"));
    }
    let shell = pick_shell();
    let output = std::process::Command::new(shell)
        .args(["-l", "-i", "-c", body])
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .context("spawn shell command")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        // Trim noise from interactive zsh (job-control banners, history-load
        // chatter) by keeping just the last meaningful chunk of stderr.
        let cleaned: String = stderr
            .lines()
            .filter(|l| !l.is_empty())
            .filter(|l| !l.contains("zsh:") || !l.contains("no job"))
            .collect::<Vec<_>>()
            .join("\n");
        return Err(anyhow!(
            "command failed (exit {}): {}{}",
            output.status.code().unwrap_or(-1),
            if cleaned.trim().is_empty() {
                stdout.trim().to_string()
            } else {
                cleaned.trim().to_string()
            },
            ""
        ));
    }
    Ok(())
}

#[cfg(unix)]
pub(crate) fn pick_shell() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into())
}
#[cfg(windows)]
pub(crate) fn pick_shell() -> String {
    std::env::var("COMSPEC").unwrap_or_else(|_| "powershell.exe".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_kind_shell_variants() {
        assert!(matches!(parse_kind("shell").unwrap(), CommandKind::Shell));
        assert!(matches!(parse_kind("SHELL").unwrap(), CommandKind::Shell));
        assert!(matches!(parse_kind("Shell").unwrap(), CommandKind::Shell));
    }

    #[test]
    fn parse_kind_other_kinds() {
        assert!(matches!(parse_kind("prompt").unwrap(), CommandKind::Prompt));
        assert!(matches!(parse_kind("url").unwrap(), CommandKind::Url));
        assert!(matches!(parse_kind("agent").unwrap(), CommandKind::Agent));
    }

    #[test]
    fn parse_kind_rejects_unknown() {
        assert!(parse_kind("").is_err());
        assert!(parse_kind("unknown").is_err());
    }

    #[test]
    fn default_commands_contains_expected() {
        let cmds = default_commands();
        assert!(!cmds.is_empty());
        assert!(cmds.iter().any(|c| c.name == "Show diff"));
        assert!(cmds.iter().any(|c| c.name == "Create PR"));
    }

    #[cfg(unix)]
    #[test]
    fn pick_shell_returns_non_empty() {
        let s = pick_shell();
        assert!(!s.is_empty());
    }
}
