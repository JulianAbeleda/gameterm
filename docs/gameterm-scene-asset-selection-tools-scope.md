# GameTerm Scene Asset Selection Tools Scope

Status: first-pass implemented.

This document scopes the next cutout precision pass after
`remove-background-polished`. The current helper can remove an edge-connected
background and can globally erase pixels matching one sampled seed color, but
it does not yet model two common paint-app selection workflows:

- Color Range: select every matching color region at once, including interior
  white pockets.
- Magic Wand add-to-selection: select the outer background first, then add
  each interior white pocket by clicking additional seeds.
- Hair edge cleanup: decontaminate leftover white matte color and provide a
  channel-matte style selector for high-contrast white hair gaps.

Both workflows are needed for Kiki-style anime sprite cleanup because hair
strands create disconnected white pockets that are not reachable from the outer
background flood fill.

## Current Gap

Existing behavior:

- `remove-background` and `remove-background-polished` sample the outer
  background and select an edge-connected mask.
- `magic-erase --global` selects every pixel matching a single clicked seed
  color.
- `magic-erase` without `--global` selects one contiguous region from one seed.

Missing behavior:

- no named Color Range command with protection, morphology, feather, and
  defringe
- no repeated seed list for contiguous add-to-selection
- no CLI/recipe way to add multiple disconnected white pockets and apply the
  mask once
- no standalone hair cleanup command for decontaminating white halos after a
  mask is already applied
- no channel-matte selection primitive for bright, low-saturation pockets
- no tests proving disconnected interior white pockets are removed without
  erasing protected foreground

## Goal

Add deterministic Rust selection tools that map cleanly to paint-app
primitives:

```text
Color Range
-> sample white
-> select all matching white pixels globally
-> protect foreground regions
-> polish mask
-> write transparent PNG
```

```text
Magic Wand Add
-> seed outer background
-> seed interior white pocket A
-> seed interior white pocket B
-> union contiguous selections
-> protect foreground regions
-> polish mask
-> write transparent PNG
```

## Primitive 1: Color Range

Color Range works by color similarity, not contour. If the background is flat
white, it should remove disconnected white pockets inside hair gaps.

Proposed CLI:

```sh
cargo run -q -p gameterm-visual --example scene_asset_edit -- color-range-erase \
  --source IMAGE \
  --output OUT \
  --sample corners \
  --tolerance 10 \
  --protect feature-map.json \
  --protect-regions face,hair,torso \
  --dilate 1 \
  --close 1 \
  --feather 1 \
  --defringe white \
  --force \
  --pretty
```

Selection inputs:

- `--sample corners`: use corner pixels as sampled colors
- `--sample edges`: use edge pixels as sampled colors
- `--seed-x N --seed-y N`: sample one explicit normalized point
- future: `--color #rrggbb`

Expected behavior:

- select every pixel within tolerance of any sample color
- include disconnected white pockets
- subtract protected regions before alpha application
- support the same morphology, feather, defringe, and quality report as
  `remove-background-polished`

## Primitive 2: Magic Wand Add-To-Selection

Magic Wand add-to-selection should behave like holding Shift while clicking
several disconnected regions. Each seed gets a contiguous flood fill. The final
mask is the union of all seed selections.

Proposed CLI:

```sh
cargo run -q -p gameterm-visual --example scene_asset_edit -- magic-erase-add \
  --source IMAGE \
  --output OUT \
  --seed 0.01,0.01 \
  --seed 0.72,0.56 \
  --seed 0.78,0.82 \
  --tolerance 10 \
  --protect feature-map.json \
  --protect-regions face,hair,torso \
  --feather 1 \
  --defringe white \
  --force \
  --pretty
```

Expected behavior:

- each `--seed x,y` runs contiguous magic selection from that clicked point
- selected components are added together
- matching but unclicked regions are not selected
- protection subtracts from the union mask
- the image is written once after all additions

First pass should be CLI and recipe driven. A GUI click workflow can come
later.

## Primitive 3: Hair Edge Cleanup

Photoshop's hair cleanup tools map to three different engine primitives.

### Refine Edge Brush

The real Photoshop tool is interactive. The first GameTerm pass should not try
to fake a brush UI. Instead, it should expose the same underlying idea as
terminal parameters:

- optional `--hair-region NAME` from a feature map
- edge-only processing around transparent pixels
- local foreground color sampling
- alpha/matte cleanup limited to the selected region when provided

This gives us a deterministic CLI route first. A GUI brush can come later.

### Decontaminate Colors

Decontamination removes the leftover white matte color from semi-transparent
hair pixels after masking.

Proposed CLI:

```sh
cargo run -q -p gameterm-visual --example scene_asset_edit -- hair-cleanup \
  --source IMAGE \
  --output OUT \
  --mode decontaminate \
  --radius 4 \
  --strength 0.85 \
  --protect feature-map.json \
  --hair-region hair \
  --force \
  --pretty
```

Expected behavior:

- detect opaque or semi-transparent light edge pixels near transparent
  neighbors
- replace their RGB matte color with nearby non-light foreground color
- preserve alpha unless an explicit matte mode is requested
- report how many edge pixels changed

### Channels Method

The channel method is a selection-building primitive. It uses channel contrast
to isolate white/bright background pockets from colored hair.

First-pass approximation:

```text
bright neutral pixel + optional hair region + optional edge relation
-> selected background pocket mask
```

Proposed CLI:

```sh
cargo run -q -p gameterm-visual --example scene_asset_edit -- channel-matte-erase \
  --source IMAGE \
  --output OUT \
  --threshold 238 \
  --neutrality 28 \
  --protect feature-map.json \
  --protect-regions face,torso,eyes \
  --feather 1 \
  --defringe white \
  --force \
  --pretty
```

Expected behavior:

- select bright low-saturation pixels, including disconnected white pockets
- preserve colored hair, skin, eyes, and uniform through protection and
  thresholding
- reuse mask polish and defringe

## Shared Data Model

Add reusable selection inputs rather than separate ad hoc flags everywhere:

```rust
pub struct SceneAssetSelectionSeed {
    pub x: f32,
    pub y: f32,
}

pub enum SceneAssetSelectionMode {
    EdgeBackground,
    ColorRange,
    MultiSeedContiguous,
}
```

The implementation should reuse the existing `SceneAssetMask` primitive from
the mask-polish pass.

Useful internal helpers:

- `color_range_mask(image, sample_colors, tolerance)`
- `multi_seed_contiguous_mask(image, seeds, tolerance)`
- `channel_matte_mask(image, threshold, neutrality)`
- `decontaminate_hair_edges(image, radius, strength, region)`
- `union_masks(a, b)`
- `apply_mask_polish(mask, options, feature_map)`

## Recipe Support

Add recipe operations so generated expression/animation work can use the same
selection primitives:

```json
{
  "op": "color_range_erase",
  "sample": "corners",
  "tolerance": 10,
  "protect_regions": ["face", "hair", "torso"],
  "dilate": 1,
  "close": 1,
  "feather": 1,
  "defringe": "white"
}
```

```json
{
  "op": "channel_matte_erase",
  "threshold": 238,
  "neutrality": 28,
  "protect_regions": ["face", "torso", "eyes"],
  "feather": 1,
  "defringe": "white"
}
```

```json
{
  "op": "hair_cleanup",
  "mode": "decontaminate",
  "radius": 4,
  "strength": 0.85,
  "region": "hair"
}
```

```json
{
  "op": "magic_erase_add",
  "seeds": [
    { "x": 0.01, "y": 0.01 },
    { "x": 0.72, "y": 0.56 },
    { "x": 0.78, "y": 0.82 }
  ],
  "tolerance": 10,
  "protect_regions": ["face", "hair", "torso"],
  "feather": 1,
  "defringe": "white"
}
```

## Test Conditions

Focused tests:

```sh
cargo test -p gameterm-visual asset_edit
cargo check -p gameterm-visual --examples
```

Required unit cases:

- Color Range selects disconnected white pockets.
- Color Range does not erase protected near-white foreground regions.
- Magic Wand add-to-selection unions multiple clicked contiguous pockets.
- Magic Wand add-to-selection does not select unclicked matching islands.
- Multi-seed output is equivalent to manually applying two seed masks and
  unioning them.
- Recipe operations deserialize and apply both selection modes.
- Feather and defringe still apply after Color Range and multi-seed selection.
- Hair cleanup recolors light edge pixels without changing alpha.
- Channel matte selects bright neutral pockets but does not select saturated
  colored hair.

Manual dogfood:

```sh
cargo run -q -p gameterm-visual --example scene_asset_edit -- color-range-erase \
  --source /Users/julianabeleda/Desktop/gameterm-vn-ai-emotion-sprites/neutral_base.png \
  --output /tmp/neutral_base-transparent-color-range.png \
  --sample corners \
  --tolerance 10 \
  --dilate 1 \
  --close 1 \
  --feather 1 \
  --defringe white \
  --force \
  --pretty
```

```sh
cargo run -q -p gameterm-visual --example scene_asset_edit -- hair-cleanup \
  --source /tmp/neutral_base-transparent-color-range.png \
  --output /tmp/neutral_base-transparent-hair-cleanup.png \
  --mode decontaminate \
  --radius 4 \
  --strength 0.85 \
  --force \
  --pretty
```

Acceptance:

- interior white pockets behind hair become transparent
- the white halo around hair is visibly reduced on a dark preview
- character skin, hair body, eyes, uniform, and highlights are not erased
- output stays 768x768 RGBA for the current Kiki source
- generated outputs stay local and are not committed

## Non-Goals

- No GUI lasso or brush selection in this pass.
- No Photoshop-compatible UI.
- No AI segmentation.
- No PSD layer parsing.
- No perfect hair matte guarantee.

## Definition Of Done

This pass is complete when:

- Color Range and Magic Wand add-selection are separate CLI commands
- both commands reuse the mask-polish pipeline
- both commands are available as recipe operations
- a hair cleanup/decontaminate command is available
- a channel-matte erase command is available
- tests prove disconnected pocket behavior
- a Kiki dark-preview smoke shows less white behind ponytail hair than the
  current edge-connected background remover

## First-Pass Result

Implemented on June 8, 2026:

- `color-range-erase`
- `magic-erase-add`
- `channel-matte-erase`
- `hair-cleanup`
- recipe operations for Color Range, Magic Wand add-selection, channel matte,
  and hair cleanup
- shared Rust mask-polish reuse for selection operations
- configurable hair decontamination radius and strength

Verification:

- `cargo test -p gameterm-visual asset_edit`: PASS, 19 tests
- `cargo check -p gameterm-visual --examples`: PASS
- `cargo test -p gameterm-visual`: PASS, 206 tests

Real Kiki smoke outputs:

- Color Range:
  `/tmp/neutral_base-transparent-color-range.png`
- Color Range plus hair cleanup:
  `/tmp/neutral_base-transparent-color-range-hair-cleanup.png`
- Color Range plus hair cleanup preview:
  `/tmp/neutral_base-transparent-color-range-hair-cleanup-preview.png`
- Channel matte:
  `/tmp/neutral_base-transparent-channel-matte.png`
- Magic Wand add-selection:
  `/tmp/neutral_base-transparent-magic-add.png`
- Magic Wand add-selection preview:
  `/tmp/neutral_base-transparent-magic-add-preview.png`
- Magic Wand add-selection plus hair cleanup:
  `/tmp/neutral_base-transparent-magic-add-hair-cleanup.png`

Smoke notes:

- Unprotected Color Range removes disconnected white pockets, but it is too
  aggressive for this Kiki source: it also erases useful white highlights,
  shirt pixels, and eye highlight pixels.
- Magic Wand add-selection keeps face, eyes, and shirt intact, but output
  quality depends on choosing good seed points. The tested seed set was useful
  for proving the primitive, not a final art pass.
- Hair cleanup/decontamination recolors remaining light edge pixels and reports
  changed-pixel counts, but it does not reconstruct missing hair strands.

Residual work:

- Add a better Kiki feature map with hair/face/eye/torso regions before using
  Color Range as a normal workflow.
- Add a small interactive seed-picking preview or mask-inspect command so manual
  `magic-erase-add` seeds can be chosen precisely.
- Consider future GUI brush support for true Refine Edge Brush behavior.
