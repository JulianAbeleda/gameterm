# GameTerm Scene Mode Packaging And Onboarding Scope

This document scopes Product Layer 10 from the broader Scene Mode product pass:
Packaging And Onboarding.

## Goal

Scene Mode should be usable without remembering a long list of helper commands.

The first pass should document the daily workflow and make the existing helper
surface easier to discover before adding a new umbrella command.

## End Goal

A user can find how to:

- initialize Scene Mode
- generate a workspace scene
- pass pane/process metadata when available
- validate a scene
- run doctor
- install safely
- launch Scene Mode
- run smoke
- recover from invalid generated files

## First-Pass Product Contract

Onboarding should be command-first and recovery-oriented.

Required workflows:

1. Fresh setup.
2. Generate from current repo.
3. Install generated scene with overwrite awareness.
4. Validate and doctor before launch.
5. Launch Scene Mode.
6. Record smoke.
7. Recover from invalid scene or bad generated output.

Avoid adding a new umbrella helper until the remaining workflows are stable.

## Documentation Contract

Docs should include:

- shortest successful path
- safe dry-run path
- strict validation path
- generated-file locations
- how to undo an install
- what commands do not execute automatically
- known warning noise separate from Scene defects

The primary docs should link to detailed layer docs rather than duplicating
every implementation detail.

## Helper Contract

Existing helpers should expose discoverable usage:

- `ci/gameterm-scene-init.sh`
- `ci/gameterm-scene-author.sh`
- `ci/gameterm-scene-workspace.sh`
- `ci/gameterm-scene-doctor.sh`
- `ci/gameterm-scene-verify.sh`
- `ci/gameterm-scene-smoke.sh`
- `ci/gameterm-scene-patch.sh`
- `ci/gameterm-scene-agent.sh`

Potential first-pass addition:

```sh
ci/gameterm-scene-workflow.sh help
```

Only add it if it reduces command discovery burden without hiding validation or
recovery steps.

## Packaging Boundary

Do not package Scene Mode as a separate app in this pass.

Packaging means:

- docs and helper workflow are coherent
- generated assets are included where needed
- smoke/verification commands are findable
- normal user config is not overwritten silently

Distribution packaging, installers, and app bundle changes are deferred.

## Verification

Deterministic verification should cover:

- helper `--help` commands work
- docs reference existing helper names
- onboarding dry-run commands produce valid scenes
- install overwrite protection remains covered
- recovery docs point to validator/doctor
- `ci/gameterm-scene-verify.sh --all`

Manual smoke should follow the documented workflow exactly.

## Implementation Status

Implemented:

- `docs/gameterm-scene-onboarding.md` command-first workflow.
- dry-run, validate, doctor, install, launch, smoke, and recovery steps.
- explicit generated-file location and overwrite behavior.
- explicit statement that discovery does not run commands, start agents, or
  submit prompts.
- verifier coverage that onboarding docs reference the real helper paths and
  recovery commands.

Deferred:

- umbrella workflow helper
- app bundle/menu onboarding
- installer integration
- GUI onboarding wizard

## Commit Lanes

1. `[docs] scope Scene packaging onboarding layer`
2. `[docs] document Scene onboarding workflow`
3. `[visual] add Scene onboarding helper path` only if helper-owned Scene tooling changes
4. `[test] verify Scene onboarding workflow`
5. `[tools] record Scene onboarding smoke`

## Deferred Work

- app bundle/menu entry changes
- installer integration
- generated desktop shortcuts
- GUI onboarding wizard
- bundled example gallery
- release-channel documentation

## Done Definition

The layer is first-pass complete when a user can follow one documented workflow
from setup through smoke and recovery without needing hidden context from the
development thread.
