# GameTerm Scene Mode Stabilization Refactor Scope

This scope follows the first complete Scene Mode product pass. It is a
behavior-preserving stabilization pass, not a second product roadmap.

## Goal

Make the completed Scene Mode first pass easier to maintain and safer to extend
while preserving the current scene schema, helper contracts, fixtures, and user
workflows.

## Coding-Principle Fit

This pass follows `structure/Development/coding-principles.md`:

- keep GameTerm-specific changes narrow and reviewable
- preserve upstream behavior and naming outside Scene Mode
- split functional fixes from NFC refactors
- use `[visual]` for Scene runtime/helper behavior
- use `[test]` for verification-only reshaping
- use `[docs]` for documentation-only changes

Do not use the older `[tools]` prefix for Scene helper changes; Scene helpers
belong to the `[visual]` product surface unless a future principle update says
otherwise.

## Non-Goals

- no schema-breaking scene JSON rename
- no broad Rust crate split
- no app bundle or installer work
- no new product feature beyond clearer workflow/helper structure
- no cleanup of unrelated macOS warning noise

## Refactor Lanes

### Lane 1: Scope And Commit Discipline

Normalize current Scene refactor docs around the active coding principles.
Remove outdated `[tools]` guidance in Scene planning docs where it conflicts
with the current commit discipline.

Status: complete.

Acceptance:

- this scope exists
- docs point helper-owned Scene work at `[visual]` or `[test]`
- no behavior changes

### Lane 2: Verification Structure

Make the verifier easier to extend without changing what it checks.

Status: complete.

Targeted cleanup:

- table-drive static shell checks
- table-drive doc-onboarding required references
- keep output labels stable

Acceptance:

- `bash -n ci/gameterm-scene-verify.sh`
- `ci/gameterm-scene-verify.sh --all`

### Lane 3: Helper Common Patterns

Reduce duplicated shell patterns only where the move is obvious and low-risk.

Status: complete.

Candidate patterns:

- overwrite-protection checks
- temp-file cleanup
- JSON validation snippets
- repeated fixture/template registries

Acceptance:

- no changed helper CLI syntax
- no changed generated JSON shape
- focused helper smoke plus full verifier

### Lane 4: Runtime Module Cleanup

Only continue if a small module move is clearer than the current shape.

Status: planned.

Candidate areas:

- relationship display helpers
- selected-entity summary formatting
- runtime status helper grouping

Acceptance:

- NFC commit only
- `cargo test -p gameterm-visual`
- full Scene verifier

### Lane 5: Live Smoke Audit

After deterministic refactors, run the documented onboarding/smoke path and
record any product issues separately from NFC cleanup.

Status: planned.

Acceptance:

- smoke report updated only when live smoke actually runs
- issues become scoped follow-up work, not mixed into refactor commits

## First Implementation Slice

The first slice should complete lanes 1 and 2:

1. add this scope
2. normalize stale commit-prefix language in Scene refactor docs
3. table-drive verifier static-script and onboarding-doc checks
4. run the full verifier

This is intentionally narrow and should be enough to prove the stabilization
workflow without creating churn.
