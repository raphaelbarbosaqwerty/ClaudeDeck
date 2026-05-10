// User-level preferences. Persisted in localStorage so they survive reloads.
// Kept separate from app.svelte.ts because settings outlive the app session
// state and shouldn't be reset when workspaces are reloaded.

const KEYS = {
  theme: "cd:theme",
  autoResume: "cd:auto-resume",
  notifications: "cd:notifications",
  terminalPreset: "cd:terminal-preset",
  backgroundOpacity: "cd:background-opacity",
  useVibrancy: "cd:use-vibrancy",
  terminalPadding: "cd:terminal-padding",
} as const;

export type Theme = "dark" | "light";

function read<T>(key: string, fallback: T, parse: (raw: string) => T): T {
  if (typeof localStorage === "undefined") return fallback;
  const raw = localStorage.getItem(key);
  if (raw === null) return fallback;
  try {
    return parse(raw);
  } catch {
    return fallback;
  }
}

class SettingsStore {
  theme = $state<Theme>(read<Theme>(KEYS.theme, "dark", (r) => (r === "light" ? "light" : "dark")));
  autoResume = $state(read(KEYS.autoResume, true, (r) => r === "true"));
  notifications = $state(read(KEYS.notifications, true, (r) => r === "true"));

  /// Active terminal preset id (matches THEME_PRESETS[].id in utils/themes.ts).
  terminalPreset = $state(read(KEYS.terminalPreset, "claudedeck", (r) => r));

  /// Terminal background opacity 0.5..1.0. Below ~0.6 the text starts
  /// looking blurry against most desktop backgrounds, so we clamp.
  backgroundOpacity = $state(
    read(KEYS.backgroundOpacity, 1.0, (r) => {
      const n = parseFloat(r);
      return Number.isFinite(n) ? Math.max(0.5, Math.min(1.0, n)) : 1.0;
    }),
  );

  /// macOS-only "vibrancy" effect: native blurred backdrop showing the
  /// desktop wallpaper through the window. No-op on Windows/Linux.
  useVibrancy = $state(read(KEYS.useVibrancy, false, (r) => r === "true"));

  /// Internal padding (px) between xterm's text content and the rounded
  /// terminal frame. Default 4px keeps the terminal compact (more rows
  /// of code visible at a glance); users who prefer Apple Terminal-style
  /// breathing room can crank it up to 32 in Settings. Clamped to a
  /// sensible range either way.
  terminalPadding = $state(
    read(KEYS.terminalPadding, 4, (r) => {
      const n = parseInt(r, 10);
      return Number.isFinite(n) ? Math.max(4, Math.min(32, n)) : 4;
    }),
  );

  setTheme(t: Theme) {
    this.theme = t;
    localStorage.setItem(KEYS.theme, t);
    applyThemeToDocument(t);
  }

  toggleTheme() {
    this.setTheme(this.theme === "dark" ? "light" : "dark");
  }

  setAutoResume(v: boolean) {
    this.autoResume = v;
    localStorage.setItem(KEYS.autoResume, String(v));
  }

  setNotifications(v: boolean) {
    this.notifications = v;
    localStorage.setItem(KEYS.notifications, String(v));
  }

  setTerminalPreset(id: string) {
    this.terminalPreset = id;
    localStorage.setItem(KEYS.terminalPreset, id);
  }

  setBackgroundOpacity(v: number) {
    const clamped = Math.max(0.5, Math.min(1.0, v));
    this.backgroundOpacity = clamped;
    localStorage.setItem(KEYS.backgroundOpacity, String(clamped));
  }

  setUseVibrancy(v: boolean) {
    this.useVibrancy = v;
    localStorage.setItem(KEYS.useVibrancy, String(v));
    applyVibrancyToDocument(v);
  }

  setTerminalPadding(v: number) {
    const clamped = Math.max(4, Math.min(32, Math.round(v)));
    this.terminalPadding = clamped;
    localStorage.setItem(KEYS.terminalPadding, String(clamped));
  }
}

export const settings = new SettingsStore();

/// Apply theme to the document root by setting `data-theme`. CSS variables
/// in theme.css branch on `[data-theme="light"]` to swap the palette.
export function applyThemeToDocument(theme: Theme) {
  if (typeof document === "undefined") return;
  document.documentElement.dataset.theme = theme;
}

/// Toggle the `data-vibrancy` attribute on <html>. CSS rules in theme.css
/// switch panel/sidebar surfaces to semi-transparent + backdrop-blur when
/// vibrancy is on, so the macOS native blur shows through. Also drives a
/// Tauri command that calls into `window-vibrancy` on the Rust side.
export async function applyVibrancyToDocument(enabled: boolean) {
  if (typeof document === "undefined") return;
  if (enabled) {
    document.documentElement.dataset.vibrancy = "true";
  } else {
    delete document.documentElement.dataset.vibrancy;
  }
  // Defer the Rust call so this function stays sync-safe in tests.
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    await invoke("set_window_vibrancy", { enabled });
  } catch {
    // Plugin not available (web preview, Linux/Windows) — silent no-op.
  }
}
