# GameTerm Scene Mode Refactor Plan

This plan starts after the first shippable Scene Mode pass. The first pass is
feature-complete and verified; refactors should preserve behavior and land in
small commits with focused tests.

## Goals

- Reduce `gameterm-visual/src/lib.rs` size and coupling.
- Keep the Scene JSON contract stable unless a migration is explicitly scoped.
- Make authoring helpers easier to extend without large shell conditionals.
- Separate runtime state, schema validation, action execution, debug reporting,
  and fixture coverage.
- Preserve the full first-pass verification gate.

## Non-Goals

- No second-pass product features in refactor commits.
- No schema-breaking rename without migration support.
- No visual redesign while moving code.
- No unrelated cleanup of existing macOS warning noise.

## Verification Gate

Every refactor lane should pass:

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

## Priority 1: Split Visual Runtime Modules

Current pressure: `gameterm-visual/src/lib.rs` owns schema, validation, runtime,
story state, patch application, rendering entry points, debug reports, and
tests. It is now doing too much.

Target module shape:

```text
gameterm-visual/src/
  lib.rs
  schema.rs
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

1. Move plain schema structs/enums into `schema.rs`.
2. Move validation helpers and `VisualSceneError` into `validation.rs`.
3. Move condition evaluation into `conditions.rs`.
4. Move deterministic resolve operations into `actions.rs`.
5. Move story-state export/import into `story_state.rs`.
6. Move patch schema/application into `patch.rs`.
7. Move debug report formatting into `debug.rs`.

Acceptance:

- Public exports remain compatible from `gameterm_visual`.
- Tests still pass without changing fixture JSON.
- Each move commit is mostly mechanical.

## Priority 2: Normalize Action Execution

Current pressure: choices, mode input maps, layer input maps, layer transitions,
story-state actions, and deterministic operations are related but implemented
through separate paths.

Target:

- A small internal action dispatcher that takes an action request and returns a
  typed outcome.
- Shared rollback behavior for deterministic mutations.
- Shared status/debug event generation.
- One clear place for action summaries.

Commit lanes:

1. Introduce an internal `SceneActionOutcome`.
2. Route `Inspect`, `AdvanceDialogue`, and pending external actions through the
   outcome type.
3. Route `Resolve` through a transaction object.
4. Convert layer-transition trigger logic to reuse the same transaction path.
5. Add focused tests for unchanged status strings and generation bumps.

Acceptance:

- Existing action statuses and debug history remain stable.
- Failed operations still do not partially mutate state.
- `ci/gameterm-scene-verify.sh --all` remains green.

## Priority 3: Make Authoring Helper Data-Driven

Current pressure: `ci/gameterm-scene-author.sh` is useful but growing through
large case blocks and jq snippets.

Target:

- Keep the shell entrypoint for local ergonomics.
- Move reusable jq filters into files or a generated helper script.
- Make command option validation consistent.
- Add one shared mutation path that validates before replacing files.

Commit lanes:

1. Extract jq filters into `ci/scene-author/*.jq`.
2. Add a small command metadata table for help text.
3. Normalize typed value and guard construction.
4. Add helper tests for invalid mutation rollback.
5. Keep existing command names stable.

Acceptance:

- Existing docs examples still work.
- Authoring verifier still covers every command.
- Failed commands leave the original scene file unchanged.

## Priority 4: Fixture And Smoke Organization

Current pressure: fixtures, smoke scenarios, and verifier checks are now broad
enough to need clearer ownership.

Target:

- Fixture README maps each fixture to the feature it proves.
- Smoke scenarios have one table of fixture, setup, key sequence, and expected
  status.
- Verifier output stays concise.

Commit lanes:

1. Add a scenario table to fixture docs.
2. Move scenario metadata in `ci/gameterm-scene-smoke.sh` into a structured
   block or external JSON.
3. Add a summary mode for the verifier.
4. Keep full verbose logs available on failure.

Acceptance:

- All current scenario names remain valid.
- Smoke output is quieter by default.
- Failure diagnostics still include enough context to debug.

## Priority 5: Test Layout

Current pressure: runtime tests are comprehensive but concentrated in one file.

Target:

- Tests grouped by behavior: schema validation, conditions, actions, story
  state, patching, fixtures, rendering.
- Shared scene builders for common setup.
- Less repeated literal construction.

Commit lanes:

1. Move test helpers to a local test module.
2. Group tests by module as code is split.
3. Add builders for `VisualCondition`, `SceneAction`, and layer state.
4. Keep fixture tests close to fixture files.

Acceptance:

- No coverage loss.
- Test names remain searchable by feature.
- Refactor commits avoid changing product behavior.

## Suggested Order

1. Split conditions and actions first; they are the highest-change areas.
2. Split schema/validation after action behavior is isolated.
3. Refactor authoring helper once runtime modules settle.
4. Refactor smoke/fixtures last, because they are the safety net.

Do not start second-pass product features until these refactors either land or
are explicitly deferred.
