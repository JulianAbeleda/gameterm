# GameTerm Scene Voice/Compose Fresh Machine Setup

Status: VERIFIED AGAINST SOURCE 2026-06-12.

What a new Mac actually needs to run the full Scene Mode voice and compose
suite, and what it does not. The base terminal, Scene/VN rendering, and shell
screens need nothing beyond the installed app
(`brew install --cask julianabeleda/tap/gameterm`).

## Dependency Matrix

| Feature | Built into the app | External requirement |
| --- | --- | --- |
| Terminal, Scene/VN rendering | yes | none |
| Voice input (STT) | whisper.cpp (`whisper_rs`) + CoreAudio capture (`cpal`) | Whisper model file; macOS microphone permission |
| Voice output (TTS) | VOICEVOX HTTP client + `afplay` playback | VOICEVOX engine reachable at `127.0.0.1:50021` |
| EN→JA translation (optional) | discovery + ready-check | repo checkout with the CT2 venv and model installed |
| AI Compose | codex backend driver | `codex` binary, signed in; existing workspace directory |

Common misconceptions, verified against source:

- No `whisper` binary is ever invoked. Transcription runs in-process via
  `whisper_rs` (`gameterm-gui/src/overlay/visual_stt_backend.rs`). A missing
  `whisper` on PATH means nothing.
- `ffmpeg` is not used by the voice suite. The only repo reference is the
  screen-capture smoke test (`ci/gameterm-scene-smoke.sh`).
- Audio playback uses `afplay`, which ships with macOS.

## Setup Steps

```sh
# 1. Repo checkout (compose workspace + translation discovery default)
git clone https://github.com/JulianAbeleda/gameterm ~/env/gameterm
cd ~/env/gameterm

# 2. Whisper model (~148MB) for voice input
bash ci/scene-stt/setup-whisper-local.sh

# 3. Optional EN→JA voice translation (~1GB venv + models)
bash ci/scene-tts/setup-ct2-en-ja.sh

# 4. VOICEVOX engine for voice output (~3.7GB), one of:
#    a. copy an existing install from another machine:
#       ~/.local/share/gameterm-tools/voicevox/macos-arm64/
#       then run: .../macos-arm64/run --host 127.0.0.1 --port 50021
#    b. tunnel to a machine already running it:
#       ssh -N -f -L 50021:127.0.0.1:50021 <user>@<host>
```

Compose additionally needs `codex` installed and signed in on the machine, and
`~/.config/gameterm/scene-compose.json` pointing `codex_workspace` at a
directory that exists.

First voice use triggers the macOS microphone permission prompt for GameTerm.

## Degradation Behavior

Each layer fails independently and visibly; none block the others:

- Missing Whisper model: voice input reports the missing model path.
- VOICEVOX unreachable: speech requests fail with a connection error; text
  flow is unaffected.
- Translation helper absent or not ready: translation silently turns off and
  VOICEVOX receives the untranslated text.
- Compose workspace missing: codex exec fails at spawn with its own error.

## Path Defaults

- Whisper model: `~/Library/Caches/gameterm/scene-stt/models/ggml-base.en.bin`
  (`GAMETERM_SCENE_STT_WHISPER_MODEL` overrides)
- Translation helper discovery: `./ci/scene-tts/ct2-en-to-ja.sh` from the
  current directory, then `~/env/gameterm/ci/scene-tts/ct2-en-to-ja.sh`
  (`GAMETERM_SCENE_TTS_CT2_COMMAND` is the canonical override)
- VOICEVOX endpoint: `127.0.0.1:50021`
  (`GAMETERM_SCENE_TTS_VOICEVOX_HOST` / `_PORT` override)

## Detailed Per-Feature Docs

- `gameterm-scene-stt-setup.md` — STT env vars, mic selection, behavior
- `gameterm-scene-tts-setup.md` — TTS backends and env vars
- `gameterm-scene-voicevox-tts.md` — VOICEVOX specifics
- `gameterm-scene-codex-compose-bridge-scope.md` — codex compose backend scope
