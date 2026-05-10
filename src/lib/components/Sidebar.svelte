<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { app } from "../stores/app.svelte";
  import CategoryPicker from "./CategoryPicker.svelte";
  import Icon from "./Icon.svelte";

  // Category picker popover state. Tracks which workspace's tag-button was
  // clicked so the popover anchors next to it. `null` means no popover.
  let categoryFor = $state<string | null>(null);
  let categoryAnchor = $state<DOMRect | undefined>(undefined);

  // Suggestions are computed from the existing categories on workspaces.
  // Deduplicate case-insensitively but preserve the casing of the first
  // occurrence — keeps the user's chosen capitalization stable.
  let categorySuggestions = $derived.by(() => {
    const seen = new Map<string, string>();
    for (const w of app.workspaces) {
      if (w.category && !seen.has(w.category.toLowerCase())) {
        seen.set(w.category.toLowerCase(), w.category);
      }
    }
    return Array.from(seen.values()).sort((a, b) => a.localeCompare(b));
  });

  function openCategoryPicker(wsId: string, ev: MouseEvent) {
    ev.stopPropagation();
    const target = (ev.currentTarget as HTMLElement) ?? null;
    categoryAnchor = target?.getBoundingClientRect();
    categoryFor = wsId;
  }

  function closeCategoryPicker() {
    categoryFor = null;
    categoryAnchor = undefined;
  }

  async function applyCategory(category: string | null) {
    if (!categoryFor) return;
    await app.setWorkspaceCategory(categoryFor, category);
  }

  let newPath = $state("");
  let adding = $state(false);

  async function pickFolder() {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Choose a workspace folder",
      });
      if (typeof selected === "string" && selected.length > 0) {
        newPath = selected;
        // Skip the manual click — adding immediately matches the
        // expectation that "I picked a folder" means "use this folder".
        adding = true;
        await app.addWorkspace(selected);
        if (!app.error) newPath = "";
        adding = false;
      }
    } catch (e) {
      app.error = String(e);
    }
  }

  // Free-text search over workspace names. Empty -> all visible.
  let search = $state("");
  let normalizedSearch = $derived(search.trim().toLowerCase());

  function matchesSearch(w: { name: string; branch?: string }): boolean {
    if (!normalizedSearch) return true;
    return (
      w.name.toLowerCase().includes(normalizedSearch) ||
      (w.branch?.toLowerCase().includes(normalizedSearch) ?? false)
    );
  }

  // Group worktrees recursively by parent. A worktree CAN have another
  // worktree as its parent — that's how stacked PRs end up represented.
  // The grouping function returns descendants of any given workspace id,
  // letting us render arbitrary depth without precomputing the whole tree.
  let projects = $derived(app.workspaces.filter((w) => w.kind === "project"));
  let childrenOf = $derived.by(() => {
    const m = new Map<string, typeof app.workspaces>();
    for (const w of app.workspaces) {
      if (w.parentId) {
        const list = m.get(w.parentId) ?? [];
        list.push(w);
        m.set(w.parentId, list);
      }
    }
    return m;
  });

  // For search, a parent stays visible if any descendant matches —
  // otherwise filtering out a parent would hide its still-relevant children.
  function subtreeMatches(id: string): boolean {
    const ws = app.workspaces.find((w) => w.id === id);
    if (ws && matchesSearch(ws)) return true;
    const kids = childrenOf.get(id) ?? [];
    return kids.some((k) => subtreeMatches(k.id));
  }

  async function handleAdd(e: Event) {
    e.preventDefault();
    if (!newPath.trim()) return;
    adding = true;
    await app.addWorkspace(newPath.trim());
    if (!app.error) newPath = "";
    adding = false;
  }

  let creatingWorktreeFor = $state<string | null>(null);
  let worktreeName = $state("");

  // Programmatic focus avoids the a11y warning around `autofocus`.
  function focusOnMount(node: HTMLInputElement) {
    queueMicrotask(() => node.focus());
  }

  /// Confirm before destroying a workspace. We don't touch the folder on
  /// disk — this just drops the entry from claudedeck's registry and
  /// closes any sessions that were running in it. For worktrees, the
  /// underlying `git worktree` is also left intact; you can still attach
  /// to it later by re-adding the folder.
  function confirmRemove(id: string, name: string) {
    const ok = confirm(
      `Remove "${name}" from ClaudeDeck?\n\nThis closes any active sessions but keeps your folder and git worktree intact.`,
    );
    if (ok) app.removeWorkspace(id);
  }

  async function handleCreateWorktree(e: Event, parentId: string) {
    e.preventDefault();
    if (!worktreeName.trim()) return;
    await app.createWorktree(parentId, worktreeName.trim());
    if (!app.error) {
      worktreeName = "";
      creatingWorktreeFor = null;
    }
  }
</script>

<aside class="sidebar">
  <header>
    <span class="brand">CLAUDEDECK</span>
  </header>

  <section>
    <div class="section-header">
      <span class="section-label">WORKSPACES</span>
      {#if app.workspaces.length > 4}
        <input
          class="search"
          type="search"
          placeholder="Filter…"
          bind:value={search}
        />
      {/if}
    </div>

    {#if projects.length === 0}
      <div class="empty">No workspaces yet.</div>
    {/if}

    {#each projects as project (project.id)}
      {#if subtreeMatches(project.id)}
        {@render workspaceNode(project, 0)}
      {/if}
    {/each}
  </section>

{#snippet workspaceNode(ws: typeof app.workspaces[number], depth: number)}
  <div class="ws-row" style="padding-left: {6 + depth * 14}px">
    {#if depth === 0}
      <span class="ws-icon"><Icon name="folder" size={13} /></span>
    {:else}
      <span class="ws-tree-glyph"><Icon name="caretRight" size={11} /></span>
    {/if}
    <span class="ws-name">{ws.name}</span>
    {#if ws.branch}
      <span class="ws-branch"><Icon name="gitBranch" size={10} /> {ws.branch}</span>
    {/if}
    {#if depth === 0}
      <button
        class="ws-action ws-category"
        class:has-category={!!ws.category}
        title={ws.category ? `Category: ${ws.category} — click to change` : "Set category"}
        aria-label="Set category"
        onclick={(e) => openCategoryPicker(ws.id, e)}
      >
        <Icon name="tag" size={11} />
      </button>
    {/if}
    <button
      class="ws-action"
      title="New worktree from this branch"
      aria-label="New worktree"
      onclick={() =>
        (creatingWorktreeFor = creatingWorktreeFor === ws.id ? null : ws.id)}
    >
      <Icon name="plus" size={12} />
    </button>
    <button
      class="ws-action danger"
      title="Remove from sidebar"
      aria-label="Remove from sidebar"
      onclick={() => confirmRemove(ws.id, ws.name)}
    >
      <Icon name="x" size={12} />
    </button>
  </div>

  {#if creatingWorktreeFor === ws.id}
    <form
      class="inline-form"
      style="padding-left: {6 + depth * 14 + 14}px"
      onsubmit={(e) => handleCreateWorktree(e, ws.id)}
    >
      <input placeholder="worktree-name" bind:value={worktreeName} use:focusOnMount />
      <button type="submit">Create</button>
    </form>
  {/if}

  {#each childrenOf.get(ws.id) ?? [] as child (child.id)}
    {#if subtreeMatches(child.id)}
      {@render workspaceNode(child, depth + 1)}
    {/if}
  {/each}
{/snippet}

  <form class="add-form" onsubmit={handleAdd}>
    <button
      type="button"
      class="folder-btn"
      title="Choose folder…"
      onclick={pickFolder}
      disabled={adding}
      aria-label="Choose folder"
    >
      <Icon name="folderOpen" size={14} />
    </button>
    <input
      placeholder="~/Projects/your-repo"
      bind:value={newPath}
      disabled={adding}
    />
    <button type="submit" disabled={adding || !newPath.trim()} title="Add Workspace">
      <Icon name="plus" size={14} />
    </button>
  </form>

  {#if app.error}
    <div class="error">{app.error}</div>
  {/if}
</aside>

{#if categoryFor}
  {@const ws = app.workspaces.find((w) => w.id === categoryFor)}
  {#if ws}
    <CategoryPicker
      value={ws.category}
      suggestions={categorySuggestions}
      anchorRect={categoryAnchor}
      onApply={applyCategory}
      onClose={closeCategoryPicker}
    />
  {/if}
{/if}

<style>
  .sidebar {
    width: 100%;
    height: 100%;
    background: var(--surface-1);
    display: flex;
    flex-direction: column;
    overflow-y: auto;
  }

  header {
    padding: 14px 14px 10px;
    border-bottom: 1px solid var(--border);
  }

  .brand {
    font-family: var(--font-mono);
    font-size: 12px;
    letter-spacing: 0.15em;
    color: var(--text-3);
  }

  section { padding: 10px 8px; flex: 1; }

  .section-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 6px 8px;
  }
  .section-label {
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 0.12em;
    color: var(--text-3);
  }
  .search {
    margin-left: auto;
    flex: 1;
    max-width: 140px;
    font-size: 11px;
    padding: 3px 8px;
  }

  .empty {
    color: var(--text-3);
    font-size: 12px;
    padding: 6px;
    font-style: italic;
  }

  .ws-row {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 6px 6px;
    border-radius: var(--radius-sm);
    font-size: 13px;
    color: var(--text-1);
  }
  .ws-row:hover { background: var(--surface-2); }
  .ws-row.indent { padding-left: 18px; color: var(--text-2); }
  /* Action buttons stay hidden until you hover the row. Keeps the sidebar
     calm at rest, surfaces affordances on intent. */
  .ws-row .ws-action { opacity: 0; transition: opacity 100ms; }
  .ws-row:hover .ws-action { opacity: 1; }

  .ws-icon { font-size: 12px; opacity: 0.7; }
  .ws-tree-glyph {
    font-family: var(--font-mono);
    color: var(--text-3);
    font-size: 11px;
  }
  .ws-name { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .ws-branch {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-3);
    display: inline-flex;
    align-items: center;
    gap: 3px;
  }

  .ws-action {
    color: var(--text-3);
    font-size: 14px;
    width: 18px;
    height: 18px;
    border-radius: 4px;
    line-height: 1;
    flex-shrink: 0;
  }
  /* Action buttons live in a tight cluster on the right of the row.
     The first one in the cluster keeps the row's normal gap from the
     name/branch; the rest sit close together (2px). */
  .ws-row .ws-action + .ws-action {
    margin-left: -3px;
  }
  .ws-action:hover { background: var(--surface-3); color: var(--text-1); }
  .ws-action.danger:hover { background: rgba(227, 93, 106, 0.15); color: var(--red); }

  /* Category button is a primary affordance — keep it visible even when
     the row isn't hovered, so users can both (a) see at a glance which
     workspaces are categorized and (b) discover that the icon is
     clickable in the first place. The +/× actions stay hover-only. */
  .ws-row .ws-category { opacity: 0.4; }
  .ws-row:hover .ws-category { opacity: 1; }
  .ws-row .ws-category.has-category {
    opacity: 0.85;
    color: var(--accent);
  }
  .ws-row:hover .ws-category.has-category { opacity: 1; }

  .inline-form, .add-form {
    display: flex;
    gap: 6px;
    padding: 8px;
    align-items: stretch;
  }
  .add-form { border-top: 1px solid var(--border); }
  .inline-form input, .add-form input {
    flex: 1;
    min-width: 0;       /* let the input shrink so the button never wraps */
    font-size: 12px;
  }
  .add-form button, .inline-form button {
    flex-shrink: 0;
    padding: 0 10px;
    min-width: 30px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    font-size: 14px;
    line-height: 1;
    color: var(--text-2);
  }
  .add-form button:hover:not(:disabled),
  .inline-form button:hover { background: var(--surface-3); color: var(--text-1); }
  .add-form button:disabled { opacity: 0.4; cursor: not-allowed; }

  .folder-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--text-2);
  }
  .folder-btn:hover:not(:disabled) { color: var(--accent); }

  .error {
    margin: 8px;
    padding: 8px 10px;
    background: rgba(227, 93, 106, 0.1);
    border: 1px solid var(--red);
    border-radius: var(--radius-sm);
    color: var(--red);
    font-size: 11px;
  }
</style>
