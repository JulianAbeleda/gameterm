# GameTerm Scene Mode VN Config Module Scope

Status: first-pass implemented.

This scope defines the ownership boundary for the current Kiki visual-novel
demo. The Kiki sprites, school backgrounds, VN scene JSON, sprite manifest,
layout overrides, and voice/compose settings are Scene Mode content. They
should live in user config as an installable module, not in core engine code or
inside the macOS app bundle.

The engine should stay generic. The VN demo should be a replaceable local
module.

## Goal

Make this the durable runtime model:

```text
GameTerm engine
  owns: runtime, renderer, input, validation, doctor, smoke harness

VN demo config module
  owns: Kiki art, school backgrounds, default VN scene, sprites.json,
        VN layout overrides, compose config, voice config, attribution
```

Expected installed layout:

```text
${XDG_CONFIG_HOME:-~/.config}/gameterm/
  scene-compose.json
  gameterm.lua
  scenes/
    default.json
    sprites.json
    vn-overlay-layout.json
    vn-demo-bindings.json
    vn-demo-asset-attribution.json
    vn-demo-script-attribution.json
    assets/
      vn-demo/
        characters/
          kiki-neutral.png
          kiki-happy.png
          kiki-concerned.png
          kiki-surprised.png
          kiki-blink-0.png
          ...
        backgrounds/
          school-classroom.png
          school-hallway.png
```

Opening the installed app should load this config module. Rebuilding the app
should not silently replace or corrupt it.

## Why This Matters

The user-facing problem this fixes:

- GameTerm was rebuilt correctly.
- The app bundle was installed correctly.
- Scene Mode still failed because the local user scene had stale policy data:
  `policy.origin = "vn_script_import"`.

That failure was not a renderer problem and not an app bundle problem. It was a
stale config module problem. The install/update path needs to validate the
module before the user sees a failure frame.

## Product End State

The first-pass end state is:

1. A single VN module helper owns install, update, doctor, backup, and smoke
   for the local VN demo config.
2. App rebuild/install never rewrites VN content directly.
3. The helper validates `default.json`, `sprites.json`, asset files, layout
   config, compose config, and voice config before replacing active files.
4. Existing user files are backed up before forced replacement.
5. Stale schema values are either migrated explicitly or rejected with a clear
   diagnostic.
6. Smoke tests can run against the installed config module, not only temporary
   fixture config.
7. Docs clearly say that Kiki/school/VN assets belong to config and are not
   part of the core GameTerm engine.

First-pass implementation:

- `ci/gameterm-scene-vn-demo.sh` owns generate, install, update, doctor,
  backup, and smoke entrypoints for the local VN demo module.
- The helper uses the existing Rust VN asset intake path to copy approved
  local assets into config-owned `assets/vn-demo` content and generate
  `sprites.json`, bindings, and attribution.
- `doctor` validates the active module scene, sprites, strict PNG assets,
  optional layout config, and optional compose config before launch.
- `update` explicitly migrates stale `policy.origin = "vn_script_import"` to
  `authored`, after creating a timestamped backup.
- `ci/gameterm-scene-verify.sh --all` covers install, stale-origin failure,
  migration, backup, strict image validation, and manifest checks.

## Non-Goals

- No committing downloaded third-party assets into the repo.
- No automatic asset downloads or itch.io scraping.
- No bundling Kiki/school assets into `GameTerm.app`.
- No changing the engine's fallback bundled scene.
- No hidden rewrite of user config during app launch.
- No renderer changes unless validation exposes a rendering bug.
- No replacing the existing scene JSON model with a separate VN engine format.

## Module Helper

Added dedicated helper:

```sh
ci/gameterm-scene-vn-demo.sh
```

Required commands:

```sh
ci/gameterm-scene-vn-demo.sh install [OPTIONS]
ci/gameterm-scene-vn-demo.sh update [OPTIONS]
ci/gameterm-scene-vn-demo.sh doctor [OPTIONS]
ci/gameterm-scene-vn-demo.sh backup [OPTIONS]
ci/gameterm-scene-vn-demo.sh smoke [OPTIONS]
```

The helper is intentionally explicit. It does not run during app launch, and it
does not replace user config unless the user runs `install --force` or
`update`.

## Command Semantics

### install

Installs a full VN module into config.

Rules:

- Generate into a temporary directory first.
- Validate all generated files.
- Refuse to overwrite existing files unless `--force` is passed.
- If `--force` is passed, back up replaced files first.
- Copy into `${config_home}/gameterm/scenes` only after validation succeeds.

### update

Updates an existing VN module in place.

Rules:

- Preserve user-owned local assets unless replacement is explicitly requested.
- Apply known migrations, such as stale policy origins.
- Validate after migration.
- Keep a timestamped backup of every changed file.
- Do not touch unrelated Scene Mode config.

### doctor

Validates the installed module.

Required checks:

- `default.json` is valid Scene Mode JSON.
- `sprites.json` is valid and covers all referenced sprite IDs.
- referenced sprite files exist.
- strict image mode confirms PNG files are real image data.
- `vn-overlay-layout.json` parses if present.
- `scene-compose.json` parses if present.
- voice config is diagnosable without requiring voice services to be running.
- unsupported policy origins are reported with suggested replacements.

### backup

Creates a timestamped module backup.

Backup target:

```text
${config_home}/gameterm/scenes/backups/
```

or another documented directory if that is already the repo convention.

### smoke

Runs a local first-pass smoke against the installed config module.

Required behavior:

- Launch the installed app or target debug binary.
- Enter Scene Mode.
- Capture a screenshot.
- Fail if the screenshot contains the Scene Mode load-error frame.
- Record the screenshot path.

This command can delegate to `ci/gameterm-scene-smoke.sh` if it can pass the
installed config through cleanly.

## Migration Rules

This pass should explicitly handle known stale config values.

Known migration:

```text
policy.origin: "vn_script_import" -> "authored"
```

Reason:

- `vn_script_import` was retired with the old import lane.
- Current supported origins are:
  `authored`, `workspace_discovery`, `agent`, `runtime`, `fixture`, `unknown`.

Migration constraints:

- Migration must be visible in command output.
- Migration must create a backup first.
- Migration must not run automatically during app launch.
- Migration must be covered by a fixture test.

## Repo-Owned Versus User-Owned Files

Repo-owned:

- engine/runtime code
- renderer and layout primitives
- validation rules
- author/doctor/smoke helper scripts
- docs
- small legal test fixtures
- fallback bundled default scene

User-owned config module:

- Kiki sprites
- school backgrounds
- local asset attribution
- user `default.json`
- user `sprites.json`
- VN layout tuning file
- compose config
- voice config
- downloaded/exported local art

This distinction should be reflected in docs and helper output.

## Test Conditions

Unit and helper tests:

```sh
bash -n ci/gameterm-scene-vn-demo.sh
ci/gameterm-scene-vn-demo.sh doctor --config-home /tmp/gameterm-vn-config
ci/gameterm-scene-vn-demo.sh update --config-home /tmp/gameterm-vn-config --dry-run
ci/gameterm-scene-verify.sh --all
```

Fixture coverage:

- valid installed module passes doctor.
- missing `default.json` reports clear error.
- invalid `default.json` reports the exact validator error.
- stale `vn_script_import` origin is migrated to `authored`.
- missing sprite file is reported with sprite id and resolved path.
- fake `.png` text file fails with `--strict-images`.
- update without `--force` refuses destructive replacement.
- update with `--force` creates backups.
- app rebuild does not modify user config files.

Smoke coverage:

```sh
ci/gameterm-scene-vn-demo.sh smoke \
  --config-home "${XDG_CONFIG_HOME:-${HOME}/.config}" \
  --output /tmp/gameterm-vn-config-module-smoke.png
```

Acceptance:

- screenshot shows school background, Kiki sprite, dialogue panel, composer
  dock, and voice indicator.
- screenshot does not show `Scene file failed to load`.
- doctor reports `0 error(s), 0 warning(s)` before smoke.

Latest verification:

- `bash -n ci/gameterm-scene-vn-demo.sh ci/gameterm-scene-verify.sh`: PASS
- `git diff --check`: PASS
- `ci/gameterm-scene-vn-demo.sh doctor --config-home ~/.config
  --strict-images`: PASS on the current local config
- `ci/gameterm-scene-verify.sh --all`: PASS

## Implementation Lanes

### 1. Module Doctor Tightening

Status: complete.

Commit prefix: `[tools]`.

Work:

- add config-module doctor entrypoint or extend the VN demo helper doctor
- validate scene, sprites, assets, layout, compose, and voice config
- report stale policy origins with migration suggestions

Done when:

- the stale `vn_script_import` case is caught and explained before launch
- installed module doctor passes on the current local config

### 2. Config Migration And Backup

Status: complete.

Commit prefix: `[tools]`.

Work:

- implement backup command
- implement explicit migration command/update path
- cover `vn_script_import -> authored`
- keep user assets untouched unless requested

Done when:

- migration creates a backup
- migrated scene validates
- no app launch side effects are needed

### 3. Installed Config Smoke

Status: first-pass implemented.

Commit prefix: `[test]`.

Work:

- add a smoke path that uses installed config, not temporary fixtures only
- capture output path
- fail on load-error frame when possible
- record manual fallback steps if screen automation is unavailable

Done when:

- the helper can prove the clicked app path is not stuck on a broken scene

### 4. Docs And Roadmap

Commit prefix: `[docs]`.

Work:

- update roadmap with the config-module ownership rule
- update onboarding with the install/update/doctor flow
- explain that Kiki/school assets are user config module content
- record the current stale-origin fix as an example diagnostic

Done when:

- future rebuilds have a clear checklist:
  rebuild app, validate module, smoke installed config

## Definition Of Done

This scope is complete when:

- VN demo content lives under config as the authoritative module.
- Rebuilding/reinstalling the app does not modify VN module files.
- A helper can doctor and migrate the installed VN module.
- The stale policy-origin failure cannot surprise the user after rebuild.
- Installed-app smoke confirms Scene Mode loads the configured Kiki/school VN
  scene.
- Docs make the module ownership boundary explicit.
