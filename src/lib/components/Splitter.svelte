<script lang="ts">
  // Vertical drag handle between two columns. The parent owns the width
  // state and updates it via the `onDrag` callback. We don't write the
  // width ourselves so this component stays purely presentational and
  // works in either side (resize the column on its left or its right).
  type Props = {
    /// "left" -> dragging right grows the column to the LEFT of the splitter.
    /// "right" -> dragging right grows the column to the RIGHT of the splitter
    ///            (i.e., shrinks the center).
    edge: "left" | "right";
    onDrag: (deltaPx: number) => void;
    onCommit?: () => void;
  };

  let { edge, onDrag, onCommit }: Props = $props();

  let dragging = $state(false);
  let lastX = 0;

  function onPointerDown(e: PointerEvent) {
    dragging = true;
    lastX = e.clientX;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
  }

  function onPointerMove(e: PointerEvent) {
    if (!dragging) return;
    const dx = e.clientX - lastX;
    lastX = e.clientX;
    // For a "right" edge, dragging right means shrinking the column on the
    // right of the splitter — invert the sign so callers can always treat
    // positive delta as "grow the column they own".
    onDrag(edge === "left" ? dx : -dx);
  }

  function onPointerUp(e: PointerEvent) {
    if (!dragging) return;
    dragging = false;
    (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
    onCommit?.();
  }
</script>

<div
  class="splitter"
  class:dragging
  role="separator"
  aria-orientation="vertical"
  onpointerdown={onPointerDown}
  onpointermove={onPointerMove}
  onpointerup={onPointerUp}
  onpointercancel={onPointerUp}
></div>

<style>
  .splitter {
    width: 5px;
    height: 100%;
    cursor: col-resize;
    background: var(--border);
    flex-shrink: 0;
    transition: background 120ms;
    /* Subtle 1px line + a wider invisible hit-target. The visible line
       lives at the inner edge so the column borders look intentional, while
       the full 5px width gives the cursor a forgiving grab area. */
    position: relative;
  }
  .splitter::after {
    content: "";
    position: absolute;
    top: 0;
    bottom: 0;
    left: 2px;
    width: 1px;
    background: var(--border);
  }
  .splitter:hover, .splitter.dragging {
    background: var(--accent);
  }
</style>
