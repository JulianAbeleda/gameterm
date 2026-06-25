# Scope: Download Japanese voice model + fail loudly

Add a Scene-menu option to install the English→Japanese translation model, and
make a missing model **fail loudly** instead of silently dropping the turn.

## Background / current behavior

Scene TTS pipeline: **translate** (EN→JA via CTranslate2) → **VOICEVOX**
synthesize → `afplay`.

- Translator: `ci/scene-tts/ct2-en-to-ja.sh`, installed by
  `ci/scene-tts/setup-ct2-en-ja.sh` — creates a venv under `.cache/scene-tts/`,
  pip-installs torch/ctranslate2/transformers, downloads + converts
  `staka/fugumt-en-ja` to CT2 int8 (~2 GB).
- Readiness: `ct2-en-to-ja.sh --ready` (exit 0 = ready).
- **Bug:** if the translator isn't installed, the TTS worker fails at the
  translate step and returns empty, so it bails **before ever calling VOICEVOX**.
  The engine gets zero requests, nothing surfaces, TTS silently does nothing —
  very hard to diagnose. (We already added `SceneTranslationConfig::Unavailable
  { reason }` in `visual_tts.rs`, which surfaces a "TTS failed: …" reason through
  the result — but it's not shown in the scene UI, and there's no install path.)

## Feature

1. **Menu option** on the boot/scene-start screen (`scene_runtime/shell.rs`
   menu, alongside New Game/Continue) and/or the "Scene voice diagnostics" Voice
   menu: when `GAMETERM_SCENE_TTS_BACKEND=voicevox` and `--ready` fails, show
   **"Download Japanese voice model (~2 GB)"**. Selecting it runs
   `setup-ct2-en-ja.sh` as a subprocess with **live status** in the UI
   (downloading → installing → converting → ready) and a clear success/failure
   result. Re-check `--ready` after; enable TTS without an app restart if
   feasible (re-resolve `SceneTranslationConfig` on next turn).
2. **Surface the silent failure:** when a turn can't translate because the model
   is missing, emit a visible non-fatal notice (toast/scene log) —
   *"Japanese voice unavailable — translation model not installed. Open the menu
   to download it."* — instead of dropping the turn. Log the structured reason
   (reuse the `Unavailable { reason }` we already produce).
3. **Gate it:** only show the download option when the voicevox backend is
   selected and the engine host is reachable (optionally reuse the `/version`
   check). Respect `GAMETERM_SCENE_TTS_TRANSLATION_BACKEND=off` (don't show it).
4. **Idempotent/safe:** if already installed, show **"Japanese voice: ready"**
   instead of the download action. Install writes only under `.cache/scene-tts/`
   (gitignored) — touch no tracked files.

## Integration points

- Readiness probe: shell out to `ct2-en-to-ja.sh --ready` (already used by
  `ct2_translation_command()` in `visual_tts.rs`).
- Install runner: `setup-ct2-en-ja.sh` as a long-running subprocess; stream
  stdout into a Scene status line (mirror the TTS-worker subprocess pattern).
- Menu surfaces: `scene_runtime/shell.rs` (boot menu) + `debug_menu.rs`
  `voice_debug_menu_lines` (Voice section).
- Notice: the scene toast/log path used elsewhere.

## Open questions

- Live progress granularity from `setup-ct2-en-ja.sh` (does it emit parseable
  phase markers, or do we just show "installing…"? May want to add phase echoes
  to the script).
- Hot-enable without restart vs. "installed — restart to use" (simplest first).

## Acceptance

Fresh machine, VOICEVOX env set, no model → menu shows the download option →
selecting it installs → next turn, the mini receives `audio_query`/`synthesis`
and the line is spoken. Missing-model turns show the notice, not silence.
