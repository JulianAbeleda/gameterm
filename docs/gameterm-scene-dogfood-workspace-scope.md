# GameTerm Scene Mode Dogfood Workspace Scope

This document scopes the product pass that makes Scene Mode usable as our
normal GameTerm development surface.

The goal is not to add another demo. The goal is to make Scene Mode good enough
to use while working on GameTerm itself.

## Product Goal

Dogfooding is successful when a developer can:

1. launch GameTerm normally
2. choose Scene Mode from the boot menu
3. see a useful live workspace scene for the current GameTerm repo
4. inspect the roadmap, current scope, git state, and verification status
5. run explicit safe actions from Scene Mode
6. keep using the native terminal path when Scene Mode is not needed

The target workflow should feel like:

```text
open GameTerm -> choose Scene Mode -> inspect current work -> run/check/open -> continue in terminal
```

## Current Starting Point

Already available:

- boot menu entries:
  - `1. Scene Mode`
  - `2. Native Terminal Mode`
- default Scene loading from `~/.config/gameterm/scenes/default.json`
- workspace discovery helper with:
  - `inspect`
  - `discover`
  - `patch`
  - `brief`
  - `--install`
  - `--brief-output`
  - `--verify-argv`
  - `--open`
  - pane/process metadata options
- native active-pane Scene action
- compose dock
- local/fake Codex compose backend smoke
- fullscreen smoke scenarios
- author/doctor/verifier coverage

Current gap:

- the generated workspace scene is useful for tests, but it is not yet a
  deliberate daily dogfood surface
- the helper has no named dogfood command/profile
- the smoke harness has no `dogfood` scenario
- the roadmap does not yet treat dogfooding as the next product gate

## Product Contract

Dogfood mode should be explicit and local.

It may:

- generate a Scene Mode workspace from the current repo
- install that workspace as the default Scene scene
- include current git state
- include important Scene roadmap/scope docs
- include explicit `OpenFile` choices
- include explicit `RunCommand` verification choices
- include a task brief file
- launch through the existing boot menu / Scene open path

It must not:

- run commands during discovery
- start Codex automatically
- submit compose prompts automatically
- overwrite config without explicit `--force`
- depend on network access
- hide the native terminal mode
- replace the shell/editor workflow

## End State

The first dogfood pass is complete when this works from the repo root:

```sh
ci/gameterm-scene-workspace.sh dogfood --install --force
```

Then launching GameTerm and choosing Scene Mode should show a generated
dogfood scene that includes:

- repo label: `gameterm`
- current branch
- current short revision
- clean/dirty state and changed-file count
- next high-level Scene work
- important docs:
  - roadmap
  - current dogfood scope
  - onboarding
  - smoke report
  - refactor plan
- explicit actions:
  - open roadmap
  - open current scope
  - open onboarding
  - open smoke report
  - run Scene verifier
  - run focused dogfood smoke
  - run `git status --short`

The scene should validate and doctor before installation.

## Implementation Lanes

### Lane 1: Scope And Roadmap

Commit:

```text
[docs] scope Scene dogfood workspace
```

Scope:

- add this document
- link it from `docs/gameterm-scene-roadmap.md`
- mark dogfood workspace as the next product gate

Checks:

```sh
git diff --check
rg -n "Dogfood" docs/gameterm-scene-roadmap.md docs/gameterm-scene-dogfood-workspace-scope.md
```

### Lane 2: Dogfood Workspace Profile

Commit:

```text
[tools] add Scene dogfood workspace profile
```

Scope:

- add a `dogfood` command or profile to `ci/gameterm-scene-workspace.sh`
- keep existing `discover` behavior stable
- use `discover` internally rather than duplicating generation logic
- set dogfood defaults:
  - `--title "GameTerm Dogfood Workspace"`
  - `--task "Dogfood Scene Mode"`
  - `--max-files` high enough for core Scene docs
  - important `--open` paths for the roadmap/scope/docs
  - `--verify-argv '["ci/gameterm-scene-verify.sh","--all"]'`
  - `--brief-output` when requested or when installing

Suggested command shape:

```sh
ci/gameterm-scene-workspace.sh dogfood \
  --cwd . \
  --scene-output /tmp/gameterm-dogfood.json

ci/gameterm-scene-workspace.sh dogfood --install --force
```

Acceptance:

- generated dogfood scene validates
- install still requires `--force` if the target exists
- `discover` output remains unchanged unless explicitly invoked through
  dogfood profile

Focused checks:

```sh
bash -n ci/gameterm-scene-workspace.sh
ci/gameterm-scene-workspace.sh dogfood --cwd . --scene-output /tmp/gameterm-dogfood.json --force
ci/gameterm-scene-author.sh validate /tmp/gameterm-dogfood.json
ci/gameterm-scene-doctor.sh --scene /tmp/gameterm-dogfood.json
```

### Lane 3: Dogfood Verification Coverage

Commit:

```text
[test] cover Scene dogfood workspace profile
```

Scope:

- add deterministic verifier coverage for the dogfood profile
- assert the generated scene includes:
  - dogfood title
  - git variables
  - key doc choices
  - all-run verifier command
  - task brief relationship when brief output is requested
- assert install overwrite protection remains intact

Focused checks:

```sh
ci/gameterm-scene-verify.sh --all
```

### Lane 4: Dogfood Smoke Scenario

Commit:

```text
[test] add Scene dogfood smoke scenario
```

Scope:

- add `dogfood` to `ci/fixtures/gameterm-scene/smoke-scenarios.psv`
- update `ci/gameterm-scene-smoke.sh` fixture installation for dogfood
- generate a dogfood scene into the smoke config as `default.json`
- keep scenario launch behavior identical to other generated workspace scenes

Expected scenario:

```sh
ci/gameterm-scene-smoke.sh --launch --scenario dogfood \
  --output /tmp/gameterm-scene-smoke-dogfood.png
```

Acceptance:

- `--list-scenarios` includes `dogfood`
- `--describe-scenario dogfood` explains the dogfood workflow
- smoke scenario validates generated output before launch

Focused checks:

```sh
bash -n ci/gameterm-scene-smoke.sh
ci/gameterm-scene-smoke.sh --list-scenarios | grep -qx dogfood
ci/gameterm-scene-smoke.sh --describe-scenario dogfood
ci/gameterm-scene-verify.sh --all
```

### Lane 5: Boot/Menu Dogfood Path

Commit:

```text
[gui] verify Scene boot menu dogfood path
```

Scope:

- confirm the boot menu Scene option loads the configured default scene
- avoid adding a new boot option unless the existing `Scene Mode` entry cannot
  dogfood reliably
- add or adjust tests only if the default-scene path is not currently covered
- preserve `Native Terminal Mode`

Acceptance:

- choosing `1. Scene Mode` opens the installed dogfood default scene
- choosing `2. Native Terminal Mode` remains unchanged
- no automatic command execution happens at boot

Focused checks:

```sh
cargo test -p gameterm-gui boot_menu --bin gameterm-gui
cargo check -p gameterm-gui
ci/gameterm-scene-verify.sh --all
```

### Lane 6: Live Dogfood Smoke And Docs

Commit:

```text
[docs] record Scene dogfood smoke pass
```

Scope:

- run live fullscreen dogfood smoke
- record result in `docs/gameterm-scene-smoke-report.md`
- update onboarding with the dogfood command only after the path is proven

Live command:

```sh
cargo build -p gameterm-gui
ci/gameterm-scene-workspace.sh dogfood --install --force
ci/gameterm-scene-smoke.sh --launch --scenario dogfood \
  --output ~/Desktop/gameterm-scene-smoke-dogfood-$(date +%Y%m%d-%H%M%S).png
```

Acceptance:

- screenshot shows the dogfood workspace scene
- scene can be exited cleanly
- native terminal mode still launches
- smoke report includes exact command and artifact path

## Dogfood Scene Content

The dogfood scene should prioritize daily-use information over decorative
coverage.

Required entities:

- workspace
- project
- active task
- verifier/process
- roadmap doc
- current scope doc
- smoke report
- onboarding doc

Required variables:

- `repo_branch`
- `repo_status`
- `workspace_root`
- `active_task_id`
- `verification_status`
- `dogfood_profile`

Required choices:

- inspect selected entity
- open roadmap
- open dogfood scope
- open onboarding
- open smoke report
- run Scene verifier
- run git status

Optional choices:

- run focused dogfood smoke
- open refactor plan
- open current next-product-lanes doc

## Acceptance Criteria

- Dogfood profile can generate and install a valid default scene.
- Install refuses to overwrite without `--force`.
- Boot menu Scene option opens the installed default scene.
- Native terminal boot option remains available.
- Dogfood smoke scenario is discoverable and documented.
- Full verifier passes.
- Live fullscreen smoke is recorded.
- No automatic command execution, prompt submission, or network access is added.

## Stop Conditions

Stop and scope a defect separately if:

- boot menu Scene mode does not load configured default scenes
- dogfood install overwrites config without explicit `--force`
- generated dogfood choices run commands without activation
- smoke launch captures the wrong app/window
- adding the dogfood profile requires changing Scene JSON schema
- compose/Codex starts automatically

## Completion Definition

The dogfood pass is complete when we can start a normal GameTerm session,
choose Scene Mode, see a useful GameTerm repo workspace, run explicit checks,
open the current docs, exit back to the terminal, and repeat the loop without
manual file setup.
