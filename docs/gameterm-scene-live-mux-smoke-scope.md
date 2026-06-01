# GameTerm Scene Live Mux Smoke Scenario Scope

This document scopes the follow-up smoke lane for live mux discovery: make
`live-mux-discovery` a first-class named smoke scenario in
`ci/gameterm-scene-smoke.sh`.

It builds on
[GameTerm Scene Live Mux Discovery Scope](gameterm-scene-live-mux-discovery-scope.md),
which implemented the helper-driven live mux context path.

## Goal

The smoke harness should be able to launch Scene Mode from a scene generated
from the currently active GameTerm mux pane, using the same command path that a
user or agent would run manually.

The scenario should make live mux discovery repeatable:

```sh
ci/gameterm-scene-smoke.sh --launch --scenario live-mux-discovery
```

## Product End State

This lane is complete when:

1. `live-mux-discovery` appears in `ci/gameterm-scene-smoke.sh --list-scenarios`.
2. `--describe-scenario live-mux-discovery` explains the generated scene,
   live mux dependency, fallback behavior, and expected visible state.
3. `--launch --scenario live-mux-discovery` generates a Scene Mode scene with
   `ci/gameterm-scene-mux-context.sh discover`.
4. The generated scene is installed into the smoke harness temporary
   `XDG_CONFIG_HOME` as `gameterm/scenes/default.json`.
5. The smoke launch still opens Scene Mode through the normal GUI automation.
6. The scenario prints the normalized mux context used for the launch or the
   fallback reason when mux context is unavailable.
7. Deterministic verification covers scenario registry, scenario description,
   fixture setup, and fallback behavior without requiring a live GUI/mux.
8. Live smoke records a capture path, pane id, mux window id, cwd, and result
   in `docs/gameterm-scene-smoke-report.md`.

## Non-Goals

- No new Scene schema.
- No direct GUI `TermWindow` introspection in this lane.
- No process polling loop.
- No terminal scrollback parsing.
- No command execution from discovered terminal text.
- No requirement that CI have a running GUI/mux.

## Existing Foundation

Already implemented:

- `ci/gameterm-scene-mux-context.sh collect|discover|patch|doctor`
- live collection through `gameterm cli --no-auto-start list --format json`
- `--allow-missing` fallback
- hosted `file://host/path` cwd normalization
- empty workspace discovery safety
- deterministic verifier coverage for mux context fixtures and CLI-list JSON
- one manual live smoke report entry for mux discovery

The smoke scenario should call this existing helper instead of duplicating mux
JSON parsing or workspace scene generation.

## Scenario Contract

Scenario name:

```text
live-mux-discovery
```

Default launch behavior:

```sh
ci/gameterm-scene-mux-context.sh discover \
  --allow-missing \
  --scene-output "${scene_dir}/default.json" \
  --force
```

The generated scene should then launch exactly like other smoke fixtures.

Expected generated scene state when live context exists:

- `pane_context=provided`
- `discovery_source=pane_cwd`
- `active_pane_id` variable exists
- `active_mux_window_id` variable exists
- `discovered-pane` entity has pane id, mux window id, and cwd metadata

Expected generated scene state when mux context is unavailable:

- `pane_context=absent`
- `discovery_source=cwd`
- no active pane/window variables
- scene still validates and launches from normal cwd discovery

## CLI And Harness Behavior

Add `live-mux-discovery` to:

- `list_smoke_scenarios`
- `describe_smoke_scenario`
- `apply_smoke_scenario_defaults`
- fixture install/setup branch
- launch-time audit text
- deterministic smoke asset/scenario checks
- user docs and smoke report checklist

The scenario should not add a new top-level smoke option unless needed.

If later we need fixture-backed deterministic launch setup for this scenario,
prefer adding an internal harness path that passes:

```sh
--cli-list-json ci/fixtures/gameterm-scene/mux-list-active.json
```

to the mux context helper. Do not make deterministic CI depend on the user's
currently running GameTerm instance.

## Verification

Focused checks:

```sh
bash -n ci/gameterm-scene-smoke.sh
ci/gameterm-scene-smoke.sh --list-scenarios | grep -qx live-mux-discovery
ci/gameterm-scene-smoke.sh --describe-scenario live-mux-discovery
ci/gameterm-scene-smoke.sh --check-assets
ci/gameterm-scene-verify.sh --all
```

Helper path checks:

```sh
tmp_home="$(mktemp -d /tmp/gameterm-live-mux-smoke.XXXXXX)"
scene_dir="${tmp_home}/gameterm/scenes"
mkdir -p "${scene_dir}"
ci/gameterm-scene-mux-context.sh discover \
  --cli-list-json ci/fixtures/gameterm-scene/mux-list-active.json \
  --scene-output "${scene_dir}/default.json" \
  --force
ci/gameterm-scene-author.sh validate "${scene_dir}/default.json"
```

Live smoke command:

```sh
cargo build -p gameterm-gui
ci/gameterm-scene-smoke.sh --launch --scenario live-mux-discovery \
  --output /Users/julianabeleda/Desktop/gameterm-scene-smoke-live-mux-discovery-YYYYMMDD-HHMMSS.png
```

Record:

- command
- capture path
- normalized context source
- pane id
- mux window id
- pane cwd
- pass/fail result
- any fallback reason

## Implementation Lanes

### Lane 1: Scope

Deliverables:

- this scope document
- roadmap/live mux scope links

Verification:

- `git diff --check`

Commit:

- `[docs] scope Scene live mux smoke scenario`

### Lane 2: Scenario Registry

Deliverables:

- [x] add `live-mux-discovery` to scenario list and description
- [x] add default scenario mapping
- [x] add launch audit text

Verification:

- `bash -n ci/gameterm-scene-smoke.sh`
- `ci/gameterm-scene-smoke.sh --list-scenarios`
- `ci/gameterm-scene-smoke.sh --describe-scenario live-mux-discovery`

Commit:

- `[test] add Scene live mux smoke scenario`

### Lane 3: Generated Scene Setup

Deliverables:

- [x] smoke fixture/setup branch that writes generated mux-discovery scene to
  `default.json`
- [x] sprite manifest install, matching generated workspace scenarios
- [x] clear fallback output when live mux context is absent

Verification:

- `ci/gameterm-scene-smoke.sh --check-assets`
- direct generated scene validation with fixture CLI-list JSON

Commit:

- `[test] generate Scene live mux smoke fixture`

### Lane 4: Deterministic Verification

Deliverables:

- [x] verifier coverage for scenario registry/description
- [x] fallback or fixture-backed generated scene check without live GUI
- [x] docs checklist update

Verification:

- `ci/gameterm-scene-verify.sh --all`

Commit:

- `[test] verify Scene live mux smoke scenario`

### Lane 5: Live Smoke Report

Deliverables:

- [x] live launch capture when a GUI/mux session is available
- [x] `docs/gameterm-scene-smoke-report.md` entry with command, artifact path, and
  normalized context

Verification:

- live smoke command succeeds
- screenshot exists and is non-empty
- `git diff --check`

Commit:

- `[test] record Scene live mux scenario smoke`

## Acceptance Checklist

- `live-mux-discovery` is discoverable through `--list-scenarios`.
- Scenario description is specific enough to run without hidden context.
- Launch path uses `gameterm-scene-mux-context.sh discover`.
- Generated scene validates before launch.
- Missing live mux context does not crash the smoke harness when
  `--allow-missing` is used.
- Deterministic checks do not require a running GUI/mux.
- Live smoke records real pane/window/cwd when available.
- No unrelated smoke scenarios regress.

## Recommendation

Implement this as a smoke-harness change only. The live mux helper already owns
context collection and workspace generation; the smoke script should only
prepare temp config, call the helper, launch the GUI, and record what happened.
