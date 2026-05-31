# GameTerm Scene Mode Agent And Workspace Scope

This document scopes the next Scene Mode product layer: Agent and Workspace.

The completed first pass proved that Scene Mode can represent state. This layer
defines what that state is for inside GameTerm: making active work visible,
inspectable, and steerable without turning Scene Mode into a generic game
engine or a detached dashboard.

## Purpose

Agent and Workspace mode should make GameTerm feel like a live work environment
with state, not a terminal with a decorative overlay.

The purpose is to connect Scene Mode's visual state model to the things users
already do in GameTerm:

- run commands
- inspect files
- manage projects
- follow task progress
- supervise agents
- understand blocked work
- choose the next action
- move between conversation, workspace, process, and agent contexts

This layer should answer a simple product question:

> What is happening in my workspace, what can I safely do next, and what state
> will change if I do it?

The first pass should not try to replace the terminal, editor, shell, or agent
runtime. It should make their state visible and give Scene Mode a stable
contract for representing them.

## End Goal

The end goal is a first-pass Agent/Workspace Scene Mode where a user can open
GameTerm and see a structured representation of current work.

In the target experience:

1. The workspace appears as a map of live entities.
2. Projects, files, tasks, panes, processes, and agents are represented as
   selectable entities.
3. Selecting an entity shows the active state that matters: phase, status,
   target file, command, owner, blockers, and next actions.
4. Agents can report lifecycle changes through the existing patch transport.
5. Processes can report queued, running, blocked, succeeded, failed, and
   cancelled states.
6. Scene choices can open relevant files, run explicit commands, switch layers,
   or update deterministic state.
7. Guards can enable or block actions based on agent, process, story, and
   workspace variables.
8. The Tile Debugger can explain why a state looks the way it does.
9. Smoke fixtures can prove the loop works without requiring a real external
   agent.
10. A real task-runner path can prove the same contracts work with shell-driven
    state.

The product end state is not "a visual novel inside the terminal." It is
"terminal-native work represented as a stateful world."

## Alignment With GameTerm Goals

This layer aligns with the broader GameTerm direction in five ways.

1. It keeps the terminal first.
   Scene Mode should expose terminal work, not hide it. Commands still run in
   panes, files still open through platform/editor paths, and shell workflows
   remain explicit.

2. It treats interfaces as states.
   The user is not just changing screens. Conversation, workspace, agent,
   process, and memory modes can each carry their own input rules, rendering,
   guards, lifecycle hooks, and available actions.

3. It gives agents a visible operating surface.
   Agent work should not be a hidden stream of text. Planning, running,
   waiting, blocked, failed, and completed phases should be visible and
   inspectable.

4. It uses the Scene Mode primitives already built.
   The layer should compose existing patches, variables, layers, deterministic
   actions, story/RPG state, process state, lifecycle hooks, input maps, and
   smoke fixtures instead of inventing a second model.

5. It creates a path from prototype to product.
   The first pass should remain small enough to test, but concrete enough that
   later work can add richer workspace discovery, agent integration, memory
   graphs, and command palettes without rewriting the foundation.

## Product Model

Agent/Workspace mode is made of overlapping state machines, not one master
screen.

Initial state layers:

- Global mode: workspace, paused, error.
- Workspace mode: idle, inspecting, running task, reviewing result.
- Agent mode: idle, planning, running, waiting, blocked, completed, failed,
  cancelled.
- Process mode: none, queued, running, blocked, succeeded, failed, cancelled.
- Entity focus mode: none, selected, actionable, hidden.
- UI mode: scene, dialogue, debugger, command/action selection.

The active state should determine:

- available inputs
- available choices
- visible status
- selected entity metadata
- guard behavior
- layer transitions
- debug report contents
- safe command execution paths

## Core Entities

The first-pass workspace layer should standardize the entity types that matter
for GameTerm work.

### Workspace

Represents the current working context.

Expected fields:

- id
- label
- root path
- active project
- active branch or revision when available
- current pane/window identity when available
- active scene source
- status summary

First-pass behavior:

- visible as the top-level map anchor
- can expose choices to open docs, run doctor, validate scene, or inspect state
- can receive patch metadata from helper scripts

### Project

Represents a codebase, repo, package, or meaningful unit of work.

Expected fields:

- id
- label
- root path
- repo status summary when known
- current task ids
- important doc links
- verification command hints

First-pass behavior:

- selectable entity
- open-file actions for docs or roadmap files
- run-command actions for explicit verification commands
- metadata-driven status in normal view and debugger

### Task

Represents a unit of work the user or an agent is trying to complete.

Expected fields:

- id
- label
- phase
- owner
- objective
- current step
- blocker
- result summary
- verification status
- related files
- related commands

First-pass behavior:

- receives process/agent lifecycle patches
- drives guards through variables such as `agent_phase` and
  `agent_process_phase`
- can unlock review, verify, or cleanup choices
- can be used by smoke fixtures without requiring external services

### Agent

Represents a worker or assistant acting on a task.

Expected fields:

- id
- label
- phase
- current objective
- current plan step
- waiting reason
- blocked reason
- last update
- result

First-pass behavior:

- updated by `ci/gameterm-scene-agent.sh`
- mapped onto typed process state for rendering and guards
- visible in the Tile Debugger
- drives layer transitions from planning to running to blocked or complete

### Process

Represents a running or recently completed command.

Expected fields:

- id
- label
- command
- cwd
- phase
- exit code
- started/ended metadata when available
- output summary when available

First-pass behavior:

- updated through typed process-state patches
- visible in entity flags and metadata
- can trigger state changes without parsing terminal output in Scene Mode
- can be simulated in fixtures and driven by a real shell helper

### File

Represents a document, source file, fixture, or generated artifact relevant to
the current task.

Expected fields:

- id
- label
- path
- kind
- role
- status
- related task or project

First-pass behavior:

- open-file action target
- selectable entity
- can be marked as planned, edited, verified, generated, stale, or blocked
- can be referenced in task and project metadata

### Memory Or Relationship

Represents context that helps explain why an entity matters.

Expected fields:

- id
- label
- relationship kind
- source entity
- target entity
- strength or status when useful
- note

First-pass behavior:

- optional fixture-level entity
- uses existing relationship/RPG-style state where useful
- should not become a full memory product in this layer

## State Contracts

This layer should prefer explicit state contracts over inferred UI behavior.

### Variables

First-pass variables should include:

- `workspace_mode`
- `selected_workspace_entity`
- `active_task_id`
- `active_agent_id`
- `agent_phase`
- `agent_process_phase`
- `process_phase`
- `review_ready`
- `verification_status`
- `blocker_present`

Variables should remain small and guard-friendly. Large structured data belongs
in entity metadata or typed state records.

### Entity Metadata

Entity metadata should carry display and inspection context.

Recommended keys:

- `path`
- `cwd`
- `command`
- `phase`
- `owner`
- `objective`
- `current_step`
- `blocker`
- `result`
- `verification`
- `related_files`
- `related_commands`

Metadata should be treated as display and integration data, not the only source
of runtime truth. If a value needs to drive guards or layer transitions, mirror
it into typed variables or process state.

### Process State

The existing typed process-state model should remain the source of truth for
process-like lifecycle rendering.

Required phases:

- queued
- running
- blocked
- succeeded
- failed
- cancelled

Agent phase aliases should continue to map into process state where that helps
normal rendering.

### Layer State

Workspace and agent layers should drive the higher-level mode behavior.

Expected first-pass layers:

- `workspace`: `overview`, `inspect`, `task`, `review`
- `agent`: `idle`, `planning`, `running`, `waiting`, `blocked`, `complete`,
  `failed`, `cancelled`
- `process`: `none`, `queued`, `running`, `blocked`, `succeeded`, `failed`,
  `cancelled`
- `ui`: `scene`, `debugger`

Layer transitions should be deterministic and visible when blocked.

## User Workflows

### Workflow 1: Inspect Current Workspace

The user opens Scene Mode from GameTerm.

Expected behavior:

1. The workspace map is visible.
2. The active project is selected by default or available as a nearby entity.
3. The dialogue/status panel summarizes the workspace state.
4. Choices expose safe actions such as open roadmap, run doctor, validate
   scene, or inspect active task.
5. The Tile Debugger shows scene source, layers, variables, selected entity,
   and patch source.

Acceptance:

- The user can understand what workspace is active without reading raw JSON.
- No command runs unless the user explicitly chooses it.

### Workflow 2: Follow Agent Task Progress

An agent or helper emits lifecycle patches.

Expected behavior:

1. Agent starts in `planning`.
2. Scene Mode updates the agent entity and `agent_phase`.
3. Agent moves to `running`.
4. A waiting or blocked phase changes available choices.
5. Completion unlocks review or verification choices.
6. Failure leaves visible recovery actions.

Acceptance:

- A fixture can simulate the whole lifecycle.
- A real shell helper can emit the same patch sequence.
- Guards can react to lifecycle variables.

### Workflow 3: Track A Running Command

A command is started from an explicit Scene choice or external helper.

Expected behavior:

1. The task/process entity enters queued or running state.
2. Status explains which command is active.
3. Success/failure updates entity flags and metadata.
4. Verification choices become available only after success when appropriate.
5. Failure does not corrupt unrelated state.

Acceptance:

- Process state is typed, visible, and testable.
- Command execution remains explicit and target-aware.

### Workflow 4: Open The Right File

The user selects an entity and chooses an open-file action.

Expected behavior:

1. File choices are derived from entity metadata or fixture choices.
2. Scene Mode opens the configured file or document.
3. Missing targets are reported by doctor/smoke checks.
4. Runtime failure is visible in status and debug state.

Acceptance:

- File actions are useful without making Scene Mode an editor.
- The authoring helper can create or validate these targets.

### Workflow 5: Recover From Blocked Work

An agent or process reports a blocker.

Expected behavior:

1. The blocked state is visible on the entity.
2. The blocker reason is visible in normal view.
3. Choices shift toward inspect, open related file, rerun, cancel, or mark
   waiting.
4. Guarded actions that are not allowed explain why they are blocked.

Acceptance:

- Blocked state is not a dead end.
- The user can see the difference between waiting, blocked, failed, and
  cancelled.

## Proposed Deliverables

### 1. Scope And Fixture Design

Deliverables:

- this scope document
- fixture inventory for Agent/Workspace mode
- list of state contracts used by the first shippable slice
- explicit non-goals and deferred work

Acceptance:

- the roadmap can point to one document for the Agent/Workspace layer
- every later commit in this layer can be mapped back to a scoped deliverable

### 2. Workspace Fixture

Deliverables:

- a `workspace-agent` or equivalent fixture
- entities for workspace, project, task, agent, process, and files
- layer setup for workspace/agent/process/UI states
- choices for inspect, open docs, run doctor/verify, and reset/reload

Acceptance:

- fixture loads through the existing authoring helper
- fixture validates through existing scene validation
- fixture can be used by smoke scripts without writing tracked state

### 3. Agent Lifecycle Product Slice

Deliverables:

- documented patch sequence for planning, running, waiting, blocked, complete,
  failed, and cancelled
- guard examples driven by `agent_phase` and process state
- normal-view copy that explains the active phase
- Tile Debugger visibility for the same state

Acceptance:

- lifecycle patches visibly update the selected entity
- blocked and failed states expose recovery choices
- completed state exposes review/verification choices

### 4. Real Task-Runner Path

Deliverables:

- helper or documented script path that emits process/agent patches around a
  real command
- status updates for start, running, success, and failure
- safe temporary patch transport by default
- no hidden autonomous command execution

Acceptance:

- a real shell command can drive Scene Mode process state
- command failures are visible and recoverable
- the same fixture works with simulated and real updates

### 5. Workspace File And Project Actions

Deliverables:

- authoring examples for project/file entities
- open-file choices for docs, roadmap, fixture, or source files
- doctor checks for referenced files and cwd paths
- metadata conventions for related files and commands

Acceptance:

- a user can navigate from task to relevant file
- missing file targets are caught before smoke where possible
- actions stay explicit and inspectable

### 6. Smoke And Verification

Deliverables:

- named smoke scenario for Agent/Workspace
- launch path that captures a final visible lifecycle state
- optional manual checklist for multiple overlays and routing
- docs update in Scene Mode usage docs

Acceptance:

- `ci/gameterm-scene-verify.sh --all` covers static fixture validity
- launch smoke can prove the visual path on macOS
- manual checklist covers cases automation cannot reliably own yet

### 7. Roadmap Integration

Deliverables:

- runtime roadmap links this layer as the next product-loop scope
- first-pass scope distinguishes completed foundation from Agent/Workspace
  product work
- deferred items are named instead of implied

Acceptance:

- status is clear: completed foundation, current product layer, later work
- the next engineer can see what to build first

## First Shippable Slice

The first shippable Agent/Workspace slice should be intentionally narrow.

It should include:

1. One fixture.
2. One workspace entity.
3. One project entity.
4. One task entity.
5. One agent entity.
6. One process entity.
7. Two file entities.
8. Agent lifecycle patch support using the existing helper.
9. At least one guarded choice based on agent/process state.
10. At least one explicit command choice.
11. At least one open-file choice.
12. Debugger visibility for layers, variables, selected entity, process state,
    and patch source.
13. Static verification.
14. One smoke path that captures a meaningful final state.

The slice is complete when a user can watch a task move from planning to
running to completed or blocked, inspect what changed, and choose a relevant
next action.

## Commit Plan

Each item should land as a separate commit.

1. `[docs] scope Scene Agent Workspace layer`
   - Add this scope.
   - Link it from the roadmap.

2. `[visual] add Scene Agent Workspace fixture`
   - Add or generate the first fixture.
   - Use existing schema where possible.

3. `[tools] extend Scene agent workspace helper`
   - Add helper paths only where existing helpers do not cover the product
     slice.
   - Keep mutation rollback guarantees.

4. `[visual] add Scene Agent Workspace guards`
   - Add fixture/runtime coverage for guarded lifecycle choices.
   - Prefer existing variable and layer contracts.

5. `[tools] add Scene Agent Workspace smoke`
   - Add static and launch smoke coverage.
   - Keep temp files outside tracked workspace state.

6. `[docs] document Scene Agent Workspace workflow`
   - Update user-facing Scene docs.
   - Add manual smoke checklist for routing and multiple overlays.

## Verification Gates

Docs-only commits:

- `git diff --check`

Fixture/tool commits:

- `ci/gameterm-scene-author.sh validate <fixture>`
- `ci/gameterm-scene-doctor.sh --scene <fixture> --strict` when applicable
- `ci/gameterm-scene-verify.sh --all`

Runtime commits:

- focused `cargo test -p gameterm-visual <test-filter>`
- `cargo test -p gameterm-visual`
- `cargo check -p gameterm-gui` when GUI transport or overlay behavior changes

Smoke commits:

- `ci/gameterm-scene-smoke.sh --describe-scenario <scenario>`
- `ci/gameterm-scene-smoke.sh --launch --scenario <scenario>` when local GUI
  automation is available

## Non-Goals

This layer should not include:

- a full IDE
- a full game engine
- autonomous hidden agent execution
- implicit command execution from patches
- terminal-output parsing as the primary state source
- broad mux or renderer rewrites
- networked agent orchestration
- persistent memory graph productization
- save-slot UI beyond existing story-state paths
- replacing existing terminal panes or shell workflows

## Deferred Work

Likely follow-up layers:

- workspace discovery from real panes, cwd, git, and project metadata
- richer memory/relationship graph rendering
- command palette integration
- multi-agent coordination views
- per-pane live process introspection
- richer task history and replay
- persisted workspace sessions
- richer visual layout and sprite assets
- user-configurable workspace entity mapping
- policy controls for what commands Scene Mode may offer

## Risks

### Risk: Scene Mode Becomes A Dashboard

Mitigation: keep inputs, guards, lifecycle, and actions stateful. The user
should move through work states, not stare at a passive status page.

### Risk: Agent State Becomes Ad Hoc Metadata

Mitigation: use typed process state and guard-friendly variables for behavior.
Use metadata for inspection only.

### Risk: Too Much Product Surface Lands At Once

Mitigation: ship the first slice around one fixture and one task lifecycle.
Everything else should be deferred unless it is required to prove that loop.

### Risk: Hidden Automation Surprises The User

Mitigation: commands remain explicit choices. Patches can update state but
should not run commands by themselves.

### Risk: Fixtures Drift From Real Work

Mitigation: add one real task-runner path after the fixture is stable. The
fixture proves determinism; the real path proves product relevance.

## Done Definition

The Agent/Workspace first pass is done when:

1. The scope is documented and linked from the roadmap.
2. A workspace-agent fixture exists and validates.
3. The fixture demonstrates workspace, project, task, agent, process, and file
   entities.
4. Agent lifecycle patches drive visible state changes.
5. At least one lifecycle state changes available actions through guards.
6. At least one real command path can drive process state.
7. At least one open-file action connects state to a useful local artifact.
8. Debugger output explains active layers, selected entity, variables, process
   state, and patch source.
9. Static verification covers the fixture.
10. Smoke coverage proves the visible product loop.
11. User-facing docs explain how to run and inspect it.
12. Deferred items are named and do not block the first pass.

At that point, Scene Mode has enough Agent/Workspace structure to support
future visual novel, RPG, memory, and agent-control features without treating
them as unrelated screens.
