# GameTerm Scene Mode VN Asset Intake Scope

This document scopes the first pass that makes open visual-novel assets usable
inside Scene Mode without making licensing, provenance, or vendoring ambiguous.

The goal is not to turn GameTerm into an asset store or downloader. The goal is
to let a user point GameTerm at approved local asset packs, produce stable Scene
Mode sprite IDs, generate a `sprites.json` manifest, and preserve attribution.

## Goal

Make the current VN demo path able to use real character and background assets
through a local, auditable asset intake workflow.

The user should be able to run a command like:

```sh
cargo run -p gameterm-visual --example scene_vn_asset_intake -- \
  --catalog ci/fixtures/gameterm-scene/renpy-demo-open-assets.json \
  --source-root ~/Downloads/vn-assets \
  --output-root ~/.config/gameterm/scenes/assets/vn-demo \
  --sprite-manifest ~/.config/gameterm/scenes/sprites.json \
  --attribution ~/.config/gameterm/scenes/vn-demo-attribution.json
```

and get:

- copied or composed local asset files under the user config directory
- stable Scene Mode sprite IDs
- a valid `sprites.json` manifest
- attribution/provenance JSON
- warnings for missing, restricted, unclear, or unsupported sources

## Product End State

This layer is complete when:

1. Asset source policy is explicit and checked before files are copied.
2. Approved local asset packs can be mapped into Scene sprite IDs.
3. The generated sprite manifest is valid according to
   `VisualSpriteManifest`.
4. The VN demo scene can reference character and background sprite IDs.
5. Missing assets degrade to placeholders rather than blocking the demo.
6. Attribution is generated beside the manifest.
7. The verifier can test the workflow with repo-safe fixture files.
8. No proprietary, unclear-license, or unapproved asset archive is committed.

## Non-Goals

- No automatic itch.io login or paid download flow.
- No scraping asset pages.
- No committing downloaded archives by default.
- No DDLC or proprietary asset import.
- No full image editor.
- No runtime dependency on an asset service.
- No replacement for the existing Scene sprite manifest model.

## Asset Policy

Use the existing open asset catalog as the policy input:

```text
ci/fixtures/gameterm-scene/renpy-demo-open-assets.json
```

The catalog should become engine-agnostic over time, but it can stay at its
current path until the implementation commit renames fixtures.

Required source fields:

- `id`
- `role`
- `title`
- `author`
- `source_url`
- `download_name`
- `license`
- `license_url`
- `source_disclosure`
- `repo_policy`
- optional `attribution`
- optional `notes`

Supported first-pass `repo_policy` values:

- `allowed_with_provenance`
- `allowed_with_attribution`
- `local_only`
- `blocked`

Default behavior:

- CC0 assets may be copied into local output with provenance.
- CC BY assets may be copied into local output with attribution.
- Local-only assets may be referenced but not committed.
- Blocked assets are skipped.
- Asset provenance and source disclosures are preserved in attribution.

## Asset IDs

Generated sprite IDs should be stable and human-readable.

Recommended first-pass IDs:

```text
vn.background.school_classroom
vn.background.school_hallway
vn.character.guide.neutral
vn.character.guide.happy
vn.character.guide.concerned
vn.character.guide.surprised
```

Rules:

- IDs are lower snake/dot names.
- IDs should describe role and expression, not original file names.
- IDs must be deterministic for the same catalog and source files.
- IDs must not include source author names unless needed to avoid collisions.

## Manifest Output

The generated manifest should use the existing Scene sprite shape:

```json
{
  "sprites": [
    {
      "id": "vn.character.guide.neutral",
      "path": "assets/vn-demo/characters/guide-neutral.png"
    },
    {
      "id": "vn.background.school_classroom",
      "path": "assets/vn-demo/backgrounds/school-classroom.png"
    }
  ]
}
```

The output should be valid for:

```sh
ci/gameterm-scene-doctor.sh \
  --scene ci/fixtures/gameterm-scene/renpy-demo.json \
  --sprites ~/.config/gameterm/scenes/sprites.json
```

## Attribution Output

The generated attribution file should include:

```json
{
  "asset_intake_version": 1,
  "generated_by": "scene_vn_asset_intake",
  "sources": [
    {
      "id": "4cher_set4_vn_sprites",
      "title": "[FREE-TO-USE] Visual Novel Sprites",
      "author": "4cher",
      "source_url": "https://4cher.itch.io/set4-vnsprites",
      "license": "CC-BY-4.0",
      "license_url": "https://creativecommons.org/licenses/by/4.0/",
      "repo_policy": "allowed_with_attribution",
      "used_assets": [
        {
          "sprite_id": "vn.character.guide.neutral",
          "source_path": "BLOND GIRL - SET 4/neutral.png",
          "output_path": "assets/vn-demo/characters/guide-neutral.png"
        }
      ]
    }
  ],
  "warnings": []
}
```

Attribution must be generated even when all assets are local-only. That keeps
the user's local Scene install auditable.

## Local Cache Layout

Use the user's Scene config directory as the default local destination:

```text
~/.config/gameterm/scenes/
  sprites.json
  vn-demo-attribution.json
  assets/
    vn-demo/
      characters/
      backgrounds/
```

Repo fixtures should use temporary output paths during verification. The tool
must not write into the user's real config during CI unless explicitly told to.

## First-Pass Source Handling

The first implementation should support two intake modes:

### Direct Copy

Use when a source pack already contains finished PNG sprites or backgrounds.

Required behavior:

- discover expected files under `--source-root`
- copy only selected files into `--output-root`
- preserve extension if it is supported
- reject unsupported image formats with a warning
- never overwrite existing output unless `--force` is passed

### Composition Placeholder

Use when a source is a sprite-parts pack but composition is not implemented yet.

Required behavior:

- report that the source is approved but needs composition
- emit no broken manifest entries
- keep the demo playable through placeholders

Real composition can be a follow-up if the chosen CC0 character source needs it.

## Scene Binding

The VN demo scene should be able to reference the generated IDs in two places:

- `scene.background`
- entity `sprite`

First-pass binding can be explicit rather than inferred:

```json
{
  "bindings": {
    "default_background": "vn.background.school_classroom",
    "characters": {
      "guide": {
        "neutral": "vn.character.guide.neutral",
        "happy": "vn.character.guide.happy"
      }
    }
  }
}
```

The VN script importer should eventually accept a binding file:

```sh
cargo run -p gameterm-visual --example scene_vn_script_import -- \
  --source ci/fixtures/gameterm-scene/renpy-demo-source.rpy \
  --bindings ~/.config/gameterm/scenes/vn-demo-bindings.json \
  --output ~/.config/gameterm/scenes/vn-demo.json
```

The first asset-intake pass does not need to modify the importer, but the data
shape should be designed so the importer can consume it.

## Rendering Expectations

Scene Mode already renders symbolic sprite IDs through the sprite manifest. The
first asset pass should not require a renderer rewrite.

Expected first visible result:

- background sprite ID resolves to a local image path
- character entity sprite ID resolves to a local image path
- unresolved IDs still show deterministic placeholders
- Tile Debugger shows the selected entity sprite ID and source metadata

If the current normal renderer cannot present large VN background/portrait art
well, document that as a rendering follow-up instead of blocking asset intake.

## Tool Shape

Add:

```text
gameterm-visual/src/vn_asset_intake.rs
gameterm-visual/examples/scene_vn_asset_intake.rs
```

Candidate public API:

```rust
pub struct VnAssetIntakeOptions {
    pub catalog_path: PathBuf,
    pub source_root: PathBuf,
    pub output_root: PathBuf,
    pub force: bool,
}

pub struct VnAssetIntakeReport {
    pub sprite_manifest: VisualSpriteManifest,
    pub attribution: VnAssetAttributionManifest,
    pub bindings: VnAssetBindings,
    pub warnings: Vec<VnAssetIntakeWarning>,
}

pub fn run_vn_asset_intake(
    options: VnAssetIntakeOptions,
) -> Result<VnAssetIntakeReport, VnAssetIntakeError>;
```

The module should use existing `VisualSpriteManifest` validation rather than
creating a separate manifest format.

## Verification

Required focused checks:

```sh
cargo test -p gameterm-visual vn_asset_intake
cargo run -q -p gameterm-visual --example scene_vn_asset_intake -- \
  --catalog ci/fixtures/gameterm-scene/renpy-demo-open-assets.json \
  --source-root ci/fixtures/gameterm-scene/vn-asset-source \
  --output-root /tmp/gameterm-vn-assets \
  --sprite-manifest /tmp/gameterm-vn-sprites.json \
  --attribution /tmp/gameterm-vn-attribution.json \
  --bindings /tmp/gameterm-vn-bindings.json
ci/gameterm-scene-doctor.sh \
  --scene ci/fixtures/gameterm-scene/renpy-demo.json \
  --sprites /tmp/gameterm-vn-sprites.json
```

Required full checks before push:

```sh
cargo fmt -p gameterm-visual --check
ci/gameterm-scene-verify.sh --all
git diff --check
```

## Fixture Strategy

Add tiny repo-owned placeholder image fixtures only if needed for deterministic
tests. Do not add third-party downloaded art to CI until attribution policy and
redistribution permission are explicit in the same commit.

Possible fixture layout:

```text
ci/fixtures/gameterm-scene/vn-asset-source/
  4cher_set4_vn_sprites/
    guide-neutral.png
    guide-happy.png
```

If placeholder files are generated for tests, they should be clearly marked as
GameTerm-owned test assets and not presented as third-party art.

## Roadmap Relationship

This scope sits between two existing layers:

- VN Script Import: creates Scene dialogue/choice state.
- Visual Layout And Assets: renders sprite IDs and validates manifests.

Asset intake is the bridge that makes imported or authored VN scenes use real
images without hardcoding raw file paths into scenes.

## Commit Plan

Use separate commits:

1. `[docs] scope Scene VN asset intake`
2. `[visual] add Scene VN asset intake model`
3. `[visual] add Scene VN asset intake example`
4. `[test] add Scene VN asset intake fixtures`
5. `[test] verify Scene VN asset intake`
6. `[docs] document Scene VN asset workflow`

Do not combine downloaded third-party art with code changes. If third-party art
is ever committed, use a separate commit whose message and diff make license
and attribution explicit.

## Acceptance Checklist

- The asset catalog blocks or warns on restricted sources.
- Approved source files become stable sprite IDs.
- Generated `sprites.json` validates.
- Generated attribution names source, author, URL, license, and output files.
- Existing Scene doctor can validate the generated manifest.
- Missing local assets do not break the demo.
- CI uses only repo-safe fixture assets.
- No proprietary or unclear-license asset file is committed.

## Follow-Ups

After this pass:

1. Add image composition for CC0 sprite-parts sources.
2. Add optional background policy if redistribution requirements change.
3. Teach VN Script Import to consume the generated bindings file.
4. Improve normal view layout for VN-style background plus portrait staging.
5. Add a local installer command that runs script import and asset intake
   together.
