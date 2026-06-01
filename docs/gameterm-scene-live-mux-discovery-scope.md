# GameTerm Scene Live Mux Discovery Scope

This document scopes the next Scene Mode product lane: automatic live
pane/process context from the active GameTerm mux session.

It builds on the completed explicit metadata path in
[Pane And Process Discovery Scope](gameterm-scene-pane-process-discovery-scope.md).

## Goal

Scene Mode should be able to generate or patch a workspace scene using the
currently active GameTerm pane context without requiring the user or caller to
manually pass `--pane-id`, `--mux-window-id`, `--pane-cwd`, and process fields.

The output should still flow through the existing `ci/gameterm-scene-workspace.sh`
contract. Live discovery is a metadata collection layer, not a new scene model.

## Product End State

This lane is complete when:

1. A stable local command can inspect the active mux session and emit Scene
   workspace metadata inputs.
2. Workspace discovery can consume that metadata through the existing explicit
   flags.
3. Missing or unavailable mux context is non-fatal and falls back to current
   behavior.
4. The active pane cwd can become the discovery cwd only when it is available
   and valid.
5. The generated scene exposes `pane_context=provided`, active pane id, mux
   window id, pane cwd, foreground process, and process phase when available.
6. The generated patch path can use the same live metadata source.
7. Deterministic verifier coverage uses fixture mux metadata, not a live GUI.
8. Live smoke covers the real active-pane path when a GUI/mux session is
   available.

## First-Pass Status

Implemented:

- fixture-backed normalized mux context fixtures
- `ci/gameterm-scene-mux-context.sh collect|discover|patch|doctor`
- direct caller and environment collection for active pane/process fields
- `gameterm cli list --format json` collection for active pane id, mux window
  id, and pane cwd
- fallback behavior with `--allow-missing`
- verifier coverage for generated scenes, generated patches, fallback, invalid
  pane ids, invalid pane cwd, CLI list parsing, and caller-style collection

Pending:

- live GUI smoke that records the real active pane id/window id used by Scene
  Mode

## Non-Goals

- No terminal scrollback parsing.
- No shell command inference from visible text.
- No background process polling loop.
- No multi-pane dashboard in this lane.
- No automatic command execution.
- No schema rewrite for pane/process state.
- No dependence on unstable GUI-only state for deterministic CI.

## Existing Foundation

Already implemented:

- `ci/gameterm-scene-workspace.sh inspect|discover|patch` accepts explicit
  pane/process metadata.
- `--pane-cwd` becomes `--cwd` when `--cwd` is absent and the path is valid.
- Generated scenes include `discovered-pane`, `discovered-process`,
  `pane_context`, `active_pane_id`, `active_mux_window_id`, and
  `process_phase`.
- Generated patches update workspace/process metadata and typed
  `process_state`.
- `ci/gameterm-scene-verify.sh --all` covers supplied and missing metadata.

This lane should reuse that foundation rather than adding parallel mapping
logic.

## Proposed Command Shape

The small helper command is:

```sh
ci/gameterm-scene-mux-context.sh collect [OPTIONS]
ci/gameterm-scene-mux-context.sh discover [OPTIONS]
ci/gameterm-scene-mux-context.sh patch [OPTIONS]
ci/gameterm-scene-mux-context.sh doctor [OPTIONS]
```

`collect` emits normalized JSON:

```json
{
  "source": "mux",
  "available": true,
  "pane_id": 231,
  "mux_window_id": 7,
  "pane_cwd": "/Users/julianabeleda/env/gameterm",
  "foreground_process_name": "zsh",
  "foreground_process_path": "/bin/zsh",
  "pane_progress": "None",
  "warnings": []
}
```

`discover` calls `ci/gameterm-scene-workspace.sh discover` with the collected
metadata.

`patch` calls `ci/gameterm-scene-workspace.sh patch` with the collected
metadata.

`doctor` validates only the availability and shape of mux context. It must not
mutate scene config.

## CLI Options

Common options:

```text
--pane-id ID                  Prefer a specific pane when supplied.
--mux-window-id ID            Prefer a specific mux window when supplied.
--pane-cwd PATH               Use active pane cwd from a live caller.
--foreground-process-name TEXT
                              Use active foreground process name.
--foreground-process-path PATH
                              Use active foreground process executable path.
--pane-progress TEXT          Use active pane progress label.
--cwd PATH                    Explicit workspace cwd override.
--scene-output PATH           Forwarded to workspace discover.
--patch-output PATH           Forwarded to workspace patch.
--config-home PATH            Forwarded where install behavior is used later.
--fixture-context PATH        Use fixture JSON instead of live mux lookup.
--cli-list-json PATH          Parse saved gameterm cli list JSON.
--gameterm-bin PATH           gameterm binary for live CLI lookup.
--class CLASS                 Forwarded to gameterm cli lookup.
--prefer-mux                  Forwarded to gameterm cli lookup.
--allow-missing               Succeed with available=false when mux context is absent.
--format json|args            Output normalized JSON or shell args from collect.
--install                     Forwarded only to workspace discover.
--force                       Forwarded only where the downstream command supports it.
```

Defaults:

- `collect` uses live mux context when available.
- when no caller/environment/fixture context is supplied, live collection uses
  `gameterm cli --no-auto-start list --format json`.
- deterministic tests use `--fixture-context`.
- missing live context is a warning for `collect --allow-missing` and a failure
  otherwise.

## Metadata Contract

The helper must normalize live data into the existing workspace flags:

```text
--pane-id
--mux-window-id
--pane-cwd
--foreground-process-name
--foreground-process-path
--pane-progress
```

Rules:

- `pane_id` and `mux_window_id` must be non-negative integers.
- `pane_cwd` must be a local directory before it is forwarded.
- process name/path are optional.
- `pane_progress` is optional and should be copied as a label, not interpreted
  as execution authority.
- if `--cwd` is supplied, the live pane cwd remains metadata only.
- if live mux context is unavailable and `--allow-missing` is supplied, the
  downstream workspace command runs without pane metadata.

## Live Data Source

Preferred implementation order:

1. Use an in-process GUI/overlay caller when Scene Mode is already running and
   the active `TermWindow`/pane metadata is directly available.
2. If a CLI path is needed, use existing mux APIs or PDU support that can list
   panes and resolve active pane/window context.
3. Use fixture context for CI and deterministic verification.

The first implementation can be helper-driven if wiring GUI state would create
too much surface area. The important boundary is that live data must become the
same explicit metadata already accepted by workspace discovery.

## Scene And Patch Behavior

No new scene schema is required.

Generated scene behavior should match the explicit metadata path:

- `pane_context=provided` when live metadata exists.
- `discovery_source=pane_cwd` when no explicit `--cwd` is supplied and pane cwd
  is valid.
- `discovered-pane` metadata includes pane id, mux window id, cwd, and progress.
- `discovered-process` metadata includes foreground process name/path and phase.
- `process_phase=running` only when foreground process or progress data exists.

Generated patch behavior should match the explicit metadata path:

- update workspace metadata with active pane/window/cwd.
- update process entity metadata.
- write typed `process_state` when foreground process/progress exists.

## Verification

Focused deterministic checks:

```sh
bash -n ci/gameterm-scene-mux-context.sh

ci/gameterm-scene-mux-context.sh collect \
  --fixture-context ci/fixtures/gameterm-scene/mux-context-active.json

ci/gameterm-scene-mux-context.sh discover \
  --fixture-context ci/fixtures/gameterm-scene/mux-context-active.json \
  --scene-output /tmp/gameterm-mux-discovered.json

ci/gameterm-scene-author.sh validate /tmp/gameterm-mux-discovered.json

ci/gameterm-scene-mux-context.sh patch \
  --fixture-context ci/fixtures/gameterm-scene/mux-context-active.json \
  --patch-output /tmp/gameterm-mux-context.patch.json
```

Verifier integration:

- add `run_mux_context_check` to `ci/gameterm-scene-verify.sh`
- assert fixture context produces `pane_context=provided`
- assert fixture pane cwd can become discovery cwd when `--cwd` is absent
- assert explicit `--cwd` overrides pane cwd as the workspace root
- assert missing fixture context with `--allow-missing` falls back safely
- assert invalid pane ids and invalid pane cwd are rejected before forwarding
- assert generated patches include typed `process_state` when process metadata
  is available

Live smoke:

- add only after a real GUI/mux caller path exists
- record the active pane id/window id used
- confirm Scene Mode opens with the generated workspace and shows the live pane
  entity/process context

## Implementation Lanes

### Lane 1: Scope And Roadmap

Deliverables:

- this scope document
- roadmap/product completion links

Verification:

- `git diff --check`

Commit:

- `[docs] scope Scene live mux discovery`

### Lane 2: Fixture Context Contract

Deliverables:

- `ci/fixtures/gameterm-scene/mux-context-active.json`
- `ci/fixtures/gameterm-scene/mux-context-missing.json`
- JSON shape documented in fixture README

Verification:

- fixture JSON parses with `jq`

Commit:

- `[test] add Scene mux context fixtures`

### Lane 3: Mux Context Helper

Deliverables:

- `ci/gameterm-scene-mux-context.sh`
- `collect`, `discover`, `patch`, and `doctor`
- fixture-backed mode first
- live lookup isolated behind one function or subcommand branch

Verification:

- `bash -n ci/gameterm-scene-mux-context.sh`
- helper collect/discover/patch checks with fixture context

Commit:

- `[tools] add Scene mux context helper`

### Lane 4: Live Context Source

Deliverables:

- stable live mux collection path
- no-op/missing behavior when no mux session is available
- no terminal output scraping

Verification:

- focused unit/helper checks where possible
- manual local run in an active GameTerm session

Commit:

- `[tools] add Scene live mux context collection`

### Lane 5: Verifier Coverage

Deliverables:

- `run_mux_context_check` in `ci/gameterm-scene-verify.sh`
- scenario coverage for active, missing, invalid, explicit cwd override, and
  patch output

Verification:

- `ci/gameterm-scene-verify.sh --all`

Commit:

- `[test] verify Scene mux context discovery`

### Lane 6: Docs And Smoke

Deliverables:

- Scene Mode docs for live mux context
- roadmap status update
- smoke report entry only after live GUI/mux validation

Verification:

- `ci/gameterm-scene-verify.sh --all`

Commit:

- `[docs] document Scene live mux discovery`
- `[test] record Scene live mux smoke` when live smoke runs

## Acceptance Checklist

- Live discovery is a thin metadata collection layer.
- Existing explicit metadata workflow still works unchanged.
- Missing mux context is non-fatal where explicitly allowed.
- No commands run from discovery.
- No terminal text is parsed.
- Generated scenes validate.
- Generated patches validate.
- Deterministic verification does not require GUI state.
- Live smoke is recorded before marking the live path complete.

## Deferred Work

- multi-pane dashboards
- process tree rendering
- continuous progress polling
- agent-proposed pane/process actions
- GUI command palette integration
- background session monitor

## Recommendation

Implement fixture-backed helper and verifier coverage before attempting live
GUI/mux collection. That proves the product contract and keeps the live
integration small: live code only needs to produce the same normalized metadata
that fixtures already verify.
