// Mirror of src-tauri/src/model.rs. Keep in sync.

export type WorkspaceKind = "project" | "worktree";

export interface Workspace {
  id: string;
  name: string;
  path: string;
  kind: WorkspaceKind;
  parentId?: string;
  branch?: string;
}

export type SessionState =
  | "idle"
  | "thinking"
  | "generating"
  | "userInput"
  | "needsAttention";

export interface Session {
  id: string;
  workspaceId: string;
  name: string;
  state: SessionState;
  model: string;
  cost: number;
  tokensIn: number;
  tokensOut: number;
  contextTokens: number;
  maxContextTokens: number;
  subagentsTotal: number;
  subagentsActive: number;
}

export type CommandKind = "shell" | "prompt" | "url" | "agent";

export interface ToolkitCommand {
  id: string;
  name: string;
  icon: string;
  kind: CommandKind;
  body: string;
}

export interface DiscoveredAgent {
  name: string;
  description?: string;
  icon: string;
}

export interface PipelineSignal {
  id: string;
  label: string;
  suggestedAgent?: string;
  suggestedPrompt?: string;
  icon: string;
}

export interface WorkspaceDiscovery {
  hasClaudeDir: boolean;
  hasWorkflow: boolean;
  hasSkill: boolean;
  agents: DiscoveredAgent[];
  pipeline: PipelineSignal[];
}

export interface ToolkitView {
  commands: ToolkitCommand[];
  discovery: WorkspaceDiscovery;
}
