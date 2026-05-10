use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A workspace: a folder where one or more Claude sessions can run.
/// Can be a regular project folder OR a git worktree of another workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    pub id: Uuid,
    pub name: String,
    pub path: String,
    pub kind: WorkspaceKind,
    /// When kind == Worktree, this is the parent project's workspace id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<Uuid>,
    /// Git branch checked out at `path` (None for non-git folders).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WorkspaceKind {
    Project,
    Worktree,
}

/// Live state of a running Claude Code session.
/// Mirrors DraftFrame's SessionState — sourced from
/// `~/.claude/sessions/<pid>.json` once we wire the watcher.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SessionState {
    Idle,
    Thinking,
    Generating,
    UserInput,
    NeedsAttention,
}

impl Default for SessionState {
    fn default() -> Self {
        SessionState::Idle
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub name: String,
    pub state: SessionState,
    pub model: String,
    pub cost: f64,
    pub tokens_in: u64,
    pub tokens_out: u64,
    pub context_tokens: u64,
    pub max_context_tokens: u64,
    /// Total subagents dispatched in this session (lifetime count).
    pub subagents_total: u32,
    /// Subagents currently running (no tool_result yet).
    pub subagents_active: u32,
}

impl Session {
    pub fn new(workspace_id: Uuid, name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            workspace_id,
            name,
            state: SessionState::Idle,
            model: "sonnet".into(),
            cost: 0.0,
            tokens_in: 0,
            tokens_out: 0,
            context_tokens: 0,
            max_context_tokens: 200_000,
            subagents_total: 0,
            subagents_active: 0,
        }
    }
}

/// A user-defined action that runs against a workspace or its active Claude
/// session. Three execution modes cover most real workflows:
///   - `Shell`: spawn a process in the workspace cwd, output goes nowhere
///     fancy — fire-and-forget for things like "Open in Xcode".
///   - `Prompt`: write the body text into the active session's PTY,
///     equivalent to the user typing it. Great for canned prompts.
///   - `Url`: hand off to the system browser via tauri-plugin-opener.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Command {
    pub id: Uuid,
    pub name: String,
    /// Single-grapheme icon (emoji preferred). Rendered as text.
    pub icon: String,
    pub kind: CommandKind,
    pub body: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CommandKind {
    Shell,
    Prompt,
    Url,
    /// Dispatch a `.claude/agents/<name>` agent. `body` is the agent name.
    /// When run, the UI optionally asks the user for a task description and
    /// writes `Use the @<name> agent to: <task>` into the active session.
    Agent,
}

impl Command {
    pub fn new(name: impl Into<String>, icon: impl Into<String>, kind: CommandKind, body: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            icon: icon.into(),
            kind,
            body: body.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_new_defaults() {
        let ws_id = Uuid::new_v4();
        let s = Session::new(ws_id, "alpha".to_string());
        assert_eq!(s.workspace_id, ws_id);
        assert_eq!(s.name, "alpha");
        assert_eq!(s.state, SessionState::Idle);
        assert_eq!(s.model, "sonnet");
        assert_eq!(s.cost, 0.0);
        assert_eq!(s.tokens_in, 0);
        assert_eq!(s.tokens_out, 0);
        assert_eq!(s.context_tokens, 0);
        assert_eq!(s.max_context_tokens, 200_000);
        assert_eq!(s.subagents_total, 0);
        assert_eq!(s.subagents_active, 0);
    }

    #[test]
    fn session_new_unique_ids() {
        let ws = Uuid::new_v4();
        let a = Session::new(ws, "a".into());
        let b = Session::new(ws, "b".into());
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn command_new_populates_fields() {
        let c = Command::new("Build", "🔨", CommandKind::Shell, "cargo build");
        assert_eq!(c.name, "Build");
        assert_eq!(c.icon, "🔨");
        assert_eq!(c.kind, CommandKind::Shell);
        assert_eq!(c.body, "cargo build");
        // Fresh UUID per call
        let c2 = Command::new("Build", "🔨", CommandKind::Shell, "cargo build");
        assert_ne!(c.id, c2.id);
    }

    #[test]
    fn session_state_default_is_idle() {
        assert_eq!(SessionState::default(), SessionState::Idle);
    }

    #[test]
    fn workspace_kind_serde_lowercase() {
        let v = serde_json::to_value(WorkspaceKind::Project).unwrap();
        assert_eq!(v, serde_json::json!("project"));
        let v2 = serde_json::to_value(WorkspaceKind::Worktree).unwrap();
        assert_eq!(v2, serde_json::json!("worktree"));
        let back: WorkspaceKind = serde_json::from_value(v).unwrap();
        assert_eq!(back, WorkspaceKind::Project);
        let back2: WorkspaceKind = serde_json::from_value(v2).unwrap();
        assert_eq!(back2, WorkspaceKind::Worktree);
    }

    #[test]
    fn command_kind_serde_lowercase() {
        for (k, expected) in [
            (CommandKind::Shell, "shell"),
            (CommandKind::Prompt, "prompt"),
            (CommandKind::Url, "url"),
            (CommandKind::Agent, "agent"),
        ] {
            let v = serde_json::to_value(k).unwrap();
            assert_eq!(v, serde_json::json!(expected));
            let back: CommandKind = serde_json::from_value(v).unwrap();
            assert_eq!(back, k);
        }
    }

    #[test]
    fn session_state_serde_camel_case() {
        for (s, expected) in [
            (SessionState::Idle, "idle"),
            (SessionState::Thinking, "thinking"),
            (SessionState::Generating, "generating"),
            (SessionState::UserInput, "userInput"),
            (SessionState::NeedsAttention, "needsAttention"),
        ] {
            let v = serde_json::to_value(s).unwrap();
            assert_eq!(v, serde_json::json!(expected), "state {:?}", s);
            let back: SessionState = serde_json::from_value(v).unwrap();
            assert_eq!(back, s);
        }
    }

    #[test]
    fn workspace_skips_parent_id_when_none() {
        let ws = Workspace {
            id: Uuid::new_v4(),
            name: "proj".into(),
            path: "/tmp/proj".into(),
            kind: WorkspaceKind::Project,
            parent_id: None,
            branch: None,
        };
        let v = serde_json::to_value(&ws).unwrap();
        let obj = v.as_object().unwrap();
        assert!(!obj.contains_key("parentId"), "parentId should be skipped when None");
        assert!(!obj.contains_key("branch"), "branch should be skipped when None");
        // camelCase confirmation
        assert!(obj.contains_key("id"));
        assert!(obj.contains_key("name"));
        assert!(obj.contains_key("kind"));
    }

    #[test]
    fn workspace_includes_parent_id_when_some() {
        let parent = Uuid::new_v4();
        let ws = Workspace {
            id: Uuid::new_v4(),
            name: "wt".into(),
            path: "/tmp/wt".into(),
            kind: WorkspaceKind::Worktree,
            parent_id: Some(parent),
            branch: Some("feature".into()),
        };
        let v = serde_json::to_value(&ws).unwrap();
        let obj = v.as_object().unwrap();
        assert!(obj.contains_key("parentId"));
        assert!(obj.contains_key("branch"));
        assert_eq!(obj.get("branch").unwrap(), &serde_json::json!("feature"));

        // round-trip
        let back: Workspace = serde_json::from_value(v).unwrap();
        assert_eq!(back.id, ws.id);
        assert_eq!(back.parent_id, Some(parent));
        assert_eq!(back.branch.as_deref(), Some("feature"));
        assert_eq!(back.kind, WorkspaceKind::Worktree);
    }
}
