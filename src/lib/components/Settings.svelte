<script lang="ts">
  // Settings modal. Currently covers: app theme (dark/light), terminal
  // preset, background opacity, macOS vibrancy, autoResume, notifications.
  // No keyboard shortcut by user request — opens only via the gear icon.
  import { settings } from "../stores/settings.svelte";
  import { THEME_PRESETS, getPreset } from "../utils/themes";
  import Icon from "./Icon.svelte";

  type Props = {
    onClose: () => void;
  };

  let { onClose }: Props = $props();

  // Detect macOS so we can hide the vibrancy section on platforms where
  // it has no effect. The user-agent dance is fine here — the result
  // only affects what's rendered, not behavior.
  const isMac =
    typeof navigator !== "undefined" &&
    /Mac/i.test(navigator.platform || "") &&
    !/iPhone|iPad/i.test(navigator.userAgent || "");

  // Live preview of the selected preset background, used as a swatch
  // next to each radio so users can pick by eye, not by name.
  function bgSwatch(id: string): string {
    return getPreset(id).theme.background;
  }
  function fgSwatch(id: string): string {
    return getPreset(id).theme.foreground;
  }
</script>

<div
  class="overlay"
  role="dialog"
  aria-modal="true"
  aria-label="Settings"
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
        <Icon name="wrench" size={14} /> Settings
      </span>
      <button class="close" onclick={onClose} aria-label="Close">
        <Icon name="x" size={14} />
      </button>
    </header>

    <div class="scroll">
      <!-- Appearance ---------------------------------------------------->
      <section>
        <div class="section-label">APPEARANCE</div>

        <div class="row">
          <span class="row-label">App theme</span>
          <div class="seg">
            <button
              class:on={settings.theme === "dark"}
              onclick={() => settings.setTheme("dark")}
            >
              <Icon name="moon" size={11} /> Dark
            </button>
            <button
              class:on={settings.theme === "light"}
              onclick={() => settings.setTheme("light")}
            >
              <Icon name="sun" size={11} /> Light
            </button>
          </div>
        </div>

        <div class="row stacked">
          <span class="row-label">Terminal preset</span>
          <div class="presets">
            {#each THEME_PRESETS as p (p.id)}
              <button
                class="preset"
                class:on={settings.terminalPreset === p.id}
                onclick={() => settings.setTerminalPreset(p.id)}
                title={p.hint}
              >
                <span
                  class="preview"
                  style="background: {bgSwatch(p.id)}; color: {fgSwatch(p.id)}"
                >
                  &gt;_
                </span>
                <span class="preset-name">{p.name}</span>
                <span class="preset-hint">{p.hint}</span>
              </button>
            {/each}
          </div>
        </div>

        <div class="row">
          <span class="row-label">
            Background opacity
            <span class="value">{Math.round(settings.backgroundOpacity * 100)}%</span>
          </span>
          <input
            type="range"
            min="0.5"
            max="1"
            step="0.01"
            value={settings.backgroundOpacity}
            oninput={(e) => settings.setBackgroundOpacity(+(e.target as HTMLInputElement).value)}
          />
        </div>

        <div class="row">
          <span class="row-label">
            Terminal padding
            <span class="value">{settings.terminalPadding}px</span>
          </span>
          <input
            type="range"
            min="4"
            max="32"
            step="1"
            value={settings.terminalPadding}
            oninput={(e) => settings.setTerminalPadding(+(e.target as HTMLInputElement).value)}
          />
        </div>

        {#if isMac}
          <div class="row">
            <span class="row-label">
              macOS vibrancy
              <span class="hint-small">native blurred backdrop</span>
            </span>
            <button
              class="toggle"
              class:on={settings.useVibrancy}
              role="switch"
              aria-checked={settings.useVibrancy}
              onclick={() => settings.setUseVibrancy(!settings.useVibrancy)}
            >
              <span class="toggle-knob"></span>
            </button>
          </div>
        {/if}
      </section>

      <!-- Behavior ------------------------------------------------------>
      <section>
        <div class="section-label">BEHAVIOR</div>

        <div class="row">
          <span class="row-label">
            Auto-resume on launch
            <span class="hint-small">resume up to 3 recent sessions</span>
          </span>
          <button
            class="toggle"
            class:on={settings.autoResume}
            role="switch"
            aria-checked={settings.autoResume}
            onclick={() => settings.setAutoResume(!settings.autoResume)}
          >
            <span class="toggle-knob"></span>
          </button>
        </div>

        <div class="row">
          <span class="row-label">
            Desktop notifications
            <span class="hint-small">when an inactive session goes idle</span>
          </span>
          <button
            class="toggle"
            class:on={settings.notifications}
            role="switch"
            aria-checked={settings.notifications}
            onclick={() => settings.setNotifications(!settings.notifications)}
          >
            <span class="toggle-knob"></span>
          </button>
        </div>
      </section>
    </div>

    <footer>
      <button class="primary" onclick={onClose}>Done</button>
    </footer>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.45);
    backdrop-filter: blur(4px);
    z-index: 70;
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding: 60px 24px;
  }
  .panel {
    width: 540px;
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
  .title { flex: 1; font-weight: 600; font-size: 13px; display: flex; align-items: center; gap: 8px; }
  .close {
    width: 24px; height: 24px;
    border-radius: 4px;
    color: var(--text-3);
    font-size: 18px; line-height: 1;
  }
  .close:hover { background: var(--surface-3); color: var(--text-1); }

  .scroll { overflow-y: auto; padding: 4px 0; }
  section { padding: 14px 18px; border-bottom: 1px dashed var(--border); }
  section:last-child { border-bottom: 0; }
  .section-label {
    font-family: var(--font-mono);
    font-size: 10px;
    letter-spacing: 0.12em;
    color: var(--text-3);
    margin-bottom: 12px;
  }

  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 12px;
  }
  .row.stacked { flex-direction: column; align-items: stretch; gap: 8px; }
  .row:last-child { margin-bottom: 0; }
  .row-label {
    font-size: 12px;
    color: var(--text-1);
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .row-label .value { color: var(--accent); font-family: var(--font-mono); font-size: 11px; }
  .hint-small {
    font-size: 10px;
    color: var(--text-3);
    font-weight: normal;
  }

  /* Segmented buttons (dark/light) */
  .seg {
    display: inline-flex;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 2px;
  }
  .seg button {
    padding: 4px 10px;
    background: transparent;
    border: 0;
    border-radius: 3px;
    color: var(--text-2);
    font-size: 11px;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    cursor: pointer;
  }
  .seg button.on {
    background: var(--surface-3);
    color: var(--text-1);
  }

  /* Toggle (switch) */
  .toggle {
    width: 36px;
    height: 20px;
    border-radius: 10px;
    background: var(--surface-3);
    border: 1px solid var(--border);
    position: relative;
    cursor: pointer;
    flex-shrink: 0;
    transition: background 140ms;
  }
  .toggle.on { background: var(--accent); border-color: var(--accent); }
  .toggle-knob {
    position: absolute;
    top: 1px; left: 1px;
    width: 16px; height: 16px;
    background: var(--text-1);
    border-radius: 50%;
    transition: transform 140ms ease;
  }
  .toggle.on .toggle-knob { transform: translateX(16px); background: #0e0e10; }

  /* Presets grid */
  .presets {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px;
  }
  .preset {
    display: grid;
    grid-template-columns: auto 1fr;
    grid-template-rows: auto auto;
    gap: 2px 8px;
    padding: 8px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    text-align: left;
    cursor: pointer;
    transition: border-color 120ms;
  }
  .preset:hover { border-color: var(--text-3); }
  .preset.on {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent) inset;
  }
  .preview {
    grid-row: 1 / span 2;
    width: 32px;
    height: 32px;
    border-radius: 4px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 600;
  }
  .preset-name {
    font-size: 12px;
    color: var(--text-1);
    align-self: end;
    line-height: 1.1;
  }
  .preset-hint {
    font-size: 10px;
    color: var(--text-3);
    align-self: start;
    line-height: 1.2;
  }

  /* Range input */
  input[type="range"] {
    flex: 1;
    max-width: 200px;
    accent-color: var(--accent);
    cursor: pointer;
  }

  footer {
    display: flex;
    justify-content: flex-end;
    padding: 12px 14px;
    border-top: 1px solid var(--border);
    background: var(--surface-2);
  }
  footer .primary {
    padding: 6px 14px;
    background: var(--accent);
    color: #0e0e10;
    border: 1px solid var(--accent);
    border-radius: var(--radius-sm);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
  }
  footer .primary:hover { filter: brightness(1.06); }
</style>
