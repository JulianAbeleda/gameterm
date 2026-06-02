# GameTerm Scene Mode VN Real Assets Run Scope

Status: COMPLETE.

This scope turns the current VN demo from a wiring/provenance pass into a
runnable visual asset pass.

The current VN path works end to end for script import, asset intake metadata,
manifest generation, install safety, and doctor validation. The gap is that the
repo fixture "PNG" files were placeholder text fixtures. This pass made the
result honest: fixture assets are real PNG files, real local image files render
in Scene Mode, and fake image placeholders fail validation when the caller asks
for image validation.

## Goal

Make this workflow reliable:

```sh
ci/gameterm-scene-vn-demo.sh generate \
  --output-dir /tmp/gameterm-vn-demo-run \
  --asset-source-root PATH_TO_APPROVED_LOCAL_ASSETS \
  --force

ci/gameterm-scene-vn-demo.sh doctor \
  --output-dir /tmp/gameterm-vn-demo-run \
  --strict-images

ci/gameterm-scene-smoke.sh \
  --launch \
  --scenario vn-demo \
  --output /tmp/gameterm-scene-vn-demo.png
```

Expected result:

- `default.json` references VN sprite IDs.
- `sprites.json` references real copied PNG files.
- attribution records the local asset source policy.
- doctor rejects text placeholders when strict image checks are enabled.
- smoke opens the generated VN demo and captures it.

## Current Baseline

Already implemented:

- Rust VN script importer.
- Rust VN asset intake helper.
- VN demo generate/install/doctor helper.
- binding file that maps guide expressions to generated sprite IDs.
- overwrite protection.
- attribution files.
- verifier coverage using repo-safe placeholder assets.

Closed limitation:

- `ci/fixtures/gameterm-scene/vn-asset-source/.../*.png` are now real
  GameTerm-owned PNG fixture images.
- `ci/gameterm-scene-doctor.sh --strict-images` proves manifest assets are PNG
  image data.
- `ci/gameterm-scene-smoke.sh --scenario vn-demo` generates, strict-validates,
  opens, drives, and captures the VN demo.

## 1. Real Local VN Assets

Purpose: make the VN demo visually meaningful without committing unclear or
proprietary art.

Scope:

- Keep the source policy local and explicit. No automatic download, scraping,
  paid asset access, DDLC import, or archive vendoring.
- Accept a user-provided extracted asset root through the existing
  `--asset-source-root` path.
- Document the expected local layout for approved sources in the catalog.
- Support at least the current guide expressions:
  - `vn.character.guide.neutral`
  - `vn.character.guide.happy`
- Optionally support a non-AI background only if the catalog has an approved
  source and local file mapping.
- Keep missing optional expressions as warnings, not hard failures.
- Keep generated paths under `assets/vn-demo/...`.
- Preserve attribution/provenance for every copied asset.
- Flatten local PSD/image downloads into the existing source-root layout with
  `ci/gameterm-scene-vn-image-export.sh` when the source provides layered art
  instead of directly runnable PNG sprites.

Acceptance:

- Running `scene_vn_asset_intake` with real PNG files copies those files into
  the output tree.
- `sprites.json` contains VN sprite IDs pointing at copied real images.
- `default.json` uses `vn.character.guide.neutral` for the guide/narrator
  entity when bindings exist.
- Script-only mode still works without assets.
- No third-party asset archive or unclear-license file is committed.

Commit:

- `[visual] run VN demo with real local assets`

Verification:

```sh
cargo test -p gameterm-visual vn_asset_intake

ci/gameterm-scene-vn-image-export.sh \
  --source PATH_TO_LOCAL_CHARACTER.psd \
  --output-source-root /tmp/gameterm-vn-source-root \
  --force

ci/gameterm-scene-vn-demo.sh generate \
  --output-dir /tmp/gameterm-vn-demo-real \
  --asset-source-root /tmp/gameterm-vn-source-root \
  --force
```

The export helper writes `4cher_set4_vn_sprites/guide-neutral.png`,
`guide-happy.png`, `guide-concerned.png`, and `guide-surprised.png` by default
because that is the current catalog shape used by the VN demo. It intentionally
does not commit or vendor the downloaded PSD/image source.

## 2. Strict Image Validation

Purpose: stop placeholder text files from being treated as runnable visual
assets.

Scope:

- Add an opt-in strict image check to `ci/gameterm-scene-doctor.sh`.
- Suggested flag: `--strict-images`.
- For every sprite manifest entry, resolve the path exactly as doctor already
  does.
- Require real PNG decode for strict checks.
- Prefer a lightweight existing local capability:
  - use the `file` command first if available and require `PNG image data`
  - optionally add a Rust-side image decode check later if renderer needs the
    same guarantee
- Keep normal doctor behavior compatible: existing file-existence checks remain
  the default so older workflows do not break.
- Teach `ci/gameterm-scene-vn-demo.sh doctor` to forward `--strict-images`.
- Add a negative fixture test that proves text placeholders fail under strict
  image validation.

Acceptance:

- `doctor --strict-images` passes for real PNG assets.
- `doctor --strict-images` fails for current placeholder text `.png` fixtures.
- non-strict doctor keeps passing for existing repo-safe fixture workflows.
- failure messages identify the bad sprite id and resolved path.

Commit:

- `[test] validate Scene sprite image files`

Verification:

```sh
bash -n ci/gameterm-scene-doctor.sh
bash -n ci/gameterm-scene-vn-demo.sh

ci/gameterm-scene-vn-demo.sh doctor \
  --output-dir /tmp/gameterm-vn-demo-real \
  --strict-images

ci/gameterm-scene-vn-demo.sh doctor \
  --output-dir /tmp/gameterm-vn-demo-run \
  --strict-images
# expected failure while /tmp/gameterm-vn-demo-run uses placeholder text assets
```

## 3. VN Demo Smoke Scenario

Purpose: prove the generated VN demo opens in the actual GUI path.

Scope:

- Add `vn-demo` to `ci/gameterm-scene-smoke.sh --list-scenarios`.
- Add `--describe-scenario vn-demo`.
- Generate the VN demo into the smoke script's temporary Scene config before
  launching.
- Support two modes:
  - fixture mode: use repo-safe placeholder assets and non-strict doctor
  - real-asset mode: accept a local asset root and run strict image doctor
- Suggested smoke option:
  - `--vn-asset-source-root PATH`
- Keep capture behavior the same as other scenarios.
- Add a short key sequence for the imported choices, for example
  `enter,j,enter`.
- Record live smoke output in `docs/gameterm-scene-smoke-report.md`.

Acceptance:

- `ci/gameterm-scene-smoke.sh --describe-scenario vn-demo` explains the flow.
- fixture mode launches without requiring third-party assets.
- real-asset mode validates real PNGs before launch.
- live capture shows the VN imported demo in Scene Mode.
- smoke writes only temporary config unless explicitly told otherwise.
- locally exported PSD/image sprites can be supplied through
  `--vn-asset-source-root`.

Commit:

- `[test] smoke Scene VN demo assets`

Verification:

```sh
bash -n ci/gameterm-scene-smoke.sh
ci/gameterm-scene-smoke.sh --describe-scenario vn-demo

ci/gameterm-scene-smoke.sh \
  --launch \
  --scenario vn-demo \
  --wait-before-capture 3 \
  --capture-timeout 8 \
  --output /tmp/gameterm-scene-vn-demo.png
```

With real local art:

```sh
ci/gameterm-scene-smoke.sh \
  --launch \
  --scenario vn-demo \
  --vn-asset-source-root PATH_TO_APPROVED_LOCAL_ASSETS \
  --wait-before-capture 3 \
  --capture-timeout 8 \
  --output /tmp/gameterm-scene-vn-demo-real.png
```

## Done Means

This pass is complete when:

- the VN demo can be generated with real local PNG assets
- strict image validation catches placeholder/fake image files
- a `vn-demo` smoke scenario opens and captures the demo
- docs clearly distinguish fixture placeholder mode from real-asset mode
- no unclear-license or proprietary assets are committed

## Non-Goals

- No automatic asset downloads.
- No itch.io scraping or login automation.
- No DDLC or proprietary content import.
- No renderer redesign for full VN staging.
- No expression changes per dialogue line unless already represented by the
  imported scene/bindings model.
- No committing user-downloaded third-party art in this pass.
