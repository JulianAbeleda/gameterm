# GameTerm Scene Mode First-Pass Completion Scope

This document scopes the remaining six items that move Scene Mode from
implemented first pass to a fully closed first-pass product baseline.

The goal is not to add new product layers. The goal is to prove the native
active-pane Scene path, remove known sharp edges, consolidate duplicate
generation paths, and leave the roadmap/refactor boundary clear.

## Current Position

The active-pane GUI entrypoint now exists:

- `ShowGameTermActivePaneScene`
- default key: `CTRL|ALT|SHIFT+g`
- transient generated scene
- no write to `~/.config/gameterm/scenes/default.json`
- deterministic command-surface tests
- Rust workspace scene generator in `gameterm-visual`

The remaining work is closure work.

## Implementation Status

Status: implementation complete except live GUI capture evidence.

Closed:

- Rust generated active-pane scenes now use the stable shell-helper contract
  for core variables, pane/window ids, process metadata, and entity ids.
- Expected missing context is recoverable: no active tab produces a visible
  toast, and no active pane opens a small generated error scene.
- Smoke automation has an `active-pane-gui` scenario that sends
  `CTRL|ALT|SHIFT+g`.
- Shell/Rust generator ownership is explicit: shell helpers remain the
  install/persistence path; the Rust generator is the transient in-process GUI
  path.

Still requiring a local interactive pass:

- Run the `active-pane-gui` live smoke capture and append the result to the
  smoke report.

## 1. Live Smoke The Native GUI Path

Purpose: prove the real app path, not just the command definition.

Scope:

- Launch GameTerm from the current build.
- Focus a real pane with a valid cwd.
- Trigger `ShowGameTermActivePaneScene` through the default shortcut or a direct
  GUI action dispatch if automation cannot send `CTRL|ALT|SHIFT+g` reliably.
- Verify the overlay opens and renders a generated active-pane scene.
- Verify debug/source text identifies it as generated active-pane context.
- Verify `Esc`/`q` closes the overlay cleanly.
- Verify normal `ShowGameTermScene` still opens the configured/default scene.

Acceptance:

- Live smoke artifact exists.
- Smoke report records command, date, build commit, output path, and result.
- No tracked files are created or modified by the smoke.
- Failure mode is captured as scoped implementation work, not left ambiguous.

Commit:

- `[test] smoke active pane Scene GUI path`

## 2. Tighten Generator Parity With The Shell Helper

Purpose: prevent the Rust GUI generator and shell workflow from drifting into
two incompatible active-pane scene contracts.

Scope:

- Compare Rust `generate_workspace_scene` output against
  `ci/gameterm-scene-mux-context.sh discover` for a fixture workspace.
- Normalize shared variable names where practical:
  - `workspace_mode`
  - `workspace_root`
  - `active_cwd`
  - `pane_context`
  - `discovery_source`
  - `discovered_file_count`
  - pane/window ids
  - foreground process metadata
- Normalize expected core entity ids where practical:
  - workspace/root/project entity
  - active pane/process entity
  - discovered files entity
- Keep Rust generation bounded and local; do not introduce command execution or
  scrollback parsing.
- Add fixture tests for the stable contract fields.

Acceptance:

- Deterministic tests assert the stable active-pane contract.
- Docs state which fields are stable and which are presentation-only.
- Existing shell helper remains usable.

Commit:

- `[visual] align active pane Scene generator contract`

## 3. Add Visible Recoverable Missing-Context Errors

Purpose: make failure understandable inside the app instead of only returning a
Rust error from action dispatch.

Scope:

- When no active tab/pane exists, open a small recoverable Scene/error overlay
  or otherwise show a visible UI error that does not crash or silently no-op.
- When pane cwd is missing or cannot convert to a local path, fall back only to
  current process cwd with explicit status metadata, or show a recoverable
  generated error scene.
- Keep normal `ShowGameTermScene` behavior unchanged.
- Add tests around the context-to-scene/error decision boundary where possible.

Acceptance:

- Missing active pane produces a visible message.
- Invalid/missing pane cwd produces a visible message or explicit fallback
  status.
- No panic/bail is the only user-visible behavior for expected context gaps.

Commit:

- `[visual] make active pane Scene context errors recoverable`

## 4. Consolidate Shell Helper And Rust Generator Direction

Purpose: decide whether the shell helper remains an install-only path or starts
delegating to the Rust generator.

Scope:

- Audit `ci/gameterm-scene-workspace.sh` and
  `ci/gameterm-scene-mux-context.sh` for fields now duplicated in Rust.
- Choose one first-pass consolidation direction:
  - document the shell helper as install/persistence workflow and Rust as
    transient GUI workflow, with an explicit parity test, or
  - expose a Rust CLI/helper path that the shell scripts can call for scene
    generation.
- Avoid rewriting both helpers unless the selected path requires it.
- Keep install overwrite protections intact.

Acceptance:

- Roadmap names one source-of-truth direction.
- Duplicate fields are either tested for parity or routed through one
  generator.
- Existing helper verification still passes.

First-pass decision:

- `ci/gameterm-scene-workspace.sh` and `ci/gameterm-scene-mux-context.sh`
  remain the install/persistence workflows.
- `gameterm-visual::generate_workspace_scene` is the source for native
  transient GUI previews.
- Shared contract fields are tested in Rust and documented in the active-pane
  entrypoint scope.
- A future refactor may route the shell helpers through a Rust CLI, but that is
  not required for first-pass closure.

Commit:

- `[tools] consolidate active pane Scene generator path`

## 5. Consolidate Roadmap And Status Docs

Purpose: make the current state obvious without reading a long chain of scope
documents.

Scope:

- Update `gameterm-scene-roadmap.md` with:
  - active-pane GUI entrypoint status
  - remaining first-pass closure items
  - what counts as first-pass complete versus refactor backlog
- Update `gameterm-scene-first-pass-scope.md` with a short addendum linking
  this completion scope.
- Update active-pane docs if implementation choices changed during items 2-4.
- Keep historical scope details intact; do not rewrite old docs into fiction.

Acceptance:

- Roadmap answers "where are we now?" in one page.
- First-pass scope points to this closure scope.
- Refactor plan remains explicitly after product closure.

Commit:

- `[docs] consolidate Scene first-pass completion status`

## 6. Plan The Refactor From The Coding Principles

Purpose: prepare the next refactor pass without mixing product fixes and NFC
moves.

Scope:

- Re-read the repo coding principles and current refactor docs.
- Convert the next refactor into small principle-aligned lanes:
  - behavior-preserving only
  - no schema renames unless migration is included
  - no broad crate split
  - no warning cleanup outside touched Scene paths
  - no product features mixed into refactor commits
- Prioritize refactors based on active pain:
  - generator/helper duplication
  - large Scene runtime modules
  - smoke/verifier complexity
  - action dispatch boundaries
- Define checks for each refactor lane.

Acceptance:

- Refactor plan has ordered lanes and explicit non-goals.
- Each lane has a test/check command.
- Product closure commits are complete before refactor commits begin.

Commit:

- `[docs] plan Scene refactor from coding principles`

## Recommended Order

1. Live smoke the native GUI path.
2. Fix visible missing-context behavior if smoke exposes it.
3. Tighten Rust/shell generator contract.
4. Decide and document generator consolidation direction.
5. Consolidate roadmap/status docs.
6. Plan the refactor pass from coding principles.

This order proves the user-facing path first, then cleans the implementation
and planning surface.

## Done Means

The six-item closure pass is complete when:

- the active-pane GUI path has live smoke evidence
- the generated active-pane scene contract is tested
- expected context failures are visible and recoverable
- shell/Rust generator ownership is explicit
- roadmap status is coherent
- refactor work is scoped separately from product completion
