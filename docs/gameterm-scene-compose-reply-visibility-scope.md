# GameTerm Scene Mode Compose Reply Visibility Scope

Status: SCOPED.

This scope follows the June 9, 2026 future-turn rendering audit. It fixes the
confirmed defect that successful compose replies can be partially or completely
invisible in the Scene transcript, plus two adjacent lifecycle defects found in
the same audit. It is a correctness pass on the compose display pipeline, not a
visual redesign and not a latency pass.

## Audit Summary

Observed while dogfooding: first turn renders and speaks correctly; future
turns sometimes show the prompt but never the reply, show it late, or feel
stalled. "Future turn" means any non-first turn, not specifically turn 2.

Root cause (confirmed by failing reproduction tests, since reverted):

The visible transcript blocks are derived from the TTS speech filter.
`stamp_runtime_blocks` in `gameterm-gui/src/overlay/visual_compose_result.rs`
builds the display block list exclusively from
`extract_speakable_segments(...)` in
`gameterm-gui/src/overlay/visual_speech_blocks.rs`. That filter exists to
decide what is worth speaking aloud. It drops:

- fenced code blocks (`strip_fenced_code`)
- `TechnicalSkipped` lines (`dialogue_line_is_technical`)
- machine-oriented prose lines (`is_machine_oriented_line`): lines starting
  with `{`, diff markers, path-like lines, identifier-heavy lines, lines over
  45% punctuation
- lines whose cleaned speakable text is empty

Whatever survives becomes both the speech queue and the visible reply.
Consequences, all reproduced against `apply_compose_backend_result`:

1. A successful reply that is entirely technical (a path, a code fence, JSON)
   produces zero speakable segments. `stamp_runtime_blocks` then calls
   `mark_compose_succeeded(speaker, "")`, and
   `push_message_with_speaker` silently discards empty text. The turn ends
   Succeeded with no assistant message in history. Nothing renders; nothing
   speaks.
2. A mixed reply (prose plus a path or code) renders only the speakable
   lines on the staged VN path. The full text exists in the scene dialogue
   patch applied moments earlier, but the VN dialogue panel renders the
   compose transcript instead, so the dropped lines are never shown.
3. A structured payload with no dialogue text (for example
   `{"status":"Scene updated"}` or a patch-only reply) resolves to
   `StructuredComposeOutcome::NoReply`, which also calls
   `mark_compose_succeeded("Scene", "")` and leaves no visible trace of the
   turn.

Why it presents as a future-turn problem: `backend_prompt_with_context` in
`gameterm-visual/src/compose_state.rs` only wraps the prompt on non-first
turns. The wrapper tells Codex to keep technical details visible and mentions
structured Scene Mode JSON patches. Future turns are therefore exactly when
Codex starts returning paths, code, JSON, and patch-only output - the shapes
the display filter eats. First-turn prompts tend to get prose and always work.
The bug is response-shape probability, not turn-count state.

Adjacent defects confirmed in the same audit:

- Busy-submit divergence: a typed Enter while the backend is running calls
  `runtime.mark_compose_failed("Compose backend is already running")` in
  `visual_loop.rs`, which flips the running turn's phase to Failed and injects
  an Error block into the live turn. The voice path
  (`apply_stt_result`) handles the same situation with a status line only.
  The typed path should match the voice path.
- Perceived stall: each turn spawns a fresh `codex exec` with a growing
  context prompt, no streaming, and a 90s default timeout. This is real
  latency, not a rendering bug. Session reuse is already scoped in
  `docs/gameterm-scene-codex-session-bridge-scope.md` and stays out of scope
  here.

Ruled out by the audit (no work needed): reveal/tick coupling (the overlay
loop drains channels and advances reveal every poll cycle; `poll_input` uses a
real 100ms `recv_timeout`), TTS worker blocking (unbounded channels, isolated
thread, correct generation guards), typed/voice backend-prompt divergence
(both compute `compose_backend_prompt` before `mark_compose_running`), and
scrollback/windowing (`reset_to_bottom` fires on completion, STT results, and
every reveal tick).

## Goals

- Every successful compose reply leaves at least one visible assistant or
  system message in the transcript, regardless of reply shape.
- Technical content (paths, code, JSON, diffs) renders in the transcript even
  when it is excluded from speech.
- TTS keeps speaking only the speakable subset, mapped to the correct visible
  blocks for Speaking/Done reveal acceleration.
- Patch-only and status-only structured replies leave a short visible system
  trace instead of nothing.
- Typed busy-submit behaves like voice busy-submit: status feedback, no
  corruption of the running turn.
- Future-turn coverage in tests means turns 3 and 4, not just turn 2.

## Non-Goals

- No Codex session reuse or streaming (see
  `docs/gameterm-scene-codex-session-bridge-scope.md`).
- No change to the speech filter's judgment about what is speakable.
- No change to the fake-stream reveal cadence or the 100ms overlay tick.
- No scene schema changes.
- No prompt-queueing while the backend is busy (status-only is the first
  pass; queueing is a possible follow-up).
- No VN panel layout or visual redesign.
- No changes to `backend_prompt_with_context` content.

## Coding-Principle Constraints

Fork discipline:

- All changes stay inside Scene Mode compose code in `gameterm-gui` and, where
  needed, `gameterm-visual` compose state.
- No upstream terminal, mux, or renderer behavior changes.

Centralization / orthogonality:

- Display content and speech content are distinct concerns and must stop
  sharing one filter. The speech filter remains the single authority for what
  is spoken; the dialogue block classifier
  (`gameterm_visual::vn_text::dialogue_text_blocks`) remains the single
  authority for what is displayed.
- The invariant "a succeeded turn with non-empty reply text produces at least
  one history message" should be encoded where messages are created, not
  left to caller discipline.

Commit discipline:

- `[gui]` for compose result/display pipeline behavior fixes.
- `[visual]` only if compose-state methods change.
- `[test]` for verification-only additions.
- `[docs]` for this scope and handoff updates.
- No NFC mixing; each lane below is a separate commit.

## Lanes

### Lane 1: Split Display Blocks From Speakable Segments

Type: behavior fix. This is the core fix.

Target: `gameterm-gui/src/overlay/visual_compose_result.rs`
(`stamp_runtime_blocks`), `gameterm-gui/src/overlay/visual_speech_blocks.rs`.

Design:

- Build the visible block list from the full reply text using
  `dialogue_text_blocks(&dialogue_text)` (already public in
  `gameterm_visual::vn_text` and already used by the renderer's wrapping
  path), keeping all non-blank blocks including `TechnicalSkipped` lines and
  code content. Group consecutive lines into display blocks using the same
  paragraph boundaries the speech extractor uses, so block granularity stays
  comparable to today.
- Keep `extract_speakable_segments` unchanged as the speech authority.
- Map each speakable segment to the ordinal of the display block containing
  its source line. Display blocks with no speakable content simply get no TTS
  segment; they stay `Queued` and reveal via the existing tick path
  (visible-by-default since `45a643615`).
- `mark_compose_succeeded_blocks` keeps its current signature: it receives the
  display blocks and returns `(turn_id, block_index)` ids; the gui side
  assigns ids to segments by display-block ordinal as it does now.
- Delete the `segments.is_empty()` early-return that calls
  `mark_compose_succeeded(speaker, "")`. With display blocks derived from the
  reply text, an empty display list can only mean empty reply text, which is
  Lane 2's case.

Invariant to encode (gameterm-visual, small change if Lane 1 needs it):
`mark_compose_succeeded` and `mark_compose_succeeded_blocks` with non-empty
input must always push at least one message. If a guard is added, it belongs
in `compose_state.rs`, not in callers.

Acceptance:

```sh
cargo test -p gameterm-visual
cargo test -p gameterm-gui compose --bin gameterm-gui
```

New tests (in `visual_tests.rs` unless noted):

- technical-only reply (single path line) appears in history and in the
  staged VN frame after reveal
- code-fence-only reply appears in history and frame
- mixed prose-plus-path reply renders both lines in the staged frame
- speakable segments still exclude the technical lines (speech filter
  unchanged)
- TTS Started/Finished for a prose block still accelerates reveal of the
  correct block when technical blocks sit between prose blocks

### Lane 2: Visible Trace For No-Reply Outcomes

Type: behavior fix.

Target: `gameterm-gui/src/overlay/visual_compose_result.rs`
(`apply_compose_backend_result`, `StructuredComposeOutcome::NoReply` arm).

Design:

- When a structured reply applies a patch or status but carries no dialogue
  text, push a short System message instead of an empty success: prefer the
  applied patch status (for example `Scene updated`), falling back to the
  backend's succeeded status string.
- Route it through `mark_compose_succeeded("Scene", <status text>)` so the
  existing System-role rendering and history cap apply.
- No TTS for these traces.

Acceptance:

```sh
cargo test -p gameterm-gui compose --bin gameterm-gui
```

New tests:

- `{"status":"Scene updated"}` reply leaves a visible System line
- patch-only reply (entity update, no dialogue) leaves a visible System line
- the System line does not enqueue TTS segments

### Lane 3: Typed Busy-Submit Parity

Type: behavior fix.

Target: `gameterm-gui/src/overlay/visual_loop.rs` (typed submit
`Submitted(prompt)` busy branch).

Design:

- Replace `mark_compose_failed("Compose backend is already running")` with
  the voice path's behavior: `mark_action_status` only (for example
  `Compose busy: finish the current reply first`), keep the dock buffer
  intact (already the case), do not touch compose phase or history.
- Keep the render call so the status line updates immediately.

Acceptance:

```sh
cargo test -p gameterm-gui compose --bin gameterm-gui
```

New tests:

- typed submit while running leaves phase Running, adds no Error message,
  preserves the dock buffer
- voice and typed busy paths produce equivalent runtime lifecycle calls
  (parity assertion at the drain/apply level)

### Lane 4: Future-Turn Regression Tests

Type: verification only.

Target: `gameterm-visual/src/scene_runtime/tests/mod.rs`,
`gameterm-gui/src/overlay/visual_tests.rs`.

Add, named for future turns generally (not "second turn"):

- a three-to-four-turn lifecycle test driving
  `mark_compose_running` -> `apply_compose_backend_result` per turn with TTS
  enabled (`voice_block_sync = true`), asserting after reveal that the staged
  frame contains the last prompt and last reply, with at least one
  technical-only turn in the middle
- a reveal-independence test: future-turn blocks become fully visible through
  `advance_compose_reveal` ticks alone, with no `mark_compose_block_*` calls
- a scrollback test: after a future-turn completion with the scroll offset
  deliberately raised, `drain_compose_results` resets offset to bottom and
  the new reply is inside the rendered window

Note: `staged_scene_future_turns_render_while_previous_voice_blocks_are_unfinished`
already covers turns 2-5 at the runtime level; the new tests cover the gui
result-application layer where the audit found the defects.

Acceptance:

```sh
cargo test -p gameterm-visual
cargo test -p gameterm-gui compose --bin gameterm-gui
cargo test -p gameterm-gui audit --bin gameterm-gui
```

### Lane 5: Verification And Smoke

Type: verification.

Run after lanes 1-4:

```sh
cargo test -p gameterm-visual
cargo test -p gameterm-gui compose --bin gameterm-gui
cargo test -p gameterm-gui speakable --bin gameterm-gui
cargo check -p gameterm-gui
ci/gameterm-scene-verify.sh --all
```

Live dogfood smoke from an interactive GUI session (Screen Recording
permission required):

```sh
ci/gameterm-scene-smoke.sh --launch --scenario vn-demo \
  --output /tmp/gameterm-scene-reply-visibility.png
```

Manual dogfood checklist for the live run, with the real Codex backend:

1. Turn 1: plain question; reply renders and speaks.
2. Turn 2: ask for a file path; reply renders the path (previously dropped).
3. Turn 3: ask for a code snippet; fence content renders (previously
   dropped); speech skips the code.
4. Turn 4: ask for a scene patch; visible System trace appears.
5. Type a prompt while turn 4 is still running: status shows busy, no Error
   block appears, the typed text survives in the dock.

## Done Means

- A succeeded compose turn can no longer end with zero visible messages.
- Technical and code content renders in the transcript; speech still skips it.
- Patch-only and status-only replies leave a visible System trace.
- Typed and voice busy-submit behavior match.
- Multi-turn (3+) coverage exists at the gui result-application layer.
- Focused tests, GUI check, Scene verifier, and the live dogfood checklist
  are recorded in the handoff.

## First Implementation Slice

1. `[gui] render full compose replies independent of speech filter` (Lane 1)
2. `[gui] add visible trace for patch-only compose replies` (Lane 2)
3. `[gui] align typed compose busy handling with voice path` (Lane 3)
4. `[test] cover future-turn compose reply visibility` (Lane 4)
5. `[docs] record compose reply visibility verification` (Lane 5 results,
   handoff update)

Lane 1 lands first; it is the fix that addresses the dogfooded symptom. Lanes
2 and 3 are small and independent. If Lane 1's display-block grouping needs a
`compose_state.rs` guard, that change ships inside the Lane 1 commit as
`[visual]` only if it is behavioral, otherwise it stays `[gui]`-side.
