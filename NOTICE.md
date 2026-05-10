# Acknowledgments

ClaudeDeck draws architectural inspiration from the projects below. **No
source code was copied.** Every line in this repository is original work,
written from scratch in different languages (Rust + TypeScript) than the
projects that inspired it.

This file exists as an extra courtesy beyond what the MIT license requires —
to give visible credit to the projects whose ideas helped shape this one.

---

## DraftFrame

- **Repository**: [intuitive-compute/DraftFrame](https://github.com/intuitive-compute/DraftFrame)
- **Author**: Intuitive Compute
- **License**: MIT
- **Language**: Swift / AppKit / SwiftTerm (macOS only)

The original open-source Claude Code worktree manager. ClaudeDeck owes the
following ideas to its design:

- The pattern of running each Claude session inside its own git worktree under
  `<repo>/.claude/worktrees/`.
- Reading the per-pid status file at `~/.claude/sessions/<pid>.json` as the
  authoritative source of `Idle / Generating / Thinking` state, rather than
  parsing the TUI's terminal output.
- Tailing `~/.claude/projects/<encoded-cwd>/*.jsonl` for token usage and
  applying local pricing for Opus / Sonnet / Haiku tiers.
- The right-hand panel of session cards showing live status and cost.
- The convention of resolving the `claude` binary across PATH, Homebrew, and
  `~/.claude/local/` when GUI applications don't inherit shell PATH on macOS.

ClaudeDeck re-implements every one of these on a cross-platform stack
(Tauri + Svelte + Rust + xterm.js + portable-pty), and extends the design with
auto-discovery of `.claude/agents/`, state-aware pipeline actions, multi-level
stacked worktrees, drag-drop bracketed paste, theme switching, and other
features beyond DraftFrame's scope.

If you want a pure-Swift, macOS-native implementation, **DraftFrame remains an
excellent option** and the original.

---

## Claude Code

- **Vendor**: [Anthropic](https://anthropic.com)
- **Product**: [Claude Code](https://claude.com/claude-code)

The CLI being orchestrated. ClaudeDeck reads its filesystem outputs
(`~/.claude/sessions/`, `~/.claude/projects/`) and spawns the `claude` binary
in pseudo-terminals. We never proxy the API or rewrite prompts — every session
is a real `claude` process and the conversation belongs entirely to the user
and Anthropic.

---

## SwiftTerm

- **Repository**: [migueldeicaza/SwiftTerm](https://github.com/migueldeicaza/SwiftTerm)
- **Author**: Miguel de Icaza
- **License**: MIT

The terminal emulator powering DraftFrame's terminal pane. ClaudeDeck uses
[xterm.js](https://github.com/xtermjs/xterm.js) (also MIT) instead, because
xterm.js works in a webview and reaches Windows and Linux, but the goal —
faithful VT100/Xterm emulation with bracketed-paste mode — is the same.

---

## License compatibility

All projects above are MIT-licensed. ClaudeDeck is also MIT-licensed
([LICENSE](LICENSE)) — the same spirit of openness that made these
inspirations possible.
