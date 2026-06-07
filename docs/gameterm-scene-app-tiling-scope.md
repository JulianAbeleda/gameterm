# GameTerm Scene Mode App Tiling Scope

Status: scoped, not implemented.

This document scopes a first pass for letting Scene Mode/Codex open another
desktop app and arrange it beside GameTerm.

Example user request:

```text
open Safari
open Firefox
open Spotify
```

Expected result:

```text
+----------------------+----------------------+
| GameTerm / Codex     | Safari / Firefox     |
| Scene Mode           | Spotify / other app  |
+----------------------+----------------------+
```

## Goal

GameTerm should support explicit, approved desktop-layout actions that can:

1. tile the current GameTerm window to a known region
2. launch or activate an allowlisted external app
3. tile that external app to a complementary region
4. report success/failure inside Scene Mode

The feature should make GameTerm feel like a workspace controller without
turning Codex into unrestricted desktop automation.

## Product End State

The first pass is complete when:

1. GameTerm can tile its own macOS window left/right/center/fullscreen-like
   within the visible screen frame.
2. GameTerm can open or activate an allowlisted app by bundle id or app name.
3. GameTerm can find the launched app's front window and move/resize it to the
   requested tile.
4. Scene Mode exposes a typed action such as `OpenAppTile`.
5. Codex can propose that action using a documented structured response shape.
6. GameTerm validates app/layout/policy before executing the action.
7. A user-facing approval path exists before external app tiling runs.
8. Failures are visible in Scene Mode and do not close the overlay.
9. Tests cover action validation, policy derivation, and command/capability
   prompt construction without requiring real macOS app automation.
10. Manual macOS smoke verifies Safari or another allowlisted app tiles beside
    GameTerm.

## Non-Goals

- No unrestricted "Codex can control my desktop" mode.
- No arbitrary app name execution without an allowlist.
- No shell-based automation as the product path.
- No cross-platform implementation in the first pass.
- No deep browser control, webpage scraping, or Spotify playback automation.
- No automatic execution of agent-proposed actions without user approval.
- No persistence of secrets, credentials, or account state.
- No attempt to override macOS Spaces/fullscreen behavior.

## Current Baseline

GameTerm already has:

- Scene Mode action kinds with policy metadata.
- `RunCommand` actions with explicit argv and target panes.
- `OpenFile` actions that ask the platform to open local files.
- Codex compose output that can return structured JSON and patch dialogue.
- policy metadata fields: origin, risk, scope, confirmation summary.
- a visual-novel-style compose/dialogue loop where user approval can be
  represented as a choice.
- macOS app-bundle install path through `ci/install-macos-dev-app.sh`.

Missing:

- no typed external app tiling action
- no native macOS window move/resize bridge for other apps
- no app allowlist for desktop automation
- no Scene/Codex capability manifest for desktop-layout actions
- no Accessibility permission diagnostic for tiling other apps

## Key Concept: Capabilities Are Runtime Context

Codex does not know GameTerm's private features by default.

GameTerm must tell Codex what it can do at runtime. That is not pretraining.
It is a capability manifest injected into the compose prompt and enforced by
GameTerm after Codex replies.

Recommended first-pass shape:

```text
User prompt
+ current Scene state
+ allowed GameTerm capabilities
+ required structured action schema
-> Codex
-> dialogue text + proposed actions
-> GameTerm validates actions
-> user approves
-> GameTerm executes approved actions
```

This means the user should not need to train Codex or remember magic commands.
They may still need to configure local policy:

- which apps are allowlisted
- which layouts are allowed
- whether actions require approval
- macOS Accessibility permission for moving other apps

## Capability Manifest

Add a small runtime-generated capability catalog for Scene compose.

Candidate manifest:

```json
{
  "version": 1,
  "capabilities": [
    {
      "name": "open_app_tile",
      "description": "Open or activate an allowlisted macOS app and tile it beside GameTerm.",
      "risk": "external_app",
      "requires_confirmation": true,
      "arguments": {
        "app": ["Safari", "Firefox", "Spotify"],
        "layout": ["right", "left", "right_50", "right_60"],
        "gameterm_layout": ["left", "left_50", "left_40"]
      }
    }
  ]
}
```

The manifest should be derived from Rust config/action registry state, not hand
copied into every prompt.

## Codex Prompt Contract

When app tiling is enabled, the compose bridge should include concise
instructions:

```text
You can propose GameTerm actions only through JSON.
Do not claim an action ran unless GameTerm reports it ran.
If the user asks to open/tile an app, return a proposed action.
Allowed action:
{
  "type": "open_app_tile",
  "app": "<allowlisted app>",
  "layout": "right",
  "gameterm_layout": "left"
}
```

Codex response shape:

```json
{
  "speaker": "Codex",
  "text": "I can open Safari beside GameTerm.",
  "actions": [
    {
      "type": "open_app_tile",
      "app": "Safari",
      "layout": "right",
      "gameterm_layout": "left"
    }
  ]
}
```

GameTerm must treat this as a proposal, not proof of execution.

## Approval Flow

External app tiling should use the same product principle as visual-novel
choices: the user chooses before an external action runs.

Recommended flow:

1. User types: `open Safari`.
2. Codex replies: `I can open Safari beside GameTerm.`
3. Scene Mode shows an approval choice:

```text
> Open Safari on the right
  Cancel
```

4. User presses Enter on the approved choice.
5. GameTerm tiles itself and Safari.
6. Scene Mode status reports:

```text
Opened Safari and tiled it right
```

Failure should be equally explicit:

```text
Safari opened, but GameTerm cannot move its window. Enable Accessibility permission.
```

## Action Schema

Add a new Scene action kind or external-action model.

Candidate JSON:

```json
{
  "label": "Open Safari on the right",
  "kind": {
    "OpenAppTile": {
      "app": "Safari",
      "bundle_id": "com.apple.Safari",
      "layout": "right",
      "gameterm_layout": "left",
      "activation": "open_or_focus"
    }
  },
  "policy": {
    "origin": "agent",
    "risk": "external_app",
    "scope": "external",
    "requires_confirmation": true,
    "summary": "Opens Safari and tiles it beside GameTerm"
  }
}
```

Fields:

- `app`: display name used by Codex/user-facing UI
- `bundle_id`: stable app identity when known
- `layout`: target layout for the external app
- `gameterm_layout`: target layout for the current GameTerm window
- `activation`: `open_or_focus` for first pass

Validation:

- app or bundle id must be allowlisted
- layout values must be known
- policy risk must be `external_app`
- confirmation must default to true
- unsupported platforms must fail visibly

## App Allowlist

First-pass config:

```json
{
  "desktop_actions": {
    "enabled": true,
    "require_confirmation": true,
    "apps": [
      {
        "name": "Safari",
        "bundle_id": "com.apple.Safari"
      },
      {
        "name": "Firefox",
        "bundle_id": "org.mozilla.firefox"
      },
      {
        "name": "Spotify",
        "bundle_id": "com.spotify.client"
      }
    ],
    "layouts": ["left", "right", "left_50", "right_50", "left_40", "right_60"]
  }
}
```

Default policy:

- feature disabled or empty allowlist by default if risk posture is strict
- or feature enabled with a tiny built-in macOS allowlist but still requires
  confirmation

Recommended for dogfood: enabled with Safari only first, then add Firefox and
Spotify after the API is stable.

## macOS Windowing Primitives

First pass should be native Rust/macOS integration, not AppleScript as the
product implementation.

Useful primitives:

- launch/activate app:
  - `NSWorkspace`
  - bundle id preferred over display name
- find target app/process:
  - `NSRunningApplication`
  - process id from launch/activation result
- inspect/move app windows:
  - Accessibility APIs through `AXUIElement`
  - `kAXWindowsAttribute`
  - `kAXPositionAttribute`
  - `kAXSizeAttribute`
- detect permission:
  - Accessibility trusted-process check
  - show clear Scene diagnostic if missing
- compute target rectangles:
  - current screen visible frame, excluding menu bar/dock
  - current GameTerm window screen
  - multi-monitor fallback to active screen

Prototype scripts may use `osascript` only for exploration. The shipped path
should use native APIs so errors, permissions, and tests are easier to control.

## Layout Model

Support a small deterministic layout enum first:

- `left`
- `right`
- `left_50`
- `right_50`
- `left_40`
- `right_60`
- `center`
- `maximize_visible`

Recommended default for `open_app_tile`:

```text
GameTerm: left_50
External app: right_50
```

Layout computation should:

- use visible screen bounds
- avoid negative sizes
- clamp to minimum width/height
- preserve menu bar/dock safe area
- not depend on current window size

## Implementation Lanes

### Lane 1: Scope And Capability Model

Purpose: define the action shape before touching platform code.

Tasks:

- add this scope doc
- add roadmap link
- define capability/action names
- decide config file owner
- decide first allowlisted apps

Acceptance:

- scope is committed
- capability answer is documented
- no runtime behavior changes

### Lane 2: Action Schema And Validation

Purpose: make `OpenAppTile` a typed action GameTerm can reason about.

Tasks:

- add `OpenAppTile` action kind or equivalent external-action type
- parse/validate known layouts
- validate allowlisted app names/bundle ids
- derive policy defaults
- doctor warns on invalid/missing policy

Acceptance:

- old scenes still load
- invalid app/layout is rejected
- tests cover schema compatibility and validation

### Lane 3: Capability Manifest For Codex

Purpose: let Codex know what GameTerm can do without pretraining.

Tasks:

- build a runtime capability manifest from config/action registry
- inject concise capability instructions into Codex compose prompt
- require structured action output for proposed desktop actions
- keep visible reply separate from proposed action data
- include capabilities in debug diagnostics

Acceptance:

- fake-Codex tests can propose `open_app_tile`
- prompt construction tests include allowlisted apps/layouts
- Codex cannot invent unallowlisted app actions without validation failure

### Lane 4: Approval Choice Flow

Purpose: make external tiling feel like a visual-novel choice, not a hidden
side effect.

Tasks:

- convert proposed desktop actions into pending Scene choices
- render action summary and risk
- Enter confirms selected action
- Cancel leaves state unchanged
- status reports pending/approved/cancelled

Acceptance:

- user can approve or cancel app tiling
- no external app opens before approval
- tests cover pending action lifecycle

### Lane 5: Self Tiling

Purpose: move GameTerm's own window reliably.

Tasks:

- add platform bridge for current GameTerm window rect
- compute target rect from layout enum
- move/resize current window
- report unsupported platform cleanly

Acceptance:

- macOS manual smoke can tile only GameTerm left/right
- unit tests cover layout math
- no external app permission required for self tiling if possible

### Lane 6: External App Launch/Tile

Purpose: open and tile Safari/Firefox/Spotify.

Tasks:

- launch/activate app by bundle id/name
- wait for app process/window
- move/resize front window with Accessibility APIs
- classify failures:
  - unsupported platform
  - app not allowlisted
  - app not installed
  - app launched but no window found
  - Accessibility permission missing
  - window refused move/resize

Acceptance:

- Safari can tile right beside GameTerm in manual smoke
- missing permission shows readable diagnostic
- failures do not close Scene Mode

### Lane 7: Smoke And Docs

Purpose: prove the actual desktop behavior.

Tasks:

- add docs for enabling desktop actions
- add Accessibility permission instructions
- add manual smoke checklist
- record screenshot/result in smoke report

Acceptance:

- documented user path from app launch to approved app tiling
- smoke report records at least one successful app tile
- smoke report records permission-missing behavior if applicable

## Testing Strategy

Unit tests:

- layout rect math for common screen sizes
- allowlist validation
- app/layout schema parsing
- policy derivation
- capability manifest generation
- Codex prompt/action proposal parsing
- pending approval lifecycle

Integration tests:

- fake platform window manager records requested window moves
- fake app launcher records bundle id/name
- Scene action applies status updates without real OS calls

Manual macOS smoke:

1. rebuild/install dev app
2. launch GameTerm
3. choose Scene Mode
4. type `open Safari`
5. confirm approval choice
6. verify GameTerm left, Safari right
7. repeat missing-permission path if Accessibility is disabled

## Security And Policy

Desktop app tiling is external automation. It must stay explicit.

Rules:

- Codex proposes actions; GameTerm executes only validated actions.
- Unknown action types are ignored or reported, never executed.
- App names/bundle ids must be allowlisted.
- User confirmation is required by default.
- Shell commands are not used to move windows.
- Scene Mode should display origin/risk/scope before approval.
- Permission failures should tell the user what permission is missing.

## Open Questions

1. Should Safari be the only default allowlisted app for first dogfood?
2. Should app tiling live under Scene Mode config, global GameTerm config, or
   both?
3. Should the approval choice persist in dialogue history after completion?
4. Should layouts be user configurable or fixed until the first pass is stable?
5. Should GameTerm restore its original window rect after the external app is
   closed?
6. Should Codex be allowed to propose multiple tiled apps in one turn, or only
   one action per approval?

## Recommended First Pass

Implement in this order:

1. `OpenAppTile` schema and validation
2. capability manifest injected into compose prompt
3. pending approval choice flow
4. self-tiling only
5. Safari external-app tiling
6. manual smoke and docs

This order proves the user experience before expanding the platform surface.
