# GameTerm Scene VOICEVOX TTS

Scene Mode can speak English compose replies through a Japanese VOICEVOX
character voice:

```text
English prose -> optional local translation -> Japanese text -> VOICEVOX WAV
```

GameTerm owns speech filtering. Code blocks, logs, diffs, and image data are
skipped before the VOICEVOX path runs.

The standalone wrapper also applies a second whole-line filter before
translation. It skips lines that are mostly technical, such as file paths,
Windows paths, URLs, commands, JSON/code-looking lines, commit hashes,
stack/log output, and build output. The visible Scene dialogue is unchanged;
only the spoken transcript is filtered.

## Requirements

- `curl`
- `jq`
- VOICEVOX engine running locally
- Optional local CTranslate2 translator
- Optional local translation command
- Optional `codex exec` fallback for the standalone wrapper

GameTerm does not bundle VOICEVOX, voices, translation services, or model
weights. Check the license for the VOICEVOX engine and speaker you use before
distributing a demo.

## Start VOICEVOX

Run the VOICEVOX desktop app, standalone engine, or Docker engine so the HTTP
API is available on port `50021`.

Check that it is reachable:

```sh
curl http://127.0.0.1:50021/version
```

The Rust backend and wrapper default to:

```text
VOICEVOX_HOST=127.0.0.1
VOICEVOX_PORT=50021
VOICEVOX_SPEAKER=14
```

## GameTerm Launch

If the GameTerm boot menu is enabled, choose:

```text
1. Scene Mode + VOICEVOX
```

That option opens Scene Mode with an explicit Rust VOICEVOX TTS launch
configuration. It does not rely on shell environment variables being present in
the already-running GUI process. The regular `2. Scene Mode` boot option
remains a quiet Scene Mode launch without TTS.

The app boot option uses:

```text
GameTerm speakable prose
-> Rust Scene TTS worker
-> optional CT2/helper translation command
-> Rust VOICEVOX HTTP audio_query/synthesis
-> temporary WAV
-> afplay
```

To force the Rust VOICEVOX backend from a shell launch:

```sh
GAMETERM_SCENE_TTS_BACKEND=voicevox \
GAMETERM_SCENE_TTS_VOICEVOX_SPEAKER=14 \
GAMETERM_SCENE_TTS_TRANSLATION_BACKEND=ct2 \
GAMETERM_SCENE_TTS_PLAYER='afplay {output}' \
gameterm start
```

The wrapper is still available as a Scene Mode command TTS backend:

```sh
GAMETERM_SCENE_TTS_BACKEND=command \
GAMETERM_SCENE_TTS_COMMAND=/Users/julianabeleda/env/gameterm/ci/scene-tts/voicevox-en-to-ja.sh \
GAMETERM_SCENE_TTS_PLAYER='afplay {output}' \
VOICEVOX_SPEAKER=14 \
gameterm start
```

For fast local translation, install the CTranslate2 English -> Japanese helper:

```sh
ci/scene-tts/setup-ct2-en-ja.sh
```

The setup writes a Python venv and converted FuguMT CTranslate2 int8 model under
`.cache/scene-tts`, which is gitignored. Once installed, the Rust VOICEVOX
backend and wrapper can use it automatically.

You can test the translator directly:

```sh
echo "hello, this is a voice test" | ci/scene-tts/ct2-en-to-ja.sh --timing
```

For predictable custom translation behavior, set an explicit translator
command. The command receives English prose on stdin and must print Japanese
text on stdout:

```sh
GAMETERM_SCENE_TTS_TRANSLATE_COMMAND='codex exec "Translate the following English text to natural Japanese. Output only the Japanese translation."' \
GAMETERM_SCENE_TTS_BACKEND=command \
GAMETERM_SCENE_TTS_COMMAND=/Users/julianabeleda/env/gameterm/ci/scene-tts/voicevox-en-to-ja.sh \
GAMETERM_SCENE_TTS_PLAYER='afplay {output}' \
VOICEVOX_SPEAKER=14 \
gameterm start
```

If `GAMETERM_SCENE_TTS_TRANSLATE_COMMAND` is unset, the Rust backend first
checks whether the local CTranslate2 helper is ready. If it is not ready, the
backend falls back to synthesizing the filtered prose directly through
VOICEVOX. The standalone wrapper checks CTranslate2 first, then can try
`codex exec`, then falls back to direct VOICEVOX when that implicit translation
fails. Explicit translator commands still fail loudly when they fail.

## Standalone Test

With VOICEVOX running:

```sh
vs hello
```

The `vs` helper reads text from arguments or stdin, writes a temporary WAV,
plays it with `afplay`, and deletes the WAV after playback:

```sh
vs "hello, this is a test"
echo "hello, this is a test" | vs
```

## Scene Fake-Codex Test

Use the Scene compose debug backend when you want to test the dialogue and TTS
path without waiting on real Codex:

```text
Tab -> v -> Fake Codex backend -> Enter
```

`Debug -> Voice -> Fake Codex backend` toggles Scene compose between the
configured compose backend and `Fake Codex`. Toggling clears the current
dialogue transcript so the next prompt starts from a clean panel. In fake mode,
submitted Composer prompts produce a deterministic `Fake Codex` reply, update
the dialogue nameplate to `Fake Codex`, and still flow through the same Scene
TTS command backend when TTS is enabled and unmuted.

Known current status:

- The standalone terminal shortcut works: `vs hello`.
- With the local CTranslate2 helper installed, the standalone wrapper generated
  the Scene test WAV in `1.617s` before playback. The old Codex translation path
  took roughly `6.1s` before audio was ready for the same text.
- Scene Mode VOICEVOX from the clicked app reaches `TTS ready` after choosing
  boot option `1. Scene Mode + VOICEVOX`, toggling
  `Debug -> Voice -> Fake Codex backend`, and submitting a short prompt.
- Boot option `1. Scene Mode + VOICEVOX` now passes a Rust VOICEVOX TTS config
  directly into the Scene overlay. Setting env only inside the native
  terminal/Codex child process still will not configure Scene TTS for an
  existing GUI overlay.
- Plain Scene launches still use `GAMETERM_SCENE_TTS_*` as the fallback config
  source when no explicit launch option is provided.

For lower-level debugging, call the wrapper directly:

```sh
export GAMETERM_SCENE_TTS_OUTPUT=/tmp/gameterm-voicevox-test.wav

echo "hello, this is a test" | \
  GAMETERM_SCENE_TTS_TRANSLATE_COMMAND='codex exec "Translate the following English text to natural Japanese. Output only the Japanese translation."' \
  VOICEVOX_SPEAKER=14 \
  ci/scene-tts/voicevox-en-to-ja.sh && \
  afplay "$GAMETERM_SCENE_TTS_OUTPUT"
```

To force the local CTranslate2 helper explicitly:

```sh
export GAMETERM_SCENE_TTS_OUTPUT=/tmp/gameterm-voicevox-test.wav

echo "hello, this is a test" | \
  GAMETERM_SCENE_TTS_TRANSLATE_COMMAND='/Users/julianabeleda/env/gameterm/ci/scene-tts/ct2-en-to-ja.sh' \
  VOICEVOX_SPEAKER=14 \
  ci/scene-tts/voicevox-en-to-ja.sh && \
  afplay "$GAMETERM_SCENE_TTS_OUTPUT"
```

The wrapper writes to a temporary WAV first, then moves it to
`GAMETERM_SCENE_TTS_OUTPUT` only after synthesis succeeds. On failure it removes
temporary files and prints a short stderr message that GameTerm can surface as
`TTS failed: ...`.

## Troubleshooting

- `VOICEVOX engine not reachable ...`: start VOICEVOX and confirm
  `curl http://127.0.0.1:50021/version` works.
- `translation returned empty text`: the translator command ran but did not
  produce Japanese text.
- `VOICEVOX audio_query failed`: VOICEVOX rejected the text or speaker id.
- `VOICEVOX synthesis failed`: VOICEVOX accepted the query but failed to
  produce WAV audio.
