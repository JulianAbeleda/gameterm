# GameTerm Scene Mode Codex Compose Bridge Scope

Status: SCOPED.

This document scopes the first pass for using a Codex-like assistant inside
Scene Mode through the staged VN compose dock.

The goal is to make Scene Mode feel like a terminal-native assistant surface:
the user types in the bottom dock, submits with Enter, and assistant output is
rendered as readable dialogue in the transparent VN text box while the staged
background and character remain visible.

This is not a full replacement for the terminal pane or a direct Codex protocol
implementation yet. The first pass should prove the input/output loop with a
small Rust-owned process bridge, then leave room for a direct Codex backend.

## Reference Repos

Local references inspected:

- `/Users/julianabeleda/env/deepseek`
- `/Users/julianabeleda/env/pkos_v0.2/arkey-rs`
- `/Users/julianabeleda/env/pkos_v0.2/arkey-core`

These references are used for architecture and behavior, not copied code.

### DeepSeek DockedComposer

Relevant files:

- `/Users/julianabeleda/env/deepseek/src/input/composer.rs`
- `/Users/julianabeleda/env/deepseek/src/repl/chat.rs`

Useful patterns:

- `DockedComposer` owns a prompt, editable buffer, cursor, history, slash
  completion state, approval modal, progress rows, stream buffer, and transcript
  view state.
- The dock remains mounted while output is printed above it.
- `poll_action()` returns structured actions: submit, approval, cancel, exit.
- Enter submits, Shift/Alt+Enter inserts a newline, Esc cancels, Ctrl-D exits,
  Ctrl-C clears, Ctrl-W deletes a previous word, arrows move cursor/history.
- Large pasted input is compacted into markers while preserving expanded
  context for submission.
- Progress is rendered in the dock while work is active.
- Streaming output is written above the dock without unmounting the composer.
- Approval prompts are dock-native so background workers do not compete with
  stdin.

Scene Mode should adopt the behavioral shape:

- compose input returns structured events, not direct runtime mutation
- the dock stays mounted while backend output changes the scene
- active work has a visible running/progress phase
- approval and future tool actions must be dock-native, not raw stdin prompts

### Arkey Bottom Composer

Relevant files:

- `/Users/julianabeleda/env/pkos_v0.2/arkey-rs/src/input.rs`
- `/Users/julianabeleda/env/pkos_v0.2/arkey-rs/src/render/prompt.rs`
- `/Users/julianabeleda/env/pkos_v0.2/arkey-rs/src/render/terminal_renderer.rs`
- `/Users/julianabeleda/env/pkos_v0.2/arkey-core/src/runtime.rs`

Useful patterns:

- `NativeInputBuffer` separates idle key actions from running key editing.
- The running phase still lets the user edit the next draft.
- `InlineDockPhase` changes footer/status copy without changing dock geometry.
- `inline_dock_height()` is stable, so layout does not jump between idle and
  running.
- `TerminalPhase` models prompt idle, context scan, response render, and prompt
  resume.
- Repaint scope is explicit: dock-only updates during active work, scanner-only
  updates during progress, full repaint when necessary.
- Cursor math accounts for prompt width, Unicode display width, wrapping, and
  ANSI-stripped prompt labels.
- The runtime facade exposes `run_turn(prompt)` and observed phase callbacks,
  separating UI from backend execution.

Scene Mode should adopt the behavioral shape:

- keep a stable compose dock height
- support editing a next draft while the backend is running
- use explicit phases instead of one boolean busy flag
- preserve Scene layout while only the dock/status changes
- separate compose UI state from backend runtime state

## Current Baseline

Already implemented:

- staged VN rendering can show background and character sprites
- staged Scene Mode renders a wide bottom VN text box
- the upper-left debug/status text is intentionally still visible
- the GUI overlay has a typeable bottom compose dock
- compose input supports printable characters, Backspace, Delete, and Enter
- Enter currently stores the submitted text locally and updates runtime status
- Scene Mode already has patch inbox and mux patch transports
- Scene Mode already has process/agent lifecycle helpers and process state
- `RunCommand` actions can spawn commands in tabs or splits

Current limitation:

- compose submissions are not routed to any assistant/backend process
- assistant output is not streamed back into the VN dialogue box
- `VisualScenePatch` can update status/process/entity/variables but cannot yet
  append or replace dialogue directly
- while compose focus is active, printable keys such as `j`, `q`, and `r` are
  treated as typed text in staged VN mode

## End Goal

When this scope is complete, the user should be able to open a staged Scene
Mode VN demo and interact with an assistant-like backend:

```text
background + character stage

+-------------------------------------------------------------+
| Codex:                                                      |
| I can inspect this workspace. What would you like to check? |
+-------------------------------------------------------------+
Compose: look at the roadmap_
```

Submitting the compose dock should:

1. record the user message as a Scene runtime event
2. send the message to a configured backend
3. show a pending/running status while the backend works
4. capture backend output
5. render the output as assistant dialogue in the VN text box
6. keep the compose dock ready for the next message

## Product Contract

The first pass should provide a terminal-like conversation surface:

- The bottom compose dock is the focused input line for staged VN scenes.
- User text remains visible while editing.
- Enter submits non-empty input.
- Empty Enter may continue to fall through to the existing Scene action flow.
- Assistant replies appear in the VN dialogue frame, not only in status.
- Status should still show backend state: idle, running, succeeded, failed.
- Esc should close Scene Mode.
- Tab should still open the Tile Debugger.
- Debug/status text remains present until a later polish pass removes or
  relocates it.

The backend should be explicit and auditable. Scene Mode should not silently
send user input to a network service.

## Backend Strategy

### First Pass: Local Process Backend

Add a Scene compose backend that runs a configured local command.

Candidate behavior:

```text
compose submit
-> spawn command with submitted prompt
-> collect stdout/stderr
-> update runtime process state
-> append assistant dialogue from stdout
```

The first implementation can be one-shot per submitted prompt. It does not need
to maintain a persistent PTY session.

Recommended backend command contract:

```text
COMMAND receives:
  GAMETERM_SCENE_COMPOSE_PROMPT=<submitted user text>
  GAMETERM_SCENE_COMPOSE_SESSION_ID=<stable overlay/session id>
  GAMETERM_SCENE_PATH=<scene json path when available>
  GAMETERM_SCENE_PANE_ID=<overlay pane id when available>

COMMAND may also receive the prompt as argv/stdin, but env-first keeps the
first pass simple and avoids shell quoting bugs.

COMMAND returns:
  stdout = assistant text
  stderr = diagnostics
  exit code 0 = succeeded
  nonzero = failed
```

The default development backend can be a repo helper that echoes a deterministic
assistant response. A later pass can configure it to call `codex`, another CLI,
or a direct API helper.

### Deferred: Persistent Process Backend

A persistent backend would keep one process/session alive across submissions.
That is closer to a terminal or live Codex chat, but it requires process
lifetime management, cancellation, buffering, and stream framing. Defer until
the one-shot loop is stable.

### Deferred: Direct Codex Backend

A direct Codex backend can use the same Scene compose contract after the local
process backend proves:

- compose input handling
- pending state
- assistant reply rendering
- error rendering
- smoke coverage

Direct Codex integration should be scoped separately because model/API details
change over time and need official API verification at implementation time.

## Runtime Model

Add a compose-specific runtime model to `gameterm-visual`.

Candidate types:

```rust
pub struct VisualComposeMessage {
    pub role: VisualComposeRole,
    pub text: String,
}

pub enum VisualComposeRole {
    User,
    Assistant,
    System,
    Error,
}

pub struct VisualComposeState {
    pub phase: VisualComposePhase,
    pub history: Vec<VisualComposeMessage>,
    pub last_prompt: Option<String>,
    pub last_reply: Option<String>,
}

pub enum VisualComposePhase {
    Idle,
    Running,
    Succeeded,
    Failed,
}
```

First-pass constraints:

- history should be capped to a small number of messages
- dialogue rendering should use the latest assistant/error message
- user messages can be shown in debug/report output first
- state must bump generation when changed
- empty or whitespace-only prompts must not dispatch

## Patch Model

Extend `VisualScenePatch` or add a parallel runtime method so external helpers
can update dialogue, not just status.

Preferred first pass:

```rust
pub struct VisualScenePatch {
    ...
    pub dialogue: Option<VisualSceneDialoguePatch>,
}

pub struct VisualSceneDialoguePatch {
    pub speaker: String,
    pub text: String,
    pub append_history: bool,
}
```

Validation:

- speaker must be non-empty after trim
- text must be non-empty after trim
- applying a dialogue patch updates the active displayed dialogue
- optionally append to dialogue history

This lets local helpers and mux patches reuse the same path as the compose
backend.

Alternative:

- add runtime-only `mark_compose_reply(...)` methods and defer patch schema
  changes

The patch approach is better because Scene Mode already has a working external
transport story.

## GUI Overlay Model

Extend `SceneComposeDock` in `gameterm-gui/src/overlay/visual.rs`.

First-pass behavior:

- `SceneComposeDock::handle_key` returns a structured submit event instead of
  only mutating status
- overlay dispatches non-empty submissions to a compose backend
- dock tracks local edit buffer and last submitted text
- while backend is running, status shows `Compose running: ...`
- if another prompt is submitted while running, first pass should either:
  - reject with `Compose busy`, or
  - queue exactly one pending prompt

Recommended first pass: reject while busy. It is simpler and avoids accidental
parallel requests.

Keyboard behavior:

- Printable characters edit the compose buffer.
- Backspace removes one char.
- Delete clears the buffer.
- Enter submits when non-empty.
- Empty Enter falls through to existing Scene Mode input.
- Esc closes Scene Mode.
- Tab toggles debugger.
- Future scope can add focus modes so `q`, `r`, `j`, and `k` can be used as
  Scene controls when the compose dock is not focused.

## Process Execution Model

Use Rust process execution inside the GUI overlay for the first pass.

Do not shell-expand user input.

Recommended shape:

- command argv comes from scene config or an environment variable
- submitted text is passed through environment or stdin
- stdout is bounded to a max number of bytes/chars
- stderr is captured for failure diagnostics
- timeout is configurable or has a conservative default
- backend process is killed on timeout

Candidate config sources, in order:

1. scene metadata/mode field for `compose_backend`
2. environment variable such as `GAMETERM_SCENE_COMPOSE_BACKEND`
3. deterministic built-in echo backend for tests/smoke

The first implementation should support the environment variable and built-in
test backend. Scene schema support can follow once behavior is proven.

## Assistant Text Rendering

The VN dialogue box should render:

- assistant replies with speaker `Codex` by default
- errors with speaker `Scene`
- wrapped text inside the existing bottom dialogue frame
- no raw ANSI escape sequences
- no unbounded stdout

Sanitization:

- strip or replace control characters except normal newlines/tabs
- cap output length
- collapse excessive blank lines for first pass

If the backend returns structured JSON later, the text renderer should still
have a plain text fallback.

## Safety And Privacy

Scene Mode must not silently call a networked assistant.

First-pass requirements:

- backend command is explicit
- no default network backend
- prompt text is not logged to disk by default
- smoke uses deterministic local helper output
- failures show a short diagnostic without dumping excessive stderr

Deferred policy:

- allow/deny lists for backend commands
- per-workspace backend trust
- persisted conversation history
- direct API key handling

## Testing Scope

Unit tests:

- compose submit creates a structured submit event
- whitespace-only submit does not dispatch
- running backend blocks or rejects a second submit
- backend success updates assistant dialogue
- backend failure updates error dialogue/status
- stdout/stderr are clipped and sanitized
- dialogue patch validates speaker/text
- dialogue patch appends history when requested

Integration/smoke:

- deterministic backend helper returns a known reply
- `vn-demo` smoke can type a prompt and capture the assistant reply
- capture remains fullscreen
- staged background and character still render
- bottom compose dock remains visible after reply

## Implementation Slices

### Slice 1: Dialogue Patch Surface

- add `VisualSceneDialoguePatch`
- validate non-empty speaker/text
- apply dialogue patch to runtime
- expose dialogue patch in debug/report output as needed
- tests for patch validation and application

### Slice 2: Compose Submit Event

- refactor `SceneComposeDock::handle_key` to return edit/submit actions
- keep existing edit behavior
- preserve Escape/Tab behavior
- tests for typed submit and empty Enter fallthrough
- add cursor-aware buffer editing: left/right/home/end
- add history recall for submitted prompts
- keep dock geometry stable across idle/running phases

### Slice 3: Local Compose Backend

- add a small backend runner in the GUI overlay
- pass prompt safely through environment or stdin
- collect bounded stdout/stderr
- timeout or failure path updates runtime
- tests around command result handling where possible

### Slice 4: Runtime Reply Wiring

- on submit, mark compose running
- on success, set assistant dialogue
- on failure, set error dialogue/status
- record runtime events
- cap compose history
- allow drafting the next prompt while the current backend turn is running
- reject or queue submit while running; first implementation may reject, but
  the UI should not discard the draft

### Slice 5: Helper And Smoke

- add deterministic helper command for smoke
- add or extend a VN smoke scenario that types a prompt
- capture fullscreen output
- record result in smoke report

### Slice 6: Docs And Roadmap

- update roadmap current status
- document backend configuration
- document that direct Codex/API backend is deferred

## Acceptance Criteria

The scope is complete when:

- a staged VN Scene Mode window accepts typed compose input
- submitting the input invokes a deterministic local backend
- backend output appears as VN dialogue in the bottom text box
- the compose dock remains usable after the reply
- errors render as readable Scene/Error dialogue
- tests cover runtime patching, compose submit behavior, and backend result
  handling
- fullscreen smoke captures the background, character, assistant reply, and
  compose dock

## Non-Goals

- direct OpenAI/Codex API integration in this first pass
- persistent PTY chat session
- streaming token-by-token output
- full terminal emulator inside the compose dock
- automatic network calls
- persisted chat history
- replacing the underlying shell pane

## Open Follow-Up Questions

- Should compose backend config live in scene mode metadata or app config?
- Should user prompts also appear in the VN dialogue history, or only in the
  debug report?
- Should the dock eventually support focus modes so navigation keys can switch
  between text entry and Scene controls?
- Should direct Codex integration reuse the process backend by shelling out to
  a CLI, or use a Rust API client with a separate security model?
