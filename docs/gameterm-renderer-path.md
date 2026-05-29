# GameTerm Renderer Path

This note records the follow-up design after Scene Mode file loading. It is
documentation only; no renderer code is changed here.

The first bitmap sprite/tile implementation should use the existing pane render path, not a new compositor. The best first hook is `render_screen_line` in `gameterm-gui/src/termwindow/render/screen_line.rs`, modeled after the existing image quad path.

Useful existing pieces:

- `paint_pass` in `gameterm-gui/src/termwindow/render/paint.rs` defines the frame order: backgrounds, panes, splits, tab bar, borders, modal.
- `paint_pane` in `gameterm-gui/src/termwindow/render/pane.rs` drives per-line rendering and owns `LineQuadCacheKey`.
- `render_screen_line` in `gameterm-gui/src/termwindow/render/screen_line.rs` already emits background, glyph, cursor, and image quads.
- `populate_image_quad` in `gameterm-gui/src/termwindow/render/mod.rs` is the nearest template for atlas-backed bitmap quads.
- `RenderLayer` in `gameterm-gui/src/renderstate.rs` has three sublayers per z-index; tile/background sprites can start on sublayer 0 and entity sprites on sublayer 2.

The render-facing API from `gameterm-visual` is `SceneRuntime::render_snapshot()`. It returns data-only scene size, tile records, entity records, sprite ids, selection flags, dialogue/choice labels, and a generation number for cache invalidation.

## `populate_visual_sprite_quad` follow-up design

Add `populate_visual_sprite_quad` beside `populate_image_quad` in
`gameterm-gui/src/termwindow/render/mod.rs`. Keep it small and bitmap-specific:
it should receive one already-filtered visual sprite record, resolve that
record's atlas sprite, compute the pane-local quad rectangle from the cell
metrics already used by `render_screen_line`, and push a single
atlas-backed quad into the existing render state.

The helper should not decide which scene records are visible. That filtering
belongs in the line renderer before the helper is called, so the helper can stay
equivalent to `populate_image_quad`: convert one sprite placement into one quad.
Expected call-site inputs are:

- pane-local origin for the current line;
- current row index in scene/tile coordinates;
- cell width and line height;
- sprite id or atlas handle from the visual record;
- tile/entity grid position and dimensions;
- target `RenderLayer` sublayer;
- selection/active flags only if they affect tint, outline, or cache identity.

Row-local filtering should happen in `render_screen_line`:

- tiles: iterate only tile records whose `tile.y == current_scene_row`, then
  skip records with `tile.x` outside the visible pane columns;
- entities: include records whose occupied grid rectangle intersects the
  current scene row, then clip or offset the quad vertically when the entity is
  taller than one row;
- both paths: skip records outside the pane viewport before atlas lookup, so
  off-screen sprites do not create cache pressure;
- keep tile/entity filtering separate at first, because entities can span rows
  or need different visual state while tiles are row-aligned background data.

Sublayer choices should remain within the current pane z-index:

- sublayer 0: scene tiles, floor, terrain, and other background sprites;
- sublayer 1: reserved for future overlays that must sit above tiles but below
  actors, such as path previews, targeting highlights, or selection fills;
- sublayer 2: entities, items, cursors, and other foreground sprites.

This keeps Scene Mode inside the normal pane ordering. It avoids a new
compositor and lets borders, modal UI, and later frame passes continue to draw
over pane content.

## Cache and atlas risks

- `LineQuadCacheKey` needs the visual snapshot generation. Without it, movement,
  selection changes, tile swaps, or animation frame changes can reuse stale
  line quads.
- If animation is time-based rather than generation-based, the cache key also
  needs an animation frame bucket or the caller must invalidate visual lines
  when frames advance.
- Sprite images will share the existing glyph/image atlas path. Repeated
  off-screen lookups or per-frame sprite allocation can evict useful atlas
  entries and create `OutOfTextureSpace` failures.
- Atlas resolution should be keyed by stable sprite id plus image generation,
  not by transient entity id, so multiple entities using the same sprite share
  atlas entries.
- Missing sprite ids should degrade to a deterministic placeholder or no-op
  quad, and should not poison the line cache with an unrecoverable failure.
- Cache invalidation must include any flags that affect the emitted quad, such
  as selected/active tint, flip state, or animation frame.

## Next-step checklist

1. Thread an optional `VisualRenderSnapshot` into `RenderScreenLineParams`.
2. Add snapshot generation to `LineQuadCacheKey`.
3. Add row-local tile filtering in `render_screen_line`.
4. Add row-intersection entity filtering in `render_screen_line`.
5. Add `populate_visual_sprite_quad` beside `populate_image_quad`.
6. Emit tile/background sprites on sublayer 0.
7. Emit entity/foreground sprites on sublayer 2.
8. Verify atlas reuse by sprite id and no atlas lookup for skipped off-screen records.
