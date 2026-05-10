use crate::model::{Session, Workspace};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

/// Global application state. Held in Tauri's managed state, accessed from commands.
/// Reads dominate writes (sidebar render, session list polling), so we use an RwLock.
pub struct AppState {
    inner: RwLock<Inner>,
    pub config_path: PathBuf,
}

struct Inner {
    workspaces: HashMap<Uuid, Workspace>,
    sessions: HashMap<Uuid, Session>,
}

impl AppState {
    pub fn new(config_path: PathBuf) -> Self {
        Self {
            inner: RwLock::new(Inner {
                workspaces: HashMap::new(),
                sessions: HashMap::new(),
            }),
            config_path,
        }
    }

    // ---- workspaces ----

    pub fn list_workspaces(&self) -> Vec<Workspace> {
        let inner = self.inner.read();
        let mut v: Vec<Workspace> = inner.workspaces.values().cloned().collect();
        v.sort_by(|a, b| a.name.cmp(&b.name));
        v
    }

    pub fn add_workspace(&self, ws: Workspace) {
        self.inner.write().workspaces.insert(ws.id, ws);
    }

    pub fn remove_workspace(&self, id: &Uuid) -> Option<Workspace> {
        let mut inner = self.inner.write();
        // Cascade: drop any sessions belonging to this workspace.
        inner.sessions.retain(|_, s| s.workspace_id != *id);
        inner.workspaces.remove(id)
    }

    pub fn get_workspace(&self, id: &Uuid) -> Option<Workspace> {
        self.inner.read().workspaces.get(id).cloned()
    }

    pub fn replace_workspaces(&self, items: Vec<Workspace>) {
        let mut inner = self.inner.write();
        inner.workspaces.clear();
        for ws in items {
            inner.workspaces.insert(ws.id, ws);
        }
    }

    // ---- sessions ----

    pub fn list_sessions(&self) -> Vec<Session> {
        self.inner.read().sessions.values().cloned().collect()
    }

    pub fn add_session(&self, s: Session) {
        self.inner.write().sessions.insert(s.id, s);
    }

    pub fn remove_session(&self, id: &Uuid) -> Option<Session> {
        self.inner.write().sessions.remove(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Session, Workspace, WorkspaceKind};

    fn mk_state() -> AppState {
        AppState::new(PathBuf::from("/tmp/claudedeck-test-config.json"))
    }

    fn mk_workspace(name: &str) -> Workspace {
        Workspace {
            id: Uuid::new_v4(),
            name: name.to_string(),
            path: format!("/tmp/{}", name),
            kind: WorkspaceKind::Project,
            parent_id: None,
            branch: None,
        }
    }

    #[test]
    fn new_creates_empty_registry() {
        let s = mk_state();
        assert!(s.list_workspaces().is_empty());
        assert!(s.list_sessions().is_empty());
    }

    #[test]
    fn add_and_list_workspaces_sorted_by_name() {
        let s = mk_state();
        s.add_workspace(mk_workspace("charlie"));
        s.add_workspace(mk_workspace("alpha"));
        s.add_workspace(mk_workspace("bravo"));
        let names: Vec<String> = s.list_workspaces().into_iter().map(|w| w.name).collect();
        assert_eq!(names, vec!["alpha", "bravo", "charlie"]);
    }

    #[test]
    fn get_workspace_returns_inserted() {
        let s = mk_state();
        let ws = mk_workspace("proj");
        let id = ws.id;
        s.add_workspace(ws);
        let got = s.get_workspace(&id).expect("should exist");
        assert_eq!(got.id, id);
        assert_eq!(got.name, "proj");
    }

    #[test]
    fn remove_workspace_returns_some_when_present_none_when_missing() {
        let s = mk_state();
        let ws = mk_workspace("x");
        let id = ws.id;
        s.add_workspace(ws);
        assert!(s.remove_workspace(&id).is_some());
        assert!(s.remove_workspace(&id).is_none());
        assert!(s.remove_workspace(&Uuid::new_v4()).is_none());
    }

    #[test]
    fn remove_workspace_cascades_sessions() {
        let s = mk_state();
        let ws_a = mk_workspace("a");
        let ws_b = mk_workspace("b");
        let a_id = ws_a.id;
        let b_id = ws_b.id;
        s.add_workspace(ws_a);
        s.add_workspace(ws_b);

        s.add_session(Session::new(a_id, "s1".into()));
        s.add_session(Session::new(a_id, "s2".into()));
        s.add_session(Session::new(b_id, "s3".into()));
        assert_eq!(s.list_sessions().len(), 3);

        s.remove_workspace(&a_id).unwrap();
        let remaining = s.list_sessions();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].workspace_id, b_id);
    }

    #[test]
    fn replace_workspaces_clears_and_replaces() {
        let s = mk_state();
        s.add_workspace(mk_workspace("old1"));
        s.add_workspace(mk_workspace("old2"));
        let new_items = vec![mk_workspace("new1"), mk_workspace("new2"), mk_workspace("new3")];
        s.replace_workspaces(new_items);
        let names: Vec<String> = s.list_workspaces().into_iter().map(|w| w.name).collect();
        assert_eq!(names, vec!["new1", "new2", "new3"]);
    }

    #[test]
    fn session_add_list_remove() {
        let s = mk_state();
        let ws_id = Uuid::new_v4();
        let sess = Session::new(ws_id, "one".into());
        let sess_id = sess.id;
        s.add_session(sess);
        assert_eq!(s.list_sessions().len(), 1);

        let removed = s.remove_session(&sess_id).expect("present");
        assert_eq!(removed.id, sess_id);
        assert!(s.list_sessions().is_empty());
        assert!(s.remove_session(&sess_id).is_none());
    }
}
