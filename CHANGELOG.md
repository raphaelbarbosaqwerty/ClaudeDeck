# Changelog

All notable changes to this project will be documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

Features and fixes that have landed on `main` but aren't yet in a tagged
release. Move these into a versioned section when cutting a new release.

---

## [0.1.1] — 2026-05-10

Patch release with two important fixes that landed shortly after v0.1.0.
**Recommended upgrade for all macOS users**, especially anyone with hooks
configured in `~/.claude/settings.json`.

### 🐛 Fixes

- **PTY: spawned `claude` now inherits a full login-shell PATH.** macOS
  GUI apps launched from Finder/Dock inherit the minimal launchd PATH
  (`/usr/bin:/bin:/usr/sbin:/sbin`), which doesn't include Homebrew, nvm,
  fvm, or asdf. Any Claude Code hook that called `node` (the default for
  many user-defined hooks) failed with `node: command not found`. v0.1.1
  resolves the user's full PATH once via `zsh -l -i -c 'echo $PATH'` —
  the same env Terminal.app sees — caches it in `OnceCell`, and injects
  it into every PTY spawn. Unblocks Claude's hook system and any CLI
  the agent invokes (npm, bun, gh, kubectl, cargo, deno, …).
- **CI: macOS release jobs no longer crash on missing Apple secrets.**
  v0.1.0's release workflow declared `APPLE_CERTIFICATE` and friends in
  the env block. With no real secrets configured, GitHub Actions
  resolved them to empty strings, and `tauri-action` invoked
  `security import` on those empty values, causing
  `SecKeychainItemImport: One or more parameters were not valid`. The
  Apple env vars are now commented out and accompanied by step-by-step
  instructions for enabling signing once a Developer ID exists.

### 📝 Notes

- v0.1.0's macOS release artifacts (`.dmg`) failed to build for the
  reason above. Use v0.1.1's installers instead.
- Linux (`.deb` / `.AppImage`) and Windows (`.exe` / `.msi`) artifacts
  in v0.1.0 were fine but identical to v0.1.1 except for the bundled
  PATH-resolution fix — upgrading is still recommended for the
  `node`-in-hook fix.

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

[Unreleased]: https://github.com/raphaelbarbosaqwerty/ClaudeDeck/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/raphaelbarbosaqwerty/ClaudeDeck/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/raphaelbarbosaqwerty/ClaudeDeck/releases/tag/v0.1.0
