# Scene Asset Edit - Full Script Scope

Date: 2026-06-08

This scope defines the full command surface for the Scene asset image editor:

```sh
cargo run -q -p gameterm-visual --example scene_asset_edit -- <command>
```

It follows `structure/Development/coding-principles.md`:

- preserve upstream behavior unless a task explicitly changes it
- keep GameTerm-specific changes narrow and reviewable
- prefer existing crate boundaries before new crates or broad abstractions
- keep core passes deterministic, scriptable, and free of Python, network, and
  GPU dependency
- commit one concern at a time with the `[visual]` subsystem prefix, or
  `[docs]` for scope-only updates

## Goal

Turn `scene_asset_edit` from a masking-only helper into a deterministic
terminal-native sprite authoring pipeline.

The tool should support this local workflow:

```text
Image Editor/
├── Input/
│   └── source PNGs
├── Transformation/
│   ├── mask previews
│   ├── dark previews
│   ├── intermediate PNGs
│   └── reports
└── Output/
    └── final usable PNGs
```

The important product rule is that destructive edits should be previewable or
reproducible. A user should be able to inspect a mask, tune coordinates/seeds,
then run a repeatable pipeline into `Output`.

## Current Implemented Commands

These commands already exist in
`gameterm-visual/examples/scene_asset_edit.rs`.

| Command | Family | Current role |
|---|---|---|
| `inspect` | Analysis | Reports dimensions, color type, alpha, bounds, checksum |
| `map-template` | Structure | Generates a feature-map template for a character image |
| `validate-map` | Structure | Validates feature map regions against an image |
| `expression` | Structure | Applies a recipe expression to a base image |
| `animation` | Structure | Generates animation frames from a recipe |
| `remove-background` | Selection | Simple connected-background removal |
| `remove-background-polished` | Selection | Connected-background removal with morphology/polish |
| `color-range-erase` | Selection | Erases all pixels matching sampled background color |
| `magic-erase` | Selection | Erases a contiguous region from one seed |
| `magic-erase-add` | Selection | Erases union of contiguous regions from repeated seeds |
| `channel-matte-erase` | Selection | Erases bright neutral pixels by channel matte |
| `mask-preview` | Selection | Writes non-destructive red-mask preview PNG |
| `hair-cleanup` | Selection | Decontaminates light/white edge pixels |
| `restore-from-source` | Restore | Copies pixels from base into a damaged cutout |
| `continuity` | Analysis | Compares frames for animation consistency |
| `export-source` | Structure | Exports a source-root layout for VN asset intake |

Shared implemented flags:

- morphology: `--erode`, `--dilate`, `--open`, `--close`
- mask cleanup: `--remove-small`, `--fill-holes`, `--feather`
- edge polish: `--defringe none|white`, `hair-cleanup`
- protection: `--protect`, `--protect-regions`
- bounded selection: `--within-regions`, `--within-polygon`
- coordinate inputs: normalized points and polygons

## Current Gap

The editor is currently strong at subtraction:

```text
select pixels -> erase / alpha / restore from existing source
```

It does not yet have a true draw/paint family:

```text
select or describe region -> lay down new pixels
```

This is a separate missing class from adjustment, transform, sharpen, or
composite. It matters for the Kiki hair workflow because some problems are not
only "erase the white"; sometimes we need to fill an exposed region with a
nearby sampled color, paint a small correction, or clone a neighboring texture.

## Global CLI Rules

All new commands should follow these rules unless a command explicitly cannot:

- command shape: `verb-noun --source IMAGE --output PATH [flags]`
- source PNGs come from `Input/`
- previews and intermediates go to `Transformation/`
- final accepted images go to `Output/`
- commands never write in-place
- `--force` is required to overwrite an existing output
- JSON reports support `--pretty`
- image outputs should preserve RGBA where possible
- commands touching pixels should support `--within-polygon`
- commands touching pixels should support `--protect` / `--protect-regions`
  where protection semantics are meaningful
- commands that make destructive changes should have either a preview mode or
  a separate preview command

## Pipeline Script

### `pipeline-run`

Purpose: execute repeatable multi-step asset workflows so we stop manually
typing five commands for every sprite.

CLI:

```sh
cargo run -q -p gameterm-visual --example scene_asset_edit -- pipeline-run \
  --pipeline PIPELINE.json \
  --input-root "Image Editor/Input" \
  --transformation-root "Image Editor/Transformation" \
  --output-root "Image Editor/Output" \
  --force \
  --pretty
```

Pipeline JSON shape:

```json
{
  "asset_pipeline_version": 1,
  "name": "kiki-transparent-cutout",
  "input": "neutral_base.png",
  "steps": [
    {
      "command": "mask-preview",
      "output": "01-ponytail-mask-preview.png",
      "args": {
        "selection_mode": "color_range",
        "tolerance": 10,
        "within_polygons": [
          "0.64,0.20;0.90,0.20;0.90,0.86;0.62,0.86"
        ]
      }
    },
    {
      "command": "color-range-erase",
      "output": "02-ponytail-pockets-erased.png",
      "args": {
        "tolerance": 10,
        "within_polygons": [
          "0.64,0.20;0.90,0.20;0.90,0.86;0.62,0.86"
        ]
      }
    },
    {
      "command": "magic-erase-add",
      "output": "03-background-erased.png",
      "args": {
        "seeds": ["0.05,0.05", "0.66,0.58"],
        "tolerance": 12,
        "feather": 1,
        "defringe": "white"
      }
    },
    {
      "command": "hair-cleanup",
      "output": "../Output/neutral_base-transparent.png",
      "args": {
        "radius": 2,
        "strength": 0.75
      }
    }
  ]
}
```

Definition of done:

- supports current implemented image commands first
- writes a per-step JSON report
- fails fast on missing inputs or overwritten outputs without `--force`
- does not introduce new image behavior; it only orchestrates existing commands

## Analysis And Coordinate Scripts

These make manual seed/polygon work tractable.

### `histogram`

Purpose: report per-channel and luminance statistics.

CLI:

```sh
scene_asset_edit histogram --source IMAGE --output REPORT.json \
  [--within-polygon X,Y;X,Y;X,Y] [--pretty]
```

Report:

- channel min/max/mean/median/std
- clipping percentage
- alpha coverage
- optional bounded region stats

### `sample`

Purpose: sample color at a point or bounded region.

CLI:

```sh
scene_asset_edit sample --source IMAGE --point X,Y --pretty
scene_asset_edit sample --source IMAGE --within-polygon X,Y;X,Y;X,Y --pretty
```

Report:

- exact pixel RGBA for a point
- average/median RGBA for a region
- nearest background sample match

### `grid-preview`

Purpose: draw a normalized coordinate grid over an image for choosing seeds and
polygons.

CLI:

```sh
scene_asset_edit grid-preview --source IMAGE --output GRID.png \
  [--step 0.05] [--labels] [--force]
```

Output:

- original image with overlaid normalized grid
- labels such as `0.65,0.55` at useful intervals

### `point-report`

Purpose: verify exactly where a normalized seed lands.

CLI:

```sh
scene_asset_edit point-report --source IMAGE --seed X,Y [--seed X,Y ...] --pretty
```

Report:

- normalized input point
- pixel coordinate
- sampled RGBA
- alpha
- optional nearest feature-map region

## Draw / Paint Scripts

This is the new feature family Claude correctly identified as missing.

These commands lay down pixels. They are deterministic paint primitives, not an
interactive canvas. They should support normalized coordinates and region masks
so they can be scripted from terminal.

### `fill-region`

Purpose: fill a bounded region with a solid color.

CLI:

```sh
scene_asset_edit fill-region --source IMAGE --output IMAGE \
  --color '#RRGGBB' \
  [--alpha N] \
  [--within-polygon X,Y;X,Y;X,Y] \
  [--within-regions CSV] \
  [--protect FEATURE_MAP] [--protect-regions CSV] \
  [--force]
```

Use cases:

- paint a flat skin-tone patch under a removed feature
- fill tiny background pockets after selection
- add temporary color-coded debug regions

Semantics:

- no bounds means full image fill is invalid unless `--whole-image` is passed
- `--alpha` defaults to 255
- `--blend normal|source-over|multiply|screen` can be added later

### `sample-fill`

Purpose: fill a bounded region using a sampled nearby color.

CLI:

```sh
scene_asset_edit sample-fill --source IMAGE --output IMAGE \
  --sample-point X,Y \
  [--sample-radius N] \
  --within-polygon X,Y;X,Y;X,Y \
  [--soft-edge N] \
  [--force]
```

Use cases:

- cheap deterministic "content-aware fill"
- paint skin tone behind a separated mouth/eye/face part
- fill small white gaps behind hair with nearby hair/skin color

Semantics:

- sample color is median RGBA within `--sample-radius`
- output fills only selected/bounded pixels
- optional `--soft-edge` feathers alpha/color at the mask edge
- this should be implemented before ML inpaint because it solves many small
  sprite repairs without external models

### `draw-shape`

Purpose: draw deterministic simple shapes.

CLI:

```sh
scene_asset_edit draw-shape --source IMAGE --output IMAGE \
  --shape rect|circle|ellipse|line|polygon \
  --color '#RRGGBB' \
  [--alpha N] \
  [--stroke N] \
  [--fill] \
  [--point X,Y ...] \
  [--rect X,Y,W,H] \
  [--force]
```

Use cases:

- debug overlays
- simple mouth/eye marker experiments
- draw bounding boxes or feature-map previews

Implementation notes:

- use `imageproc` drawing helpers where possible
- support normalized coordinates
- keep anti-aliasing optional; default can be pixel-exact

### `stroke-path`

Purpose: draw an outline along a polygon/path.

CLI:

```sh
scene_asset_edit stroke-path --source IMAGE --output IMAGE \
  --path X,Y;X,Y;X,Y \
  --color '#RRGGBB' \
  [--width N] \
  [--closed] \
  [--force]
```

Use cases:

- outline traced hair regions
- create visible mask boundaries
- debug polygon accuracy

### `gradient-fill`

Purpose: fill a region with a linear or radial gradient.

CLI:

```sh
scene_asset_edit gradient-fill --source IMAGE --output IMAGE \
  --kind linear|radial \
  --from X,Y --to X,Y \
  --color-a '#RRGGBB' --color-b '#RRGGBB' \
  [--within-polygon X,Y;X,Y;X,Y] \
  [--force]
```

Use cases:

- simple deterministic shading
- temporary lighting tests
- background construction

### `clone-stamp`

Purpose: copy pixels from one region to another with an offset.

CLI:

```sh
scene_asset_edit clone-stamp --source IMAGE --output IMAGE \
  --sample-origin X,Y \
  --target-origin X,Y \
  --within-polygon X,Y;X,Y;X,Y \
  [--soft-edge N] \
  [--force]
```

Use cases:

- copy nearby hair texture into a damaged strip
- repair a small cutout defect
- duplicate local shading without ML

Semantics:

- source and target coordinates are normalized
- only the bounded target mask is changed
- optional soft edge blends the cloned pixels into surrounding pixels

### `alpha-paint`

Purpose: paint alpha only, without changing RGB.

CLI:

```sh
scene_asset_edit alpha-paint --source IMAGE --output IMAGE \
  --alpha N \
  [--within-polygon X,Y;X,Y;X,Y] \
  [--soft-edge N] \
  [--force]
```

Use cases:

- make a region transparent without re-running selection
- repair alpha edge mistakes
- fade a region in/out for animation tests

## Adjustment Scripts

### `levels`

Purpose: black point / white point / gamma correction.

CLI:

```sh
scene_asset_edit levels --source IMAGE --output IMAGE \
  [--channel rgb|r|g|b|a] \
  [--black N] [--white N] [--gamma N] \
  [--within-polygon X,Y;X,Y;X,Y] \
  [--force]
```

### `curves`

Purpose: LUT remap from control points or a curve file.

CLI:

```sh
scene_asset_edit curves --source IMAGE --output IMAGE \
  --curve "0,0;128,140;255,255" \
  [--channel rgb|r|g|b|a] \
  [--force]
```

### `hsl`

Purpose: hue, saturation, and lightness shifts.

CLI:

```sh
scene_asset_edit hsl --source IMAGE --output IMAGE \
  [--hue N] [--saturation N] [--lightness N] \
  [--within-polygon X,Y;X,Y;X,Y] \
  [--force]
```

### `brightness-contrast`

Purpose: simple tonal adjustment.

CLI:

```sh
scene_asset_edit brightness-contrast --source IMAGE --output IMAGE \
  [--brightness N] [--contrast N] \
  [--force]
```

### `color-match`

Purpose: normalize one sprite frame to a reference image.

CLI:

```sh
scene_asset_edit color-match --source IMAGE --reference IMAGE --output IMAGE \
  [--within-polygon X,Y;X,Y;X,Y] \
  [--force]
```

Use cases:

- keep expression frames consistent
- correct generated sprite drift
- normalize idle animation frames

## Transform Scripts

### `crop`

Purpose: crop to a rect or content bounds.

CLI:

```sh
scene_asset_edit crop --source IMAGE --output IMAGE \
  (--rect X,Y,W,H | --content-bounds) \
  [--force]
```

### `pad`

Purpose: expand canvas without resizing content.

CLI:

```sh
scene_asset_edit pad --source IMAGE --output IMAGE \
  --width N --height N \
  [--anchor center|bottom-center|top-left] \
  [--color '#RRGGBB'] [--alpha N] \
  [--force]
```

### `transform`

Purpose: affine transform.

CLI:

```sh
scene_asset_edit transform --source IMAGE --output IMAGE \
  [--scale N] [--rotate-deg N] [--translate X,Y] \
  [--flip-x] [--flip-y] \
  [--resample nearest|bilinear|lanczos3] \
  [--force]
```

Use cases:

- breathing/idle frame offsets
- sprite alignment
- deterministic small motion

## Filter / Sharpen Scripts

### `unsharp-mask`

Purpose: sharpen while controlling halos.

CLI:

```sh
scene_asset_edit unsharp-mask --source IMAGE --output IMAGE \
  --radius N --amount N [--threshold N] \
  [--within-polygon X,Y;X,Y;X,Y] \
  [--force]
```

### `blur`

Purpose: gaussian, box, or median blur.

CLI:

```sh
scene_asset_edit blur --source IMAGE --output IMAGE \
  --kind gaussian|box|median \
  --radius N \
  [--within-polygon X,Y;X,Y;X,Y] \
  [--force]
```

### `denoise`

Purpose: classical noise reduction.

CLI:

```sh
scene_asset_edit denoise --source IMAGE --output IMAGE \
  --kind median|bilateral \
  [--radius N] \
  [--force]
```

## Compositing Scripts

### `composite`

Purpose: stack layers into a final PNG.

CLI:

```sh
scene_asset_edit composite --output IMAGE \
  --layer base.png,normal,1.0,0,0 \
  --layer eyes.png,normal,1.0,0,0 \
  --layer mouth.png,normal,1.0,0,0 \
  [--width N --height N] \
  [--force]
```

Layer format:

```text
path,blend,opacity,x_offset,y_offset
```

Initial blend modes:

- `normal`
- `multiply`
- `screen`
- `overlay`
- `add`

## State / Variant Scripts

These are the structural Figma-inspired primitives. They turn a pile of PNGs
into one character model with switchable parts.

### `state-manifest`

Purpose: generate or validate a state manifest.

CLI:

```sh
scene_asset_edit state-manifest --base IMAGE --output manifest.json \
  --character kiki \
  [--part eyes=open.png,half.png,closed.png] \
  [--part mouth=closed.png,a.png,i.png,u.png,e.png,o.png] \
  [--force]
```

Manifest model:

```json
{
  "asset_state_version": 1,
  "character": "kiki",
  "base": "base.png",
  "parts": {
    "eyes": {
      "default": "open",
      "states": {
        "open": "eyes/open.png",
        "closed": "eyes/closed.png"
      }
    },
    "mouth": {
      "default": "closed",
      "states": {
        "closed": "mouth/closed.png",
        "a": "mouth/a.png"
      }
    }
  }
}
```

### `state-render`

Purpose: render one named state combination.

CLI:

```sh
scene_asset_edit state-render --manifest manifest.json --output IMAGE \
  --state eyes=closed \
  --state mouth=a \
  [--force]
```

### `state-sheet`

Purpose: batch-render a spritesheet from state combinations.

CLI:

```sh
scene_asset_edit state-sheet --manifest manifest.json --output SHEET.png \
  --frames frames.json \
  --index frame-index.json \
  [--force]
```

## Interpolation Scripts

### `tween`

Purpose: create deterministic in-between frames.

CLI:

```sh
scene_asset_edit tween --from IMAGE --to IMAGE --output-dir DIR \
  --frames N \
  --mode crossfade|transform \
  [--force]
```

First pass:

- crossfade
- transform offsets
- no mesh warp

Later pass:

- part-aware tweening through state manifests
- mesh/landmark warp only after the deterministic version works

## Optional ML Scripts

These are optional and gated behind a cargo feature. They are not required for
the core local pipeline.

### `detect`

Purpose: generate feature maps automatically.

CLI:

```sh
scene_asset_edit detect --source IMAGE --output feature-map.json \
  --model MODEL.onnx \
  [--force]
```

### `matte-ml`

Purpose: alpha matting for difficult hair cases.

CLI:

```sh
scene_asset_edit matte-ml --source IMAGE --output IMAGE \
  --model MODEL.onnx \
  [--force]
```

### `upscale`

Purpose: super-resolution for low-resolution source art.

CLI:

```sh
scene_asset_edit upscale --source IMAGE --output IMAGE \
  --model MODEL.onnx \
  [--scale 2|4] \
  [--force]
```

### `inpaint-ml`

Purpose: content-aware fill after classical `sample-fill` is insufficient.

CLI:

```sh
scene_asset_edit inpaint-ml --source IMAGE --output IMAGE \
  --mask MASK.png \
  --model MODEL.onnx \
  [--force]
```

## Implementation Priority

### Pass 1: Scriptability and coordinate tooling

Commands:

- `pipeline-run`
- `grid-preview`
- `point-report`
- `sample`

Reason:

- reduces manual command repetition
- makes seed/polygon work less guessy
- directly improves the Kiki transparent cutout workflow

Commit shape:

- `[visual] add Scene asset pipeline runner`
- `[visual] add Scene asset coordinate previews`

### Pass 2: Draw / Paint minimum viable family

Commands:

- `fill-region`
- `sample-fill`
- `draw-shape`
- `alpha-paint`

Reason:

- fills the missing capability Claude identified
- lets us add color back instead of only erasing or restoring from source
- solves small occlusion and hair/skin repair cases without ML

Commit shape:

- `[visual] add Scene asset fill operations`
- `[visual] add Scene asset drawing operations`

### Pass 3: Transform and basic adjustment

Commands:

- `crop`
- `pad`
- `transform`
- `levels`
- `brightness-contrast`
- `hsl`

Reason:

- supports sprite alignment
- supports idle/breath frame generation
- improves consistency across expression frames

Commit shape:

- `[visual] add Scene asset transform operations`
- `[visual] add Scene asset tonal adjustments`

### Pass 4: Sharpen/filter

Commands:

- `unsharp-mask`
- `blur`
- `denoise`

Reason:

- fixes soft generated assets
- gives us local quality-control passes before ML

Commit shape:

- `[visual] add Scene asset filter operations`

### Pass 5: Composite and state variants

Commands:

- `composite`
- `state-manifest`
- `state-render`
- `state-sheet`

Reason:

- turns flat PNGs into a structured character asset system
- supports VN expressions, blink states, mouth shapes, and idle sheets

Commit shape:

- `[visual] add Scene asset compositing`
- `[visual] add Scene asset state variants`

### Pass 6: Optional ML

Commands:

- `detect`
- `matte-ml`
- `upscale`
- `inpaint-ml`

Reason:

- useful later, but heavier and less aligned with the immediate Rust-first
  dogfood path

Commit shape:

- `[visual] add optional Scene asset detection`
- `[visual] add optional Scene asset ML matting`

## Verification

Every implemented command needs:

- focused unit tests in `gameterm-visual/src/asset_edit.rs`
- `cargo check -p gameterm-visual --examples`
- `cargo test -p gameterm-visual asset_edit`
- `cargo test -p gameterm-visual`
- at least one real Kiki smoke using:

```text
/Users/julianabeleda/Desktop/gameterm-vn-ai-emotion-sprites/Image Editor/Input
/Users/julianabeleda/Desktop/gameterm-vn-ai-emotion-sprites/Image Editor/Transformation
/Users/julianabeleda/Desktop/gameterm-vn-ai-emotion-sprites/Image Editor/Output
```

For visual commands, generate a dark preview in `Transformation` so alpha and
edge mistakes are easy to see.

## Non-Goals

- No interactive GUI canvas in this scope.
- No Python for core passes.
- No network dependency for core passes.
- No GPU dependency for core passes.
- No Photoshop/Figma clone.
- No vector authoring in the first pass.
- No ML required for the default build.

## Product End State

After the full first product pass, the user can:

1. put a PNG in `Input`
2. run a reproducible pipeline
3. preview masks and coordinates in `Transformation`
4. erase, fill, clone, adjust, transform, and compose deterministic PNGs
5. output a final VN-ready sprite in `Output`
6. later graduate that flat sprite into a state/variant character model

