# GameTerm Scene Mode Aspect-Safe Image Scope

Status: SCOPED.

This document scopes the pass to make Scene Mode image assets preserve their
source aspect ratio when the GameTerm window moves between fullscreen,
maximized, and smaller windowed sizes.

## Problem

Some visual-novel assets look correct in one window size and distorted in
another. The most visible failure is character art stretching when Scene Mode
resizes from fullscreen to a smaller window.

The likely local cause is in
`gameterm-gui/src/termwindow/render/visual_quad.rs`:

```rust
fn stage_displayable_rect(displayable: &VisualRenderStageDisplayable, stage_rect: RectF) -> RectF {
    match displayable.placement {
        VisualStagePlacement::Fullscreen => stage_rect,
        VisualStagePlacement::Left | VisualStagePlacement::Center | VisualStagePlacement::Right => {
            let height = (stage_rect.size.height * 0.78).max(1.0);
            let width = height;
            ...
        }
    }
}
```

That code gives non-fullscreen staged displayables a square destination rect.
If the source sprite is not square, the GPU quad maps the sprite texture into a
different aspect ratio. Resizing can make the distortion more noticeable.

## Reference Model

Open-source/game-engine references point to the same primitive:

- Bevy `SpriteScalingMode` defines `Fill*` and `Fit*` variants that scale
  textures uniformly while maintaining aspect ratio.
- Godot multiple-resolution settings distinguish distorted stretching from
  aspect-preserving keep/expand behavior.
- Libretro/RetroArch separates the game viewport from fullscreen overlays,
  which maps well to GameTerm's stage images vs VN dialogue/composer UI.

Useful references:

- <https://docs.rs/bevy/latest/bevy/sprite/enum.SpriteScalingMode.html>
- <https://docs.godotengine.org/en/4.5/tutorials/rendering/multiple_resolutions.html>
- <https://docs.libretro.com/development/retroarch/input/overlay/>

## Product Goal

Scene Mode should render visual assets with stable proportions across window
sizes:

- VN backgrounds fill or fit the stage without visually squashing.
- VN character sprites keep their original proportions.
- Character sprites stay anchored to the stage bottom for VN-style staging.
- UI panels, nameplates, dialogue text, and Composer dock continue to use the
  existing VN overlay layout and are not governed by image aspect policies.
- Resizing recomputes image rectangles deterministically without needing to
  reload or mutate the image assets.

## Non-Goals

This pass should not:

- import a full game engine
- change the existing VN panel/nameplate layout
- change dialogue/composer text placement
- rewrite sprite manifest format unless a small optional field is needed
- implement animation, skeletal sprites, or PSD layer rendering
- add a global terminal scaling mode
- solve pixel-perfect integer scaling for all tilemap content unless the core
  primitive makes it trivial to expose later

## Core Primitives

Add small internal rendering primitives near the image quad code.

```rust
enum VisualImageScaleMode {
    Stretch,
    FitCenter,
    FitBottomCenter,
    FillCenter,
    IntegerFitCenter,
}

struct VisualImageSourceSize {
    width: f32,
    height: f32,
}

fn resolve_aspect_rect(
    source: VisualImageSourceSize,
    target: RectF,
    mode: VisualImageScaleMode,
) -> RectF
```

Behavior:

- `Stretch`: use `target` exactly. This preserves old behavior and is useful
  for explicitly stretchable surfaces.
- `FitCenter`: uniform scale so the full source fits inside target; center the
  result.
- `FitBottomCenter`: uniform scale so the full source fits inside target;
  center horizontally and align to target bottom.
- `FillCenter`: uniform scale so target is fully covered; center the result.
  This may crop visually if paired with source-rect cropping later.
- `IntegerFitCenter`: uniform whole-number scale for pixel-art assets; center
  result. If target is smaller than source, clamp to a minimum nonzero scale or
  fall back to `FitCenter`.

First pass can implement `FillCenter` by allowing destination overflow inside
the stage if clipping already happens, or by adding source-rect cropping if the
renderer does not clip. Prefer destination-only `FillCenter` only if it cannot
bleed outside the stage or is covered by stage bounds.

## Recommended Defaults

Stage displayables:

| Placement | Default mode | Rationale |
| --- | --- | --- |
| `Fullscreen` | `FillCenter` | VN backgrounds should fill the stage without squashing. |
| `Left` | `FitBottomCenter` | Characters should preserve proportions and stand on the bottom edge. |
| `Center` | `FitBottomCenter` | Same as above. |
| `Right` | `FitBottomCenter` | Same as above. |

Tilemap/background grid:

- Keep current tile/cell behavior in this pass unless it is directly causing
  VN asset distortion.
- If tile sprites need aspect safety later, add a separate tile policy so map
  cells can remain intentionally square/rectangular.

VN UI panels:

- Do not use image aspect logic for panels/nameplates.
- Panels are UI textures/sliced rounded boxes and should continue to stretch to
  their layout rects.

## Implementation Lanes

### Lane 1: Aspect Resolver

Add `VisualImageScaleMode`, `VisualImageSourceSize`, and
`resolve_aspect_rect(...)` in `visual_quad.rs` or a small sibling module if the
file becomes too dense.

Acceptance:

- zero/invalid source sizes return the target rect or a safe 1px fallback
- `FitCenter` preserves `resolved.width / resolved.height`
- `FitBottomCenter` preserves aspect and anchors to `target.max_y()`
- `FillCenter` preserves aspect and covers target on both axes
- `IntegerFitCenter` returns whole-number scale when target is larger

### Lane 2: Source Image Dimensions

Thread source dimensions into the stage displayable rect calculation.

Likely change:

- `stage_displayable_rect(...)` should accept the loaded/cached sprite image
  dimensions or a small source-size helper.
- The current flow resolves the destination rect before calling
  `populate_visual_sprite_quad(...)`; this may need to split image lookup from
  quad population for staged displayables.

Candidate shape:

```rust
let Some(image_data) = visual_sprite_image_data(...);
let source = visual_image_source_size(image_data);
let target = stage_displayable_target_rect(displayable, stage_rect);
let rect = resolve_aspect_rect(source, target, mode);
self.populate_visual_sprite_quad(..., rect, ...);
```

Acceptance:

- non-square sprites no longer get square destination rects unless explicitly
  configured as `Stretch`
- fallback placeholders still render if image lookup fails
- no extra image decode is introduced beyond existing cached image path

### Lane 3: Stage Placement Targets

Separate "where the asset is allowed to live" from "how the image fits inside
that target."

Candidate helpers:

```rust
fn stage_displayable_target_rect(
    displayable: &VisualRenderStageDisplayable,
    stage_rect: RectF,
) -> RectF

fn stage_displayable_scale_mode(
    displayable: &VisualRenderStageDisplayable,
) -> VisualImageScaleMode
```

Initial target behavior:

- `Fullscreen`: full `stage_rect`
- `Left`/`Center`/`Right`: a tall target region derived from stage height and a
  placement-specific center point
- character target may be wider than its final sprite rect so wide sprites can
  fit without stretching

Acceptance:

- placement math remains deterministic and easy to test
- existing left/center/right staging still looks like VN character staging
- no hardcoded square `width = height` remains for real image quads

### Lane 4: Tests

Add focused unit tests in `visual_quad.rs`.

Test cases:

- `fit_center_preserves_aspect_for_wide_source`
- `fit_bottom_center_preserves_aspect_and_bottom_anchor`
- `fill_center_preserves_aspect_and_covers_target`
- `integer_fit_center_uses_whole_scale`
- `stage_displayable_rect_preserves_character_aspect`
- `fullscreen_background_uses_fill_policy`

Recommended numeric checks:

```rust
assert_approx_eq(rect.size.width / rect.size.height, source.width / source.height);
assert_approx_eq(rect.max_y(), target.max_y());
assert!(rect.size.width <= target.size.width);
assert!(rect.size.height <= target.size.height);
```

For fill:

```rust
assert!(rect.size.width >= target.size.width);
assert!(rect.size.height >= target.size.height);
```

### Lane 5: Smoke

Run two visual smoke captures with the same VN demo:

```sh
cargo build -p gameterm-gui
ci/gameterm-scene-smoke.sh --launch --scenario vn-compose \
  --output /tmp/gameterm-scene-aspect-vn-fullscreen.png

ci/gameterm-scene-smoke.sh --launch --scenario renderer-rows-windowed \
  --output /tmp/gameterm-scene-aspect-windowed.png
```

If the smoke harness can run `vn-compose` with an explicit window size, prefer
that over `renderer-rows-windowed`:

```sh
ci/gameterm-scene-smoke.sh --launch --scenario vn-compose \
  --window-size 1000x560 \
  --output /tmp/gameterm-scene-aspect-vn-windowed.png
```

Acceptance:

- fullscreen and windowed captures show the same character proportions
- background does not squash
- dialogue/composer panels remain aligned with existing VN overlay layout
- no obvious image bleed over dialogue/composer panels unless intentionally
  layered behind them

## Risks

- If `FillCenter` is implemented as destination overflow without clipping, a
  background may draw outside the stage area.
- If source dimensions are taken from padded cached textures rather than the
  original image, the computed aspect ratio may be wrong.
- Character art may need per-sprite intended scale in the future; this pass
  should not overfit one downloaded sprite.
- Tile sprites may intentionally fill cells and should not be "fixed" into
  letterboxed tiny tiles without a separate tile policy.

## Commit Plan

Use separate commits by concern:

```text
[docs] scope Scene aspect-safe image placement
[visual] add Scene image aspect resolver
[visual] preserve staged image aspect ratios
[test] cover Scene image aspect resizing
[docs] record Scene aspect smoke
```

If implementation can stay entirely in one focused behavior commit plus tests,
that is acceptable:

```text
[visual] preserve Scene staged image aspect ratios
```

## Definition Of Done

- Scope document is linked from the roadmap.
- Aspect resolver tests pass.
- Staged VN character sprites preserve aspect ratio across fullscreen and
  windowed target rects.
- VN backgrounds preserve aspect ratio using a fill/cover policy.
- Existing VN panel/nameplate/text tests remain green.
- `cargo test -p gameterm-gui visual_quad --bin gameterm-gui` passes.
- `cargo check -p gameterm-gui` passes with only pre-existing warning noise.
- Smoke captures are recorded in the smoke report if a visual pass is run.
