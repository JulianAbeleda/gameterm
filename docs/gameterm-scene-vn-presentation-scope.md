# GameTerm Scene Mode VN Presentation Scope

Status: SCOPED.

This scope defines the next visual-novel pass: render VN backgrounds and
characters as a staged scene instead of only loading them as ordinary Scene
sprites.

The goal is not to embed Ren'Py or reproduce its renderer. The goal is to copy
the useful architecture at a GameTerm scale: layers, tagged displayables,
scene/show/hide operations, simple transforms, and deterministic renderer
placement.

## Ren'Py Reference Model

References inspected:

- Ren'Py docs: Displaying Images
  <https://www.renpy.org/doc/html/displaying_images.html>
- Ren'Py docs: Transforms
  <https://www.renpy.org/doc/html/transforms.html>
- Ren'Py docs: Transform Properties
  <https://www.renpy.org/doc/html/transform_properties.html>
- Ren'Py source: `renpy/display/scenelists.py`
  <https://github.com/renpy/renpy/blob/master/renpy/display/scenelists.py>
- Ren'Py source: `renpy/exports/displayexports.py`
  <https://github.com/renpy/renpy/blob/master/renpy/exports/displayexports.py>
- Ren'Py source: `renpy/display/displayable.py`
  <https://github.com/renpy/renpy/blob/master/renpy/display/displayable.py>
- Ren'Py source: `renpy/display/transform.py`
  <https://github.com/renpy/renpy/blob/master/renpy/display/transform.py>

Relevant ideas:

- Images are displayables with names. The first name component is the tag; later
  components are attributes such as emotion or costume.
- Layers are ordered lists of displayables. The default visual layer is used
  for backgrounds and character sprites.
- `scene` clears a layer and can then show a new image, commonly a background.
- `show` adds or replaces a displayable by tag on a layer.
- `hide` removes a displayable by tag.
- Z-order and `behind` control ordering inside a layer.
- Transforms control placement and presentation. Common VN usage is simple
  placement such as left, center, right, or fullscreen/background fill.

GameTerm should adopt the model shape, not the implementation. Ren'Py is
Python/SDL/displayable-tree based; GameTerm is Rust/terminal-renderer based and
must keep Scene Mode typed, inspectable, and testable.

## Current Baseline

Already working:

- VN script importer reads a conservative `.rpy` subset in Rust.
- VN asset intake copies local approved assets and generates stable sprite ids.
- VN demo helper generates `default.json`, `sprites.json`, bindings, and
  attribution.
- Strict image validation confirms real PNG data.
- Fullscreen `vn-demo` smoke opens Scene Mode and captures the GUI.
- Local PSD/image export helper can flatten downloaded character art into the
  current VN source-root layout.

Current limitation:

- `default.json.background` can reference `vn.background.school_classroom`.
- The kiki entity can reference `vn.character.kiki.neutral`.
- Rendering is still entity/grid oriented. The character is drawn in a small
  cell-sized entity rect. The background is loaded and referenced, but not
  composed as a full VN backdrop behind staged characters.
- The `.rpy` fixture does not yet contain `scene`, `show`, or `hide`.

## End Goal

When this pass is complete, a generated VN demo should visually read as a
minimal VN stage:

- background image fills the Scene viewport or the chosen stage region
- character sprites are placed at named stage positions
- expression changes update the visible character sprite by tag
- dialogue and choices remain terminal-native and readable
- Tile Debugger exposes the active stage layers/displayables
- smoke captures show the VN background and character placement clearly

The first pass should be deterministic and conservative. Animation and advanced
Ren'Py behavior can wait.

## Product Contract

The user should be able to write a small VN-shaped source like:

```renpy
label start:
    scene school_classroom
    show kiki neutral at center
    kiki "Scene Mode has a stage now."
    show kiki happy at right
    kiki "Expressions and positions can change."
    hide kiki
```

and generate a Scene Mode view that:

- uses `vn.background.school_classroom` as the active background displayable
- shows `vn.character.kiki.neutral` at center
- replaces the same `kiki` tag with `vn.character.kiki.happy`
- moves the kiki to the right stage slot
- removes the kiki after `hide kiki`

## Data Model

Add a VN-stage presentation model to `gameterm-visual`. Candidate schema:

```rust
pub struct VisualStage {
    pub viewport: VisualStageViewport,
    pub layers: Vec<VisualStageLayer>,
}

pub struct VisualStageLayer {
    pub layer_id: String,
    pub zorder: i32,
    pub displayables: Vec<VisualStageDisplayable>,
}

pub struct VisualStageDisplayable {
    pub tag: String,
    pub sprite: String,
    pub placement: VisualStagePlacement,
    pub zorder: i32,
    pub visible: bool,
}

pub enum VisualStagePlacement {
    Fullscreen,
    Left,
    Center,
    Right,
    Custom(VisualStageTransform),
}

pub struct VisualStageTransform {
    pub xalign: f32,
    pub yalign: f32,
    pub xanchor: f32,
    pub yanchor: f32,
    pub xoffset: f32,
    pub yoffset: f32,
    pub width_scale: Option<f32>,
    pub height_scale: Option<f32>,
    pub fit: VisualStageFit,
}
```

First-pass defaults:

- `background` layer zorder: `0`
- `characters` layer zorder: `10`
- `ui`/dialogue remains terminal text, not image-composited
- `Fullscreen` uses cover or contain fit; choose one and document it
- `Left`, `Center`, and `Right` use character-friendly anchor rules:
  - `yalign=1.0`
  - `yanchor=1.0`
  - character bottom aligns with the stage bottom
  - scale capped so a tall sprite fits the stage height

## Runtime Model

Add stage operations as deterministic Scene actions:

- `SetStageBackground { sprite, transition? }`
- `ShowStageDisplayable { tag, sprite, placement, layer?, zorder? }`
- `HideStageDisplayable { tag, layer? }`
- `ClearStageLayer { layer }`

The importer can emit those actions from script statements. The runtime applies
them to `VisualRuntimeState`, bumps generation, and exposes the resulting
`VisualRenderSnapshot`.

State persistence should include stage state so export/import restores the
visible VN composition.

## Importer Scope

Extend the Rust VN script importer to parse a small presentation subset:

- `scene NAME`
- `show TAG ATTRIBUTES...`
- `show TAG ATTRIBUTES... at left|center|right`
- `hide TAG`

Mapping rules:

- `scene school_classroom` -> `SetStageBackground` using bindings or a
  generated sprite id.
- `show kiki neutral` -> `ShowStageDisplayable` with tag `kiki` and sprite
  `vn.character.kiki.neutral`.
- `show kiki happy at right` -> replace tag `kiki` with
  `vn.character.kiki.happy` and placement `Right`.
- `hide kiki` -> remove the `kiki` displayable from the characters layer.

Unsupported Ren'Py properties should warn, not guess:

- ATL blocks
- arbitrary expressions
- transforms beyond the named first-pass positions
- `behind`, `zorder`, and `onlayer` until explicitly supported
- transitions from `with`

## Renderer Scope

Add staged visual quads to the GUI renderer.

Current renderer draws visual entities in cell-sized rects. The stage renderer
should compute pixel-space rectangles over the Scene pane:

- resolve stage sprite ids through the existing `VisualSpriteImages`
- draw background layer before tile/entity layers
- draw character layer above background and below text/debug UI
- preserve fallback placeholder behavior if a sprite id is missing
- respect `AllowImage::No`
- keep cache identity tied to visual generation and sprite generation

The first renderer pass can be a helper beside
`gameterm-gui/src/termwindow/render/visual_quad.rs`.

Candidate helper responsibilities:

- compute stage viewport in pixels from pane bounds
- convert placement/fit into destination rect
- draw image quad if available
- draw deterministic placeholder if unavailable
- return enough debug/test data to verify placement without a live GL context

## Fixture Changes

Update `ci/fixtures/gameterm-scene/renpy-demo-source.rpy` to include staged
presentation statements.

Example:

```renpy
label start:
    scene school_classroom
    show kiki neutral at center
    "A terminal window glows like a tiny stage."
    kiki "Scene Mode can read a VN-shaped script."
```

Add a second expression/position:

```renpy
label explain:
    show kiki happy at right
    kiki "Labels become dialogue targets, and menu items become choices."
```

The fixture should still be GameTerm-owned. Do not import Ren'Py demo script
text or third-party game text.

## Verification

Focused verification:

```sh
cargo test -p gameterm-visual vn_script_import
cargo test -p gameterm-visual stage
cargo test -p gameterm-gui visual_quad
ci/gameterm-scene-vn-demo.sh generate \
  --output-dir /tmp/gameterm-vn-demo-stage \
  --asset-source-root ci/fixtures/gameterm-scene/vn-asset-source \
  --strict-images \
  --force
ci/gameterm-scene-vn-demo.sh doctor \
  --output-dir /tmp/gameterm-vn-demo-stage \
  --strict-images
```

Full verification:

```sh
ci/gameterm-scene-verify.sh --all
git diff --check
```

Live smoke when a GUI session is available:

```sh
ci/gameterm-scene-smoke.sh \
  --launch \
  --scenario vn-demo \
  --vn-asset-source-root .cache/gameterm-scene/vn-assets \
  --wait-before-capture 3 \
  --capture-timeout 8 \
  --output /tmp/gameterm-scene-vn-demo-stage.png
```

Expected smoke result:

- screenshot clearly shows the school background as a large backdrop
- screenshot clearly shows the kiki character staged at a named position
- dialogue/choices remain readable
- Tile Debugger can report active stage displayables

## Acceptance Criteria

1. `VisualScene::validate()` accepts valid stage data and rejects empty ids,
   unknown placement names, duplicate active displayable tags within a layer,
   and invalid scale/align values.
2. `VisualRenderSnapshot` carries active stage displayables.
3. Runtime stage actions update stage state and generation deterministically.
4. Story-state export/import preserves stage state.
5. VN importer parses `scene`, `show`, and `hide` for the first-pass subset.
6. VN demo generated from fixture assets uses staged background and character
   displayables.
7. Renderer tests cover background full-stage placement and left/center/right
   character placement.
8. Missing sprite ids fall back to placeholders without crashing.
9. `ci/gameterm-scene-verify.sh --all` passes.
10. Live fullscreen VN smoke is recorded.

## Non-Goals

- Full Ren'Py engine support.
- Running Python or Ren'Py script code.
- ATL animation language.
- Transitions from `with`.
- Screen language.
- Audio, video, rollback, or save-slot parity.
- Automatic downloads from itch.io or any other asset host.
- Committing user-downloaded third-party assets without clean redistribution
  rights.

## Commit Plan

Use separate commits:

1. `[docs] scope Scene VN presentation pass`
   - this scope document
   - roadmap link/update
2. `[visual] add Scene VN stage model`
   - typed schema, validation, runtime snapshot/state
   - focused tests
3. `[visual] import Scene VN stage statements`
   - parse `scene`, `show`, `hide`
   - update fixture and generated demo
4. `[render] render Scene VN stage quads`
   - pixel placement helper
   - background and character image quads
   - renderer tests
5. `[test] verify Scene VN staged demo`
   - verifier checks and strict-image generation
6. `[docs] record Scene VN staged smoke`
   - fullscreen smoke result and capture path

## Open Design Decisions

1. Background fit: `cover` feels more VN-like; `contain` avoids cropping. Pick
   one for first pass and make it explicit.
2. Stage region: use the full pane, or reserve text rows for dialogue. First
   pass should likely use full pane for background and let terminal text remain
   on top.
3. Character scale: use image-native scale, cap to stage height, or fixed
   fraction of stage height. First pass should cap to stage height and keep
   aspect ratio.
4. Text readability: if full backgrounds make text hard to read, add a
   semi-transparent dialogue band or keep existing text background opacity.
5. Source-id ergonomics: current local PSD export writes into the
   `4cher_set4_vn_sprites` source id. The presentation pass can work without
   fixing this, but the following asset-intake polish pass should add a cleaner
   custom local source id.
