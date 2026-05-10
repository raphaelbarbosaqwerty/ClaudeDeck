import { describe, it, expect } from "vitest";
import { THEME_PRESETS, getPreset, withOpacity } from "./themes";

describe("THEME_PRESETS", () => {
  it("ships at least 8 presets", () => {
    expect(THEME_PRESETS.length).toBeGreaterThanOrEqual(8);
  });

  it("has unique ids across presets", () => {
    const ids = THEME_PRESETS.map((p) => p.id);
    const uniq = new Set(ids);
    expect(uniq.size).toBe(ids.length);
  });

  it("includes the brand default 'claudedeck'", () => {
    expect(THEME_PRESETS.some((p) => p.id === "claudedeck")).toBe(true);
  });

  it.each(THEME_PRESETS)("preset $id has all required ITheme fields", (p) => {
    // Every preset must populate the full xterm.js theme shape — missing a
    // field would leave xterm rendering with a fallback color that breaks
    // the preset's visual identity.
    const required = [
      "background", "foreground", "cursor", "cursorAccent", "selectionBackground",
      "black", "red", "green", "yellow", "blue", "magenta", "cyan", "white",
      "brightBlack", "brightRed", "brightGreen", "brightYellow",
      "brightBlue", "brightMagenta", "brightCyan", "brightWhite",
    ] as const;
    for (const key of required) {
      // Two-step cast: TerminalTheme isn't structurally a Record<string,string>
      // (its keys are a closed union). Narrowing through `unknown` keeps tsc
      // honest while letting us iterate by string key for the assertion.
      const value = (p.theme as unknown as Record<string, string>)[key];
      expect(value, `preset ${p.id} is missing ${key}`).toBeDefined();
      expect(value, `preset ${p.id} has empty ${key}`).not.toBe("");
    }
  });

  it.each(THEME_PRESETS)("preset $id colors are valid hex strings", (p) => {
    // We allow rgba in `withOpacity` results but raw presets are pure hex.
    // Catch typos like "##282a36" or "rgb(...)" creeping into a preset.
    const hexRe = /^#[0-9a-fA-F]{6}$/;
    for (const [key, value] of Object.entries(p.theme)) {
      expect(value, `preset ${p.id}.${key} should be #RRGGBB`).toMatch(hexRe);
    }
  });
});

describe("getPreset", () => {
  it("returns the named preset when id matches", () => {
    expect(getPreset("dracula").id).toBe("dracula");
    expect(getPreset("tokyo-night").id).toBe("tokyo-night");
  });

  it("falls back to claudedeck for unknown id", () => {
    expect(getPreset("not-a-real-preset").id).toBe("claudedeck");
    expect(getPreset("").id).toBe("claudedeck");
  });
});

describe("withOpacity", () => {
  const preset = getPreset("claudedeck").theme;

  it("returns the same theme reference at opacity ≥ 0.999", () => {
    // Avoiding the rgba conversion when opacity is effectively 1 keeps
    // the cheap path cheap and avoids unnecessary string allocation per
    // re-render.
    const out = withOpacity(preset, 1);
    expect(out).toBe(preset);
  });

  it("returns the same theme reference at exactly 0.999", () => {
    const out = withOpacity(preset, 0.999);
    expect(out).toBe(preset);
  });

  it("converts background to rgba when opacity < 0.999", () => {
    const out = withOpacity(preset, 0.7);
    expect(out).not.toBe(preset);
    expect(out.background).toMatch(/^rgba\(\d+, \d+, \d+, [\d.]+\)$/);
  });

  it("preserves non-background fields when applying opacity", () => {
    const out = withOpacity(preset, 0.5);
    expect(out.foreground).toBe(preset.foreground);
    expect(out.cursor).toBe(preset.cursor);
    expect(out.red).toBe(preset.red);
  });

  it("encodes the alpha into the rgba string", () => {
    const out = withOpacity(preset, 0.5);
    // 0.500 should appear in the alpha slot.
    expect(out.background).toMatch(/, 0\.500\)$/);
  });

  it("hexToRgba correctly decodes #0e0e10", () => {
    const out = withOpacity(preset, 0.8);
    // ClaudeDeck preset background = #0e0e10 => 14, 14, 16
    expect(out.background).toBe("rgba(14, 14, 16, 0.800)");
  });
});
