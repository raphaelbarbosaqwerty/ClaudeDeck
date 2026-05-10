<script lang="ts">
  import type { Session, Workspace } from "../types";
  import { avatarDataUri } from "../utils/avatar";

  type Props = {
    session: Session;
    workspace: Workspace;
    parentWorkspace?: Workspace;
    isChild?: boolean;
    isLast?: boolean;
    active?: boolean;
    onclick?: () => void;
  };

  let {
    session,
    workspace,
    parentWorkspace,
    isChild = false,
    isLast = false,
    active = false,
    onclick,
  }: Props = $props();

  // Procedural pixel-art sprite. Stable per workspace id, so the same project
  // always gets the same little creature. Worktrees inherit a hue shift from
  // the parent project so the family is visible at a glance.
  let avatarUri = $derived(
    avatarDataUri(workspace.id, {
      parentId: isChild ? parentWorkspace?.id : undefined,
      size: 28,
    }),
  );

  const stateMeta = {
    idle: { label: "Idle", color: "var(--text-3)" },
    thinking: { label: "Thinking", color: "var(--yellow)" },
    generating: { label: "Generating", color: "var(--green)" },
    userInput: { label: "Input", color: "var(--accent)" },
    needsAttention: { label: "Attention", color: "var(--red)" },
  };
</script>

<button
  class="card"
  class:active
  class:child={isChild}
  class:last={isLast}
  {onclick}
>
  {#if isChild}
    <span class="connector" aria-hidden="true"></span>
  {/if}

  <span
    class="avatar"
    class:state-thinking={session.state === 'thinking'}
    class:state-generating={session.state === 'generating'}
    class:state-userInput={session.state === 'userInput'}
    class:state-needsAttention={session.state === 'needsAttention'}
    style="background-image: url('{avatarUri}');"
  ></span>

  <div class="body">
    <div class="row1">
      <span class="name">{workspace.name}</span>
      <span class="cost">${session.cost.toFixed(2)}</span>
    </div>
    <div class="row2">
      <span class="dot" style="background: {stateMeta[session.state].color}"></span>
      <span class="state">{stateMeta[session.state].label}</span>
      {#if session.subagentsTotal > 0}
        <span
          class="subagents"
          title="{session.subagentsActive} running · {session.subagentsTotal} total"
        >
          ↳ {session.subagentsActive > 0
            ? `${session.subagentsActive}/${session.subagentsTotal}`
            : session.subagentsTotal}
        </span>
      {/if}
      {#if isChild && workspace.branch}
        <span class="branch">⌥ {workspace.branch}</span>
      {/if}
      <span class="model">{session.model}</span>
    </div>
    <!-- Context window gauge: only renders once we have real usage data,
         so empty cards stay clean. Color shifts to red past 80%. -->
    {#if session.contextTokens > 0 && session.maxContextTokens > 0}
      {@const pct = Math.min(100, (session.contextTokens / session.maxContextTokens) * 100)}
      <div class="gauge" title="{Math.round(pct)}% of context window in use">
        <div
          class="gauge-fill"
          class:warn={pct >= 60 && pct < 80}
          class:hot={pct >= 80}
          style="width: {pct}%"
        ></div>
      </div>
    {/if}
  </div>
</button>

<style>
  .card {
    position: relative;
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    text-align: left;
    padding: 10px 12px;
    background: var(--surface-1);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    transition: background 120ms, border-color 120ms;
  }
  .card:hover { background: var(--surface-2); }
  .card.active {
    background: var(--surface-2);
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent) inset;
  }

  /* Indented worktree card with a tree-style L connector. */
  .card.child {
    margin-left: 22px;
    width: calc(100% - 22px);
  }
  .connector {
    position: absolute;
    left: -16px;
    top: -8px;
    bottom: 50%;
    width: 12px;
    border-left: 1px solid var(--border);
    border-bottom: 1px solid var(--border);
    border-bottom-left-radius: 6px;
  }
  /* The last sibling's connector stops at its own midpoint (already does, by virtue
     of bottom: 50%). The non-last sibling needs the line to keep going down: */
  .card.child:not(.last) .connector::after {
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
    flex-shrink: 0;
    background-color: var(--surface-2);
    background-size: contain;
    background-repeat: no-repeat;
    background-position: center;
    image-rendering: pixelated;
    image-rendering: crisp-edges;
    /* Animations only kick in on non-idle states; calm by default so a
       wall of sessions doesn't jitter the user's peripheral vision. */
    transition: filter 200ms, transform 200ms;
  }

  /* Thinking — very subtle inhale/exhale. The avatar appears to "consider". */
  .avatar.state-thinking {
    animation: cd-breathe 1.6s ease-in-out infinite;
    filter: drop-shadow(0 0 4px rgba(224, 194, 92, 0.4));
  }
  @keyframes cd-breathe {
    0%, 100% { transform: scale(1); }
    50%      { transform: scale(1.06); }
  }

  /* Generating — stronger pulse + green glow. The avatar feels "alive". */
  .avatar.state-generating {
    animation: cd-pulse 1.1s ease-in-out infinite;
    filter: drop-shadow(0 0 6px rgba(95, 191, 111, 0.55));
  }
  @keyframes cd-pulse {
    0%, 100% { transform: scale(1);    filter: drop-shadow(0 0 4px rgba(95, 191, 111, 0.4)); }
    50%      { transform: scale(1.08); filter: drop-shadow(0 0 10px rgba(95, 191, 111, 0.7)); }
  }

  /* User input — gentle nod, like the avatar is waiting attentively. */
  .avatar.state-userInput {
    animation: cd-nod 2s ease-in-out infinite;
    filter: drop-shadow(0 0 4px rgba(255, 140, 66, 0.4));
  }
  @keyframes cd-nod {
    0%, 100% { transform: translateY(0); }
    50%      { transform: translateY(-1.5px); }
  }

  /* Needs attention — short shake every couple of seconds. Eye-catching
     without being annoying. */
  .avatar.state-needsAttention {
    animation: cd-shake 2.4s ease-in-out infinite;
    filter: drop-shadow(0 0 6px rgba(227, 93, 106, 0.6));
  }
  @keyframes cd-shake {
    0%, 100%        { transform: translateX(0); }
    2%, 6%, 10%     { transform: translateX(-1.5px); }
    4%, 8%          { transform: translateX(1.5px); }
    14%             { transform: translateX(0); }
  }

  /* Reduced motion users opt out of all the animation entirely — the color
     ring on the status row already conveys state. */
  @media (prefers-reduced-motion: reduce) {
    .avatar.state-thinking,
    .avatar.state-generating,
    .avatar.state-userInput,
    .avatar.state-needsAttention {
      animation: none;
    }
  }

  .body { flex: 1; min-width: 0; }

  .row1 {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 8px;
  }
  .name {
    font-weight: 500;
    color: var(--text-1);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .cost {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-2);
  }

  .row2 {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 4px;
    font-size: 11px;
    color: var(--text-3);
  }
  .subagents {
    font-family: var(--font-mono);
    color: var(--purple);
    font-size: 10px;
  }
  .gauge {
    margin-top: 6px;
    height: 3px;
    background: var(--surface-3);
    border-radius: 2px;
    overflow: hidden;
  }
  .gauge-fill {
    height: 100%;
    background: var(--cyan);
    transition: width 200ms ease, background 200ms;
  }
  .gauge-fill.warn { background: var(--yellow); }
  .gauge-fill.hot { background: var(--red); }
  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .state { color: var(--text-2); }
  .branch {
    font-family: var(--font-mono);
    color: var(--text-3);
  }
  .model {
    margin-left: auto;
    font-family: var(--font-mono);
    color: var(--text-3);
  }
</style>
