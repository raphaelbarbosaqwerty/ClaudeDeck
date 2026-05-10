# Changelog

All notable changes to this project will be documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

Features and fixes that have landed on `main` but aren't yet in a tagged
release. Move these into a versioned section when cutting a new release.

---

## [0.1.0] — 2026-05-10

Initial public release.

### ✨ Features

- **Multi-session terminals** — every tab is an independent `claude` process
  in its own pseudo-terminal (`portable-pty`: openpty on Unix, ConPTY on
  Windows).
- **Worktree isolation** — sessions run in `git worktree`-managed branches
  under `<repo>/.claude/worktrees/`. Stacked worktrees (worktree-of-a-worktree)
  are first-class for stacked-PR workflows.
- **Status awareness** — reads Claude Code's per-pid file
  (`~/.claude/sessions/<pid>.json`) for authoritative `Idle / Generating /
  Thinking` state.
- **Cost & token tracking** — tails `~/.claude/projects/<cwd>/*.jsonl` and
  applies local pricing for Opus / Sonnet / Haiku, including cache-creation
  and cache-read tiers.
- **Subagent counter** — `↳ N/M` chip on session cards shows live `Task` /
  `Agent` tool dispatches in flight vs. completed.
- **Context window gauge** — per-session bar tints from cyan → yellow → red
  as you fill the model's context.
- **Resume previous sessions** — detects the latest JSONL per workspace and
  offers `↻ Resume` to continue exactly where you left off. Optional
  auto-resume on launch (cap of 3 most recent within 6 h).
- **Auto-discovery of `.claude/`** — parses `.claude/agents/*.md` for each
  workspace and surfaces one-click dispatch buttons. Pipeline state
  (unpushed commits, `seeds-pending.md` size) becomes clickable CTAs in the
  status bar.
- **Toolkit panel (⌘K)** — three-section drawer: Pipeline (state-aware
  actions), Your Agents (auto-discovered), Custom Commands (shell / prompt /
  url / agent kinds).
- **Native drag-drop of files** — drop an image or any file into the window
  and it lands as a bracketed paste in the active session, matching native
  Claude Code terminal behavior.
- **Notifications** — desktop alert when a non-active session settles back
  to Idle.
- **Drag-to-reorder tabs**, **light/dark theme**, **workspace search**,
  **procedural pixel-art avatars** per workspace.

### 🛠 Stack

- Backend: Rust + Tauri 2 + portable-pty + notify
- Frontend: SvelteKit + Svelte 5 (runes) + xterm.js + Phosphor Icons
- Bundle: ~10 MB on macOS, ~12 MB on Windows, ~15 MB on Linux

### 🧪 Tests

- 73 Rust unit tests across `model`, `state`, `git`, `discovery`,
  `status_watcher`, `jsonl_watcher`, `toolkit`, `workspaces`.
- 14 frontend tests across `color`, `avatar`, `settings` utilities.
- CI runs the full suite on macOS, Windows, and Ubuntu on every push.

### 📝 Notes

- macOS builds are not yet code-signed. First launch may show a Gatekeeper
  warning — see the [Troubleshooting](README.md#troubleshooting) section
  for the one-line `xattr -cr` fix.
- Windows builds are not yet Authenticode-signed. SmartScreen will warn on
  first launch; click **More info → Run anyway**.

### 🙏 Acknowledgments

Inspired by [DraftFrame](https://github.com/intuitive-compute/DraftFrame)
(Swift / macOS only). Clean-room cross-platform implementation. See
[NOTICE.md](NOTICE.md) for full attribution.

---

[Unreleased]: https://github.com/raphaelbarbosaqwerty/ClaudeDeck/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/raphaelbarbosaqwerty/ClaudeDeck/releases/tag/v0.1.0
