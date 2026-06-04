# GameTerm Scene Mode Handoff

This document is the current handoff for continuing Scene Mode work across
Codex sessions. Check this file first, then use the roadmap and scope docs for
deeper product context.

## Current Snapshot

- Date: 2026-06-04
- Branch: `main`
- Latest behavior commit: current pass `[gui] add Rust Scene VOICEVOX backend`
- Latest tooling commit: `ec1b29b8e [tools] add local CT2 Scene translation helper`
- Remote state at handoff time: local commits pending push
- Worktree state at handoff time: Rust TTS backend/docs changes being committed
- Local app bundle refreshed: `/Users/julianabeleda/Applications/GameTerm.app`
- Persistent Scene compose config:
  `/Users/julianabeleda/.config/gameterm/scene-compose.json`

Current user goal:

Keep moving Scene Mode toward a dogfoodable visual-novel-style surface where
the user can see Codex dialogue, type through a Composer dock, and use the
normal macOS GameTerm app without shell-only setup.

## Latest Commits

Recent committed work:

- current pass `[gui] add Rust Scene VOICEVOX backend`
- `60920fd3d [docs] record CT2 Scene voice benchmark`
- `ec1b29b8e [tools] add local CT2 Scene translation helper`
- `09edb63fe [tools] keep VOICEVOX speaking without implicit translation`
- `3eddb9b61 [gui] add explicit Scene VOICEVOX launch config`
- `9f0a5f246 [docs] document Scene fake Codex TTS test`
- `8409cc3c3 [visual] add Scene fake Codex debug toggle`
- `55661ed85 [tools] add VOICEVOX speak shortcut`
- `6ba902cfe [docs] document VOICEVOX technical line filtering`
- `9cd9a2ec5 [tools] skip technical lines in VOICEVOX TTS`
- `f17b5bf17 [gui] validate Scene Codex config lazily`
- `76ebbdfc2 [docs] record Scene real Codex dogfood pass`
- `4fe553bd1 [gui] make Scene Codex compose dogfoodable`

Latest behavior change:

- Scene Mode now has an `Alt+C` debug toggle for `Codex` vs `Fake Codex`.
- Toggling clears the dialogue transcript, changes the compose backend state,
  and lets the next prompt render a deterministic `Fake Codex` reply.
- Fake Codex replies flow through the same speakable-segment/TTS path as real
  compose replies when Scene TTS is configured and unmuted.
- The `vs` terminal shortcut speaks text through the VOICEVOX wrapper and
  deletes the generated WAV after playback.
- Boot option `1. Scene Mode + VOICEVOX` now routes through a dedicated Scene
  launch action that passes a Rust VOICEVOX TTS config directly into the
  overlay.

Real Codex dogfood pass:

- Scene Mode can run one-shot local `codex exec` through the Composer dock.
- Submitted user prompts render in the dialogue box as a highlighted `>` line.
- Codex replies render below the submitted prompt in the dialogue box.
- The Composer dock clears and remains ready after submit.
- Finder/app launches can opt into Codex through `scene-compose.json`.
- Failure diagnostics classify common Codex CLI issues such as rate limit,
  auth/connect failures, missing binary, and timeout.

VOICEVOX/TTS status:

- `vs hello` works from the regular terminal path, but has roughly 5-6 seconds
  of latency before audio playback.
- The clicked GameTerm app Scene Mode VOICEVOX path now reaches `TTS ready`
  after boot option `1. Scene Mode + VOICEVOX`, `Alt+C` Fake Codex, and a
  short `hi` prompt.
- Before the explicit launch-config fix, option 1 reached Scene Mode but Scene
  status still reported `TTS disabled` and no VOICEVOX command spawned.
- The expected Scene voice path is:
  Composer reply -> speakable prose segment -> Rust Scene TTS worker ->
  optional CT2/helper translation command -> Rust VOICEVOX HTTP
  `audio_query`/`synthesis` -> temporary WAV -> `afplay`.
- VOICEVOX does not receive raw Codex JSON. GameTerm parses/extracts the reply
  first and sends only speakable prose to the TTS command.
- Important primitive: Scene TTS config belongs to the Scene overlay launch.
  Boot option `1. Scene Mode + VOICEVOX` now passes that config directly.
  Plain Scene launches still fall back to `GAMETERM_SCENE_TTS_*` env visible to
  the GUI process. Env set only inside Codex/native terminal does not configure
  the already-running GUI overlay.
- The Rust VOICEVOX backend uses an explicit translator command when provided.
  Otherwise it prefers the local CTranslate2 helper when ready, and falls back
  to direct VOICEVOX synthesis if translation is unavailable.
- The standalone VOICEVOX wrapper still tries `codex exec` translation after
  CT2 when no explicit translator is configured, but falls back to direct
  VOICEVOX if that implicit translation fails. Explicit translator commands
  still fail loudly.
- The VOICEVOX path now prefers the local CTranslate2 helper
  `ci/scene-tts/ct2-en-to-ja.sh` when `.cache/scene-tts` has the int8 FuguMT
  model installed. This removes the `codex exec` translation delay from the
  default app path.

## App And Config State

The installed app bundle was refreshed during this pass:

```text
/Users/julianabeleda/Applications/GameTerm.app
```

For future UI or Scene Mode changes, use this repo command when the clickable
macOS app needs to reflect the latest build:

```sh
make dev-app-open
```

This builds the required binaries, reinstalls `~/Applications/GameTerm.app`,
verifies the app bundle binaries match the build output, quits the currently
installed app if it is running, and reopens the refreshed app.

The current local app-launch Scene compose config is:

```json
{
  "backend_kind": "codex",
  "codex_bin": "/opt/homebrew/bin/codex",
  "codex_workspace": "/Users/julianabeleda/env/gameterm",
  "codex_sandbox": "read-only",
  "codex_approval": "never",
  "codex_timeout_seconds": 90
}
```

This config is user-local and is not committed to the repository.

## Verification Baseline

Commands already run successfully for the latest Codex dogfood and lazy
validation pass:

```sh
cargo test -p gameterm-gui compose_backend --bin gameterm-gui
cargo test -p gameterm-gui codex_compose --bin gameterm-gui
cargo test -p gameterm-gui visual_compose --bin gameterm-gui
cargo check -p gameterm-gui
cargo build -p gameterm-gui
ci/install-macos-dev-app.sh --no-build
```

Latest focused regression check:

```sh
cargo test -p gameterm-visual staged_scene_uses_compose_speaker_for_nameplate_and_can_clear_history
cargo test -p gameterm-gui fake_codex_compose_result_renders_fake_speaker --bin gameterm-gui
cargo test -p gameterm-gui compose_debug_backend_toggle_uses_alt_c_without_consuming_plain_c --bin gameterm-gui
cargo test -p gameterm-gui scene_compose --bin gameterm-gui
cargo test -p gameterm-gui overlay::visual --bin gameterm-gui
cargo check -p gameterm-gui
git diff --check
make dev-app-open
```

Smoke captures:

```text
/tmp/gameterm-scene-vn-compose-codex-fake.png
/tmp/gameterm-scene-vn-real-codex-diagnostic.png
```

Smoke status:

- `vn-compose-codex` fake-Codex smoke: PASS
- `vn-compose` real local Codex CLI smoke: PASS

The real-Codex capture showed:

```text
> say hi

Hi!
```

Earlier in the same pass, a direct standalone `codex exec` probe returned
`429 Too Many Requests` plus websocket `403 Forbidden`. A later Scene Mode
real-Codex smoke succeeded, so treat the earlier direct failure as transient
Codex account/session state unless it reappears.

Known warning noise remains outside this Scene Mode lane:

- existing macOS `objc` macro `unexpected cfg` warnings
- existing `gameterm-toast-notification` unnecessary `unsafe` warnings
- existing `screen_line.rs` unused assignment warning

## Current Product State

Scene Mode now has a first dogfoodable Codex-in-VN loop:

1. Launch GameTerm from the macOS app.
2. Choose Scene Mode from the boot menu.
3. Type in the Composer dock.
4. Press Enter.
5. See the submitted prompt and Codex reply in the dialogue panel.
6. Press `Alt+C` to toggle between real Codex and deterministic `Fake Codex`
   for TTS/debug testing.

This is still a one-shot local Codex bridge. It does not yet preserve Codex
session identity, stream progress into the dialogue panel, or support
`codex exec resume`.

## Recommended Next Actions

1. Reduce remaining VOICEVOX latency:
   - local CTranslate2 translation is now wired and avoids `codex exec`
   - current Scene test phrase benchmark: wrapper generation `1.617s` before
     playback, down from roughly `6.1s` before audio was ready with Codex
     translation
   - Rust now owns the persistent Scene TTS worker and VOICEVOX HTTP request
     path
   - next bottleneck is translation helper process startup plus VOICEVOX
     synthesis; pure Rust in-process translation remains deferred
2. Push local commits after the handoff refresh is committed.
3. Scope persistent Codex sessions:
   - parse session/thread metadata from Codex JSONL if available
   - persist session identity per Scene overlay/session
   - add reset/new-session action
   - add `codex exec resume` support
4. Decide whether progress/streaming events should render into Scene Mode or
   stay a follow-up.
6. Keep app-launch config behavior explicit; do not silently enable network
   backends without user/app config.
7. Keep future commits separated by concern.

Commit discipline:

- Keep separate commits by concern.
- Use established prefixes such as `[docs]`, `[gui]`, `[visual]`, and `[test]`.
- Do not mix formatting-only changes with behavior changes.
- Before committing, run `git diff --check` and targeted tests.
- Treat pre-existing warning noise as separate from Scene Mode failures.
