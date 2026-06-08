# GameTerm Scene TTS Polish Scope

Status: FIRST PASS IMPLEMENTED.

Date: 2026-06-08

This document scopes the second-pass text-to-speech polish lane for Scene Mode.
The first pass already makes compose replies speak through local TTS backends.
This pass makes that behavior predictable enough to dogfood.

## Goal

Scene Mode should treat speech as an ordered presentation layer over compose
turns.

When Codex replies, GameTerm should:

```text
compose reply
-> readable dialogue blocks
-> cleaned speakable blocks
-> ordered TTS queue
-> no-overlap playback
-> visible state in the debug menu
```

The visible dialogue box should keep useful technical information. The spoken
audio should skip or naturalize technical spans so VOICEVOX does not read raw
paths, URLs, commands, hashes, JSON, logs, or code.

## Current State

Implemented first pass:

- `visual_speech_blocks.rs` extracts speakable compose prose.
- `SpeakableSegment` carries `turn_id`, `block_index`, `speaker`,
  `display_text`, cleaned spoken `text`, `kind`, and `source`.
- `visual_tts.rs` supports disabled, silent, command, and Rust VOICEVOX
  backends.
- VOICEVOX can use a local CTranslate2 helper or direct synthesis.
- The TTS worker serializes backend and player work, so audio blocks do not
  start at the same time.
- Temporary WAV files are deleted after successful playback when a player is
  configured.
- Compose text remains visible even if TTS is delayed or stale.

Known dogfood gaps before this pass:

- visible text appears before audio because synthesis and playback happen
  after the compose result is already rendered
- long replies can still feel like blobs when headings, colon labels, or lists
  are flattened
- display formatting and speech extraction use separate logic, so they can
  disagree on where blocks begin and end
- inline URL/path/command filtering needs stronger regression coverage
- old queued speech cannot be stopped or discarded when the user submits a new
  prompt
- debug output does not expose queue depth, current block, last timings, or
  why a block was skipped
- no explicit TTS test action exists in the central Scene debug menu

## Implementation Status

Implemented commits:

- `6d76dc425 [docs] scope Scene TTS polish`
- `363b0c804 [visual] add Scene dialogue block projection`
- `e53a8efdf [gui] align Scene TTS extraction with dialogue blocks`
- `883385ae6 [visual] add Scene TTS queue diagnostics`

The first pass implements:

- shared dialogue block projection for visible text and TTS extraction
- colon heading, bullet, numbered-list, prose, and technical-line block
  classification
- speakable cleanup coverage for raw URLs, Unix paths, Windows paths, env vars,
  flags, hashes, and command/path-looking spans
- per-request TTS generation ids so stale queued or playing speech cannot mark
  the wrong visible turn/block
- interruption of old TTS work when a new compose turn starts, fake Codex is
  toggled, history is cleared, STT auto-submit starts, or Stop TTS is used
- waited player execution so speech blocks do not overlap
- queue depth, current block, phase, skipped count, errors, and timing summary
  in `Debug -> Voice`
- `Debug -> Voice -> Test TTS playback` and `Stop TTS playback`

The implementation consolidated queue invalidation, stale-event handling,
diagnostics, and debug controls into one behavior commit because they share the
same `VisualOverlaySession` and `SceneTtsState` coordination path.

Remaining follow-up:

- run a live app dogfood pass with real Codex plus VOICEVOX and record the
  text/audio timing result
- decide whether first-block text reveal should be delayed behind a config or
  debug setting; by default, completed Codex text still renders immediately
- add visible highlighting for the currently speaking block if dogfood shows it
  is needed

## Product Contract

TTS should speak:

- assistant prose
- short explanations
- headings when they help the sentence make sense
- short bullet or numbered findings when they are natural language
- Scene dialogue and future agent dialogue

TTS should not speak:

- raw code
- JSON, TOML, YAML, logs, stack traces, diffs, or command output
- full Unix or Windows paths
- URLs
- commit hashes
- env vars and flags
- file extensions as literal words
- long identifier-heavy lines

Visible text should still show useful technical content. Filtering applies to
the spoken copy only.

## Non-Goals

This pass should not:

- add cloud TTS
- bundle voices or voice models
- make VOICEVOX required for tests
- hide completed Codex replies while waiting for audio
- implement streaming TTS
- replace the current VOICEVOX backend with a pure Rust translation model
- redesign the compose backend
- add persistent Codex sessions

## Lane 1: Shared Dialogue Block Projection

Owner prefix: `[visual]`, `[gui]`

Purpose: make the display formatter and TTS extractor agree on block
boundaries.

Current issue:

- `gameterm-visual/src/vn_text.rs` formats visible dialogue.
- `gameterm-gui/src/overlay/visual_speech_blocks.rs` extracts spoken segments.
- They both know about prose, lists, headings, URLs, and technical text, but
  they do not share a block model.

Target behavior:

```text
raw reply
-> SceneDialogueBlock[]
-> visible wrapped lines
-> SpeakableSegment[]
```

Block kinds:

- `Prose`
- `Heading`
- `Bullet`
- `Numbered`
- `TechnicalSkipped`

Formatting rules:

- a short colon heading such as `Plan:` gets a blank line above and below
- markdown headings become standalone heading blocks
- flattened numbered items become separate numbered blocks
- flattened bullets become separate bullet blocks
- raw URLs are displayed as links or visible text, but spoken as "the link" or
  skipped
- technical-only lines are visible only if they belong in the compose reply,
  but they are not sent to TTS

Done means:

- visible text and speakable text come from one block projection
- long structured replies do not collapse into a single blob
- no existing VN transcript behavior regresses

## Lane 2: Speakable Cleanup Second Pass

Owner prefix: `[gui]`

Purpose: make VOICEVOX input natural and bounded.

Actions:

- strengthen inline URL stripping for raw URLs, markdown links, and URLs with
  trailing punctuation
- replace Unix paths with `the project folder` or `that file`
- replace Windows paths with `that folder` or `that file`
- replace command snippets with `the command`
- drop env vars, flags, commit hashes, and pure file-extension noise
- skip whole lines that are mostly code/log/path/JSON
- keep `display_text` unchanged so the dialogue box remains useful

Done means:

- the spoken string never contains raw `http://`, `https://`, `/Users/`,
  `C:\`, `GAMETERM_`, or obvious commit hashes in tests
- cleaned speech still keeps enough prose to understand the reply
- long replies are chunked without dropping text

## Lane 3: Queue State, Stop, And Stale Event Policy

Owner prefix: `[gui]`

Purpose: make playback order and cancellation explicit.

Current behavior:

- one worker serializes synthesis and playback
- each request emits `Started` and `Finished`
- playback waits for `afplay`, so blocks do not overlap
- queued old blocks still exist after a new prompt unless they naturally drain

Target behavior:

- every TTS request carries a queue generation id
- submitting a new compose prompt cancels or invalidates older queued speech
- fake-Codex toggle and dialogue clear invalidate old TTS events
- Stop Speech kills the active player/backend process when possible
- stale events update diagnostics but do not mutate current visible turn state

Default policy:

- new user prompt interrupts old queued or playing assistant speech
- current reply blocks still render immediately
- muted TTS does not enqueue new requests

Done means:

- no stale TTS event can mark the wrong turn/block as speaking
- no old reply keeps talking after the user starts a new prompt
- stop/mute controls are testable without VOICEVOX

## Lane 4: Timing And Debug Diagnostics

Owner prefix: `[gui]`

Purpose: make latency easy to reason about.

Add per-block timings:

- queued timestamp
- synthesis start
- translation duration
- VOICEVOX query duration
- VOICEVOX synthesis duration
- player start
- player duration
- total elapsed

Expose in `Debug -> Voice`:

- backend
- speaker id
- muted/unmuted
- queue depth
- current turn/block
- current phase: queued, synthesizing, playing, done, failed
- last skipped-block reason
- last error
- last timing summary

Done means:

- dogfood can show whether the delay is translation, VOICEVOX synthesis, or
  playback
- the debug menu remains a selectable menu, not hotkey-only text
- fake Codex and real Codex produce the same TTS diagnostics shape

## Lane 5: Text/Voice Synchronization Policy

Owner prefix: `[visual]`, `[gui]`

Purpose: improve perceived sync without reintroducing hidden replies.

Important constraint:

TTS must not be the source of truth for visible dialogue. A delayed voice
worker must never make a completed Codex reply disappear.

First polish target:

- keep all reply text visible once compose succeeds
- mark the currently speaking block visually through compose block state
- optionally delay only the first assistant block reveal behind an explicit
  config/debug setting, not by default
- never gate later turns behind earlier audio

Done means:

- text never disappears because audio is slow
- the speaking block can be identified visually or via debug state
- first-block sync can be tested without breaking future-turn rendering

## Lane 6: Voice Test Action

Owner prefix: `[gui]`, `[docs]`

Purpose: let the user prove TTS from the central debug menu.

Debug menu path:

```text
Debug -> Voice -> TTS test
```

Behavior:

- sends a short fixed prose block through the active TTS backend
- does not require Codex
- shows the same queue, timing, and failure diagnostics as normal compose TTS
- respects mute and stop controls

Done means:

- user can validate VOICEVOX output without typing a Codex prompt
- tests can cover the command path with the silent backend

## Implementation Commit Plan

Suggested commits:

1. `[docs] scope Scene TTS polish`
2. `[visual] add Scene dialogue block projection`
3. `[gui] align Scene TTS extraction with dialogue blocks`
4. `[gui] harden Scene TTS speakable cleanup`
5. `[gui] add Scene TTS queue generation and stop policy`
6. `[gui] expose Scene TTS timing diagnostics`
7. `[gui] add Scene TTS debug test action`
8. `[docs] record Scene TTS polish pass`

If a behavior bug is found while refactoring, stop the NFC-style work and land
the fix as its own behavior commit.

## Verification

Focused tests:

```sh
cargo test -p gameterm-visual vn_text
cargo test -p gameterm-visual staged_scene_splits_flattened_numbered_reply_sections
cargo test -p gameterm-gui visual_speech_blocks
cargo test -p gameterm-gui visual_tts_
cargo test -p gameterm-gui scene_voice_debug
```

Broad checks:

```sh
cargo test -p gameterm-visual
cargo test -p gameterm-gui --bin gameterm-gui
cargo check -p gameterm-gui
git diff --check
```

Manual smoke:

```sh
make dev-app-open
```

Then in the app:

```text
1. Scene Mode + VOICEVOX
Debug -> Voice -> Fake Codex backend
Debug -> Voice -> TTS test
Composer: ask for a short plan with a URL and a file path
```

Expected:

- visible text includes the useful reply
- spoken text does not read the raw URL or path
- blocks play in order
- starting a new prompt stops or invalidates old queued speech
- `Debug -> Voice` shows timing and current queue state

## Definition Of Done

This polish pass is complete when:

- visible dialogue and speakable TTS blocks share the same block boundaries
- colon headings and list indicators format with clear spacing
- URLs and technical spans are not spoken literally
- long replies are chunked without missing text
- queued speech does not overlap and can be stopped
- stale speech events cannot corrupt current turn state
- the debug menu can test TTS without Codex
- latency can be diagnosed from the debug menu
- all focused and broad checks pass

## Stop Conditions

Pause and rescope if:

- text visibility would depend on audio completion
- stop/cancel requires unsafe process handling across platforms
- the shared block projection forces a public Scene JSON schema change
- VOICEVOX-specific behavior starts leaking into generic TTS code
- a latency improvement requires bundling a model or network service
