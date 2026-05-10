import { describe, it, expect } from "vitest";
import { avatarSvg, avatarDataUri } from "./avatar";

interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

function parseRects(svg: string): Rect[] {
  const re = /<rect\s+x="([\d.]+)"\s+y="([\d.]+)"\s+width="([\d.]+)"\s+height="([\d.]+)"/g;
  const out: Rect[] = [];
  let m: RegExpExecArray | null;
  while ((m = re.exec(svg)) !== null) {
    out.push({ x: parseFloat(m[1]), y: parseFloat(m[2]), w: parseFloat(m[3]), h: parseFloat(m[4]) });
  }
  return out;
}

describe("avatarSvg", () => {
  it("returns a string starting with <svg and ending with </svg>", () => {
    const s = avatarSvg("hello");
    expect(s.startsWith("<svg")).toBe(true);
    expect(s.endsWith("</svg>")).toBe(true);
  });

  it("contains at least one <rect> element", () => {
    const s = avatarSvg("hello");
    expect(s).toMatch(/<rect\b/);
  });

  it("is deterministic for the same id", () => {
    expect(avatarSvg("alice")).toBe(avatarSvg("alice"));
  });

  it("different ids produce different outputs", () => {
    expect(avatarSvg("alice")).not.toBe(avatarSvg("bob"));
  });

  it("background option includes a full-canvas background rect with that fill", () => {
    const s = avatarSvg("alice", { background: "#000" });
    expect(s).toContain('<rect width="100" height="100" fill="#000"/>');
  });

  it("each row is mirrored left-right (vertical symmetry)", () => {
    // Sample several ids to be sure.
    const ids = ["a", "alice", "workspace-1", "long-uuid-abc-123", "z"];
    for (const id of ids) {
      const svg = avatarSvg(id);
      // Strip out the background rect (width=100) — we only want the cell rects.
      const rects = parseRects(svg).filter((r) => r.w < 99);

      // Group by y (row).
      const byRow = new Map<string, Rect[]>();
      for (const r of rects) {
        const k = r.y.toFixed(2);
        if (!byRow.has(k)) byRow.set(k, []);
        byRow.get(k)!.push(r);
      }

      for (const [, row] of byRow) {
        const xs = new Set(row.map((r) => r.x.toFixed(2)));
        for (const r of row) {
          // Mirror x around the 100-wide canvas: mirroredX = 100 - x - w
          const mirroredX = (100 - r.x - r.w).toFixed(2);
          expect(xs.has(mirroredX)).toBe(true);
        }
      }
    }
  });
});

describe("avatarDataUri", () => {
  it("returns a data:image/svg+xml;utf8,... uri", () => {
    const uri = avatarDataUri("hello");
    expect(uri.startsWith("data:image/svg+xml;utf8,")).toBe(true);
    // Body should be URL-encoded — no raw '<' allowed.
    const body = uri.slice("data:image/svg+xml;utf8,".length);
    expect(body).not.toContain("<");
    expect(body).toContain("%3Csvg");
  });
});
