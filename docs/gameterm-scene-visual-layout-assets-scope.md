# GameTerm Scene Mode Visual Layout And Assets Scope

This document scopes Product Layer 9 from the broader Scene Mode product pass:
Visual Layout And Assets.

## Goal

Generated Scene Mode workspaces should be easier to scan and audit visually.

The first pass should improve deterministic placement and role clarity without
turning Scene Mode into a full layout engine or asset pipeline.

## End Goal

A generated workspace scene should:

- avoid overlapping important entities
- place related entity groups predictably
- visually distinguish workspace, files, tasks, agents, panes, processes, and
  relationships
- remain readable in small and large terminal windows
- use sprite ids consistently

## First-Pass Layout Contract

Use deterministic zones on the existing grid:

- workspace/project anchors near top-left
- pane/process context near top-right
- tasks/agents in center rows
- files/docs along lower rows
- relationship/memory entities near related files or lower side rows

Rules:

- no two visible entities should occupy the same position unless explicitly
  allowed
- generated positions must be deterministic for the same input
- overflow should degrade predictably
- small scenes should prefer fewer entities over unreadable placement

## Asset Contract

The first pass should reuse existing bundled sprite ids where possible:

- `workspace-map`
- `project_core`
- `task_tile`
- `agent_idle`
- `memory_note`

Add new sprite ids only when a role cannot be distinguished clearly with the
existing set. Any new sprite id must be covered by:

- bundled asset or placeholder-safe fallback
- sprite manifest example when applicable
- doctor coverage for missing manifest entries

## Helper Contract

Workspace Discovery should own generated placement rules.

Potential helper changes:

- layout function for generated entities
- deterministic file-row wrapping
- pane/process zone placement
- optional max entity cap by role
- collision check before writing scene

## Runtime Impact

Avoid runtime layout changes in the first pass. Runtime should render the scene
positions it receives. Generated layout belongs to helper/authoring code unless
manual scenes require validation improvements.

## Verification

Deterministic verification should cover:

- generated scene has no overlapping visible entity positions
- generated workspace/pane/process/file entities land in expected zones
- sprite ids are valid or covered by manifest/fallback
- small max-file configurations remain valid
- `ci/gameterm-scene-verify.sh --all`

Live smoke should capture at least one generated workspace scene after layout
rules change.

## Implementation Status

Implemented:

- Workspace Discovery validates generated layout before writing scene output.
- visible generated entities must have unique positions.
- generated entity positions must be inside scene bounds.
- verifier coverage for workspace/project/pane/process zones.
- verifier coverage that generated file entities stay in lower rows.
- existing bundled sprite ids remain the first-pass asset set:
  `workspace-map`, `project_core`, `task_tile`, `agent_idle`, and
  `memory_note`.

Deferred:

- new sprite assets
- graph layout algorithms
- terminal-size adaptive placement
- live screenshot smoke refresh
- drag/reposition UI

## Commit Lanes

1. `[docs] scope Scene visual layout assets layer`
2. `[visual] add Scene generated layout rules`
3. `[test] verify Scene generated layout`
4. `[docs] document Scene layout workflow`
5. `[tools] record Scene layout smoke`

## Deferred Work

- graph layout algorithms
- adaptive terminal-size layout planner
- custom generated raster assets
- theme-aware sprites
- drag/reposition UI
- animation

## Done Definition

The layer is first-pass complete when generated workspace scenes use
deterministic non-overlapping placement, role-specific sprite ids, verification
coverage, and a recorded smoke screenshot.
