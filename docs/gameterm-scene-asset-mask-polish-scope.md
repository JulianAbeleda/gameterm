# GameTerm Scene Asset Mask Polish Scope

Status: scoped, not implemented.

This document scopes the precision pass for Scene character cutouts after the
first `magic-erase` and `remove-background` helper landed.

The current helper can remove simple backgrounds, but anime character art often
has light skin, light hair, white highlights, and anti-aliased outlines close
to the background color. A plain color-threshold mask can therefore either:

- leave a visible white halo, or
- erase too much of the character.

The next pass should improve that by treating background removal as a mask
pipeline with composable sub-functions.

## Goal

Make this workflow produce cleaner character PNGs:

```text
source PNG
-> select background mask
-> protect foreground regions
-> clean mask
-> feather/matte edge
-> defringe white halo
-> write transparent RGBA PNG
-> report quality metrics
```

This remains deterministic Rust image processing. No AI segmentation is part of
this pass.

## Why Sub-Functions

Yes: each feature should be a sub-function. The helper needs small primitives
because precision work is iterative.

Bad shape:

```text
remove_background(image, tolerance, magic_everything)
```

Better shape:

```text
select_background_mask(...)
protect_regions(...)
erode_mask(...)
dilate_mask(...)
smooth_mask(...)
apply_alpha_mask(...)
defringe_edges(...)
score_cutout(...)
```

That lets us test each step and build CLI flags or recipes from the same
functions.

## Pipeline Model

### 1. Selection

Creates a boolean or grayscale mask.

Existing:

- `contiguous_magic_mask`
- `global_magic_mask`
- `background_magic_mask`

Next additions:

- multi-seed background selection
- normalized seed list from CLI/recipe
- optional edge-only seed discovery
- optional subject bounding box

Output:

```text
Mask
  width
  height
  alpha/coverage per pixel
```

First pass can keep `Vec<bool>`. The polish pass should introduce a named mask
type, eventually with `u8` coverage for soft alpha.

### 2. Protection

Prevents the mask from erasing known foreground areas.

Inputs:

- feature-map regions
- explicit protected normalized rectangles
- optional subject bounds

Examples:

```json
{
  "protected_regions": ["face", "hair", "torso"],
  "protect": [
    { "x": 0.26, "y": 0.05, "w": 0.58, "h": 0.94 }
  ]
}
```

Behavior:

```text
selection_mask - protected_mask = final_selection_mask
```

This is important because Kiki-like assets have face/hair colors close to the
white background.

### 3. Morphology

Edits mask shape before applying alpha.

Required functions:

- `erode_mask(radius)`: shrink selected region
- `dilate_mask(radius)`: grow selected region
- `open_mask(radius)`: erode then dilate, removes small noise
- `close_mask(radius)`: dilate then erode, fills tiny holes
- `remove_small_components(min_pixels)`: removes isolated mask islands
- `fill_small_holes(max_pixels)`: fills tiny unselected holes inside selected
  areas

Why this matters:

- erosion can reduce white halos
- dilation can catch missed background pixels
- component cleanup prevents accidental specks
- hole filling avoids tiny white holes around strands/outline

### 4. Feather And Matte

Converts hard mask edges into cleaner alpha edges.

Current:

- simple feather lowers alpha around selected pixels.

Next:

- operate on a grayscale coverage mask
- distance-based feather from mask boundary
- preserve fully opaque interior pixels
- avoid feathering protected foreground interiors

Expected behavior:

```text
background: alpha 0
edge: alpha 40..220
foreground: alpha 255
```

### 5. Defringe

Removes the white outline left by anti-aliased pixels.

Problem:

```text
edge pixel = mostly character color + white background
after alpha cutout = white halo visible on dark backgrounds
```

Required functions:

- `find_edge_pixels(mask, radius)`
- `sample_nearby_foreground_color(pixel, radius)`
- `replace_light_fringe_color(amount)`
- `desaturate_background_bias`

Practical first implementation:

```text
if edge pixel is light/low-saturation and near transparent background:
  replace RGB with median nearby opaque foreground RGB
  keep alpha
```

This should reduce white halos without changing the face/hair interior.

### 6. Quality Metrics

The helper should report objective diagnostics.

Metrics:

- selected pixel count
- transparent pixel ratio
- content bounds before/after
- number of isolated components removed
- number of holes filled
- edge-lightness score before/after
- protected-region erase count
- warnings if too much foreground was selected

Example report:

```json
{
  "operation": "remove_background_polished",
  "selected_pixels": 322829,
  "transparent_pixel_ratio": 0.5473,
  "edge_lightness_before": 0.81,
  "edge_lightness_after": 0.34,
  "protected_pixels_erased": 0,
  "warnings": []
}
```

## Proposed CLI

Add a polished mode rather than overloading every simple command too much:

```sh
cargo run -p gameterm-visual --example scene_asset_edit -- remove-background-polished \
  --source IMAGE \
  --output OUT \
  --sample corners \
  --tolerance 10 \
  --protect feature-map.json \
  --erode 1 \
  --close 1 \
  --feather 1 \
  --defringe white \
  --force
```

Also expose lower-level commands for dogfooding:

```sh
scene_asset_edit mask-inspect --source IMAGE --output-mask mask.png
scene_asset_edit mask-apply --source IMAGE --mask mask.png --output OUT
scene_asset_edit defringe --source IMAGE --output OUT --radius 2
```

## Recipe Support

Background cleanup should work in recipes too:

```json
{
  "op": "remove_background_polished",
  "tolerance": 10,
  "sample": "corners",
  "erode": 1,
  "close": 1,
  "feather": 1,
  "defringe": "white",
  "protect_regions": ["face", "hair", "torso"]
}
```

Simple `remove_background` and `magic_erase` should remain for predictable
basic use. The polished operation composes sub-functions.

## Implementation Lanes

### 1. Mask Type And Basic Morphology

Commit prefix: `[visual]`.

Work:

- introduce `SceneAssetMask`
- move current `Vec<bool>` selection code behind mask methods
- add erode/dilate/open/close
- add focused tests on synthetic masks

Done when:

- existing `remove-background` and `magic-erase` behavior stays green
- morphology tests prove masks shrink/grow/fill expected pixels

### 2. Protected Regions

Commit prefix: `[visual]`.

Work:

- support protected normalized rectangles
- support feature-map region names as protected regions
- subtract protection from the selection mask
- report protected pixels that would have been erased

Done when:

- synthetic test proves protected subject pixels stay opaque even when close to
  background color

### 3. Feather And Soft Mask

Commit prefix: `[visual]`.

Work:

- introduce grayscale mask/coverage or alpha-factor map
- replace current simple feather with distance-based feather
- keep hard-mask compatibility for old behavior

Done when:

- edge alpha is gradual
- opaque interior stays opaque
- transparent background stays transparent

### 4. Defringe

Commit prefix: `[visual]`.

Work:

- detect edge pixels next to transparent/low-alpha pixels
- replace white-biased RGB with nearby foreground median
- add radius/amount controls
- avoid modifying fully opaque interior pixels

Done when:

- dark-background preview has reduced white halo
- tests prove defringe changes RGB but preserves alpha

### 5. Polished CLI And Recipes

Commit prefix: `[visual]`.

Work:

- add `remove-background-polished`
- add recipe operation `remove_background_polished`
- add flags for tolerance, sample, protect, erode, close, feather, defringe
- keep old commands backward-compatible

Done when:

- the Kiki neutral base can produce a better transparent PNG than the current
  tight-threshold output

### 6. Metrics And Docs

Commit prefix: `[docs]` or `[test]`.

Work:

- add quality report fields
- record before/after CLI smoke
- update character asset editing scope and roadmap

Done when:

- verification includes a dark-background preview or objective edge-lightness
  metric

## Test Conditions

Focused tests:

```sh
cargo test -p gameterm-visual asset_edit
cargo check -p gameterm-visual --examples
```

Required unit cases:

- erode shrinks a known mask
- dilate grows a known mask
- open removes isolated mask noise
- close fills a small hole
- protected region prevents selection
- feather produces partial alpha only at edges
- defringe changes white-biased edge RGB but not alpha
- polished background removal preserves a light foreground subject

Manual dogfood:

```sh
cargo run -q -p gameterm-visual --example scene_asset_edit -- remove-background-polished \
  --source ~/Desktop/gameterm-vn-ai-emotion-sprites/neutral_base.png \
  --output ~/Desktop/gameterm-vn-ai-emotion-sprites/neutral_base-transparent-polished.png \
  --tolerance 10 \
  --erode 1 \
  --close 1 \
  --feather 1 \
  --defringe white \
  --force
```

Acceptance:

- character body, skin, hair, and highlights are not erased
- background is transparent
- white halo is visibly reduced on a dark preview
- output remains 768x768 RGBA
- original source file is not overwritten

## Non-Goals

- No AI segmentation in this pass.
- No automatic perfect hair extraction guarantee.
- No full paint-app selection UI.
- No PSD layer parsing.
- No committing generated local Kiki outputs into the repo.

## Definition Of Done

This pass is complete when:

- the asset helper has composable mask sub-functions
- polished background removal is available from CLI and recipes
- Kiki-style light-background cutouts look better than simple thresholding
- tests cover mask morphology, protection, feathering, and defringe
- docs explain how to tune precision from terminal
