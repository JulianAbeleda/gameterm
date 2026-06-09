# GameTerm Scene Asset Editor Cookbook

Date: 2026-06-08

This is the practical command cookbook for the Rust-only Scene asset editor.
The implementation lives behind:

```sh
cargo run -q -p gameterm-visual --example scene_asset_edit -- <command>
```

The expected local working folders are:

```sh
BASE="/Users/julianabeleda/Desktop/gameterm-vn-ai-emotion-sprites/Image Editor"
IN="$BASE/Input/neutral_base.png"
TX="$BASE/Transformation"
OUT="$BASE/Output"
```

Use `Transformation` for previews/intermediates and `Output` for accepted
VN-ready PNGs. These desktop outputs are local working artifacts, not repo
assets.

## Inspect And Sample

Inspect a source image:

```sh
cargo run -q -p gameterm-visual --example scene_asset_edit -- inspect "$IN" --pretty
```

Sample points and a polygon region:

```sh
cargo run -q -p gameterm-visual --example scene_asset_edit -- sample \
  --source "$IN" \
  --point 0.05,0.05 \
  --point 0.50,0.42 \
  --within-polygon '0.64,0.20;0.90,0.20;0.90,0.86;0.62,0.86' \
  --output "$TX/24-sample-report.json" \
  --pretty \
  --force
```

Generate a coordinate grid:

```sh
cargo run -q -p gameterm-visual --example scene_asset_edit -- grid-preview \
  --source "$IN" \
  --output "$TX/20-coordinate-grid-preview.png" \
  --step 0.1 \
  --force
```

## Pipeline Runner

Run a repeatable pipeline fixture:

```sh
cargo run -q -p gameterm-visual --example scene_asset_edit -- pipeline-run \
  --pipeline ci/fixtures/gameterm-scene/kiki-asset-pipeline.json \
  --input-root "$BASE/Input" \
  --transformation-root "$TX" \
  --output-root "$OUT" \
  --output "$TX/25-pipeline-run-report.json" \
  --pretty \
  --force
```

Use `--dry-run` first when editing a pipeline JSON and you only want the report.

## Operation Files And Sessions

Use operation files when an AI or human should describe one edit as data rather
than as a long command line. The stable loop is:

```text
inspect -> write operation JSON -> dry-run or preview -> run -> compare
-> keep editing or accept the output
```

Run one operation from a JSON envelope:

```sh
cargo run -q -p gameterm-visual --example scene_asset_edit -- operation-run \
  --operation ci/fixtures/gameterm-scene/kiki-asset-operation-draw-shape.json \
  --input-root ci/fixtures/gameterm-scene/vn-asset-source \
  --transformation-root "$TX" \
  --output-root "$OUT" \
  --output "$TX/41-operation-report.json" \
  --pretty \
  --force
```

Preview one operation without accepting the requested output:

```sh
cargo run -q -p gameterm-visual --example scene_asset_edit -- operation-run \
  --operation ci/fixtures/gameterm-scene/kiki-asset-operation-draw-shape.json \
  --input-root ci/fixtures/gameterm-scene/vn-asset-source \
  --transformation-root "$TX" \
  --output-root "$OUT" \
  --output "$TX/41-operation-preview-report.json" \
  --preview \
  --pretty \
  --force
```

Preview mode writes review artifacts such as:

```text
kiki-fixture-draw-shape.preview.png
kiki-fixture-draw-shape.diff.png
```

Run an ordered edit session:

```sh
cargo run -q -p gameterm-visual --example scene_asset_edit -- session-run \
  --session ci/fixtures/gameterm-scene/kiki-asset-session.json \
  --input-root ci/fixtures/gameterm-scene/vn-asset-source \
  --transformation-root "$TX" \
  --output-root "$OUT" \
  --output "$TX/42-session-report.json" \
  --pretty \
  --force
```

Compare a source and a generated output:

```sh
cargo run -q -p gameterm-visual --example scene_asset_edit -- compare \
  --before ci/fixtures/gameterm-scene/vn-asset-source/4cher_set4_vn_sprites/kiki-neutral.png \
  --after "$TX/42-operation-alpha-debug.png" \
  --output "$TX/43-compare-report.json" \
  --pretty \
  --force
```

Operation JSON shape:

```json
{
  "asset_operation_version": 1,
  "id": "kiki-fixture-draw-shape",
  "intent": "Describe the exact edit in human language.",
  "source": "4cher_set4_vn_sprites/kiki-neutral.png",
  "output": "41-operation-draw-shape-debug.png",
  "command": "draw-shape",
  "args": {
    "shape": "rect",
    "rect": "0.02,0.02,0.08,0.08",
    "color": "#3366ffff",
    "fill": true
  },
  "expectations": {
    "max_changed_pixel_ratio": 0.03,
    "review_points": ["0.03,0.03"]
  }
}
```

Session JSON shape:

```json
{
  "asset_session_version": 1,
  "name": "kiki-fixture-operation-session",
  "current_source": "4cher_set4_vn_sprites/kiki-neutral.png",
  "accepted_outputs": [],
  "operations": [
    "kiki-asset-operation-draw-shape.json",
    "kiki-asset-operation-alpha-paint.json"
  ]
}
```

Use deterministic operations first. Reach for external semantic inpainting only
when the target edit requires inventing pixels that cannot be restored, sampled,
cloned, or drawn from the existing source.

Prompt template for an AI-assisted edit:

```text
You are editing a GameTerm Scene asset through deterministic Rust operations.

Roots:
- Input: <input-root>
- Transformation: <transformation-root>
- Output: <output-root>

Goal:
<describe the visible edit>

Rules:
- Return exactly one SceneAssetOperation JSON object.
- Use normalized coordinates from 0.0 to 1.0.
- Write outputs to Transformation unless I explicitly ask to accept into Output.
- Prefer bounded regions or polygons over whole-image edits.
- Protect face, eyes, mouth, and body unless the goal explicitly edits them.
- Include a max_changed_pixel_ratio and review_points.
- Do not invent hidden files or run network/ML tools.

Available commands:
sample, mask-preview, remove-background, remove-background-polished,
color-range-erase, magic-erase-add, hair-cleanup, fill-region, sample-fill,
alpha-paint, clone-stamp, draw-shape, stroke-path, crop, pad, transform,
levels, brightness-contrast, hsl, blur, unsharp-mask.

After I run the operation report, revise only the JSON fields needed to fix
reported expectation failures or visual issues.
```

## Mask And Cutout

Preview a bounded color-range selection:

```sh
cargo run -q -p gameterm-visual --example scene_asset_edit -- mask-preview \
  --source "$IN" \
  --output "$TX/01-ponytail-color-range-mask-preview.png" \
  --selection-mode color-range \
  --tolerance 10 \
  --within-polygon '0.64,0.20;0.90,0.20;0.90,0.86;0.62,0.86' \
  --force
```

Erase white pockets by color range:

```sh
cargo run -q -p gameterm-visual --example scene_asset_edit -- color-range-erase \
  --source "$IN" \
  --output "$TX/02-ponytail-white-pockets-erased.png" \
  --tolerance 10 \
  --within-polygon '0.64,0.20;0.90,0.20;0.90,0.86;0.62,0.86' \
  --force
```

Remove the connected background and polish the edge:

```sh
cargo run -q -p gameterm-visual --example scene_asset_edit -- remove-background-polished \
  --source "$IN" \
  --output "$OUT/neutral_base-transparent.png" \
  --tolerance 24 \
  --feather 1 \
  --defringe white \
  --force
```

## Paint And Draw

Fill a bounded region:

```sh
cargo run -q -p gameterm-visual --example scene_asset_edit -- fill-region \
  --source "$IN" \
  --output "$TX/21-fill-region-debug.png" \
  --color '#3366ffff' \
  --within-polygon '0.02,0.02;0.12,0.02;0.12,0.12;0.02,0.12' \
  --force
```

Fill a bounded region from a sampled nearby color:

```sh
cargo run -q -p gameterm-visual --example scene_asset_edit -- sample-fill \
  --source "$IN" \
  --output "$TX/22-sample-fill-debug.png" \
  --sample-point 0.45,0.36 \
  --within-polygon '0.46,0.34;0.54,0.34;0.54,0.42;0.46,0.42' \
  --force
```

Paint alpha in a bounded region:

```sh
cargo run -q -p gameterm-visual --example scene_asset_edit -- alpha-paint \
  --source "$IN" \
  --output "$TX/23-alpha-paint-debug.png" \
  --alpha 96 \
  --within-polygon '0.02,0.14;0.12,0.14;0.12,0.24;0.02,0.24' \
  --force
```

Draw a filled shape:

```sh
cargo run -q -p gameterm-visual --example scene_asset_edit -- draw-shape \
  --source "$IN" \
  --output "$TX/26-draw-shape-debug.png" \
  --shape rect \
  --rect 0.02,0.02,0.08,0.08 \
  --color '#3366ffff' \
  --fill \
  --force
```

Stroke a traced path:

```sh
cargo run -q -p gameterm-visual --example scene_asset_edit -- stroke-path \
  --source "$IN" \
  --output "$TX/27-stroke-path-debug.png" \
  --path '0.34,0.16;0.42,0.12;0.50,0.16;0.42,0.20' \
  --color '#ff3366ff' \
  --width 4 \
  --closed \
  --force
```

Clone pixels from one normalized origin to another:

```sh
cargo run -q -p gameterm-visual --example scene_asset_edit -- clone-stamp \
  --source "$IN" \
  --output "$TX/28-clone-stamp-debug.png" \
  --sample-origin 0.43,0.34 \
  --target-origin 0.50,0.36 \
  --within-polygon '0.47,0.32;0.55,0.32;0.55,0.42;0.47,0.42' \
  --force
```

## Transform

Crop to content bounds:

```sh
cargo run -q -p gameterm-visual --example scene_asset_edit -- crop \
  --source "$IN" \
  --output "$TX/29-crop-debug.png" \
  --content-bounds \
  --force
```

Pad to a fixed transparent canvas:

```sh
cargo run -q -p gameterm-visual --example scene_asset_edit -- pad \
  --source "$TX/29-crop-debug.png" \
  --output "$TX/30-pad-debug.png" \
  --width 1024 \
  --height 1024 \
  --anchor bottom-center \
  --force
```

Translate, scale, and flip:

```sh
cargo run -q -p gameterm-visual --example scene_asset_edit -- transform \
  --source "$IN" \
  --output "$TX/31-transform-debug.png" \
  --scale 0.92 \
  --translate 16,-8 \
  --flip-x \
  --force
```

## Tonal And Filter

Levels:

```sh
cargo run -q -p gameterm-visual --example scene_asset_edit -- levels \
  --source "$IN" \
  --output "$TX/32-levels-debug.png" \
  --black 8 \
  --white 246 \
  --gamma 1.05 \
  --force
```

Brightness and contrast:

```sh
cargo run -q -p gameterm-visual --example scene_asset_edit -- brightness-contrast \
  --source "$IN" \
  --output "$TX/33-brightness-contrast-debug.png" \
  --brightness 0.04 \
  --contrast 0.12 \
  --force
```

HSL:

```sh
cargo run -q -p gameterm-visual --example scene_asset_edit -- hsl \
  --source "$IN" \
  --output "$TX/34-hsl-debug.png" \
  --hue 4 \
  --saturation 0.08 \
  --lightness 0.02 \
  --force
```

Blur and unsharp mask:

```sh
cargo run -q -p gameterm-visual --example scene_asset_edit -- blur \
  --source "$IN" \
  --output "$TX/35-blur-debug.png" \
  --radius 2 \
  --force

cargo run -q -p gameterm-visual --example scene_asset_edit -- unsharp-mask \
  --source "$TX/35-blur-debug.png" \
  --output "$TX/36-unsharp-debug.png" \
  --radius 2 \
  --amount 1.4 \
  --threshold 2 \
  --force
```

## Composite And States

Composite layers:

```sh
cargo run -q -p gameterm-visual --example scene_asset_edit -- composite \
  --output "$TX/37-composite-debug.png" \
  --width 1024 \
  --height 1024 \
  --layer "$IN,normal,1.0,0,0" \
  --layer "$TX/33-brightness-contrast-debug.png,screen,0.25,0,0" \
  --force
```

Create a state manifest:

```sh
cargo run -q -p gameterm-visual --example scene_asset_edit -- state-manifest \
  --base "$IN" \
  --output "$TX/38-state-manifest.json" \
  --character kiki \
  --part tone=32-levels-debug.png,33-brightness-contrast-debug.png \
  --force
```

Render one state:

```sh
cargo run -q -p gameterm-visual --example scene_asset_edit -- state-render \
  --manifest "$TX/38-state-manifest.json" \
  --output "$TX/39-state-render-debug.png" \
  --state tone=33-brightness-contrast-debug \
  --force
```

Render a spritesheet and frame index:

```sh
cargo run -q -p gameterm-visual --example scene_asset_edit -- state-sheet \
  --manifest "$TX/38-state-manifest.json" \
  --frames ci/fixtures/gameterm-scene/kiki-state-frames.json \
  --output "$TX/40-state-sheet-debug.png" \
  --index "$TX/40-state-sheet-index.json" \
  --force
```

## Verification

The first-pass completion smoke generated local artifacts:

```text
24-sample-report.json
25-pipeline-*.json / 25-pipeline-fill-debug.png
26-draw-shape-debug.png
27-stroke-path-debug.png
28-clone-stamp-debug.png
29-crop-debug.png
30-pad-debug.png
31-transform-debug.png
32-levels-debug.png
33-brightness-contrast-debug.png
34-hsl-debug.png
35-blur-debug.png
36-unsharp-debug.png
37-composite-debug.png
38-state-manifest.json
39-state-render-debug.png
40-state-sheet-debug.png
40-state-sheet-index.json
41-operation-*.json / 41-operation-draw-shape-debug.png
42-operation-alpha-debug.png / 42-session-report.json
43-compare-report.json
```

Repo verification:

```sh
cargo test -p gameterm-visual asset_edit
cargo check -p gameterm-visual --examples
cargo test -p gameterm-visual
```

After this pass, the missing work is GUI surface area only: file browser, mouse
point picking, lasso/polygon drawing, drag handles, live previews, and state or
timeline panels.
