# GameTerm Renderer Path

This note records the renderer path for Scene Mode. The original placeholder
quad plan has now landed; this document tracks the current state and the next
renderer cleanup lane.

The first bitmap sprite/tile step should stay inside the existing pane render
path, not introduce a new compositor. The implementation should extend
`render_screen_line` in
`gameterm-gui/src/termwindow/render/screen_line.rs`, modeled after the current
image quad path.

Useful existing pieces:

- `paint_pass` in `gameterm-gui/src/termwindow/render/paint.rs` defines the
  frame order: backgrounds, panes, splits, tab bar, borders, modal.
- `paint_pane` in `gameterm-gui/src/termwindow/render/pane.rs` drives per-line
  rendering and owns `LineQuadCacheKey`.
- `render_screen_line` in
  `gameterm-gui/src/termwindow/render/screen_line.rs` already emits
  background, glyph, cursor, and image quads.
- `populate_image_quad` in `gameterm-gui/src/termwindow/render/mod.rs` is the
  nearest template for atlas-backed bitmap quads.
- `SceneRuntime::render_snapshot()` in `gameterm-visual` returns the data-only
  scene records, selected flags, labels, and generation counter that the GUI
  renderer should consume.

## Implemented Renderer Path

Scene Mode now threads `VisualRenderSnapshot` metadata through the pane render
path and emits visual records inside the normal pane ordering. The renderer:

- reads `gameterm_visual_snapshot` pane metadata once per pane render path;
- keys line quad caching with the visual generation;
- filters visual tiles and entities by row through helpers in
  `gameterm-visual::render` before emitting quads;
- renders deterministic placeholder blocks when no sprite image is available;
- resolves sprite manifest entries into cached image quads when images are
  available;
- keeps sprite image data cached per pane metadata and prunes that cache when
  panes are removed.

This keeps Scene Mode additive: no non-GameTerm terminal path needs a separate
compositor, and pane borders/modal UI still draw above Scene Mode content.

## Next Renderer Cleanup Lane

The remaining renderer work is no longer "make sprites appear"; it is about
making the visual path cheaper, easier to audit, and ready for richer state.

1. Move visual quad population into a small dedicated module under
   `gameterm-gui/src/termwindow/render/`.
2. Keep the cache identity contract explicit: every visual property that
   changes emitted geometry, tint, sprite frame, or selection treatment must be
   represented in `VisualRenderSnapshot.generation` or a future cache key field.
3. Add fixture-backed tests for cache invalidation at the helper boundary
   instead of relying only on full renderer smoke checks. Row filtering already
   has helper-level tests in `gameterm-visual`.
4. Keep missing sprite ids recoverable. A sprite manifest reload must be able to
   replace placeholders without restarting the pane.
5. Defer a true packed atlas until there are enough distinct sprites to justify
   the complexity. The current cached image path is sufficient for the
   authoring MVP.

## Historical Implementation Plan

### 1. Thread real snapshots

Thread an optional `VisualRenderSnapshot` from the Scene Mode runtime into the
pane render path and then into `RenderScreenLineParams`.

The snapshot should be captured once for the frame or pane pass and shared by
reference while rendering each line. Do not call `render_snapshot()` from inside
the line loop; that would make generation handling unclear and could rebuild
tile/entity vectors per row.

When no visual scene is active, keep the snapshot field as `None` and preserve
the existing terminal-only render behavior.

### 2. Add `visual_generation` to cache identity

Add the snapshot generation to `LineQuadCacheKey` as `visual_generation:
Option<u64>`.

The value should be:

- `Some(snapshot.generation)` when a visual snapshot is present for the pane.
- `None` when the pane has no active visual snapshot.

This prevents stale cached line quads after selection changes, movement, tile
swaps, dialogue state, or future animation changes. If animation advances
without changing the snapshot generation, the cache key will also need a frame
bucket or the caller must invalidate the affected visual lines.

Any visual flag that changes emitted quad geometry, tint, outline, flip state,
or atlas frame must be represented either in the snapshot generation or in a
future cache-key component.

### 3. Keep row filtering local and explicit

Add small row-local helpers near `render_screen_line` before adding atlas
resolution. The helpers should filter records before any sprite lookup:

- `visual_tiles_for_row(snapshot, row, visible_cols)`:
  return only tile records whose `position.y == row` and whose `position.x`
  intersects the visible pane columns.
- `visual_entities_for_row(snapshot, row, visible_cols)`:
  return entities whose occupied grid rectangle intersects `row` and whose
  horizontal range intersects the visible pane columns.

Keep tile and entity helpers separate. Tile records are row-aligned background
data; entities may later span rows, carry selection state, or need different
z-order and clipping.

The first entity implementation can treat entities as one-cell records because
`VisualRenderEntity` currently exposes a single position. The helper shape
should still be row-intersection friendly so multi-cell entities can be added
without rewriting the call site.

### 4. Emit deterministic placeholder quads first

Before real sprite atlas assets exist, emit deterministic placeholder quads for
visual records instead of blocking on asset loading.

Placeholder rules:

- Derive the placeholder color from stable inputs such as sprite id, layer, and
  selected state.
- Use the same sprite id to produce the same placeholder across frames.
- Use a visibly different treatment for selected entities, such as a stable
  tint or outline encoded in the emitted quad state.
- Do not allocate atlas entries for skipped off-screen records.
- Do not cache a missing sprite id as an unrecoverable failure.

After real sprite atlas assets land, keep the same filtering and cache-key
shape. Replace only the placeholder quad body with sprite-id atlas resolution.

### 5. Convert one visual record into one quad

Add a small `populate_visual_sprite_quad` helper beside `populate_image_quad` in
`gameterm-gui/src/termwindow/render/mod.rs`.

The helper should receive one already-filtered tile or entity record and push
one pane-local quad. It should not decide which scene records are visible.
Expected inputs are:

- pane-local origin for the current line;
- current scene row;
- cell width and line height;
- sprite id from the visual record;
- grid position and dimensions;
- target `RenderLayer` sublayer;
- selected/active flags only when they affect emitted quad state.

Use the current pane z-index and existing sublayers:

- sublayer 0: scene tiles, floor, terrain, and other background sprites;
- sublayer 1: reserved for overlays such as path previews, targeting
  highlights, or selection fills;
- sublayer 2: entities, items, cursors, and other foreground sprites.

This keeps Scene Mode inside the normal pane ordering so borders, modal UI, and
later frame passes continue to draw over pane content.

## Next Checklist

1. Capture one optional `VisualRenderSnapshot` for the active visual pane.
2. Thread the snapshot reference into `RenderScreenLineParams`.
3. Populate `LineQuadCacheKey.visual_generation` from the snapshot.
4. Add row-local tile and entity filtering helpers near `render_screen_line`.
5. Emit deterministic placeholder quads for filtered tiles on sublayer 0.
6. Emit deterministic placeholder quads for filtered entities on sublayer 2.
7. Verify skipped off-screen records do not perform atlas or placeholder work.
8. Swap placeholder quads for atlas-backed sprite quads once sprite assets land.
