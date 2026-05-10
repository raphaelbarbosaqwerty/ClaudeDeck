<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "../api";
  import { app } from "../stores/app.svelte";
  import Icon from "./Icon.svelte";
  import type {
    DiscoveredAgent,
    PipelineSignal,
    Session,
    ToolkitView,
    Workspace,
  } from "../types";

  type Props = {
    workspace: Workspace;
    session: Session;
    onOpenToolkit: () => void;
  };

  let { workspace, session, onOpenToolkit }: Props = $props();

  let view = $state<ToolkitView | null>(null);
  let busyAgent = $state<string | null>(null);

  // Refresh strategy:
  //   * On every workspace/session change — instant.
  //   * Plus a soft poll every 8s while mounted, so pipeline signals (commit
  //     pushed, seeds appended) reflect within seconds without thrashing git.
  async function refresh() {
    try {
      view = await api.getToolkitView(workspace.id);
    } catch {
      view = null;
    }
  }

  onMount(() => {
    refresh();
    const t = window.setInterval(refresh, 8000);
    return () => clearInterval(t);
  });

  $effect(() => {
    // Touch the workspace id so this re-runs when the active session
    // switches to a different workspace.
    void workspace.id;
    refresh();
  });

  // Top agents to surface as quick-pills. We prioritize the ones the user
  // is statistically most likely to invoke from a status bar — the "do work"
  // and "review" agents over meta agents like learner-from-pr.
  const PRIORITY = [
    "senior-dev",
    "qa-tester",
    "code-reviewer",
    "pr-reviewer",
    "open-pr",
    "pre-open-pr",
    "learner-from-pr",
  ];

  let quickAgents = $derived.by<DiscoveredAgent[]>(() => {
    const all = view?.discovery.agents ?? [];
    if (all.length === 0) return [];
    const score = (a: DiscoveredAgent) => {
      const exact = PRIORITY.indexOf(a.name);
      if (exact >= 0) return exact;
      // Partial match: "senior-dev-2" hits "senior-dev"
      const part = PRIORITY.findIndex((p) => a.name.includes(p));
      return part >= 0 ? part + 0.5 : 999;
    };
    return [...all].sort((a, b) => score(a) - score(b)).slice(0, 5);
  });

  async function dispatchAgent(agentName: string) {
    busyAgent = agentName;
    try {
      await api.dispatchAgent(session.id, agentName);
    } catch (e) {
      app.error = String(e);
    } finally {
      setTimeout(() => {
        if (busyAgent === agentName) busyAgent = null;
      }, 250);
    }
  }

  async function runPipeline(p: PipelineSignal) {
    if (p.suggestedAgent) {
      await dispatchAgent(p.suggestedAgent);
    } else if (p.suggestedPrompt) {
      await api.writePty(session.id, p.suggestedPrompt + "\n");
    }
    // After running, refresh so the signal disappears once the action
    // changes underlying state (e.g., commits pushed → no longer unpushed).
    setTimeout(refresh, 500);
  }

  let hasAnything = $derived(
    !!view &&
      (view.discovery.pipeline.length > 0 ||
        view.discovery.agents.length > 0 ||
        view.discovery.hasClaudeDir),
  );
</script>

{#if view}
  <div class="bar" class:dim={!hasAnything}>
    <!-- Left: branch / workspace context. Always visible. -->
    <div class="ctx">
      {#if workspace.branch}
        <span class="branch" title={workspace.path}>
          <Icon name="gitBranch" size={11} /> {workspace.branch}
        </span>
      {:else}
        <span class="branch dim" title={workspace.path}>{workspace.name}</span>
      {/if}
      {#if view.discovery.hasClaudeDir}
        <span class="badge" title=".claude/ folder detected">.claude</span>
      {/if}
    </div>

    <!-- Pipeline alerts: clickable, dispatches the suggested action directly.
         Empty when there's no actionable state, which keeps the bar quiet
         99% of the time. -->
    {#if view.discovery.pipeline.length > 0}
      <div class="pipeline">
        {#each view.discovery.pipeline as p (p.id)}
          <button
            class="signal"
            onclick={() => runPipeline(p)}
            title={p.suggestedAgent ? `Dispatch @${p.suggestedAgent}` : "Run"}
          >
            <Icon name={p.icon} size={12} />
            <span class="label">{p.label}</span>
          </button>
        {/each}
      </div>
    {/if}

    <div class="spacer"></div>

    <!-- Quick-access agents: top 5 from .claude/agents/. Hover shows name. -->
    {#if quickAgents.length > 0}
      <div class="agents">
        {#each quickAgents as a (a.name)}
          <button
            class="agent-pill"
            class:busy={busyAgent === a.name}
            onclick={() => dispatchAgent(a.name)}
            title={`@${a.name}${a.description ? " — " + a.description : ""}`}
            aria-label={`Dispatch @${a.name}`}
          >
            <Icon name={a.icon} size={14} />
          </button>
        {/each}
      </div>
    {/if}

    <!-- Right: toolkit opener with kbd hint. -->
    <button class="toolkit-btn" onclick={onOpenToolkit} title="Open toolkit (⌘K)">
      <Icon name="toolbox" size={13} />
      <span class="label">Toolkit</span>
      <span class="kbd">⌘K</span>
    </button>
  </div>
{/if}

<style>
  .bar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 10px;
    height: 30px;
    background: var(--surface-1);
    border-top: 1px solid var(--border);
    font-size: 11px;
    color: var(--text-2);
    overflow: hidden;
    user-select: none;
  }
  .bar.dim { opacity: 0.85; }

  .ctx {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }
  .branch {
    font-family: var(--font-mono);
    color: var(--text-1);
    font-size: 11px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 240px;
  }
  .branch.dim { color: var(--text-3); }

  .badge {
    background: rgba(95, 191, 111, 0.15);
    color: var(--green);
    border: 1px solid rgba(95, 191, 111, 0.3);
    border-radius: 3px;
    padding: 1px 5px;
    font-size: 9px;
    letter-spacing: 0.05em;
    font-family: var(--font-mono);
  }

  .pipeline {
    display: flex;
    align-items: center;
    gap: 4px;
    overflow-x: auto;
    flex-shrink: 1;
  }
  .pipeline::-webkit-scrollbar { display: none; }

  .signal {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 3px 8px;
    background: rgba(255, 140, 66, 0.12);
    border: 1px solid rgba(255, 140, 66, 0.35);
    border-radius: 12px;
    color: var(--text-1);
    font-size: 10px;
    font-family: var(--font-mono);
    transition: background 100ms, border-color 100ms;
    white-space: nowrap;
    flex-shrink: 0;
  }
  .signal:hover {
    background: rgba(255, 140, 66, 0.22);
    border-color: var(--accent);
  }
  .signal .icon { font-size: 11px; }

  .spacer { flex: 1; min-width: 8px; }

  .agents {
    display: flex;
    align-items: center;
    gap: 3px;
    flex-shrink: 0;
  }
  .agent-pill {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 22px;
    border-radius: 4px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    font-size: 13px;
    transition: background 100ms, border-color 100ms, transform 100ms;
  }
  .agent-pill:hover {
    background: var(--surface-3);
    border-color: var(--accent);
    transform: translateY(-1px);
  }
  .agent-pill.busy {
    opacity: 0.5;
    transform: scale(0.95);
  }

  .toolkit-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 3px 10px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-1);
    font-size: 11px;
    flex-shrink: 0;
    transition: background 100ms, border-color 100ms;
  }
  .toolkit-btn:hover {
    background: var(--surface-3);
    border-color: var(--accent);
  }
  .toolkit-btn .icon { font-size: 13px; }
  .toolkit-btn .label { font-weight: 500; }
  .toolkit-btn .kbd {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--text-3);
    background: var(--bg);
    padding: 1px 5px;
    border-radius: 3px;
    border: 1px solid var(--border);
  }
</style>
