# GameTerm Scene Mode VN Demo Install Scope

This document scopes the first pass that turns the Rust VN script importer and
Rust VN asset intake helper into one usable local demo install workflow.

The goal is not to build an app-store installer or download third-party assets.
The goal is to produce a ready-to-open local Scene Mode demo under the user's
Scene config directory with safe overwrite behavior, validation, attribution,
and a clear relationship between script output and optional local art.

## Goal

Make this user workflow possible:

```sh
ci/gameterm-scene-vn-demo.sh install \
  --source ci/fixtures/gameterm-scene/renpy-demo-source.rpy \
  --asset-source-root ~/Downloads/vn-assets \
  --force
```

Expected result:

```text
${XDG_CONFIG_HOME:-~/.config}/gameterm/scenes/
  default.json
  sprites.json
  vn-demo-script-attribution.json
  vn-demo-asset-attribution.json
  vn-demo-bindings.json
  assets/
    vn-demo/
      characters/
      backgrounds/
```

After that, opening Scene Mode should load the installed `default.json` and
resolve any generated sprite IDs through the installed `sprites.json`.

## Product End State

This layer is complete when:

1. A single helper command can install the VN demo into Scene Mode config.
2. The helper uses the Rust `scene_vn_script_import` example for script import.
3. The helper uses the Rust `scene_vn_asset_intake` example for optional local
   assets.
4. The install path validates generated scene JSON before replacing
   `default.json`.
5. Existing files are not overwritten unless `--force` is passed.
6. Existing `sprites.json` is preserved or merged intentionally.
7. Attribution files are written beside the installed scene.
8. Doctor can validate the installed scene and sprite manifest.
9. CI verifies the install path with temporary config and repo-owned fixtures.

## Non-Goals

- No automatic itch.io login or archive download.
- No paid asset handling.
- No committing downloaded third-party art.
- No app bundle or OS-level installer work.
- No full VN engine compatibility.
- No renderer rewrite for VN staging.
- No hidden mutation of user config before validation succeeds.

## Command Shape

Add:

```text
ci/gameterm-scene-vn-demo.sh
```

Commands:

```sh
ci/gameterm-scene-vn-demo.sh generate [OPTIONS]
ci/gameterm-scene-vn-demo.sh install [OPTIONS]
ci/gameterm-scene-vn-demo.sh doctor [OPTIONS]
```

`generate` writes to an explicit output directory and never touches user config.

`install` writes to:

```text
${config_home}/gameterm/scenes/
```

where `config_home` is `--config-home`, `XDG_CONFIG_HOME`, or `~/.config`.

`doctor` runs the existing Scene doctor against the generated or installed
files.

## CLI Options

Common options:

```text
--source PATH                 VN script source. Default: fixture .rpy source.
--source-dialect rpy          First supported dialect.
--source-title TEXT           Source title metadata.
--source-version TEXT         Source version metadata.
--asset-catalog PATH          Open asset catalog JSON.
--asset-source-root PATH      Local extracted asset root.
--output-dir PATH             Required for generate.
--config-home PATH            Config root for install/doctor.
--allow-ai-assisted-assets    Permit catalog entries marked AI-assisted.
--force                       Overwrite existing generated/install files.
--skip-assets                 Install script-only demo.
```

Defaults:

- `--source`: `ci/fixtures/gameterm-scene/renpy-demo-source.rpy`
- `--source-dialect`: `rpy`
- `--source-title`: `GameTerm Ren'Py Demo Fixture`
- `--source-version`: `fixture`
- `--asset-catalog`: `ci/fixtures/gameterm-scene/renpy-demo-open-assets.json`
- `--asset-source-root`: unset; asset intake is skipped unless provided

## Output Layout

Generated output directory:

```text
OUTPUT_DIR/
  default.json
  sprites.json
  vn-demo-script-attribution.json
  vn-demo-asset-attribution.json
  vn-demo-bindings.json
  assets/
    vn-demo/
```

Installed output directory:

```text
${config_home}/gameterm/scenes/
  default.json
  sprites.json
  vn-demo-script-attribution.json
  vn-demo-asset-attribution.json
  vn-demo-bindings.json
  assets/
    vn-demo/
```

The first pass should keep these files separate rather than merging all
attribution into one file. Separate files make it clear which content came from
script import and which content came from asset intake.

## Binding Integration

Add `--bindings PATH` support to `scene_vn_script_import`.

First-pass behavior:

- The importer loads `vn-demo-bindings.json` when provided.
- If `default_background` exists, generated `scene.background` uses it.
- If `characters.guide.expressions.neutral` exists, the narrator/guide entity
  can use that sprite ID.
- If bindings are missing or incomplete, importer keeps existing fallback
  sprite IDs.

The first pass does not need full inline staging support for expressions per
dialogue line. The binding file is the bridge that proves generated scenes can
reference asset-intake sprite IDs.

## Install Safety

Install must be atomic enough to avoid leaving a broken default scene:

1. Generate into a temporary directory.
2. Validate generated `default.json` with `scene_validate`.
3. Validate generated or merged `sprites.json` shape.
4. Run `gameterm-scene-doctor.sh` against the generated files.
5. Refuse to overwrite existing install files unless `--force` is passed.
6. Copy files into the Scene config directory only after validation succeeds.

If validation fails, existing installed files must remain untouched.

## Sprite Manifest Behavior

The installer should support script-only and asset-backed demos.

Script-only:

- install `default.json`
- copy the base fixture `sprites.json` so fallback Scene sprite IDs resolve
- write script attribution
- skip asset attribution/bindings unless explicitly generated

Asset-backed:

- run asset intake with `--base-manifest ci/fixtures/gameterm-scene/sprites.json`
- write generated `sprites.json`
- write generated `vn-demo-bindings.json`
- write asset attribution
- run script import again or run script import after asset intake with
  `--bindings` so `default.json` references generated VN sprite IDs

## Verification

Focused checks:

```sh
bash -n ci/gameterm-scene-vn-demo.sh

ci/gameterm-scene-vn-demo.sh generate \
  --output-dir /tmp/gameterm-vn-demo \
  --asset-source-root ci/fixtures/gameterm-scene/vn-asset-source \
  --force

ci/gameterm-scene-author.sh validate /tmp/gameterm-vn-demo/default.json

ci/gameterm-scene-doctor.sh \
  --scene /tmp/gameterm-vn-demo/default.json \
  --sprites /tmp/gameterm-vn-demo/sprites.json

ci/gameterm-scene-vn-demo.sh install \
  --config-home /tmp/gameterm-vn-demo-config \
  --asset-source-root ci/fixtures/gameterm-scene/vn-asset-source \
  --force
```

Verifier integration:

- add `run_vn_demo_install_check` to `ci/gameterm-scene-verify.sh`
- assert generated `default.json` validates
- assert generated `sprites.json` contains base sprites and VN sprite IDs when
  assets are supplied
- assert generated bindings contain `vn.character.guide.neutral`
- assert attribution files exist
- assert overwrite protection preserves existing files without `--force`
- assert `doctor` reports `0 error(s)`

Full check before push:

```sh
cargo fmt -p gameterm-visual --check
cargo test -p gameterm-visual vn_script_import
cargo test -p gameterm-visual vn_asset_intake
ci/gameterm-scene-verify.sh --all
git diff --check
```

## Docs

Update:

- `docs/gameterm-scene-mode.md`
- `docs/gameterm-scene-roadmap.md`
- `docs/gameterm-scene-onboarding.md` if the workflow becomes a recommended
  onboarding path

Docs should show:

- script-only install
- asset-backed install with local asset source root
- where files are written
- how to undo or overwrite safely
- that no third-party assets are downloaded or committed

## Commit Plan

Use separate commits:

1. `[docs] scope Scene VN demo install flow`
2. `[visual] add Scene VN script bindings`
3. `[tools] add Scene VN demo install helper`
4. `[tools] verify Scene VN demo install`
5. `[docs] document Scene VN demo install workflow`

Do not combine bindings implementation, helper wiring, verifier wiring, and
docs-only changes into one commit.

## Acceptance Checklist

- One command can generate a local VN demo output directory.
- One command can install the VN demo into Scene Mode config.
- The installer refuses overwrites unless `--force` is passed.
- Generated `default.json` validates before install.
- Generated `sprites.json` validates and resolves paths.
- Asset-backed installs can use VN sprite IDs from bindings.
- Script-only installs still work.
- Attribution files are written.
- Existing Scene doctor reports zero errors for generated outputs.
- CI covers generate, install, overwrite protection, and doctor.

## Follow-Ups

After this pass:

1. Improve VN-specific layout/staging so background and portrait art render
   more like a VN scene.
2. Add expression changes per dialogue line.
3. Add sprite-parts composition for CC0 character sources.
4. Add a live smoke scenario that launches the installed VN demo.
5. Decide whether AI-assisted backgrounds are acceptable for local-only demos.
