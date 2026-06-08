# GameTerm Scene Character Asset Editing Scope

Status: first-pass implemented.

This document scopes a Rust-first terminal asset helper for modifying existing
Scene Mode character PNGs, generating expression variants, and keeping those
variants usable through the existing VN config module.

The goal is not to build a full Krita or Clip Studio replacement. The goal is a
repeatable GameTerm helper that can take a base character image, apply explicit
edits, generate expression or animation frames, and install the outputs into
Scene Mode config.

## Goal

Make this workflow possible:

```text
base character PNG
-> map facial/body feature regions with the user
-> apply deterministic edit recipes from terminal
-> generate expression and animation PNGs
-> validate continuity across frames
-> register generated sprites in the Scene config module
-> dogfood them in Scene Mode
```

Example end-user shape:

```sh
cargo run -p gameterm-visual --example scene_asset_edit -- \
  expression \
  --base ~/.config/gameterm/scenes/assets/vn-demo/characters/kiki-neutral.png \
  --feature-map ~/.config/gameterm/scenes/assets/vn-demo/kiki.features.json \
  --recipe ~/.config/gameterm/scenes/assets/vn-demo/kiki.expressions.json \
  --expression concerned \
  --output ~/.config/gameterm/scenes/assets/vn-demo/characters/kiki-concerned.png
```

The helper should be dogfoodable from a normal terminal first. Scene Mode can
surface it later as an action.

First-pass implementation:

- `gameterm-visual/src/asset_edit.rs` owns image inspection, feature-map
  validation, deterministic PNG edit operations, animation-frame generation,
  continuity checks, magic/background selection, and source-root export.
- `gameterm-visual/examples/scene_asset_edit.rs` exposes the terminal helper
  commands.
- `ci/fixtures/gameterm-scene/kiki-expression-recipes.json` provides a
  deterministic Kiki recipe book for expression, blink, and breathing frame
  smoke tests.
- The Rust helper can generate output PNGs that feed the existing
  `scene_vn_asset_intake` config-module path.

## Product End State

The desired first-pass end state:

1. A Rust helper can inspect a PNG and emit a feature-map template.
2. A user can adjust feature regions for eyes, brows, mouth, head, torso, and
   other visible anchors.
3. Expression recipes can erase, draw, transform, and composite controlled
   regions without hand-editing every output PNG.
4. Blink and breathing frame recipes can generate repeatable animation frames
   while preserving character continuity.
5. Generated PNGs are validated for resolution, alpha, anchor drift, and
   output existence before they are installed.
6. Existing VN asset intake remains the source of truth for sprite IDs,
   bindings, attribution, and config-module ownership.
7. The helper remains pure Rust for image operations. Shell scripts may remain
   thin launch wrappers only.

## Reference Repos

These projects are references for primitives, not dependencies to vendor.

| Repo | Useful primitive | What GameTerm should borrow | What GameTerm should avoid |
| --- | --- | --- | --- |
| Krita | document, layers, brush engines, color pipeline | the idea that a painting app is a canvas plus operations over image buffers | its large desktop/editor architecture |
| MyPaint | brush dabs and stroke sampling | simple paint-operation primitives for freehand strokes later | tablet/pressure complexity in the first pass |
| Pinta | lightweight raster editing tools | simple command surface: erase, fill, draw, crop, layer-like overlays | broad photo-editor scope |
| miniPaint | browser canvas with layers, filters, selections | selection/region operations and JSON-ish layer state | JavaScript/browser implementation model |
| Pixelorama | sprites, frames, animation timeline | expression frames, blink/breath frame export, sprite-sheet thinking | full pixel-art editor UX in the first pass |
| Graphite | procedural/non-destructive graphics | recipe graph as data, editable operations rather than destructive-only output | full node UI |
| Imogen | procedural texture node graph | generate assets from parameterized operations and export images | material/PBR-specific features |
| Drawpile | event log of drawing operations | later undo/replay model for edit recipes | collaboration/networking |

## Existing GameTerm Pieces

The new helper should connect to current assets instead of replacing them.

### VN asset intake

Existing Rust path:

```text
gameterm-visual/src/vn_asset_intake.rs
gameterm-visual/examples/scene_vn_asset_intake.rs
```

Current responsibility:

- copy approved local VN assets into config-owned output paths
- generate `sprites.json`
- generate character expression bindings
- generate attribution
- refuse unsupported or blocked catalog entries

The new asset editor should feed this path. It should not create a separate
sprite manifest system.

### Local image export helper

Existing shell path:

```text
ci/gameterm-scene-vn-image-export.sh
```

Current responsibility:

- flatten a local PSD/PNG/TIFF-style source through macOS `sips`
- copy one flattened image into many expression filenames

Gap:

- it does not edit eyes, mouths, brows, or animation frames
- it does not validate character continuity
- it is shell/macOS-specific and not the Rust-owned terminal helper the user
  wants

The future Rust helper should absorb its durable behavior:

```text
source image flatten/copy
-> expression output naming
-> source-root layout compatible with scene_vn_asset_intake
```

The shell script can remain as a compatibility wrapper or be retired after the
Rust replacement is dogfooded.

### Panel codegen helper

Existing Rust path:

```text
gameterm-visual/examples/scene_panel_style.rs
docs/gameterm-scene-panel-codegen-scope.md
```

Current responsibility:

- inspect a VN panel PNG
- emit compact procedural style data
- preserve optional PNG nine-slice compatibility

Important distinction:

```text
panel PNG -> compact renderer style
character PNG -> feature map + edit recipes + output PNGs
```

Panels should not be converted into raw pixel arrays. Character sprites should
not be forced into procedural style constants. The shared idea is not "PNG to a
huge source file"; it is "PNG to a useful, smaller, repeatable representation."

## Core Primitives

### 1. Image document

Decode each source image into an editable RGBA buffer.

Required metadata:

- source path
- width and height
- color type when available
- alpha presence
- transparent pixel ratio
- content bounding box
- checksum for repeatability

First-pass format:

```json
{
  "asset_document_version": 1,
  "source": "kiki-neutral.png",
  "width": 1024,
  "height": 2048,
  "content_bounds": { "x": 128, "y": 12, "w": 760, "h": 1990 },
  "sha256": "..."
}
```

### 2. Feature map

Feature mapping is the part the user and Codex can do together. The helper
should emit a template, then the user can tune coordinates.

Use normalized coordinates so maps survive reasonable asset-size changes:

```json
{
  "feature_map_version": 1,
  "character": "kiki",
  "base": "characters/kiki-neutral.png",
  "regions": {
    "left_eye": { "x": 0.365, "y": 0.285, "w": 0.085, "h": 0.052 },
    "right_eye": { "x": 0.535, "y": 0.285, "w": 0.085, "h": 0.052 },
    "mouth": { "x": 0.465, "y": 0.465, "w": 0.070, "h": 0.035 },
    "brows": { "x": 0.345, "y": 0.245, "w": 0.310, "h": 0.045 },
    "torso": { "x": 0.250, "y": 0.560, "w": 0.500, "h": 0.380 }
  },
  "anchors": {
    "head_center": { "x": 0.500, "y": 0.330 },
    "mouth_center": { "x": 0.500, "y": 0.482 },
    "feet_bottom": { "x": 0.500, "y": 0.995 }
  }
}
```

The helper should validate:

- regions are inside the image
- region names are unique
- normalized values are finite and non-negative
- anchors are inside the image
- output pixel rectangles are non-empty

### 3. Edit operations

First-pass operations should stay deterministic and explainable:

```text
erase_region
fill_region
draw_line
draw_polyline
draw_ellipse
draw_arc
draw_cubic_bezier
composite_png
mirror_region
translate_region
scale_region
opacity
color_tint
magic_erase
remove_background
crop_to_content
resize_contain
```

These cover expression edits such as:

- erase eye detail inside an eye region
- draw closed, happy, concerned, or surprised eye shapes
- draw mouth variants
- composite a hand-authored eye or mouth patch
- shift torso/head pixels subtly for breathing frames
- tint or soften small regions
- select a contiguous color region from a seed point and make it transparent
- select background-like pixels from corners or edges and make them transparent

Deferred operations:

- semantic inpainting
- freehand polygon lasso
- AI generation
- pressure-sensitive brush strokes
- advanced layer blending modes
- arbitrary PSD layer parsing

### 4. Expression recipes

Recipes are explicit command data. They should keep visible text and generated
artifacts reproducible.

Example:

```json
{
  "expression_recipe_version": 1,
  "character": "kiki",
  "expressions": {
    "surprised": [
      { "op": "erase_region", "region": "mouth", "soften": 2 },
      {
        "op": "draw_ellipse",
        "region": "mouth",
        "stroke": "#2b1f24ff",
        "fill": "#f2c6c8ff",
        "width": 2
      },
      { "op": "erase_region", "region": "left_eye", "soften": 1 },
      { "op": "composite_png", "region": "left_eye", "path": "parts/eyes/surprised-left.png" },
      { "op": "composite_png", "region": "right_eye", "path": "parts/eyes/surprised-right.png" }
    ]
  }
}
```

Patch PNGs are allowed because they preserve art quality. The helper should
make the patch placement repeatable.

### 5. Animation recipes

Blinking and breathing should be separate animation recipes, not one combined
idle output.

Blink:

```text
neutral eye
-> partial close
-> closed
-> partial open
-> neutral
```

Breathing:

```text
neutral torso/head
-> subtle inhale offset/scale
-> hold
-> subtle exhale offset/scale
-> neutral
```

First-pass animation config:

```json
{
  "animation_recipe_version": 1,
  "character": "kiki",
  "animations": {
    "blink": {
      "fps": 12,
      "frames": [
        { "expression": "neutral", "duration_ms": 500 },
        { "expression": "blink.1", "duration_ms": 60 },
        { "expression": "blink.2", "duration_ms": 80 },
        { "expression": "blink.1", "duration_ms": 60 },
        { "expression": "neutral", "duration_ms": 500 }
      ]
    },
    "breath": {
      "fps": 8,
      "frames": [
        { "expression": "breath.0", "duration_ms": 180 },
        { "expression": "breath.1", "duration_ms": 180 },
        { "expression": "breath.2", "duration_ms": 180 },
        { "expression": "breath.3", "duration_ms": 180 },
        { "expression": "breath.4", "duration_ms": 180 },
        { "expression": "breath.5", "duration_ms": 180 }
      ]
    }
  }
}
```

Scene Mode can later decide how to select and time these frames. The asset
helper only needs to generate and validate them.

### 6. Continuity validation

Generated frames should be checked before install.

Required first-pass checks:

- all generated frames have identical dimensions
- all generated frames preserve alpha where expected
- content bounding box does not drift beyond a configured tolerance
- configured anchors stay within tolerance
- frame-to-frame pixel diff is neither zero nor huge for animation frames
- generated sprite IDs match the intake helper's expected naming scheme

Example report:

```json
{
  "character": "kiki",
  "checks": [
    { "name": "dimensions", "status": "pass", "value": "1024x2048" },
    { "name": "head_center_drift", "status": "pass", "max_pixels": 2 },
    { "name": "blink_diff", "status": "pass", "max_changed_ratio": 0.018 }
  ],
  "warnings": []
}
```

### 7. Scene integration

After outputs are generated, the helper should be able to install them through
the config module path:

```text
generated PNGs
-> source-root layout
-> scene_vn_asset_intake
-> sprites.json
-> bindings
-> attribution
-> Scene doctor
-> smoke
```

No generated local art should be committed by default. Repo fixtures should
remain small and license-safe.

## Proposed CLI Surface

Use one Rust helper with explicit subcommands:

```sh
cargo run -p gameterm-visual --example scene_asset_edit -- inspect IMAGE
cargo run -p gameterm-visual --example scene_asset_edit -- map-template IMAGE --character kiki --output kiki.features.json
cargo run -p gameterm-visual --example scene_asset_edit -- validate-map --image IMAGE --feature-map kiki.features.json
cargo run -p gameterm-visual --example scene_asset_edit -- expression --base IMAGE --feature-map MAP --recipe RECIPES --expression happy --output OUT
cargo run -p gameterm-visual --example scene_asset_edit -- animation --base IMAGE --feature-map MAP --recipe RECIPES --animation blink --output-dir DIR
cargo run -p gameterm-visual --example scene_asset_edit -- remove-background --source IMAGE --output OUT --tolerance 24 --feather 1
cargo run -p gameterm-visual --example scene_asset_edit -- magic-erase --source IMAGE --output OUT --seed-x 0.0 --seed-y 0.0 --tolerance 24 --feather 1
cargo run -p gameterm-visual --example scene_asset_edit -- continuity --frames 'DIR/kiki-blink-*.png' --feature-map MAP
cargo run -p gameterm-visual --example scene_asset_edit -- export-source --source IMAGE --source-root DIR --character kiki --expressions neutral,happy,concerned,surprised
```

Long-term, this can become a real binary if the helper stops being
experimental. The first pass can stay an example CLI to match existing
`scene_vn_asset_intake` and `scene_panel_style` conventions.

## Implementation Lanes

### 1. Scope And Command Contract

Status: complete.

Commit prefix: `[docs]`.

Work:

- record the primitives
- tie the helper to existing VN asset intake and panel codegen
- define the CLI and data formats
- define tests and non-goals

### 2. Rust Asset Document And Feature Map

Status: complete.

Commit prefix: `[visual]`.

Work:

- add Rust structs for image document reports and feature maps
- add parser/serializer for feature-map JSON
- add normalized-to-pixel region conversion
- add map validation
- add `inspect`, `map-template`, and `validate-map` commands

Done when:

- feature maps can be generated and validated from a PNG
- invalid regions produce clear diagnostics
- no Scene runtime rendering code changes are needed

### 3. Deterministic Edit Operations

Status: complete.

Commit prefix: `[visual]`.

Work:

- implement RGBA PNG load/save
- implement erase/fill/draw/composite operations
- implement recipe parsing and operation dispatch
- add overwrite protection with `--force`
- preserve image dimensions unless an explicit resize operation is requested

Done when:

- a fixture PNG can generate a visibly different expression PNG
- operation errors name the operation and target region
- transparent pixels and alpha compositing are handled predictably

### 4. Expression And Animation Recipes

Status: complete.

Commit prefix: `[visual]`.

Work:

- add expression recipe schema
- add animation recipe schema
- generate blink frames and breathing frames from recipes
- optionally export an animated GIF or sprite sheet for preview

Done when:

- `blink` and `breath` can be generated as separate frame sets
- each output can be consumed by `scene_vn_asset_intake`
- generated frames keep stable naming such as `kiki-blink-0.png`

### 5. Continuity And Install Integration

Status: first-pass implemented.

Commit prefix: `[visual]` or `[tools]`.

Work:

- add continuity checks
- add `export-source` replacement for the current shell image-export helper
- add an install path that delegates to `scene_vn_asset_intake`
- keep attribution and sprite IDs under the existing intake helper

Done when:

- generated outputs can be installed into
  `~/.config/gameterm/scenes/assets/vn-demo`
- `ci/gameterm-scene-vn-demo.sh doctor --strict-images` passes
- continuity warnings are visible before install

### 6. Panel Codegen Coordination

Status: complete.

Commit prefix: `[visual]` or `[tools]`.

Work:

- keep `scene_panel_style` as the renderer-style helper for UI panels
- optionally expose a wrapper subcommand that points users to panel style
  extraction
- document that character sprites and panels use different conversions

Done when:

- users do not confuse panel style extraction with character editing
- no raw PNG pixel arrays are added to Rust source
- panel rendering remains covered by the existing rounded-panel tests

### 7. Dogfood And Docs

Status: first-pass implemented.

Commit prefix: `[test]`, `[docs]`.

Work:

- generate at least one expression from a base fixture
- generate blink and breathing frame sets from fixture recipes
- install generated outputs through the VN demo helper in a temp config home
- run doctor and a targeted smoke
- update roadmap, onboarding, and handoff with the asset-editing workflow

Done when:

- the user can run one command sequence from terminal and see the generated
  expression in Scene Mode
- the output path is config-owned, not repo-owned
- docs explain where feature maps, recipes, generated PNGs, and sprite
  manifests live

## Test Conditions

Focused tests:

```sh
cargo test -p gameterm-visual asset_edit
cargo test -p gameterm-visual vn_asset_intake
bash -n ci/gameterm-scene-vn-image-export.sh
```

Helper smoke:

```sh
tmp="$(mktemp -d)"
cargo run -p gameterm-visual --example scene_asset_edit -- \
  map-template ci/fixtures/gameterm-scene/vn-asset-source/4cher_set4_vn_sprites/kiki-neutral.png \
  --character kiki \
  --output "${tmp}/kiki.features.json"

cargo run -p gameterm-visual --example scene_asset_edit -- \
  expression \
  --base ci/fixtures/gameterm-scene/vn-asset-source/4cher_set4_vn_sprites/kiki-neutral.png \
  --feature-map "${tmp}/kiki.features.json" \
  --recipe ci/fixtures/gameterm-scene/kiki-expression-recipes.json \
  --expression surprised \
  --output "${tmp}/kiki-surprised.png"
```

Config-module smoke:

```sh
ci/gameterm-scene-vn-demo.sh doctor \
  --config-home "${XDG_CONFIG_HOME:-${HOME}/.config}" \
  --strict-images
```

Assertions:

- invalid feature maps fail with region-specific errors
- missing patch PNGs fail before any output file is written
- existing output files are not overwritten without `--force`
- generated PNGs are real PNG files
- generated frame sets have stable dimensions
- generated sprite IDs match the existing intake bindings
- no downloaded third-party assets are committed

## Non-Goals

- No full paint application UI.
- No terminal tablet/pressure support in the first pass.
- No automatic semantic detection of eyes/mouths as a promise. The helper may
  emit rough templates, but the user/Codex mapping step is explicit.
- No AI inpainting requirement.
- No PSD layer parser requirement.
- No raw PNG-to-Rust pixel-array conversion for character sprites.
- No bundling local Kiki/school assets into `GameTerm.app`.
- No new Scene runtime schema unless animation playback itself is separately
  scoped.

## Risks

- Hand-drawn anime features are sensitive. Pure primitive edits can look worse
  than patch-based edits if the feature map is poor.
- Erasing pixels does not reconstruct hidden skin/hair detail. Patch PNGs or
  hand-authored base variants may still be needed.
- Animation continuity needs objective checks, but visual taste still needs
  dogfooding.
- A single "asset tool" can become too broad. Keep first-pass commands narrow
  and testable.

## Latest Verification

Commands run:

```sh
cargo test -p gameterm-visual asset_edit
cargo run -q -p gameterm-visual --example scene_asset_edit -- map-template \
  ci/fixtures/gameterm-scene/vn-asset-source/4cher_set4_vn_sprites/kiki-neutral.png \
  --character kiki \
  --output /tmp/gameterm-asset-edit.../kiki.features.json \
  --force
cargo run -q -p gameterm-visual --example scene_asset_edit -- validate-map \
  --image ci/fixtures/gameterm-scene/vn-asset-source/4cher_set4_vn_sprites/kiki-neutral.png \
  --feature-map /tmp/gameterm-asset-edit.../kiki.features.json
cargo run -q -p gameterm-visual --example scene_asset_edit -- expression \
  --base ci/fixtures/gameterm-scene/vn-asset-source/4cher_set4_vn_sprites/kiki-neutral.png \
  --feature-map /tmp/gameterm-asset-edit.../kiki.features.json \
  --recipe ci/fixtures/gameterm-scene/kiki-expression-recipes.json \
  --expression surprised \
  --output /tmp/gameterm-asset-edit.../kiki-surprised.png \
  --force
cargo run -q -p gameterm-visual --example scene_asset_edit -- animation \
  --base ci/fixtures/gameterm-scene/vn-asset-source/4cher_set4_vn_sprites/kiki-neutral.png \
  --feature-map /tmp/gameterm-asset-edit.../kiki.features.json \
  --recipe ci/fixtures/gameterm-scene/kiki-expression-recipes.json \
  --animation blink \
  --output-dir /tmp/gameterm-asset-edit.../blink \
  --character kiki \
  --force
cargo run -q -p gameterm-visual --example scene_asset_edit -- continuity \
  /tmp/gameterm-asset-edit.../blink/kiki-blink-0.png \
  /tmp/gameterm-asset-edit.../blink/kiki-blink-1.png \
  /tmp/gameterm-asset-edit.../blink/kiki-blink-2.png \
  --pretty
cargo run -q -p gameterm-visual --example scene_asset_edit -- remove-background \
  --source ci/fixtures/gameterm-scene/vn-asset-source/4cher_set4_vn_sprites/kiki-neutral.png \
  --output /tmp/gameterm-magic-erase.../kiki-bg-transparent.png \
  --tolerance 24 \
  --feather 1 \
  --force
cargo run -q -p gameterm-visual --example scene_asset_edit -- magic-erase \
  --source ci/fixtures/gameterm-scene/vn-asset-source/4cher_set4_vn_sprites/kiki-neutral.png \
  --output /tmp/gameterm-magic-erase.../kiki-magic-erase.png \
  --seed-x 0.0 \
  --seed-y 0.0 \
  --tolerance 24 \
  --feather 1 \
  --force
cargo test -p gameterm-visual
cargo check -p gameterm-visual --examples
```

Result:

- focused asset-edit tests: 9 passed
- full `gameterm-visual` suite: 196 passed
- example targets: checked cleanly
- CLI smoke generated feature-map JSON, one surprised expression PNG, and five
  blink frame PNGs from the Kiki fixture
- CLI smoke generated background-transparent and magic-erased PNG outputs from
  the Kiki fixture

## Definition Of Done

The first pass is complete when:

- a Rust helper can inspect a base PNG and validate a feature map
- deterministic recipes can generate at least one expression PNG
- blink and breathing frame outputs can be generated as separate animation
  sets
- generated outputs install through the existing VN asset intake/config module
  path
- doctor verifies generated PNGs and sprite manifests
- docs explain how this differs from panel codegen and full paint apps
- no repo-unsafe third-party art is committed
