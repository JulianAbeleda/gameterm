# GameTerm Scene Mode First-Pass Scope

This document defines the 100% target for the first shippable Scene Mode pass.
The goal is not to finish every future visual novel, RPG, or agent feature. The
goal is to make Scene Mode stable, authorable, inspectable, and usable enough
for internal daily use.

## Completion Status

Status: COMPLETE for the first shippable Scene Mode pass.

Final verification:

- `ci/gameterm-scene-verify.sh --all`: PASS.
- `cargo test -p gameterm-visual`: PASS, 127 tests.
- `cargo check -p gameterm-gui`: PASS.
- Live smoke: PASS for all ten named scenarios, recorded in
  [GameTerm Scene Mode Smoke Report](gameterm-scene-smoke-report.md).
- Repo-local artifact churn: none required by the final smoke helper path.

Known non-blocking warning noise:

- Existing macOS `objc` macro `unexpected cfg` warnings.
- Existing `gameterm-toast-notification` unnecessary `unsafe` warnings.
- Existing `screen_line.rs` unused assignment warning.

These warnings are outside Scene Mode first-pass scope and did not block
verification.

## Definition Of Done

Scene Mode first pass is complete when a user can:

1. Open Scene Mode from the app without special setup.
2. Install or create a scene from a fixture/template.
3. Author entities, choices, variables, layers, and lightweight RPG state
   through helpers instead of hand-editing every field.
4. Play through dialogue, exploration/menu, quest/process, command, and
   agent/task state flows.
5. Save and reload story state from the GUI path.
6. Send local and mux patches into an active Scene Mode overlay.
7. Inspect current state, blocked guards, patches, process/agent state, and
   transition history in the Tile Debugger.
8. Run deterministic verification and live smoke without repo-local artifact
   churn.

## Layer 1: Launch And Runtime Stability

Purpose: Scene Mode must behave like a reliable app feature, not a demo path.

Current coverage:

- Bundled scene fallback.
- User `default.json` loading.
- Manual reload and auto-reload.
- Overlay close/reopen path.
- Multiple overlay registration.
- Mux window/pane patch targeting.
- Full ten-scenario live smoke pass.

Completed scope:

- Add a manual product smoke checklist for app-launch-to-close flows.
- Re-run live smoke after every first-pass lane that touches runtime, overlay,
  input, patching, or rendering.
- Record known warning noise separately from Scene Mode defects.
- Confirm crash recovery expectations: invalid scene, invalid patch, missing
  sprite, missing target pane, closed overlay.

Acceptance criteria:

- `ci/gameterm-scene-verify.sh --all` passes.
- `cargo check -p gameterm-gui` passes.
- Full live smoke suite passes.
- Manual product smoke checklist is current and green.
- No smoke fixture writes tracked files or persistent repo-local saves.

Commit lanes:

1. Add manual product smoke checklist doc.
2. Run and record final first-pass live smoke.
3. Fix any runtime defects found by manual smoke.

## Layer 2: Scene Schema And Validation

Purpose: authored scenes should fail early with useful diagnostics.

Current coverage:

- Core scene validation.
- Entity id, dimensions, positions, action target validation.
- Mode descriptors, lifecycle hooks, input maps.
- Variables, dialogue lines, RPG state.
- Layers, transitions, and deterministic operations.
- Patch validation.
- Doctor checks for scene/sprite/action issues.

Completed scope:

- Improve validation messages for common authoring mistakes:
  - unknown sprite id
  - unknown variable in guard
  - unknown layer target
  - unknown quest/inventory/stat reference
  - malformed metadata
  - invalid RunCommand target/cwd
- Add doctor suggestions for how to fix those cases.
- Add authoring helper checks that fail before writing invalid JSON.

Acceptance criteria:

- Common broken fixtures fail with actionable messages.
- Doctor output includes direct suggestions for fixable authoring mistakes.
- Validation failures do not mutate source files.

Commit lanes:

1. Add broken-fixture validation cases.
2. Add doctor suggestions.
3. Harden author helper pre-write validation.

## Layer 3: Game-State Model

Purpose: Scene Mode should support the common computational modes found in
games without treating everything as a screen.

Current coverage:

- Top-level mode descriptor.
- Layered state model.
- Layer input ownership.
- Guarded layer transitions.
- `game-states.json` fixture covering dialogue, exploration, inventory/menu,
  quest, command/combat-like, and agent/task layers.
- Transition history.

Completed scope:

- Add guard helper coverage for:
  - quest stage/completed
  - inventory item/count
  - stat value
  - process phase
  - agent phase
  - selected entity metadata/flags
- Add chained transition fixture:
  - dialogue completes
  - exploration unlocks menu
  - menu triggers quest update
  - quest update unlocks command state
  - command state triggers agent/process state
- Define rollback behavior for chained transitions.

Acceptance criteria:

- A fixture can demonstrate blocked, successful, and chained transitions.
- Failed transitions do not partially mutate variables, RPG state, layers, or
  selected entity state.
- Debug output explains why a transition was blocked.

Commit lanes:

1. Add richer guard helper schema/tests.
2. Add chained transition operation support if needed.
3. Add chained transition fixture and verifier coverage.

## Layer 4: Deterministic Actions

Purpose: choices and input bindings should mutate structured runtime state
without requiring external scripts for basic game behavior.

Current coverage:

- `Resolve` operations:
  - set/increment/clear variable
  - add/remove inventory
  - set/adjust stat
  - advance/complete quest
  - append quest journal
  - adjust relationship
  - set layer state
- Atomic validation and rollback.

Completed scope:

- Add deterministic operations for:
  - select entity by id
  - set entity flags/metadata
  - set entity visibility
  - advance dialogue and layer state together
  - trigger named transition if guard passes
- Decide whether chained transitions are an operation or a layer feature.
- Add readable action summaries in debug history.

Acceptance criteria:

- Common visual-novel/RPG flows can be authored without patch scripts.
- Deterministic actions remain in-memory unless explicitly exported.
- Every new operation has focused tests and fixture coverage.

Commit lanes:

1. Add entity-state operations.
2. Add named transition operation or chained layer transition support.
3. Add debug summaries for deterministic operations.

## Layer 5: Authoring UX

Purpose: a user should be able to build a useful scene without knowing the full
JSON schema by memory.

Current coverage:

- Install/list fixtures.
- List/create templates.
- Create new scene.
- Add/remove/move entity.
- Add/update/remove choice.
- Set dialogue.
- Format and validate.
- Doctor diagnostics.

Completed scope:

- Add authoring commands for:
  - add/set/clear variable
  - add/remove inventory item
  - set/adjust stat
  - add/advance/complete quest
  - append quest journal
  - add/set layer
  - add layer transition
  - add mode input binding
  - set lifecycle status
- Add command examples to docs.
- Ensure helper output names the file changed and validates after mutation.

Acceptance criteria:

- The `game-states` fixture can be recreated or meaningfully edited using
  helpers.
- Authoring helpers reject bad references before writing.
- Docs contain one copy-pasteable authoring path from blank scene to playable
  stateful scene.

Commit lanes:

1. Add variable/layer authoring commands.
2. Add RPG authoring commands.
3. Add mode/input/lifecycle authoring commands.
4. Update docs and verifier.

## Layer 6: Persistence UX

Purpose: save/load should feel like a normal first-pass game/tool loop.

Current coverage:

- Story-state export/import actions.
- GUI dispatch for story-state actions.
- Default story-state path.
- Visible story-state path/status in Scene view.
- Debugger story-state status.
- Authoring-loop smoke fixture.

Completed scope:

- Add authoring helper commands for story-state export/import/inspect examples.
- Add manual smoke for close/reopen/load state.
- Decide whether default save path should be configurable in scene JSON.
- Add validation for unsafe or surprising story-state paths if needed.

Acceptance criteria:

- User can save, mutate, reload, close, reopen, and inspect the result.
- Story-state path is visible before and after action.
- Failed import/export is visible and does not corrupt runtime state.

Commit lanes:

1. Add close/reopen/load manual smoke checklist.
2. Add configurable save path only if needed by first-pass smoke.
3. Add failed import/export fixture or focused test.

## Layer 7: Patch, Process, And Agent Integration

Purpose: external commands and agents should update Scene Mode through stable
state contracts instead of ad hoc UI coupling.

Current coverage:

- Patch inbox transport.
- Mux patch transport.
- Explicit pane targeting.
- Patch source debug visibility.
- Process helper with typed process state.
- Agent helper with idle/planning/running/waiting/blocked/completed/failed/
  cancelled phases.
- Agent phase variables for scene guards.

Remaining scope:

- Add a sample real task-runner integration path:
  - start task
  - emit running patch
  - emit blocked/waiting patch
  - emit completed/failed patch
- Add process/agent guard fixture.
- Confirm multiple overlays receive intended patch targets in manual smoke.
- Decide what remains simulated versus real for first pass.

Acceptance criteria:

- A real shell command can drive process state.
- An agent/task lifecycle can drive scene guards/layers.
- Failed/cancelled states are visible and recoverable.
- Multiple-overlay routing behavior is documented and smoke-tested.

Commit lanes:

1. Add process/agent guard fixture.
2. Add real task-runner smoke helper if needed.
3. Add multiple-overlay manual smoke checklist.

## Layer 8: Rendering And Presentation

Purpose: Scene Mode should be understandable to a user, not only to a debugger.

Current coverage:

- Text frame rendering.
- Sprite/tile rendering.
- Bundled fallback sprites.
- Selected entity/choice/status display.
- Debugger display.

Remaining scope:

- Improve normal Scene view hierarchy:
  - mode/layer summary
  - state summaries that do not crowd dialogue
  - clearer locked choice display
  - concise patch/process/agent status
- Add presentation checks for narrow terminal sizes.
- Keep debugger dense, but keep normal view user-readable.

Acceptance criteria:

- Normal view explains what is happening after state changes.
- Debug details remain available but are not required for basic play.
- Text does not overlap or truncate critical status in common terminal sizes.

Commit lanes:

1. Polish normal view state/status layout.
2. Add narrow-size render tests.
3. Update smoke screenshots after layout changes.

## Layer 9: Observability

Purpose: complex scenes need clear answers to "what just happened?"

Current coverage:

- Tile Debugger shows mode, layers, input map, lifecycle, selected entity,
  variables, RPG state, process state, story state, patch source, and bounded
  transition history.

Remaining scope:

- Add clearer event categories for:
  - input
  - guard blocked
  - operation applied
  - operation failed
  - patch applied
  - process/agent update
  - story import/export
- Add final debug report shape doc.
- Add one fixture that deliberately blocks multiple action types.

Acceptance criteria:

- Debugger can diagnose guard failure, failed operation, failed patch, and
  failed import/export.
- Transition history stays bounded and readable.
- Debug output is covered by focused tests.

Commit lanes:

1. Expand event categories.
2. Add blocked-actions fixture.
3. Document debug report fields.

## Layer 10: Smoke And Release Discipline

Purpose: first-pass completion should be provable, not just claimed.

Current coverage:

- Deterministic verifier.
- Ten live smoke scenarios.
- Smoke report with artifact convention.
- Scenario contracts with expected status lines.

Remaining scope:

- Add first-pass manual smoke checklist.
- Add final first-pass smoke report section.
- Add "known deferrals" list.
- Require each first-pass lane to land as a separate commit.

Acceptance criteria:

- `ci/gameterm-scene-verify.sh --all` passes.
- `cargo check -p gameterm-gui` passes.
- Full live smoke suite passes.
- Manual smoke checklist passes.
- First-pass roadmap marks all required items complete.

Commit lanes:

1. Add checklist.
2. Run final verification.
3. Run final live smoke.
4. Mark first pass complete in roadmap.

## Explicit Deferrals

These are out of scope for the first pass:

- Full Ren'Py/Ink/Yarn compatibility.
- A visual drag-and-drop scene editor.
- Full combat engine.
- Full ECS implementation.
- Real-time physics.
- Audio system.
- Save-slot UI beyond visible story-state paths.
- Networked multiplayer or shared scene sessions.
- Fully autonomous agent execution UI.

## Recommended Implementation Order

1. Authoring commands for variables/layers/RPG state.
2. Guard helper coverage.
3. Chained transition fixture.
4. Manual product smoke checklist.
5. Normal Scene view presentation polish.
6. Process/agent guard fixture.
7. Debug event category polish.
8. Final verification and live smoke report.

This order keeps the remaining work focused on making Scene Mode authorable and
provable before polishing presentation.
