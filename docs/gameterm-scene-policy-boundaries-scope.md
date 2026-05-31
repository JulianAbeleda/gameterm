# GameTerm Scene Mode Policy And Permission Boundaries Scope

This document scopes Product Layer 4 from the broader Scene Mode product pass:
Policy And Permission Boundaries.

## Goal

Scene Mode should make generated and agent-proposed actions auditable before a
user runs them.

The first pass keeps execution behavior unchanged. It adds policy diagnostics
around command shape so unsafe or incomplete generated actions are visible
before live use.

## Product End State

The first complete layer is done when:

1. `RunCommand` choices still require explicit activation.
2. `OpenFile` choices remain non-shell actions.
3. Generated `RunCommand` choices include an explicit cwd.
4. `doctor` warns when a `RunCommand` choice has no cwd.
5. `doctor` continues to validate target and argv shape.
6. User-facing docs explain that discovery does not run commands.
7. Deterministic verification covers policy diagnostics.

## First-Pass Policy Model

The first pass treats these fields as the minimum command policy surface:

- explicit argv array
- explicit target
- explicit cwd for generated or reusable commands

Why cwd matters:

- it makes command intent auditable
- it prevents a generated command from depending on an ambiguous launch cwd
- it gives future policy checks a stable workspace boundary

## Current Runtime Boundary

Scene Mode does not run discovery commands automatically.

`RunCommand` only becomes executable after:

1. A scene author or generator writes an explicit choice.
2. The user selects that choice.
3. The user activates the choice.
4. The GUI dispatch path opens the command in the requested pane/tab target.

This layer does not introduce allowlists, prompts, or automatic command
approval. Those belong in a later policy iteration.

## Deferred Work

Deferred items:

- explicit origin field in the scene schema
- risk labels or badges
- command allowlists
- agent-proposed command review UI
- workspace-bound command policy
- policy-aware command palette filtering

These should build on the current doctor diagnostics and grouped command
surface.

## Verification

Deterministic verification should cover:

- valid `RunCommand` target diagnostics
- explicit argv diagnostics
- missing cwd warning
- generated workspace commands include cwd
- `doctor --strict` still fails when warnings are present

Live smoke should continue to cover actual `RunCommand` dispatch separately
from policy diagnostics.

## Done Status

Implemented:

- `doctor` warning for `RunCommand` choices without cwd
- default fixture updated to include cwd on its command choice
- verifier coverage for missing-cwd policy diagnostics

Deferred:

- schema-level action origin
- risk metadata
- allowlist enforcement
