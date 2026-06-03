# GameTerm Scene Mode Handoff

This document is the current handoff for continuing Scene Mode work across
Codex sessions. Check this file first, then use the roadmap and scope docs for
deeper product context.

## Current Snapshot

- Date: 2026-06-03
- Branch: `main`
- Latest behavior commit: `f17b5bf17 [gui] validate Scene Codex config lazily`
- Remote state at handoff time: `main` is ahead of `origin/main` by 2 commits
- Worktree state at handoff time: clean after this handoff commit is created
- Local app bundle refreshed: `/Users/julianabeleda/Applications/GameTerm.app`
- Persistent Scene compose config:
  `/Users/julianabeleda/.config/gameterm/scene-compose.json`

Current user goal:

Keep moving Scene Mode toward a dogfoodable visual-novel-style surface where
the user can see Codex dialogue, type through a Composer dock, and use the
normal macOS GameTerm app without shell-only setup.

## Latest Commits

Recent committed work:

- `f17b5bf17 [gui] validate Scene Codex config lazily`
- `76ebbdfc2 [docs] record Scene real Codex dogfood pass`
- `4fe553bd1 [gui] make Scene Codex compose dogfoodable`
- `bbd785622 [docs] scope Scene real Codex dogfood pass`
- `4707ffed2 [visual] render compose prompts in VN dialogue`

Latest behavior change:

- Scene compose config now validates Codex-specific fields only when the
  selected backend is actually `codex`.
- Invalid stale Codex fields no longer disable `built_in` or `command`
  backends.
- Regression coverage proves `built_in` and `command` still work with unused
  invalid Codex settings.

Real Codex dogfood pass:

- Scene Mode can run one-shot local `codex exec` through the Composer dock.
- Submitted user prompts render in the dialogue box as a highlighted `>` line.
- Codex replies render below the submitted prompt in the dialogue box.
- The Composer dock clears and remains ready after submit.
- Finder/app launches can opt into Codex through `scene-compose.json`.
- Failure diagnostics classify common Codex CLI issues such as rate limit,
  auth/connect failures, missing binary, and timeout.

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
cargo test -p gameterm-gui visual_compose --bin gameterm-gui
cargo test -p gameterm-gui compose_backend --bin gameterm-gui
cargo check -p gameterm-gui
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

This is still a one-shot local Codex bridge. It does not yet preserve Codex
session identity, stream progress into the dialogue panel, or support
`codex exec resume`.

## Recommended Next Actions

1. Push the latest two local commits if the user wants the audit fix and
   handoff refresh on `origin/main`.
2. Scope persistent Codex sessions:
   - parse session/thread metadata from Codex JSONL if available
   - persist session identity per Scene overlay/session
   - add reset/new-session action
   - add `codex exec resume` support
3. Decide whether progress/streaming events should render into Scene Mode or
   stay a follow-up.
4. Keep app-launch config behavior explicit; do not silently enable network
   backends without user/app config.
5. Keep future commits separated by concern.

Commit discipline:

- Keep separate commits by concern.
- Use established prefixes such as `[docs]`, `[gui]`, `[visual]`, and `[test]`.
- Do not mix formatting-only changes with behavior changes.
- Before committing, run `git diff --check` and targeted tests.
- Treat pre-existing warning noise as separate from Scene Mode failures.
