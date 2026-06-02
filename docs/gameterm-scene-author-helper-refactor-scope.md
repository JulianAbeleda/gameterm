# GameTerm Scene Author Helper Refactor Scope

This document scopes the next Scene Mode refactor pass for
`ci/gameterm-scene-author.sh`.

The goal is to complete the author-helper cleanup in one execution pass while
still landing separate, reviewable commits. "One pass" means the lanes are run
back-to-back until complete or explicitly deferred; it does not mean combining
unrelated changes into one commit.

## Current State

`ci/gameterm-scene-author.sh` is stable and covered by
`ci/gameterm-scene-verify.sh --all`.

It currently owns:

- command parsing
- usage/help text
- fixture installation
- guided template generation
- scene mutation commands
- repeated jq filters
- validation-before-replace behavior
- list-fixtures/list-templates output

The helper is useful, but the file is growing through large shell case blocks
and repeated jq snippets. The verifier already protects the most important
behavior, including failed mutation rollback.

## Principles

This pass must follow the coding principles already captured in
`docs/gameterm-scene-refactor-plan.md`:

- stay inside GameTerm-specific Scene Mode helper code
- keep the shell entrypoint
- do not replace shell/jq with a new framework
- keep command names and option names stable
- preserve Scene JSON schema
- preserve generated template output unless a separate behavior commit is
  explicitly scoped
- preserve validation-before-replace and rollback behavior
- keep each commit scoped and testable

## Non-Goals

- No product feature work.
- No Scene runtime changes.
- No visual redesign.
- No fixture content redesign.
- No schema migration.
- No broad rewrite of the helper into Rust, Python, or another script.
- No command rename.
- No user-facing output changes unless the lane explicitly calls them out.

## Refactor Lanes

### Lane 1: Extract Author Catalog Data

Commit:

```text
[test] NFC - table-drive Scene author catalogs
```

Scope:

- Move fixture names and template names into small catalog files or readonly
  shell arrays near the top of the helper.
- Make `list-fixtures`, `list-templates`, usage text, and validation share the
  same catalog source where practical.
- Keep existing list order stable.

Do not:

- change fixture/template names
- change installed fixture behavior
- change generated template JSON

Focused checks:

```sh
bash -n ci/gameterm-scene-author.sh
ci/gameterm-scene-author.sh list-fixtures
ci/gameterm-scene-author.sh list-templates
ci/gameterm-scene-verify.sh --all
```

### Lane 2: Extract Reusable jq Filters

Commit:

```text
[test] NFC - extract Scene author jq filters
```

Scope:

- Extract repeated jq fragments into named shell functions or filter variables.
- Prioritize repeated patterns:
  - find-or-error by id
  - upsert variable/stat/inventory/quest arrays
  - ensure `.rpg`, `.mode`, `.layers`, and `.variables` defaults
  - condition construction
- Keep the jq execution model simple and inspectable.

Do not:

- create a large external jq library unless repeated inline strings remain a
  demonstrated problem
- change status/output strings
- change generated JSON shape

Focused checks:

```sh
bash -n ci/gameterm-scene-author.sh
ci/gameterm-scene-verify.sh --all
```

### Lane 3: Normalize Typed Value Parsing

Commit:

```text
[test] normalize Scene author typed values
```

Scope:

- Consolidate duplicated boolean/number/text parsing between state values and
  condition values.
- Preserve error messages where possible.
- Keep integer-only number semantics unless a behavior change is separately
  justified.

Expected behavior:

- `--value-bool true|false` still produces `{ "Bool": ... }`
- `--value-number N` still requires an integer and produces `{ "Number": ... }`
- `--value-text TEXT` still produces `{ "Text": ... }`
- condition values keep the same structure.

Focused checks:

```sh
bash -n ci/gameterm-scene-author.sh
ci/gameterm-scene-verify.sh --all
```

### Lane 4: Centralize Mutation Pipeline

Commit:

```text
[test] NFC - centralize Scene author mutations
```

Scope:

- Make all mutating commands route through one named helper that:
  1. writes to a temporary file
  2. validates with the Rust parser
  3. atomically replaces the target
- Preserve the existing `write_json` behavior and rollback guarantees.
- Remove redundant post-write validation calls only if the shared helper already
  proves the same validation happened.

Do not:

- weaken validation
- allow partial writes on failure
- change command output

Focused checks:

```sh
bash -n ci/gameterm-scene-author.sh
ci/gameterm-scene-verify.sh --all
```

### Lane 5: Table-Drive Help Text

Commit:

```text
[test] NFC - table-drive Scene author help
```

Scope:

- Reduce duplication between command list, option sections, and catalog output
  only after catalogs and mutation helpers are stable.
- Keep the rendered `--help` text substantially the same.
- If exact help formatting changes, update docs only in the docs lane.

Focused checks:

```sh
bash -n ci/gameterm-scene-author.sh
ci/gameterm-scene-author.sh --help
ci/gameterm-scene-verify.sh --all
```

### Lane 6: Documentation Closeout

Commit:

```text
[docs] update Scene author refactor status
```

Scope:

- Update `docs/gameterm-scene-refactor-plan.md`.
- Update this scope doc with completed/deferred lanes.
- Update user-facing docs only if help text or examples changed.

Focused checks:

```sh
git diff --check
rg -n "gameterm-scene-author.sh" docs/gameterm-scene-mode.md docs/gameterm-scene-refactor-plan.md
```

## Acceptance Criteria

The pass is complete when:

- `ci/gameterm-scene-author.sh` still exposes the same commands.
- `list-fixtures` and `list-templates` keep the same names and order.
- Existing generated templates validate.
- Existing docs examples still run.
- Failed mutations still leave the original scene file unchanged.
- `ci/gameterm-scene-verify.sh --all` passes after each behavior-sensitive
  lane.
- Each lane has its own commit.

## Stop Conditions

Stop and reassess if:

- a lane requires changing Scene JSON schema
- a lane requires changing command names or option names
- a jq extraction becomes harder to read than the inline filter it replaces
- a focused check passes but `ci/gameterm-scene-verify.sh --all` fails
- the diff becomes a broad rewrite instead of a scoped refactor
- a behavior bug is discovered; fix or scope that separately before continuing

## Recommended Execution Order

1. Lane 1: catalogs.
2. Lane 2: jq filters.
3. Lane 3: typed values.
4. Lane 4: mutation pipeline.
5. Lane 5: help text.
6. Lane 6: docs closeout.

If any lane turns out to have low value or high churn, defer it explicitly in
this document and continue with the next safe lane.
