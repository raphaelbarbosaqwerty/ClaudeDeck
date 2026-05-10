<script lang="ts">
  import { onMount } from "svelte";
  import { app } from "../stores/app.svelte";
  import { api, type ResumableSession } from "../api";
  import type { Session, Workspace } from "../types";
  import { avatarDataUri } from "../utils/avatar";
  import Icon from "./Icon.svelte";
  import SessionCard from "./SessionCard.svelte";

  // Persist which category sections the user has collapsed. Keyed by the
  // category name (case-preserved) so renaming a category resets the state.
  const COLLAPSED_KEY = "cd:collapsed-categories";
  const UNCATEGORIZED = "Uncategorized";

  function loadCollapsed(): Set<string> {
    if (typeof localStorage === "undefined") return new Set();
    try {
      const raw = localStorage.getItem(COLLAPSED_KEY);
      if (!raw) return new Set();
      return new Set(JSON.parse(raw));
    } catch {
      return new Set();
    }
  }

  let collapsed = $state<Set<string>>(loadCollapsed());

  function toggleCollapsed(cat: string) {
    const next = new Set(collapsed);
    if (next.has(cat)) next.delete(cat);
    else next.add(cat);
    collapsed = next;
    localStorage.setItem(COLLAPSED_KEY, JSON.stringify([...next]));
  }

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

  /// Categories collected for rendering. Each section holds the groups
  /// that share that category. The "Uncategorized" bucket is always last
  /// even when alphabetical order would put it elsewhere — it's a visual
  /// "inbox" the user knows where to find.
  type Section = {
    category: string;
    groups: Group[];
    /// Total live sessions in this section — surfaced in the header.
    liveCount: number;
  };

  let groups = $derived.by<Group[]>(() => {
    const sessionsByWorkspace = new Map<string, Session[]>();
    // Aux shell sessions are tab-local utilities and never appear in the
    // right-hand Sessions panel — exclude them from grouping.
    for (const s of app.sessions) {
      if (s.isAux) continue;
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

    // Float groups with live sessions to the top WITHIN each category, so
    // the user's active work never gets buried among placeholder/resume
    // cards. A group is "live" when its project workspace has at least
    // one running session OR any of its worktree children does.
    const isLive = (g: Group) =>
      g.rootSessions.length > 0 ||
      g.children.some((c) => c.sessions.length > 0);

    out.sort((a, b) => {
      const liveDiff = Number(isLive(b)) - Number(isLive(a));
      if (liveDiff !== 0) return liveDiff;
      return a.project.name.localeCompare(b.project.name);
    });
    return out;
  });

  /// Top-level structure rendered by the panel: groups bucketed by
  /// category, with "Uncategorized" pinned at the bottom.
  let sections = $derived.by<Section[]>(() => {
    const buckets = new Map<string, Group[]>();
    for (const g of groups) {
      const cat = g.project.category?.trim() || UNCATEGORIZED;
      const list = buckets.get(cat) ?? [];
      list.push(g);
      buckets.set(cat, list);
    }

    const isLive = (g: Group) =>
      g.rootSessions.length > 0 ||
      g.children.some((c) => c.sessions.length > 0);

    const out: Section[] = [];
    for (const [category, gs] of buckets) {
      out.push({
        category,
        groups: gs,
        liveCount: gs.filter(isLive).length,
      });
    }

    // Sort: real categories first (alpha, but live sections to the top),
    // then "Uncategorized" pinned last regardless.
    out.sort((a, b) => {
      const aIsUncat = a.category === UNCATEGORIZED;
      const bIsUncat = b.category === UNCATEGORIZED;
      if (aIsUncat !== bIsUncat) return aIsUncat ? 1 : -1;
      const liveDiff = Number(b.liveCount > 0) - Number(a.liveCount > 0);
      if (liveDiff !== 0) return liveDiff;
      return a.category.localeCompare(b.category);
    });
    return out;
  });

  let totalCost = $derived(
    app.sessions.filter((s) => !s.isAux).reduce((acc, s) => acc + s.cost, 0),
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

    {#each sections as section (section.category)}
      {@const isCollapsed = collapsed.has(section.category)}
      <div class="section" class:collapsed={isCollapsed}>
        <button
          class="section-header"
          onclick={() => toggleCollapsed(section.category)}
          aria-expanded={!isCollapsed}
        >
          <span class="caret" class:rotated={!isCollapsed}>
            <Icon name="caretRight" size={10} />
          </span>
          <span class="section-name">{section.category}</span>
          <span class="section-count">{section.groups.length}</span>
          {#if section.liveCount > 0}
            <span class="section-live">●</span>
          {/if}
        </button>

        {#if !isCollapsed}
          <div class="section-body">

    {#each section.groups as group (group.project.id)}
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
        {/if}
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

  .section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .section-header {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    text-align: left;
    padding: 4px 4px 4px 0;
    background: transparent;
    border: 0;
    border-radius: var(--radius-sm);
    font-family: var(--font-mono);
    font-size: 10px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--text-3);
    cursor: pointer;
  }
  .section-header:hover { color: var(--text-2); }
  .caret {
    display: inline-flex;
    transition: transform 140ms ease;
  }
  .caret.rotated { transform: rotate(90deg); }
  .section-name { flex: 1; }
  .section-count {
    background: var(--surface-2);
    color: var(--text-3);
    border-radius: 9px;
    padding: 1px 7px;
    font-size: 9px;
    letter-spacing: 0;
  }
  .section-live {
    color: var(--green);
    font-size: 10px;
    line-height: 1;
    /* Soft pulse mirrors the avatar's "generating" pulse so the user
       links the section header to the activity inside. */
    animation: cd-section-live 1.6s ease-in-out infinite;
  }
  @keyframes cd-section-live {
    0%, 100% { opacity: 0.6; }
    50%      { opacity: 1; }
  }
  .section-body {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding-top: 2px;
  }

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
