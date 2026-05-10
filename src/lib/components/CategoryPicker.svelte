<script lang="ts">
  // Combobox for workspace categories.
  //
  //   ┌────────────────────────┐
  //   │ Category               │
  //   │ [type or pick…       ] │ ← typed value creates a new category
  //   │                        │
  //   │ Existing               │
  //   │   ✓ Work               │ ← current value, click to keep
  //   │     Side projects      │ ← click to switch
  //   │     Clients · Acme     │
  //   │                        │
  //   │ [ Clear ]    [ Apply ] │
  //   └────────────────────────┘
  //
  // Anchored to a trigger element by the parent. Closes on Escape, on
  // outside-click, or on Apply/Clear.
  import { onMount } from "svelte";
  import Icon from "./Icon.svelte";

  type Props = {
    value: string | undefined;
    /// Existing categories to suggest. Computed by the parent from the
    /// current workspace list (deduped, alpha-sorted).
    suggestions: string[];
    onApply: (category: string | null) => void;
    onClose: () => void;
    /// Optional anchor rect from getBoundingClientRect on the trigger.
    /// We position the popover bottom-aligned to its right edge by default.
    anchorRect?: DOMRect;
  };

  let { value, suggestions, onApply, onClose, anchorRect }: Props = $props();

  let typed = $state(value ?? "");
  let inputEl: HTMLInputElement | null = $state(null);

  // Filter suggestions by what the user has typed. Always show every
  // suggestion when the field is empty so the user can pick without typing.
  let filtered = $derived.by(() => {
    const q = typed.trim().toLowerCase();
    if (!q) return suggestions;
    return suggestions.filter((s) => s.toLowerCase().includes(q));
  });

  // Whether the typed value is a brand-new category (not in suggestions).
  // Drives the "Create '<x>'" affordance.
  let isNew = $derived.by(() => {
    const t = typed.trim();
    if (!t) return false;
    return !suggestions.some(
      (s) => s.toLowerCase() === t.toLowerCase(),
    );
  });

  function apply() {
    const t = typed.trim();
    onApply(t === "" ? null : t);
    onClose();
  }

  function clear() {
    onApply(null);
    onClose();
  }

  function pick(s: string) {
    typed = s;
    onApply(s);
    onClose();
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    } else if (e.key === "Enter") {
      e.preventDefault();
      apply();
    }
  }

  onMount(() => {
    inputEl?.focus();
    inputEl?.select();

    function onDocClick(e: MouseEvent) {
      const target = e.target as HTMLElement;
      if (!target.closest(".category-popover")) onClose();
    }
    // Defer one tick so the click that opened us doesn't immediately close.
    setTimeout(() => document.addEventListener("mousedown", onDocClick), 0);
    return () => document.removeEventListener("mousedown", onDocClick);
  });

  // Position: prefer opening to the right of the trigger (works for
  // sidebar buttons where there's empty space to the right). If the
  // popover would overflow the viewport, fall back to opening to the
  // left of the trigger. Final fallback clamps to the viewport so the
  // popover never bleeds off-screen — the previous implementation
  // assumed the trigger lived on the right side of the window and
  // pushed the popover off the left edge for sidebar buttons.
  const POPOVER_WIDTH = 240;
  const VIEWPORT_MARGIN = 8;

  let style = $derived.by(() => {
    if (!anchorRect) return "";
    const top = anchorRect.bottom + 6;

    // First choice: just to the right of the trigger.
    let left = anchorRect.right + 6;
    if (left + POPOVER_WIDTH + VIEWPORT_MARGIN > window.innerWidth) {
      // Doesn't fit on the right — try the left side.
      left = anchorRect.left - POPOVER_WIDTH - 6;
    }
    // Clamp to viewport so it stays visible no matter what.
    if (left < VIEWPORT_MARGIN) left = VIEWPORT_MARGIN;
    if (left + POPOVER_WIDTH + VIEWPORT_MARGIN > window.innerWidth) {
      left = window.innerWidth - POPOVER_WIDTH - VIEWPORT_MARGIN;
    }

    return `top: ${top}px; left: ${left}px;`;
  });
</script>

<div
  class="category-popover"
  role="dialog"
  aria-label="Set workspace category"
  {style}
  onkeydown={onKeydown}
>
  <div class="header">Category</div>

  <input
    bind:this={inputEl}
    bind:value={typed}
    placeholder="Type or pick…"
    autocomplete="off"
    spellcheck="false"
  />

  {#if filtered.length > 0}
    <div class="section-label">Existing</div>
    <ul class="suggestions">
      {#each filtered as s (s)}
        {@const isCurrent = s === value}
        <li>
          <button class="suggestion" class:current={isCurrent} onclick={() => pick(s)}>
            {#if isCurrent}
              <Icon name="check" size={11} />
            {:else}
              <span class="dot"></span>
            {/if}
            <span class="label">{s}</span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}

  {#if isNew}
    <div class="hint">
      Press Enter or click Apply to create "<strong>{typed.trim()}</strong>"
    </div>
  {/if}

  <div class="actions">
    <button class="ghost" onclick={clear}>Clear</button>
    <button class="primary" onclick={apply}>Apply</button>
  </div>
</div>

<style>
  .category-popover {
    position: fixed;
    z-index: 90;
    width: 240px;
    background: var(--surface-1);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.5);
    padding: 10px 12px;
    font-size: 12px;
    color: var(--text-1);
    animation: cd-pop-in 140ms ease-out;
  }
  @keyframes cd-pop-in {
    from { opacity: 0; transform: translateY(-4px); }
    to   { opacity: 1; transform: translateY(0); }
  }

  .header {
    font-family: var(--font-mono);
    font-size: 10px;
    letter-spacing: 0.12em;
    color: var(--text-3);
    margin-bottom: 6px;
  }
  .section-label {
    font-family: var(--font-mono);
    font-size: 9px;
    letter-spacing: 0.1em;
    color: var(--text-3);
    margin-top: 10px;
    margin-bottom: 4px;
  }

  input {
    width: 100%;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text-1);
    padding: 6px 8px;
    font: inherit;
  }
  input:focus { border-color: var(--accent); outline: none; }

  .suggestions {
    list-style: none;
    margin: 0;
    padding: 0;
    max-height: 180px;
    overflow-y: auto;
  }
  .suggestion {
    width: 100%;
    text-align: left;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 6px;
    background: transparent;
    border: 0;
    color: var(--text-2);
    border-radius: var(--radius-sm);
    cursor: pointer;
    font-size: 12px;
  }
  .suggestion:hover { background: var(--surface-2); color: var(--text-1); }
  .suggestion.current { color: var(--accent); }
  .dot {
    width: 11px;
    display: inline-block;
  }
  .label { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  .hint {
    margin-top: 8px;
    padding: 6px 8px;
    background: rgba(255, 140, 66, 0.08);
    border: 1px solid rgba(255, 140, 66, 0.25);
    border-radius: var(--radius-sm);
    color: var(--text-2);
    font-size: 11px;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 6px;
    margin-top: 10px;
  }
  .actions button {
    padding: 5px 10px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    font-size: 11px;
    background: var(--surface-2);
    color: var(--text-2);
    cursor: pointer;
  }
  .actions .ghost:hover { color: var(--text-1); border-color: var(--text-3); }
  .actions .primary {
    background: var(--accent);
    color: #0e0e10;
    border-color: var(--accent);
  }
  .actions .primary:hover { filter: brightness(1.06); }
</style>
