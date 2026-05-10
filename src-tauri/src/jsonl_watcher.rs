//! Tail Claude Code's per-session JSONL log to extract token usage and cost.
//!
//! Files live at `~/.claude/projects/<encoded-cwd>/<session-id>.jsonl`. Each
//! line is a JSON record describing one event in the session: user message,
//! assistant message, tool call, etc. Assistant messages carry a `usage`
//! block with token counts; we accumulate them and apply local pricing.

use crate::status_watcher::encode_path_for_claude;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageSnapshot {
    pub cost: f64,
    pub tokens_in: u64,
    pub tokens_out: u64,
    pub model: String,
    pub context_tokens: u64,
    /// Total Task/Agent tool_use invocations seen so far in this session.
    /// Useful for "↳ N subagents" hints on the session card. Not deduped —
    /// each dispatch counts, even if the same agent is run multiple times.
    pub subagents_total: u32,
    /// Subagents whose `tool_use` we've seen but no matching `tool_result`
    /// yet — i.e., still running. May go briefly out-of-sync if Claude
    /// emits the result on a later poll, which corrects on next refresh.
    pub subagents_active: u32,
}

pub struct JsonlWatcher {
    stop: Arc<AtomicBool>,
}

impl JsonlWatcher {
    pub fn spawn(app: AppHandle, session_id: Uuid, cwd: String) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let stop_thread = stop.clone();
        thread::spawn(move || {
            let event = format!("session:usage:{}", session_id);
            let mut acc = Accumulator::default();
            let mut current: Option<PathBuf> = None;
            let mut last_size: u64 = 0;

            while !stop_thread.load(Ordering::Relaxed) {
                // Re-resolve newest file — Claude rotates per session id.
                let latest = find_latest_jsonl(&cwd);
                if latest != current {
                    current = latest;
                    last_size = 0;
                    acc = Accumulator::default();
                }
                if let Some(path) = current.as_ref() {
                    if let Ok(bytes) = std::fs::read(path) {
                        if (bytes.len() as u64) > last_size {
                            // Process only the appended slice, then update offset.
                            let slice = &bytes[last_size as usize..];
                            for line in slice.split(|b| *b == b'\n') {
                                if line.is_empty() {
                                    continue;
                                }
                                acc.feed_line(line);
                            }
                            last_size = bytes.len() as u64;
                            let _ = app.emit(&event, acc.snapshot());
                        }
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

fn projects_dir_for(cwd: &str) -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".claude/projects")
        .join(encode_path_for_claude(cwd))
}

fn find_latest_jsonl(cwd: &str) -> Option<PathBuf> {
    let dir = projects_dir_for(cwd);
    let entries = std::fs::read_dir(&dir).ok()?;
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().and_then(|s| s.to_str()) != Some("jsonl") {
            continue;
        }
        let mtime = e
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(std::time::UNIX_EPOCH);
        if best.as_ref().map_or(true, |(t, _)| mtime > *t) {
            best = Some((mtime, p));
        }
    }
    best.map(|(_, p)| p)
}

/// Public helper for the "Resume" feature: returns the most recently modified
/// Claude session id for a given cwd, plus its modified-at as unix millis.
/// The session id is the JSONL file's stem (`<uuid>.jsonl` -> `<uuid>`).
pub fn latest_session_for(cwd: &str) -> Option<(String, i64)> {
    let path = find_latest_jsonl(cwd)?;
    let id = path.file_stem()?.to_string_lossy().into_owned();
    let mtime = std::fs::metadata(&path).ok()?.modified().ok()?;
    let millis = mtime
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_millis() as i64;
    Some((id, millis))
}

#[derive(Default)]
pub(crate) struct Accumulator {
    cost: f64,
    tokens_in: u64,
    tokens_out: u64,
    model: String,
    last_context_tokens: u64,
    subagents_total: u32,
    /// Maps `tool_use_id` -> seen-result. We track ids so that a duplicate
    /// `tool_use` block (Claude does emit those on retries) doesn't
    /// double-count. Active = entries without a recorded result.
    subagent_use_ids: std::collections::HashSet<String>,
    subagent_done_ids: std::collections::HashSet<String>,
}

impl Accumulator {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn snapshot_for_test(&self) -> UsageSnapshot {
        self.snapshot()
    }

    fn snapshot(&self) -> UsageSnapshot {
        let active = self
            .subagent_use_ids
            .difference(&self.subagent_done_ids)
            .count() as u32;
        UsageSnapshot {
            cost: self.cost,
            tokens_in: self.tokens_in,
            tokens_out: self.tokens_out,
            model: if self.model.is_empty() {
                "sonnet".into()
            } else {
                self.model.clone()
            },
            context_tokens: self.last_context_tokens,
            subagents_total: self.subagents_total,
            subagents_active: active,
        }
    }

    pub(crate) fn feed_line(&mut self, line: &[u8]) {
        // We only care about the assistant turns that carry usage. Other
        // line shapes (user, tool_use, summary) are skipped silently.
        let parsed: serde_json::Value = match serde_json::from_slice(line) {
            Ok(v) => v,
            Err(_) => return,
        };

        // Subagent tracking: scan content arrays of assistant turns for
        // `tool_use` blocks where name is "Task" or "Agent" (Claude's
        // subagent-dispatching tool). Mirror that against `tool_result`
        // blocks (in user turns) referencing those tool_use_ids.
        if let Some(content) = content_array(&parsed) {
            for block in content {
                let Some(btype) = block.get("type").and_then(|v| v.as_str()) else {
                    continue;
                };
                match btype {
                    "tool_use" => {
                        let name = block.get("name").and_then(|v| v.as_str()).unwrap_or("");
                        if name == "Task" || name == "Agent" {
                            if let Some(id) = block.get("id").and_then(|v| v.as_str()) {
                                if self.subagent_use_ids.insert(id.to_string()) {
                                    self.subagents_total = self.subagents_total.saturating_add(1);
                                }
                            }
                        }
                    }
                    "tool_result" => {
                        if let Some(id) =
                            block.get("tool_use_id").and_then(|v| v.as_str())
                        {
                            // Mark done only if the corresponding use_id is one
                            // of ours (subagent). Random tool_results don't count.
                            if self.subagent_use_ids.contains(id) {
                                self.subagent_done_ids.insert(id.to_string());
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        let (model, usage) = if let Some(msg) = parsed.get("message") {
            (
                msg.get("model").and_then(|v| v.as_str()).map(str::to_owned),
                msg.get("usage").cloned(),
            )
        } else {
            (
                parsed.get("model").and_then(|v| v.as_str()).map(str::to_owned),
                parsed.get("usage").cloned(),
            )
        };

        let Some(usage) = usage else { return };
        let Some(model) = model else { return };

        let input = usage.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
        let output = usage
            .get("output_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let cache_creation = usage
            .get("cache_creation_input_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let cache_read = usage
            .get("cache_read_input_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let pricing = pricing_for(&model);
        self.cost += pricing.input * input as f64
            + pricing.output * output as f64
            + pricing.cache_creation * cache_creation as f64
            + pricing.cache_read * cache_read as f64;

        self.tokens_in += input + cache_creation + cache_read;
        self.tokens_out += output;
        self.last_context_tokens = input + cache_creation + cache_read;
        self.model = canonical_model(&model);
    }
}

pub(crate) struct Pricing {
    pub(crate) input: f64,
    pub(crate) output: f64,
    pub(crate) cache_creation: f64,
    pub(crate) cache_read: f64,
}

pub(crate) fn pricing_for(model: &str) -> Pricing {
    // Per-token pricing derived from per-million-token rates. Cache creation
    // is 1.25x input; cache read is 0.1x input. Rates match DraftFrame's
    // table for parity until Anthropic publishes a programmatic source.
    let (i, o) = if model.contains("opus") {
        (15.0, 75.0)
    } else if model.contains("haiku") {
        (0.25, 1.25)
    } else {
        // sonnet and unknown models default to sonnet pricing.
        (3.0, 15.0)
    };
    Pricing {
        input: i / 1_000_000.0,
        output: o / 1_000_000.0,
        cache_creation: (i * 1.25) / 1_000_000.0,
        cache_read: (i * 0.1) / 1_000_000.0,
    }
}

pub(crate) fn canonical_model(raw: &str) -> String {
    if raw.contains("opus") {
        "opus".into()
    } else if raw.contains("haiku") {
        "haiku".into()
    } else {
        "sonnet".into()
    }
}

/// Extract the `content` array from either `{type, message:{content:[...]}}`
/// (assistant turns) or `{type:"user", message:{content:[...]}}` (tool_result
/// turns). Returns None when the line doesn't carry a content array.
fn content_array(v: &serde_json::Value) -> Option<&Vec<serde_json::Value>> {
    let msg = v.get("message")?;
    let content = msg.get("content")?;
    content.as_array()
}

#[allow(dead_code)]
pub fn _unused_compile_check() -> HashMap<String, ()> {
    HashMap::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-12;

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < EPS
    }

    #[test]
    fn pricing_opus() {
        let p = pricing_for("claude-opus-4-5");
        assert!(approx(p.input, 15.0 / 1_000_000.0));
        assert!(approx(p.output, 75.0 / 1_000_000.0));
        assert!(approx(p.cache_creation, (15.0 * 1.25) / 1_000_000.0));
        assert!(approx(p.cache_read, (15.0 * 0.1) / 1_000_000.0));
    }

    #[test]
    fn pricing_sonnet() {
        let p = pricing_for("claude-sonnet-4-6");
        assert!(approx(p.input, 3.0 / 1_000_000.0));
        assert!(approx(p.output, 15.0 / 1_000_000.0));
        assert!(approx(p.cache_creation, (3.0 * 1.25) / 1_000_000.0));
        assert!(approx(p.cache_read, (3.0 * 0.1) / 1_000_000.0));
    }

    #[test]
    fn pricing_haiku() {
        let p = pricing_for("claude-haiku-4-5");
        assert!(approx(p.input, 0.25 / 1_000_000.0));
        assert!(approx(p.output, 1.25 / 1_000_000.0));
        assert!(approx(p.cache_creation, (0.25 * 1.25) / 1_000_000.0));
        assert!(approx(p.cache_read, (0.25 * 0.1) / 1_000_000.0));
    }

    #[test]
    fn pricing_unknown_defaults_to_sonnet() {
        let p = pricing_for("unknown-model");
        let s = pricing_for("claude-sonnet-4-6");
        assert!(approx(p.input, s.input));
        assert!(approx(p.output, s.output));
        assert!(approx(p.cache_creation, s.cache_creation));
        assert!(approx(p.cache_read, s.cache_read));
    }

    #[test]
    fn canonical_model_recognizes_families() {
        assert_eq!(canonical_model("claude-opus-4-5"), "opus");
        assert_eq!(canonical_model("claude-sonnet-4-6"), "sonnet");
        assert_eq!(canonical_model("claude-haiku-4-5"), "haiku");
        assert_eq!(canonical_model("something-weird"), "sonnet");
    }

    #[test]
    fn feed_line_garbage_is_ignored() {
        let mut acc = Accumulator::new();
        acc.feed_line(b"this is not json");
        let snap = acc.snapshot_for_test();
        assert_eq!(snap.tokens_in, 0);
        assert_eq!(snap.tokens_out, 0);
        assert!(approx(snap.cost, 0.0));
    }

    #[test]
    fn feed_line_empty_is_ignored() {
        let mut acc = Accumulator::new();
        acc.feed_line(b"");
        let snap = acc.snapshot_for_test();
        assert_eq!(snap.tokens_in, 0);
        assert_eq!(snap.tokens_out, 0);
    }

    #[test]
    fn feed_line_assistant_usage_accumulates() {
        let mut acc = Accumulator::new();
        let line = br#"{"type":"assistant","message":{"model":"claude-sonnet-4-6","usage":{"input_tokens":100,"output_tokens":50,"cache_creation_input_tokens":10,"cache_read_input_tokens":5},"content":[]}}"#;
        acc.feed_line(line);
        let snap = acc.snapshot_for_test();
        assert_eq!(snap.tokens_in, 115);
        assert_eq!(snap.tokens_out, 50);
        assert_eq!(snap.model, "sonnet");
        let p = pricing_for("claude-sonnet-4-6");
        let expected =
            p.input * 100.0 + p.output * 50.0 + p.cache_creation * 10.0 + p.cache_read * 5.0;
        assert!(approx(snap.cost, expected), "cost {} vs {}", snap.cost, expected);
    }

    #[test]
    fn feed_line_two_assistant_turns_accumulate() {
        let mut acc = Accumulator::new();
        let line = br#"{"type":"assistant","message":{"model":"claude-sonnet-4-6","usage":{"input_tokens":100,"output_tokens":50,"cache_creation_input_tokens":10,"cache_read_input_tokens":5},"content":[]}}"#;
        acc.feed_line(line);
        acc.feed_line(line);
        let snap = acc.snapshot_for_test();
        assert_eq!(snap.tokens_in, 230);
        assert_eq!(snap.tokens_out, 100);
    }

    #[test]
    fn tool_use_task_with_matching_result_marks_done() {
        let mut acc = Accumulator::new();
        let use_line = br#"{"type":"assistant","message":{"model":"claude-sonnet-4-6","content":[{"type":"tool_use","id":"tu_1","name":"Task","input":{}}]}}"#;
        let result_line = br#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"tu_1","content":"ok"}]}}"#;
        acc.feed_line(use_line);
        acc.feed_line(result_line);
        let snap = acc.snapshot_for_test();
        assert_eq!(snap.subagents_total, 1);
        assert_eq!(snap.subagents_active, 0);
    }

    #[test]
    fn tool_use_task_without_result_is_active() {
        let mut acc = Accumulator::new();
        let use_line = br#"{"type":"assistant","message":{"model":"claude-sonnet-4-6","content":[{"type":"tool_use","id":"tu_1","name":"Task","input":{}}]}}"#;
        acc.feed_line(use_line);
        let snap = acc.snapshot_for_test();
        assert_eq!(snap.subagents_total, 1);
        assert_eq!(snap.subagents_active, 1);
    }

    #[test]
    fn duplicate_tool_use_id_counts_once() {
        let mut acc = Accumulator::new();
        let use_line = br#"{"type":"assistant","message":{"model":"claude-sonnet-4-6","content":[{"type":"tool_use","id":"tu_1","name":"Task","input":{}}]}}"#;
        acc.feed_line(use_line);
        acc.feed_line(use_line);
        let snap = acc.snapshot_for_test();
        assert_eq!(snap.subagents_total, 1);
        assert_eq!(snap.subagents_active, 1);
    }

    #[test]
    fn tool_use_non_task_is_not_counted() {
        let mut acc = Accumulator::new();
        let use_line = br#"{"type":"assistant","message":{"model":"claude-sonnet-4-6","content":[{"type":"tool_use","id":"tu_1","name":"Bash","input":{}}]}}"#;
        acc.feed_line(use_line);
        let snap = acc.snapshot_for_test();
        assert_eq!(snap.subagents_total, 0);
        assert_eq!(snap.subagents_active, 0);
    }
}
