import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import { api, type UsageSnapshot } from "../api";
import { settings } from "./settings.svelte";
import type { Session, SessionState, Workspace } from "../types";

class AppStore {
  workspaces = $state<Workspace[]>([]);
  sessions = $state<Session[]>([]);
  activeSessionId = $state<string | null>(null);
  loading = $state(false);
  error = $state<string | null>(null);

  /// Track which sessions we've subscribed to so we don't double-listen.
  private listeners = new Map<string, UnlistenFn[]>();

  async refreshWorkspaces() {
    this.loading = true;
    this.error = null;
    try {
      this.workspaces = await api.listWorkspaces();
    } catch (e) {
      this.error = String(e);
    } finally {
      this.loading = false;
    }
  }

  async addWorkspace(path: string) {
    this.error = null;
    try {
      const ws = await api.addWorkspace(path);
      this.workspaces = [...this.workspaces, ws];
    } catch (e) {
      this.error = String(e);
    }
  }

  async removeWorkspace(id: string) {
    try {
      await api.removeWorkspace(id);
      this.workspaces = this.workspaces.filter((w) => w.id !== id);
      // Also close any sessions belonging to this workspace.
      const orphaned = this.sessions.filter((s) => s.workspaceId === id);
      for (const s of orphaned) await this.closeSession(s.id);
    } catch (e) {
      this.error = String(e);
    }
  }

  async createWorktree(parentId: string, name: string) {
    try {
      const ws = await api.createWorktree(parentId, name);
      this.workspaces = [...this.workspaces, ws];
    } catch (e) {
      this.error = String(e);
    }
  }

  async startSessionFor(
    workspaceId: string,
    opts: { cols?: number; rows?: number; resumeId?: string } = {},
  ) {
    const { cols = 80, rows = 24, resumeId } = opts;
    this.error = null;
    try {
      const s = await api.createSession(workspaceId, cols, rows, false, resumeId);
      this.sessions = [...this.sessions, s];
      this.activeSessionId = s.id;
      await this.subscribeToSession(s.id);
      return s;
    } catch (e) {
      this.error = String(e);
      return null;
    }
  }

  async closeSession(sessionId: string) {
    try {
      await api.closeSession(sessionId);
    } catch (e) {
      // ignore — backend may have cleaned up already on child exit.
    }
    this.unsubscribeFromSession(sessionId);
    this.sessions = this.sessions.filter((s) => s.id !== sessionId);
    if (this.activeSessionId === sessionId) {
      this.activeSessionId = this.sessions[0]?.id ?? null;
    }
  }

  selectSession(id: string | null) {
    this.activeSessionId = id;
  }

  /// Wire up the three event streams a session emits. Bytes from the PTY are
  /// handled inside the Terminal component (it owns the xterm instance), so
  /// here we only listen for state/usage updates that drive the right panel.
  private async subscribeToSession(id: string) {
    if (this.listeners.has(id)) return;

    const stateUnlisten = await listen<SessionState>(
      `session:state:${id}`,
      (event) => {
        const next = event.payload;
        const prev = this.sessions.find((s) => s.id === id)?.state;
        this.sessions = this.sessions.map((s) =>
          s.id === id ? { ...s, state: next } : s,
        );
        // Fire a desktop notification when an active agent settles back to
        // idle — the typical "your work is done" signal. We deliberately
        // skip the noisy intermediate transitions (thinking ↔ generating).
        if (
          (prev === "thinking" || prev === "generating") &&
          next === "idle" &&
          settings.notifications &&
          // Avoid notifying for the session the user is already watching.
          this.activeSessionId !== id
        ) {
          this.notifyIfAllowed(id);
        }
      },
    );

    const usageUnlisten = await listen<UsageSnapshot>(
      `session:usage:${id}`,
      (event) => {
        const u = event.payload;
        this.sessions = this.sessions.map((s) =>
          s.id === id
            ? {
                ...s,
                cost: u.cost,
                tokensIn: u.tokensIn,
                tokensOut: u.tokensOut,
                model: u.model,
                contextTokens: u.contextTokens,
                subagentsTotal: u.subagentsTotal,
                subagentsActive: u.subagentsActive,
              }
            : s,
        );
      },
    );

    const exitUnlisten = await listen<void>(`pty:exit:${id}`, () => {
      // Child exited — drop the session from the registry. The watchers stop
      // themselves once we close the session backend-side.
      this.closeSession(id);
    });

    this.listeners.set(id, [stateUnlisten, usageUnlisten, exitUnlisten]);
  }

  private unsubscribeFromSession(id: string) {
    const unlisteners = this.listeners.get(id);
    if (unlisteners) {
      for (const u of unlisteners) u();
      this.listeners.delete(id);
    }
  }

  /// Fire a system notification for the named session. Lazily requests
  /// permission the first time. We swallow errors silently — notification
  /// support varies across platforms and we'd rather degrade gracefully
  /// than surface a permission failure as a real error in the UI.
  private async notifyIfAllowed(sessionId: string) {
    try {
      let granted = await isPermissionGranted();
      if (!granted) {
        const perm = await requestPermission();
        granted = perm === "granted";
      }
      if (!granted) return;
      const session = this.sessions.find((s) => s.id === sessionId);
      const ws = this.workspaces.find((w) => w.id === session?.workspaceId);
      sendNotification({
        title: ws ? `${ws.name} · idle` : "Session idle",
        body: ws?.branch
          ? `${ws.branch} finished — click to switch back.`
          : "Session finished — click to switch back.",
      });
    } catch {
      // Best-effort. Don't surface.
    }
  }

  /// Auto-resume the most recently active session for each workspace, as
  /// long as it was modified within the last 6 hours. Capped at 3 to avoid
  /// hammering the user with N parallel claudes on a 10-workspace setup.
  async autoResumeRecent() {
    if (!settings.autoResume) return;
    const SIX_HOURS = 6 * 60 * 60 * 1000;
    const now = Date.now();

    type Candidate = { workspaceId: string; resumeId: string; modifiedAtMs: number };
    const candidates: Candidate[] = [];
    for (const ws of this.workspaces) {
      try {
        const r = await api.latestClaudeSession(ws.id);
        if (r && now - r.modifiedAtMs <= SIX_HOURS) {
          candidates.push({
            workspaceId: ws.id,
            resumeId: r.id,
            modifiedAtMs: r.modifiedAtMs,
          });
        }
      } catch {
        // skip
      }
    }
    candidates.sort((a, b) => b.modifiedAtMs - a.modifiedAtMs);
    for (const c of candidates.slice(0, 3)) {
      await this.startSessionFor(c.workspaceId, { resumeId: c.resumeId });
    }
  }

  /// Listen for OS-level file drops on the window. When a user drags a
  /// file (image, doc, anything) onto the app, Tauri delivers the absolute
  /// paths via this event. We paste them into the active session's PTY
  /// using shell-escape conventions — same as Terminal.app — so Claude's
  /// TUI sees them as if the user typed the paths.
  ///
  /// The web-layer drag-drop is also bypassed by Tauri's native handler,
  /// so we don't need to add `ondrop` listeners on individual elements.
  async initDragDropListener() {
    const { listen } = await import("@tauri-apps/api/event");
    await listen<{ paths: string[] }>("tauri://drag-drop", (event) => {
      const paths = event.payload?.paths;
      if (!paths?.length) return;
      if (!this.activeSessionId) {
        this.error =
          "Drop a file with an active Claude session selected to attach it.";
        return;
      }
      const text = paths.map(shellEscape).join(" ");
      // Wrap in bracketed paste mode so Claude's TUI recognizes the input
      // as a paste, not keystrokes. That's what triggers the collapsed
      // `[Pasted text #1]` / `[Image #1]` placeholders instead of dumping
      // the raw path into the input. The terminal sequences are:
      //   ESC [ 200 ~  → start of paste
      //   ESC [ 201 ~  → end of paste
      // Followed by a trailing space so the user can keep typing or hit
      // Enter without the path running into the next character.
      const ESC = "\x1b";
      const pasted = `${ESC}[200~${text}${ESC}[201~ `;
      void api.writePty(this.activeSessionId, pasted);
    });
  }

  /// Reorder sessions in place (used by drag-to-reorder in the tab bar).
  reorderSessions(fromIndex: number, toIndex: number) {
    if (fromIndex === toIndex) return;
    const next = [...this.sessions];
    const [moved] = next.splice(fromIndex, 1);
    next.splice(toIndex, 0, moved);
    this.sessions = next;
  }
}

export const app = new AppStore();

/// Backslash-escape spaces and a handful of shell metacharacters in `p`.
/// Claude's TUI doesn't itself need shell escaping (it's not a shell prompt),
/// but matching Terminal.app's behavior keeps muscle memory consistent: a
/// dragged path "looks right" the moment it lands in the input.
function shellEscape(p: string): string {
  return p.replace(/([ \\"'$`!()&|;<>*?[\]{}~#])/g, "\\$1");
}
