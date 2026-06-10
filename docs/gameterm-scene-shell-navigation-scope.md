# GameTerm Scene Mode Shell Navigation Scope

Status: SCOPED.

This scope adds a cozy-game navigation shell around Scene Mode: a boot
"press start" screen, a main menu, and an in-scene Tab-cycle of mode screens.
This first pass builds the navigation primitives with placeholder rendering;
pixel-art assets and panel skinning come in a later pass.

## Operator Decisions (locked)

- Boot gates every open: the overlay opens to Boot, Enter advances to the Main
  Menu, and a menu choice enters the scene.
- Continue and New Session both just enter the scene in this pass (stub); the
  real resume/reset difference is wired later. Settings opens the existing
  debug menu (`VnLayoutDebugger`).
- The Tab-cycle modes (Character Select, Stage Select, Setting Mode) are
  navigable placeholder screens; real character/stage swaps are wired after
  assets land.

## Architecture Fit

Scene Mode already runs a view state machine, so this is additive:

- `VisualView { Scene, CommandSelection, TileDebugger, VnLayoutDebugger }` is a
  field on `SceneRuntime`; `handle_input` dispatches per view and the render
  entry branches per view.
- Input vocabulary already covers menus: `Activate` (Enter), `ToggleDebug`
  (Tab), `Next`/`Previous` (Down/Up), `Close` (Esc).
- The composer chrome already renders only when `view == Scene`, so menu views
  are clean by construction.
- The rounded-panel + text + selection-highlight render primitives already
  exist for a later skinning pass.

New views are added to `VisualView`: `Boot`, `MainMenu`, `CharacterSelect`,
`StageSelect`, `SettingMode`. Navigation state and rendering for these live in
a new `scene_runtime/shell.rs` module to keep them orthogonal to the scene
runtime internals.

## Non-Goals

- No pixel-art assets, nine-patch panels, or GPU panel skinning in this pass
  (screens render through the existing text-frame path).
- No real Continue/New Session semantics yet.
- No real character/stage swapping yet.
- No save/load of session state.
- No new general widget framework; each screen is a small bespoke handler.

## Screen Flow

```
overlay open
  -> Boot            Enter -> MainMenu        Esc -> close overlay
  -> MainMenu        Up/Down select; Enter:
                       Continue    -> Scene (stub)
                       New Session -> Scene (stub)
                       Settings    -> VnLayoutDebugger (existing)
                     Esc -> Boot
  -> Scene           Tab -> CharacterSelect (enter mode cycle)
  -> CharacterSelect Tab -> StageSelect      Esc -> Scene
  -> StageSelect     Tab -> SettingMode      Esc -> Scene
  -> SettingMode     Tab -> CharacterSelect  Esc -> Scene
```

Mode-cycle screens move a placeholder list cursor with Up/Down and show a tab
bar highlighting the active mode. Enter on a placeholder item reports a status
line for now.

## Lanes

### Lane 1: Boot Screen And View Machine

Type: behavior feature. The foundational slice; proves the view + input
plumbing end to end.

Targets: `gameterm-visual/src/scene_model.rs` (`VisualView`),
`gameterm-visual/src/scene_runtime/shell.rs` (new),
`gameterm-visual/src/scene_runtime/mod.rs` (dispatch + render branch),
`gameterm-gui/src/overlay/visual_loop.rs` (route keys to the runtime for shell
views instead of the compose dock),
overlay launch (enter Boot on open).

Behavior:

- Add `VisualView::Boot`; the overlay enters Boot on open.
- `is_shell()` helper on `VisualView` for the loop's input-routing bypass.
- Boot render: centered title and "Press Enter to Start" via the text frame.
- Boot input: `Activate` -> Scene (temporary until Lane 2 inserts the menu),
  `Close` -> exit overlay.
- The loop routes keys to `runtime.handle_input` for shell views, ahead of the
  compose dock.

Acceptance:

```sh
cargo test -p gameterm-visual shell --bin <n/a>
cargo test -p gameterm-visual shell
cargo test -p gameterm-gui visual --bin gameterm-gui
```

Tests: boot is the entry view; Activate leaves Boot; Close exits; the boot
frame contains the title and prompt.

### Lane 2: Main Menu

Type: behavior feature.

- Add `VisualView::MainMenu`; Boot `Activate` -> MainMenu.
- Menu items: Continue, New Session, Settings. Cursor moves with
  `Next`/`Previous`; `Activate` selects; `Close` -> Boot.
- Continue and New Session -> Scene (stub, identical for now).
- Settings -> `VnLayoutDebugger`.
- Render: title, item list with a selection marker on the cursor row.

Tests: cursor wraps/clamps; each item routes correctly; Continue and New both
enter Scene; Settings enters the debug view.

### Lane 3: Tab-Cycle Mode Screens

Type: behavior feature.

- Add `VisualView::{CharacterSelect, StageSelect, SettingMode}`.
- In Scene, `ToggleDebug` (Tab) enters CharacterSelect.
- In a mode screen, Tab cycles Character -> Stage -> Setting -> Character;
  `Close` returns to Scene.
- Each screen renders a tab bar with the active mode highlighted and a
  placeholder list navigated with `Next`/`Previous`.
- `Activate` on an item reports a status line (swap behavior wired later).

Note: this repurposes Scene Tab from "toggle layout debugger" to "enter mode
cycle." The layout debugger remains reachable via Main Menu -> Settings.

Tests: Scene Tab enters the cycle; Tab cycles all three; Esc returns to Scene;
list cursor moves.

### Lane 4: Verification

```sh
cargo test -p gameterm-visual
cargo test -p gameterm-gui visual --bin gameterm-gui
cargo check -p gameterm-gui
```

Live dogfood: open Scene Mode, see Boot; Enter -> menu; select an item ->
scene; Tab -> mode cycle; Tab cycles; Esc -> scene.

## Done Means

- Opening Scene Mode shows the Boot screen; Enter advances to the menu.
- The menu navigates and routes Continue/New/Settings correctly.
- Tab in the scene opens the mode cycle; Tab cycles the three modes; Esc
  returns to the scene.
- All screens render through the text-frame path with working selection.
- Tests cover every transition; the GUI build and existing suites stay green.

## Later (Out Of This Scope)

- Skin the screens with the rounded-panel renderer, then nine-patch pixel-art
  frames and decorative sprites.
- Real Continue/New Session semantics and session persistence.
- Real character/stage catalogs and on-stage swaps.

## First Implementation Slice

1. `[visual] add Scene boot screen and shell view machine` (Lane 1)
2. `[visual] add Scene main menu navigation` (Lane 2)
3. `[visual] add Scene Tab-cycle mode screens` (Lane 3)
4. `[docs] record Scene shell navigation verification` (Lane 4)
