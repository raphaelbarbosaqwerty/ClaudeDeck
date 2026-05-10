//! PTY layer. Each session spawns a child process inside a pseudo-terminal,
//! its stdout streams back to the frontend as Tauri events, and keystrokes
//! flow the other direction via commands.
//!
//! Cross-platform via `portable-pty`: openpty on Unix, ConPTY on Windows.

use anyhow::{anyhow, Context, Result};
use parking_lot::Mutex;
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::thread;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

pub struct PtyHandle {
    /// Used to send bytes to the child's stdin.
    writer: Box<dyn Write + Send>,
    /// Used to resize the child's TTY when the frontend reflows.
    master: Box<dyn MasterPty + Send>,
    /// Kept alive so the child isn't reaped while we still hold the writer.
    _child: Box<dyn portable_pty::Child + Send + Sync>,
}

#[derive(Default)]
pub struct PtyManager {
    inner: Mutex<HashMap<Uuid, PtyHandle>>,
}

impl PtyManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Spawn `claude` (or a fallback shell) in `cwd` and return a session id.
    /// When `command` is `Claude { resume: Some(id) }`, we add `--resume <id>`
    /// so Claude continues the existing conversation from its JSONL log.
    pub fn spawn(
        &self,
        app: AppHandle,
        session_id: Uuid,
        cwd: &str,
        cols: u16,
        rows: u16,
        command: SpawnCommand,
    ) -> Result<()> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("openpty")?;

        let mut cmd = match &command {
            SpawnCommand::Claude { resume } => build_claude_command(cwd, resume.as_deref())?,
            SpawnCommand::Shell => build_shell_command(cwd)?,
        };
        cmd.cwd(cwd);
        // Force a sane terminal type — Claude Code's TUI assumes 256 colors.
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        // Enrich PATH with the user's login-shell environment so that the
        // child `claude` (and any hooks it spawns via /bin/sh) can find
        // node / npm / fvm / asdf-managed tools. macOS GUI apps launched
        // via Finder/Dock inherit the minimal launchd PATH (essentially
        // /usr/bin:/bin:/usr/sbin:/sbin), which makes `node: command not
        // found` the default failure mode for any user-defined hook.
        if let Some(path) = enriched_path() {
            cmd.env("PATH", path);
        }

        let child = pair.slave.spawn_command(cmd).context("spawn child")?;
        let mut reader = pair
            .master
            .try_clone_reader()
            .context("clone pty reader")?;
        let writer = pair
            .master
            .take_writer()
            .context("take pty writer")?;

        // Spawn a stdout pump thread. We can't async/await this easily because
        // portable-pty gives us a blocking std::io::Read, so we just dedicate
        // a thread per session and emit Tauri events from inside.
        let app_for_thread = app.clone();
        let event_name = format!("pty:output:{}", session_id);
        let exit_event = format!("pty:exit:{}", session_id);
        thread::spawn(move || {
            // Buffer for UTF-8 sequences that span chunk boundaries. A 4-byte
            // codepoint (any emoji, most CJK, many box-drawing chars used by
            // Claude's TUI) can land split between two `read` returns. If we
            // decoded each chunk independently with `from_utf8_lossy`, the
            // partial bytes would render as `\u{FFFD}` replacement chars,
            // and xterm.js would advance the cursor by a different cell
            // count than Claude expected — which is *exactly* how the
            // "linhas concatenadas no meio do texto" artifacts appear.
            //
            // Carry forward up to 3 trailing bytes (max prefix of a 4-byte
            // codepoint) into the next chunk. If pending grows past 4, the
            // bytes are genuinely invalid (not just a partial codepoint),
            // so flush them lossy and reset.
            let mut pending: Vec<u8> = Vec::with_capacity(4);
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        // Splice pending + new chunk, find the longest valid
                        // UTF-8 prefix, hold the trailing partial bytes for
                        // the next iteration.
                        let combined: Vec<u8> = if pending.is_empty() {
                            buf[..n].to_vec()
                        } else {
                            let mut v = std::mem::take(&mut pending);
                            v.extend_from_slice(&buf[..n]);
                            v
                        };

                        let valid_up_to = match std::str::from_utf8(&combined) {
                            Ok(_) => combined.len(),
                            Err(e) => e.valid_up_to(),
                        };

                        if valid_up_to > 0 {
                            // SAFETY: from_utf8 just told us the prefix is valid.
                            let s = unsafe {
                                std::str::from_utf8_unchecked(&combined[..valid_up_to])
                            };
                            let _ = app_for_thread.emit(&event_name, s.to_string());
                        }

                        // Hold the unread tail. Cap at 4 bytes — any longer
                        // means the bytes are genuinely invalid, not just a
                        // partial codepoint, so flush them lossy and reset.
                        let tail = &combined[valid_up_to..];
                        if tail.len() > 4 {
                            let lossy = String::from_utf8_lossy(tail).into_owned();
                            let _ = app_for_thread.emit(&event_name, lossy);
                            pending.clear();
                        } else {
                            pending = tail.to_vec();
                        }
                    }
                    Err(_) => break,
                }
            }
            let _ = app_for_thread.emit(&exit_event, ());
        });

        let handle = PtyHandle {
            writer,
            master: pair.master,
            _child: child,
        };
        self.inner.lock().insert(session_id, handle);
        Ok(())
    }

    pub fn write(&self, session_id: &Uuid, data: &[u8]) -> Result<()> {
        let mut guard = self.inner.lock();
        let h = guard
            .get_mut(session_id)
            .ok_or_else(|| anyhow!("session not found"))?;
        h.writer.write_all(data).context("write to pty")?;
        h.writer.flush().ok();
        Ok(())
    }

    pub fn resize(&self, session_id: &Uuid, cols: u16, rows: u16) -> Result<()> {
        let guard = self.inner.lock();
        let h = guard
            .get(session_id)
            .ok_or_else(|| anyhow!("session not found"))?;
        h.master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("resize pty")?;
        Ok(())
    }

    pub fn close(&self, session_id: &Uuid) -> Result<()> {
        let mut guard = self.inner.lock();
        guard.remove(session_id);
        // Dropping the handle drops the writer, master, and child in that order.
        // `child` doesn't auto-kill on drop in all implementations, but closing
        // the master triggers SIGHUP for the child on Unix and breaks the pipe
        // on Windows, which is enough to terminate `claude` cleanly.
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum SpawnCommand {
    /// Run `claude` fresh (no `--resume`) when `resume` is None, or resume
    /// the named session id when Some(...).
    Claude { resume: Option<String> },
    Shell,
}

fn build_shell_command(_cwd: &str) -> Result<CommandBuilder> {
    let shell = pick_shell();
    Ok(CommandBuilder::new(shell))
}

/// Build a CommandBuilder that runs the user's `claude` CLI.
/// We don't go through a shell — passing args directly to `claude` is more
/// predictable and avoids quoting issues with paths that contain spaces.
fn build_claude_command(_cwd: &str, resume: Option<&str>) -> Result<CommandBuilder> {
    let claude = find_claude()?;
    let mut cmd = CommandBuilder::new(claude);
    if let Some(id) = resume {
        cmd.args(["--resume", id]);
    }
    Ok(cmd)
}

#[cfg(unix)]
fn pick_shell() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string())
}

#[cfg(windows)]
fn pick_shell() -> String {
    std::env::var("COMSPEC").unwrap_or_else(|_| "powershell.exe".to_string())
}

/// Resolve the user's full PATH the way Terminal.app would see it — a
/// login + interactive shell sources `.zprofile` AND `.zshrc`, which is
/// where most version managers (nvm/fvm/asdf) and tooling installs put
/// themselves. Cached at first call: spawning a shell takes ~100-200ms,
/// and we don't want to pay it on every session create.
fn enriched_path() -> Option<String> {
    use once_cell::sync::OnceCell;
    static CACHED: OnceCell<Option<String>> = OnceCell::new();
    CACHED
        .get_or_init(|| {
            #[cfg(unix)]
            {
                let shells = [
                    std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into()),
                    "/bin/zsh".into(),
                    "/bin/bash".into(),
                ];
                for shell in shells {
                    if !std::path::Path::new(&shell).is_file() {
                        continue;
                    }
                    if let Ok(out) = std::process::Command::new(&shell)
                        .args(["-l", "-i", "-c", "echo $PATH"])
                        .output()
                    {
                        if out.status.success() {
                            if let Ok(raw) = String::from_utf8(out.stdout) {
                                let path = raw.trim();
                                if !path.is_empty() {
                                    // Always prepend Homebrew Apple Silicon
                                    // path defensively, in case the user's
                                    // shell rc forgot to add it but we still
                                    // want `claude` from /opt/homebrew/bin
                                    // to resolve.
                                    let homebrew = "/opt/homebrew/bin";
                                    if path
                                        .split(':')
                                        .any(|p| p == homebrew)
                                    {
                                        return Some(path.to_string());
                                    } else {
                                        return Some(format!("{homebrew}:{path}"));
                                    }
                                }
                            }
                        }
                    }
                }
            }
            // Windows / fallback: leave PATH unchanged (parent inherits).
            None
        })
        .clone()
}

/// Locate the `claude` binary. macOS GUI apps typically don't inherit the
/// shell PATH, so we have to look in well-known locations and as a last
/// resort ask a login shell.
pub fn find_claude() -> Result<PathBuf> {
    // 1) crate `which` checks $PATH.
    if let Ok(p) = which::which("claude") {
        return Ok(p);
    }

    // 2) Common install locations.
    let home = dirs::home_dir().unwrap_or_default();
    let candidates: Vec<PathBuf> = vec![
        PathBuf::from("/opt/homebrew/bin/claude"),
        PathBuf::from("/usr/local/bin/claude"),
        home.join(".claude/local/claude"),
        home.join(".local/bin/claude"),
        // npm global install on Windows
        home.join("AppData/Roaming/npm/claude.cmd"),
        home.join("AppData/Roaming/npm/claude"),
    ];
    for c in candidates {
        if c.is_file() {
            return Ok(c);
        }
    }

    // 3) Login-shell PATH (Unix only — Windows already covered by `which`).
    #[cfg(unix)]
    {
        if let Ok(out) = std::process::Command::new("/bin/zsh")
            .args(["-l", "-c", "command -v claude"])
            .output()
        {
            if out.status.success() {
                let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !p.is_empty() {
                    let pb = PathBuf::from(p);
                    if pb.is_file() {
                        return Ok(pb);
                    }
                }
            }
        }
    }

    Err(anyhow!(
        "Could not find the `claude` CLI. Install it from https://claude.ai/claude-code and ensure it's in your PATH."
    ))
}
