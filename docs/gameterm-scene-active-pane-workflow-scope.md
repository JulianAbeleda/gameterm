# GameTerm Scene Active Pane Workflow Scope

This document scopes the daily-use workflow for Scene Mode: generate, validate,
install, and open a Scene workspace from the active GameTerm pane.

It builds on:

- [GameTerm Scene Live Mux Discovery Scope](gameterm-scene-live-mux-discovery-scope.md)
- [GameTerm Scene Live Mux Smoke Scenario Scope](gameterm-scene-live-mux-smoke-scope.md)

## Goal

A user should be able to turn the active GameTerm pane into the default Scene
Mode workspace with one clear command, then open Scene Mode normally.

Target workflow:

```sh
ci/gameterm-scene-mux-context.sh discover --install --force
```

Then open Scene Mode with the configured keybinding.

## Product End State

This lane is complete when:

1. The active-pane install workflow is documented as a first-class user path.
2. The command collects active mux context when available.
3. The command validates generated Scene JSON before installing.
4. The command installs to the same default scene path used by Scene Mode:
   `~/.config/gameterm/scenes/default.json`, or a supplied `--config-home`.
5. Existing overwrite behavior remains explicit through `--force`.
6. Missing mux context has a clear fallback path and user-facing explanation.
7. Invalid pane cwd fails before install and prints a recovery-oriented error.
8. Deterministic verifier coverage proves install behavior with fixture mux
   context and temporary config homes.
9. Live smoke proves install + launch against a running mux session.
10. Future GUI/keybinding work is scoped as a separate lane, not mixed into the
    shell workflow.

## Non-Goals

- No direct GUI button or command palette action in this lane.
- No automatic Scene Mode launch from the helper.
- No background watcher.
- No process polling loop.
- No command execution based on terminal text.
- No silent overwrite of user scene config.
- No new Scene schema.

## Existing Foundation

Already implemented:

- `ci/gameterm-scene-mux-context.sh discover`
- `--install`, `--config-home`, and `--force` forwarding to
  `ci/gameterm-scene-workspace.sh discover`
- live mux context collection via `gameterm cli --no-auto-start list --format json`
- fixture-backed mux context verification
- named `live-mux-discovery` smoke scenario
- live smoke report with real pane/window/cwd capture

This workflow should reuse those pieces and add a documented, verified product
entrypoint.

## User Workflow Contract

Primary command:

```sh
ci/gameterm-scene-mux-context.sh discover --install --force
```

Safer preview command:

```sh
ci/gameterm-scene-mux-context.sh discover \
  --scene-output /tmp/gameterm-active-pane-scene.json \
  --force
ci/gameterm-scene-author.sh validate /tmp/gameterm-active-pane-scene.json
```

Temporary config command for smoke and demos:

```sh
tmp_home="$(mktemp -d /tmp/gameterm-active-pane.XXXXXX)"
ci/gameterm-scene-mux-context.sh discover \
  --fixture-context ci/fixtures/gameterm-scene/mux-context-active.json \
  --install \
  --config-home "${tmp_home}" \
  --force
ci/gameterm-scene-author.sh validate "${tmp_home}/gameterm/scenes/default.json"
```

Expected result with live context:

- generated default scene uses active pane cwd as discovery cwd
- `pane_context=provided`
- `discovery_source=pane_cwd`
- active pane/window variables are present
- `discovered-pane` metadata records pane id, mux window id, and cwd

Expected result without live context when fallback is allowed:

- user must pass `--allow-missing`
- generated default scene uses normal cwd discovery
- `pane_context=absent`
- no active pane/window variables

## Error And Recovery Contract

Missing mux context:

- without `--allow-missing`: fail before install
- with `--allow-missing`: generate a cwd-based scene and make fallback visible
  in output/docs

Invalid pane cwd:

- fail before install
- do not write `default.json`
- print the invalid path
- recommended recovery: rerun from a valid cwd or use explicit `--cwd PATH`

Existing default scene:

- without `--force`: fail with overwrite protection
- with `--force`: replace default scene after generated scene validation

Invalid generated scene:

- fail before install
- leave existing scene untouched

## Documentation Requirements

Update:

- `docs/gameterm-scene-mode.md`
- `docs/gameterm-scene-onboarding.md`
- `docs/gameterm-scene-product-smoke.md` if a product smoke checklist entry is
  needed
- roadmap links/status

The docs should distinguish:

- preview generation
- install as default scene
- live smoke scenario
- future GUI action

## Verification

Focused deterministic checks:

```sh
bash -n ci/gameterm-scene-mux-context.sh ci/gameterm-scene-workspace.sh

tmp_home="$(mktemp -d /tmp/gameterm-active-pane.XXXXXX)"
ci/gameterm-scene-mux-context.sh discover \
  --fixture-context ci/fixtures/gameterm-scene/mux-context-active.json \
  --install \
  --config-home "${tmp_home}" \
  --force
ci/gameterm-scene-author.sh validate "${tmp_home}/gameterm/scenes/default.json"
```

Negative checks:

```sh
ci/gameterm-scene-mux-context.sh discover \
  --fixture-context ci/fixtures/gameterm-scene/mux-context-invalid-cwd.json \
  --install \
  --config-home "${tmp_home}" \
  --force
```

Expected: non-zero, no installed scene.

Full verifier:

```sh
ci/gameterm-scene-verify.sh --all
```

Live smoke:

```sh
cargo build -p gameterm-gui
ci/gameterm-scene-mux-context.sh discover --install --force
ci/gameterm-scene-smoke.sh --launch --scenario live-mux-discovery
```

Record:

- installed scene path
- active pane id
- mux window id
- pane cwd
- capture path
- fallback status, if any

## Implementation Lanes

### Lane 1: Scope

Deliverables:

- this scope document
- roadmap/live mux links

Verification:

- `git diff --check`

Commit:

- `[docs] scope Scene active pane workflow`

### Lane 2: Install Workflow Verification

Deliverables:

- [x] deterministic verifier coverage for `discover --install --config-home`
- [x] overwrite protection check through the mux helper
- [x] invalid cwd does not install
- [x] fallback install with `--allow-missing`

Verification:

- `ci/gameterm-scene-verify.sh --all`

Commit:

- `[test] verify Scene active pane install workflow`

### Lane 3: User Documentation

Deliverables:

- [x] active pane preview and install commands in Scene Mode docs
- [x] onboarding entry for "open Scene from where I am now"
- [x] recovery notes for missing mux context and invalid cwd

Verification:

- `git diff --check`
- docs command snippets match implemented flags

Commit:

- `[docs] document Scene active pane workflow`

### Lane 4: Live Install Smoke

Deliverables:

- live install to temporary or user-approved config home
- launch/capture through the named smoke scenario
- smoke report entry with active pane/window/cwd

Verification:

- live smoke command succeeds
- screenshot is non-empty
- generated installed scene validates

Commit:

- `[test] record Scene active pane workflow smoke`

### Lane 5: Future GUI Entrypoint Scope

Deliverables:

- separate scope for a GUI action/keybinding such as "Open Scene From Active
  Pane"
- decide whether the GUI action should install, preview, or open a transient
  generated scene

Verification:

- `git diff --check`

Commit:

- `[docs] scope Scene active pane GUI entrypoint`

## Acceptance Checklist

- User can preview an active-pane scene without installing it.
- User can install an active-pane scene as the default Scene Mode scene.
- Install validates before writing.
- Existing default scene is protected unless `--force` is supplied.
- Missing mux context behavior is explicit.
- Invalid pane cwd does not corrupt config.
- Deterministic verifier does not require live GUI/mux.
- Live smoke proves the workflow on a running GameTerm session.
- GUI action remains deferred and separately scoped.

## Recommendation

Implement this as a thin product workflow around the existing mux-context
helper. The helper already owns collection and generation; this lane should
make the install path documented, verified, and safe enough for daily use.
