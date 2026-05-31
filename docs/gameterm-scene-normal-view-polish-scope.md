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

The first complete layer is done when:

1. Normal view shows the selected entity and a compact metadata summary.
2. Active mode and active layer state are visible.
3. Last known process state is visible when present.
4. Important variables are summarized without taking over the screen.
5. Choices remain visible and selectable in small terminal windows.
6. The Tile Debugger continues to expose full detail.
7. Deterministic tests cover the normal-view summary.

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

The first pass keeps the existing text renderer and adds compact summaries:

- selected entity metadata: first four key/value pairs plus overflow count
- variables: first five entries plus overflow count
- layers: first four layer states plus overflow count
- process state: phase, entity id, command, exit code, and message when present

Long metadata, variable, and process values are clipped before rendering so one
large path or message cannot crowd out choices.

## Deferred Work

Deferred items:

- richer two-column layout
- terminal-size-specific layout planner
- grouped command palette rendering
- color/style treatment in the GUI renderer
- screenshots for several terminal sizes

These belong after the first product summary proves useful and stable.

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
- focused unit coverage for the product-state summary

Deferred:

- layout planner
- command palette view
- multi-size screenshot assertions
