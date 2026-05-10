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
