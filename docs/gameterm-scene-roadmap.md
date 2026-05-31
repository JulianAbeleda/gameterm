# GameTerm Scene Mode Roadmap

This is the consolidated roadmap for Scene Mode. It points to the detailed
scope documents, but this file is the first place to check for current product
status and next priorities.

## Current Status

Status: first shippable Scene Mode pass complete.

Scene Mode currently has:

- a stable scene runtime with bundled fallback and user scene loading
- schema validation, fixtures, authoring helpers, doctor checks, and verifier
- normal rendering, sprite fallback, Tile Debugger, and live smoke discipline
- dialogue, variables, guarded choices, layers, lifecycle hooks, input maps,
  deterministic state actions, lightweight RPG state, and story-state
  persistence
- patch inbox and mux patch transports
- process and agent lifecycle patch helpers
- Agent/Workspace authored product slice
- Workspace Discovery from cwd, git, important files, generated scenes, and
  generated patches
- stabilization/refactor pass covering commit discipline, verifier structure,
  helper overwrite checks, enter-lifecycle cleanup, and live smoke audit

Latest verification baseline:

- `ci/gameterm-scene-verify.sh --all`: PASS
- `cargo test -p gameterm-visual`: PASS
- `cargo build -p gameterm-gui`: PASS with pre-existing macOS warning noise
- Live smoke: PASS for `workspace-discovery`, recorded in
  [GameTerm Scene Mode Smoke Report](gameterm-scene-smoke-report.md)

Known non-blocking warning noise remains outside Scene Mode scope:

- existing macOS `objc` macro `unexpected cfg` warnings
- existing `gameterm-toast-notification` unnecessary `unsafe` warnings
- existing `screen_line.rs` unused assignment warning

## Document Map

Use this file for roadmap status. Use the detailed docs below for scoped
implementation details.

| Area | Owner Doc | Status |
| --- | --- | --- |
| First shippable Scene Mode pass | [First-Pass Scope](gameterm-scene-first-pass-scope.md) | Complete |
| Runtime history and lower-level feature roadmap | [Runtime Roadmap](gameterm-scene-runtime-roadmap.md) | Mostly historical; keep for design context |
| Broad product completion stack | [Product Completion Scope](gameterm-scene-product-completion-scope.md) | Active planning source |
| Live pane/process context | [Pane And Process Discovery Scope](gameterm-scene-pane-process-discovery-scope.md) | Implemented through explicit metadata |
| Agent/Workspace authored model | [Agent And Workspace Scope](gameterm-scene-agent-workspace-scope.md) | First-pass implemented |
| Workspace Discovery | [Workspace Discovery Scope](gameterm-scene-workspace-discovery-scope.md) | First-pass implemented |
| Stabilization/refactor pass | [Stabilization Refactor Scope](gameterm-scene-stabilization-refactor-scope.md) | First pass complete |
| Refactor backlog | [Refactor Plan](gameterm-scene-refactor-plan.md) | Active backlog, do only scoped lanes |
| Smoke checklist | [Product Smoke Checklist](gameterm-scene-product-smoke.md) | Current smoke procedure |
| Smoke results | [Smoke Report](gameterm-scene-smoke-report.md) | Current live/deterministic evidence |
| Onboarding | [Onboarding](gameterm-scene-onboarding.md) | Current user workflow |

## Product Direction

Scene Mode should become a terminal-native workspace surface where the user can
see, inspect, and steer live work.

It should answer:

- What workspace am I in?
- What panes, processes, files, tasks, agents, and state matter right now?
- What actions are available?
- Why is an action blocked?
- What changed after a command, patch, or agent update?
- What should I inspect or do next?

Scene Mode is not a replacement for the shell, editor, mux, or agent runtime.
It is a stateful visual layer over them.

## Priority Stack

### Priority 1: Live Pane And Process Context

Status: complete for explicit metadata input; live mux auto-discovery remains
deferred.

Goal: connect Workspace Discovery to the active GameTerm session.

Why it mattered: Workspace Discovery knew the repo, but not enough about the
current pane/process to feel like a live operational surface.

Scope owner:

- [Pane And Process Discovery Scope](gameterm-scene-pane-process-discovery-scope.md)

Completed first slice:

- accept optional pane metadata input in the workspace helper
- represent pane cwd, mux window id, pane id, foreground process, and process
  phase as explicit Scene state when available
- keep missing metadata non-fatal
- verify with deterministic fixture metadata before live mux smoke

Verified behavior:

- `ci/gameterm-scene-workspace.sh inspect`, `discover`, and `patch` accept
  explicit pane/process metadata.
- `--pane-cwd` becomes the discovery cwd when `--cwd` is absent.
- Generated scenes include `discovered-pane`, `discovered-process`,
  `pane_context`, `active_pane_id`, `active_mux_window_id`, and
  `process_phase` when supplied.
- Generated patches update workspace/process metadata and write typed
  `process_state` when a foreground process is known.
- `ci/gameterm-scene-verify.sh --all` covers supplied and missing metadata.

Deferred:

- automatic live mux discovery
- multi-pane process maps
- progress polling

### Priority 2: Normal View Product Polish

Status: next.

Goal: make Scene Mode useful without opening the Tile Debugger.

Why next: the debugger explains the state well, but daily use needs the common
path visible in normal view.

Scope owner:

- [Normal View Polish Scope](gameterm-scene-normal-view-polish-scope.md)

First slice:

- clarify selected entity, active layers, status, blockers, process/agent
  state, and choices in normal view
- keep small terminal windows readable
- add focused render tests and screenshot smoke where layout changes matter

Commit shape:

- `[docs] scope Scene normal view polish`
- `[visual] polish Scene normal view state`
- `[test] verify Scene normal view polish`
- `[test] record Scene layout smoke` when live smoke runs

### Priority 3: Command Selection Surface

Goal: avoid turning generated workspaces into long flat choice lists.

Why next: Workspace Discovery can produce many files and actions; repeated use
needs grouped, explicit action selection.

Scope owner:

- [Command Selection Scope](gameterm-scene-command-selection-scope.md)

First slice:

- group actions by selected entity, file, project, or process
- preserve existing choices and explicit activation semantics
- ensure `RunCommand` and `OpenFile` remain visibly different

Commit shape:

- `[docs] scope Scene command selection`
- `[visual] add Scene command grouping`
- `[test] verify Scene command selection`

### Priority 4: Policy And Permission Boundaries

Goal: make generated and future agent-proposed actions auditable before they
can run.

Why next: command grouping increases action surface area; policy keeps that
surface explicit and safe.

Scope owner:

- [Policy Boundaries Scope](gameterm-scene-policy-boundaries-scope.md)

First slice:

- tag generated actions with origin and policy metadata
- show policy/source in normal view or debug view
- preserve explicit activation for all command execution

Commit shape:

- `[docs] scope Scene policy boundaries`
- `[visual] add Scene action policy metadata`
- `[test] verify Scene policy boundaries`

### Priority 5: Persisted Workspace Sessions

Goal: restore useful workspace state across repeated Scene Mode use.

Why later: session persistence is more valuable after live context and normal
view shape are stable.

Scope owner:

- [Workspace Sessions Scope](gameterm-scene-workspace-sessions-scope.md)

First slice:

- define what session state is worth saving
- avoid tracked or repo-local churn
- keep generated scenes reproducible from explicit inputs

### Priority 6: Memory, Relationships, And Multi-Agent State

Goal: make memory and multi-agent state useful as product behavior, not only
as model data.

Why later: the schema can already represent this state. The missing piece is
how it appears in the live workspace surface.

Scope owners:

- [Memory Relationships Scope](gameterm-scene-memory-relationships-scope.md)
- [Multi-Agent Coordination Scope](gameterm-scene-multi-agent-coordination-scope.md)
- [Agent Task Bootstrap Scope](gameterm-scene-agent-task-bootstrap-scope.md)

First slice:

- show relationship/memory state in the polished normal view
- connect multiple agent/process updates to live pane/process context
- keep task bootstrap explicit and user-confirmed

### Priority 7: Packaging And Onboarding

Goal: make Scene Mode easy to use repeatedly without tribal knowledge.

Why later: onboarding should describe the settled product path, not every
intermediate helper detail.

Scope owner:

- [Packaging Onboarding Scope](gameterm-scene-packaging-onboarding-scope.md)

First slice:

- update onboarding after priorities 1-4 land
- keep quickstart commands current
- record a fresh live smoke pass for the documented path

## Refactor Backlog

Refactor work should remain subordinate to product priorities. Do it only when
it directly reduces risk for the next lane.

Allowed next refactor lanes:

- table-drive smoke scenario definitions if smoke edits become hard to review
- extract small helper registries from author/workspace scripts when adding
  new product actions
- move narrow runtime helpers out of `lib.rs` only when a lane already touches
  that area

Avoid:

- broad crate splits
- schema renames without migration
- visual redesign mixed with NFC moves
- unrelated macOS warning cleanup inside Scene Mode commits

## Next Recommended Work

Do Priority 2 next: normal view product polish.

Reasoning:

- Priority 1 is already complete for the explicit metadata path.
- The debugger explains state well, but the normal product view is still the
  daily-use bottleneck.
- Normal-view polish makes live pane/process context, command selection,
  memory, and multi-agent state easier to understand before adding more data.

Concrete next checklist:

1. Re-read [Normal View Polish Scope](gameterm-scene-normal-view-polish-scope.md).
2. Update that scope if it is stale against current generated workspace scenes.
3. Define the normal-view information hierarchy for selected entity, active
   layers, status, blockers, process/agent state, and choices.
4. Implement the smallest render/runtime slice that improves generated
   workspace readability.
5. Add focused render tests.
6. Run `cargo test -p gameterm-visual`.
7. Run `ci/gameterm-scene-verify.sh --all`.
8. Run screenshot smoke if rendering dimensions or layout behavior changed.
