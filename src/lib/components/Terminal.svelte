<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { Terminal } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import { WebLinksAddon } from "@xterm/addon-web-links";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import "@xterm/xterm/css/xterm.css";
  import { api } from "../api";
  import { app } from "../stores/app.svelte";
  import { settings } from "../stores/settings.svelte";
  import { getPreset, withOpacity } from "../utils/themes";

  type Props = {
    sessionId: string;
    visible: boolean;
  };

  let { sessionId, visible }: Props = $props();

  let host: HTMLDivElement;
  let term: Terminal | null = null;
  let fit: FitAddon | null = null;
  let unlistenOutput: UnlistenFn | null = null;
  let resizeObserver: ResizeObserver | null = null;

  // We re-fit whenever the host element resizes (sidebar collapse, window
  // resize, etc.). Debounce keeps the resize round-trip with the backend
  // from spamming during drag.
  let resizeTimeout: number | null = null;
  function scheduleFit() {
    if (resizeTimeout !== null) clearTimeout(resizeTimeout);
    resizeTimeout = window.setTimeout(() => {
      if (!fit || !term) return;
      try {
        fit.fit();
        api.resizePty(sessionId, term.cols, term.rows).catch(() => {});
      } catch {}
      resizeTimeout = null;
    }, 60);
  }

  /// Wait for the host element to have non-zero size before the initial fit.
  /// Without this, mounting while the tab is hidden (or before layout) hands
  /// xterm a 0×0 box, which it caches as "1 col" — the wrap-everything bug.
  async function waitForLayout(el: HTMLElement) {
    for (let i = 0; i < 30; i++) {
      if (el.clientWidth > 0 && el.clientHeight > 0) return;
      await new Promise((r) => requestAnimationFrame(r));
    }
  }

  onMount(async () => {
    term = new Terminal({
      fontFamily:
        '"JetBrains Mono", "SF Mono", Menlo, Monaco, "Cascadia Code", monospace',
      fontSize: 13,
      lineHeight: 1.2,
      cursorBlink: true,
      allowProposedApi: true,
      // Pull initial theme from the active preset + opacity. If the user
      // changes either later, the $effect below pushes the new theme
      // into xterm without remounting.
      allowTransparency: true,
      theme: withOpacity(
        getPreset(settings.terminalPreset).theme,
        settings.backgroundOpacity,
      ),
    });

    fit = new FitAddon();
    term.loadAddon(fit);
    term.loadAddon(new WebLinksAddon());

    // Make sure the host has measured layout before we open xterm. Otherwise
    // the first fit() runs against a zero-size box and the PTY gets sized
    // to ~1 column — every subsequent line wraps until the user resizes.
    await waitForLayout(host);
    term.open(host);
    fit.fit();
    api.resizePty(sessionId, term.cols, term.rows).catch(() => {});

    // Subscribe to PTY output for this session.
    unlistenOutput = await listen<string>(
      `pty:output:${sessionId}`,
      (event) => {
        term?.write(event.payload);
      },
    );

    // Forward keystrokes to the PTY.
    term.onData((data) => {
      api.writePty(sessionId, data).catch(() => {});
    });

    // React to host resizes.
    resizeObserver = new ResizeObserver(() => scheduleFit());
    resizeObserver.observe(host);
  });

  onDestroy(() => {
    unlistenOutput?.();
    resizeObserver?.disconnect();
    term?.dispose();
    term = null;
    fit = null;
  });

  // On visibility change we force a resize round-trip to the PTY even when
  // the measured size hasn't actually changed. Claude's TUI is in alt-buffer
  // mode and only repaints on input or SIGWINCH; without this nudge, the
  // previous tab's stale frame can stick around as artifacts (or the screen
  // looks "empty" because Claude positioned the cursor off-viewport before
  // the user switched away). Sending the same dims is harmless on the PTY
  // side but triggers SIGWINCH, which makes Claude redraw the current frame.
  $effect(() => {
    if (visible && term && fit) {
      queueMicrotask(async () => {
        try {
          fit?.fit();
          await api.resizePty(sessionId, term!.cols, term!.rows);
          // Nudge Claude to repaint: resize +0,+0 once more after a tick.
          // SIGWINCH coalesces if back-to-back, so we space these out.
          setTimeout(() => {
            void api.resizePty(sessionId, term!.cols, term!.rows);
          }, 30);
          term?.refresh(0, term.rows - 1);
          term?.focus();
        } catch {}
      });
    }
  });

  // Reactive theme application: whenever the user changes preset or
  // opacity in Settings, xterm gets the new ITheme. xterm does the work
  // of repainting cells with the new palette — no remount, no flicker.
  $effect(() => {
    const presetId = settings.terminalPreset;
    const opacity = settings.backgroundOpacity;
    if (!term) return;
    const next = withOpacity(getPreset(presetId).theme, opacity);
    term.options.theme = next;
  });

  // Force-redraw recovery — wired to ⌘⇧R from +page.svelte. When Claude's
  // TUI gets out of sync (cursor positioning artifacts, half-overwritten
  // lines), this resync sequence usually clears the corruption:
  //   1. fit + resizePty so xterm and PTY agree on cols/rows.
  //   2. Send SIGWINCH twice with a tick between to make sure Claude re-runs
  //      its layout on the *current* dims.
  //   3. xterm.refresh repaints every cell from the buffer, which clears
  //      any stale visual cells xterm itself was holding.
  // Only the visible terminal acts on the tick — all instances see it but
  // only the active one does work.
  $effect(() => {
    const tick = app.forceRedrawTick;
    if (tick === 0) return;
    if (!visible || !term || !fit) return;
    queueMicrotask(async () => {
      try {
        fit?.fit();
        await api.resizePty(sessionId, term!.cols, term!.rows);
        setTimeout(() => {
          void api.resizePty(sessionId, term!.cols, term!.rows);
          term?.refresh(0, term!.rows - 1);
        }, 40);
      } catch {}
    });
  });
</script>

<div
  bind:this={host}
  class="term-host"
  class:hidden={!visible}
  aria-hidden={!visible}
  style="padding: {settings.terminalPadding}px;"
></div>

<style>
  /* Stack every terminal on top of each other inside the parent. We toggle
     `visibility` instead of `display` so xterm always sees the correct
     measured size — even for inactive tabs. Switching tabs is then a paint,
     not a re-layout, and the PTY column count stays in sync.
     The host floats inside `.terminal-area` with a small outer inset and
     rounded corners so the terminal reads as a discrete card the way
     Apple Terminal and iTerm windows do, rather than a full-bleed
     surface that hugs the splitters. */
  .term-host {
    position: absolute;
    inset: 0;
    background: var(--bg);
    /* No border-radius: xterm renders into its own canvas/DOM that
       doesn't respect the parent's rounded clipping, so rounding the
       host only rounds an "envelope" around a still-rectangular
       terminal — visible mismatch in light theme. Keeping the host
       full-bleed is honest. Internal padding (set inline from the
       user's settings) gives xterm's text breathing room without
       lying about the shape. */
  }
  .term-host.hidden {
    visibility: hidden;
    pointer-events: none;
    z-index: 0;
  }
  /* xterm.css sets some defaults we want to override for tighter spacing. */
  :global(.xterm) {
    height: 100%;
  }
  :global(.xterm-viewport::-webkit-scrollbar) { width: 8px; }
  :global(.xterm-viewport::-webkit-scrollbar-thumb) {
    background: var(--surface-3);
    border-radius: 4px;
  }
</style>
