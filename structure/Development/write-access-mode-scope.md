# Scope: Full write-access mode toggle (F10)

A sticky toggle that puts the Scene compose agent into **full write-access mode**
so it can perform real terminal/system actions (e.g. `open -a Music`, open a
browser/URL) — off by default, clearly indicated, user-controlled.

## Background / current behavior

- The Scene compose backend is **Codex**, configured via env in
  `visual_compose_backend.rs`: `…CODEX_BIN`, `…CODEX_WORKSPACE`,
  **`…CODEX_SANDBOX`**, **`…CODEX_APPROVAL`**, timeout, reasoning. So the
  permission/sandbox surface already exists — it's just **env-fixed at launch**,
  not runtime-toggleable.
- Input pattern to mirror: **press-to-talk** (`overlay/visual_voice_hold.rs`) —
  a held key drives a transient mode with a visible state. The user wants the
  same plumbing but as a **sticky toggle on F10** (press once = on, press again =
  off), not press-and-hold.

## Feature

1. **F10 toggle:** bind F10 (in the scene input map / the same layer as voice
   hold) to flip a persistent `write_access: bool`. Sticky, not held.
2. **Effect:** when ON, compose runs use a **full-access** Codex profile —
   sandbox = full/workspace-write and approval = never (vs. the default
   restricted/read-only profile) — so the agent's emitted actions (shell
   `open -a Music`, `open <url>`, etc.) actually execute. When OFF, the agent
   stays in the safe default and cannot take system actions.
3. **Always-visible indicator:** like press-to-talk shows its state, show a clear
   badge/status ("WRITE ACCESS: ON") whenever it's enabled, so the user always
   knows the agent can act. Default **OFF** every launch unless explicitly
   persisted.

## Execution model (confirmed by tracing the code)

GameTerm does **not** parse the agent's reply and run commands itself. The
compose backend spawns the **`codex` CLI** (`run_codex_compose_backend()` in
`visual_compose_backend.rs`), and **Codex's own agent loop** executes any
shell/tools (`open -a Music`, `open <url>`) under the `sandbox`/`approval` policy
GameTerm passes it. (`SceneComposeAction` in `visual_compose_dock.rs` is only UI
key-handling, not action execution.)

**So the entire feature reduces to: choose which `CodexComposeConfig`
sandbox/approval profile is passed per compose run, based on the F10 flag** —
permissive when ON, the safe default when OFF. No new command-execution code in
GameTerm; the guardrail is Codex's sandbox. This is why the safety model below is
really "which Codex profile," not "how do we sandbox shell ourselves."

## Integration points

- Input: scene input map / `visual_voice_hold.rs` sibling — add an F10 sticky
  toggle action.
- State: a `write_access` flag on the overlay/compose state (mirrors the
  voice-hold state field).
- Backend: `visual_compose_backend.rs` — select the Codex sandbox/approval
  profile from the flag at compose-run time instead of only from env. Keep the
  env values as the default/floor.
- Indicator: the status/overlay render path (where mode badges draw).

## Safety model (this is the load-bearing part)

This grants the agent real exec power, so:
- **Default OFF**; requires an explicit, deliberate F10 each session.
- **Unmissable indicator** while ON.
- The "full access" profile should still be a *named, bounded* Codex profile
  (decide exact sandbox/approval values), not literally unrestricted, unless you
  want that.
- Consider an allowlist or a confirm step for clearly-destructive actions
  (decide below).

## Open questions / decisions

- Exact "full-access" Codex values: `sandbox=danger-full-access` + `approval=never`,
  or `workspace-write` + `on-request`? (Trade convenience vs. blast radius.)
- Scope of allowed actions: any shell the agent emits, or an allowlist of `open`/
  app-launch verbs? (`open -a`, `open <url>` only, vs. arbitrary commands.)
- Persist the toggle across launches, or always default OFF on start? (Recommend
  always-OFF-on-start for safety.)
- Confirmation prompt for destructive/irreversible actions even when ON?

## Acceptance

F10 toggles a visible "WRITE ACCESS" state. With it ON, asking the agent to
"open Apple Music" / "open <url>" actually launches it; with it OFF, the agent
cannot run system actions. Default is OFF, clearly indicated when ON.
