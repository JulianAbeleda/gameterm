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
GAMETERM_SCENE_STT_DEVICE="MacBook Pro Microphone"
```

The default boot menu voice option builds this config in-process. The env vars
are useful when launching from a regular terminal or overriding the model path.
`GAMETERM_SCENE_STT_DEVICE` is optional. If omitted, GameTerm uses the system
default input device. If set, GameTerm matches the configured microphone name
case-insensitively and reports the available devices if it cannot find it.

## Model Setup

Install the default local model:

```bash
ci/scene-stt/setup-whisper-local.sh
```

The helper downloads `ggml-base.en.bin` from the public whisper.cpp model repo
into the default cache path. Runtime does not download models automatically.

## Controls

- Hold `Command+Shift`: start listening.
- Release `Command+Shift`: stop recording and transcribe.
- `Esc` while listening: cancel the active recording.
- `Tab`: enter the Scene debug surface.
- `v` while in the debug surface: open the Scene Voice Debug menu.
- `j/k` or arrows in the voice menu: select a voice debug item.
- `Enter` in the voice menu: toggle the selected voice debug item.
- `Tab` or `Esc` in the voice menu: return to the main debug surface.

Completed transcripts land in the composer dock by default. They are editable
before sending. Auto-submit is opt-in through
`GAMETERM_SCENE_STT_AUTO_SUBMIT=true`; even then, submission uses the existing
Scene compose backend boundary.

Voice test mode is for checking whether the microphone and Whisper recognize
speech. Open `Debug -> Voice -> Voice test mode`, hold `Command+Shift`, and
speak normally. The transcript appears in Scene Voice Diagnostics, but it is
not inserted into the composer and is not submitted to Codex.

## Diagnostics

Open `Debug -> Voice -> Scene voice diagnostics` to show:

- STT backend
- selected microphone, or `system default`
- Whisper model path
- language
- max recording duration
- current status
- last transcript
- last error

Use this before debugging Codex or VOICEVOX. It isolates the input primitive:
microphone capture -> Whisper -> transcript.

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
starts while the user holds `Command+Shift` in Scene Mode. The local Whisper
path does not send microphone audio to a network service.

## Not Implemented Yet

- voice activity detection
- Apple Speech backend
- cloud STT backends
- streaming partial transcripts
