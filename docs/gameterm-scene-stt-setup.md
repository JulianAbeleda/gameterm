# GameTerm Scene STT Setup

Status: LOCAL WHISPER FIRST PASS IMPLEMENTED.

Scene Mode can record explicit hold-to-talk speech, transcribe it locally with
Whisper, and insert the transcript into the compose dock as editable text. STT
is disabled in native terminal mode and enabled by the boot menu voice path.

## Environment

```text
GAMETERM_SCENE_STT_BACKEND=whisper
GAMETERM_SCENE_STT_WHISPER_MODEL=~/Library/Caches/gameterm/scene-stt/models/ggml-base.en.bin
GAMETERM_SCENE_STT_LANGUAGE=en
GAMETERM_SCENE_STT_MAX_SECONDS=20
GAMETERM_SCENE_STT_TIMEOUT_SECONDS=20
GAMETERM_SCENE_STT_AUTO_SUBMIT=false
```

The default boot menu voice option builds this config in-process. The env vars
are useful when launching from a regular terminal or overriding the model path.

## Model Setup

Install the default local model:

```bash
ci/scene-stt/setup-whisper-local.sh
```

The helper downloads `ggml-base.en.bin` from the public whisper.cpp model repo
into the default cache path. Runtime does not download models automatically.

## Controls

- Hold `Shift+Space`: start listening.
- Release `Shift+Space`: stop recording and transcribe.
- `Esc` while listening: cancel the active recording.

Completed transcripts land in the composer dock by default. They are editable
before sending. Auto-submit is opt-in through
`GAMETERM_SCENE_STT_AUTO_SUBMIT=true`; even then, submission uses the existing
Scene compose backend boundary.

## Command Backend

The older command backend still exists for custom helpers:

```text
GAMETERM_SCENE_STT_BACKEND=command
GAMETERM_SCENE_STT_COMMAND='my-stt-helper'
```

GameTerm starts the helper when voice input begins, reads stdout as the
transcript, and reads stderr as diagnostics.

```text
stdout: transcript text
stderr: diagnostics
exit 0: transcript succeeded
nonzero: STT failed
```

## Privacy

GameTerm does not listen globally and does not use wake words. Recording only
starts while the user holds `Shift+Space` in Scene Mode. The local Whisper path
does not send microphone audio to a network service.

## Not Implemented Yet

- voice activity detection
- Apple Speech backend
- cloud STT backends
- streaming partial transcripts
