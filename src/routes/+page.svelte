<script lang="ts">
  import { onMount } from "svelte";
  import "../lib/theme.css";
  import { app } from "../lib/stores/app.svelte";
  import { settings, applyThemeToDocument } from "../lib/stores/settings.svelte";
  import Icon from "../lib/components/Icon.svelte";
  import Sidebar from "../lib/components/Sidebar.svelte";
  import SessionsPanel from "../lib/components/SessionsPanel.svelte";
  import SessionStatusBar from "../lib/components/SessionStatusBar.svelte";
  import Splitter from "../lib/components/Splitter.svelte";
  import Terminal from "../lib/components/Terminal.svelte";
  import ToolkitPanel from "../lib/components/ToolkitPanel.svelte";

  // Resizable column widths. Persisted in localStorage so the layout survives
  // reloads and app restarts. Min/max bounds keep the user from dragging a
  // column to zero (which would trap them with no way to drag back).
  const SIDEBAR_KEY = "cd:sidebar-width";
  const PANEL_KEY = "cd:panel-width";
  const SIDEBAR_DEFAULT = 240;
  const PANEL_DEFAULT = 320;
  const SIDEBAR_MIN = 160;
  const SIDEBAR_MAX = 480;
  const PANEL_MIN = 220;
  const PANEL_MAX = 560;

  function readWidth(key: string, fallback: number): number {
    if (typeof localStorage === "undefined") return fallback;
    const raw = localStorage.getItem(key);
    const n = raw ? parseInt(raw, 10) : NaN;
    return Number.isFinite(n) ? n : fallback;
  }

  let sidebarWidth = $state(readWidth(SIDEBAR_KEY, SIDEBAR_DEFAULT));
  let panelWidth = $state(readWidth(PANEL_KEY, PANEL_DEFAULT));

  function clamp(n: number, lo: number, hi: number) {
    return Math.max(lo, Math.min(hi, n));
  }

  function dragSidebar(dx: number) {
    sidebarWidth = clamp(sidebarWidth + dx, SIDEBAR_MIN, SIDEBAR_MAX);
  }
  function dragPanel(dx: number) {
    panelWidth = clamp(panelWidth + dx, PANEL_MIN, PANEL_MAX);
  }
  function commitSidebar() {
    localStorage.setItem(SIDEBAR_KEY, String(sidebarWidth));
  }
  function commitPanel() {
    localStorage.setItem(PANEL_KEY, String(panelWidth));
  }

  onMount(async () => {
    applyThemeToDocument(settings.theme);
    await app.initDragDropListener();
    await app.refreshWorkspaces();
    await app.autoResumeRecent();
  });

  // Drag-to-reorder tab state. We keep `dragFrom` in component-local state
  // (not the store) because it's pure presentational and resetting on
  // pointer-up doesn't need to round-trip through the store.
  let dragFrom = $state<number | null>(null);
  let dragOver = $state<number | null>(null);

  function onTabDragStart(e: DragEvent, i: number) {
    dragFrom = i;
    if (e.dataTransfer) e.dataTransfer.effectAllowed = "move";
  }
  function onTabDragOver(e: DragEvent, i: number) {
    e.preventDefault();
    dragOver = i;
  }
  function onTabDrop(e: DragEvent, i: number) {
    e.preventDefault();
    if (dragFrom !== null && dragFrom !== i) {
      app.reorderSessions(dragFrom, i);
    }
    dragFrom = null;
    dragOver = null;
  }
  function onTabDragEnd() {
    dragFrom = null;
    dragOver = null;
  }

  // Each session keeps its own xterm instance alive. We render all of them
  // and only set `visible` on the active one — switching tabs is then a
  // CSS toggle, not a remount, preserving scrollback and focus state.
  let activeSession = $derived(
    app.sessions.find((s) => s.id === app.activeSessionId) ?? null,
  );

  function workspaceFor(workspaceId: string) {
    return app.workspaces.find((w) => w.id === workspaceId);
  }

  async function closeActive() {
    if (activeSession) {
      await app.closeSession(activeSession.id);
    }
  }

  // Toolkit panel state. We open it for whichever session is active right now.
  let toolkitOpen = $state(false);
  let activeWorkspace = $derived(
    activeSession ? workspaceFor(activeSession.workspaceId) ?? null : null,
  );

  // Cmd/Ctrl+K toggles the toolkit when there's an active session — common
  // muscle memory from VS Code/Linear and friends.
  function onKeydown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
      if (activeWorkspace) {
        e.preventDefault();
        toolkitOpen = !toolkitOpen;
      }
    } else if (e.key === "Escape" && toolkitOpen) {
      toolkitOpen = false;
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div
  class="shell"
  style="--sidebar-w:{sidebarWidth}px; --panel-w:{panelWidth}px;"
>
  <div class="col sidebar-col">
    <Sidebar />
  </div>
  <Splitter edge="left" onDrag={dragSidebar} onCommit={commitSidebar} />

  <main class="center">
    <div class="tabs">
      {#each app.sessions as session, i (session.id)}
        {@const ws = workspaceFor(session.workspaceId)}
        <button
          class="tab"
          class:active={session.id === app.activeSessionId}
          class:drop-target={dragOver === i && dragFrom !== null && dragFrom !== i}
          draggable="true"
          onclick={() => app.selectSession(session.id)}
          ondragstart={(e) => onTabDragStart(e, i)}
          ondragover={(e) => onTabDragOver(e, i)}
          ondrop={(e) => onTabDrop(e, i)}
          ondragend={onTabDragEnd}
          title={ws?.path ?? ""}
        >
          <span class="tab-name">{ws?.name ?? session.name}</span>
          {#if ws?.branch}
            <span class="tab-branch"><Icon name="gitBranch" size={10} /> {ws.branch}</span>
          {/if}
          <span
            class="tab-close"
            role="button"
            tabindex="0"
            onclick={(e) => {
              e.stopPropagation();
              app.closeSession(session.id);
            }}
            onkeydown={(e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.stopPropagation();
                app.closeSession(session.id);
              }
            }}
            aria-label="Close tab"
          >
            <Icon name="x" size={11} />
          </span>
        </button>
      {/each}

      {#if app.sessions.length === 0}
        <span class="tab-empty">No active sessions</span>
      {/if}

      <button
        class="theme-toggle"
        onclick={() => settings.toggleTheme()}
        title={`Switch to ${settings.theme === "dark" ? "light" : "dark"} theme`}
        aria-label="Toggle theme"
      >
        <Icon name={settings.theme === "dark" ? "sun" : "moon"} size={14} />
      </button>
    </div>

    <div class="terminal-area">
      {#if app.sessions.length === 0}
        <div class="welcome">
          <h2>Welcome to ClaudeDeck</h2>
          <p>
            1. Add a workspace on the left (a project folder, e.g.
            <code>~/Projects/my-app</code>).
          </p>
          <p>2. Click <strong>+ Start session</strong> in the right panel.</p>
          <p>3. Each session runs <code>claude</code> in its own PTY.</p>
          {#if app.error}
            <div class="error">{app.error}</div>
          {/if}
        </div>
      {:else}
        {#each app.sessions as session (session.id)}
          <Terminal
            sessionId={session.id}
            visible={session.id === app.activeSessionId}
          />
        {/each}
      {/if}
    </div>

    {#if activeSession && activeWorkspace}
      <SessionStatusBar
        workspace={activeWorkspace}
        session={activeSession}
        onOpenToolkit={() => (toolkitOpen = true)}
      />
    {/if}

    {#if app.error}
      <div class="status-error">{app.error}</div>
    {/if}
  </main>

  <Splitter edge="right" onDrag={dragPanel} onCommit={commitPanel} />
  <div class="col panel-col">
    <SessionsPanel />
  </div>
</div>

{#if toolkitOpen && activeWorkspace}
  <ToolkitPanel
    workspace={activeWorkspace}
    sessionId={activeSession?.id}
    onClose={() => (toolkitOpen = false)}
  />
{/if}

<style>
  .shell {
    /* Three columns + two splitter rails. The center column eats whatever's
       left after the side columns and splitters take their fixed widths. */
    display: grid;
    grid-template-columns: var(--sidebar-w) auto 1fr auto var(--panel-w);
    height: 100vh;
    width: 100vw;
  }
  .col { min-width: 0; min-height: 0; overflow: hidden; }
  .sidebar-col { border-right: 1px solid var(--border); }
  .panel-col { border-left: 1px solid var(--border); }
  .center { display: flex; flex-direction: column; min-width: 0; }

  .tabs {
    height: 38px;
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0 8px;
    background: var(--surface-1);
    border-bottom: 1px solid var(--border);
    overflow-x: auto;
  }
  .tab {
    display: flex;
    align-items: center;
    gap: 6px;
    font-family: var(--font-mono);
    font-size: 12.5px;
    padding: 6px 10px 6px 12px;
    border-radius: var(--radius-sm);
    color: var(--text-2);
    border: 1px solid transparent;
    white-space: nowrap;
  }
  .tab:hover { background: var(--surface-2); }
  .tab.active {
    background: var(--surface-2);
    color: var(--text-1);
    border-color: var(--border);
  }
  /* Visible drop indicator when dragging tabs to reorder. */
  .tab.drop-target {
    border-left: 2px solid var(--accent);
  }
  .tab[draggable="true"] { cursor: grab; }
  .tab[draggable="true"]:active { cursor: grabbing; }

  .theme-toggle {
    margin-left: auto;
    padding: 4px 10px;
    border-radius: var(--radius-sm);
    font-size: 14px;
    line-height: 1;
    color: var(--text-3);
  }
  .theme-toggle:hover {
    background: var(--surface-2);
    color: var(--text-1);
  }
  .tab-name { font-weight: 500; }
  .tab-branch {
    color: var(--text-3);
    font-size: 11px;
    display: inline-flex;
    align-items: center;
    gap: 3px;
  }
  .tab-close {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    border-radius: 3px;
    color: var(--text-3);
    font-size: 13px;
    line-height: 1;
    cursor: pointer;
  }
  .tab-close:hover { background: var(--surface-3); color: var(--text-1); }
  .tab-empty {
    color: var(--text-3);
    font-size: 12px;
    font-family: var(--font-mono);
    padding: 0 6px;
  }


  .terminal-area {
    flex: 1;
    position: relative;
    background: var(--bg);
    overflow: hidden;
  }

  .welcome {
    padding: 48px;
    color: var(--text-2);
    max-width: 560px;
  }
  .welcome h2 {
    font-family: var(--font-mono);
    font-size: 18px;
    color: var(--text-1);
    margin-bottom: 16px;
  }
  .welcome p { line-height: 1.6; margin: 8px 0; }
  .welcome code {
    font-family: var(--font-mono);
    background: var(--surface-2);
    padding: 1px 6px;
    border-radius: 3px;
    font-size: 12px;
  }

  .error, .status-error {
    margin-top: 16px;
    padding: 10px 12px;
    background: rgba(227, 93, 106, 0.1);
    border: 1px solid var(--red);
    border-radius: var(--radius-sm);
    color: var(--red);
    font-size: 12px;
  }
  .status-error {
    margin: 0;
    border-left: 0;
    border-right: 0;
    border-radius: 0;
  }
</style>
