# GameTerm Scene Agent STT Scope

Status: SCOPED.

This document scopes first-pass speech-to-text for Scene Mode agent control.
It complements `gameterm-scene-agent-tts-scope.md`: STT turns explicit user
speech into compose input, and TTS speaks selected agent prose back.

The product goal is voice control for Scene agents, not ambient terminal
listening. GameTerm should only capture microphone input during a visible,
user-triggered listening session.

## Goal

When Scene Mode is open, the user should be able to press a key or command,
speak a request, and have the transcript submitted through the existing compose
path.

Example flow:

```text
press-to-talk
-> record microphone audio
-> STT backend
-> transcript
-> compose dock draft or submit
-> Codex/Claude compose backend
-> optional TTS reply
```

The first pass should keep voice input local to Scene Mode and reuse the
compose backend contract that already exists.

## Product Contract

STT should:

- require explicit activation
- show a visible listening/processing status
- feed transcripts into the compose dock or submit them through compose
- preserve keyboard compose behavior
- avoid capturing audio when Scene Mode is closed
- stop recording on explicit release/cancel/timeout

STT should not:

- listen globally by default
- read or alter terminal scrollback directly
- execute commands without passing through compose/agent policy
- silently send microphone audio to a network service
- remain active after Scene Mode closes

## Control Model

First pass should prefer Scene-local controls over app-global controls.

Recommended first-pass controls:

- `v`: toggle a bounded Scene listening session when compose dock is focused
- `Esc`: cancel listening if active, otherwise keep current Scene close behavior
- optional `Enter`: submit transcript after it lands in the compose dock

Deferred:

- global push-to-talk shortcut
- hold-to-talk modifier chord
- wake word
- always-on voice activity detection

This intentionally differs from Clicky. Clicky uses a global macOS
push-to-talk shortcut because it lives across all apps. GameTerm should start
with a Scene-local input because the terminal already has a focused interaction
surface.

## Fit With Existing Architecture

Scene compose already owns the user-to-agent path:

- `SceneComposeDock` edits and submits text
- `spawn_compose_backend` runs configured assistant backends
- compose replies update Scene dialogue
- backend state is visible in the Scene overlay

STT should feed that path. It should not introduce a parallel agent command
system.

Recommended new module boundary:

```text
gameterm-gui/src/overlay/visual_stt.rs
```

`visual_stt.rs` should own:

- STT configuration from environment
- recording/session state
- STT backend request/result types
- local command backend execution
- transcript sanitization
- timeout/cancel handling
- unit tests for config, transcript cleanup, and command construction

`visual.rs` should only:

- route Scene key input into STT state
- insert or submit completed transcripts through `SceneComposeDock`
- render STT status in the existing Scene status area

## Backend Strategy

### Built-In Silent Backend

Default behavior should be disabled/silent:

- no microphone capture
- no external service
- no network
- deterministic tests without audio hardware

STT should become active only when explicitly configured.

### Command Backend

Add a generic local command backend first.

Suggested environment:

```text
GAMETERM_SCENE_STT_BACKEND=command
GAMETERM_SCENE_STT_COMMAND='my-stt-helper --input {input}'
GAMETERM_SCENE_STT_TIMEOUT_SECONDS=20
```

Backend contract:

```text
input file: recorded audio path
stdout: transcript text
stderr: diagnostics
exit 0: transcript succeeded
nonzero: STT failed and status is updated
```

This backend can support local Whisper, whisper.cpp, mlx-whisper, or a user
script without making GameTerm own model downloads in the first pass.

Command parsing should use structured argv parsing, matching compose backend
discipline.

### Apple Dictation/Speech Backend

On macOS, Apple Speech is a reasonable later first-party backend candidate, but
it touches platform permissions and framework-specific code. Scope it after the
command backend proves the Scene control loop.

### Cloud Backend

AssemblyAI, OpenAI, Deepgram, and similar services should be deferred. They are
useful, but they introduce API keys, policy, cost, privacy, and networking
concerns.

If added later, cloud STT must be explicit in config and visible in Scene
status.

## Recording Strategy

First pass can avoid native microphone capture if the command backend records
itself.

Two acceptable paths:

1. Helper-owned recording:
   - GameTerm starts the command when listening begins.
   - GameTerm cancels/kills the command on cancel/timeout.
   - The helper returns transcript on stdout.

2. GameTerm-owned recording:
   - GameTerm records bounded audio to a temp file.
   - GameTerm passes the file to the command backend.
   - This requires platform audio integration and should be scoped separately
     if it grows.

Recommended first pass: helper-owned recording. It is narrower and preserves
upstream behavior.

## Transcript Handling

Completed transcripts should be treated like typed compose input.

First pass behavior:

- trim whitespace
- strip obvious transcription artifacts
- cap transcript length
- insert transcript into compose dock as a draft
- optionally auto-submit only when explicitly configured

Suggested environment:

```text
GAMETERM_SCENE_STT_AUTO_SUBMIT=false
```

Default should be draft insertion, not auto-submit. This gives the user a
chance to correct bad transcripts before sending them to Codex/Claude.

## Runtime State

Candidate state:

```rust
pub enum SceneSttPhase {
    Disabled,
    Idle,
    Listening,
    Processing,
    Succeeded,
    Failed,
}

pub struct SceneSttState {
    pub phase: SceneSttPhase,
    pub last_transcript: Option<String>,
    pub last_error: Option<String>,
}
```

Status examples:

```text
Voice idle
Voice listening
Voice processing
Voice transcript ready
Voice failed: command timed out
```

## Implementation Slices

### Slice 1: STT Model And Transcript Path

Files:

- `gameterm-gui/src/overlay/visual_stt.rs`
- `gameterm-gui/src/overlay/mod.rs`
- `gameterm-gui/src/overlay/visual.rs`

Work:

- add STT config/state/result types
- add transcript sanitization
- add a method on `SceneComposeDock` to insert transcript text
- keep default STT disabled
- add unit tests for transcript cleanup and compose insertion

Verification:

```sh
cargo test -p gameterm-gui visual_stt
cargo test -p gameterm-gui scene_compose
cargo check -p gameterm-gui
```

Commit:

```text
[gui] add Scene STT transcript model
```

### Slice 2: Command Backend

Files:

- `gameterm-gui/src/overlay/visual_stt.rs`

Work:

- add structured command backend parsing
- spawn helper-owned recording/transcription command
- bound command runtime
- collect stdout/stderr without blocking Scene input
- report success/failure through STT state

Verification:

```sh
cargo test -p gameterm-gui visual_stt
cargo check -p gameterm-gui
```

Commit:

```text
[gui] add Scene STT command backend
```

### Slice 3: Scene Input Integration

Files:

- `gameterm-gui/src/overlay/visual.rs`
- `gameterm-gui/src/overlay/visual_stt.rs`

Work:

- route `v` into STT start/cancel when compose dock is focused
- receive completed transcripts through a channel
- insert transcript into compose dock
- optionally auto-submit when configured
- render STT status without changing Scene dialogue text

Verification:

```sh
cargo test -p gameterm-gui visual_stt
cargo check -p gameterm-gui
ci/gameterm-scene-verify.sh --all
```

Commit:

```text
[gui] route Scene voice transcripts into compose
```

### Slice 4: Docs And Manual Smoke

Files:

- `docs/gameterm-scene-mode.md`
- optional `ci/gameterm-scene-voice-smoke.sh`

Work:

- document command backend setup
- document local Whisper/helper examples without bundling models
- document privacy behavior
- add a deterministic fake command smoke path if practical

Verification:

```sh
ci/gameterm-scene-smoke.sh --describe-scenario vn-compose
```

Commit:

```text
[docs] document Scene voice input setup
```

## Risks

### Risk: Voice Input Executes Too Much

Mitigation:

- transcripts land in compose draft by default
- auto-submit is opt-in
- compose/agent policy remains the execution boundary

### Risk: Privacy Boundary Is Unclear

Mitigation:

- no default backend
- visible listening status
- Scene-local activation first
- cloud backends deferred and explicit

### Risk: Native Audio Expands Scope

Mitigation:

- command backend can own recording first
- native mic capture is a later scoped pass
- no platform framework changes in the initial model slice

### Risk: Bad Transcripts Pollute Agent Context

Mitigation:

- draft insertion by default
- transcript length cap
- user can edit before submit

## Out Of Scope

- global always-on microphone listener
- wake word
- voice activity detection
- native cross-platform audio capture
- cloud STT backend
- voice biometric identification
- direct command execution from transcript
- reading terminal output aloud

## Ready Definition

The first usable version is ready when:

- Scene Mode can accept an explicit voice-input trigger
- the configured STT backend returns a transcript without blocking input
- transcript text appears in the compose dock as editable draft text
- auto-submit remains opt-in
- cancel/timeout leaves Scene Mode usable
- docs explain privacy and backend configuration clearly
