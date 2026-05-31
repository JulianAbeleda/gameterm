# GameTerm Scene Mode Normal View Polish Scope

This document scopes Product Layer 2 from the broader Scene Mode product pass:
Normal View Product Polish.

## Goal

Scene Mode should communicate the common workspace state without requiring the
Tile Debugger.

The debugger remains the complete inspection surface. The normal view should
show enough of the selected entity, active mode, layers, process state, and
available actions that a generated workspace scene is understandable during
daily use.

## Product End State

Status: complete for the first product-summary pass.

The first complete layer is done because normal view now:

1. Shows the selected entity and a compact metadata summary.
2. Shows active mode and active layer state.
3. Shows last known process state when present.
4. Summarizes important variables without taking over the screen.
5. Keeps choices visible and selectable in ordinary terminal windows.
6. Leaves full detail in the Tile Debugger.
7. Has deterministic test coverage for the product-state summary.

## Information Hierarchy

Normal view should prioritize:

1. Scene title and controls.
2. Map/entity grid.
3. Selected entity identity.
4. Selected entity details.
5. Active mode and layers.
6. Process summary.
7. Compact state summary.
8. Dialogue.
9. Choices.
10. Status.

The normal view should not duplicate the full debugger. It should summarize
state and leave exhaustive metadata, variables, transition history, and action
diagnostics to the Tile Debugger.

## First-Pass Runtime Shape

The implemented first pass keeps the existing text renderer and adds compact
summaries:

- selected entity metadata: first four key/value pairs plus overflow count
- variables: first five entries plus overflow count
- layers: first four layer states plus overflow count
- process state: phase, entity id, command, exit code, and message when present
- relationship summary for the selected entity
- grouped choices by action kind

Long metadata, variable, and process values are clipped before rendering so one
large path or message cannot crowd out choices.

## Implemented Evidence

Runtime behavior:

- `render_scene` prints selected entity details through compact metadata.
- `render_scene` prints selected-entity relationships when present.
- `render_scene` prints mode, layer, process, variable, RPG, and story-state
  summaries before dialogue and choices.
- Choices are grouped by action kind in the normal text frame.
- Full metadata, variables, transitions, patches, and diagnostics remain in the
  Tile Debugger.

Focused tests:

- `scene_frame_contains_selected_entity`
- `scene_frame_contains_product_state_summary`
- `scene_frame_groups_choices_by_action_kind`

Verification:

- `cargo test -p gameterm-visual`
- `ci/gameterm-scene-verify.sh --all`

## Second-Pass Scope

The next normal-view pass should be treated as product design work, not a small
follow-up to the first pass.

Candidate goals:

- define a stable compact layout for generated workspace scenes
- decide whether normal view needs a two-column text layout or just better
  section ordering
- make long paths and process messages predictable across small terminal sizes
- decide which relationship/memory/agent facts belong in normal view versus
  command selection or Tile Debugger
- add screenshot smoke only for layout behavior that deterministic text tests
  cannot cover

Deferred items:

- richer two-column layout
- terminal-size-specific layout planner
- grouped command palette rendering
- color/style treatment in the GUI renderer
- screenshots for several terminal sizes

These belong after command selection and policy boundaries are clearer, because
those layers define which actions and safety signals the normal view must carry.

## Verification

Deterministic verification should cover:

- selected metadata appears in normal view
- active layers appear in normal view
- process state appears in normal view
- state summary remains compact
- choices still render

Live smoke should be used when layout changes affect GUI rendering dimensions or
terminal-size behavior.

## Done Status

Implemented:

- compact selected entity metadata in normal view
- active layer summary in normal view
- process state summary in normal view
- compact variable summary in normal view
- selected-entity relationship summary in normal view
- choice grouping by action kind
- focused unit coverage for the product-state summary

Deferred:

- layout planner
- command palette view
- multi-size screenshot assertions

Recommendation: treat Product Layer 2 as complete for the current first pass.
Move next product work to command selection unless a manual smoke pass shows a
specific normal-view readability defect.
