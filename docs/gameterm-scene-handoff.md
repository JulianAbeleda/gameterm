# GameTerm Scene Mode Handoff

This document is the current handoff for continuing Scene Mode work across
Codex sessions. Check this file first, then use the roadmap and scope docs for
deeper product context.

## Current Snapshot

- Date: 2026-06-04
- Branch: `main`
- Latest behavior commit: `3eddb9b61 [gui] add explicit Scene VOICEVOX launch config`
- Latest tooling commit: `09edb63fe [tools] keep VOICEVOX speaking without implicit translation`
- Remote state at handoff time: `main` is ahead of `origin/main` by 5 commits before this docs commit
- Worktree state at handoff time: docs-only changes pending until this handoff commit is created
- Local app bundle refreshed: `/Users/julianabeleda/Applications/GameTerm.app`
- Persistent Scene compose config:
  `/Users/julianabeleda/.config/gameterm/scene-compose.json`

Current user goal:

Keep moving Scene Mode toward a dogfoodable visual-novel-style surface where
the user can see Codex dialogue, type through a Composer dock, and use the
normal macOS GameTerm app without shell-only setup.

## Latest Commits

Recent committed work:

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
  launch action that passes a command TTS config directly into the overlay.

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
  Composer reply -> speakable prose segment -> `GAMETERM_SCENE_TTS_COMMAND`
  stdin -> VOICEVOX WAV -> `afplay`.
- VOICEVOX does not receive raw Codex JSON. GameTerm parses/extracts the reply
  first and sends only speakable prose to the TTS command.
- Important primitive: Scene TTS config belongs to the Scene overlay launch.
  Boot option `1. Scene Mode + VOICEVOX` now passes that config directly.
  Plain Scene launches still fall back to `GAMETERM_SCENE_TTS_*` env visible to
  the GUI process. Env set only inside Codex/native terminal does not configure
  the already-running GUI overlay.
- The VOICEVOX wrapper tries `codex exec` translation when no explicit
  translator is configured, but falls back to synthesizing the filtered prose
  directly if that implicit translation fails. Explicit translator commands
  still fail loudly.

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

1. Reduce VOICEVOX latency:
   - measure translation time vs VOICEVOX `audio_query` vs synthesis vs
     `afplay`
   - avoid spawning `codex exec` for translation where possible
   - consider a faster local/static translator path for short debug phrases
   - consider moving the VOICEVOX HTTP path into Rust after the command path is
     proven, while still using the same VOICEVOX speaker
2. Push local commits after the handoff refresh is committed.
4. Scope persistent Codex sessions:
   - parse session/thread metadata from Codex JSONL if available
   - persist session identity per Scene overlay/session
   - add reset/new-session action
   - add `codex exec resume` support
5. Decide whether progress/streaming events should render into Scene Mode or
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
