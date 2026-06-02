# GameTerm Scene Mode Codex Session Bridge Scope

Status: FIRST PASS IMPLEMENTED.

This document scopes the next Scene Mode compose pass after the local backend
bridge. The goal is to make the staged VN compose dock able to run a real local
Codex CLI turn, render the answer as Scene dialogue, and preserve a path toward
session continuity without making Scene Mode silently send input to a networked
service.

This scope covers the next 1-4:

1. push the completed local compose bridge
2. scope the real Codex backend
3. implement a first Codex-backed pass
4. smoke test the VN compose loop end to end

## Current Baseline

Completed and pushed:

- `9ca0eaca5` `[visual] add Scene dialogue patch support`
- `cc655beda` `[gui] add Scene compose backend`
- `01a355fe8` `[test] add Scene compose smoke`
- `9ef53badc` `[docs] record Scene compose bridge pass`

Scene Mode currently supports:

- staged VN background and character rendering
- wide transparent VN dialogue box
- bottom compose dock with editable input, cursor movement, and history
- deterministic built-in compose backend
- explicit local process backend through `GAMETERM_SCENE_COMPOSE_BACKEND`
- backend stdout/stderr capture, timeout, clipping, sanitization, and dialogue
  patch rendering
- fullscreen `vn-compose` smoke scenario
- explicit Codex backend kind through
  `GAMETERM_SCENE_COMPOSE_BACKEND_KIND=codex`
- local Codex CLI command construction using `codex exec`,
  `--output-last-message`, `--json`, `-C`, sandbox, and approval flags
- fake-Codex deterministic smoke through `vn-compose-codex`

The first Codex session bridge pass is implemented as a one-shot local Codex CLI
backend. Persistent `codex exec resume`, session-id capture, streaming, and
approval UI remain deferred to the next pass.

## Local Codex Surface

Local CLI inspection found:

```text
codex exec [OPTIONS] [PROMPT]
codex exec resume [OPTIONS]
```

Useful options for this bridge:

- `--json` prints events as JSONL
- `--output-last-message <FILE>` writes the final assistant message
- `-C, --cd <DIR>` sets the working root
- `-s, --sandbox <read-only|workspace-write|danger-full-access>` controls
  sandbox mode
- `-a, --ask-for-approval <untrusted|on-request|never>` controls approval
  behavior
- `--skip-git-repo-check` allows non-git workspaces
- `--ephemeral` avoids persisted session files
- `resume` can continue a previous session by id or by picker/last mode

This scope should use the local CLI process contract first. A direct API backend
remains out of scope for this pass because model/API details are time-sensitive
and should be verified against official docs immediately before implementation.

## Product End Goal

When this pass is complete, the user should be able to:

1. open the VN demo Scene Mode
2. type a prompt in the bottom compose dock
3. press Enter
4. see Scene Mode show a running Codex status
5. receive the Codex final response in the VN dialogue box
6. continue typing the next prompt without leaving Scene Mode

The UI should feel like:

```text
Codex:
I inspected the roadmap. The next blocker is the Codex session bridge smoke.

Compose: what should we test next_
```

## Non-Goals

This pass should not attempt:

- raw interactive Codex TUI embedding inside Scene Mode
- streaming token-by-token rendering as the primary success criterion
- full approval UI for Codex tool calls
- direct OpenAI API integration
- multi-agent orchestration
- automatic network dispatch without explicit user/config opt-in
- replacing the terminal pane or shell

## Scope 1: Push State

Status: COMPLETE.

The local compose bridge commits are pushed to `origin/main`. The branch should
be clean before implementation begins.

Acceptance:

- `git status --short --branch` shows no local uncommitted work
- `main` is not ahead of `origin/main`

## Scope 2: Real Codex Backend Contract

Status: COMPLETE for the one-shot local CLI backend.

Add a named backend mode for Scene compose:

```text
GAMETERM_SCENE_COMPOSE_BACKEND=codex
```

or a more explicit flag:

```text
GAMETERM_SCENE_COMPOSE_BACKEND_KIND=codex
```

Recommended first-pass command shape:

```text
codex exec \
  --json \
  --output-last-message <tempfile> \
  -C <workspace> \
  -s read-only \
  -a never \
  <prompt>
```

The bridge should derive:

- workspace root from `GAMETERM_SCENE_COMPOSE_WORKSPACE`, active scene base dir,
  or current process cwd
- prompt from the compose submit event
- session id from the overlay/session id already passed to local backend env
- output from `--output-last-message` first, stdout fallback second

Security default:

- default sandbox should be `read-only`
- default approval should be `never`
- no dangerous bypass flags by default
- no shell expansion
- timeout remains required

Open questions to answer in implementation:

- Does `codex exec --json` expose a stable session id we can persist?
- Does `codex exec resume` accept a session id in the currently installed CLI?
- Should session resume be enabled in this pass or only recorded for the next
  pass?

First-pass decision: implement one-shot `codex exec` first. Record session
metadata only if the JSONL events expose it cleanly.

Acceptance:

- backend kind resolves deterministically from env/config
- missing `codex` binary fails with clear Scene dialogue
- Codex command argv is visible in debug/test output without secrets
- prompt is passed without shell quoting bugs
- final assistant text is read from `--output-last-message` when present
- stderr/nonzero/timeout are rendered as Scene error dialogue

## Scope 3: First Implementation Pass

Status: COMPLETE for backend selection, command construction, final-message
output handling, status/dialogue updates, and deterministic tests.

Implementation layers:

1. Backend selection
   - keep deterministic built-in backend as default
   - keep custom local command backend for explicit process tests
   - add explicit Codex backend kind

2. Codex command builder
   - build `Command` argv structurally
   - include `codex exec`
   - pass workspace with `-C`
   - pass `--output-last-message`
   - optionally pass `--json`
   - pass sandbox and approval defaults from env/config

3. Output parser
   - prefer final message file
   - fallback to sanitized stdout
   - cap output to existing compose output limit
   - preserve stderr diagnostics for failures

4. Scene state update
   - `speaker = "Codex"` for success
   - `speaker = "Scene"` or `speaker = "Codex Error"` for failure
   - `append_history = true`
   - status values: `Codex running`, `Codex succeeded`, `Codex failed`,
     `Codex timed out`

5. Tests
   - backend kind selection
   - codex argv construction
   - final-message-file preferred over stdout
   - missing binary or failed command becomes error dialogue
   - timeout still kills backend

6. Docs
   - update this scope status after implementation
   - update roadmap current status
   - update smoke report after live/manual smoke

Acceptance:

- existing `compose_backend_*` tests remain green
- new Codex backend tests do not require network/auth
- no test requires the real `codex` binary unless guarded as smoke/manual
- `cargo test -p gameterm-gui compose_backend` passes
- `cargo test -p gameterm-gui overlay::visual` passes
- `ci/gameterm-scene-verify.sh --all` passes

## Scope 4: Smoke Test

Status: COMPLETE for deterministic fake-Codex smoke.

Add or extend a smoke scenario:

```text
ci/gameterm-scene-smoke.sh --launch \
  --scenario vn-compose-codex \
  --vn-asset-source-root .cache/gameterm-scene/vn-assets \
  --allow-ai-assisted-vn-assets \
  --wait-before-capture 3 \
  --capture-timeout 12 \
  --output /tmp/gameterm-scene-vn-compose-codex-fullscreen.png
```

For CI-safe metadata checks, the scenario may use a fake Codex helper. For live
manual smoke, it should use the local `codex` binary if present and authenticated.

Smoke should verify:

- fullscreen staged VN still renders background and character
- compose dock accepts text
- submit enters running state
- Codex/fake-Codex output appears in dialogue box
- status shows success or a readable failure
- Esc/q cleanup still exits

Acceptance:

- deterministic fake-Codex smoke is available for verifier checks
- live Codex smoke command is documented
- screenshot path is recorded in the smoke report
- smoke does not require raw downloaded archives

## Completion Definition

This pass is complete when:

- local compose bridge remains pushed: yes
- Codex session bridge scope is committed: yes
- Codex backend kind is implemented: yes
- deterministic tests cover command building and output handling: yes
- fullscreen smoke has a fake-Codex scenario: yes
- manual live Codex smoke is deferred; fake-Codex smoke proves Scene Mode's
  backend contract without requiring local auth/config

## Next Pass After This

After this pass, the likely next scope is persistent sessions:

- capture Codex session id from JSONL if available
- persist session id per Scene overlay/session
- add `resume` support
- allow user-visible reset/new-session action
- consider streaming/progress event rendering
