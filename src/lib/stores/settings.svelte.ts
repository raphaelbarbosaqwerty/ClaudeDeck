// User-level preferences. Persisted in localStorage so they survive reloads.
// Kept separate from app.svelte.ts because settings outlive the app session
// state and shouldn't be reset when workspaces are reloaded.

const KEYS = {
  theme: "cd:theme",
  autoResume: "cd:auto-resume",
  notifications: "cd:notifications",
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
}

export const settings = new SettingsStore();

/// Apply theme to the document root by setting `data-theme`. CSS variables
/// in theme.css branch on `[data-theme="light"]` to swap the palette.
export function applyThemeToDocument(theme: Theme) {
  if (typeof document === "undefined") return;
  document.documentElement.dataset.theme = theme;
}
