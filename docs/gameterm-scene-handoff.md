# GameTerm Scene Mode Handoff

This document is the current handoff for continuing Scene Mode work across
Codex sessions. Check this file first, then use the roadmap and scope docs for
deeper product context.

## Current Snapshot

- Date: 2026-06-09
- Branch: `main`
- Latest behavior commit: `0891c8942 [visual] add Scene asset review previews`
- Latest refactor commit: `017ddf309 [gui] NFC - move SceneRuntime test import`
- Latest tooling commit: `ec1b29b8e [tools] add local CT2 Scene translation helper`
- Remote state at handoff time: local branch is ahead of `origin/main` by the
  Scene TTS polish, Scene asset editor, Scene asset operation, and Scene asset
  primitive-tightening commits listed below
- Worktree state at handoff time: clean after primitive workflow docs commit
- Local app bundle refreshed: `/Users/julianabeleda/Applications/GameTerm.app`
- Persistent Scene compose config:
  `/Users/julianabeleda/.config/gameterm/scene-compose.json`

Current user goal:

Keep moving Scene Mode toward a dogfoodable visual-novel-style surface where
the user can see Codex dialogue, type through a Composer dock, and use the
normal macOS GameTerm app without shell-only setup.

Current next priority:

- Scene asset editor non-GUI substrate, first-pass AI/human operation layer,
  and primitive-tightening pass are complete. The command cookbook is recorded in
  [Scene Asset Editor Cookbook](gameterm-scene-asset-editor-cookbook.md), the
  operation contract is recorded in
  [Scene Asset Editor AI/Human Operation Scope](gameterm-scene-asset-editor-ai-operation-scope.md),
  the primitive-tightening pass is recorded in
  [Scene Asset Primitive Tightening Scope](gameterm-scene-asset-primitive-tightening-scope.md),
  and the original completion scope is recorded in
  `structure/Development/scene-asset-edit-feature-scope.md`.
- TTS polish first pass is implemented. The scope and remaining dogfood items
  are recorded in [Scene TTS Polish Scope](gameterm-scene-tts-polish-scope.md).
- Next priority is either a live app dogfood pass with real Codex plus VOICEVOX,
  or a GUI pass on top of the completed asset editor operation primitives.
- Persistent Codex sessions are deferred for now because the current dogfood
  use case does not need cross-disconnect conversation resume.

## Latest Commits

Recent committed work:

- `0891c8942 [visual] add Scene asset review previews`
- `c1907b9a8 [visual] assert Scene asset protected regions`
- `e5e9e0577 [visual] add Scene asset mask roundtrip`
- `2935b00f5 [visual] add Scene asset operation validation`
- `42184062a [visual] add Scene asset output acceptance`
- `897945771 [docs] scope Scene asset primitive tightening`
- `8c4f0aaf9 [visual] add Scene asset edit sessions`
- `ce40e36fe [visual] add Scene asset operation diagnostics`
- `2a26cf563 [visual] add Scene asset operation previews`
- `7685bb7a4 [visual] add Scene asset operation runner`
- `ceaca84eb [docs] scope Scene asset operation layer`
- `af3d1afb6 [visual] add Scene asset compositing and states`
- `e8cdfe80e [visual] add Scene asset tonal and filter operations`
- `f87b770c2 [visual] add Scene asset transform operations`
- `953234b3d [visual] add Scene asset drawing operations`
- `ecec652ca [visual] add Scene asset pipeline runner`
- `439dbceb5 [visual] add Scene asset sampling reports`
- `6002f7ea9 [docs] define Scene asset editor completion`
- `48691d047 [visual] add Scene asset paint primitives`
- `883385ae6 [visual] add Scene TTS queue diagnostics`
- `e53a8efdf [gui] align Scene TTS extraction with dialogue blocks`
- `363b0c804 [visual] add Scene dialogue block projection`
- `6d76dc425 [docs] scope Scene TTS polish`
- `017ddf309 [gui] NFC - move SceneRuntime test import`
- `b7ffb250d [gui] NFC - split Scene stage image helpers`
- `96e1cbae7 [gui] NFC - split Scene overlay input helpers`
- `67b72c4e9 [gui] NFC - split Scene STT backend`
- `26cfacf5d [gui] NFC - split Scene compose backend`
- `709729bad [gui] NFC - split Scene TTS speech blocks`
- `a3f9d2981 [docs] scope Scene maintainability refactor`
- `bc7529a58 [test] harden Scene compose TTS visibility`
- `41fc33eec [visual] serialize Scene TTS playback`
- `d529eb9a7 [visual] add ordered Scene speech blocks`
- `1c642e468 [visual] add Scene compose turn display blocks`
- `3f14ed146 [visual] auto-submit Scene voice prompts`
- `d48711e09 [visual] latch Scene voice hold captures`
- `0d33471b4 [visual] stabilize Scene voice hold state`
- `123d7eb7f [visual] forward Scene voice hold modifiers`
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

- Scene asset editing now has explicit `accept-output` semantics. Intermediate
  work stays in `Transformation`; reviewed files enter `Output` only through an
  acceptance command that writes a report with image metadata and SHA-256.
- Scene asset editing now has write-free `validate-operation`, so a human or
  AI can validate operation JSON, roots, feature maps, protected regions,
  command arguments, and overwrite policy before previewing or running.
- Scene asset editing now treats masks as durable artifacts through
  `mask-export`, `mask-apply-alpha`, and `mask-composite`.
- Scene asset operation reports can assert protected regions after an edit using
  `must_preserve_regions` and `max_changed_pixels_in_protected_regions`.
- Scene asset review previews now include raw diff, overlay diff, alpha diff,
  checkerboard, dark-background, and contact-sheet artifacts. `operation-run
  --preview` emits the richer paths.
- Scene asset editing now has a versioned AI/human operation layer. A user or
  agent can run a single JSON operation through `operation-run`, preview it
  without accepting the requested output, inspect a diff artifact, and receive a
  before/after compare report.
- Scene asset editing now has structured operation diagnostics with stable
  error codes and hints for common correction paths.
- Scene asset editing now has `session-run` for ordered edit session files.
  Operation sources can chain from `Input`, `Transformation`, or `Output` roots,
  and session reports record operation order plus final output.
- Scene TTS polish first pass is implemented. Visible dialogue formatting and
  speech extraction now share `VisualDialogueTextBlock` projection, so headings,
  bullets, numbered items, prose, and technical skipped lines use the same block
  boundaries.
- Scene TTS cleanup now has regression coverage proving raw URLs, Unix paths,
  Windows paths, env vars, flags, and commit hashes do not reach speakable
  output while display text stays useful.
- Scene TTS requests now carry queue generation ids. New prompts, fake-Codex
  toggles, history clears, STT auto-submit, and Stop TTS invalidate older queued
  or playing speech.
- Scene TTS diagnostics now show queue depth, current block, current phase,
  skipped count, last error, and timing summary in `Debug -> Voice`.
- `Debug -> Voice -> Test TTS playback` can enqueue a deterministic TTS test
  without requiring Codex. `Stop TTS playback` invalidates the active queue.
- Scene compose history now carries turn/block metadata. The VN dialogue panel
  renders through a transcript formatter instead of directly wrapping raw
  message strings, so user prompts, structured compose JSON, and flattened
  numbered replies display with more readable turn spacing.
- Scene TTS now extracts ordered speech blocks with separate display text and
  cleaned speakable text. Inline paths, URLs, command snippets, flags, env
  vars, commit hashes, and technical filenames are cleaned or skipped before
  VOICEVOX sees the text.
- Scene TTS playback now waits for the configured player command to finish
  before starting the next speech block. Temporary WAV files are deleted after
  successful playback, and Scene status reports `TTS played` for played audio.
- Scene Mode now has `Debug -> Voice -> Fake Codex backend` for toggling
  between the configured compose backend and deterministic `Fake Codex`.
- Toggling clears the dialogue transcript, changes the compose backend state,
  and lets the next prompt render a deterministic `Fake Codex` reply.
- Fake Codex replies flow through the same speakable-segment/TTS path as real
  compose replies when Scene TTS is configured and unmuted.
- The `vs` terminal shortcut speaks text through the VOICEVOX wrapper and
  deletes the generated WAV after playback.
- Boot option `1. Scene Mode + VOICEVOX` now routes through a dedicated Scene
  launch action that passes a Rust VOICEVOX TTS config directly into the
  overlay.

Latest refactor pass:

- `visual_speech_blocks.rs` now owns speakable segment types, extraction,
  technical cleanup, and chunk splitting. `visual_tts.rs` keeps TTS
  worker/backend execution.
- `visual_compose.rs` is now a facade over `visual_compose_backend.rs`.
- `visual_stt.rs` is now a facade over `visual_stt_backend.rs`.
- `visual_scene_debug_input.rs` owns Scene debug-menu side effects.
- `visual_voice_hold_flow.rs` owns voice hold reconciliation.
- `visual_stage_image.rs` owns Scene stage image scaling, placement,
  placeholder fit, and sprite lookup. `visual_quad.rs` remains the GPU quad
  allocation entrypoint.
- The pass was behavior-neutral. No Scene JSON schema, keybinding, app config,
  TTS/STT, compose, or render behavior changes were intentionally made.

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
- The clicked GameTerm app Scene Mode VOICEVOX path now reaches `TTS played`
  after boot option `1. Scene Mode + VOICEVOX`,
  `Debug -> Voice -> Fake Codex backend`, and a short `hi` prompt.
- Before the explicit launch-config fix, option 1 reached Scene Mode but Scene
  status still reported `TTS disabled` and no VOICEVOX command spawned.
- The expected Scene voice path is:
  Composer reply -> speakable prose segment -> Rust Scene TTS worker ->
  optional CT2/helper translation command -> Rust VOICEVOX HTTP
  `audio_query`/`synthesis` -> temporary WAV -> waited `afplay` -> cleanup.
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
- TTS blocks are still not allowed to hide completed Codex text while audio is
  delayed. The current implementation records speaking/done state and
  diagnostics; a future pass can decide whether to visually highlight the active
  block or delay only the first block behind an explicit setting.

Scene asset editor status:

- `scene_asset_edit` is now a deterministic Rust-only terminal image editor
  substrate. It can sample, run pipelines, mask/cutout, fill/paint/clone/draw,
  crop/pad/transform, adjust/filter, composite, and render character state
  sheets.
- Primitive tightening is first-pass complete: validation, explicit
  acceptance, durable mask roundtrip, protected-region assertions, and richer
  review previews are implemented and documented.
- Local Kiki smokes wrote artifacts `24` through `40` under:
  `/Users/julianabeleda/Desktop/gameterm-vn-ai-emotion-sprites/Image Editor/Transformation`.
- Repo-safe primitive smokes wrote validation, preview, mask, composite,
  review, and accepted-output artifacts under:
  `/tmp/gameterm-scene-asset-primitive-smoke`.
- The scope doc marks the first-pass non-GUI substrate as 100% complete. The
  primitive-tightening scope marks the safe no-GUI loop as first-pass complete.
  The user has paused GUI work for now; remaining GUI surface area is file
  browser, point picking, lasso/polygon drawing, drag handles, live previews,
  and state/timeline UI.
- Optional ML helpers such as detection, matting, upscaling, or inpainting are
  explicitly post-100 extensions.

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

Latest refactor verification:

```sh
cargo test -p gameterm-gui visual_speech_blocks
cargo test -p gameterm-gui visual_tts_
cargo test -p gameterm-gui compose_backend
cargo test -p gameterm-gui codex_
cargo test -p gameterm-gui visual_stt_
cargo test -p gameterm-gui scene_debug_menu
cargo test -p gameterm-gui scene_voice_debug
cargo test -p gameterm-gui scene_voice
cargo test -p gameterm-gui visual_quad
cargo test -p gameterm-gui vn_panel
cargo test -p gameterm-gui stage_displayable
cargo test -p gameterm-visual
cargo test -p gameterm-gui --bin gameterm-gui
cargo check -p gameterm-gui
git diff --check
```

Results:

- `gameterm-visual`: 186 passed, 0 failed.
- `gameterm-gui --bin gameterm-gui`: 155 passed, 0 failed.
- `cargo check -p gameterm-gui`: passed with only known existing warnings.
- Focused GUI filters passed.

Latest TTS polish verification:

```sh
cargo test -p gameterm-visual vn_text
cargo test -p gameterm-gui visual_speech_blocks --bin gameterm-gui
cargo test -p gameterm-gui visual_tts_ --bin gameterm-gui
cargo test -p gameterm-gui scene_debug_menu_tts_test --bin gameterm-gui
git diff --check
```

Results:

- `vn_text`: 4 passed, 0 failed.
- `visual_speech_blocks`: 6 passed, 0 failed.
- `visual_tts_`: 10 passed, 0 failed.
- `scene_debug_menu_tts_test`: 1 passed, 0 failed.
- `git diff --check`: clean.

Latest asset primitive verification:

```sh
cargo test -p gameterm-visual asset_edit
cargo check -p gameterm-visual --examples
cargo test -p gameterm-visual
git diff --check
```

Results:

- `asset_edit`: 52 passed, 0 failed.
- `gameterm-visual`: 239 passed, 0 failed.
- `cargo check -p gameterm-visual --examples`: passed.
- `git diff --check`: clean.

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
6. Open `Debug -> Voice -> Fake Codex backend` to toggle between the configured
   compose backend and deterministic `Fake Codex` for TTS/debug testing.

This is still a one-shot local Codex bridge. It does not yet preserve Codex
session identity, stream progress into the dialogue panel, or support
`codex exec resume`.

Scene asset editing now has a first complete non-GUI command substrate. A GUI
would add interaction on top of existing Rust commands instead of adding new
image semantics.

## Recommended Next Actions

1. Decide whether the next asset-editor step is GUI:
   - use [Scene Asset Editor Cookbook](gameterm-scene-asset-editor-cookbook.md)
     to rerun terminal commands
   - GUI-only scope should focus on point picking, lasso/polygon drawing, live
     previews, file picking, command menus, and state/timeline controls
   - do not add ML into the first GUI pass unless explicitly scoped
2. Run the live TTS dogfood pass:
   - launch the installed app with `make dev-app-open`
   - choose `1. Scene Mode + VOICEVOX`
   - use `Debug -> Voice -> Test TTS playback`
   - send one real Codex prompt and one fake-Codex prompt
   - confirm text remains visible, speech does not overlap, and diagnostics
     identify translation/synthesis/player timing
3. Decide whether to add visible speaking-block polish:
   - current state marks blocks internally and in diagnostics
   - optional follow-up is visual highlighting for the currently speaking block
   - optional first-block reveal delay must stay config/debug-controlled, not
     the default
4. Reduce remaining VOICEVOX latency:
   - local CTranslate2 translation is now wired and avoids `codex exec`
   - current Scene test phrase benchmark: wrapper generation `1.617s` before
     playback, down from roughly `6.1s` before audio was ready with Codex
     translation
   - Rust now owns the persistent Scene TTS worker and VOICEVOX HTTP request
     path
   - next bottleneck is translation helper process startup plus VOICEVOX
     synthesis; pure Rust in-process translation remains deferred
5. Decide the next Scene product work lane:
   - product: app tiling and desktop actions
   - product: Arkey-style capability routing
   - refactor: only scoped cleanup that directly supports the next product lane
   - tooling: table-driven cleanup for Scene shell helpers
6. Persistent Codex sessions stay deferred unless disconnected-session resume
   becomes a real dogfood problem.
7. Decide whether progress/streaming events should render into Scene Mode or
   stay a follow-up.
8. Keep app-launch config behavior explicit; do not silently enable network
   backends without user/app config.
9. Keep future commits separated by concern.

Commit discipline:

- Keep separate commits by concern.
- Use established prefixes such as `[docs]`, `[gui]`, `[visual]`, and `[test]`.
- Do not mix formatting-only changes with behavior changes.
- Before committing, run `git diff --check` and targeted tests.
- Treat pre-existing warning noise as separate from Scene Mode failures.
