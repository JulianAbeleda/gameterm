# GameTerm Scene Mode Product Completion Scope

This document scopes the broader Scene Mode product direction above the
individual layer scopes.

The first shippable Scene Mode pass is complete. Agent/Workspace and Workspace
Discovery have also reached first-pass implementation. This document defines
what remains to turn Scene Mode from a proven visual state surface into a
coherent daily GameTerm product surface.

## Product Goal

Scene Mode should become a terminal-native workspace surface where the user can
see, inspect, and steer live work.

It should let the user answer:

- What workspace am I in?
- What files, tasks, agents, panes, and processes matter right now?
- What state is active?
- What actions are available?
- Why is an action blocked?
- What changed after a command, agent update, or patch?
- What should I inspect or do next?

The product is not a replacement for the shell, editor, mux, or agent runtime.
It is a stateful visual layer over them.

## Current Product State

Completed first-pass layers:

- Scene runtime foundation.
- Scene schema, validation, fixtures, authoring helpers, doctor checks, and
  deterministic verifier.
- Scene rendering, sprite fallback, Tile Debugger, and live smoke discipline.
- Dialogue, variables, guarded choices, layered state, lifecycle hooks, input
  maps, deterministic actions, lightweight RPG state, and story-state
  persistence.
- Patch inbox and mux patch transport.
- Process and agent lifecycle patches.
- Agent/Workspace authored product slice.
- Workspace Discovery from cwd, git, important files, generated scenes, and
  generated patches.

Known remaining product gap:

- Scene Mode can represent live work, but it does not yet have enough direct
  live GameTerm context to feel like a daily operational surface.

## Product North Star

The first complete product pass is done when a user can:

1. Open Scene Mode from a normal GameTerm session.
2. See the current workspace without hand-authoring a scene.
3. See active pane/process context where available.
4. See current or recent agent/task state.
5. Inspect important files, docs, tasks, commands, and blockers.
6. Run explicit safe actions from Scene Mode.
7. Save or restore useful workspace state.
8. Understand state through a normal view, not only through the Tile Debugger.
9. Verify the feature through deterministic checks and live smoke.
10. Use it repeatedly without generated files, stale binaries, or config churn
    becoming part of normal operation.

## Product Layers

### Layer 1: Pane And Process Discovery

Purpose: connect Workspace Discovery to the active GameTerm session.

End state:

- Scene Mode can include the active pane id, mux window id, pane cwd, foreground
  process name/path when available, and process progress state.
- This data appears as metadata, variables, or process state depending on how
  it is used.
- Discovery can operate from the active pane rather than only from a supplied
  `--cwd`.

Why it matters:

- Workspace Discovery currently knows the repo.
- Pane/process discovery tells Scene Mode what GameTerm is doing right now.

First-pass deliverables:

- Scope document for pane/process discovery:
  `docs/gameterm-scene-pane-process-discovery-scope.md`.
- Add helper support for optional pane metadata input.
- Add mux/CLI path only if local APIs are stable enough.
- Generate/update process entities from pane/process data.
- Add deterministic tests with fixture metadata.
- Add live smoke where mux context is available.

Acceptance criteria:

- Missing pane/process metadata does not break workspace discovery.
- When pane metadata is available, it is visible in the generated scene or
  patch.
- Process state remains explicit and does not parse arbitrary terminal output.

### Layer 2: Normal View Product Polish

Purpose: make Scene Mode understandable without opening the Tile Debugger.

End state:

- Normal view clearly separates map, selected entity, status, active layers,
  blockers, and choices.
- Important state is visible before the user needs debug mode.
- Long state lines do not crowd out choices.

Why it matters:

- Current normal view is functional and smokeable, but dense.
- The debugger explains everything; the product view should explain the
  common path.

First-pass deliverables:

- Scope normal-view information hierarchy:
  `docs/gameterm-scene-normal-view-polish-scope.md`.
- Add compact selected-entity metadata rendering.
- Add concise agent/process/status panel.
- Add truncation/wrapping rules that preserve choices.
- Add screenshot/liveness smoke checks for generated workspace scenes.

Acceptance criteria:

- A generated workspace scene fits useful information in a small terminal
  window.
- Agent/process status is visible without debug mode.
- Choices remain readable and selectable.

### Layer 3: Command Palette And Action Selection

Purpose: provide a better command surface than a long flat choices list.

End state:

- Scene Mode can expose actions as grouped command options.
- Actions can be filtered by selected entity or active layer.
- RunCommand and OpenFile remain explicit.

Why it matters:

- Generated workspaces can produce more actions than a simple list handles.
- A command palette makes Scene Mode usable as a repeated tool.

First-pass deliverables:

- Scope command/action model:
  `docs/gameterm-scene-command-selection-scope.md`.
- Reuse existing choices where possible.
- Add action grouping metadata or command palette mode.
- Add keyboard flow and smoke scenario.

Acceptance criteria:

- Existing choices still work.
- Generated discovery actions can be grouped by file/project/process.
- No command runs without explicit activation.

### Layer 4: Policy And Permission Boundaries

Purpose: make generated and agent-proposed actions safe and predictable.

End state:

- Generated commands carry a clear origin and policy.
- Users can distinguish safe/open-file actions from shell commands.
- Future agent-proposed commands can be allowed, blocked, or inspected before
  running.

Why it matters:

- Scene Mode is moving toward generated and agent-driven workflows.
- Explicit actions are good, but policy makes them auditable.

First-pass deliverables:

- Scope command policy model:
  `docs/gameterm-scene-policy-boundaries-scope.md`.
- Add metadata conventions for action origin and risk.
- Add doctor/verifier warnings for generated commands without cwd/origin.
- Add docs explaining command safety.

Acceptance criteria:

- Discovery-generated commands are tagged.
- Doctor can flag unsafe or incomplete command metadata.
- User-facing docs state that discovery does not run commands.

### Layer 5: Persisted Workspace Sessions

Purpose: preserve useful workspace state across launches without making source
scene files the only storage.

End state:

- User can save and restore discovered workspace state, selected entity, recent
  task/agent/process state, and useful context.
- Persistence is explicit and inspectable.

Why it matters:

- Story-state persistence exists, but daily workspace use needs a product
  concept of session state.
- Generated scenes should not become the only durable record.

First-pass deliverables:

- Scope workspace session state:
  `docs/gameterm-scene-workspace-sessions-scope.md`.
- Decide whether to reuse story-state export/import or add a workspace-session
  helper.
- Add save/load status in normal view and debugger.
- Add verifier coverage for no-corruption restore.

Acceptance criteria:

- Save/load does not mutate source scene unexpectedly.
- Restore failure is visible and does not corrupt active runtime state.
- User can recover a previous workspace state.

### Layer 6: Memory And Relationship Graph

Purpose: make context relationships visible without turning Scene Mode into a
general knowledge graph product too early.

End state:

- Scene Mode can show relationships between files, tasks, agents, docs, and
  memories.
- Relationships can explain why a file or task matters.

Why it matters:

- Workspace Discovery finds entities.
- Memory/relationship view explains connections between entities.

First-pass deliverables:

- Scope relationship graph product slice:
  `docs/gameterm-scene-memory-relationships-scope.md`.
- Define relationship entity/metadata conventions.
- Add fixture and generated examples.
- Keep search/recall out of first pass unless a local source exists.

Acceptance criteria:

- A user can inspect why one entity is related to another.
- Relationship data remains local and explicit.
- No background memory indexing is required.

### Layer 7: Multi-Agent Coordination

Purpose: represent multiple agents or task actors working in the same
workspace.

End state:

- Scene Mode can show more than one agent/task lifecycle.
- Blockers, waiting states, ownership, and completed work are visible per
  agent/task.

Why it matters:

- Current agent lifecycle support is single-entity friendly.
- Real workflows may involve user, assistant, scripts, and background tasks.

First-pass deliverables:

- Scope multi-agent product model:
  `docs/gameterm-scene-multi-agent-coordination-scope.md`.
- Extend helper conventions for agent ids and task ids.
- Add fixture with two agents and two tasks.
- Add guard examples for blocked/review states.

Acceptance criteria:

- Multiple agent patches do not overwrite each other accidentally.
- Selected entity state determines visible action context.
- Smoke covers at least two independent agent lifecycles.

### Layer 8: Agent Task Bootstrap

Purpose: let discovered workspace state become an explicit task setup for an
agent without hidden autonomy.

End state:

- Discovery can produce a task brief from workspace metadata.
- The user can inspect the brief and explicitly start or hand off work.

Why it matters:

- Workspace Discovery tells us where we are.
- Agent bootstrap turns that into structured work only when the user chooses.

First-pass deliverables:

- Scope task brief schema:
  `docs/gameterm-scene-agent-task-bootstrap-scope.md`.
- Generate task entity metadata from workspace discovery.
- Add explicit action to export/copy/open task brief.
- Do not start agents automatically.

Acceptance criteria:

- Generated task brief is inspectable.
- Starting work remains explicit.
- No hidden network or agent process is launched.

### Layer 9: Visual Layout And Assets

Purpose: improve readability and visual identity without breaking terminal
first constraints.

End state:

- Generated scenes lay out entities predictably.
- Workspace, files, tasks, agents, processes, and relationships have clear
  visual roles.
- Sprite assets support the product model.

Why it matters:

- Current layouts are functional but basic.
- Better layout makes the normal view more useful and screenshots easier to
  audit.

First-pass deliverables:

- Scope layout rules:
  `docs/gameterm-scene-visual-layout-assets-scope.md`.
- Improve generated entity placement.
- Add or refine sprite assets only where they clarify state.
- Add screenshot smoke expectations.

Acceptance criteria:

- Generated scenes avoid overlapping important entities.
- Small and large terminal sizes remain readable.
- Visual state supports scanning.

### Layer 10: Packaging And Onboarding

Purpose: make Scene Mode usable without remembering every helper command.

End state:

- User can discover how to initialize, generate, install, validate, smoke, and
  recover Scene Mode.
- Common commands are documented and possibly surfaced in GameTerm actions.

Why it matters:

- The feature is powerful but helper-heavy.
- Daily use requires predictable entry points.

First-pass deliverables:

- Scope onboarding flow:
  `docs/gameterm-scene-packaging-onboarding-scope.md`.
- Add concise docs for daily workflows.
- Add `doctor` suggestions for discovery/session/policy issues.
- Consider a single umbrella helper command after flows stabilize.

Acceptance criteria:

- New user can install a generated workspace scene from docs alone.
- Recovery path is documented for invalid generated scene.
- Smoke and verification commands are easy to find.

## Priority Order

Priority should follow product dependency, not feature excitement.

1. Pane And Process Discovery.
2. Normal View Product Polish.
3. Command Palette And Action Selection.
4. Policy And Permission Boundaries.
5. Persisted Workspace Sessions.
6. Memory And Relationship Graph.
7. Multi-Agent Coordination.
8. Agent Task Bootstrap.
9. Visual Layout And Assets.
10. Packaging And Onboarding.

Rationale:

- Live context comes before richer interaction.
- Readability comes before more actions.
- Policy comes before agent-proposed command workflows.
- Persistence comes before long-running multi-agent/session behavior.
- Memory and multi-agent features should build on stable entities, actions,
  policy, and persistence.

## First Usable Product Pass

The first usable product pass should include only the minimum set that makes
Scene Mode useful repeatedly:

1. Workspace Discovery.
2. Pane/process metadata when available.
3. Normal view polish for selected entity, status, and choices.
4. Explicit grouped actions or a small command palette.
5. Command origin/safety metadata.
6. Workspace session save/restore.
7. Deterministic verification.
8. Live smoke.
9. User-facing docs.

Memory graph, multi-agent coordination, agent bootstrap, and richer visuals can
start after this pass unless one of them becomes necessary to make the core
workflow usable.

## Cross-Cutting Rules

### Safety

- Discovery and patches must not run commands.
- Generated commands must require explicit user activation.
- Agent bootstrap must not start agents automatically.
- Failed validation must not mutate existing config or scene files.

### State

- Small guard-driving values belong in variables.
- Inspectable context belongs in metadata.
- Process lifecycle belongs in typed process state.
- Durable user state should not silently overwrite source scene JSON.

### Verification

- Every layer needs deterministic verification.
- Live smoke is required for GUI or mux behavior.
- Screenshot artifacts should be recorded when product behavior is visual.
- Known warning noise must remain separate from Scene Mode defects.

### Scope Control

- Prefer one product slice per layer.
- Do not add broad engine abstractions before the product need is proven.
- Do not make generated output depend on background services.
- Keep helpers local and inspectable.

## Umbrella Commit Lanes

Each broad layer should land in separate commits:

1. `[docs] scope Scene <layer> layer`
2. `[visual] add Scene <layer> helper path`
3. `[visual] add Scene <layer> runtime/schema support` when needed
4. `[test] verify Scene <layer>`
5. `[docs] document Scene <layer> workflow`
6. `[test] record Scene <layer> smoke` when live smoke is run

Not every layer needs every commit type. Docs-only layers should stay docs-only
until implementation starts.

## Broad Done Definition

The broader Scene Mode product pass is complete when:

1. Scene Mode opens reliably from the app.
2. A generated workspace scene can be created from the current session.
3. Pane/process context appears when available.
4. Normal view communicates selected entity, state, blockers, and actions.
5. Actions are explicit, grouped, and policy-tagged.
6. Workspace session state can be saved/restored.
7. Memory/relationship context has a small working slice.
8. Multi-agent state has a small working slice.
9. Agent task bootstrap is explicit and inspectable.
10. Visual layout is readable enough for daily use.
11. Onboarding docs explain setup, generation, validation, smoke, and recovery.
12. `ci/gameterm-scene-verify.sh --all` passes.
13. Required live smoke scenarios pass and are recorded.

## Current Recommendation

The next scope document should be:

```text
docs/gameterm-scene-pane-process-discovery-scope.md
```

Status: implemented as the explicit-metadata first pass. Live mux
auto-discovery remains deferred until the caller/API path is stable.

That scope should answer:

- which pane/process data is available now
- how to pass it into workspace discovery
- what should be metadata vs variables vs typed process state
- what can be verified deterministically
- what requires live mux smoke

This is the next product dependency because it connects generated workspace
state to the actual active GameTerm session.
