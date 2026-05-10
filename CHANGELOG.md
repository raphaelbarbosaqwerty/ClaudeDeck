# Changelog

All notable changes to this project will be documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

Features and fixes that have landed on `main` but aren't yet in a tagged
release. Move these into a versioned section when cutting a new release.

---

## [0.2.0] — 2026-05-10

Big polish + features release. The core multi-session orchestrator now
has serious organization, customization, and ergonomics work. Categories,
themes, mascot animations, shell side-panes — the kind of stuff that
turns "useful tool" into "tool I keep open all day."

### ✨ Features

- **Workspace categories.** Tag any workspace with a free-form label
  ("Work", "Side projects", "Clients · Acme"). The right-hand Sessions
  panel groups by category, with collapsible section headers, live-count
  badges, and "Uncategorized" pinned at the bottom. Picker is a combobox
  that autocompletes from your existing categories so you don't end up
  with `Work` / `work` / `WORK` fragmentation. Worktrees inherit their
  parent project's category automatically.
- **Live sessions float to the top.** Within each category section,
  groups with at least one running session sort above dormant ones —
  your active work never gets buried among placeholder/resume cards.
- **Settings modal** (gear icon next to the theme toggle). Three sections:
  - 8 terminal theme presets: ClaudeDeck, Dracula, One Dark, Tokyo
    Night, Catppuccin Mocha, Solarized Dark, GitHub Dark, Nord. Live
    preview swatches, instant apply.
  - Background opacity slider (50–100%, applies as rgba alpha to the
    xterm background).
  - Terminal padding (4–32px, default 4px — keeps the layout dense).
  - macOS vibrancy toggle: native NSVisualEffectView blur showing the
    desktop wallpaper through the window. No-op on Windows/Linux.
  - Auto-resume on launch + desktop notifications toggles (moved from
    implicit defaults to explicit user-controlled settings).
- **Auxiliary shell panes.** Click `▦ Shell` in the bottom status bar
  and a real `zsh`/`bash` column drops into a strip below the active
  Claude session. Click again to add another column side-by-side. Each
  column has its own PTY in the workspace's `cwd`, perfect for
  `npm run dev`, `supabase db start`, `tail -f logs`, etc. without
  leaving Claude. Drag the resizer to adjust strip height; close
  individual columns and the strip reflows. Aux panes are tab-scoped
  (each main session has its own set), hidden from the sessions panel
  and cost roll-up since they're not work units.
- **Mascot animations.** The procedural pixel-art avatars now react to
  state — gentle breathe on `Thinking`, stronger pulse + green glow on
  `Generating`, soft nod on `userInput`, alarm-shake on
  `needsAttention`. Respects `prefers-reduced-motion`.
- **Phosphor icon library.** Replaced ad-hoc emoji + inline SVGs with a
  consistent semantic icon set (folder, gitBranch, plus, x, terminal,
  toolbox, hammer, flask, magnifyingGlass, plant, etc.). Backend now
  emits semantic icon names for auto-discovered agents and pipeline
  signals, frontend resolves to actual Phosphor components.

### 🛠 Developer experience

- **`CLAUDEDECK_CONFIG_DIR` env override.** Run an isolated dev build
  side-by-side with the installed app:
  ```bash
  CLAUDEDECK_CONFIG_DIR="$HOME/.config/claudedeck-dev" pnpm tauri dev
  ```
  The dev instance reads/writes its own `workspaces.json` and `toolkits/`,
  so feature testing never touches your real workspace state.
- **`pnpm bump` script** (`scripts/bump-version.mjs`). Synchronizes the
  three places Tauri reads the version (`package.json`,
  `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`) so future
  releases can't drift.

### 🔧 Internal

- **Test coverage**: 134 tests passing (was 87 before — added 47).
  - Rust: `Workspace.category` serde, `Session.is_aux` defaults +
    serialization, `normalize_category` (whitespace, empty, etc.),
    `config_base_dir` env override.
  - Frontend (Vitest): all 8 theme presets validated for shape +
    color format, `getPreset` fallback, `withOpacity` math, settings
    store clamping (opacity 0.5–1.0, padding 4–32px), localStorage
    persistence.
- **Package metadata**: README updated with screenshots covering the
  Settings modal and multi-shell aux panes.
- **CI release workflow**: Apple signing env vars stay commented out
  pending Developer ID setup. The previously-added Tauri auto-updater
  signing block is also commented out (the auto-updater feature is
  deferred to a future release — see `docs/AUTO_UPDATE_SETUP.md`).

### 📝 Notes

- **Auto-updater is deferred**, not removed. The full scaffolding doc
  lives at `docs/AUTO_UPDATE_SETUP.md`. Wiring it up requires generating
  a tauri-signer keypair + adding two GitHub secrets, which we've held
  for a future release. Existing v0.1.x users will need to download
  v0.2.0 manually one more time. Subsequent versions can be auto-updated
  once the updater plugin is reintroduced with a real pubkey.
- **Homebrew tap setup doc** lives at `docs/HOMEBREW.md` for whenever
  the project's ready to publish a `brew install --cask` formula.
- macOS users: same `xattr -cr /Applications/claudedeck.app` dance as
  before applies to v0.2.0 since builds remain unsigned.

### 🙏 Acknowledgments

Inspired (still!) by [DraftFrame](https://github.com/intuitive-compute/DraftFrame).
The mascot pixel-art convention, the worktree-per-session approach, and
the right-hand sessions panel all owe their existence to that project.

---

## [0.1.2] — 2026-05-10

Patch release fixing terminal-rendering corruption that affected sessions
with multi-byte characters (Portuguese accents, emoji, box-drawing) —
which is essentially every real Claude Code conversation.

### 🐛 Fixes

- **PTY: UTF-8 sequences split across read boundaries no longer corrupt
  the terminal display.** The reader thread previously decoded each
  4 KB chunk independently with `from_utf8_lossy`; when a multi-byte
  codepoint landed split between two reads, the partial bytes became
  `U+FFFD` replacement characters. Because `�` advances the cursor
  1 cell while the original CJK/wide character would have advanced 2,
  Claude's TUI and xterm.js diverged on cursor column. Subsequent lines
  drew on top of one another, producing the
  `"Twi---confirm hyperframeskfoldersgone---ght"` style of garbled text.
  The reader now buffers up to 4 trailing bytes (max UTF-8 codepoint
  length) between chunks and only emits valid UTF-8.

### ✨ New

- **Keyboard shortcut `⌘⇧R` (`Ctrl+Shift+R` on Linux/Windows) forces a
  redraw of the active terminal.** Recovery path for the rare cases when
  Claude's alt-buffer still drifts: re-fits the xterm host, sends two
  spaced `resize_pty` calls (back-to-back SIGWINCH coalesce), and calls
  `xterm.refresh()` to repaint every cell from the buffer. Equivalent to
  the "Ctrl+L → redraw" muscle memory but for a TUI app.

### 📝 Notes

- v0.1.1 users on macOS will benefit immediately from upgrading. The
  artifact frequency depends heavily on what Claude is rendering; users
  who run in projects with lots of file lists, git output, or non-ASCII
  text were hitting it constantly.

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

[Unreleased]: https://github.com/raphaelbarbosaqwerty/ClaudeDeck/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/raphaelbarbosaqwerty/ClaudeDeck/compare/v0.1.2...v0.2.0
[0.1.2]: https://github.com/raphaelbarbosaqwerty/ClaudeDeck/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/raphaelbarbosaqwerty/ClaudeDeck/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/raphaelbarbosaqwerty/ClaudeDeck/releases/tag/v0.1.0
