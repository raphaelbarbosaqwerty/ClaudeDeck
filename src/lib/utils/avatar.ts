// Procedural pixel-art avatar generator. Inspired by GitHub's identicons and
// the sprites seen in DraftFrame. Produces a deterministic, vertically-symmetric
// 6x6 sprite from any string id, returned as an inline SVG data URI suitable
// for `background-image` or an `<img src>`.
//
// Why 6x6 with vertical symmetry: the human eye reads symmetric shapes as
// "creatures" — even random pixel patterns become little ghosts/blobs once
// mirrored. 6x6 is the smallest grid that produces visually distinct outputs
// for thousands of ids while staying readable at 24-32px.

import { colorFor, childColorFor } from "./color";

const GRID = 6;
const HALF = GRID / 2; // generate 3 columns, mirror to fill 6.

function hash32(s: string): number {
  // FNV-1a — small, fast, good enough for visual distribution.
  let h = 0x811c9dc5;
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i);
    h = Math.imul(h, 0x01000193);
  }
  return h >>> 0;
}

/// Bit at position i of the hash, expanded into a deterministic stream.
/// We need GRID*HALF=18 bits of "fill or not"; one 32-bit hash is plenty.
function bit(h: number, i: number): boolean {
  return ((h >>> i) & 1) === 1;
}

export interface AvatarOptions {
  /// Optional parent id; when given, the foreground hue derives from the
  /// parent so worktree avatars stay visually related to their project.
  parentId?: string;
  /// Pixel size of the rendered SVG. Defaults to 32.
  size?: number;
  /// Background color (transparent if omitted).
  background?: string;
}

export function avatarSvg(id: string, opts: AvatarOptions = {}): string {
  const { parentId, size = 32, background } = opts;
  const h = hash32(id);

  // Foreground from id (or shifted from parent for worktrees).
  const fg = parentId ? childColorFor(parentId, id) : colorFor(id);

  // Build a 6x6 grid by mirroring 3 columns.
  // We pick a slightly-above-50% fill rate so sprites don't look hollow.
  const cells: boolean[] = new Array(GRID * GRID).fill(false);
  let bitIdx = 0;
  for (let y = 0; y < GRID; y++) {
    for (let x = 0; x < HALF; x++) {
      const on = bit(h, bitIdx++);
      cells[y * GRID + x] = on;
      cells[y * GRID + (GRID - 1 - x)] = on; // mirror
    }
  }

  // Render rectangles. Slightly inset cells (95% of grid step) give the
  // sprite a soft "tile" feel instead of a solid blob.
  const step = 100 / GRID;
  const inset = step * 0.05;
  let rects = "";
  for (let y = 0; y < GRID; y++) {
    for (let x = 0; x < GRID; x++) {
      if (!cells[y * GRID + x]) continue;
      const rx = x * step + inset;
      const ry = y * step + inset;
      const rw = step - inset * 2;
      const rh = step - inset * 2;
      rects += `<rect x="${rx.toFixed(2)}" y="${ry.toFixed(2)}" width="${rw.toFixed(2)}" height="${rh.toFixed(2)}"/>`;
    }
  }

  const bg = background
    ? `<rect width="100" height="100" fill="${background}"/>`
    : "";

  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100" width="${size}" height="${size}" shape-rendering="crispEdges">${bg}<g fill="${fg}">${rects}</g></svg>`;

  return svg;
}

export function avatarDataUri(id: string, opts: AvatarOptions = {}): string {
  const svg = avatarSvg(id, opts);
  // Encode as a data URI. We URL-encode `#` since we use HSL colors which
  // don't contain it, but it's the one byte that breaks `url("data:...")`.
  return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
}
