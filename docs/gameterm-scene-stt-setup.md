# GameTerm Scene STT Setup

Status: FIRST PASS IMPLEMENTED.

Scene Mode can accept an explicit voice transcript from a local command helper
and insert it into the compose dock as editable text. STT is disabled by
default.

## Environment

```text
GAMETERM_SCENE_STT_BACKEND=command
GAMETERM_SCENE_STT_COMMAND='my-stt-helper'
GAMETERM_SCENE_STT_TIMEOUT_SECONDS=20
GAMETERM_SCENE_STT_AUTO_SUBMIT=false
```

The command backend is helper-owned recording. GameTerm starts the helper when
you trigger voice input, and the helper decides how to record or transcribe.
GameTerm reads stdout as the transcript and stderr as diagnostics.

```text
stdout: transcript text
stderr: diagnostics
exit 0: transcript succeeded
nonzero: STT failed
```

This supports local tools such as Whisper wrappers or user scripts without
making GameTerm download models or request microphone permissions directly.

## Controls

- `Alt-v`: start a bounded voice helper session.
- `Alt-v` again while running: request cancellation.

Completed transcripts land in the compose dock by default. They are editable
before sending. Auto-submit is opt-in through
`GAMETERM_SCENE_STT_AUTO_SUBMIT=true`; even then, submission uses the existing
Scene compose backend boundary.

## Privacy

GameTerm does not listen globally, does not start STT by default, and does not
send microphone audio to a network service. Any recording, model use, or cloud
call is controlled by the configured helper command.

## Not Implemented Yet

- native microphone capture
- Apple Speech backend
- cloud STT backends
- wake word or always-on voice activity detection
