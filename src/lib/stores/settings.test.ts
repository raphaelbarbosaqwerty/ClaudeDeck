import { describe, it, expect, beforeEach, vi } from "vitest";

// jsdom provides document/localStorage but to be safe we stub localStorage
// with an in-memory polyfill so the settings module's read() doesn't throw.
const memStore = new Map<string, string>();
vi.stubGlobal("localStorage", {
  getItem: (k: string) => (memStore.has(k) ? memStore.get(k)! : null),
  setItem: (k: string, v: string) => void memStore.set(k, v),
  removeItem: (k: string) => void memStore.delete(k),
  clear: () => memStore.clear(),
  key: (i: number) => Array.from(memStore.keys())[i] ?? null,
  get length() {
    return memStore.size;
  },
});

beforeEach(() => {
  memStore.clear();
  document.documentElement.removeAttribute("data-theme");
});

describe("applyThemeToDocument", () => {
  it("sets data-theme=light on the document root", async () => {
    const { applyThemeToDocument } = await import("./settings.svelte");
    applyThemeToDocument("light");
    expect(document.documentElement.dataset.theme).toBe("light");
  });

  it("sets data-theme=dark on the document root", async () => {
    const { applyThemeToDocument } = await import("./settings.svelte");
    applyThemeToDocument("dark");
    expect(document.documentElement.dataset.theme).toBe("dark");
  });
});

describe("settings store clamping", () => {
  it("setBackgroundOpacity clamps below 0.5", async () => {
    const { settings } = await import("./settings.svelte");
    settings.setBackgroundOpacity(0.2);
    expect(settings.backgroundOpacity).toBe(0.5);
  });

  it("setBackgroundOpacity clamps above 1.0", async () => {
    const { settings } = await import("./settings.svelte");
    settings.setBackgroundOpacity(1.5);
    expect(settings.backgroundOpacity).toBe(1.0);
  });

  it("setBackgroundOpacity passes through valid values", async () => {
    const { settings } = await import("./settings.svelte");
    settings.setBackgroundOpacity(0.85);
    expect(settings.backgroundOpacity).toBe(0.85);
  });

  it("setTerminalPadding clamps below 4px", async () => {
    const { settings } = await import("./settings.svelte");
    settings.setTerminalPadding(2);
    expect(settings.terminalPadding).toBe(4);
  });

  it("setTerminalPadding clamps above 32px", async () => {
    const { settings } = await import("./settings.svelte");
    settings.setTerminalPadding(50);
    expect(settings.terminalPadding).toBe(32);
  });

  it("setTerminalPadding rounds floats", async () => {
    // The slider can in theory produce floats; we keep the field integer
    // because xterm doesn't sub-pixel-position the cell grid based on
    // padding deltas, so fractional values are wasted precision.
    const { settings } = await import("./settings.svelte");
    settings.setTerminalPadding(14.7);
    expect(settings.terminalPadding).toBe(15);
  });

  it("setTerminalPadding accepts boundary values", async () => {
    const { settings } = await import("./settings.svelte");
    settings.setTerminalPadding(4);
    expect(settings.terminalPadding).toBe(4);
    settings.setTerminalPadding(32);
    expect(settings.terminalPadding).toBe(32);
  });
});

describe("settings store persistence", () => {
  it("writes terminalPreset to localStorage when set", async () => {
    const { settings } = await import("./settings.svelte");
    settings.setTerminalPreset("dracula");
    expect(localStorage.getItem("cd:terminal-preset")).toBe("dracula");
  });

  it("writes useVibrancy to localStorage when toggled", async () => {
    const { settings } = await import("./settings.svelte");
    settings.setUseVibrancy(true);
    expect(localStorage.getItem("cd:use-vibrancy")).toBe("true");
    settings.setUseVibrancy(false);
    expect(localStorage.getItem("cd:use-vibrancy")).toBe("false");
  });
});
