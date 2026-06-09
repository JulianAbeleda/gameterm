# GameTerm Scene Mode Refactor Plan

This plan starts after the first shippable Scene Mode pass. The first pass is
feature-complete and verified; refactors should preserve behavior and land in
small commits with focused tests.

Current product status and next priorities are consolidated in
[GameTerm Scene Mode Roadmap](gameterm-scene-roadmap.md). Use this refactor
plan only for scoped behavior-preserving cleanup lanes.

This plan is constrained by
[`structure/Development/coding-principles.md`](../structure/Development/coding-principles.md).
Scene Mode is a GameTerm-specific surface in a WezTerm fork, so refactors must
stay narrow, keep upstream behavior intact, and avoid broad rename or ownership
sweeps.

Current next-pass scope:

- [Scene Asset Editor Refactor Scope](gameterm-scene-asset-editor-refactor-scope.md)
- [Scene Maintainability Refactor Scope](gameterm-scene-maintainability-refactor-scope.md)

The first maintainable refactor pass is closed out later in this document. Use
the asset-editor refactor scope when continuing the primitive-editor path. Use
the broader maintainability scope for overlay/runtime follow-up work unless a
product defect requires a separate behavior fix first.

## Goals

- Reduce `gameterm-visual/src/lib.rs` size and coupling.
- Keep the Scene JSON contract stable unless a migration is explicitly scoped.
- Make authoring helpers easier to extend without large shell conditionals.
- Separate runtime state, schema validation, action execution, debug reporting,
  and fixture coverage.
- Preserve the full first-pass verification gate.
- Keep refactor scope principle-driven: stop when the remaining move is mostly
  aesthetic, high-churn, or harder to review than the code it replaces.

## Non-Goals

- No second-pass product features in refactor commits.
- No schema-breaking rename without migration support.
- No visual redesign while moving code.
- No unrelated cleanup of existing macOS warning noise.
- No upstream-wide reorganization outside the Scene Mode ownership boundary.
- No crate-boundary changes unless a lane proves they are necessary.

## Coding-Principle Constraints

Fork discipline:

- Keep Scene Mode refactors isolated to GameTerm-specific code paths.
- Do not rename upstream concepts, docs, binaries, config keys, or packages.
- Preserve upstream attribution and licensing.
- Put local planning and audit notes in `structure/` or focused Scene Mode docs;
  do not scatter transient notes through upstream documentation.

Rust workspace discipline:

- Treat root `Cargo.toml` workspace membership as fixed for this refactor.
- Prefer existing `gameterm-visual`, `gameterm-gui`, and `ci/` boundaries over
  new cross-cutting crates.
- Inspect each target module before moving shared types or APIs.
- Keep public `gameterm_visual` exports compatible unless a separate migration
  commit is explicitly scoped.

Commit discipline:

- Use small commits with project-specific prefixes.
- Mark mechanical behavior-preserving commits with `NFC`.
- Do not mix an NFC move with a behavior fix.
- Use `[visual]` for Scene runtime/module and helper changes, `[test]` for
  verification-only reshaping, and `[docs]` for documentation.

Verification discipline:

- Start each lane with focused checks, then broaden before commit.
- Run live smoke only when the lane touches GUI, input, rendering, patch
  transport, story-state dispatch, or smoke automation.

## Verification Gate

Every refactor lane should have a focused check tied to what moved. Before
committing a lane, broaden to:

```sh
cargo test -p gameterm-visual
cargo check -p gameterm-gui
ci/gameterm-scene-verify.sh --all
```

Run live smoke when a lane touches overlay dispatch, input handling, rendering,
patch transport, story-state import/export, or smoke automation:

```sh
ci/gameterm-scene-smoke.sh --launch --scenario vertical-slice \
  --output /tmp/gameterm-scene-refactor-vertical-slice.png
```

## Priority 0: First Refactor Gate

Do not start broad cleanup until the active-pane GUI closure pass is either
green or has a concrete product defect scoped separately.

Entry checks:

```sh
cargo test -p gameterm-visual workspace_scene --lib
cargo test -p gameterm-gui commands::tests::active_pane_scene --bin gameterm-gui
cargo check -p gameterm-gui
ci/gameterm-scene-smoke.sh --launch --scenario active-pane-gui \
  --output /tmp/gameterm-scene-active-pane-gui.png
```

Principle alignment:

- product fixes land before NFC refactors
- shell/Rust generator ownership stays explicit unless a dedicated migration
  lane routes helpers through Rust
- warning cleanup outside touched Scene paths remains out of scope
- each refactor lane has one owner prefix and one focused verification command

First behavior-preserving lanes after closure:

1. `[test] NFC - table-drive Scene smoke scenario metadata`
   - Reason: smoke scenario branching is now the highest-churn helper surface.
   - Check: `bash -n ci/gameterm-scene-smoke.sh` and scenario list/describe
     commands.
2. `[visual] NFC - extract workspace scene contract helpers`
   - Reason: active-pane parity fields should be centralized before more
     workspace entities are added.
   - Check: `cargo test -p gameterm-visual workspace_scene --lib`.
3. `[docs] clarify Scene generator ownership`
   - Reason: prevent future feature lanes from re-opening shell-vs-Rust
     direction without a migration plan.
   - Check: `rg -n "install/persistence|transient GUI|active-pane" docs`.

## Priority 1: Split Visual Runtime Modules

Current pressure: `gameterm-visual/src/lib.rs` owns schema, validation, runtime,
story state, patch application, rendering entry points, debug reports, and
tests. It is now doing too much.

Principle fit: this is an internal `[visual] NFC` refactor. It keeps the crate
boundary stable and preserves the public `gameterm_visual` API.

Target module shape after the lane is complete:

```text
gameterm-visual/src/
  lib.rs
  schema.rs        optional; only if a narrow, low-churn slice is available
  validation.rs
  runtime.rs
  actions.rs
  conditions.rs
  story_state.rs
  patch.rs
  debug.rs
  fixtures.rs
  render/
```

Commit lanes:

1. `[visual] NFC - move Scene condition helpers` ✅ done
   - Move only condition evaluation and guard-detail formatting.
   - Focused check: `cargo test -p gameterm-visual guarded_choice rpg_condition_sources_guard_choices`.
2. `[visual] NFC - move Scene action resolution` ✅ done
   - Move deterministic operation summaries and transaction helpers without
     changing status strings.
   - Focused check: `cargo test -p gameterm-visual resolve_action`.
3. `[visual] NFC - move Scene story-state helpers` ✅ done
   - Move story export/import structures and helpers.
   - Focused check: `cargo test -p gameterm-visual story_state`.
4. `[visual] NFC - move Scene patch helpers` ✅ done
   - Move patch schema and patch application helpers.
   - Focused check: `cargo test -p gameterm-visual scene_patch`.
5. `[visual] NFC - move Scene validation helpers` ✅ done
   - Move validation only after schema/action dependencies are stable.
   - Focused check: `cargo test -p gameterm-visual scene_rejects`.
6. `[visual] NFC - move Scene debug reporting` ✅ done
   - Move debug report data structure and report builder.
   - Focused check: `cargo test -p gameterm-visual debug_report`.
7. `[visual] NFC - move Scene debugger text rendering` optional
   - Move only the text-frame debugger renderer if the extracted function can
     avoid broad runtime field exposure.
   - Focused check: `cargo test -p gameterm-visual debugger`.
8. `[visual] NFC - move Scene mode/layer schema` ✅ done
   - Optional schema slice. Move only mode/layer DTOs if serde defaults and
     default mode construction stay easy to review.
   - Focused check: `cargo test -p gameterm-visual mode_ layered_`.
9. `[visual] NFC - move Scene RPG/state schema`
   - Optional schema slice. Move state values, variables, RPG structs, and
     state operations only if imports remain local and public exports stay
     compatible.
   - Focused check: `cargo test -p gameterm-visual rpg resolve_action`.
10. `[visual] NFC - move Scene core schema`
    - Optional final schema slice. Move VisualScene, entity, dialogue, render
      DTOs, and sprite manifest DTOs only if the diff is still mostly
      mechanical and does not hide behavior changes.
    - Focused check: full `cargo test -p gameterm-visual`.

Acceptance:

- Public exports remain compatible from `gameterm_visual`.
- Tests still pass without changing fixture JSON.
- Each move commit is mostly mechanical.
- `git diff --stat` for each commit is dominated by moved code, not rewrites.
- It is acceptable to leave schema in `lib.rs` if moving it requires a large
  low-signal diff or exposes private runtime fields just to satisfy a shape.

Current status:

- Completed the core high-value split: conditions, actions, story state, patch,
  validation helpers, debug report building, and mode/layer schema.
- Deferred full schema extraction after the mode/layer slice. The remaining
  schema block is still intertwined with public API compatibility, sprite
  manifest DTOs, render DTOs, and central runtime construction. Per coding
  principles, forcing that as a broad sweep would be higher churn than value.

## Priority 2: Normalize Action Execution

Current pressure: choices, mode input maps, layer input maps, layer transitions,
story-state actions, and deterministic operations are related but implemented
through separate paths.

Principle fit: this is not NFC unless behavior is provably unchanged. It should
start only after the module split makes the behavior easy to test.

Target:

- A small internal action dispatcher that takes an action request and returns a
  typed outcome.
- Shared rollback behavior for deterministic mutations.
- Shared status/debug event generation.
- One clear place for action summaries.

Commit lanes:

1. `[visual] add internal Scene action outcome` ✅ done
   - Introduce the type behind existing behavior.
   - Tests assert unchanged status strings and generation bumps.
   - Do not rename existing actions or status text.
2. `[visual] route Scene pending actions through action outcome` ✅ done
   - Cover `OpenFile`, `RunCommand`, `Navigate`, story export/import.
   - Existing `VisualActionRequest` shape remains compatible.
3. `[visual] route Scene deterministic actions through transactions` ✅ done
   - Preserve rollback behavior and action summaries.
4. `[visual] unify Scene layer transition trigger path` ✅ done
   - Reuse transaction guard evaluation for trigger operations.
5. `[test] cover Scene action status compatibility` ✅ done
   - Add status/generation compatibility tests if earlier commits expose gaps.

Acceptance:

- Existing action statuses and debug history remain stable.
- Failed operations still do not partially mutate state.
- `ci/gameterm-scene-verify.sh --all` remains green.

Current status:

- Completed. The runtime now has a typed internal action outcome, a named
  deterministic transaction boundary, a shared layer-transition application
  helper, and explicit status compatibility coverage.
- Deferred deeper input-map dispatch unification because the current shared
  outcome path covers the high-risk pending/deterministic behavior without
  forcing unrelated input refactors.

## Priority 3: Make Authoring Helper Data-Driven

Current pressure: `ci/gameterm-scene-author.sh` is useful but growing through
large case blocks and jq snippets.

Principle fit: this is `[test]` work. Avoid replacing the helper with a new
language or framework unless shell/jq becomes a demonstrated blocker.

Detailed execution scope:

- [Scene Author Helper Refactor Scope](gameterm-scene-author-helper-refactor-scope.md)

Target:

- Keep the shell entrypoint for local ergonomics.
- Move reusable jq filters into named shell helpers where that reduces
  duplication without hiding behavior.
- Make command option validation consistent.
- Add one shared mutation path that validates before replacing files.

Commit lanes:

Guardrail already complete:

- `[test] cover Scene author mutation rollback` ✅ done
  - Add regression checks for failed mutations leaving files unchanged.

Follow-up cleanup lanes:

1. `[test] NFC - table-drive Scene author catalogs` ✅ done
   - Centralize fixture/template names used by help and list commands.
2. `[test] NFC - extract Scene author jq filters` ✅ done
   - Move reusable jq snippets without changing command output.
3. `[test] normalize Scene author typed values` ✅ done
   - Share boolean/number/text parsing for state and condition values.
4. `[test] NFC - centralize Scene author mutations` ✅ done
   - Rename the validated write path and remove redundant post-write
     validation calls.
5. `[test] NFC - table-drive Scene author help` ✅ done
   - Keep rendered help output byte-for-byte stable.
6. `[docs] update Scene author refactor status` ✅ done
   - Docs only; no user-facing example changes needed.

Acceptance:

- Existing docs examples still work.
- Authoring verifier still covers every command.
- Failed commands leave the original scene file unchanged.

Current status:

- Completed the guardrail slice: failed author mutations are now verified to
  leave the scene file unchanged.
- Completed the scoped author-helper cleanup pass:
  [Scene Author Helper Refactor Scope](gameterm-scene-author-helper-refactor-scope.md).
  The shell entrypoint, command names, option names, help output, generated JSON
  shape, and rollback behavior remain stable.

## Priority 4: Fixture And Smoke Organization

Current pressure: fixtures, smoke scenarios, and verifier checks are now broad
enough to need clearer ownership.

Principle fit: this is `[test]` or `[visual]` depending on whether it changes
verification behavior. Keep smoke changes separate from runtime changes.

Target:

- Fixture README maps each fixture to the feature it proves.
- Smoke scenarios have one table of fixture, setup, key sequence, and expected
  status.
- Verifier output stays concise.

Commit lanes:

1. `[docs] document Scene fixture scenario ownership` ✅ done
   - Map each fixture to the feature it proves and the focused tests that load
     it.
2. `[test] NFC - table-drive Scene smoke scenarios` ✅ done
   - Keep existing scenario names valid.
3. `[test] add Scene verifier summary mode`
   - Optional, only if verifier output becomes hard to scan.
4. `[test] cover Scene smoke scenario registry`
   - Add coverage only after the registry exists.

Acceptance:

- All current scenario names remain valid.
- Smoke output is quieter by default.
- Failure diagnostics still include enough context to debug.

Current status:

- Completed fixture ownership documentation.
- Completed a narrow smoke scenario catalog extraction to
  `ci/fixtures/gameterm-scene/smoke-scenarios.psv`, preserving existing
  scenario names and smoke helper behavior.
- Deferred verifier summary mode. The verifier output is still acceptable for
  the first maintainable pass.

## Priority 5: Test Layout

Current pressure: runtime tests are comprehensive but concentrated in one file.

Principle fit: this is `[test] NFC` when no assertions change. Keep test moves
separate from new coverage.

Target:

- Tests grouped by behavior: schema validation, conditions, actions, story
  state, patching, fixtures, rendering.
- Shared scene builders for common setup.
- Less repeated literal construction.

Commit lanes:

1. `[test] NFC - move Scene test helpers` ✅
   - Move helper constructors and fixture path helpers first.
2. `[test] NFC - group Scene condition tests`
   - Preserve test names where possible for searchability.
3. `[test] NFC - group Scene action tests`
   - Keep behavior assertions unchanged.
4. `[test] add Scene runtime builders`
   - Add builders only when they reduce repeated setup without hiding intent.
5. `[test] NFC - group Scene fixture tests`
   - Move fixture tests after helpers settle.

Acceptance:

- No coverage loss.
- Test names remain searchable by feature.
- Refactor commits avoid changing product behavior.

Current status:

- Completed a narrow helper extraction to `gameterm-visual/src/tests/test_support.rs`
  (fixture path, snapshot fixture, branching dialogue scene).
- Deferred broad test grouping and runtime builders. The tests are still large, but
  moving many test blocks now would create high review churn while the runtime
  surface is still settling.

## Priority 6: Runtime Method Split

Current pressure: `SceneRuntime` still owns lifecycle hooks, input dispatch,
layer transitions, rendering entry points, selection, reload, and status helpers
in one impl block.

Principle fit: do this only after module helper moves are stable. Prefer small
impl-block moves over introducing a new runtime abstraction.

Target:

- Keep `SceneRuntime` as the central runtime type.
- Split method groups by concern without changing public behavior.
- Avoid exposing runtime fields publicly only to make modules compile.

Commit lanes:

1. `[visual] NFC - group Scene runtime lifecycle methods` ✅ done
   - Move enter/update/exit hook methods or group them in a focused impl block.
   - Focused check: `cargo test -p gameterm-visual mode_lifecycle`.
2. `[visual] NFC - group Scene runtime input methods`
   - Move mode/layer input helpers only if field access remains contained.
   - Focused check: `cargo test -p gameterm-visual mode_ layered_input`.
3. `[visual] NFC - group Scene runtime selection methods`
   - Move entity/choice selection helpers.
   - Focused check: `cargo test -p gameterm-visual selection mode_next mode_previous`.
4. `[visual] NFC - group Scene runtime status helpers`
   - Move mark/open/run-command/status helpers only if no behavior changes.
   - Focused check: `cargo test -p gameterm-visual status_helpers open_file run_command`.

Acceptance:

- No public field exposure added solely for module access.
- `SceneRuntime` constructor and public methods remain compatible.
- Status strings and generation bumps remain unchanged.

Current status:

- Completed lifecycle method grouping.
- Completed input-method grouping.
- Completed selection-method grouping.
- Completed status-helper grouping.

## Priority 7: Retire Unneeded Scene Compatibility Paths

Current pressure: the first product pass intentionally kept several demo,
compatibility, and fallback paths while the product direction was still
settling. Those paths are useful only if they still support the current Rust
native Scene Mode direction. They should be audited explicitly instead of
remaining as silent complexity.

Principle fit: this is cleanup, but not automatically NFC. Removing a supported
demo, importer, fixture, or fallback changes the product surface. Each removal
must either be docs-only scoping or a small behavior-changing commit with tests
updated in the same commit. Do not mix this with mechanical module moves.

Candidates to audit:

1. RPY/VN import demo lane
   - `gameterm-visual/src/vn_script_import.rs`
   - `gameterm-visual/examples/scene_vn_script_import.rs`
   - `ci/gameterm-scene-vn-demo.sh`
   - `ci/fixtures/gameterm-scene/renpy-demo*`
   - Decision: keep as a generic VN script subset importer, rename/scope it
     away from Ren'Py-specific product language, or remove it if Rust-native
     authored scenes are the only supported direction.
2. PNG panel compatibility path
   - `assets/gameterm-scene/vn-panel.png`
   - `GAMETERM_SCENE_VN_PANEL_TEXTURE=1`
   - `populate_vn_panel_texture` and nine-slice fallback path in
     `gameterm-gui/src/termwindow/render/visual_quad.rs`
   - Decision: keep as a visual regression/fallback path, or remove once the
     procedural rounded panel renderer is accepted as the permanent renderer.
3. Unwired image placement primitives
   - `VisualImageScaleMode::FitCenter`
   - `VisualImageScaleMode::IntegerFitCenter`
   - Decision: keep if they are part of the planned renderer primitive toolkit,
     or remove if they are only speculative.
4. Historical Scene scope docs
   - older one-off scope documents that are superseded by roadmap, handoff,
     smoke report, and this refactor plan
   - Decision: archive, consolidate, or leave if they still provide useful
     audit history.

Non-candidates:

- `mux`
- terminal compatibility crates
- Unicode/generated table files
- platform windowing code
- Scene tests that cover active behavior
- Fake Codex, TTS, STT, and current voice diagnostics
- Kiki/background fixture assets used by current smoke/demo validation

Commit lanes:

1. `[docs] scope Scene compatibility retirement`
   - Record the keep/remove decision for RPY import, PNG panel fallback, unused
     image placement primitives, and historical docs.
   - Check: `rg -n "renpy|rpy|VN_PANEL_TEXTURE|IntegerFitCenter|FitCenter" docs ci gameterm-visual gameterm-gui`.
2. `[visual] retire or rename Scene VN script import compatibility`
   - Only if the decision is to remove or de-Ren'Py the importer.
   - Update verifier fixture lists, examples, docs, and validation origin names
     in the same behavior commit.
   - Focused check: `cargo test -p gameterm-visual vn_script_import`.
3. `[render] retire VN PNG panel fallback`
   - Only if procedural panels are accepted as the permanent path.
   - Remove the env toggle, embedded image data, nine-slice panel fallback, and
     related smoke verifier expectations.
   - Focused check: `cargo test -p gameterm-gui vn_panel --bin gameterm-gui`.
4. `[render] retire unused image placement primitives`
   - Only if `FitCenter` and `IntegerFitCenter` are not part of the next
     renderer toolkit.
   - Keep tests for the active `FillCenter` and `FitBottomCenter` primitives.
   - Focused check: `cargo test -p gameterm-gui visual_quad --bin gameterm-gui`.
5. `[docs] consolidate historical Scene scope docs`
   - Move old scope material into a compact archive/index if it is no longer
     active planning material.
   - Check: `rg -n "Current status|Status:" docs/gameterm-scene-*.md`.

Acceptance:

- No current app-launch Scene Mode path regresses.
- `ci/gameterm-scene-verify.sh --all` remains green or is intentionally
  updated because a retired fixture/test is no longer part of the contract.
- Deleted functionality is documented as intentionally retired, not lost by
  accident.
- No generated/table data is removed merely to reduce raw LOC.
- No upstream terminal feature is removed as part of Scene cleanup.

Current status:

- Scoped here only. No compatibility path has been retired yet.

## Full Refactor Definition

This refactor is considered complete for the first maintainable pass when:

- High-coupling helpers are no longer embedded in the main runtime file.
- Remaining code in `lib.rs` has an explicit reason to stay there for now:
  schema/public API compatibility, the central runtime type, or tests awaiting a
  test-layout pass.
- Every completed move has a separate commit, focused test, broad visual test,
  GUI check, and Scene verifier pass.
- Deferred work is documented with a reason tied to the coding principles, not
  just left vague.

This refactor is not considered complete merely because `lib.rs` reaches an
arbitrary line count. Line count is a signal, not the acceptance criterion.

## Suggested Order

1. Finish any remaining narrow schema slice only if it is low-churn.
2. Normalize action execution after mechanical moves prove behavior is stable.
3. Refactor authoring helper once runtime module names settle.
4. Organize fixtures and smoke after runtime/helper behavior is stable.
5. Split tests after helper modules are stable enough that test grouping will
   not churn repeatedly.
6. Split runtime method groups only where it improves reviewability without
   exposing internals.
7. Audit and retire only those Scene compatibility paths that no longer match
   the Rust-native Scene Mode product direction.

Do not start second-pass product features until these refactors either land or
are explicitly deferred.

## First-Pass Closeout

Status: first maintainable refactor pass complete.

Completed commits:

- `[visual] NFC - move Scene mode and layer schema`
- `[visual] add internal Scene action outcome`
- `[visual] route Scene story-state requests through action outcome`
- `[visual] route Scene deterministic actions through transactions`
- `[visual] unify Scene layer transition actions`
- `[test] cover Scene action status compatibility`
- `[test] cover Scene author mutation rollback`
- `[docs] document Scene fixture ownership`
- `[test] NFC - move Scene fixture test helper`
- `[visual] NFC - group Scene runtime lifecycle methods`
- `[test] NFC - move Scene test helpers`

Final verification:

- `cargo test -p gameterm-visual`
- `cargo check -p gameterm-gui`
- `ci/gameterm-scene-verify.sh --all`
- `ci/gameterm-scene-smoke.sh --launch --scenario vertical-slice --output /tmp/gameterm-scene-refactor-vertical-slice.png`

Audit result:

- Public Scene JSON shape remains stable.
- Public `gameterm_visual` exports remain compatible.
- Existing status strings and pending action request shapes are covered.
- Failed deterministic actions and failed author-helper mutations preserve
  rollback behavior.
- Remaining work is deliberately deferred as polish rather than required for
  the first maintainable pass.

## Stop Conditions

Pause the refactor lane and reassess if:

- A move requires changing public JSON shape.
- A move requires touching non-Scene upstream modules beyond imports/exports.
- Focused tests pass but broad `ci/gameterm-scene-verify.sh --all` fails.
- The diff stops being mostly mechanical.
- A behavior fix is discovered; commit or plan that fix separately before
  continuing NFC work.
