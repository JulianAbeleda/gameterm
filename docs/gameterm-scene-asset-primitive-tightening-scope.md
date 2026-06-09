# Scene Asset Primitive Tightening Scope

Date: 2026-06-09

Status: scoped.

## Purpose

The GUI is intentionally deferred. The current priority is to tighten the Rust
primitive layer so GameTerm's image editor works as a deterministic API for a
human, Codex, or another local agent.

The target is not a paint application UI yet. The target is a stable function
surface:

```text
inspect -> select/mask -> preview -> operate -> compare -> accept
```

Every step should be scriptable, reportable, and safe to compose without
depending on a GUI, network, Python, or embedded ML.

## Current State

Implemented first-pass substrate:

- image inspection, sampling, coordinate grids, and point reports
- background removal, color range, magic add-selection, channel matte, mask
  preview, and hair cleanup
- fill, sample-fill, alpha paint, clone stamp, draw shape, stroke path, and
  restore from source
- crop, pad, transform, levels, brightness/contrast, HSL, blur, and unsharp
  mask
- composite, state manifests, state render, state sheet, and continuity
  reports
- pipeline JSON and operation JSON runners
- preview mode with operation preview and basic diff PNG
- before/after compare reports
- structured operation diagnostics
- ordered edit sessions

The useful missing work is now refinement, not breadth.

## Product Rule

GameTerm should own image-editing state and safety.

The caller should provide intent and parameters. GameTerm should provide:

- root ownership for `Input`, `Transformation`, and `Output`
- explicit acceptance into `Output`
- mask files and mask provenance
- protected-region assertions
- stable validation reports
- richer visual diffs
- deterministic operation/session replay

## Non-Goals

- No GUI canvas in this scope.
- No mouse/lasso UI in this scope.
- No embedded machine learning.
- No network dependency.
- No automatic visual taste judgment.
- No destructive in-place writes.
- No broad refactor unrelated to asset primitives.

## Core Concepts

### Input

Original or imported source PNGs. Commands can read from `Input`, but should
not write there.

### Transformation

Intermediate and reviewable outputs: previews, masks, operation outputs, diff
PNGs, reports, and session artifacts.

### Output

Accepted outputs only. Files should arrive here through a tracked acceptance
step, not because an intermediate command happened to write there.

### Mask

A mask is a first-class artifact, not just a transient selection. It should be
usable by deterministic commands and by future external inpainting bridges.

First pass mask format:

- PNG, same dimensions as source
- alpha or luminance encodes selected pixels
- white or alpha 255 means selected
- black or alpha 0 means unselected

### Protected Region

A named or explicit region that must not be changed unless the operation says
so. Protection should be assertable after the command, not only used as a mask
input before the command.

## Lane 1: Accept Output

Add an explicit acceptance command.

CLI:

```sh
cargo run -q -p gameterm-visual --example scene_asset_edit -- accept-output \
  --source Transformation/neutral_base-transparent.review.png \
  --output neutral_base-transparent.png \
  --input-root "Image Editor/Input" \
  --transformation-root "Image Editor/Transformation" \
  --output-root "Image Editor/Output" \
  --report "Image Editor/Transformation/accept-neutral-base.json" \
  --pretty
```

Behavior:

- source may be absolute or under `Transformation`
- destination resolves under `Output` unless explicitly absolute
- no overwrite without `--force`
- validate source is a readable PNG
- write an acceptance report with source path, output path, dimensions,
  checksum, and timestamp-like deterministic metadata if available
- never mutate `Input`

Report shape:

```json
{
  "operation": "accept_output",
  "source_path": ".../Transformation/neutral_base-transparent.review.png",
  "output_path": ".../Output/neutral_base-transparent.png",
  "status": "ok",
  "image": {
    "width": 1024,
    "height": 1024,
    "sha256": "..."
  }
}
```

Definition of done:

- command exists in the CLI
- Rust function exists for direct callers
- overwrite behavior is covered by tests
- acceptance report is JSON-serializable
- cookbook includes the acceptance workflow

Commit:

- `[visual] add Scene asset output acceptance`

## Lane 2: Operation Validation Mode

Add validation without writes.

CLI:

```sh
cargo run -q -p gameterm-visual --example scene_asset_edit -- validate-operation \
  --operation repair-hair-pocket-001.json \
  --input-root "Image Editor/Input" \
  --transformation-root "Image Editor/Transformation" \
  --output-root "Image Editor/Output" \
  --output "Image Editor/Transformation/repair-hair-pocket-001.validation.json" \
  --pretty
```

Behavior:

- parse operation JSON
- validate version, id, source, output, command, args, feature maps, protected
  regions, and output overwrite policy
- do not load and transform the full image unless needed for dimension or
  feature-map validation
- return structured diagnostics using the existing operation error report shape
- return success report when the operation is safe to preview or run

Validation report:

```json
{
  "operation": "validate_operation",
  "id": "repair-hair-pocket-001",
  "status": "ok",
  "source_path": "...",
  "requested_output_path": "...",
  "command": "sample-fill",
  "warnings": []
}
```

Definition of done:

- validation command rejects the same invalid operations as `operation-run`
- validation does not write operation output PNGs
- tests cover valid operation, unknown command, missing source, unsafe output,
  invalid args, and unknown protected region

Commit:

- `[visual] add Scene asset operation validation`

## Lane 3: Mask Export, Import, And Composite

Make masks explicit artifacts.

### Mask Export

CLI:

```sh
cargo run -q -p gameterm-visual --example scene_asset_edit -- mask-export \
  --source neutral_base.png \
  --output hair-pocket-mask.png \
  --selection-mode color-range \
  --within-polygon '0.64,0.20;0.90,0.20;0.90,0.86;0.62,0.86' \
  --tolerance 10 \
  --input-root "Image Editor/Input" \
  --transformation-root "Image Editor/Transformation" \
  --pretty
```

Behavior:

- build a selection mask using existing selection primitives
- apply protection before export when requested
- write a mask PNG under `Transformation`
- write a mask report with selected pixel count and bounds

### Mask Import

CLI:

```sh
cargo run -q -p gameterm-visual --example scene_asset_edit -- mask-apply-alpha \
  --source neutral_base.png \
  --mask hair-pocket-mask.png \
  --output neutral_base-mask-applied.png \
  --alpha 0 \
  --pretty
```

Behavior:

- load a mask PNG
- apply alpha or color operation only where mask is selected
- assert mask dimensions match the source image
- preserve source RGB unless an explicit color operation is requested

### Mask Composite

CLI:

```sh
cargo run -q -p gameterm-visual --example scene_asset_edit -- mask-composite \
  --source neutral_base.png \
  --patch repaired-pocket.png \
  --mask hair-pocket-mask.png \
  --output neutral_base-repaired.png \
  --pretty
```

Behavior:

- copy pixels from patch into source where mask is selected
- reject mismatched dimensions
- optionally feather the mask edge
- compare/report changed bounds

Definition of done:

- exported masks can round-trip through apply/composite
- mask files can be generated from existing color range and magic-add selection
  logic
- tests cover dimension mismatch, binary mask, alpha mask, and composite bounds
- future external inpainting tools can use this bridge without changing core
  editor commands

Commit:

- `[visual] add Scene asset mask roundtrip`

## Lane 4: Stronger Protected-Region Assertions

Turn protection into a post-operation invariant.

Current behavior restores/protects regions during many commands, but reports do
not yet make protected-region violations obvious enough for automated callers.

Add expectation fields:

```json
{
  "expectations": {
    "max_changed_pixel_ratio": 0.02,
    "must_preserve_alpha_outside_region": true,
    "must_preserve_regions": ["face", "eyes", "mouth"],
    "max_changed_pixels_in_protected_regions": 0,
    "review_points": ["0.72,0.38"]
  }
}
```

Report additions:

```json
{
  "protected_region_report": {
    "checked_regions": ["face", "eyes", "mouth"],
    "changed_pixels": 0,
    "changed_regions": []
  },
  "expectation_failures": []
}
```

Behavior:

- compare before/after inside named feature-map regions
- report changed pixel count per protected region
- fail the operation status when thresholds are exceeded
- leave the output artifact for inspection unless the operation failed before
  writing

Definition of done:

- protected-region assertions work for operation reports
- tests prove an intentionally modified protected region fails
- tests prove unchanged protected regions pass
- docs explain that `protect_regions` is mask-time protection while
  `must_preserve_regions` is report-time assertion

Commit:

- `[visual] assert Scene asset protected regions`

## Lane 5: Richer Diff And Review Previews

Make visual review easier without a GUI.

Add preview variants:

- raw diff: changed pixels highlighted over transparent background
- overlay diff: changed pixels highlighted over source
- alpha diff: alpha-only changes highlighted
- checkerboard preview: transparent output over a checkerboard
- dark preview: transparent output over dark background
- side-by-side contact sheet: before, after, diff

CLI:

```sh
cargo run -q -p gameterm-visual --example scene_asset_edit -- diff-preview \
  --before neutral_base.png \
  --after neutral_base-transparent.png \
  --output neutral_base-review-sheet.png \
  --mode contact-sheet \
  --pretty
```

Operation preview additions:

```json
{
  "preview_paths": {
    "raw_diff": "...diff.png",
    "alpha_diff": "...alpha-diff.png",
    "checkerboard": "...checkerboard.png",
    "contact_sheet": "...review.png"
  }
}
```

Definition of done:

- `diff-preview` works as a standalone command
- `operation-run --preview` can emit the richer review paths
- tests cover color-only diff, alpha-only diff, no-op diff, and transparent
  checkerboard preview

Commit:

- `[visual] add Scene asset review previews`

## Lane 6: Real Kiki Primitive Smoke Fixtures

The repo-safe 32x32 fixtures prove code paths, but they do not prove the
primitives are useful on the real Kiki-style local assets.

Add local smoke documentation and optional fixture recipes that can run when the
user has the desktop asset folder:

```text
/Users/julianabeleda/Desktop/gameterm-vn-ai-emotion-sprites/Image Editor
```

Smoke path:

```text
inspect real Kiki source
-> export mask
-> preview mask
-> apply mask or composite
-> run protected-region assertions
-> generate review sheet
-> accept reviewed output
```

Definition of done:

- smoke report records exact local commands
- docs state which paths are local and not repo assets
- CI remains independent of local desktop assets
- repo fixture tests still use small safe PNGs

Commit:

- `[docs] record Scene asset primitive smoke`

## Lane 7: Cookbook And Agent Contract Refresh

Update the user-facing cookbook after the primitive pass.

Add:

- accept-output workflow
- validate-operation workflow
- mask export/import/composite workflow
- protected-region assertion examples
- diff/review preview examples
- updated AI prompt template that requires validation before preview/run

Definition of done:

- the cookbook describes the whole no-GUI loop
- every new command has one copy-pasteable example
- the roadmap marks primitive tightening as first-pass implemented after the
  implementation commits land

Commit:

- `[docs] document Scene asset primitive workflow`

## Implementation Order

Recommended order:

1. `accept-output`
2. `validate-operation`
3. mask export/import/composite
4. protected-region assertions
5. richer diff/review previews
6. real local smoke docs
7. cookbook and roadmap refresh

Reasoning:

- acceptance makes the `Transformation` to `Output` boundary explicit first
- validation makes later commands safer to call from agents
- mask round-trip is the bridge for future semantic inpainting or manual masks
- protected assertions make reports trustworthy
- review previews improve dogfooding without a GUI

## Test Matrix

Rust tests:

- `accept_output_writes_report_and_refuses_overwrite_without_force`
- `validate_operation_reports_success_without_writing_output`
- `validate_operation_reports_unknown_command`
- `mask_export_roundtrips_through_apply_alpha`
- `mask_composite_rejects_dimension_mismatch`
- `protected_region_assertion_fails_when_region_changes`
- `protected_region_assertion_passes_when_region_is_restored`
- `diff_preview_highlights_color_changes`
- `diff_preview_highlights_alpha_changes`
- `review_contact_sheet_preserves_dimensions`

CLI smoke:

```sh
cargo test -p gameterm-visual asset_edit
cargo check -p gameterm-visual --examples
cargo test -p gameterm-visual
git diff --check
```

Optional local smoke:

```sh
cargo run -q -p gameterm-visual --example scene_asset_edit -- validate-operation ...
cargo run -q -p gameterm-visual --example scene_asset_edit -- operation-run --preview ...
cargo run -q -p gameterm-visual --example scene_asset_edit -- mask-export ...
cargo run -q -p gameterm-visual --example scene_asset_edit -- diff-preview ...
cargo run -q -p gameterm-visual --example scene_asset_edit -- accept-output ...
```

## Completion Criteria

This scope is complete when:

- no GUI is required to do a safe edit loop
- outputs only enter `Output` through an explicit acceptance path
- masks are durable artifacts that can leave and re-enter GameTerm
- operation validation is available without image writes
- protected regions can be asserted after an operation
- review previews are rich enough to inspect alpha and pixel changes from a
  terminal file path
- docs give both a human and an AI the same reproducible workflow

## After This Scope

Once this is complete, the primitive layer should be tight enough for:

- a GUI wrapper
- an Arkey-style agent routing layer
- optional external semantic inpainting bridge
- more advanced character expression editing

Those should remain separate scopes.
