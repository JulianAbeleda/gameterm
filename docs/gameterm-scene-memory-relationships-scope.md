# GameTerm Scene Mode Memory And Relationship Graph Scope

This document scopes Product Layer 6 from the broader Scene Mode product pass:
Memory And Relationship Graph.

## Goal

Scene Mode should explain why workspace entities matter to each other without
becoming a general knowledge graph product.

Workspace Discovery can find files, tasks, projects, agents, and processes. The
relationship layer gives those entities local, explicit connections that the
user can inspect.

## End Goal

A user can select a file, task, agent, doc, or memory and see:

- related entities
- relationship type
- short reason
- optional confidence or weight
- local source of the relationship

No background indexing or hidden recall is required for the first pass.

## First-Pass Product Contract

Relationships should be explicit data authored by fixtures, generated from
known local metadata, or supplied by helper input.

Relationship examples:

- task `uses` file
- doc `explains` task
- agent `owns` task
- process `verifies` project
- memory `references` doc
- file `belongs_to` project

The first pass should keep relationships local and inspectable.

## Data Contract

Use the existing RPG relationship structure where it fits:

```json
{
  "source_id": "scene-agent",
  "target_id": "scene-verify-process",
  "kind": "monitors",
  "value": 1,
  "metadata": [["reason", "agent waits for verification"]]
}
```

Rules:

- `source_id` and `target_id` must reference scene entities.
- `kind` must be non-empty and human-readable.
- `value` is optional weight, not permission.
- metadata carries reason/source details.
- generated relationships must be deterministic.

Do not add a new graph schema unless existing relationship records cannot
support the product view.

## Rendering Contract

Normal view should show a compact relationship summary for the selected entity:

- incoming count
- outgoing count
- first few related labels/kinds

Tile Debugger should show full relationship rows:

- source label/id
- target label/id
- kind
- value
- metadata

## Helper Contract

Workspace Discovery can generate first-pass relationships from known entities:

- workspace -> project
- project -> important files
- task -> related files
- process -> project
- pane -> process when pane metadata exists

No file-content semantic search is required.

## Verification

Deterministic verification should cover:

- fixtures validate relationship references
- generated workspace scenes contain expected relationships
- normal view shows selected relationship summary
- Tile Debugger shows full relationship detail
- missing entity references are rejected or diagnosed
- `ci/gameterm-scene-verify.sh --all`

## Implementation Status

Implemented:

- relationship endpoint validation against scene entities.
- compact selected-entity relationship summary in normal view.
- full relationship rows in the Tile Debugger.
- deterministic Workspace Discovery relationship generation for:
  - workspace -> project
  - project -> files
  - task -> files
  - process -> project
  - pane -> process when pane metadata is provided
- verifier coverage for generated relationship rows.
- unit coverage for rendering and missing endpoint validation.

Deferred:

- graph layout mode
- relationship editing UI
- semantic search or background indexing
- relationship updates through scene patches
- relationship persistence outside existing scene/story/session JSON

## Commit Lanes

1. `[docs] scope Scene memory relationships layer`
2. `[visual] add Scene relationship display support`
3. `[visual] add Scene relationship generation` if implemented in helper-owned Scene tooling
4. `[test] verify Scene memory relationships`
5. `[docs] document Scene relationship workflow`
6. `[test] record Scene relationship smoke` when live smoke is run

## Deferred Work

- background indexing
- vector search
- LLM-generated relationship inference
- graph layout engine
- relationship editing UI
- relationship persistence separate from scene/session files

## Done Definition

The layer is first-pass complete when relationships between existing workspace
entities are local, explicit, validated, visible in normal view, and inspectable
in the Tile Debugger.
