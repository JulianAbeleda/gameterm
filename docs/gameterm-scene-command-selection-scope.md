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

The first complete layer is done when:

1. Existing choices still work.
2. Normal view groups choices by action kind.
3. Generated workspace actions can be scanned by category.
4. Selected choice activation behavior is unchanged.
5. RunCommand and OpenFile remain explicit.
6. Deterministic tests cover grouped choice rendering.

## First-Pass Runtime Shape

The first pass does not add a new schema field. It groups the existing choice
list by action kind at render time:

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
