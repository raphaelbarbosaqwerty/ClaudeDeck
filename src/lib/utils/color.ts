// Deterministic color from a UUID, so each workspace/session gets a stable
// avatar tint. We use HSL with fixed S/L so all generated colors share a
// coherent palette weight (no harsh primaries, no washed-out pastels).

function hashStr(s: string): number {
  let h = 0;
  for (let i = 0; i < s.length; i++) {
    h = (h * 31 + s.charCodeAt(i)) | 0;
  }
  return Math.abs(h);
}

export function colorFor(id: string, opts?: { lighten?: number }): string {
  const hue = hashStr(id) % 360;
  const saturation = 55;
  const lightness = 50 + (opts?.lighten ?? 0);
  return `hsl(${hue} ${saturation}% ${lightness}%)`;
}

/// Derive a child color from a parent's id, shifting hue slightly so worktrees
/// stay visually related to the parent project but distinguishable.
export function childColorFor(parentId: string, childId: string): string {
  const parentHue = hashStr(parentId) % 360;
  const offset = (hashStr(childId) % 40) - 20; // ±20° drift
  const hue = (parentHue + offset + 360) % 360;
  return `hsl(${hue} 55% 58%)`;
}
