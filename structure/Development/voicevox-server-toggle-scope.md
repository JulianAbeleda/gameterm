# Scope: VOICEVOX Local/Server toggle in the Voice menu

A yes/no toggle in the Voice menu to switch VOICEVOX between **Local** (an engine
on this machine) and **Server** (a remote engine, e.g. the Mac mini over
Tailscale at `100.94.210.27:50021`) — instead of hardcoding the host in config/env.

## Background / current behavior

- The TTS backend + VOICEVOX host are resolved **once at launch from env** in
  `visual_tts.rs` (`SceneVoicevoxConfig::from_env` → `GAMETERM_SCENE_TTS_VOICEVOX_HOST`
  /`_PORT`/`_SPEAKER`). To point at a different engine you edit env/config and
  relaunch.
- We already built the backbone: `overlay/voice_settings.rs` —
  `VoiceSettings { voicevox_host_mode: Local|Server, voicevox_server, … }` with
  `load_or_seed()` (persisted JSON, env-seeded; defaults **Local**, seeds
  **Server** if env already points at a remote) and `resolve_endpoint() ->
  (host, port)`. **It is not yet wired into `SceneVoicevoxConfig`.**

## Feature

1. **Voice-menu row:** "VOICEVOX host: Local / Server" — a yes/no (left/right or
   enter) toggle in `debug_menu.rs` `voice_debug_menu_lines`, dispatched in the
   gameterm-gui voice handler (where TTS-mute etc. toggle).
2. **Wire it through:** `SceneVoicevoxConfig` reads the effective endpoint from
   `VoiceSettings::resolve_endpoint()` (Local → `127.0.0.1:port`; Server →
   stored `voicevox_server`, default the env-seeded Mac mini). Toggling persists
   via `VoiceSettings::save_to_path()` and takes effect on the next turn (no
   restart).
3. **Server address:** seeded from the existing `…VOICEVOX_HOST` env on first run
   (auto-captures `100.94.210.27`); editable later. (Local-engine auto-launch is
   the separate Phase-2 management piece — out of scope here; this is just URL
   switching.)

## Integration points

- `overlay/voice_settings.rs` (done) → load in the overlay session; persist on
  toggle.
- `visual_tts.rs` `SceneVoicevoxConfig::from_env` → `from_env_and_settings`
  (endpoint from `resolve_endpoint()`, keep env as override/seed).
- `debug_menu.rs` Voice section + the gui voice toggle dispatch.

## Decision (recorded earlier)

Default **Local**; **Server** is opt-in. This is tasks #8/#9's menu-wiring half —
the persistence/env-seed/endpoint backbone is already committed.

## Acceptance

Voice menu shows "VOICEVOX host: Local/Server"; flipping to Server routes the
next turn's `audio_query`/`synthesis` to the stored server (the mini over
Tailscale); flipping to Local routes to `127.0.0.1`; choice persists across
restarts.
