# GameTerm Scene Asset Editor Refactor Scope

Date: 2026-06-09

Status: first pass implemented.

## Assessment Grade

Current repo grade after this pass: **A-**.

The codebase is in a good product position:

- worktree is clean and synced with `origin/main`
- Scene asset editor primitives are first-pass complete
- tests cover the risky image-editing paths
- commit discipline has improved
- docs, handoff, smoke report, and roadmap are current

The remaining reason this is A- rather than a stricter A is residual
operation-family concentration:

- `gameterm-visual/src/asset_edit.rs` is down to roughly 3.9k lines from about
  9.9k lines
- it now acts as the public API facade plus operation command surface
- models, IO, roots, mask core, pixel primitives, composite/state rendering,
  review previews, recipe/continuity helpers, pipeline argument parsing,
  operation diagnostics, and tests have clear owners
- the next strict-A cleanup would split the remaining public mask/paint/filter
  operation bodies into family modules

This was a refactor problem, not a correctness problem. The primitive layer
continues to work, and this pass preserved behavior.

## Principles Applied

This scope follows `structure/Development/coding-principles.md`:

- preserve upstream behavior unless a task explicitly scopes a fork-specific
  change
- keep GameTerm-specific changes narrow and easy to review
- prefer existing crate/module boundaries before adding new crates
- do not mix NFC refactors with behavior changes
- keep commits small and subsystem-prefixed
- use focused checks first, then broaden when touching shared behavior
- keep `structure/` and docs factual, compact, and free of transient logs

Commit prefixes for this pass:

- `[visual] NFC` for behavior-preserving Rust module splits in
  `gameterm-visual`
- `[test] NFC` for test-only relocation or naming cleanup
- `[docs]` for scope, roadmap, handoff, and smoke docs

If a real behavior bug is found during refactor, stop the NFC lane and land the
fix as its own `[visual]` or `[test]` commit before continuing.

## Current Hotspots

Scene asset-editor size snapshot after this pass:

```text
3853  gameterm-visual/src/asset_edit.rs
2280  gameterm-visual/src/asset_edit/tests.rs
1315  gameterm-visual/src/asset_edit/model.rs
647   gameterm-visual/src/asset_edit/pipeline_args.rs
466   gameterm-visual/src/asset_edit/pixels.rs
333   gameterm-visual/src/asset_edit/mask.rs
253   gameterm-visual/src/asset_edit/review.rs
246   gameterm-visual/src/asset_edit/recipes.rs
230   gameterm-visual/src/asset_edit/composite.rs
180   gameterm-visual/src/asset_edit/operation_support.rs
118   gameterm-visual/src/asset_edit/io.rs
103   gameterm-visual/src/asset_edit/roots.rs
```

Why `asset_edit.rs` goes first:

- it grew most recently from valid product work
- it is all GameTerm-specific, so upstream risk is low
- its internal domains are clear enough for mechanical moves
- the test suite is already strong enough to protect NFC extraction
- it directly supports the user's current goal: tightening primitives before
  returning to GUI work

Why other hotspots wait:

- `gameterm-visual/src/lib.rs` is large but already heavily test-protected and
  mostly Scene runtime/schema; broad schema moves are higher churn
- `visual.rs`, TTS, compose, STT, and render already had a first extraction pass
- CI helper cleanup is lower priority until smoke maintenance becomes the
  blocker
- mux is not a line-count or memory shortcut and should not be removed as
  cleanup

## Goal

Make the Scene asset editor easier to extend without changing behavior.

After this refactor:

- callers should still import the same public `gameterm_visual::*` API
- the CLI example should keep the same command names and flags
- existing JSON operation/session/pipeline formats should remain stable
- tests should remain at least as strong as today
- each image-editing concern should have a clear module owner

Implemented module shape:

```text
gameterm-visual/src/
  asset_edit.rs                  facade, public API, operation command surface
  asset_edit/
    model.rs                     DTOs, options, reports, error type
    io.rs                        JSON/image load-save, hashes, output writes
    roots.rs                     Input/Transformation/Output path resolution
    mask.rs                      mask data structure and morphology core
    pixels.rs                    pixel indexing, drawing, blend, transform helpers
    composite.rs                 layers, blend modes, state manifests/sheets
    review.rs                    diff previews, contact sheets, preview path names
    operation_support.rs         operation diagnostics and expectation checks
    pipeline_args.rs             pipeline argument parsing and region validation
    recipes.rs                   expression, animation, continuity helpers
    tests.rs                     asset editor regression suite
```

Paint/filter public command bodies remain in the facade for now because moving
them would require broader family-module extraction. Ownership is clear enough
for the next GUI/semantic-editing work.

## Non-Goals

- No behavior changes.
- No new image primitives.
- No GUI work.
- No CLI flag changes.
- No JSON schema changes.
- No new crate.
- No upstream terminal/mux/render refactor.
- No deletion of tests to reduce line count.
- No broad rename of Scene asset types.

## Lane 0: Baseline And Guardrails

Owner prefix: `[docs]`, `[test]`

Purpose: make the refactor measurable before moving code.

Actions:

- record current hotspots and assessment grade
- record current verification commands
- confirm worktree is clean before code movement
- keep the primitive smoke command in the smoke report as the behavior contract

Verification:

```sh
git status --short --branch
cargo test -p gameterm-visual asset_edit
cargo check -p gameterm-visual --examples
git diff --check
```

Definition of done:

- this scope is committed
- no behavior changes are mixed into the scope commit
- first implementation lane has a narrow target

## Lane 1: Model And Error Types

Owner prefix: `[visual] NFC`

Purpose: separate data contracts from algorithms.

Move to `asset_edit/model.rs`:

- `SceneAssetImageReport`
- coordinate types
- feature-map and recipe DTOs
- pipeline, operation, session DTOs
- all option/report DTOs
- `SceneAssetEditOperation`
- `SceneAssetEditError`
- small impls that belong to those types

Keep in facade:

- public re-exports so existing callers do not change
- no semantic changes to serde defaults or enum tagging

Verification:

```sh
cargo test -p gameterm-visual asset_edit
cargo check -p gameterm-visual --examples
```

Pause criteria:

- serde attributes become harder to audit
- public API import churn spreads outside `asset_edit`
- tests need rewrites beyond module paths

## Lane 2: IO And Root Ownership

Owner prefix: `[visual] NFC`

Purpose: isolate filesystem behavior from image algorithms.

Move to `asset_edit/io.rs`:

- JSON read/write helpers
- image load/save helpers
- SHA and image report helpers
- force/overwrite handling
- content bounds helpers if the dependency stays simple

Move to `asset_edit/roots.rs`:

- `Input`, `Transformation`, `Output` resolution
- prefixed path resolution
- output acceptance path helpers
- report path naming helpers

Keep behavior:

- no change to overwrite rules
- no change to default roots
- no change to absolute path handling
- no change to `accept-output`

Verification:

```sh
cargo test -p gameterm-visual accept_output
cargo test -p gameterm-visual validate_operation
cargo test -p gameterm-visual asset_edit
cargo check -p gameterm-visual --examples
```

## Lane 3: Mask And Selection Module

Owner prefix: `[visual] NFC`

Purpose: put all selection/mask primitives in one place.

Move to `asset_edit/masks.rs`:

- background magic masks
- color range masks
- magic-add masks
- channel matte masks
- mask polish/morphology
- protected/within region mask application
- mask preview rendering
- mask export image encoding
- mask import decoding
- mask apply alpha
- mask composite
- edge defringe and hair cleanup only if the dependency stays local

Public functions remain available through `asset_edit.rs`:

- `preview_scene_asset_selection_mask`
- `export_scene_asset_selection_mask`
- `apply_scene_asset_mask_alpha`
- `composite_scene_asset_mask`
- `make_scene_asset_background_transparent`
- `make_scene_asset_background_transparent_polished`
- `color_range_erase_scene_asset_image`
- `magic_erase_add_scene_asset_image`
- `channel_matte_erase_scene_asset_image`
- `cleanup_scene_asset_hair_edges`

Verification:

```sh
cargo test -p gameterm-visual mask_
cargo test -p gameterm-visual color_range
cargo test -p gameterm-visual magic_erase
cargo test -p gameterm-visual channel_matte
cargo test -p gameterm-visual hair_cleanup
cargo test -p gameterm-visual asset_edit
```

## Lane 4: Paint, Draw, And Transform Modules

Owner prefix: `[visual] NFC`

Purpose: separate pixel editing algorithms by family.

Move to `asset_edit/paint.rs`:

- fill region
- sample fill
- alpha paint
- clone stamp
- draw shape
- stroke path
- low-level line/disk/ellipse helpers
- region mask application helpers if mostly paint-owned

Move to `asset_edit/transform.rs`:

- crop
- pad
- scale/translate/flip
- levels
- brightness/contrast
- HSL
- blur
- unsharp mask
- resample/filter helpers

Keep behavior:

- no pixel output changes
- no default value changes
- no command argument changes

Verification:

```sh
cargo test -p gameterm-visual fill_region
cargo test -p gameterm-visual sample_fill
cargo test -p gameterm-visual alpha_paint
cargo test -p gameterm-visual clone_stamp
cargo test -p gameterm-visual draw_shape
cargo test -p gameterm-visual crop_pad
cargo test -p gameterm-visual tonal
cargo test -p gameterm-visual asset_edit
```

## Lane 5: Composite, State, Recipe, And Review Modules

Owner prefix: `[visual] NFC`

Purpose: separate output assembly/review from edit execution.

Move to `asset_edit/composite.rs`:

- blend modes
- composite layers
- state manifests
- state render
- state sheet render

Move to `asset_edit/recipes.rs`:

- expression generation
- animation generation
- source export
- continuity reports
- source restore if it fits better with recipe/edit workflows than masks

Move to `asset_edit/review.rs`:

- before/after compare
- raw diff
- overlay diff
- alpha diff
- checkerboard/dark preview
- contact sheet
- review preview path construction

Verification:

```sh
cargo test -p gameterm-visual composite
cargo test -p gameterm-visual state_
cargo test -p gameterm-visual animation
cargo test -p gameterm-visual continuity
cargo test -p gameterm-visual diff_preview
cargo test -p gameterm-visual review_contact
cargo test -p gameterm-visual asset_edit
```

## Lane 6: Operation And Pipeline Runners

Owner prefix: `[visual] NFC`

Purpose: separate orchestration from primitive algorithms.

Move to `asset_edit/operation.rs`:

- `run_scene_asset_operation`
- `validate_scene_asset_operation`
- `run_scene_asset_edit_session`
- operation error reporting
- protected-region post-checks
- expectation failure construction
- operation preview output naming if review dependency is simple

Move to `asset_edit/pipeline.rs`:

- `run_scene_asset_pipeline`
- step command validation
- step argument parsing
- pipeline command dispatch
- command advances-source policy

Keep behavior:

- no command names change
- no JSON field names change
- no validation strings change unless unavoidable
- no preview/acceptance path changes

Verification:

```sh
cargo test -p gameterm-visual operation_run
cargo test -p gameterm-visual validate_operation
cargo test -p gameterm-visual session_run
cargo test -p gameterm-visual pipeline_run
cargo test -p gameterm-visual protected_region
cargo test -p gameterm-visual asset_edit
```

## Lane 7: CLI Example Follow-Up

Owner prefix: `[visual] NFC`, `[test] NFC`

Purpose: only refactor the example if module extraction leaves it harder to
read.

Current issue:

`gameterm-visual/examples/scene_asset_edit.rs` is about 1.8k lines and carries:

- a large hand-written `CliArgs`
- a large usage string
- a large command match
- repeated `run_*` wrappers
- command-specific parser rules

Allowed cleanup:

- group helper functions by command family
- move parser-only helpers into a private module inside the example if that
  reduces scanning cost
- keep all command names, options, and output behavior identical

Do not:

- introduce a new CLI parser crate in this pass
- change user-facing syntax
- rewrite every command wrapper only for style

Verification:

```sh
cargo check -p gameterm-visual --examples
cargo test -p gameterm-visual asset_edit
```

## Lane 8: Test Locality

Owner prefix: `[test] NFC`

Purpose: keep tests close to the module they protect without weakening them.

Actions:

- move test helpers into `asset_edit/tests.rs` or per-module `#[cfg(test)]`
  blocks only when it improves locality
- keep broad operation/pipeline tests where cross-module behavior is
  intentional
- keep fixture-based smoke docs unchanged
- avoid reducing assertion detail

Verification:

```sh
cargo test -p gameterm-visual asset_edit
cargo test -p gameterm-visual
```

## Recommended Execution Order

1. Lane 0: Baseline and guardrails
2. Lane 1: Model and error types
3. Lane 2: IO and root ownership
4. Lane 3: Mask and selection module
5. Lane 4: Paint, draw, and transform modules
6. Lane 5: Composite, state, recipe, and review modules
7. Lane 6: Operation and pipeline runners
8. Lane 8: Test locality
9. Lane 7: CLI example follow-up, only if still valuable

Reasoning:

- model extraction gives later modules stable shared types
- IO/root helpers reduce repeated filesystem concerns early
- masks and paint are the highest-growth primitive families
- operation/pipeline extraction should happen after primitives have owners
- tests move last so they can follow the final module shape

## Verification Matrix

Minimum checks by lane:

| Lane | Focused checks | Broad checks |
| --- | --- | --- |
| Model | `cargo test -p gameterm-visual asset_edit` | `cargo check -p gameterm-visual --examples` |
| IO/roots | `cargo test -p gameterm-visual accept_output validate_operation` | `cargo test -p gameterm-visual asset_edit` |
| Masks | `cargo test -p gameterm-visual mask_ color_range magic_erase` | `cargo test -p gameterm-visual asset_edit` |
| Paint/transform | `cargo test -p gameterm-visual fill_region draw_shape crop_pad tonal` | `cargo test -p gameterm-visual asset_edit` |
| Composite/review | `cargo test -p gameterm-visual composite diff_preview review_contact` | `cargo test -p gameterm-visual asset_edit` |
| Operation/pipeline | `cargo test -p gameterm-visual operation_run pipeline_run session_run protected_region` | `cargo test -p gameterm-visual asset_edit` |
| CLI example | `cargo check -p gameterm-visual --examples` | `cargo test -p gameterm-visual asset_edit` |
| Final pass | `cargo test -p gameterm-visual` | `git diff --check` |

Before final closeout:

```sh
cargo test -p gameterm-visual asset_edit
cargo check -p gameterm-visual --examples
cargo test -p gameterm-visual
git diff --check
```

Optional smoke, if any public command wiring changes:

```sh
rm -rf /tmp/gameterm-scene-asset-refactor-smoke
mkdir -p /tmp/gameterm-scene-asset-refactor-smoke/Transformation \
  /tmp/gameterm-scene-asset-refactor-smoke/Output

cargo run -q -p gameterm-visual --example scene_asset_edit -- validate-operation \
  --operation ci/fixtures/gameterm-scene/kiki-asset-operation-draw-shape.json \
  --input-root ci/fixtures/gameterm-scene/vn-asset-source \
  --transformation-root /tmp/gameterm-scene-asset-refactor-smoke/Transformation \
  --output-root /tmp/gameterm-scene-asset-refactor-smoke/Output \
  --pretty \
  --force
```

## Stop Conditions

Pause and rescope if:

- a move changes public JSON serialization
- a move changes CLI behavior
- a move requires broad imports across unrelated Scene modules
- a helper needs to become public only to satisfy a module split
- tests require large rewrites unrelated to moved ownership
- the diff becomes mostly formatting churn
- a product bug appears during the refactor

## Definition Of Done

This refactor pass is complete when:

- `asset_edit.rs` is a facade and public API owner, not the home of every
  shared support algorithm
- masks, paint/draw, transforms, review, operation, pipeline, and IO each have
  clear module ownership
- public imports remain compatible for the example CLI and callers
- operation/session/pipeline JSON remains stable
- no behavior changes are hidden inside NFC commits
- the focused and broad verification commands pass
- the roadmap and handoff describe the new module layout

## Actual Grade After Completion

Repo grade after this pass: **A-**.

The codebase will still be a large terminal emulator fork with real complexity.
That is appropriate. The improvement is that the newest GameTerm-owned
complexity is now split by primitive responsibility, so future GUI or
semantic-editing work can build on the image editor without reopening a
9k-line all-purpose module.
