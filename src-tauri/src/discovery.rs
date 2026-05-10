//! Inspect a workspace's `.claude/` folder to surface team-defined agents
//! and pipeline-relevant state. Everything here is read-only; we never
//! modify the user's `.claude/` tree.

use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredAgent {
    /// File stem of `.claude/agents/<name>.md` — used as the dispatch token.
    pub name: String,
    /// First line of the markdown body (after frontmatter), or the
    /// `description:` frontmatter field, whichever is shorter and useful.
    pub description: Option<String>,
    /// Suggested icon: an emoji we infer from the agent's name. Keeps the
    /// auto-discovered list visually consistent with manually-added commands.
    pub icon: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineSignal {
    /// Stable id the UI uses for the "next action" button.
    pub id: String,
    /// User-visible label, e.g. "3 unpushed commits".
    pub label: String,
    /// What to do: an agent name to dispatch, or a literal prompt body.
    pub suggested_agent: Option<String>,
    pub suggested_prompt: Option<String>,
    pub icon: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceDiscovery {
    pub has_claude_dir: bool,
    pub has_workflow: bool,
    pub has_skill: bool,
    pub agents: Vec<DiscoveredAgent>,
    pub pipeline: Vec<PipelineSignal>,
}

pub fn discover(cwd: &str) -> WorkspaceDiscovery {
    let claude = Path::new(cwd).join(".claude");
    let has_claude_dir = claude.is_dir();

    let mut agents = Vec::new();
    if has_claude_dir {
        if let Ok(entries) = std::fs::read_dir(claude.join("agents")) {
            for e in entries.flatten() {
                let p = e.path();
                if p.extension().and_then(|s| s.to_str()) != Some("md") {
                    continue;
                }
                if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                    let description = read_agent_description(&p);
                    agents.push(DiscoveredAgent {
                        name: stem.to_string(),
                        icon: icon_for_agent(stem),
                        description,
                    });
                }
            }
            agents.sort_by(|a, b| a.name.cmp(&b.name));
        }
    }

    WorkspaceDiscovery {
        has_claude_dir,
        has_workflow: claude.join("WORKFLOW.md").is_file(),
        has_skill: skill_dir_exists(&claude),
        agents,
        pipeline: detect_pipeline_signals(cwd, &claude),
    }
}

/// `.claude/skills/<anything>/SKILL.md` — the project name folder is unknown,
/// so we look for any subdir of `.claude/skills/` containing `SKILL.md`.
fn skill_dir_exists(claude: &Path) -> bool {
    let skills = claude.join("skills");
    let Ok(entries) = std::fs::read_dir(&skills) else {
        return false;
    };
    for e in entries.flatten() {
        if e.path().join("SKILL.md").is_file() {
            return true;
        }
    }
    false
}

/// Parse the description from an agent's markdown file. We accept either:
///   * YAML frontmatter `description:` line
///   * The first non-empty paragraph after the frontmatter
/// Truncated to ~120 chars so the UI doesn't render a wall of text.
pub(crate) fn read_agent_description(path: &Path) -> Option<String> {
    let body = std::fs::read_to_string(path).ok()?;
    let mut lines = body.lines();

    // Frontmatter: starts with `---` on the very first line.
    let first = lines.next()?;
    if first.trim() == "---" {
        for line in lines.by_ref() {
            if line.trim() == "---" {
                break;
            }
            if let Some(rest) = line.strip_prefix("description:") {
                return Some(truncate(rest.trim().trim_matches(|c| c == '"' || c == '\'')));
            }
        }
    } else {
        // No frontmatter — `first` was already real content.
        let trimmed = first.trim();
        if !trimmed.is_empty() && !trimmed.starts_with('#') {
            return Some(truncate(trimmed));
        }
    }

    // Fall back to the first non-empty body paragraph.
    for line in lines {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        return Some(truncate(t));
    }
    None
}

pub(crate) fn truncate(s: &str) -> String {
    const MAX: usize = 120;
    if s.chars().count() <= MAX {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(MAX).collect();
        out.push('…');
        out
    }
}

/// Map agent names to a Phosphor icon name (camelCase, matching
/// `Icon.svelte`'s registry on the frontend). The frontend resolves the
/// string to a real component; if a name doesn't match, the frontend
/// renders it as raw text — so we can also return emoji here as a
/// fallback if a future agent type genuinely needs one.
pub(crate) fn icon_for_agent(name: &str) -> String {
    let n = name.to_ascii_lowercase();
    // Order matters: more-specific patterns must be checked before broader
    // ones that they contain as substrings. `pre-open-pr` is a superstring
    // of `open-pr`, so its branch must come first; otherwise `pre-open-pr`
    // would match the open-pr arm and never reach airplaneTilt.
    let icon = if n.contains("senior") || n.contains("dev") {
        "hammer"
    } else if n.contains("qa") || n.contains("test") {
        "flask"
    } else if n.contains("review") && n.contains("pr") {
        "chatCircleText"
    } else if n.contains("review") {
        "magnifyingGlass"
    } else if n.contains("pre-open") || n.contains("preflight") {
        "airplaneTilt"
    } else if n.contains("open-pr") || n.contains("openpr") {
        "paperPlaneTilt"
    } else if n.contains("learn") {
        "plant"
    } else if n.contains("plan") {
        "mapTrifold"
    } else {
        "robot"
    };
    icon.to_string()
}

/// Two cheap, high-signal pipeline detections for MVP:
///   1. unpushed commits — "Open PR" candidate
///   2. seeds-pending.md non-empty — "Learner from PR" candidate
/// We deliberately avoid `gh pr list` / API calls here so the toolkit
/// renders fast even on slow networks; deeper signals can come later.
fn detect_pipeline_signals(cwd: &str, claude: &Path) -> Vec<PipelineSignal> {
    let mut out = Vec::new();

    if let Some(n) = unpushed_commit_count(cwd) {
        if n > 0 {
            out.push(PipelineSignal {
                id: "unpushed-commits".into(),
                label: format!(
                    "{n} unpushed commit{} on this branch",
                    if n == 1 { "" } else { "s" }
                ),
                suggested_agent: Some("open-pr".into()),
                suggested_prompt: None,
                icon: "paperPlaneTilt".into(),
            });
        }
    }

    if let Some(seeds) = find_seeds_pending(claude) {
        if seeds.has_content {
            out.push(PipelineSignal {
                id: "seeds-pending".into(),
                label: format!("seeds-pending.md has {} entries", seeds.entries),
                suggested_agent: Some("learner-from-pr".into()),
                suggested_prompt: None,
                icon: "plant".into(),
            });
        }
    }

    out
}

fn unpushed_commit_count(cwd: &str) -> Option<u32> {
    let out = Command::new("git")
        .args(["-C", cwd, "rev-list", "--count", "@{u}..HEAD"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?;
    s.trim().parse().ok()
}

struct SeedsInfo {
    has_content: bool,
    entries: usize,
}

fn find_seeds_pending(claude: &Path) -> Option<SeedsInfo> {
    // `seeds-pending.md` lives under .claude/skills/<project>/, but we don't
    // know the project folder name — search the first matching one.
    let skills = claude.join("skills");
    let entries = std::fs::read_dir(&skills).ok()?;
    for e in entries.flatten() {
        let candidate = e.path().join("seeds-pending.md");
        if !candidate.is_file() {
            continue;
        }
        let body = std::fs::read_to_string(&candidate).ok()?;
        // Heuristic for "non-empty": at least one bullet or numbered line.
        let bullets = body
            .lines()
            .filter(|l| {
                let t = l.trim_start();
                t.starts_with("- ") || t.starts_with("* ") || t.starts_with("1.")
            })
            .count();
        return Some(SeedsInfo {
            has_content: bullets > 0,
            entries: bullets,
        });
    }
    None
}

#[allow(dead_code)]
pub fn _unused() -> PathBuf {
    PathBuf::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use uuid::Uuid;

    fn unique_tempdir(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("claudedeck-test-{}-{}", tag, Uuid::new_v4()));
        fs::create_dir_all(&p).expect("create tempdir");
        p
    }

    #[test]
    fn icon_for_agent_senior_dev() {
        assert_eq!(icon_for_agent("senior-dev"), "hammer");
        assert_eq!(icon_for_agent("senior-dev-2"), "hammer");
        assert_eq!(icon_for_agent("dev"), "hammer");
        assert_eq!(icon_for_agent("DEV"), "hammer");
    }

    #[test]
    fn icon_for_agent_qa_test() {
        assert_eq!(icon_for_agent("qa-tester"), "flask");
        assert_eq!(icon_for_agent("test-runner"), "flask");
        assert_eq!(icon_for_agent("qa"), "flask");
    }

    #[test]
    fn icon_for_agent_pr_reviewer() {
        assert_eq!(icon_for_agent("pr-reviewer"), "chatCircleText");
        assert_eq!(icon_for_agent("review-pr"), "chatCircleText");
    }

    #[test]
    fn icon_for_agent_code_reviewer() {
        assert_eq!(icon_for_agent("code-reviewer"), "magnifyingGlass");
        assert_eq!(icon_for_agent("reviewer"), "magnifyingGlass");
    }

    #[test]
    fn icon_for_agent_open_pr() {
        assert_eq!(icon_for_agent("open-pr"), "paperPlaneTilt");
        assert_eq!(icon_for_agent("openpr"), "paperPlaneTilt");
    }

    #[test]
    fn icon_for_agent_pre_open_pr() {
        assert_eq!(icon_for_agent("pre-open-pr"), "airplaneTilt");
        assert_eq!(icon_for_agent("preflight"), "airplaneTilt");
    }

    #[test]
    fn icon_for_agent_learner() {
        assert_eq!(icon_for_agent("learner-from-pr"), "plant");
    }

    #[test]
    fn icon_for_agent_planner() {
        assert_eq!(icon_for_agent("planner"), "mapTrifold");
        assert_eq!(icon_for_agent("plan"), "mapTrifold");
    }

    #[test]
    fn icon_for_agent_unknown() {
        assert_eq!(icon_for_agent("foobar"), "robot");
        assert_eq!(icon_for_agent(""), "robot");
    }

    #[test]
    fn truncate_short_passes_through() {
        assert_eq!(truncate("hello"), "hello");
        assert_eq!(truncate(""), "");
        let exactly = "a".repeat(120);
        assert_eq!(truncate(&exactly), exactly);
    }

    #[test]
    fn truncate_long_is_cut_with_ellipsis() {
        let s = "a".repeat(200);
        let out = truncate(&s);
        assert_eq!(out.chars().count(), 121);
        assert!(out.ends_with('…'));
    }

    #[test]
    fn read_agent_description_yaml_frontmatter() {
        let dir = unique_tempdir("read-fm");
        let p = dir.join("a.md");
        fs::write(
            &p,
            "---\nname: foo\ndescription: \"Does the foo\"\n---\n\nbody text\n",
        )
        .unwrap();
        assert_eq!(read_agent_description(&p).as_deref(), Some("Does the foo"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_agent_description_no_frontmatter_uses_first_line() {
        let dir = unique_tempdir("read-nofm");
        let p = dir.join("a.md");
        fs::write(&p, "Just a body line.\nMore stuff\n").unwrap();
        assert_eq!(
            read_agent_description(&p).as_deref(),
            Some("Just a body line.")
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_agent_description_frontmatter_without_description_falls_back() {
        let dir = unique_tempdir("read-fm-nodesc");
        let p = dir.join("a.md");
        fs::write(
            &p,
            "---\nname: foo\n---\n\n# Heading\n\nFallback paragraph here.\n",
        )
        .unwrap();
        assert_eq!(
            read_agent_description(&p).as_deref(),
            Some("Fallback paragraph here.")
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_agent_description_empty_body() {
        let dir = unique_tempdir("read-empty");
        let p = dir.join("a.md");
        fs::write(&p, "").unwrap();
        assert_eq!(read_agent_description(&p), None);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn discover_finds_agent() {
        let dir = unique_tempdir("discover");
        let agents_dir = dir.join(".claude").join("agents");
        fs::create_dir_all(&agents_dir).unwrap();
        fs::write(
            agents_dir.join("foo.md"),
            "---\ndescription: Foo agent\n---\n",
        )
        .unwrap();

        let result = discover(dir.to_str().unwrap());
        assert!(result.has_claude_dir);
        assert_eq!(result.agents.len(), 1);
        assert_eq!(result.agents[0].name, "foo");
        assert_eq!(result.agents[0].description.as_deref(), Some("Foo agent"));
        let _ = fs::remove_dir_all(&dir);
    }
}
