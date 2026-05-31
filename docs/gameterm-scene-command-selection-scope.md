# GameTerm Scene Mode Command Selection Scope

This document scopes Product Layer 3 from the broader Scene Mode product pass:
Command Palette And Action Selection.

## Goal

Scene Mode should expose available actions as an understandable command surface
instead of only a long flat list.

The first pass should reuse existing choices. `OpenFile`, `RunCommand`,
`Navigate`, `Inspect`, and deterministic state actions remain explicit user
choices. No command should run from grouping, filtering, or display alone.

## Product End State

Status: complete for the first grouped-choice pass.

The first complete layer is done because:

1. Existing choices still work.
2. Normal view groups choices by action kind.
3. Generated workspace actions can be scanned by category.
4. Selected choice activation behavior is unchanged.
5. RunCommand and OpenFile remain explicit.
6. Deterministic tests cover grouped choice rendering.

## First-Pass Runtime Shape

The implemented first pass does not add a new schema field. It groups the
existing choice list by action kind at render time:

- `Inspect`
- `OpenFile`
- `RunCommand`
- `Navigate`
- `ExportStoryState`
- `ImportStoryState`
- `AdvanceDialogue`
- `Resolve`

This keeps authored and generated scene JSON compatible while improving the
normal view immediately.

## Implemented Evidence

Runtime behavior:

- normal view emits action-kind headings such as `Inspect`, `OpenFile`, and
  `RunCommand`
- selected choice markers stay attached to the original choice index
- activation still uses the existing selected-choice flow
- command execution remains explicit and user-activated

Focused tests:

- `scene_frame_groups_choices_by_action_kind`

Verification:

- `cargo test -p gameterm-visual scene_frame`
- `ci/gameterm-scene-verify.sh --all`

## Second-Pass Scope

The next command-selection pass is real product work, not a small rendering
cleanup.

Candidate goals:

- add a modal command-selection mode or overlay
- support text filtering/search
- support selected-entity scoped action filtering
- add explicit action group/origin metadata if policy needs it
- display policy/risk badges once the policy layer defines them
- preserve existing choice activation and schema compatibility unless a
  migration is explicitly scoped

## Deferred Work

Deferred items:

- true modal command palette mode
- text filtering
- selected-entity scoped filtering
- explicit action group metadata
- policy/risk badges
- command search

These should come after the policy layer defines safe display language for
generated and agent-proposed commands.

## Verification

Deterministic verification should cover:

- grouped headings appear in normal view
- selected choice markers remain attached to the actual choice
- existing activation behavior is unchanged
- existing authoring and workspace-generated choices still validate

Live smoke should be used when this becomes a separate overlay or changes input
ownership.

## Done Status

Implemented:

- normal-view grouping of choices by action kind
- focused unit coverage for grouped choice rendering

Deferred:

- modal command palette
- filtering
- group metadata

Recommendation: treat Product Layer 3 as complete for the current first pass.
The next meaningful work is a combined command-selection and policy second pass
that defines action origin, risk display, and filtering together.
That combined work is scoped in
[GameTerm Scene Command Policy Second-Pass Scope](gameterm-scene-command-policy-second-pass-scope.md).
