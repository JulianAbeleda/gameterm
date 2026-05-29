# GameTerm Renderer Path

This note records the next rendering hook after Scene Mode file loading.

The first bitmap sprite/tile implementation should use the existing pane render path, not a new compositor. The best first hook is `render_screen_line` in `gameterm-gui/src/termwindow/render/screen_line.rs`, modeled after the existing image quad path.

Useful existing pieces:

- `paint_pass` in `gameterm-gui/src/termwindow/render/paint.rs` defines the frame order: backgrounds, panes, splits, tab bar, borders, modal.
- `paint_pane` in `gameterm-gui/src/termwindow/render/pane.rs` drives per-line rendering and owns `LineQuadCacheKey`.
- `render_screen_line` in `gameterm-gui/src/termwindow/render/screen_line.rs` already emits background, glyph, cursor, and image quads.
- `populate_image_quad` in `gameterm-gui/src/termwindow/render/mod.rs` is the nearest template for atlas-backed bitmap quads.
- `RenderLayer` in `gameterm-gui/src/renderstate.rs` has three sublayers per z-index; tile/background sprites can start on sublayer 0 and entity sprites on sublayer 2.

The render-facing API from `gameterm-visual` is `SceneRuntime::render_snapshot()`. It returns data-only scene size, tile records, entity records, sprite ids, selection flags, dialogue/choice labels, and a generation number for cache invalidation.

Primary risks:

- `LineQuadCacheKey` needs a visual generation field, or sprite selection/movement/animation can reuse stale quads.
- Sprite images will share the existing glyph/image atlas, so atlas pressure needs to be watched.
- Scene tiles are grid based while pane rendering is line/cell based, so the first pass should emit only visible row-local quads.

Minimal GUI implementation order:

1. Thread an optional `VisualRenderSnapshot` into `RenderScreenLineParams`.
2. Add snapshot generation to `LineQuadCacheKey`.
3. Add a small `populate_visual_sprite_quad` beside `populate_image_quad`.
4. Emit only visible tile/entity quads first.
5. Use tile/background records on sublayer 0 and entity records on sublayer 2.
