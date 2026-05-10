<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "../api";
  import { app } from "../stores/app.svelte";
  import Icon from "./Icon.svelte";
  import type {
    CommandKind,
    DiscoveredAgent,
    PipelineSignal,
    ToolkitCommand,
    ToolkitView,
    Workspace,
  } from "../types";

  type Props = {
    workspace: Workspace;
    /// Active session id in this workspace, used as the target for `prompt` /
    /// `agent` kinds. Required for those commands to do anything.
    sessionId?: string;
    onClose: () => void;
  };

  let { workspace, sessionId, onClose }: Props = $props();

  let view = $state<ToolkitView | null>(null);
  let loading = $state(true);
  let busyId = $state<string | null>(null);

  // ---- create form state ----
  let showCreate = $state(false);
  let newName = $state("");
  let newIcon = $state("⚡");
  let newKind = $state<CommandKind>("shell");
  let newBody = $state("");

  // ---- agent task modal state ----
  // When the user clicks an agent button, we show a small inline prompt
  // asking what the task is. They can submit empty to dispatch the agent
  // with no task description (some agents don't need one).
  let agentTaskFor = $state<DiscoveredAgent | null>(null);
  let agentTask = $state("");

  async function refresh() {
    loading = true;
    try {
      view = await api.getToolkitView(workspace.id);
    } catch (e) {
      app.error = String(e);
    } finally {
      loading = false;
    }
  }

  onMount(refresh);

  async function runCustom(c: ToolkitCommand) {
    if (!ensureSessionForKind(c.kind)) return;
    busyId = c.id;
    try {
      await api.runCommand(
        workspace.id,
        c.id,
        c.kind === "prompt" || c.kind === "agent" ? sessionId : undefined,
      );
    } catch (e) {
      app.error = String(e);
    } finally {
      setTimeout(() => {
        if (busyId === c.id) busyId = null;
      }, 200);
    }
  }

  async function runAgentDispatch(agent: string, task?: string) {
    if (!sessionId) {
      app.error = "Start a Claude session in this workspace first.";
      return;
    }
    busyId = `agent:${agent}`;
    try {
      await api.dispatchAgent(sessionId, agent, task);
      onClose();
    } catch (e) {
      app.error = String(e);
    } finally {
      setTimeout(() => {
        if (busyId === `agent:${agent}`) busyId = null;
      }, 200);
    }
  }

  async function runPipeline(p: PipelineSignal) {
    if (p.suggestedAgent) {
      await runAgentDispatch(p.suggestedAgent);
    } else if (p.suggestedPrompt && sessionId) {
      // No persisted command — just write the suggested prompt directly.
      await api.writePty(sessionId, p.suggestedPrompt + "\n");
      onClose();
    }
  }

  function ensureSessionForKind(kind: CommandKind): boolean {
    if ((kind === "prompt" || kind === "agent") && !sessionId) {
      app.error =
        "Start a Claude session in this workspace before running this command.";
      return false;
    }
    return true;
  }

  async function remove(c: ToolkitCommand, ev: Event) {
    ev.stopPropagation();
    if (!confirm(`Remove "${c.name}" from this toolkit?`)) return;
    await api.removeCommand(workspace.id, c.id);
    refresh();
  }

  async function create(e: Event) {
    e.preventDefault();
    if (!newName.trim() || !newBody.trim()) return;
    await api.addCommand(
      workspace.id,
      newName.trim(),
      newIcon || "⚡",
      newKind,
      newBody.trim(),
    );
    newName = "";
    newBody = "";
    newIcon = "⚡";
    showCreate = false;
    refresh();
  }

  function startAgentDialog(a: DiscoveredAgent) {
    agentTaskFor = a;
    agentTask = "";
  }

  async function submitAgentDialog(e: Event) {
    e.preventDefault();
    const a = agentTaskFor;
    if (!a) return;
    agentTaskFor = null;
    await runAgentDispatch(a.name, agentTask);
    agentTask = "";
  }

  const KIND_LABEL: Record<CommandKind, string> = {
    shell: "Shell",
    prompt: "Prompt",
    url: "URL",
    agent: "Agent",
  };
</script>

<div
  class="overlay"
  role="dialog"
  aria-modal="true"
  aria-label="Toolkit"
  tabindex="-1"
  onclick={(e) => {
    if (e.target === e.currentTarget) onClose();
  }}
  onkeydown={(e) => {
    if (e.key === "Escape") onClose();
  }}
>
  <div class="panel" role="document">
    <header>
      <span class="title">
        <Icon name="toolbox" size={14} /> Toolkit
      </span>
      <span class="crumb">
        {workspace.name}{#if workspace.branch} · <Icon name="gitBranch" size={10} /> {workspace.branch}{/if}
        {#if view?.discovery?.hasClaudeDir}
          <span class="badge" title="Found .claude/ folder in this workspace">
            .claude ✓
          </span>
        {/if}
      </span>
      <button class="close" onclick={onClose} aria-label="Close">
        <Icon name="x" size={14} />
      </button>
    </header>

    {#if loading}
      <div class="empty">Loading…</div>
    {:else if view}
      <div class="scroll">
        <!-- PIPELINE: state-aware — only renders when the workspace is in a
             specific actionable state (unpushed commits, pending seeds). -->
        {#if view.discovery.pipeline.length > 0}
          <section>
            <div class="section-label">PIPELINE</div>
            <div class="list">
              {#each view.discovery.pipeline as p (p.id)}
                <button
                  class="signal"
                  onclick={() => runPipeline(p)}
                  disabled={busyId !== null}
                >
                  <Icon name={p.icon} size={16} />
                  <span class="signal-body">
                    <span class="name">{p.label}</span>
                    {#if p.suggestedAgent}
                      <span class="hint">→ dispatches @{p.suggestedAgent}</span>
                    {/if}
                  </span>
                </button>
              {/each}
            </div>
          </section>
        {/if}

        <!-- AGENTS: auto-discovered from .claude/agents/. Empty when the
             workspace has no .claude/, which is fine. -->
        {#if view.discovery.agents.length > 0}
          <section>
            <div class="section-label">
              YOUR AGENTS
              <span class="section-sub">from .claude/agents/</span>
            </div>
            <div class="grid">
              {#each view.discovery.agents as a (a.name)}
                <button
                  class="card"
                  class:busy={busyId === `agent:${a.name}`}
                  onclick={() => startAgentDialog(a)}
                  disabled={!sessionId}
                  title={a.description ?? `Dispatch @${a.name}`}
                >
                  <span class="icon"><Icon name={a.icon} size={16} /></span>
                  <span class="name">{a.name}</span>
                  <span class="kind">AGENT</span>
                </button>
              {/each}
            </div>
            {#if !sessionId}
              <div class="agents-locked">
                Start a Claude session to dispatch agents.
              </div>
            {/if}
          </section>
        {/if}

        <!-- CUSTOM: persisted user commands -->
        <section>
          <div class="section-label">
            COMMANDS
            <span class="section-sub">your custom toolkit</span>
          </div>
          {#if view.commands.length === 0}
            <div class="empty small">No custom commands yet.</div>
          {:else}
            <div class="grid">
              {#each view.commands as c (c.id)}
                <button
                  class="card"
                  class:busy={busyId === c.id}
                  onclick={() => runCustom(c)}
                  title={KIND_LABEL[c.kind]}
                >
                  <span class="icon"><Icon name={c.icon} size={16} /></span>
                  <span class="name">{c.name}</span>
                  <span class="kind">{c.kind}</span>
                  <span
                    class="remove"
                    role="button"
                    tabindex="0"
                    onclick={(e) => remove(c, e)}
                    onkeydown={(e) => {
                      if (e.key === "Enter" || e.key === " ") remove(c, e);
                    }}
                    aria-label="Remove"
                  >
                    <Icon name="x" size={11} />
                  </span>
                </button>
              {/each}
            </div>
          {/if}
        </section>

        {#if !view.discovery.hasClaudeDir}
          <section>
            <div class="empty small">
              No <code>.claude/</code> folder found in this workspace.
              Auto-discovered agents and pipeline signals will appear here once
              you set up your team's <code>.claude/agents/</code> tree.
            </div>
          </section>
        {/if}
      </div>
    {/if}

    {#if showCreate}
      <form class="create-form" onsubmit={create}>
        <div class="row">
          <input
            class="icon-input"
            placeholder="emoji or hammer/plus/etc"
            bind:value={newIcon}
            title="Emoji or Phosphor icon name (e.g. hammer, plus, gitBranch)"
          />
          <input class="name-input" placeholder="Command name" bind:value={newName} required />
          <select bind:value={newKind}>
            <option value="shell">Shell</option>
            <option value="prompt">Prompt</option>
            <option value="url">URL</option>
            <option value="agent">Agent</option>
          </select>
        </div>
        <textarea
          rows="3"
          placeholder={
            newKind === "shell"
              ? 'e.g.  npm run dev'
              : newKind === "url"
                ? 'e.g.  https://github.com/owner/repo'
                : newKind === "agent"
                  ? 'e.g.  senior-dev'
                  : 'e.g.  Review the diff and open a PR.'
          }
          bind:value={newBody}
          required
        ></textarea>
        <div class="form-actions">
          <button type="button" onclick={() => (showCreate = false)}>Cancel</button>
          <button type="submit" class="primary">Add</button>
        </div>
      </form>
    {:else}
      <footer>
        <button onclick={() => (showCreate = true)}>＋ Create command</button>
      </footer>
    {/if}
  </div>
</div>

<!-- Agent task dialog. Lives outside the panel so it visually layers above
     the toolkit while the user types their description. -->
{#if agentTaskFor}
  <div class="overlay nested" role="dialog" aria-modal="true">
    <form class="agent-dialog" onsubmit={submitAgentDialog}>
      <header>
        <span class="title">
          <Icon name={agentTaskFor.icon} size={14} /> @{agentTaskFor.name}
        </span>
        <button
          type="button"
          class="close"
          onclick={() => (agentTaskFor = null)}
          aria-label="Cancel"
        >
          <Icon name="x" size={14} />
        </button>
      </header>
      {#if agentTaskFor.description}
        <p class="agent-desc">{agentTaskFor.description}</p>
      {/if}
      <textarea
        rows="3"
        placeholder="What should this agent do? (leave empty to dispatch with no task)"
        bind:value={agentTask}
      ></textarea>
      <div class="form-actions">
        <button type="button" onclick={() => (agentTaskFor = null)}>Cancel</button>
        <button type="submit" class="primary">Dispatch</button>
      </div>
    </form>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.45);
    backdrop-filter: blur(4px);
    z-index: 50;
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding: 60px 24px;
  }
  .overlay.nested { z-index: 60; align-items: center; }

  .panel {
    width: 620px;
    max-width: 100%;
    max-height: calc(100vh - 120px);
    background: var(--surface-1);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: 0 16px 48px rgba(0, 0, 0, 0.5);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  header {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 14px;
    border-bottom: 1px solid var(--border);
    background: var(--surface-2);
  }
  .title { font-weight: 600; font-size: 13px; }
  .crumb {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-3);
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .badge {
    background: rgba(95, 191, 111, 0.15);
    color: var(--green);
    border: 1px solid rgba(95, 191, 111, 0.3);
    border-radius: 3px;
    padding: 1px 6px;
    font-size: 9px;
    letter-spacing: 0.05em;
  }
  .close {
    width: 24px; height: 24px;
    border-radius: 4px;
    color: var(--text-3);
    font-size: 18px; line-height: 1;
  }
  .close:hover { background: var(--surface-3); color: var(--text-1); }

  .scroll { overflow-y: auto; padding: 4px 0 8px; }
  section { padding: 10px 14px; border-bottom: 1px dashed var(--border); }
  section:last-child { border-bottom: 0; }

  .section-label {
    font-family: var(--font-mono);
    font-size: 10px;
    letter-spacing: 0.12em;
    color: var(--text-3);
    margin-bottom: 8px;
    display: flex;
    align-items: baseline;
    gap: 8px;
  }
  .section-sub {
    text-transform: none;
    letter-spacing: 0;
    font-size: 10px;
    color: var(--text-3);
    opacity: 0.7;
  }

  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }

  .card {
    position: relative;
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: 8px;
    text-align: left;
    padding: 10px 12px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    transition: border-color 120ms, background 120ms, opacity 120ms;
  }
  .card:hover:not(:disabled) {
    background: var(--surface-3);
    border-color: var(--accent);
  }
  .card:disabled { opacity: 0.5; cursor: not-allowed; }
  .card.busy { opacity: 0.6; }

  .icon { font-size: 16px; }
  .name {
    font-size: 12px;
    color: var(--text-1);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .kind {
    font-family: var(--font-mono);
    font-size: 9px;
    color: var(--text-3);
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }
  .remove {
    position: absolute;
    top: 4px; right: 6px;
    width: 16px; height: 16px;
    border-radius: 3px;
    display: flex; align-items: center; justify-content: center;
    color: var(--text-3);
    font-size: 12px;
    opacity: 0;
    transition: opacity 100ms;
    cursor: pointer;
  }
  .card:hover .remove { opacity: 1; }
  .remove:hover { background: rgba(227, 93, 106, 0.15); color: var(--red); }

  /* Pipeline signals — wider single-column row, more contrast (it's a CTA) */
  .list { display: flex; flex-direction: column; gap: 6px; }
  .signal {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 10px;
    align-items: center;
    text-align: left;
    padding: 10px 12px;
    background: rgba(255, 140, 66, 0.08);
    border: 1px solid rgba(255, 140, 66, 0.3);
    border-radius: var(--radius-md);
    transition: background 120ms, border-color 120ms;
  }
  .signal:hover { background: rgba(255, 140, 66, 0.16); border-color: var(--accent); }
  .signal-body { display: flex; flex-direction: column; gap: 2px; }
  .signal .name { font-size: 12px; color: var(--text-1); }
  .hint { font-size: 10px; color: var(--text-3); font-family: var(--font-mono); }

  .empty {
    padding: 16px;
    text-align: center;
    color: var(--text-3);
    font-style: italic;
    font-size: 12px;
  }
  .empty.small { padding: 8px 12px; font-size: 11px; }
  .empty code {
    font-family: var(--font-mono);
    background: var(--surface-2);
    padding: 1px 4px;
    border-radius: 3px;
    font-style: normal;
  }
  .agents-locked {
    margin-top: 6px;
    font-size: 10px;
    color: var(--text-3);
    font-style: italic;
  }

  footer {
    padding: 10px 14px;
    border-top: 1px solid var(--border);
    display: flex;
    justify-content: flex-end;
  }
  footer button {
    padding: 6px 12px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    font-size: 11px;
    color: var(--text-2);
  }
  footer button:hover { color: var(--accent); border-color: var(--accent); }

  .create-form, .agent-dialog {
    padding: 12px 14px;
    border-top: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .agent-dialog {
    width: 480px; max-width: 100%;
    background: var(--surface-1);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: 0 16px 48px rgba(0,0,0,0.5);
    overflow: hidden;
    border-top: 1px solid var(--border);
    padding: 0;
  }
  .agent-dialog header { padding: 12px 14px; }
  .agent-dialog .agent-desc {
    margin: 0;
    padding: 0 14px;
    color: var(--text-2);
    font-size: 12px;
    line-height: 1.5;
  }
  .agent-dialog textarea { margin: 0 14px; }
  .agent-dialog .form-actions { padding: 0 14px 14px; }

  .row { display: flex; gap: 6px; align-items: center; }
  .row .icon-input { width: 44px; text-align: center; font-size: 14px; }
  .row .name-input { flex: 1; min-width: 0; }
  .row select {
    background: var(--surface-2);
    color: var(--text-1);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 6px 8px;
    font-size: 12px;
  }
  textarea {
    font: inherit;
    background: var(--surface-2);
    color: var(--text-1);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 8px 10px;
    resize: vertical;
    font-size: 12px;
    font-family: var(--font-mono);
  }
  textarea:focus { border-color: var(--accent); outline: none; }
  textarea::placeholder { color: var(--text-3); font-style: italic; opacity: 0.7; }
  .form-actions { display: flex; justify-content: flex-end; gap: 6px; }
  .form-actions button {
    padding: 6px 12px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    font-size: 11px;
    color: var(--text-2);
  }
  .form-actions button.primary {
    background: var(--accent);
    color: #0e0e10;
    border-color: var(--accent);
  }
</style>
