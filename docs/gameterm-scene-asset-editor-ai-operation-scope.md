# Scene Asset Editor AI/Human Operation Scope

Date: 2026-06-09

Status: first-pass implemented.

Implemented commits:

- `7685bb7a4 [visual] add Scene asset operation runner`
- `2a26cf563 [visual] add Scene asset operation previews`
- `ce40e36fe [visual] add Scene asset operation diagnostics`
- `8c4f0aaf9 [visual] add Scene asset edit sessions`

## Purpose

The Scene asset editor now has enough deterministic image primitives to modify
Kiki-style PNG assets from terminal. The next layer is not more pixel commands.
The next layer is a stable operation contract that an AI or human can use in a
repeatable edit loop.

Target loop:

```text
inspect -> propose operation -> validate/dry-run -> preview -> run
-> compare -> accept or revise
```

The goal is to let a human or AI modify an image by calling functions, while
GameTerm keeps ownership of file paths, masks, protected regions, reports,
validation, and reproducibility.

## Current Substrate

Implemented commands already cover the first non-GUI editing substrate:

| Need | Existing command family |
|---|---|
| Understand image | `inspect`, `sample`, `point-report`, `grid-preview` |
| Select/mask | `mask-preview`, `color-range-erase`, `magic-erase-add`, `channel-matte-erase` |
| Remove/repair | `remove-background-polished`, `hair-cleanup`, `restore-from-source` |
| Lay down pixels | `fill-region`, `sample-fill`, `alpha-paint`, `clone-stamp`, `draw-shape`, `stroke-path` |
| Reframe image | `crop`, `pad`, `transform` |
| Tune appearance | `levels`, `brightness-contrast`, `hsl`, `blur`, `unsharp-mask` |
| Compose/state | `composite`, `state-manifest`, `state-render`, `state-sheet` |
| Repeat work | `pipeline-run` |

This is enough to perform deterministic Kiki neutral-base work such as:

- remove white pockets
- protect face/hair/body regions
- sample-fill small missing regions
- clone-stamp nearby hair/skin texture
- restore traced pixels from the original source
- draw small corrections
- validate and compare outputs

## Gap

The primitives existed, but the AI/human contract was command-shaped rather
than operation-shaped.

First-pass resolved:

- `operation-run` accepts one versioned operation JSON envelope.
- `--preview` writes review artifacts under `Transformation` without accepting
  the requested final output.
- Operation reports include before/after image reports, compare output,
  expectation failures, preview paths, and step reports.
- `compare` provides before/after quality checks with changed pixel count,
  changed bounds, alpha changes, dimensions, and checksums.
- structured operation error reports expose stable codes and hints for common
  correction paths.
- `session-run` executes an ordered session file and records operation order
  plus final output.

Still deferred:

- Mask export/import is implicit through preview/cutout commands, not a clean
  bridge for external semantic inpainting tools.
- `accept_output` is still a manual copy/review policy, not a dedicated
  command.

## Design Principle

Do not make the AI own image editing state.

GameTerm should own:

- source/input roots
- output roots
- normalized coordinate system
- masks and protected regions
- operation validation
- deterministic execution
- output reports
- before/after comparison
- final acceptance into `Output`

The AI or human should only choose operations and parameters.

## Operation Schema

Add a versioned operation envelope:

```json
{
  "asset_operation_version": 1,
  "id": "repair-hair-pocket-001",
  "intent": "Fill a small white pocket near Kiki's ponytail using nearby hair color.",
  "source": "neutral_base.png",
  "output": "repair-hair-pocket-001.png",
  "command": "sample-fill",
  "args": {
    "sample_point": "0.72,0.38",
    "within_polygons": [
      "0.70,0.35;0.76,0.35;0.76,0.42;0.70,0.42"
    ],
    "protect_regions": ["face", "eyes", "mouth"]
  },
  "expectations": {
    "max_changed_pixel_ratio": 0.02,
    "must_preserve_alpha_outside_region": true,
    "review_points": ["0.72,0.38", "0.74,0.40"]
  }
}
```

The operation envelope should be accepted by a single runner:

```sh
cargo run -q -p gameterm-visual --example scene_asset_edit -- operation-run \
  --operation repair-hair-pocket-001.json \
  --input-root "Image Editor/Input" \
  --transformation-root "Image Editor/Transformation" \
  --output-root "Image Editor/Output" \
  --dry-run \
  --pretty
```

The implementation reuses the existing `pipeline-run` internals. A single
operation is executed as one pipeline step plus a richer report envelope.

## Operation Report

Every operation should produce one report shape:

```json
{
  "operation": "operation_run",
  "id": "repair-hair-pocket-001",
  "command": "sample-fill",
  "intent": "Fill a small white pocket near Kiki's ponytail using nearby hair color.",
  "source": ".../neutral_base.png",
  "output": ".../repair-hair-pocket-001.png",
  "dry_run": false,
  "status": "ok",
  "before": {
    "width": 1024,
    "height": 1024,
    "checksum": "..."
  },
  "after": {
    "width": 1024,
    "height": 1024,
    "checksum": "..."
  },
  "diff": {
    "changed_pixels": 431,
    "changed_bounds": "0.70,0.35,0.06,0.07",
    "changed_pixel_ratio": 0.00041,
    "alpha_changed_outside_region": false
  },
  "review": {
    "preview_path": ".../repair-hair-pocket-001.preview.png",
    "sample_report_path": ".../repair-hair-pocket-001.samples.json"
  }
}
```

This makes the result inspectable by both a person and an AI.

## Function Surface

The AI-facing function surface should be small and stable:

| Function | Purpose |
|---|---|
| `asset.inspect` | Return image metadata, alpha bounds, checksum |
| `asset.sample` | Return colors and alpha around points/regions |
| `asset.validate_operation` | Check paths, command, args, protected regions, expected bounds |
| `asset.preview_operation` | Produce mask/diff/overlay preview without accepting output |
| `asset.run_operation` | Execute one operation and write report |
| `asset.compare` | Compare source/output and report changed pixels/bounds/checksum |
| `asset.run_pipeline` | Execute an ordered sequence of operations |
| `asset.run_session` | Execute an ordered session file |
| `asset.accept_output` | Copy a reviewed transformation into `Output` |

The CLI now exposes the implemented operation/session/compare surface. A future
GUI, Codex tool, or local agent can call the same Rust functions directly.

## Semantic Inpainting Bridge

Semantic inpainting should be treated as an optional backend, not the default
editor model.

Bridge shape:

```text
GameTerm mask/protect regions -> external inpaint backend -> output PNG
-> GameTerm composite masked pixels back -> compare/report -> accept or reject
```

Scoped commands:

```sh
scene_asset_edit mask-export --source IMAGE --output MASK.png ...
scene_asset_edit inpaint-bridge --source IMAGE --mask MASK.png \
  --prompt PROMPT.txt --backend-command COMMAND --output IMAGE
scene_asset_edit mask-composite --source IMAGE --patch IMAGE --mask MASK.png \
  --output IMAGE
```

Non-goal for this pass: embedded machine learning. The first bridge should call
an external local tool and then bring the result back through GameTerm's
deterministic compare/cleanup pipeline.

## Implementation Lanes

### Lane 1: Operation Envelope

Add:

- `SceneAssetOperation`
- `SceneAssetOperationExpectations`
- `SceneAssetOperationRunReport`
- `operation-run` CLI command

Definition of done:

- validates `asset_operation_version`
- supports one operation command
- supports `--dry-run`
- resolves paths through existing Input/Transformation/Output roots
- rejects unknown commands and unsafe outputs before writing
- reuses existing command implementations

Commit:

- `[visual] add Scene asset operation runner`

Status: complete.

### Lane 2: Unified Compare Report

Add:

- `compare` command
- before/after checksums
- changed pixel count
- changed bounds
- changed alpha count
- optional region/protected-region violation checks

Definition of done:

- can compare any two same-size PNGs
- reports dimension mismatch clearly
- operation reports embed compare results
- tests cover no-op, bounded change, alpha-only change, and dimension mismatch

Commit:

- `[visual] add Scene asset compare reports`

Status: implemented inside `[visual] add Scene asset operation runner`.

### Lane 3: Preview Artifacts

Add:

- operation preview mode
- diff overlay PNG
- optional dark-background preview for alpha work
- optional mask preview path in operation reports

Definition of done:

- preview writes to `Transformation`
- preview never mutates `Output`
- report links preview paths
- tests prove dry-run does not write destructive outputs

Commit:

- `[visual] add Scene asset operation previews`

Status: complete.

### Lane 4: AI-Correctable Errors

Normalize operation errors:

```json
{
  "status": "error",
  "code": "unknown_region",
  "message": "region `hair_fringe` does not exist",
  "hint": "Run map-template or choose one of: face, hair, eyes, mouth"
}
```

Definition of done:

- major validation failures have stable error codes
- error reports are JSON-serializable
- CLI still prints concise human stderr
- tests cover common correction paths

Commit:

- `[visual] add Scene asset operation diagnostics`

Status: complete.

### Lane 5: Operation Session Files

Add an optional session file:

```json
{
  "asset_session_version": 1,
  "name": "kiki-neutral-repair",
  "current_source": "neutral_base.png",
  "accepted_outputs": [],
  "operations": [
    "repair-hair-pocket-001.json"
  ]
}
```

Definition of done:

- sessions are append-only by default
- `session-run` can execute all pending operations
- reports record operation order and final output
- user can resume an interrupted edit session

Commit:

- `[visual] add Scene asset edit sessions`

Status: complete.

### Lane 6: Docs And Agent Prompt

Add:

- AI/human operation cookbook
- prompt template for an AI image editor loop
- examples against the Kiki neutral base

Definition of done:

- documents function surface and JSON schema
- includes a safe first Kiki repair operation
- explains when to use deterministic operations vs external inpainting
- records verification commands and local smoke outputs

Commit:

- `[docs] document Scene asset operation workflow`

Status: complete after the docs/fixture pass paired with this scope.

## Definition Of Done

The AI/human operation layer is complete when:

- a human can run one JSON operation without remembering command flags: done
- an AI can propose one JSON operation, validate it, run it, inspect the report,
  and revise parameters: done
- every operation has a dry-run path: done
- every operation has a stable report path: done
- before/after comparison is automatic for image-writing operations: done
- protected regions can be asserted: inherited from the operation command args
- errors are structured enough for automatic correction: done
- pipeline/session files can chain accepted operations: done
- no embedded ML is required: done

## Non-Goals

- No GUI canvas in this scope.
- No embedded machine learning.
- No network dependency.
- No automatic visual taste judgment.
- No destructive in-place edits.
- No hidden writes outside `Input`, `Transformation`, or `Output` roots unless a
  user explicitly provides an absolute path.

## Follow-Up

The next asset-editor layer should be GUI-specific: file browser, point picker,
polygon/lasso drawing, preview comparison panes, operation history, and an
explicit accept/copy command if the GUI needs one.
