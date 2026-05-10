use anyhow::{anyhow, Context, Result};
use std::path::Path;
use std::process::Command;

/// Resolve the git repo root for `path`. Returns None if `path` isn't inside a git repo.
pub fn repo_root(path: &str) -> Option<String> {
    let out = Command::new("git")
        .args(["-C", path, "rev-parse", "--show-toplevel"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?;
    let trimmed = s.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Best-effort current branch name. Returns None when in detached HEAD or non-git path.
pub fn current_branch(path: &str) -> Option<String> {
    let out = Command::new("git")
        .args(["-C", path, "branch", "--show-current"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?;
    let trimmed = s.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct WorktreeEntry {
    pub path: String,
    pub branch: Option<String>,
    pub head: String,
}

/// `git worktree list --porcelain` parser. The porcelain format is line-oriented,
/// blank-line-separated records of `key value` pairs.
pub fn list_worktrees(repo: &str) -> Result<Vec<WorktreeEntry>> {
    let out = Command::new("git")
        .args(["-C", repo, "worktree", "list", "--porcelain"])
        .output()
        .context("failed to spawn git worktree list")?;
    if !out.status.success() {
        return Err(anyhow!(
            "git worktree list failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    Ok(parse_worktree_porcelain(&stdout))
}

/// Parse the output of `git worktree list --porcelain`. Extracted so it can be
/// unit-tested without spawning git.
fn parse_worktree_porcelain(stdout: &str) -> Vec<WorktreeEntry> {
    let mut result = Vec::new();
    let mut path = String::new();
    let mut head = String::new();
    let mut branch: Option<String> = None;

    let flush = |path: &mut String,
                 head: &mut String,
                 branch: &mut Option<String>,
                 acc: &mut Vec<WorktreeEntry>| {
        if !path.is_empty() {
            acc.push(WorktreeEntry {
                path: std::mem::take(path),
                branch: branch.take(),
                head: std::mem::take(head),
            });
        }
    };

    for line in stdout.lines() {
        if line.is_empty() {
            flush(&mut path, &mut head, &mut branch, &mut result);
            continue;
        }
        if let Some(rest) = line.strip_prefix("worktree ") {
            path = rest.to_string();
        } else if let Some(rest) = line.strip_prefix("HEAD ") {
            head = rest.to_string();
        } else if let Some(rest) = line.strip_prefix("branch ") {
            // refs/heads/foo -> foo
            branch = Some(rest.trim_start_matches("refs/heads/").to_string());
        }
    }
    flush(&mut path, &mut head, &mut branch, &mut result);

    result
}

/// Worktrees managed by claudedeck live under `<repo>/.claude/worktrees/`,
/// matching DraftFrame's convention so existing repos stay compatible.
pub const WORKTREE_SUBPATH: &str = ".claude/worktrees";

/// Create a new worktree at `<repo>/.claude/worktrees/<name>` on a branch named `name`.
/// If the branch already exists, attaches to it instead of creating it.
pub fn create_worktree(repo: &str, name: &str, base_branch: Option<&str>) -> Result<String> {
    let base_dir = Path::new(repo).join(WORKTREE_SUBPATH);
    std::fs::create_dir_all(&base_dir).context("creating worktrees base dir")?;

    let target = base_dir.join(name);
    let target_str = target.to_string_lossy().into_owned();

    let resolved_base = base_branch
        .map(str::to_string)
        .or_else(|| current_branch(repo))
        .unwrap_or_else(|| "main".to_string());

    let with_b = Command::new("git")
        .args([
            "-C",
            repo,
            "worktree",
            "add",
            "-b",
            name,
            &target_str,
            &resolved_base,
        ])
        .output()
        .context("spawn git worktree add -b")?;

    if with_b.status.success() {
        return Ok(target_str);
    }

    let stderr = String::from_utf8_lossy(&with_b.stderr);
    if stderr.contains("already exists") {
        // Branch exists — attach instead of creating.
        let attach = Command::new("git")
            .args(["-C", repo, "worktree", "add", &target_str, name])
            .output()
            .context("spawn git worktree add (attach)")?;
        if attach.status.success() {
            return Ok(target_str);
        }
        return Err(anyhow!(
            "git worktree add failed: {}",
            String::from_utf8_lossy(&attach.stderr)
        ));
    }

    Err(anyhow!("git worktree add failed: {}", stderr))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worktree_subpath_constant() {
        assert_eq!(WORKTREE_SUBPATH, ".claude/worktrees");
    }

    #[test]
    fn parse_empty_input() {
        let v = parse_worktree_porcelain("");
        assert!(v.is_empty());
    }

    #[test]
    fn parse_single_worktree() {
        let input = "worktree /home/u/repo\n\
                     HEAD abcdef1234567890abcdef1234567890abcdef12\n\
                     branch refs/heads/main\n";
        let v = parse_worktree_porcelain(input);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].path, "/home/u/repo");
        assert_eq!(v[0].head, "abcdef1234567890abcdef1234567890abcdef12");
        assert_eq!(v[0].branch.as_deref(), Some("main"));
    }

    #[test]
    fn parse_multiple_worktrees_with_detached() {
        let input = "worktree /home/u/repo\n\
                     HEAD 1111111111111111111111111111111111111111\n\
                     branch refs/heads/main\n\
                     \n\
                     worktree /home/u/repo/.claude/worktrees/feat-a\n\
                     HEAD 2222222222222222222222222222222222222222\n\
                     branch refs/heads/feat-a\n\
                     \n\
                     worktree /home/u/repo/.claude/worktrees/detached\n\
                     HEAD 3333333333333333333333333333333333333333\n\
                     detached\n\
                     \n";
        let v = parse_worktree_porcelain(input);
        assert_eq!(v.len(), 3);

        assert_eq!(v[0].path, "/home/u/repo");
        assert_eq!(v[0].branch.as_deref(), Some("main"));
        assert_eq!(v[0].head, "1111111111111111111111111111111111111111");

        assert_eq!(v[1].path, "/home/u/repo/.claude/worktrees/feat-a");
        assert_eq!(v[1].branch.as_deref(), Some("feat-a"));

        assert_eq!(v[2].path, "/home/u/repo/.claude/worktrees/detached");
        assert_eq!(v[2].branch, None);
        assert_eq!(v[2].head, "3333333333333333333333333333333333333333");
    }

    #[test]
    fn parse_handles_blank_lines_and_missing_fields() {
        // Leading blank lines, trailing blank lines, and a record missing HEAD.
        let input = "\n\
                     \n\
                     worktree /a\n\
                     branch refs/heads/x\n\
                     \n\
                     \n\
                     worktree /b\n\
                     HEAD deadbeefdeadbeefdeadbeefdeadbeefdeadbeef\n\
                     \n";
        let v = parse_worktree_porcelain(input);
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].path, "/a");
        assert_eq!(v[0].branch.as_deref(), Some("x"));
        assert_eq!(v[0].head, ""); // missing HEAD -> empty default
        assert_eq!(v[1].path, "/b");
        assert_eq!(v[1].branch, None);
        assert_eq!(v[1].head, "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef");
    }

    #[test]
    fn parse_no_trailing_blank_line_still_flushes() {
        let input = "worktree /only\nHEAD aaa\nbranch refs/heads/only";
        let v = parse_worktree_porcelain(input);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].path, "/only");
        assert_eq!(v[0].branch.as_deref(), Some("only"));
    }
}

/// Remove a worktree (does not delete the branch).
pub fn remove_worktree(repo: &str, path: &str) -> Result<()> {
    let out = Command::new("git")
        .args(["-C", repo, "worktree", "remove", path])
        .output()
        .context("spawn git worktree remove")?;
    if !out.status.success() {
        return Err(anyhow!(
            "git worktree remove failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(())
}
