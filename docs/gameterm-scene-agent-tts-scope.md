# GameTerm Scene Agent TTS Scope

Status: SCOPED.

This document scopes a first-pass text-to-speech layer for Scene Mode agent
messages.

The product goal is not to read raw terminal output. GameTerm should speak
agent-readable prose when Codex, Claude, or a Scene agent responds, while
skipping source code, diffs, logs, stack traces, JSON, command output, and other
machine-oriented text.

## Goal

When an agent speaks in Scene Mode, GameTerm should be able to narrate the
human-readable parts of the message through a configured voice.

Example flow:

```text
Codex backend reply
-> speakable segment extractor
-> TTS backend
-> cached audio file
-> playback queue
```

The first pass should make this local, explicit, and auditable:

- local VOICEVOX backend as the main useful Japanese voice path
- Open JTalk-compatible command backend as a simple fallback path
- no default network TTS
- no bundled third-party voices without explicit license metadata
- no raw scrollback narration

## Product Contract

TTS should speak:

- assistant explanations
- task plans and status summaries
- review findings
- Scene dialogue
- choices and prompts when they are written as prose

TTS should skip:

- fenced code blocks
- inline or block diffs
- command output
- stack traces
- JSON, TOML, YAML, and log blobs
- long file lists
- long paths and identifiers unless the surrounding text is still readable

The user should be able to stop speech without closing Scene Mode. Muting TTS
should not affect compose, rendering, or Scene runtime state.

## Fit With Existing Architecture

Scene compose already has a useful backend pattern:

- explicit environment/configuration
- deterministic built-in behavior for tests
- process isolation for external tools
- bounded output and timeout handling
- narrow GUI integration from `overlay/visual.rs`

TTS should follow that shape instead of becoming a global terminal audio
feature.

Recommended new module boundary:

```text
gameterm-gui/src/overlay/visual_tts.rs
```

`visual_tts.rs` should own:

- TTS configuration from environment
- speakable segment extraction
- TTS backend request/result types
- local backend process calls
- bounded output/error handling
- cache path generation
- unit tests for filtering and backend command construction

`visual.rs` should only call a small interface when new speakable dialogue is
available.

## First-Pass Backend Contract

### VOICEVOX

VOICEVOX should be treated as the first useful local backend because it is
Japanese-first, local, and exposes an HTTP engine API.

Suggested environment:

```text
GAMETERM_SCENE_TTS_BACKEND=voicevox
GAMETERM_SCENE_TTS_VOICEVOX_URL=http://127.0.0.1:50021
GAMETERM_SCENE_TTS_VOICEVOX_SPEAKER=1
GAMETERM_SCENE_TTS_CACHE_DIR=<optional cache dir>
```

Backend behavior:

1. POST text to VOICEVOX audio query endpoint.
2. POST the returned query to synthesis endpoint.
3. Write WAV output into a local cache file.
4. Enqueue the file for playback.

The implementation should store configured speaker id and source metadata in
runtime/reporting surfaces where practical. VOICEVOX voice terms vary by
speaker, so GameTerm should not imply every voice is license-equivalent.

### Command Backend

Add a generic local command backend for Open JTalk or user scripts:

```text
GAMETERM_SCENE_TTS_BACKEND=command
GAMETERM_SCENE_TTS_COMMAND='my-tts-helper --voice default --output {output}'
```

Command contract:

```text
stdin: speakable text
argv/env: output path, speaker/voice metadata, scene/session id when available
exit 0: output audio was written
nonzero: TTS failed and status is updated
```

Command parsing should use structured argv parsing, matching compose backend
discipline.

### Built-In Test Backend

The default should be silent or deterministic:

- no external service
- no network
- no audio device dependency
- returns a predictable fake audio-cache result for unit tests

This keeps verification stable without requiring VOICEVOX to be installed.

## Speakable Segment Extraction

Add a conservative extractor before any backend work.

Input:

- assistant reply text from compose backend
- Scene dialogue body text
- future Claude/Codex message events

Output:

```rust
pub struct SpeakableSegment {
    pub speaker: Option<String>,
    pub text: String,
    pub source: SpeakableSource,
}
```

Rules:

- remove fenced code blocks before line classification
- remove diff hunks and patch markers
- drop lines that look like stack frames, logs, JSON, or shell output
- drop lines dominated by paths, punctuation, or identifiers
- preserve normal prose paragraphs and short bullet findings
- cap segment length before synthesis

This should be intentionally conservative. False negatives are acceptable in
the first pass; reading code aloud is not.

## Agent Voice Identity

First pass:

- map backend label to a voice id by environment/config
- support a single default voice
- optionally map `Codex` and `Claude` labels separately

Suggested environment:

```text
GAMETERM_SCENE_TTS_DEFAULT_VOICE=voicevox:1
GAMETERM_SCENE_TTS_CODEX_VOICE=voicevox:1
GAMETERM_SCENE_TTS_CLAUDE_VOICE=voicevox:3
```

Deferred:

- scene JSON voice manifests
- per-agent voice settings in entity metadata
- UI voice picker
- attribution panel

## Playback

Keep playback separate from synthesis.

First pass can use a local player command selected by platform or environment:

```text
GAMETERM_SCENE_TTS_PLAYER=afplay
```

Requirements:

- queue synthesized files in order
- stop current playback on user command
- update Scene status when synthesis/playback fails
- never block Scene input while audio is playing

Deferred:

- cross-platform native audio integration
- volume ducking
- streaming playback before full synthesis
- interrupt-on-new-message policy

## UI Controls

First-pass controls should be minimal:

- enable/disable TTS
- stop current speech
- status line showing idle/synthesizing/playing/failed

Candidate key behavior:

- `m`: mute/unmute Scene TTS when compose dock is not focused
- `s`: stop current speech when compose dock is not focused

Exact bindings can change during implementation if they conflict with current
Scene input handling.

## Implementation Slices

### Slice 1: TTS Model And Extraction

Files:

- `gameterm-gui/src/overlay/visual_tts.rs`
- `gameterm-gui/src/overlay/mod.rs`

Work:

- add TTS request/result/config types
- add speakable segment extractor
- add unit tests for prose/code/diff/log filtering
- keep default behavior silent

Verification:

```sh
cargo test -p gameterm-gui visual_tts
cargo check -p gameterm-gui
```

Commit:

```text
[gui] add Scene TTS speech extraction model
```

### Slice 2: Local Backend And Cache

Files:

- `gameterm-gui/src/overlay/visual_tts.rs`

Work:

- add command backend
- add VOICEVOX HTTP request path
- add bounded synthesis timeout
- write output to cache dir
- avoid bundling or downloading voices

Verification:

```sh
cargo test -p gameterm-gui visual_tts
cargo check -p gameterm-gui
```

Commit:

```text
[gui] add local Scene TTS backends
```

### Slice 3: Compose Integration

Files:

- `gameterm-gui/src/overlay/visual.rs`
- `gameterm-gui/src/overlay/visual_tts.rs`

Work:

- pass assistant dialogue replies through extractor
- enqueue speakable segments only after compose success
- surface TTS status without changing dialogue text
- keep TTS opt-in until backend config is present

Verification:

```sh
cargo test -p gameterm-gui visual_tts
cargo check -p gameterm-gui
ci/gameterm-scene-verify.sh --all
```

Commit:

```text
[gui] speak Scene compose replies through TTS
```

### Slice 4: Playback Controls

Files:

- `gameterm-gui/src/overlay/visual.rs`
- `gameterm-gui/src/overlay/visual_tts.rs`
- `docs/gameterm-scene-mode.md`

Work:

- add playback queue
- add stop/mute controls
- add docs for VOICEVOX setup and command fallback
- keep controls local to Scene Mode

Verification:

```sh
cargo test -p gameterm-gui visual_tts
cargo check -p gameterm-gui
ci/gameterm-scene-smoke.sh --describe-scenario vn-compose
```

Commit:

```text
[gui] add Scene TTS playback controls
```

## Risks

### Risk: TTS Reads Code

Mitigation:

- extractor is conservative
- code/diff/log fixtures are tested
- no raw scrollback feed

### Risk: Licensing Gets Blurry

Mitigation:

- do not bundle voices in first pass
- require explicit backend configuration
- track provider/speaker metadata
- document that VOICEVOX speaker terms vary

### Risk: Audio Blocks Scene Input

Mitigation:

- synthesize and play on worker threads
- communicate through bounded channels
- never wait on TTS from the render/input loop

### Risk: Feature Becomes Too Broad

Mitigation:

- no global terminal reader
- no network backend in first pass
- no voice picker
- no persistent assistant audio session
- no release claim until live smoke/manual playback has passed

## Out Of Scope

- screen reader/accessibility replacement
- reading all terminal output
- voice input/STT
- direct OpenAI/Google/ElevenLabs TTS backend
- bundling VOICEVOX or speaker models
- streaming token-by-token speech
- multi-window global playback coordination

## Ready Definition

The first usable version is ready when:

- compose assistant prose can be converted into one or more speakable segments
- code blocks and diffs are skipped in tests
- a local backend can synthesize an audio file from a segment
- Scene input remains responsive while synthesis/playback occurs
- the user can stop or mute playback
- docs explain how to configure VOICEVOX without implying bundled voice rights
