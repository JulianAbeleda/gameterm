# GameTerm Scene VOICEVOX TTS

This helper lets Scene Mode speak English compose replies through a Japanese
VOICEVOX character voice:

```text
English prose -> local translation command -> Japanese text -> VOICEVOX WAV
```

GameTerm still owns speech filtering. The command backend sends this helper
plain prose only; code blocks, logs, diffs, and image data are skipped before
the helper runs.

## Requirements

- `curl`
- `jq`
- VOICEVOX engine running locally
- Optional local translation command
- Optional `codex exec` fallback if no translation command is configured

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

The wrapper defaults to:

```text
VOICEVOX_HOST=127.0.0.1
VOICEVOX_PORT=50021
VOICEVOX_SPEAKER=3
```

## GameTerm Launch

Use the wrapper as a Scene Mode command TTS backend:

```sh
GAMETERM_SCENE_TTS_BACKEND=command \
GAMETERM_SCENE_TTS_COMMAND=/Users/julianabeleda/env/gameterm/ci/scene-tts/voicevox-en-to-ja.sh \
GAMETERM_SCENE_TTS_PLAYER='afplay {output}' \
VOICEVOX_SPEAKER=3 \
gameterm start
```

For predictable translation behavior, set an explicit translator command. The
command receives English prose on stdin and must print Japanese text on stdout:

```sh
GAMETERM_SCENE_TTS_TRANSLATE_COMMAND='codex exec "Translate the following English text to natural Japanese. Output only the Japanese translation."' \
GAMETERM_SCENE_TTS_BACKEND=command \
GAMETERM_SCENE_TTS_COMMAND=/Users/julianabeleda/env/gameterm/ci/scene-tts/voicevox-en-to-ja.sh \
GAMETERM_SCENE_TTS_PLAYER='afplay {output}' \
VOICEVOX_SPEAKER=3 \
gameterm start
```

If `GAMETERM_SCENE_TTS_TRANSLATE_COMMAND` is unset, the wrapper tries the same
`codex exec` translation prompt when the `codex` binary is available. If neither
is available, it exits with a short setup error.

## Standalone Test

With VOICEVOX running:

```sh
export GAMETERM_SCENE_TTS_OUTPUT=/tmp/gameterm-voicevox-test.wav

echo "hello, this is a test" | \
  GAMETERM_SCENE_TTS_TRANSLATE_COMMAND='codex exec "Translate the following English text to natural Japanese. Output only the Japanese translation."' \
  VOICEVOX_SPEAKER=3 \
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
- `translation command not configured`: set
  `GAMETERM_SCENE_TTS_TRANSLATE_COMMAND`, or install/configure `codex`.
- `translation returned empty text`: the translator command ran but did not
  produce Japanese text.
- `VOICEVOX audio_query failed`: VOICEVOX rejected the text or speaker id.
- `VOICEVOX synthesis failed`: VOICEVOX accepted the query but failed to
  produce WAV audio.
