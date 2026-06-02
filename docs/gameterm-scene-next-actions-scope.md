# GameTerm Scene Next Actions Scope

Status: SCOPED.

This document scopes the immediate next execution lane after the Scene compose
and Codex approval-policy work.

The goal is to keep the next steps concrete:

1. prove the native Scene smoke launch hook,
2. record or fix the smoke result,
3. then choose the next product lane with a clean verification baseline.

## Current Position

The current pushed head is:

```text
244330e60 [gui] default Scene Codex approval to on-request
```

Scene Mode now has:

- stabilized compose backend split
- native Scene smoke launch hook
- Codex compose backend using `approval_policy="on-request"` by default
- TTS and STT scoped, but not implemented

The next risk is not another feature. The next risk is whether the live GUI
smoke path can reliably prove Scene changes without keyboard-shortcut focus
automation.

## Lane 1: Native Scene Smoke Validation

Purpose: prove the current GUI launch hook in a real macOS GUI session.

Command:

```sh
cargo build -p gameterm-gui
ci/gameterm-scene-smoke.sh --launch --scenario vn-compose \
  --output /tmp/gameterm-scene-smoke-vn-compose-native.png
```

Expected result:

- GameTerm launches from the current build.
- Scene Mode opens through the native smoke hook, not the keyboard shortcut.
- The `vn-compose` fixture renders.
- Capture produces a non-empty PNG.
- Failure messages identify host permission, process, focus, or capture issues.

Acceptance:

- live smoke either passes and is recorded, or fails with a scoped defect
  written down before implementation
- no product feature work starts before this is resolved

Commit if it passes:

```text
[docs] record Scene native smoke pass
```

Commit if it needs a fix:

```text
[test] improve Scene smoke reliability
```

or, if the defect is in GUI launch behavior:

```text
[gui] fix Scene smoke launch hook
```

## Lane 2: Smoke Evidence Or Fix

If the smoke passes, update `docs/gameterm-scene-smoke-report.md` with:

- date
- command
- scenario
- output path
- current commit
- result
- host caveats, such as Screen Recording or Accessibility requirements

If the smoke fails, keep the fix narrow:

- no renderer redesign
- no Scene schema changes
- no broad launch lifecycle refactor
- preserve normal GameTerm boot behavior
- keep manual `--no-auto-open-scene` fallback

Verification after a fix:

```sh
bash -n ci/gameterm-scene-smoke.sh
cargo check -p gameterm-gui
ci/gameterm-scene-verify.sh --all
```

Live verification:

```sh
ci/gameterm-scene-smoke.sh --launch --scenario vn-compose \
  --output /tmp/gameterm-scene-smoke-vn-compose-native.png
```

## Lane 3: Product Lane Decision

After smoke reliability is stable, choose exactly one product lane:

### Option A: TTS First Pass

Scope owner:

- [Scene Agent TTS Scope](gameterm-scene-agent-tts-scope.md)

Why first:

- gives Codex/agent replies a VN-like voice layer
- does not require microphone permissions
- can start with silent/test and command backends

First commits:

```text
[gui] add Scene TTS model and extractor
[gui] add Scene TTS command backend
[test] cover Scene TTS speakable filtering
[docs] record Scene TTS first pass
```

### Option B: STT First Pass

Scope owner:

- [Scene Agent STT Scope](gameterm-scene-agent-stt-scope.md)

Why first:

- lets spoken requests feed the compose dock
- moves Scene Mode toward voice-controlled agent steering

Tradeoff:

- microphone permissions and recording behavior are more complex than TTS

### Option C: VN Presentation Polish

Scope owner:

- [VN Presentation Scope](gameterm-scene-vn-presentation-scope.md)

Why first:

- improves the visible VN staging experience
- helps prove background/character readability before voice features

Tradeoff:

- does not advance agent input/output functionality

## Recommendation

Recommended order:

1. Native Scene smoke validation.
2. Smoke report or smoke reliability fix.
3. TTS first pass.
4. STT first pass.
5. VN presentation polish if visual staging still blocks usability.

Rationale: TTS is the lowest-risk voice feature because it can be implemented
with deterministic tests and no microphone capture. It also matches the VN
direction: agent/Codex prose becomes spoken dialogue after the user makes an
explicit Scene/compose choice.

## Stop Conditions

Stop and rescope if:

- live smoke cannot open Scene Mode through the native hook
- smoke requires changing normal GameTerm startup behavior
- TTS requires a network service by default
- STT requires always-on listening
- a product feature needs broad renderer, mux, or terminal protocol changes

