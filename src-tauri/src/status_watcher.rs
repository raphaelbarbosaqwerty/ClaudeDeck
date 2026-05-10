//! Authoritative session-state watcher.
//!
//! Claude Code writes a per-pid status file at `~/.claude/sessions/<pid>.json`
//! containing `cwd`, `pid`, and what the agent is currently doing. This is far
//! more reliable than parsing the TUI's terminal output (the original
//! DraftFrame approach was racy with redraws). We poll every 1.5s and filter
//! by cwd to attribute each file to the right session.

use crate::model::SessionState;
use anyhow::Result;
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct PidFile {
    cwd: String,
    pid: i32,
    /// Claude Code writes `status` with a small known vocabulary:
    /// "idle"  -> waiting for user input
    /// "busy"  -> actively working (covers thinking + generating)
    /// null    -> session not yet active or already exited
    #[serde(default)]
    status: Option<String>,
    /// `updatedAt` is a unix-millis timestamp Claude rewrites on each state
    /// transition. We use it to pick the freshest pid file when several
    /// match the same cwd (rare, but happens during quick restart).
    #[serde(default)]
    updated_at: Option<i64>,
}

pub struct StatusWatcher {
    stop: Arc<AtomicBool>,
}

impl StatusWatcher {
    /// Spawn a thread polling for `cwd`'s status file. Emits
    /// `session:state:{session_id}` whenever the parsed state changes.
    pub fn spawn(app: AppHandle, session_id: Uuid, cwd: String) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let stop_thread = stop.clone();
        thread::spawn(move || {
            let event = format!("session:state:{}", session_id);
            let mut last: Option<SessionState> = None;
            while !stop_thread.load(Ordering::Relaxed) {
                if let Some(state) = read_state_for_cwd(&cwd) {
                    if last != Some(state) {
                        last = Some(state);
                        let _ = app.emit(&event, state);
                    }
                }
                thread::sleep(Duration::from_millis(1500));
            }
        });
        Self { stop }
    }

    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

fn sessions_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_default().join(".claude/sessions")
}

fn read_state_for_cwd(cwd: &str) -> Option<SessionState> {
    // serde_json::from_slice + camelCase rename — Claude writes `updatedAt`,
    // not `updated_at`. The PidFile struct above relies on this rename.
    fn parse_pid_file(bytes: &[u8]) -> Option<PidFile> {
        let raw: serde_json::Value = serde_json::from_slice(bytes).ok()?;
        Some(PidFile {
            cwd: raw.get("cwd")?.as_str()?.to_owned(),
            pid: raw.get("pid")?.as_i64()? as i32,
            status: raw
                .get("status")
                .and_then(|v| v.as_str())
                .map(str::to_owned),
            updated_at: raw.get("updatedAt").and_then(|v| v.as_i64()),
        })
    }

    let dir = sessions_dir();
    let entries = std::fs::read_dir(&dir).ok()?;
    let mut best: Option<(i64, SessionState)> = None;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(_) => continue,
        };
        let parsed = match parse_pid_file(&bytes) {
            Some(p) => p,
            None => continue,
        };
        if parsed.cwd != cwd {
            continue;
        }
        if !pid_alive(parsed.pid) {
            continue;
        }
        let state = classify(parsed.status.as_deref());
        // Prefer Claude's own `updatedAt` over filesystem mtime — it's
        // updated on every state flip while mtime can lag during writes.
        let key = parsed.updated_at.unwrap_or(0);
        if best.as_ref().map_or(true, |(k, _)| key > *k) {
            best = Some((key, state));
        }
    }
    best.map(|(_, s)| s)
}

#[cfg(unix)]
fn pid_alive(pid: i32) -> bool {
    // signal 0 doesn't actually send anything, just probes whether the
    // pid exists and we have permission to signal it.
    unsafe { libc_kill(pid, 0) == 0 }
}

#[cfg(windows)]
fn pid_alive(_pid: i32) -> bool {
    // Cross-platform pid_alive on Windows requires OpenProcess. For an MVP
    // we treat any matching JSON as alive — Claude tends to clean up its
    // own state files on exit, so the false-positive window is short.
    true
}

#[cfg(unix)]
extern "C" {
    fn kill(pid: i32, sig: i32) -> i32;
}
#[cfg(unix)]
unsafe fn libc_kill(pid: i32, sig: i32) -> i32 {
    kill(pid, sig)
}

/// Map Claude's `status` string to our canonical enum.
/// Observed vocabulary: "idle", "busy", null. We treat anything we don't
/// recognize as Idle so a future Claude version can ship new statuses
/// without us showing a misleading "active" badge.
pub(crate) fn classify(raw: Option<&str>) -> SessionState {
    match raw.unwrap_or("").to_ascii_lowercase().as_str() {
        "busy" | "thinking" | "generating" | "streaming" => SessionState::Generating,
        "waiting" | "input" | "prompt" => SessionState::UserInput,
        "attention" | "ask" | "permission" => SessionState::NeedsAttention,
        _ => SessionState::Idle,
    }
}

/// Helper kept here for parity with DraftFrame's path encoding logic — the
/// JSONL watcher uses the same scheme and benefits from sharing it.
pub fn encode_path_for_claude(path: &str) -> String {
    // macOS/Linux scheme: `/Users/foo/bar` -> `-Users-foo-bar`.
    // Windows scheme isn't documented; we approximate by replacing both
    // `/` and `\` with `-` and stripping the leading drive colon.
    #[allow(unused_mut)]
    let mut s = path.to_string();
    #[cfg(windows)]
    {
        // "C:\Users\foo" -> "C-Users-foo"
        s = s.replace(':', "");
    }
    let mut chars = s.chars();
    let first = chars.next();
    let rest: String = chars.collect();
    let rest = rest.replace('/', "-").replace('\\', "-");
    match first {
        Some('/') | Some('\\') => format!("-{}", rest),
        Some(c) => format!("-{}{}", c, rest),
        None => String::new(),
    }
}

#[derive(Debug)]
pub struct WatcherError(pub String);
impl From<anyhow::Error> for WatcherError {
    fn from(e: anyhow::Error) -> Self {
        Self(e.to_string())
    }
}
impl std::fmt::Display for WatcherError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for WatcherError {}

#[allow(dead_code)]
pub fn ok() -> Result<(), WatcherError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{classify, encode_path_for_claude};
    use crate::model::SessionState;

    #[test]
    fn classify_idle() {
        assert_eq!(classify(Some("idle")), SessionState::Idle);
    }

    #[test]
    fn classify_busy_is_generating() {
        assert_eq!(classify(Some("busy")), SessionState::Generating);
    }

    #[test]
    fn classify_thinking_is_generating() {
        assert_eq!(classify(Some("thinking")), SessionState::Generating);
    }

    #[test]
    fn classify_waiting_is_user_input() {
        assert_eq!(classify(Some("waiting")), SessionState::UserInput);
    }

    #[test]
    fn classify_attention() {
        assert_eq!(classify(Some("attention")), SessionState::NeedsAttention);
    }

    #[test]
    fn classify_none_empty_unknown_is_idle() {
        assert_eq!(classify(None), SessionState::Idle);
        assert_eq!(classify(Some("")), SessionState::Idle);
        assert_eq!(classify(Some("unknown")), SessionState::Idle);
    }

    #[test]
    fn classify_is_case_insensitive() {
        assert_eq!(classify(Some("BUSY")), SessionState::Generating);
        assert_eq!(classify(Some("Busy")), SessionState::Generating);
        assert_eq!(classify(Some("WAITING")), SessionState::UserInput);
    }

    #[test]
    fn encode_absolute_unix_path() {
        assert_eq!(encode_path_for_claude("/Users/foo/bar"), "-Users-foo-bar");
    }

    #[test]
    fn encode_root() {
        assert_eq!(encode_path_for_claude("/"), "-");
    }

    #[test]
    fn encode_empty() {
        assert_eq!(encode_path_for_claude(""), "");
    }

    #[test]
    fn encode_relative_path() {
        // Documents current behavior: leading char gets "-" prepended.
        assert_eq!(encode_path_for_claude("foo/bar"), "-foo-bar");
    }
}
