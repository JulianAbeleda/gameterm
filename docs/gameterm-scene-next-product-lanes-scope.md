# GameTerm Scene Next Product Lanes Scope

Status: SCOPED.

This document scopes the next product lanes after native smoke reliability is
proved. It consolidates the three candidate lanes from
[Scene Next Actions Scope](gameterm-scene-next-actions-scope.md):

- TTS first pass
- STT first pass
- VN presentation polish

The goal is not to start all three at once. The goal is to make each lane
implementation-ready, define dependencies, and keep commit boundaries clear.

## Entry Gate

Do not start these product lanes until the native Scene smoke path is either:

- passing and recorded, or
- failing with a narrow smoke-reliability defect scoped separately

Entry command:

```sh
cargo build -p gameterm-gui
ci/gameterm-scene-smoke.sh --launch --scenario vn-compose \
  --output /tmp/gameterm-scene-smoke-vn-compose-native.png
```

Entry acceptance:

- Scene opens through the native smoke hook
- capture succeeds or failure is actionable
- current `main` is clean before product work begins

## Lane A: TTS First Pass

Scope owner:

- [Scene Agent TTS Scope](gameterm-scene-agent-tts-scope.md)
- [Scene TTS Setup](gameterm-scene-tts-setup.md)

### End Goal

Scene Mode can speak agent/Codex prose as VN-like dialogue while skipping code,
diffs, logs, JSON, command output, and long machine-oriented text.

The first pass should support:

- conservative speakable segment extraction
- deterministic silent/test backend
- local command backend
- optional VOICEVOX backend if implemented without bundling voices
- playback stop/mute controls
- status reporting for idle, synthesizing, playing, failed

### Product Behavior

When Codex returns a compose reply:

```text
Codex reply -> extract speakable prose -> synthesize -> play or queue audio
```

The user experience should feel like a VN character/agent speaking after the
user chose to submit a compose prompt.

TTS must not:

- read raw terminal scrollback
- read code blocks or diffs aloud
- require a network service by default
- bundle third-party voices
- block Scene input while audio is playing

### Implementation Slices

1. `[gui] add Scene TTS speech extraction model`
   - add `visual_tts.rs`
   - config/request/result/state types
   - speakable extractor
   - tests for prose, code, diff, logs, JSON, long identifiers

2. `[gui] add local Scene TTS backends`
   - built-in silent/test backend
   - local command backend
   - optional VOICEVOX HTTP backend if kept isolated and explicit
   - cache file generation
   - bounded timeout handling

3. `[gui] speak Scene compose replies through TTS`
   - integrate compose success path
   - enqueue speakable segments after successful replies
   - preserve dialogue text and compose behavior
   - surface TTS status without changing Scene runtime schema unless needed

4. `[gui] add Scene TTS playback controls`
   - stop current playback
   - mute/unmute
   - keep controls Scene-local

5. `[docs] document Scene TTS setup`
   - VOICEVOX setup caveats
   - command backend examples
   - voice licensing boundary

### Verification

Focused:

```sh
cargo test -p gameterm-gui visual_tts
cargo check -p gameterm-gui
```

Broad:

```sh
ci/gameterm-scene-verify.sh --all
```

Manual/live:

```sh
ci/gameterm-scene-smoke.sh --launch --scenario vn-compose \
  --output /tmp/gameterm-scene-tts-vn-compose.png
```

### Done Means

- compose prose can produce speakable segments
- code/diff/log filtering is tested
- local backend can produce or fake an audio result
- playback does not block input
- user can stop or mute speech
- docs explain setup and licensing boundaries

## Lane B: STT First Pass

Scope owner:

- [Scene Agent STT Scope](gameterm-scene-agent-stt-scope.md)
- [Scene STT Setup](gameterm-scene-stt-setup.md)

### End Goal

Scene Mode can accept explicit voice input and route the transcript into the
compose dock, without global listening and without bypassing compose policy.

The first pass should support:

- disabled-by-default STT
- Scene-local activation
- local command backend
- transcript sanitization
- insert transcript into compose draft
- optional auto-submit only if explicitly configured

### Product Behavior

When Scene Mode is open:

```text
user starts listening -> local STT command -> transcript -> compose draft
```

The transcript should behave like typed input. It should not execute commands
directly.

STT must not:

- listen globally
- stay active after Scene Mode closes
- silently send audio to a network service
- auto-submit by default
- alter terminal scrollback

### Implementation Slices

1. `[gui] add Scene STT transcript model`
   - add `visual_stt.rs`
   - config/state/result types
   - transcript sanitization
   - compose dock insertion method
   - default disabled

2. `[gui] add Scene STT command backend`
   - structured command parsing
   - helper-owned recording/transcription command
   - bounded runtime
   - stdout transcript, stderr diagnostics
   - failure status

3. `[gui] route Scene voice transcripts into compose`
   - Scene-local key/control
   - channel result polling
   - insert transcript into compose draft
   - optional auto-submit config only if explicitly scoped in code

4. `[docs] document Scene voice input setup`
   - local Whisper/helper examples
   - privacy boundary
   - no bundled model claim

### Verification

Focused:

```sh
cargo test -p gameterm-gui visual_stt
cargo test -p gameterm-gui scene_compose
cargo check -p gameterm-gui
```

Broad:

```sh
ci/gameterm-scene-verify.sh --all
```

Manual/live:

```sh
ci/gameterm-scene-smoke.sh --launch --scenario vn-compose \
  --output /tmp/gameterm-scene-stt-vn-compose.png
```

### Done Means

- STT is disabled unless configured
- a fake/local command backend can return a transcript
- transcript lands in compose draft
- Scene status shows listening/processing/failure
- no microphone or cloud dependency is required for tests
- privacy behavior is documented

## Lane C: VN Presentation Polish

Scope owner:

- [VN Presentation Scope](gameterm-scene-vn-presentation-scope.md)

### End Goal

The VN demo reads visually as a staged VN scene:

- background fills the intended stage
- character sprites are placed at left/center/right slots
- expression changes replace the character by tag
- dialogue and compose panels remain readable
- Tile Debugger exposes active stage state

### Product Behavior

A VN-shaped source like:

```renpy
scene school_classroom
show kiki neutral at center
kiki "Scene Mode has a stage now."
show kiki happy at right
```

should render as a staged background plus character, not as a tiny grid entity.

### Implementation Slices

1. `[visual] add Scene VN stage model`
   - `VisualStage`
   - stage layers/displayables
   - placement enum
   - validation
   - snapshot/debug report state

2. `[visual] import VN scene and show statements`
   - parse `scene`
   - parse `show ... at left|center|right`
   - parse `hide`
   - map bindings to sprite IDs
   - warnings for unsupported transforms

3. `[gui] render Scene VN stage displayables`
   - background viewport
   - character placement rects
   - missing sprite placeholders
   - image-disabled fallback
   - cache identity tied to generation

4. `[test] verify Scene VN staged assets`
   - importer fixture traversal
   - stage snapshot/debug text
   - renderer helper placement tests

5. `[docs] record Scene VN staged smoke`
   - smoke report entry
   - fixture/artifact path
   - caveats

### Verification

Focused:

```sh
cargo test -p gameterm-visual vn_script_import
cargo test -p gameterm-visual stage
cargo test -p gameterm-gui visual_quad --bin gameterm-gui
cargo check -p gameterm-gui
```

Broad:

```sh
ci/gameterm-scene-verify.sh --all
```

Manual/live:

```sh
ci/gameterm-scene-smoke.sh --launch --scenario vn-demo \
  --output /tmp/gameterm-scene-vn-staged.png
```

### Done Means

- generated VN fixture has stage state
- background and character placement are visible in smoke
- expression changes update the visible character tag
- debug output shows active displayables
- existing non-VN Scene rendering still works

## Recommended Order

1. Smoke reliability gate.
2. TTS first pass.
3. STT first pass.
4. VN presentation polish.

Reasoning:

- TTS is lowest risk and strengthens the VN-like agent experience.
- STT is useful but brings microphone/privacy complexity.
- VN presentation improves visuals, but the current staged text/compose work is
  already usable enough to continue agent IO.

If visual readability blocks smoke or user testing, move VN presentation polish
ahead of STT.

## Cross-Lane Rules

- Keep each lane in separate commits.
- Do not mix TTS/STT/VN presentation changes in one commit.
- Do not add default network services.
- Do not bundle voice, model, or third-party art assets without license
  metadata.
- Do not change normal terminal behavior.
- Preserve compose as the execution boundary for agent actions.
- Preserve Scene choices as explicit user decisions.

## Roadmap Done Definition

This product stack reaches the next meaningful checkpoint when:

- native Scene smoke is reliable enough to prove GUI changes
- Codex/agent text can become speakable dialogue through TTS
- explicit user speech can become compose text through STT
- the VN demo can render a stage background and character at readable scale
