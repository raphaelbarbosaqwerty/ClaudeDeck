import { describe, it, expect } from "vitest";
import { colorFor, childColorFor } from "./color";

// Local re-implementation of the private hashStr so we can predict hues
// for the childColorFor drift assertion. Mirrors the exact shape of the
// hash function in color.ts.
function hashStr(s: string): number {
  let h = 0;
  for (let i = 0; i < s.length; i++) {
    h = (h * 31 + s.charCodeAt(i)) | 0;
  }
  return Math.abs(h);
}

function parseHsl(hsl: string): { h: number; s: number; l: number } {
  // Format: "hsl(<h> <s>% <l>%)"
  const m = hsl.match(/hsl\(\s*(\d+(?:\.\d+)?)\s+(\d+(?:\.\d+)?)%\s+(\d+(?:\.\d+)?)%\s*\)/);
  if (!m) throw new Error(`Unparseable hsl: ${hsl}`);
  return { h: parseFloat(m[1]), s: parseFloat(m[2]), l: parseFloat(m[3]) };
}

describe("colorFor", () => {
  it("returns a stable HSL string for the same id", () => {
    const a = colorFor("workspace-123");
    const b = colorFor("workspace-123");
    expect(a).toBe(b);
    expect(a).toMatch(/^hsl\(\d+(?:\.\d+)?\s+\d+(?:\.\d+)?%\s+\d+(?:\.\d+)?%\)$/);
  });

  it("produces at least 30 unique hues for 50 different ids", () => {
    const hues = new Set<number>();
    for (let i = 0; i < 50; i++) {
      hues.add(parseHsl(colorFor(`id-${i}-${i * 7}`)).h);
    }
    expect(hues.size).toBeGreaterThanOrEqual(30);
  });

  it("lighten option shifts lightness from 50% to 60%", () => {
    const base = parseHsl(colorFor("x"));
    const lit = parseHsl(colorFor("x", { lighten: 10 }));
    expect(base.l).toBe(50);
    expect(lit.l).toBe(60);
    expect(lit.h).toBe(base.h);
    expect(lit.s).toBe(base.s);
  });
});

describe("childColorFor", () => {
  it("produces a hue within ±20° of the parent's hue", () => {
    const parents = ["proj-a", "proj-b", "another-project-uuid", "x"];
    const children = ["wt-1", "wt-2", "branch-feature-foo", "y"];
    for (const p of parents) {
      const parentHue = hashStr(p) % 360;
      for (const c of children) {
        const childHue = parseHsl(childColorFor(p, c)).h;
        // circular distance, accounting for wrap-around at 360.
        const raw = Math.abs(childHue - parentHue);
        const dist = Math.min(raw, 360 - raw);
        expect(dist).toBeLessThanOrEqual(20);
      }
    }
  });

  it("is deterministic for the same parent/child pair", () => {
    expect(childColorFor("p", "c")).toBe(childColorFor("p", "c"));
  });
});
