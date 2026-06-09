# GameTerm Principles Audit Fix Scope

Date: 2026-06-09

## Input Reviews

Two external reviews were checked against the current repo:

- fork-owned Scene/GameTerm surface review
- broader workspace architecture review

The fork-owned review reflects the current repo closely. The broader review
captures real upstream-wide smells, but it mixes fork-owned code with inherited
WezTerm architecture. This pass prioritizes narrow GameTerm-owned fixes that can
land without destabilizing upstream behavior.

## Verified Findings

### True: `gameterm-visual/src/lib.rs` Remains Too Large

Current state:

- `gameterm-visual/src/lib.rs` is about 7.7k lines.
- It still combines schema/data definitions, runtime behavior, rendering text
  helpers, and a large inline test module.
- The recent `asset_edit/` refactor is the model to continue following:
  focused modules, a small facade, and boundary tests.

Risk:

- Full extraction is mechanically large.
- It should land as separate NFC commits, not mixed with behavior fixes.

First-pass scope:

- Keep product behavior unchanged.
- Continue extracting cohesive Scene data/runtime/view slices from `lib.rs`.
- Do not widen public API beyond existing re-exports.

### True: VN Asset `repo_policy` Is Stringly Typed

Current state:

- `VnAssetCatalogSource.repo_policy` is `String`.
- The intake path matches raw strings:
  `allowed_with_provenance`, `allowed_with_attribution`, `local_only`,
  `blocked`.

Fix:

- Add a typed `VnAssetRepoPolicy` enum with serde string compatibility.
- Preserve the existing unknown-policy behavior by representing unknown values
  explicitly and warning/skipping them at intake time.
- Serialize attribution with the same string values as before.

### True: Commit Discipline Is Documented But Not Machine-Enforced

Current state:

- Commit prefix rules live in `structure/Development/coding-principles.md`.
- No tracked script or CI workflow enforces the prefix/NFC shape.

Fix:

- Add `ci/check-commit-message.sh`.
- Add a GitHub workflow that checks commit subjects on push and pull request.
- Support local use as a `commit-msg` hook without requiring tracked `.git`
  hook files.

### Partly True: Workspace-Wide Config/Env/CLI Debt

Current state:

- There are many `std::env` and `config::configuration()` access points across
  inherited upstream crates.
- `gameterm/src/main.rs` and `gameterm-gui/src/main.rs` duplicate parts of the
  CLI surface.

Decision:

- Do not refactor upstream-wide config/CLI in this pass.
- Treat it as a future architectural scope because it risks broad behavior
  changes across mux, windowing, config, SSH, and platform entry points.

## Commit Plan

1. `[docs] scope principles audit fixes`
   - record which review claims are true, partial, and deferred
2. `[visual] encode VN asset repo policy`
   - replace raw `repo_policy` string matching with a typed enum
3. `[ci] enforce GameTerm commit prefixes`
   - add script and workflow for commit subject validation
4. `[visual] NFC - split Scene <slice>`
   - optional first low-risk `lib.rs` extraction if the slice stays mechanical

## Definition Of Done

- The repo-policy invariant is encoded in Rust types.
- Unknown repo policies remain diagnosable rather than silently accepted.
- Commit-prefix rules have a tracked checker and CI workflow.
- Any `lib.rs` extraction is behavior-preserving and separately committed as
  NFC.
- Verification:
  - `cargo test -p gameterm-visual vn_asset_intake`
  - `cargo check -p gameterm-visual`
  - `ci/check-commit-message.sh HEAD~1..HEAD` or equivalent
  - `git diff --check`
