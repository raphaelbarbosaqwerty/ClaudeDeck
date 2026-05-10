<script lang="ts">
  import { onMount } from "svelte";
  import { app } from "../stores/app.svelte";
  import { api, type ResumableSession } from "../api";
  import type { Session, Workspace } from "../types";
  import { avatarDataUri } from "../utils/avatar";
  import SessionCard from "./SessionCard.svelte";

  function avatarFor(ws: Workspace, parentId?: string): string {
    return avatarDataUri(ws.id, { parentId, size: 32 });
  }

  // Resumable session per workspace, looked up via the backend (which scans
  // ~/.claude/projects/<cwd>/*.jsonl). We re-poll lightly so a session that
  // was just closed becomes resumable on the next render.
  let resumable = $state<Record<string, ResumableSession | null>>({});

  async function refreshResumable() {
    const updates: Record<string, ResumableSession | null> = {};
    for (const ws of app.workspaces) {
      try {
        updates[ws.id] = await api.latestClaudeSession(ws.id);
      } catch {
        updates[ws.id] = null;
      }
    }
    resumable = updates;
  }

  onMount(() => {
    refreshResumable();
    const t = window.setInterval(refreshResumable, 5000);
    return () => clearInterval(t);
  });

  // Re-fetch when the workspace list changes (added/removed) so new entries
  // immediately know whether they have a resumable session.
  $effect(() => {
    // Touch the array length so Svelte tracks it.
    void app.workspaces.length;
    refreshResumable();
  });

  function ago(ms: number): string {
    const diff = Date.now() - ms;
    const m = Math.floor(diff / 60_000);
    if (m < 1) return "just now";
    if (m < 60) return `${m}m ago`;
    const h = Math.floor(m / 60);
    if (h < 24) return `${h}h ago`;
    const d = Math.floor(h / 24);
    return `${d}d ago`;
  }

  type Group = {
    project: Workspace;
    rootSessions: Session[];
    children: Array<{ workspace: Workspace; sessions: Session[] }>;
  };

  let groups = $derived.by<Group[]>(() => {
    const sessionsByWorkspace = new Map<string, Session[]>();
    for (const s of app.sessions) {
      const list = sessionsByWorkspace.get(s.workspaceId) ?? [];
      list.push(s);
      sessionsByWorkspace.set(s.workspaceId, list);
    }

    const out: Group[] = [];
    for (const ws of app.workspaces) {
      if (ws.kind !== "project") continue;
      const children: Group["children"] = [];
      for (const w of app.workspaces) {
        if (w.kind === "worktree" && w.parentId === ws.id) {
          children.push({
            workspace: w,
            sessions: sessionsByWorkspace.get(w.id) ?? [],
          });
        }
      }
      out.push({
        project: ws,
        rootSessions: sessionsByWorkspace.get(ws.id) ?? [],
        children,
      });
    }
    out.sort((a, b) => a.project.name.localeCompare(b.project.name));
    return out;
  });

  let totalCost = $derived(
    app.sessions.reduce((acc, s) => acc + s.cost, 0),
  );
</script>

<aside class="panel">
  <header>
    <div class="title">SESSIONS</div>
    <div class="total">${totalCost.toFixed(2)}</div>
  </header>

  <div class="list">
    {#if groups.length === 0}
      <div class="empty">
        Add a workspace on the left, then start a session here.
      </div>
    {/if}

    {#each groups as group (group.project.id)}
      <div class="group">
        {#if group.rootSessions.length === 0}
          {@const resume = resumable[group.project.id]}
          <div class="placeholder-row">
            <button
              class="placeholder"
              onclick={() => app.startSessionFor(group.project.id)}
              title="Start a fresh Claude session"
            >
              <span
                class="avatar"
                style="background-image: url('{avatarFor(group.project)}');"
              ></span>
              <div class="body">
                <span class="name">{group.project.name}</span>
                <span class="hint">+ New session</span>
              </div>
            </button>
            {#if resume}
              <button
                class="resume-btn"
                title="Continue the previous Claude session in this workspace"
                onclick={() =>
                  app.startSessionFor(group.project.id, { resumeId: resume.id })}
              >
                ↻ Resume
                <span class="resume-when">{ago(resume.modifiedAtMs)}</span>
              </button>
            {/if}
          </div>
        {/if}

        {#each group.rootSessions as session (session.id)}
          <SessionCard
            {session}
            workspace={group.project}
            active={app.activeSessionId === session.id}
            onclick={() => app.selectSession(session.id)}
          />
        {/each}

        {#each group.children as child, i (child.workspace.id)}
          {@const isLast = i === group.children.length - 1}
          {#if child.sessions.length === 0}
            {@const resume = resumable[child.workspace.id]}
            <div class="placeholder-row child" class:last={isLast}>
              <button
                class="placeholder child"
                class:last={isLast}
                onclick={() => app.startSessionFor(child.workspace.id)}
              >
                <span class="connector" aria-hidden="true"></span>
                <span
                  class="avatar small"
                  style="background-image: url('{avatarFor(child.workspace, group.project.id)}');"
                ></span>
                <div class="body">
                  <span class="name">{child.workspace.name}</span>
                  <span class="hint">+ New session</span>
                </div>
              </button>
              {#if resume}
                <button
                  class="resume-btn small"
                  title="Resume previous session"
                  onclick={() =>
                    app.startSessionFor(child.workspace.id, { resumeId: resume.id })}
                >
                  ↻ {ago(resume.modifiedAtMs)}
                </button>
              {/if}
            </div>
          {/if}
          {#each child.sessions as session, j (session.id)}
            <SessionCard
              {session}
              workspace={child.workspace}
              parentWorkspace={group.project}
              isChild
              isLast={isLast && j === child.sessions.length - 1}
              active={app.activeSessionId === session.id}
              onclick={() => app.selectSession(session.id)}
            />
          {/each}
        {/each}
      </div>
    {/each}
  </div>
</aside>

<style>
  .panel {
    width: 100%;
    height: 100%;
    background: var(--surface-1);
    display: flex;
    flex-direction: column;
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 14px 14px 10px;
    border-bottom: 1px solid var(--border);
  }
  .title {
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 0.15em;
    color: var(--text-3);
  }
  .total {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-2);
  }

  .list { flex: 1; overflow-y: auto; padding: 12px; display: flex; flex-direction: column; gap: 10px; }

  .empty {
    color: var(--text-3);
    font-size: 12px;
    text-align: center;
    padding: 24px 12px;
    font-style: italic;
  }

  .group { display: flex; flex-direction: column; gap: 8px; }

  /* Row that pairs the new-session placeholder with the resume affordance. */
  .placeholder-row {
    display: flex;
    align-items: stretch;
    gap: 6px;
  }
  .placeholder-row .placeholder { flex: 1; }
  .placeholder-row.child { margin-left: 0; } /* child placeholder owns its indent */

  .resume-btn {
    flex-shrink: 0;
    padding: 8px 10px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    color: var(--text-2);
    font-size: 11px;
    font-family: var(--font-mono);
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    justify-content: center;
    gap: 2px;
    line-height: 1.2;
    transition: border-color 120ms, color 120ms;
    white-space: nowrap;
  }
  .resume-btn:hover {
    color: var(--accent);
    border-color: var(--accent);
  }
  .resume-btn.small { padding: 6px 8px; font-size: 10px; }
  .resume-when { color: var(--text-3); font-size: 10px; }

  .placeholder {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    text-align: left;
    padding: 10px 12px;
    background: transparent;
    border: 1px dashed var(--border);
    border-radius: var(--radius-md);
    color: var(--text-3);
    transition: border-color 120ms, color 120ms;
    position: relative;
  }
  .placeholder:hover {
    border-color: var(--accent);
    color: var(--text-2);
  }
  .placeholder.child {
    margin-left: 22px;
    width: calc(100% - 22px);
  }
  .placeholder.child .connector {
    position: absolute;
    left: -16px;
    top: -8px;
    bottom: 50%;
    width: 12px;
    border-left: 1px solid var(--border);
    border-bottom: 1px solid var(--border);
    border-bottom-left-radius: 6px;
  }
  .placeholder.child:not(.last) .connector::after {
    content: "";
    position: absolute;
    left: -1px;
    top: 100%;
    bottom: -100%;
    border-left: 1px solid var(--border);
  }

  .avatar {
    width: 32px;
    height: 32px;
    border-radius: 6px;
    background-color: var(--surface-3);
    background-size: contain;
    background-repeat: no-repeat;
    background-position: center;
    image-rendering: pixelated;
    image-rendering: crisp-edges;
    flex-shrink: 0;
  }
  .avatar.small { width: 28px; height: 28px; }

  .body { display: flex; flex-direction: column; gap: 2px; }
  .name { font-size: 13px; color: inherit; }
  .hint { font-size: 11px; color: var(--text-3); }
</style>
