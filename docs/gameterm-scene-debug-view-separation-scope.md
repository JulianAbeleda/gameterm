# GameTerm Scene Debug View Separation Scope

Date: 2026-06-06

This document scopes the first pass to separate Scene Mode presentation from
debugging surfaces.

The current issue is not that Scene Mode lacks debugging. The issue is that the
debug surfaces bleed into the VN surface: status text is hard to read over the
background, informational debug text shares space with scene art, and voice
debug controls are reached through an implicit key path instead of a central
interactive debug menu.

## Product Contract

Scene Mode should have three clear view states.

### Scene View

Purpose: the normal dogfoodable VN/workspace experience.

Expected behavior:

- show the background, character, dialogue panel, dialogue transcript, composer
  dock, nameplates, and voice indicator
- hide top-left debug/help/status text that is only useful for implementation
  debugging
- keep user-facing runtime messages inside the intended VN surfaces
- keep normal compose, Codex, TTS, STT, dialogue scrollback, and close/reload
  behavior

Scene View should not render:

- tile/debug information
- layout parameter menus
- voice diagnostic menus
- command/debug hints over the background
- duplicated dialogue/composer text outside the VN panels

### Debug 1: Info Debug View

Purpose: read-only diagnostics.

Expected behavior:

- render as an information-only debug page
- use a readable dark overlay instead of scene art
- include scrollback for diagnostic content
- show runtime state, scene metadata, input state, compose/TTS/STT state,
  loaded assets, selected entity, layout dimensions, and recent errors
- be bounded to the debug view, not terminal scrollback
- never show the Scene View dialogue transcript or composer input as part of
  the VN layout

Info Debug View should not render:

- background art
- character sprites
- VN panels/nameplates
- composer dock text
- dialogue transcript text
- interactive layout sliders or voice toggle controls

### Debug 2: Interactive Debug View

Purpose: selectable controls and debug tools.

Expected behavior:

- render a frontmost debug shell
- use a black overlay behind the menu
- render debug menu text in purple
- clear inactive panes when moving left/right between submenus
- make the active pane obvious and isolated
- separate tile debugging from scene/VN debugging instead of treating layout
  tooling as one mixed pane
- host voice controls, compose controls, runtime controls, voice diagnostics,
  voice test mode, and fake Codex toggle inside the Scene Mode Debug Menu
- retain explicit routes to tile inspection and Scene/VN layout tuning, but
  only inside their active panes

Interactive Debug View should not render:

- normal Scene View dialogue/composer content behind the active menu
- voice controls in a hidden or separate shortcut-only path
- multiple debug panes at once unless a specific preview pane is selected

## Current Implementation Notes

Current runtime names:

```text
VisualView::Scene
VisualView::CommandSelection
VisualView::TileDebugger
VisualView::VnLayoutDebugger
```

Current tab cycle:

```text
Scene -> TileDebugger -> VnLayoutDebugger -> Scene
```

The product contract maps to the current enum as:

```text
Scene View              -> VisualView::Scene
Debug 1 Info Debug      -> VisualView::TileDebugger, first pass
Debug 2 Interactive     -> VisualView::VnLayoutDebugger, first pass
```

Renaming `TileDebugger` and `VnLayoutDebugger` to product names is a separate
cleanup decision. The first pass may keep the enum names to avoid unnecessary
churn while changing the behavior and presentation.

Known current coupling:

- `render_scene` still writes Scene Mode help/status text into the normal frame.
- staged VN rendering has debug/layout text paths that can share rows with the
  scene.
- `visual_quad.rs` suppresses all stage art for `TileDebugger`, but
  `VnLayoutDebugger` still draws live panels/nameplates for layout tuning.
- `visual_render.rs` applies voice debug frame lines over the top of the
  terminal frame.
- `visual_voice_debug.rs` owns voice menu state, but access is currently tied
  to `TileDebugger` plus a plain `v` key.

## Non-Goals

- Do not rewrite Scene Mode runtime architecture.
- Do not remove mux.
- Do not remove Command Selection.
- Do not change Codex compose behavior.
- Do not change TTS/STT backend contracts.
- Do not redesign VN panel geometry, opacity, or character placement in this
  pass.
- Do not make broad enum/module renames unless the implementation becomes
  clearer and the tests remain tight.
- Do not hide useful technical output from the visible dialogue transcript in
  normal Scene View.

## Lane 1: View Contract And Naming Boundary

Commit prefix: `[visual]`

Goal: make the view contract explicit in code and tests without forcing a broad
rename.

Work:

- document the logical names for the three views near `VisualView`
- keep or update the tab cycle so:

```text
Scene View -> Info Debug View -> Interactive Debug View -> Scene View
```

- add helper predicates if useful:

```text
is_scene_view()
is_info_debug_view()
is_interactive_debug_view()
```

- use helpers in render/input paths where they reduce repeated enum matching

Tests:

- tab cycle test asserts the product order
- command selection still exits back to Scene View correctly
- debug-only input does not affect normal Scene View

Definition of done:

- the runtime has one obvious source of truth for which view is normal,
  informational, and interactive
- no behavior change outside debug navigation

## Lane 2: Clean Scene View Chrome

Commit prefix: `[visual]`

Goal: remove debug/help/status text from normal Scene View.

Work:

- remove or suppress top-left implementation text in `VisualView::Scene`
- move persistent help/status text into Info Debug View
- keep normal user-facing dialogue and compose transcript inside VN surfaces
- keep close/reload behavior unchanged even if the help text is hidden
- make any necessary status message ephemeral or bounded to an existing VN
  surface

Tests:

- staged Scene View frame does not contain implementation help text
- staged Scene View still contains dialogue transcript lines after compose
- default non-staged Scene View still renders meaningful content
- close/reload/input tests remain green

Definition of done:

- normal Scene View is clean enough to dogfood without unreadable debug text
  on top of the background

## Lane 3: Info Debug View Renderer And Scrollback

Commit prefix: `[visual]`

Goal: make Debug 1 a pure, readable information page with bounded scrollback.

Work:

- render Info Debug View from a dedicated diagnostic line builder
- draw or expose a dark debug background layer
- remove VN art, panels, dialogue transcript, and composer dock from this view
- include useful diagnostics:
  - view and mode
  - scene source and title
  - viewport cols/rows
  - overlay layout dimensions
  - compose backend and status
  - TTS/STT enabled state
  - voice debug summary
  - selected entity/choice
  - asset manifest status
  - recent runtime/status messages
- add bounded diagnostic scrollback state
- support PageUp/PageDown and mouse wheel within the debug view
- clamp scrollback when content or viewport size changes

Tests:

- Info Debug View contains diagnostics and excludes Scene dialogue/composer
  text
- scroll up reveals earlier diagnostic lines
- scroll down returns toward latest diagnostic lines
- scroll offset clamps at top and bottom
- window resize reclamps the offset

Definition of done:

- Debug 1 is readable, independent from scene art, and useful as a diagnostic
  page

## Lane 4: Interactive Debug Shell

Commit prefix: `[visual]` or `[gui]`, depending on where the renderer changes
land

Goal: make Debug 2 the central interactive debug menu.

Work:

- render a black overlay behind the interactive menu
- render menu text in purple
- ensure the menu draws in front of any retained preview content
- isolate panes so left/right navigation clears inactive pane content
- define first-pass panes:

```text
Tile Debug Menu
Scene Mode Debug Menu
```

- `Tile Debug Menu` owns grid/tile/entity inspection
- `Scene Mode Debug Menu` owns VN overlay layout, dialogue/composer box
  tuning, nameplate tuning, voice, compose, runtime, fake Codex, and
  scene-specific preview controls
- inside `Scene Mode Debug Menu`, use sub-sections rather than peer top-level
  panes:

```text
Scene Layout
Voice
Compose
Runtime
```

- keep tile and scene/VN layout tuning available, but do not always show live
  VN panels behind unrelated panes
- preserve existing VN layout override editing controls inside the Scene Layout
  sub-section
- move any current tile debugger controls into the Tile Debug Menu

Tests:

- Interactive Debug View frame contains the active pane title
- inactive pane content is absent
- Tile Debug Menu content is absent when Scene Mode Debug Menu is active
- Scene Mode Debug Menu content is absent when Tile Debug Menu is active
- Scene Layout, Voice, Compose, and Runtime sub-sections do not render on top
  of each other
- purple/debug style metadata is applied where the renderer supports it
- black overlay metadata is present in the GPU/debug snapshot where applicable
- existing layout override tests remain green

Definition of done:

- Debug 2 reads like a menu, not like the normal Scene View with debug text
  placed on top

## Lane 5: Centralize Voice Debug Under Debug 2

Commit prefix: `[gui]`

Goal: put voice debugging in the interactive debug menu instead of a separate
shortcut path.

Work:

- move voice menu access under `Scene Mode Debug Menu -> Voice`
- remove or retire the plain `v` debug-entry shortcut if it conflicts with the
  centralized model
- keep fake Codex toggle in the Voice sub-section
- fake Codex toggle should still:
  - clear the dialogue transcript
  - update the Codex nameplate to the fake label
  - prevent toggling while compose is running
  - optionally continue to hit TTS so voice can be tested end to end
- keep voice diagnostics and voice test mode inside the Voice sub-section
- make left/right navigation switch between Tile Debug Menu and Scene Mode
  Debug Menu, then switch sub-sections inside Scene Mode Debug Menu without
  leaving stale content on screen

Tests:

- Voice sub-section is reachable from Scene Mode Debug Menu
- Voice sub-section is not rendered in Scene View, Info Debug View, or Tile
  Debug Menu
- fake Codex toggle clears dialogue history
- fake Codex toggle is blocked while compose is running
- voice test mode still records diagnostic output

Definition of done:

- all voice debug actions live in one predictable debug location

## Lane 6: GPU And Frame Layer Separation

Commit prefix: `[gui]`

Goal: make the terminal text layer and GPU panel/art layer agree about which
view is active.

Work:

- suppress stage art for Info Debug View
- suppress stage art and normal VN panels for Interactive Debug panes that do
  not explicitly request layout preview
- keep tile preview and Scene/VN layout preview as explicit menu/sub-section
  behavior
- ensure black debug overlay draws before menu text and after scene art, or
  simply replaces scene art in debug views
- ensure debug menu text is not hidden by stage art or panels

Tests:

- `visual_quad` tests assert stage art suppression for Info Debug View
- `visual_quad` tests assert Interactive Debug View only draws preview panels
  in the Scene Mode Debug Menu layout sub-section
- `visual_quad` tests assert tile/grid debug drawing only appears in the Tile
  Debug Menu
- debug overlay snapshot rows/cols match the terminal text frame dimensions

Definition of done:

- there is no split-brain state where text says one debug view but GPU art
  renders another

## Lane 7: Smoke And Documentation

Commit prefix: `[test]` for smoke harness changes, `[docs]` for docs updates

Goal: prove the user-visible behavior after implementation.

Work:

- add or update smoke scenarios for:
  - normal Scene View
  - Info Debug View
  - Interactive Debug Tile Debug Menu
  - Interactive Debug Scene Mode Debug Menu
  - Interactive Debug Scene Mode Debug Menu Voice sub-section
- capture fullscreen and one windowed size
- update the smoke report with expected screenshots/observations
- update the handoff doc with the new debug navigation model

Definition of done:

- screenshots show Scene View clean, Debug 1 readable, and Debug 2 frontmost
- smoke report records whether voice/fake-Codex diagnostics were exercised

## Test Plan

Minimum automated checks:

```sh
cargo test -p gameterm-visual runtime_toggles_debugger
cargo test -p gameterm-visual debugger
cargo test -p gameterm-visual vn_layout_debug
cargo test -p gameterm-gui scene_voice_debug --bin gameterm-gui
cargo test -p gameterm-gui visual_quad --bin gameterm-gui
cargo test -p gameterm-gui overlay::visual --bin gameterm-gui
cargo check -p gameterm-gui
git diff --check
```

Manual smoke checks:

- launch GameTerm app and enter Scene Mode
- confirm Scene View has no top-left debug/help text over the background
- press Tab once and confirm Info Debug View is readable on a dark background
- scroll Info Debug View with PageUp/PageDown and mouse wheel
- press Tab again and confirm Interactive Debug View is a black/purple menu
- move left/right between Tile Debug Menu and Scene Mode Debug Menu and confirm
  inactive menus are cleared
- confirm Tile Debug Menu shows only tile/grid/entity debug controls
- confirm Scene Mode Debug Menu shows Scene Layout, Voice, Compose, and Runtime
  sub-sections without rendering them all at once
- toggle fake Codex from the Scene Mode Debug Menu Voice sub-section and
  confirm dialogue clears and fake compose can be exercised
- return to Scene View and confirm normal compose, dialogue scrollback, TTS,
  STT indicator, and close/reload still work

## Commit Plan

Recommended sequence:

```text
[docs] scope Scene debug view separation
[visual] define Scene debug view contract
[visual] clean Scene Mode chrome
[visual] add Scene info debug scrollback
[visual] add Scene interactive debug shell
[gui] centralize Scene voice debug menu
[gui] align Scene debug GPU layers
[test] add Scene debug view smoke coverage
[docs] record Scene debug view pass
```

Keep each commit self-contained. Do not mix NFC extraction with behavior fixes
inside this pass unless the extraction is required to make the behavior change
small and readable.

## Open Decisions

- Keep `TileDebugger` and `VnLayoutDebugger` as internal names for the first
  pass, or rename them to `InfoDebugger` and `InteractiveDebugger` later?
- Should Scene View hide all runtime status text, or keep a short ephemeral
  status line inside an existing VN panel?
- Should the Scene Layout sub-section keep live panel preview by default, or
  require an explicit preview toggle?
- Should the Tile Debug Menu keep a live grid preview by default, or use a
  text-only debug representation first?
- Should Info Debug View scrollback use only PageUp/PageDown and mouse wheel,
  or also support vim-style `j/k` once the page is focused?

## Stop Conditions

Stop and rescope if:

- a required enum rename forces broad unrelated churn
- debug navigation requires a new global input model
- GPU overlay ordering cannot be made deterministic with existing snapshot data
- smoke shows the black/purple menu is readable in tests but unreadable in the
  actual app
- a discovered voice/Codex bug is unrelated to debug menu separation

## First-Pass Success

This pass is complete when:

- Scene View looks like the product, not like an implementation console
- Debug 1 is a readable diagnostic page with bounded scrollback
- Debug 2 is a central interactive debug menu with isolated panes
- voice diagnostics and fake Codex live under Debug 2
- the text layer and GPU layer agree on what each view should render
- automated tests and at least one app smoke pass support the behavior
