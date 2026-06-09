# Scene Runtime A Refactor Scope

Date: 2026-06-09

## Purpose

This scope applies the updated
`structure/Development/coding-principles.md` to the remaining Scene Runtime
quality issue.

The issue is not raw line count by itself. The issue is that
`gameterm-visual/src/lib.rs` still carries multiple kinds of authority and
execution in one file:

- public facade exports
- central `SceneRuntime` state
- runtime lifecycle and action dispatch glue
- text-frame rendering
- VN dialogue layout projection and scroll metrics
- interactive debug menu rendering and input
- render snapshot projection
- dialogue helper logic
- a large inline test module

That violates the principles most directly under:

- **Modularize**: execution surfaces are not narrow enough.
- **Orthogonalize**: runtime state, presentation, debug UI, VN projection, and
  tests are still fused.
- **Keep Public Surfaces Boring**: callers should see stable exported types,
  not the internal machinery.

The earlier audit fixes already addressed two other findings:

- commit discipline is now machine-enforced by `ci/check-commit-message.sh`
- VN asset `repo_policy` is now typed as `VnAssetRepoPolicy`

This scope is the remaining A-pass for the Scene Runtime surface.

## Current State

Line counts at scope time:

```text
6325 gameterm-visual/src/lib.rs
1391 gameterm-visual/src/scene_model.rs
 310 gameterm-visual/src/asset_edit.rs
2026 gameterm-gui/src/overlay/visual.rs
1061 gameterm-gui/src/termwindow/render/visual_quad.rs
```

The good precedent is `asset_edit.rs`: it is now a small facade over focused
modules. The Scene Runtime should follow that shape without forcing a behavior
change.

## Non-Goals

- No Scene JSON schema change.
- No visual redesign.
- No input/keybinding change.
- No TTS/STT/Codex behavior change.
- No upstream-wide config, env, mux, window, or CLI refactor.
- No deletion of tests to reduce line count.
- No public API break for existing `gameterm_visual::*` callers.
- No hidden behavior changes inside NFC commits.

The broader config/env/CLI critique from the external review is real, but it
crosses inherited WezTerm architecture and should be scoped separately under
`[config]`, `[repo]`, or entrypoint ownership after this Scene Runtime pass.

## Target Shape

Target module layout:

```text
gameterm-visual/src/
  lib.rs                         # facade exports and module declarations
  scene_model.rs                 # durable schema/model DTOs
  scene_runtime/
    mod.rs                       # SceneRuntime type, constructor, small public API
    command_options.rs           # command option filtering/projection
    lifecycle.rs                 # mode lifecycle, view toggles, reload state
    dialogue.rs                  # active dialogue/history helpers
    snapshot.rs                  # VisualRenderSnapshot projection
    text_frame.rs                # non-VN text frame and command selection render
    vn_frame.rs                  # staged VN text frame, scroll metrics, nameplates
    debug_menu.rs                # interactive debug menu render/input
  tests/
    mod.rs                       # test module root
    test_support.rs              # existing helpers
    validation.rs
    actions.rs
    runtime.rs
    rendering.rs
    vn.rs
    fixtures.rs
```

Expected `lib.rs` final role:

- declare modules
- re-export stable public types/functions
- keep minimal crate-local imports needed by test/support modules
- avoid owning runtime execution details

Target `lib.rs` size: roughly 300-600 lines. This is a guideline, not the
definition of quality. The real goal is that each concern has one obvious owner.

## Principle Mapping

### Centralize

Keep durable data shape in `scene_model.rs`.

Keep runtime authority in `scene_runtime/mod.rs`.

Keep VN layout math in `vn_layout.rs`; runtime modules should call it rather
than redefining ratios or rect math.

Keep VN text wrapping/block logic in `vn_text.rs`; runtime modules should call
it rather than duplicating transcript formatting.

### Modularize

Each runtime module should own one execution concern:

- lifecycle and view state
- dialogue state
- command option projection
- render snapshot projection
- text-frame presentation
- VN presentation
- debug menu presentation/input

### Abstract

The public crate surface should remain boring:

```rust
gameterm_visual::SceneRuntime
gameterm_visual::VisualScene
gameterm_visual::VisualRenderSnapshot
```

Internal modules may grow, but callers should not need to know the module
layout.

### Orthogonalize

Do not let debug menu logic own VN layout policy.

Do not let text rendering mutate runtime state.

Do not let snapshot projection decide input behavior.

Do not move tests only to chase LOC if that makes behavior harder to find.

### Encode Invariants

Do not replace typed fields with strings during extraction.

Do not widen private runtime fields to `pub` just to make modules compile.
Prefer child modules under `scene_runtime/` so they can share the runtime's
private state without exposing it to the crate.

### Test At The Boundary

Keep behavior tests at the boundary they protect:

- schema validation tests near model/validation behavior
- runtime action tests near runtime behavior
- VN text/render tests near text/VN frame modules
- fixture tests grouped by fixture loading behavior

## Implementation Lanes

### Lane 1: Runtime Module Root

Commit:

```text
[visual] NFC - move Scene runtime root
```

Move:

- `SceneRuntime` struct
- constructors
- simple accessors
- `VisualMode for SceneRuntime`
- direct helper functions that are only meaningful to the runtime root

Create:

```text
gameterm-visual/src/scene_runtime/mod.rs
```

Keep:

- existing public re-export from `lib.rs`
- behavior unchanged
- runtime fields private inside `scene_runtime`

Verification:

```sh
cargo check -p gameterm-visual
cargo test -p gameterm-visual demo_scene_validates runtime_toggles_debugger
git diff --check
```

Stop condition:

- if the move requires making many runtime fields `pub(crate)`, stop and
  re-scope. That would violate the "boring surface" and "contain authority"
  principles.

### Lane 2: Command Option Projection

Commit:

```text
[visual] NFC - move Scene command option helpers
```

Move:

- `command_options`
- `filtered_command_options`
- `command_option_matches_filter`

Create:

```text
gameterm-visual/src/scene_runtime/command_options.rs
```

Verification:

```sh
cargo test -p gameterm-visual command_options
cargo check -p gameterm-visual
```

Definition of done:

- command filtering behavior stays byte-equivalent
- no filtering rules are duplicated

### Lane 3: Lifecycle And View State

Commit:

```text
[visual] NFC - move Scene runtime lifecycle
```

Move:

- `toggle_debugger`
- command selection show/hide/toggle
- mode lifecycle hook methods
- reload failure and scene replacement state preservation
- generation bump and runtime event recording if they remain lifecycle-owned

Create:

```text
gameterm-visual/src/scene_runtime/lifecycle.rs
```

Verification:

```sh
cargo test -p gameterm-visual mode_lifecycle_hooks_update_status_and_generation
cargo test -p gameterm-visual reload_success_preserves_selected_entity_id
cargo test -p gameterm-visual reload_failure_updates_source_status_and_preserves_scene
cargo check -p gameterm-visual
```

Definition of done:

- generation semantics are unchanged
- reload and enter-hook behavior stay covered

### Lane 4: Dialogue Runtime Helpers

Commit:

```text
[visual] NFC - move Scene dialogue helpers
```

Move:

- `dialogue_index`
- `active_dialogue_line`
- `initial_dialogue_history`
- any dialogue-history helper currently stranded in `lib.rs`

Create:

```text
gameterm-visual/src/scene_runtime/dialogue.rs
```

Verification:

```sh
cargo test -p gameterm-visual dialogue
cargo test -p gameterm-visual story_state_import_restores_variables_and_dialogue
cargo check -p gameterm-visual
```

Definition of done:

- legacy `dialogue_speaker`/`dialogue` fallback remains unchanged
- indexed `dialogue_lines` behavior remains unchanged

### Lane 5: Render Snapshot Projection

Commit:

```text
[visual] NFC - move Scene render snapshot projection
```

Move:

- `render_snapshot`
- `render_tiles`
- `render_stage`
- `render_entities`
- `entity_mode`

Create:

```text
gameterm-visual/src/scene_runtime/snapshot.rs
```

Verification:

```sh
cargo test -p gameterm-visual snapshot
cargo test -p gameterm-visual render_snapshot_uses_stage_displayables_when_present
cargo check -p gameterm-visual
```

Definition of done:

- snapshot shape is unchanged
- stage displayable ordering remains deterministic
- tile fallback remains disabled for staged VN scenes

### Lane 6: Non-VN Text Frame Rendering

Commit:

```text
[visual] NFC - move Scene text frame rendering
```

Move:

- `render_text_frame`
- `render_text_frame_with_dialogue_scroll`
- `render_text_frame_with_dialogue_scroll_and_voice_hold`
- `render_scene` for non-staged grid mode
- `render_command_selection`
- test-only `render_debugger` if it remains tied to tile debugger tests

Create:

```text
gameterm-visual/src/scene_runtime/text_frame.rs
```

Verification:

```sh
cargo test -p gameterm-visual scene_frame
cargo test -p gameterm-visual command_selection
cargo test -p gameterm-visual debugger_frame_contains_scene_source_status
cargo check -p gameterm-visual
```

Definition of done:

- normal Scene Mode text frame stays byte-equivalent
- command selection rendering stays byte-equivalent
- test-only tile debugger rendering remains available to tests only

### Lane 7: VN Text Frame And Scroll Metrics

Commit:

```text
[visual] NFC - move Scene VN frame rendering
```

Move:

- `vn_dialogue_scroll_metrics`
- `vn_dialogue_panel_rect`
- `active_vn_overlay_layout`
- `vn_dialogue_text_width`
- `vn_dialogue_visible_rows`
- `vn_dialogue_scroll_metrics_for_line_count`
- `render_staged_scene`
- `vn_dialogue_nameplate`
- `render_vn_dialogue_lines`
- `recent_compose_transcript_lines`

Create:

```text
gameterm-visual/src/scene_runtime/vn_frame.rs
```

Verification:

```sh
cargo test -p gameterm-visual staged_scene
cargo test -p gameterm-visual vn_overlay
cargo test -p gameterm-visual dialogue_scroll
cargo check -p gameterm-visual
```

Definition of done:

- VN dialogue box text placement is unchanged
- scroll metrics remain bounded to the dialogue panel
- staged VN mode still hides fallback tile/grid text
- no new layout constants are introduced outside `vn_layout.rs`

### Lane 8: Interactive Debug Menu

Commit:

```text
[visual] NFC - move Scene debug menu runtime
```

Move:

- `debug_menu_row_count_for`
- `debug_menu_row_count`
- `selected_debug_marker`
- `debug_section_tabs`
- `vn_layout_debug_menu_lines`
- `static_debug_menu_lines`
- `text_debug_menu_lines`
- `voice_debug_menu_lines`
- `compose_debug_menu_lines`
- `runtime_debug_menu_lines`
- `render_interactive_debugger`
- `handle_vn_layout_debug_input`
- `sync_layout_selected_param`

Create:

```text
gameterm-visual/src/scene_runtime/debug_menu.rs
```

Verification:

```sh
cargo test -p gameterm-visual interactive_debugger
cargo test -p gameterm-visual vn_layout_debug
cargo test -p gameterm-visual debug_report
cargo check -p gameterm-visual
```

Definition of done:

- debug menu navigation remains unchanged
- Scene Layout parameter edit behavior remains unchanged
- Voice/Compose/Runtime menu rows remain visible and centralized

### Lane 9: Test Module Split

Commit series:

```text
[test] NFC - move Scene runtime tests
[test] NFC - move Scene rendering tests
[test] NFC - move Scene VN tests
[test] NFC - move Scene fixture tests
```

Move the large inline `#[cfg(test)] mod tests` out of `lib.rs` into:

```text
gameterm-visual/src/tests/mod.rs
gameterm-visual/src/tests/validation.rs
gameterm-visual/src/tests/actions.rs
gameterm-visual/src/tests/runtime.rs
gameterm-visual/src/tests/rendering.rs
gameterm-visual/src/tests/vn.rs
gameterm-visual/src/tests/fixtures.rs
```

Keep:

- `test_support.rs`
- broad behavior tests
- fixture coverage

Do not:

- delete tests
- weaken assertions
- combine unrelated test groups only to reduce files

Verification after each test move:

```sh
cargo test -p gameterm-visual <moved-test-filter>
cargo test -p gameterm-visual
```

Definition of done:

- `lib.rs` no longer owns a giant inline test module
- tests are easier to find by behavior
- full `gameterm-visual` test suite remains green

### Lane 10: Scope/Docs Closeout

Commit:

```text
[docs] record Scene runtime A refactor
```

Update:

- this scope with completion notes
- `docs/gameterm-scene-refactor-plan.md`
- `docs/gameterm-scene-handoff.md`
- `structure/cache/repo-cache.md` if the cache is current and tracked in the
  active repo state

Verification:

```sh
ci/check-commit-message.sh HEAD~N..HEAD
git status --short --branch
```

## Full Pass Verification

Before calling this A-pass complete:

```sh
cargo test -p gameterm-visual
cargo check -p gameterm-visual
cargo check -p gameterm-gui
ci/check-commit-message.sh origin/main..HEAD
git diff --check
```

Optional smoke only if a lane accidentally touches GUI overlay dispatch or live
render plumbing:

```sh
ci/gameterm-scene-smoke.sh --launch --scenario vertical-slice \
  --output /tmp/gameterm-scene-runtime-a-refactor.png
```

## A-Pass Definition Of Done

The pass is complete when:

- `lib.rs` is primarily a facade and no longer owns runtime presentation,
  debug-menu, VN frame, snapshot, or test bulk.
- `SceneRuntime` remains the central runtime authority, but execution is split
  into narrow child modules.
- Stable public exports remain compatible.
- No runtime fields are widened just to satisfy extraction.
- No behavior changes are hidden in NFC commits.
- Full verification passes.
- Deferred upstream-wide config/env/CLI debt is documented separately rather
  than mixed into this Scene Runtime pass.

## Expected Grade After Completion

For the fork-owned Scene Runtime surface:

- Centralize: A-
- Modularize: A
- Abstract / boring surface: A-
- Orthogonalize: A-
- Encode invariants: A
- Errors as system information: A
- Contain dangerous power: A
- Test at boundary: A
- Human-facing and machine-enforced: A

The remaining A-minus items are acceptable if they are tied to preserving public
API compatibility and upstream fork discipline rather than accidental coupling.
