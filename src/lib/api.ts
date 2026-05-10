import { invoke } from "@tauri-apps/api/core";
import type {
  CommandKind,
  Session,
  SessionState,
  ToolkitCommand,
  ToolkitView,
  Workspace,
} from "./types";

export interface WorktreeEntry {
  path: string;
  branch?: string;
  head: string;
}

export interface UsageSnapshot {
  cost: number;
  tokensIn: number;
  tokensOut: number;
  model: string;
  contextTokens: number;
  subagentsTotal: number;
  subagentsActive: number;
}

export interface ResumableSession {
  id: string;
  modifiedAtMs: number;
}

export const api = {
  // ---- workspaces ----
  listWorkspaces: () => invoke<Workspace[]>("list_workspaces"),
  addWorkspace: (path: string) => invoke<Workspace>("add_workspace", { path }),
  setWorkspaceCategory: (id: string, category: string | null) =>
    invoke<Workspace>("set_workspace_category", {
      id,
      category: category ?? null,
    }),
  removeWorkspace: (id: string) => invoke<void>("remove_workspace", { id }),
  listWorktreesFor: (id: string) =>
    invoke<WorktreeEntry[]>("list_worktrees_for", { id }),
  createWorktree: (workspaceId: string, name: string, baseBranch?: string) =>
    invoke<Workspace>("create_worktree", {
      workspaceId,
      name,
      baseBranch: baseBranch ?? null,
    }),

  // ---- sessions ----
  listSessions: () => invoke<Session[]>("list_sessions"),
  createSession: (
    workspaceId: string,
    cols: number,
    rows: number,
    useShell = false,
    resumeId?: string,
    isAux = false,
  ) =>
    invoke<Session>("create_session", {
      workspaceId,
      cols,
      rows,
      useShell,
      resumeId: resumeId ?? null,
      isAux,
    }),
  latestClaudeSession: (workspaceId: string) =>
    invoke<ResumableSession | null>("latest_claude_session", { workspaceId }),
  writePty: (sessionId: string, data: string) =>
    invoke<void>("write_pty", { sessionId, data }),
  resizePty: (sessionId: string, cols: number, rows: number) =>
    invoke<void>("resize_pty", { sessionId, cols, rows }),
  closeSession: (sessionId: string) =>
    invoke<void>("close_session", { sessionId }),

  // ---- toolkit ----
  getToolkit: (workspaceId: string) =>
    invoke<ToolkitCommand[]>("get_toolkit", { workspaceId }),
  getToolkitView: (workspaceId: string) =>
    invoke<ToolkitView>("get_toolkit_view", { workspaceId }),
  addCommand: (
    workspaceId: string,
    name: string,
    icon: string,
    kind: CommandKind,
    body: string,
  ) =>
    invoke<ToolkitCommand>("add_command", {
      workspaceId,
      name,
      icon,
      kind,
      body,
    }),
  removeCommand: (workspaceId: string, commandId: string) =>
    invoke<void>("remove_command", { workspaceId, commandId }),
  runCommand: (
    workspaceId: string,
    commandId: string,
    targetSessionId?: string,
  ) =>
    invoke<void>("run_command", {
      workspaceId,
      commandId,
      targetSessionId: targetSessionId ?? null,
    }),
  dispatchAgent: (sessionId: string, agent: string, task?: string) =>
    invoke<void>("dispatch_agent", {
      sessionId,
      agent,
      task: task ?? null,
    }),
};

// Re-export for convenience.
export type { SessionState };
