# GameTerm Scene Mode Ren'Py Demo Scope

This document scopes a first pass for using the bundled Ren'Py demo/tutorial
material as a GameTerm Scene Mode demo.

The goal is not full Ren'Py compatibility. The goal is to prove that Scene
Mode can ingest a real visual-novel-shaped source, preserve its story
structure, respect asset licensing, and expose it as a playable Scene Mode
fixture.

## Goal

Scene Mode should be able to show a recognizable visual novel demo built from
Ren'Py-style source material.

The user should be able to:

1. Import a small, licensed Ren'Py demo/tutorial slice.
2. Generate a valid Scene Mode JSON fixture from that source.
3. Launch the generated scene in Scene Mode.
4. Read dialogue, make choices, and navigate between labels.
5. See asset/license provenance for imported demo material.
6. Verify the import deterministically in CI without depending on a live app
   session.

## Product End State

The first pass is complete when:

1. The repo has a scoped Ren'Py demo importer or fixture generation helper.
2. The importer accepts an explicit Ren'Py project/source path and output path.
3. The importer supports a conservative Ren'Py subset:
   - `label`
   - dialogue/say statements
   - narrator lines
   - `menu` choices
   - `jump`
   - simple boolean/text variable assignment
   - simple variable guards when they can be represented safely
4. The generated Scene Mode file validates through the existing visual runtime.
5. The generated Scene Mode file includes action policy metadata.
6. Asset use is explicit and attribution-preserving.
7. Missing or unsupported assets degrade to placeholders instead of failing the
   whole demo.
8. Unsupported Ren'Py statements are reported as warnings with line/context.
9. CI covers import, validation, doctor, and a focused runtime traversal.
10. User docs explain that this is a Ren'Py demo subset, not a full Ren'Py
    player.

## Non-Goals

- No full Ren'Py interpreter.
- No Python execution.
- No arbitrary expression evaluation.
- No save-slot/rollback parity with Ren'Py.
- No Ren'Py screen-language support.
- No animation/transition/audio parity in the first pass.
- No automatic download of third-party games.
- No import of proprietary or unclear-license assets.
- No app-store/package inclusion of demo assets until license review is
  complete.
- No replacement for Ren'Py as a game engine.

## Source Material

Preferred first source:

- Ren'Py bundled demo/tutorial project material.

Reasoning:

- Ren'Py is the dominant open-source VN engine and its language maps cleanly to
  Scene Mode's dialogue/choice/navigation model.
- The Ren'Py documentation describes first-class `label` and `menu` control
  flow that maps well to Scene Mode labels and choices.
- The Ren'Py license page states that demo artwork is released by various
  copyright holders under the same terms as described there. We still preserve
  attribution because the copyright holders vary.

Required source tracking:

- upstream source URL or local Ren'Py distribution path
- Ren'Py version used for import
- source file path
- asset file paths
- license text or pointer
- attribution/credits file copied or generated beside imported demo output

## License And Asset Boundary

This pass must be conservative.

Rules:

1. Do not commit imported raster/audio assets until the license attribution
   file is present.
2. Do not copy assets from arbitrary Ren'Py games.
3. Only use Ren'Py demo/tutorial assets or assets with clear open licenses.
4. Preserve upstream license files where available.
5. Generate a local attribution manifest for imported assets.
6. Keep generated demo output separate from core bundled Scene Mode sprites
   until we decide packaging policy.
7. CI may use placeholders if assets are absent.

Suggested generated attribution manifest:

```json
{
  "source": "renpy-demo",
  "renpy_version": "8.x",
  "source_path": "/path/to/renpy/the_question/game/script.rpy",
  "license_url": "https://www.renpy.org/doc/html/license.html",
  "assets": [
    {
      "id": "eileen_happy",
      "source_path": "game/eileen_happy.png",
      "output_path": "assets/eileen_happy.png",
      "copyright": "upstream Ren'Py demo attribution",
      "license": "see included license/credits"
    }
  ]
}
```

## Format Mapping

### Ren'Py `label`

Ren'Py:

```renpy
label start:
    "Welcome."
    jump question
```

Scene Mode:

- label becomes a dialogue node or generated scene section
- first label becomes initial runtime position
- `jump` becomes `Navigate` if emitted as a separate scene file, or
  `AdvanceDialogue`/`Resolve` if emitted as one scene with indexed dialogue

First-pass choice:

- Generate one Scene Mode file for the demo slice.
- Keep label ids in metadata.
- Convert label movement into `AdvanceDialogue` where possible.
- Use `Navigate` only when a label becomes a separate file later.

### Dialogue/Say Statements

Ren'Py:

```renpy
e "Hello."
"Narration."
```

Scene Mode:

```json
{
  "speaker": "e",
  "text": "Hello.",
  "metadata": [["source_label", "start"]]
}
```

Rules:

- speaker ids are preserved as raw ids first
- optional character display names can be added later
- unknown speakers remain valid
- narrator lines use speaker `Narrator`

### `menu`

Ren'Py:

```renpy
menu:
    "Go left.":
        jump left
    "Go right.":
        jump right
```

Scene Mode:

- menu choices become `SceneAction` choices
- choice labels preserve user-facing text
- jump target becomes action metadata or runtime operation
- policy origin is `renpy_import`
- risk is `navigate` or `state_change`
- scope is `scene`

Candidate generated choice:

```json
{
  "label": "Go left.",
  "kind": {
    "AdvanceDialogue": {
      "target": 12
    }
  },
  "policy": {
    "origin": "renpy_import",
    "risk": "state_change",
    "scope": "scene",
    "summary": "Continue imported Ren'Py demo at label left"
  }
}
```

### Variables

Supported first pass:

```renpy
$ met_eileen = True
$ route = "left"
```

Scene Mode:

- boolean variables become `VisualStateValue::Bool`
- integer variables become `VisualStateValue::Number`
- string variables become `VisualStateValue::Text`

Unsupported first pass:

- Python expressions
- arithmetic beyond direct literal assignment
- function calls
- list/dict mutations
- Ren'Py store object access

Unsupported variable statements should warn and be skipped unless skipping
would make control flow ambiguous.

### Guards

Supported first pass:

```renpy
"Ask again." if met_eileen:
    jump ask_again
```

Scene Mode:

```json
{
  "conditions": [
    {
      "variable": "met_eileen",
      "equals": { "Bool": true }
    }
  ]
}
```

Unsupported guards:

- compound boolean expressions
- comparisons other than direct truthiness/equality
- function calls

Unsupported guards should warn and either:

- omit the guarded choice if it cannot be represented safely, or
- include it locked with a generated diagnostic variable.

The first implementation should prefer explicit warnings over clever
translation.

### Images And Sprites

Ren'Py image declarations and show/hide statements are not required for the
first pass.

First-pass asset handling:

- detect common image declarations when straightforward
- copy only clearly licensed demo/tutorial assets
- generate sprite ids for characters/background markers when possible
- fall back to existing Scene Mode placeholder sprites when not possible
- preserve source asset path in entity metadata

Scene Mode already has symbolic sprites. The first demo can be playable without
perfect VN staging.

### Audio

Audio is out of scope for the first pass.

Importer should warn on:

- `play music`
- `play sound`
- `stop music`

Do not fail import for audio commands.

### Transitions And Effects

Transitions are out of scope for the first pass.

Importer should warn on:

- `with dissolve`
- ATL blocks
- `show ... at ...`
- image transforms

Do not fail import for transitions.

## Implementation Plan

### Lane 1: Scope And Roadmap

Files:

- `docs/gameterm-scene-renpy-demo-scope.md`
- `docs/gameterm-scene-roadmap.md`

Commit:

- `[docs] scope Scene RenPy demo import`

Acceptance:

- scope defines source, license boundary, subset, helper plan, tests, and
  non-goals
- roadmap points to this document as the next VN demo layer

### Lane 2: Import Helper

Preferred shape:

- add `ci/gameterm-scene-renpy-import.sh`
- keep parsing conservative and line-oriented for the first pass
- emit a complete Scene Mode JSON file
- emit an attribution manifest beside output

Why shell first:

- current Scene Mode helpers are shell + `jq`
- helper can stay fixture-oriented
- avoids committing to a Rust parser before we understand the real subset

Escalate to Rust only if:

- indentation parsing becomes brittle
- expression support expands
- we need reusable parser tests beyond fixture scripts

Proposed command:

```sh
ci/gameterm-scene-renpy-import.sh \
  --source /path/to/renpy-demo/game/script.rpy \
  --asset-root /path/to/renpy-demo/game \
  --output ci/fixtures/gameterm-scene/renpy-demo.json \
  --attribution ci/fixtures/gameterm-scene/renpy-demo-attribution.json
```

Acceptance:

- missing source fails clearly
- invalid output path fails clearly
- unsupported statements produce warnings
- generated scene validates
- attribution file is generated

### Lane 3: Demo Fixture

Files:

- `ci/fixtures/gameterm-scene/renpy-demo.json`
- `ci/fixtures/gameterm-scene/renpy-demo-attribution.json`
- optional `ci/fixtures/gameterm-scene/renpy-demo-source.rpy`

Important constraint:

- If the upstream assets cannot be committed cleanly, the fixture should use a
  tiny checked-in source excerpt plus placeholders, and the docs should explain
  how to run the importer against a local Ren'Py install for full assets.

Acceptance:

- fixture loads with `gameterm-visual`
- fixture contains at least:
  - three dialogue lines
  - one menu
  - two branches or branch targets
  - one variable assignment or guard if source supports it
  - policy metadata on generated choices

### Lane 4: Validation And Doctor

Files:

- `ci/gameterm-scene-doctor.sh`
- `ci/gameterm-scene-verify.sh`

Acceptance:

- doctor reports generated Ren'Py demo scene as valid
- doctor warns if attribution manifest is missing for imported demo assets
- verifier has a focused Ren'Py demo import check
- verifier has a fixture runtime check

Potential doctor checks:

- imported demo scene has `source_engine=renpy`
- imported choices have `policy.origin=renpy_import`
- attribution manifest exists when imported asset metadata is present
- unsupported statement warnings are captured in importer output

### Lane 5: Runtime/Rendering Fit

This lane should be minimal unless the fixture exposes an actual gap.

Possible changes:

- add `renpy_import` as a supported action policy origin
- add metadata rendering for `source_engine`
- improve dialogue label metadata in debug report

Acceptance:

- existing Scene Mode behavior remains unchanged
- generated imported choices render with clear policy summaries
- no full rendering rewrite

### Lane 6: User Docs

Files:

- `docs/gameterm-scene-mode.md`
- `docs/gameterm-scene-onboarding.md`
- optionally `docs/gameterm-scene-product-smoke.md`

Acceptance:

- docs explain how to generate the Ren'Py demo scene
- docs explain that the importer supports a subset
- docs explain asset/license expectations
- docs explain how to launch the generated scene in Scene Mode

### Lane 7: Smoke

Deterministic smoke:

- importer smoke
- visual crate fixture smoke
- doctor smoke

Live smoke:

- optional unless runtime input/rendering changes
- run after the generated scene is installed or selected

Acceptance:

- `ci/gameterm-scene-verify.sh --all` includes Ren'Py demo checks
- live smoke result is recorded only if app-level behavior changes

## Data Model Additions

### Policy Origin

Add `renpy_import` to supported policy origins.

Why:

- imported VN actions are distinct from authored GameTerm scenes, workspace
  discovery, agent proposals, and fixtures

### Scene Metadata

Use existing entity/dialogue metadata where possible.

Recommended scene variables:

```json
[
  {
    "key": "source_engine",
    "value": { "Text": "renpy" }
  },
  {
    "key": "source_title",
    "value": { "Text": "Ren'Py Demo" }
  }
]
```

Recommended entity metadata:

```json
[
  ["source_engine", "renpy"],
  ["source_file", "game/script.rpy"],
  ["source_label", "start"]
]
```

## Verification Checklist

Required before commit:

- `bash -n ci/gameterm-scene-renpy-import.sh`
- `ci/gameterm-scene-renpy-import.sh --help`
- `ci/gameterm-scene-renpy-import.sh ...` against fixture source
- `cargo test -p gameterm-visual renpy`
- `ci/gameterm-scene-doctor.sh --scene ci/fixtures/gameterm-scene/renpy-demo.json`
- `ci/gameterm-scene-verify.sh --fixture renpy-demo`
- `ci/gameterm-scene-verify.sh --all`
- `git diff --check`

If assets are committed:

- verify every asset path appears in attribution manifest
- verify no asset is copied without a license pointer

## Commit Plan

Use separate commits:

1. `[docs] scope Scene RenPy demo import`
2. `[tools] add Scene RenPy import helper`
3. `[visual] add Scene RenPy import policy origin`
4. `[test] add Scene RenPy demo fixture`
5. `[tools] verify Scene RenPy demo import`
6. `[docs] document Scene RenPy demo workflow`

If runtime changes are unnecessary, skip commit 3.

## Risks

### Risk: License Confusion

Mitigation:

- keep attribution manifest required
- use local Ren'Py install paths instead of vendoring assets by default
- only commit placeholder-based fixtures until attribution is clear

### Risk: Parser Overreach

Mitigation:

- explicitly support a tiny subset
- warn on unsupported statements
- do not evaluate Python
- add fixture tests before expanding syntax

### Risk: Scene Mode Looks Like A Bad Ren'Py Clone

Mitigation:

- frame the demo as import/interoperability proof
- keep Scene Mode terminal-native
- preserve symbolic rendering first
- defer VN-accurate staging until the data model proves useful

### Risk: Branching Model Mismatch

Mitigation:

- start with one small branch
- preserve source label metadata
- use existing `AdvanceDialogue`, `Resolve`, and guarded choices before adding
  new runtime concepts

## Definition Of Done

This layer is done when a user can generate and open a Ren'Py-demo-derived
Scene Mode fixture, make at least one VN-style choice, and inspect clear
license/source metadata for the imported material.

At that point, Scene Mode has proven it can host a real visual-novel-shaped
demo without pretending to be a full Ren'Py runtime.
