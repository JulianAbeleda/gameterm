# GameTerm Scene TTS Setup

Status: FIRST PASS IMPLEMENTED.

Scene Mode can speak human-readable compose replies through opt-in local
text-to-speech backends. Plain Scene launches keep TTS disabled by default.
The boot-menu VOICEVOX option enables the Rust VOICEVOX backend explicitly.

## Environment

```text
GAMETERM_SCENE_TTS_BACKEND=command
GAMETERM_SCENE_TTS_COMMAND='my-tts-helper --output {output}'
GAMETERM_SCENE_TTS_PLAYER='afplay {output}'
GAMETERM_SCENE_TTS_CACHE_DIR=/tmp
GAMETERM_SCENE_TTS_TIMEOUT_SECONDS=20
```

Supported backend values:

- `command`: send prose to an external command that writes a WAV.
- `voicevox`: call a local VOICEVOX HTTP engine from Rust.
- `silent`: mark speech as handled without producing audio, for tests.

`GAMETERM_SCENE_TTS_COMMAND` receives speakable text on stdin and should write
audio to the path provided through `{output}` or `GAMETERM_SCENE_TTS_OUTPUT`.
GameTerm also passes:

- `GAMETERM_SCENE_TTS_OUTPUT`
- `GAMETERM_SCENE_TTS_SPEAKER`
- `GAMETERM_SCENE_TTS_SOURCE`

`GAMETERM_SCENE_TTS_PLAYER` is optional. If set, GameTerm runs it after the TTS
command succeeds. Use `{output}` in the player command to receive the generated
audio path.

For a Japanese VOICEVOX command wrapper that translates English prose first, see
[GameTerm Scene VOICEVOX TTS](gameterm-scene-voicevox-tts.md).

The Rust VOICEVOX backend uses:

```text
GAMETERM_SCENE_TTS_BACKEND=voicevox
GAMETERM_SCENE_TTS_VOICEVOX_HOST=127.0.0.1
GAMETERM_SCENE_TTS_VOICEVOX_PORT=50021
GAMETERM_SCENE_TTS_VOICEVOX_SPEAKER=14
GAMETERM_SCENE_TTS_TRANSLATION_BACKEND=ct2
GAMETERM_SCENE_TTS_TRANSLATE_COMMAND='optional translator command'
GAMETERM_SCENE_TTS_PLAYER='afplay {output}'
```

If no explicit translation command is set, the Rust backend uses the local
CTranslate2 helper when it is installed and ready. Otherwise it sends the
filtered prose directly to VOICEVOX.

For tests or dry runs:

```text
GAMETERM_SCENE_TTS_BACKEND=silent
```

## Controls

- `Alt-m`: mute or unmute future Scene TTS output.

Muting does not affect compose, Scene rendering, or runtime state. It only
prevents new compose replies from being sent to the TTS backend.

## Speech Filtering

The first pass speaks prose from successful compose replies and skips common
machine-oriented text:

- fenced code blocks
- diffs
- command failures and log prefixes
- stack traces
- path-like or punctuation-heavy lines

## Licensing

GameTerm does not bundle voices, voice models, or third-party TTS assets. If you
configure a local engine or downloaded voice, check that engine and voice's
license before using it in a distributed demo.

## Not Implemented Yet

- stop current speech process
- per-character voices
- scene-dialogue auto narration outside compose replies
- pure Rust in-process translation model bindings
