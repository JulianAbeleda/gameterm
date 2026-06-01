# GameTerm Scene Mode VN Script Import Scope

This document scopes replacing the current Python visual-novel script prototype
with a Rust-native importer owned by `gameterm-visual`.

The current `.rpy` fixture remains an external authoring/source format. Ren'Py
is not a runtime dependency and should not name the product layer. Scene Mode
should parse conservative visual-novel script dialects directly in Rust and emit
native `VisualScene` data.

## Goal

Move the visual-novel script import path from an auxiliary Python prototype to
Rust so the import path is part of the same typed model, validation path, and
test discipline as the rest of Scene Mode.

The user should be able to run:

```sh
cargo run -p gameterm-visual --example scene_vn_script_import -- \
  --source ci/fixtures/gameterm-scene/renpy-demo-source.rpy \
  --output /tmp/gameterm-renpy-demo.json \
  --attribution /tmp/gameterm-renpy-demo-attribution.json \
  --source-dialect rpy \
  --source-title "GameTerm Ren'Py Demo Fixture"
```

and get the same class of output the Python helper currently produces.

## Why Rust

The importer is no longer a throwaway prototype once it becomes part of the
Scene Mode roadmap.

Rust gives us:

- typed import warnings and errors
- direct `VisualScene` construction instead of ad hoc JSON assembly
- direct reuse of `VisualScene::validate`
- normal cargo tests for parsing and output behavior
- easier future reuse by GUI, authoring helpers, and fixture generators
- no Python dependency for the core import path

## Product End State

The Rust import pass is complete when:

1. `gameterm-visual` exposes a Rust VN script importer module.
2. The importer accepts source text/path and import options.
3. The importer returns:
   - `VisualScene`
   - attribution/provenance data
   - structured warnings
4. The first supported dialect matches the current `.rpy` Python prototype:
   - `label`
   - dialogue/say statements
   - narrator lines
   - `menu`
   - menu choices
   - `jump`
   - `default name = literal`
   - `$ name = literal`
   - simple variable guards: `"Choice" if flag:`
5. The generated fixture remains valid and behaviorally equivalent.
6. `vn_script_import` policy metadata is used on generated choices.
7. Legacy `renpy_import` metadata remains accepted for existing fixtures.
8. The open asset catalog remains separate from imported scene data.
9. CI no longer depends on Python for visual-novel script import verification.
10. The old Python helper is deleted or reduced to a compatibility wrapper that
   calls the Rust example.

## Non-Goals

- No full Ren'Py interpreter.
- No full Ink/Yarn interpreter.
- No Python expression execution.
- No screen language.
- No ATL/animation support.
- No audio import.
- No rollback/save-slot parity.
- No download/install of itch.io asset archives.
- No automatic import of DDLC or any proprietary VN.
- No broadened asset vendoring policy.

## Module Shape

Add:

```text
gameterm-visual/src/vn_script_import.rs
gameterm-visual/examples/scene_vn_script_import.rs
```

Export from `gameterm-visual/src/lib.rs`:

```rust
pub use vn_script_import::{
    VnScriptAttributionManifest, VnScriptDialect, VnScriptImportOptions,
    VnScriptImportReport, VnScriptImportWarning, import_vn_script_scene,
};
```

Candidate API:

```rust
pub enum VnScriptDialect {
    Rpy,
}

pub struct VnScriptImportOptions {
    pub dialect: VnScriptDialect,
    pub source_path: Option<PathBuf>,
    pub source_title: String,
    pub source_version: Option<String>,
    pub asset_root: Option<PathBuf>,
}

pub struct VnScriptImportReport {
    pub scene: VisualScene,
    pub attribution: VnScriptAttributionManifest,
    pub warnings: Vec<VnScriptImportWarning>,
}

pub fn import_vn_script_scene(
    source: &str,
    options: VnScriptImportOptions,
) -> Result<VnScriptImportReport, VnScriptImportError>;
```

The importer should construct `VisualScene` directly. JSON serialization should
happen only at the example/CLI boundary.

## Parser Strategy

Use a conservative line-oriented parser for the first Rust pass.

Reasoning:

- the current supported subset is indentation-light and small
- exact engine grammar support is not the goal
- parser behavior should be easy to audit in tests

Rules:

- strip comments outside quoted strings
- track current label
- preserve source line numbers in metadata/warnings
- detect menu choice lines before narrator lines
- associate a menu choice with the next supported `jump`
- warn on unsupported statements rather than guessing
- never evaluate arbitrary expressions

Escalate to a parser crate only when:

- we support nested blocks beyond menu choices
- expression parsing expands past simple literals/guards
- error recovery becomes hard to reason about

## Data Model

### Warnings

Warnings should be structured, not free-form strings.

Candidate:

```rust
pub struct VnScriptImportWarning {
    pub line: usize,
    pub kind: VnScriptImportWarningKind,
    pub detail: String,
}

pub enum VnScriptImportWarningKind {
    UnsupportedStatement,
    UnsupportedAssignment,
    NonMenuJump,
    UnknownJumpTarget,
}
```

The CLI/example can format warnings as:

```text
WARN: line 19: non-menu jump is recorded as source flow only: ending
```

### Attribution

Keep attribution serializable and separate from `VisualScene`.

Candidate:

```rust
pub struct VnScriptAttributionManifest {
    pub source: String,
    pub source_title: String,
    pub source_dialect: String,
    pub source_version: String,
    pub source_path: Option<String>,
    pub asset_root: Option<String>,
    pub license_url: String,
    pub assets: Vec<VnScriptAssetAttribution>,
    pub recommended_open_asset_sources: Vec<VnScriptOpenAssetSource>,
    pub notes: Vec<String>,
    pub warnings: Vec<String>,
}
```

The current `renpy-demo-open-assets.json` remains the source catalog. The
manifest can reference preferred source ids, but should not duplicate the full
catalog unless needed.

## Fixture Strategy

Keep:

- `ci/fixtures/gameterm-scene/renpy-demo-source.rpy`
- `ci/fixtures/gameterm-scene/renpy-demo-open-assets.json`

Regenerate:

- `ci/fixtures/gameterm-scene/renpy-demo.json`
- `ci/fixtures/gameterm-scene/renpy-demo-attribution.json`

The regenerated JSON should remain stable. If formatting changes, use the Rust
serializer consistently and accept the mechanical fixture diff in the import
commit.

## Verification

Required focused checks:

```sh
cargo test -p gameterm-visual vn_script_import
cargo test -p gameterm-visual renpy
cargo run -q -p gameterm-visual --example scene_vn_script_import -- \
  --source ci/fixtures/gameterm-scene/renpy-demo-source.rpy \
  --output /tmp/gameterm-renpy-demo.json \
  --attribution /tmp/gameterm-renpy-demo-attribution.json \
  --source-dialect rpy \
  --source-title "GameTerm Ren'Py Demo Fixture" \
  --source-version fixture
cmp /tmp/gameterm-renpy-demo.json ci/fixtures/gameterm-scene/renpy-demo.json
ci/gameterm-scene-verify.sh --fixture renpy-demo
```

Required full check before push:

```sh
ci/gameterm-scene-verify.sh --all
git diff --check
```

`cargo fmt -p gameterm-visual --check` should remain clean. Repo-wide
`cargo fmt --check` may still report existing unrelated formatting drift.

## CI Changes

Update `ci/gameterm-scene-verify.sh`:

- remove Python syntax checking for `gameterm-scene-renpy-import.py` if the
  Python file is deleted
- call the Rust example in `run_vn_script_import_check`
- compare or validate generated scene/attribution
- keep the open asset catalog jq checks

If a compatibility shell wrapper is kept, verify it with `bash -n`.

## Deletion Strategy

Preferred:

- delete `ci/gameterm-scene-renpy-import.py`
- replace docs references with the Rust example command

Acceptable transitional option:

- replace the Python file with a tiny shell script named
  `ci/gameterm-scene-vn-script-import.sh`
- the shell script calls `cargo run -p gameterm-visual --example
  scene_vn_script_import -- "$@"`

Do not keep two independent import implementations.

## Commit Plan

Use separate commits:

1. `[docs] rename Scene VN script import scope`
2. `[visual] add Scene VN script import model`
3. `[visual] add Scene VN script import example`
4. `[test] regenerate Scene VN demo fixture`
5. `[tools] verify Scene VN script import path`
6. `[docs] document Scene VN script import workflow`
7. `[tools] remove Python VN script importer` if not already deleted in the tools
   commit

If commits 2 and 3 are tightly coupled, they can be combined, but do not mix
docs-only changes with runtime implementation.

## Acceptance Checklist

- Rust importer generates a valid `VisualScene`.
- Rust importer preserves the current fixture behavior.
- Python is no longer the canonical import path.
- Unsupported statements remain warnings, not silent drops.
- No proprietary assets are committed.
- Open asset source policy remains explicit.
- Verifier covers Rust import, fixture setup, doctor validation, and runtime
  traversal.

## Follow-Ups

After this pass:

1. Add a Rust asset-composition helper for CC0/CC BY sprite sources.
2. Decide whether AI-assisted backgrounds are acceptable for demo fixtures.
3. Add local menu visibility or current-label state if global choices become
   confusing.
4. Add a command for installing the generated VN demo into the user Scene
   config.
