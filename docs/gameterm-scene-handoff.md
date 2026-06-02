# GameTerm Scene Mode Handoff

This document is the current handoff for continuing Scene Mode work across
Codex sessions. Check this file first, then use the roadmap and scope docs for
deeper product context.

## Current Snapshot

- Date: 2026-06-02
- Branch: `main`
- Latest local commit: `16656cafe [visual] share VN overlay layout primitives`
- Remote state at handoff time: `main` is ahead of `origin/main` by 1 commit
- Local app bundle refreshed: `/Users/julianabeleda/Applications/GameTerm.app`

Current user goal:

Keep moving Scene Mode toward a dogfoodable visual-novel-style surface where
the user can see Codex dialogue, type through a composer dock, and trust that
the visual layout remains stable while resizing or switching between windowed
and fullscreen views.

## Latest Commit

`16656cafe [visual] share VN overlay layout primitives`

Files changed:

- `gameterm-visual/src/lib.rs`
- `gameterm-gui/src/overlay/visual.rs`
- `gameterm-gui/src/termwindow/render/visual_quad.rs`

What changed:

- Added shared VN layout primitives:
  - `VnOverlayRect`
  - `VnOverlayLayout`
  - `vn_overlay_layout(...)`
- Refactored staged VN text rendering so dialogue text and speaker nameplate
  rows come from the shared layout.
- Refactored the Scene compose dock so the `Composer` nameplate and input row
  use the same shared layout.
- Refactored GPU VN panel/nameplate quads so transparent rounded boxes use the
  same layout as terminal text.
- Removed the stale bottom-offset compose placement helper.

Expected outcome:

- Dialogue panel, composer dock, nameplates, and text remain aligned across
  resolution changes.
- Windowed mode can be more compact than fullscreen, but labels should remain
  attached, text should stay inside panels, and panel/text geometry should not
  drift.

## Verification Baseline

Commands already run successfully:

```sh
cargo test -p gameterm-visual vn_overlay_layout_derives_panels_and_nameplates
cargo test -p gameterm-visual staged_scene_renders_vn_dialogue_box_and_compose_dock
cargo test -p gameterm-gui scene_compose_dock_staged_nameplate_and_input_are_separate
cargo test -p gameterm-gui vn_panel_rects_use_shared_fullscreen_proportions
cargo test -p gameterm-gui vn_panel_nameplate_rects_attach_to_dialogue_and_dock
cargo build -p gameterm-gui
ci/install-macos-dev-app.sh
```

Smoke capture:

```text
/Users/julianabeleda/Desktop/gameterm-scene-smoke-vn-shared-layout-20260602-182247.png
```

Smoke status: PASS. The capture confirmed that the `Codex` and `Composer`
nameplates/text are inside their respective transparent boxes.

Caveat: the capture was a 1920x1080 desktop screenshot, but macOS left
GameTerm in a smaller foreground window rather than true fullscreen. Redo a
true fullscreen or maximized visual smoke if the next session needs stronger
visual confidence.

Known warning noise remains outside this Scene Mode lane:

- existing macOS `objc` macro `unexpected cfg` warnings
- existing `gameterm-toast-notification` unnecessary `unsafe` warnings
- existing `screen_line.rs` unused assignment warning

## Dirty Worktree Caveat

At handoff time, two unstaged formatting-only Rust diffs were present:

- `gameterm-visual/src/vn_script_import.rs`
- `gameterm-visual/src/workspace_scene.rs`

Do not include them in future behavior commits unless the next task verifies
they are relevant. Keep commits separated by concern.

## VN Overlay Audit Checklist

Use this checklist when reviewing or continuing the latest VN overlay work.

### Shared Dynamic Layout

- Confirm the VN overlay layout is derived from current terminal columns and
  rows.
- Confirm terminal text placement and GPU panel rectangles both use
  `vn_overlay_layout`.
- Confirm resizing recomputes dialogue panel, composer dock, nameplates, and
  text rows from the shared layout.
- Look for remaining hardcoded margin, top, bottom, or nameplate calculations
  that could cause text and boxes to drift apart.

### Dialogue Panel

- Dialogue panel should sit in the upper/middle stage area.
- It should not sit too high, too low, or overlap the composer dock.
- Dialogue text should start inside the panel with sane padding.
- Dialogue text should not escape outside the panel.

### Composer Dock

- Composer dock should sit near the bottom of Scene Mode.
- It should span most of the window width.
- Composer input text should start inside the dock with sane padding.
- Composer text should not escape outside the dock.

### Nameplates

- Speaker/Codex nameplate should attach to the top-left edge of the dialogue
  panel.
- Composer nameplate should attach to the top-left edge of the composer dock.
- Nameplates should not cover the first line of dialogue text or composer input
  text.
- Nameplates should not float too far above panels.
- Nameplates should not be hidden behind panels, clipped by the window edge, or
  visually detached.
- A small edge overlap is acceptable if it makes the tab look attached, but it
  should not intrude into the content area.

### Windowed And Fullscreen Behavior

- Fullscreen or tall viewports should use the larger VN-style layout.
- Smaller windowed viewports should use a compact layout that still keeps
  boxes, nameplates, and text aligned.
- Windowed mode can be tighter than fullscreen, but it should not show detached
  labels, text outside boxes, or panel/text mismatch.

## Recommended Next Actions

1. If reviewing the latest visual work, inspect commit `16656cafe` and the
   three changed files listed above.
2. Add any missing compact/windowed resize tests before changing layout again.
3. Redo visual smoke in a true fullscreen or maximized window if possible.
4. Keep any follow-up fixes in separate commits.
5. Push only when the user asks.

Commit discipline:

- Keep separate commits by concern.
- Do not mix formatting-only changes with behavior changes.
- Before committing, run `git diff --check` and targeted tests.
- Treat pre-existing warning noise as separate from Scene Mode failures.
