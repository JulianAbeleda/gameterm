# GameTerm Scene Fresh Install Hardening Scope

Status: scoped from Air fresh-install dogfood on 2026-06-12.

## Problem

A cask-installed GameTerm can reach the full Scene Mode voice and compose suite,
but the first-run path is not reliable enough. The Air setup required manual
repair across app validity, portable assets, Compose workspace, Whisper model,
VOICEVOX reachability, and macOS window restoration. These are install and
onboarding defects, not user workflow defects.

The target experience is:

1. Install GameTerm.
2. Open it.
3. See the Scene shell boot screen.
4. Use terminal and Scene/VN rendering immediately.
5. Get explicit status and one-command setup for optional Codex, STT, and TTS.

## Observed Fresh-Install Failures

The Air dogfood found these concrete issues:

- macOS reported `GameTerm.app` as damaged because the bundle had quarantine
  state and invalid signing resources.
- `~/.config/gameterm/scenes/sprites.json` contained machine-local absolute
  paths under `/Users/julianabeleda/env/gameterm/...`.
- primitive scene assets were not present in the config asset tree.
- `scene-compose.json` pointed at `/Users/julianabeleda/env/gameterm`, but that
  workspace did not exist.
- Codex itself was installed and authenticated, but Compose looked off because
  the configured workspace was invalid.
- STT could not work until the Whisper model was downloaded to
  `~/Library/Caches/gameterm/scene-stt/models/ggml-base.en.bin`.
- TTS could not work until VOICEVOX was reachable at `127.0.0.1:50021`.
- macOS state restoration could reopen stale windows and hide the fresh boot
  path.
- `ci/gameterm-scene-doctor.sh` covered scene/sprite authoring but not product
  readiness for app, Compose, STT, TTS, or first-run shell behavior.

## Product Contract

Fresh install should be self-diagnosing and progressively functional.

Required behavior:

- Base terminal, Scene shell, and bundled Scene/VN rendering work with only the
  installed app.
- Missing optional subsystems do not make the app feel broken.
- Scene shell shows explicit readiness for:
  - Codex Compose
  - voice input / STT
  - voice output / TTS
  - translation helper
- Every missing subsystem has a precise remediation command or in-app action.
- Generated config contains no absolute paths to a developer machine.
- Restarting the app after install reaches the same first-run state; macOS
  restoration must not obscure the boot screen.

## Non-Goals

- Bundle VOICEVOX itself in the GameTerm app.
- Bundle the Whisper model in the base app.
- Require a GameTerm source checkout for regular users.
- Make Codex mandatory for Scene Mode.
- Hide optional setup behind opaque automatic network downloads.
- Replace existing Scene authoring helpers in this pass.

## Implementation Lanes

### Lane 1: Release App Validity

Goal: installed macOS app opens without Gatekeeper repair.

Work:

- Ensure release packaging produces a valid app signature layout.
- Confirm cask installation does not leave a broken-signature/quarantine
  combination.
- Add release verification:
  - `codesign --verify --deep --strict --verbose=4 GameTerm.app`
  - `spctl -a -vv GameTerm.app` for release artifacts
  - `open GameTerm.app` smoke on a clean machine or CI runner
- Keep `ci/install-macos-dev-app.sh` ad-hoc signing for local development, but
  separate dev signing from release notarization expectations.

Acceptance:

- A downloaded/cask-installed app does not trigger the macOS "damaged" dialog.
- `gameterm --version` and `open GameTerm.app` both work immediately after
  install.

### Lane 2: Portable Default Scene Install

Goal: Scene/VN visuals work without a repo checkout or manual asset copy.

Work:

- Move the required default Scene sprite assets into a distributable app or
  config seed location.
- Seed `~/.config/gameterm/scenes/` only when files are absent.
- Generate `sprites.json` with paths relative to the manifest directory.
- Preserve user-owned config; never overwrite without backup or explicit
  consent.
- Add a config migration that rewrites known bad absolute paths when they point
  at the GameTerm repo asset tree and an equivalent bundled/config asset exists.

Acceptance:

- A fresh install has no sprite warnings for the bundled/default Scene.
- `ci/gameterm-scene-doctor.sh --strict-images` passes for seeded config.
- `sprites.json` contains no `/Users/.../env/gameterm` paths.

### Lane 3: Compose Workspace Defaults

Goal: Codex Compose can be enabled without a source checkout, and failure is
legible when Codex is not ready.

Work:

- Change default `scene-compose.json` generation so `codex_workspace` points to
  a directory that exists on fresh install, for example:
  `~/.local/share/gameterm/compose-workspace`.
- Initialize that workspace as a minimal Git repo if Codex requires trusted Git
  context.
- Keep source-dogfood config supported for developers who want
  `~/env/gameterm`.
- Add readiness checks:
  - `codex_bin` exists and is executable
  - `codex --version` succeeds
  - workspace exists
  - workspace is acceptable to `codex exec`
  - auth failure is reported distinctly from missing binary/workspace
- Consider a GameTerm-side fallback that invokes Codex with
  `--skip-git-repo-check` only for the generated non-source workspace if that
  matches the desired trust model.

Acceptance:

- Fresh install reports Compose as "ready" when Codex is installed and signed in.
- Fresh install reports "Codex not installed", "Codex not signed in", or
  "Compose workspace missing" with exact remediation when not ready.
- Compose does not silently point at a missing developer path.

### Lane 4: STT First-Use Setup

Goal: voice input setup is discoverable and verifiable.

Work:

- Add an STT readiness check:
  - expected Whisper model exists and is non-empty
  - model file size is plausible
  - default microphone device is visible
  - microphone permission state is reported when available
- Add a setup command that downloads the model to the expected cache path:
  `~/Library/Caches/gameterm/scene-stt/models/ggml-base.en.bin`.
- Surface a first-use prompt or Scene status action for installing the model.
- Keep network download explicit unless product policy allows automatic
  download.

Acceptance:

- Missing model produces a clear Scene status and a copy/paste setup command.
- After setup, doctor reports STT ready.
- First mic use either works or shows the macOS permission requirement.

### Lane 5: TTS / VOICEVOX Readiness

Goal: voice output setup is obvious whether using local VOICEVOX or a tunnel.

Work:

- Add a TTS readiness check:
  - `127.0.0.1:50021/version` reachable
  - `audio_query` succeeds for configured speaker
  - `synthesis` produces non-empty WAV bytes
  - `afplay` exists
- Surface configured host/port/speaker in Scene status.
- Add guidance for supported setup modes:
  - local VOICEVOX desktop/engine
  - SSH tunnel to another machine
  - disable TTS
- Optionally store a known tunnel command in config without auto-starting it.

Acceptance:

- If VOICEVOX is unavailable, Scene shows "TTS unavailable" without blocking
  text flow.
- If endpoint is available, doctor proves WAV synthesis.
- Speaker-id failures are reported separately from connection failures.

### Lane 6: Unified Doctor

Goal: one command explains the whole installed Scene stack.

Work:

- Extend `ci/gameterm-scene-doctor.sh` or add a product-level wrapper such as
  `gameterm doctor`.
- Keep scene/sprite authoring checks, and add install-readiness sections:
  - App bundle/signature/quarantine on macOS
  - config file loadability
  - shell boot/backdrop config
  - seeded default Scene files
  - sprite asset resolution
  - Codex Compose
  - STT
  - TTS
  - translation helper
  - running GameTerm process/socket sanity
  - macOS state restoration warning if it can obscure first launch
- Output must be task-oriented:
  - `OK`
  - `WARN`
  - `ERROR`
  - `FIX`
- Add `--json` for GUI consumption.
- Add `--repair-safe` for deterministic safe repairs:
  - create missing directories
  - seed default config when absent
  - rewrite known bad asset paths
  - create compose workspace
  - remove stale generated temp files
- Keep unsafe/network repairs explicit:
  - download Whisper model
  - start VOICEVOX
  - create SSH tunnel
  - install Codex

Acceptance:

- A fresh install can run one doctor command and see every missing layer.
- `--repair-safe` can fix portable config and workspace issues without touching
  user-authored files.
- JSON output can drive an in-app readiness panel later.

### Lane 7: First-Run UX

Goal: the app explains readiness from inside the Scene shell.

Work:

- On first Scene shell open, show a compact readiness strip or settings screen:
  - Terminal: ready
  - Scene assets: ready/missing
  - Compose: ready/setup needed
  - Voice input: ready/model missing/permission needed
  - Voice output: ready/VOICEVOX unreachable
- Add actions that either run safe repairs or copy/show the exact command.
- Ensure macOS saved-state restoration does not bypass the intended first-run
  readiness surface. Options:
  - disable restorable windows for the app
  - always open Scene shell after restored window creation
  - show readiness when Scene is opened manually with `Ctrl+G`

Acceptance:

- Opening the installed app on a fresh machine explains what is usable now.
- Users can still enter Native Terminal without completing optional setup.
- Reopen/restored-window paths do not hide setup forever.

### Lane 8: Verification Matrix

Goal: fresh install reliability is tested, not anecdotal.

Add deterministic test fixtures for these machine states:

1. No user config, no Codex, no Whisper model, no VOICEVOX.
2. Config present, scene assets missing.
3. Config present with absolute repo paths.
4. Codex installed but workspace missing.
5. Codex installed and signed in, generated workspace exists.
6. Whisper model missing.
7. Whisper model present.
8. VOICEVOX unreachable.
9. VOICEVOX reachable but bad speaker id.
10. VOICEVOX reachable and synthesis succeeds.
11. macOS app has quarantine/signature issue.
12. macOS app valid and first launch opens Scene shell.

Verification commands:

- `ci/gameterm-scene-doctor.sh --strict --strict-images`
- new `gameterm doctor --json`
- `gameterm show-keys | rg ShowGameTermScene`
- `codex exec` smoke from configured workspace when Codex is installed
- Whisper model cache existence check
- VOICEVOX version/audio_query/synthesis smoke
- macOS `codesign`, `spctl`, and `open` checks for packaged artifacts

## Suggested Milestones

### Milestone 1: Stop Broken First Launches

- Fix app signing/quarantine release path.
- Seed portable default Scene assets.
- Prevent missing developer-path config from shipping.
- Extend doctor with app/config/asset checks.

### Milestone 2: Make Optional Capabilities Legible

- Add Compose/STT/TTS readiness checks.
- Add generated compose workspace.
- Add clear in-Scene unavailable statuses.
- Add setup docs that match doctor output exactly.

### Milestone 3: One-Command Repair

- Add `--repair-safe`.
- Add Whisper model install helper to doctor output.
- Add VOICEVOX tunnel/local-engine guidance.
- Add JSON output for future GUI readiness.

### Milestone 4: Release Gate

- Add fresh-machine smoke checklist to release process.
- Run package validation on every cask artifact.
- Require doctor clean or known-warning-only status before publishing.

## Done Definition

This work is complete when a clean macOS user can install GameTerm, open it,
see the Scene shell, use the default Scene/VN visuals, and run one doctor
command that either reports Codex/STT/TTS ready or gives exact setup actions.
No first-run path should require private knowledge of `~/env/gameterm`,
absolute developer paths, manual code signing repair, or hidden SSH tunnel
state.
