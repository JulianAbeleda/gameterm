# GameTerm Scene Mode Roadmap

This is the consolidated roadmap for Scene Mode. It points to the detailed
scope documents, but this file is the first place to check for current product
status and next priorities.

## Current Status

Status: first shippable Scene Mode pass complete; command policy second pass
complete; VN asset intake, VN script import, VN demo install, real asset run,
strict image validation, fullscreen smoke capture, local PSD/image export, and
native active-pane GUI first-pass closure complete. Staged VN text presentation
and compose dock input are implemented; the Codex compose bridge local-backend
first pass is implemented. The local Codex CLI session bridge first pass is
implemented with a deterministic fake-Codex smoke path. VN dialogue/composer
panel text, nameplates, and GPU quads now share one dynamic overlay layout
primitive.

Scene Mode currently has:

- a stable scene runtime with bundled fallback and user scene loading
- schema validation, fixtures, authoring helpers, doctor checks, and verifier
- normal rendering, sprite fallback, Tile Debugger, and live smoke discipline
- dialogue, variables, guarded choices, layers, lifecycle hooks, input maps,
  deterministic state actions, lightweight RPG state, and story-state
  persistence
- patch inbox and mux patch transports
- process and agent lifecycle patch helpers
- Agent/Workspace authored product slice
- Workspace Discovery from cwd, git, important files, generated scenes, and
  generated patches
- stabilization/refactor pass covering commit discipline, verifier structure,
  helper overwrite checks, enter-lifecycle cleanup, and live smoke audit
- command/action policy metadata, diagnostics, workspace-generated policy
  metadata, command filtering data model, and command-selection view
- native active-pane Scene action that opens a transient generated workspace
  scene without overwriting the configured scene
- Rust VN script import, local VN asset intake, generated VN demo
  install/doctor workflow, strict PNG validation, fullscreen `vn-demo` smoke,
  and a local PSD/image export helper for downloaded assets
- staged VN background/character rendering, wide VN text presentation, and a
  typeable compose dock surface
- structured Scene VN compose submit events, dialogue patch replies, sanitized
  deterministic/local process backend output, and fullscreen `vn-compose` smoke
- explicit local Codex CLI backend selection, `codex exec` command construction,
  final-message output handling, persistent app-launch Scene compose config,
  Codex failure diagnostics, and fullscreen `vn-compose-codex` fake-Codex smoke

Latest verification baseline:

- `ci/gameterm-scene-verify.sh --all`: PASS
- `cargo test -p gameterm-visual`: PASS
- `cargo build -p gameterm-gui`: PASS with pre-existing macOS warning noise
- Live smoke: PASS for `workspace-discovery`, recorded in
  [GameTerm Scene Mode Smoke Report](gameterm-scene-smoke-report.md)
- Live smoke: PASS for `active-pane-gui`, recorded in
  [GameTerm Scene Mode Smoke Report](gameterm-scene-smoke-report.md)
- Live smoke: PASS for `live-mux-discovery`, recorded in
  [GameTerm Scene Mode Smoke Report](gameterm-scene-smoke-report.md)
- Live smoke: PASS for `vn-demo` with repo fixture PNGs and with local
  downloaded PSD/school assets, recorded in
  [GameTerm Scene Mode Smoke Report](gameterm-scene-smoke-report.md)
- Live smoke: PASS for `vn-compose` shared VN overlay layout, recorded in
  [GameTerm Scene Mode Smoke Report](gameterm-scene-smoke-report.md)
- Live smoke: PASS for `vn-compose-codex` with deterministic fake-Codex
  backend, recorded in
  [GameTerm Scene Mode Smoke Report](gameterm-scene-smoke-report.md)
- Live smoke: PASS for `vn-compose` with the real local Codex CLI backend,
  recorded in
  [GameTerm Scene Mode Smoke Report](gameterm-scene-smoke-report.md)

Known non-blocking warning noise remains outside Scene Mode scope:

- existing macOS `objc` macro `unexpected cfg` warnings
- existing `gameterm-toast-notification` unnecessary `unsafe` warnings
- existing `screen_line.rs` unused assignment warning

## Document Map

Use this file for roadmap status. Use the detailed docs below for scoped
implementation details.

| Area | Owner Doc | Status |
| --- | --- | --- |
| First shippable Scene Mode pass | [First-Pass Scope](gameterm-scene-first-pass-scope.md) | Complete |
| First-pass closure items | [First-Pass Completion Scope](gameterm-scene-first-pass-completion-scope.md) | Complete |
| Runtime history and lower-level feature roadmap | [Runtime Roadmap](gameterm-scene-runtime-roadmap.md) | Mostly historical; keep for design context |
| Broad product completion stack | [Product Completion Scope](gameterm-scene-product-completion-scope.md) | First-pass complete; refactor backlog active |
| Command policy second pass | [Command Policy Second-Pass Scope](gameterm-scene-command-policy-second-pass-scope.md) | Complete |
| Live mux discovery | [Live Mux Discovery Scope](gameterm-scene-live-mux-discovery-scope.md) | Complete |
| Active-pane GUI entrypoint | [Active Pane GUI Entrypoint Scope](gameterm-scene-active-pane-gui-entrypoint-scope.md) | Complete |
| Ren'Py demo import | [Ren'Py Demo Scope](gameterm-scene-renpy-demo-scope.md) | Prototype implemented |
| VN script importer | [VN Script Import Scope](gameterm-scene-vn-script-import-scope.md) | First-pass implemented |
| VN asset intake | [VN Asset Intake Scope](gameterm-scene-vn-asset-intake-scope.md) | First-pass implemented |
| VN demo install | [VN Demo Install Scope](gameterm-scene-vn-demo-install-scope.md) | First-pass implemented |
| VN real asset run | [VN Real Assets Run Scope](gameterm-scene-vn-real-assets-run-scope.md) | Complete |
| VN local PSD/image export | [VN Real Assets Run Scope](gameterm-scene-vn-real-assets-run-scope.md) | Complete |
| VN staged presentation | [VN Presentation Scope](gameterm-scene-vn-presentation-scope.md) | First-pass implemented |
| Codex compose bridge | [Codex Compose Bridge Scope](gameterm-scene-codex-compose-bridge-scope.md) | First-pass implemented |
| Codex session bridge | [Codex Session Bridge Scope](gameterm-scene-codex-session-bridge-scope.md) | First-pass implemented |
| Real Codex dogfood pass | [Real Codex Dogfood Scope](gameterm-scene-real-codex-dogfood-scope.md) | Scoped |
| Dogfood workspace | [Dogfood Workspace Scope](gameterm-scene-dogfood-workspace-scope.md) | Scoped |
| Current handoff | [Scene Mode Handoff](gameterm-scene-handoff.md) | Current session snapshot |
| Next actions | [Scene Next Actions Scope](gameterm-scene-next-actions-scope.md) | Current execution scope |
| Next product lanes | [Scene Next Product Lanes Scope](gameterm-scene-next-product-lanes-scope.md) | Scoped |
| Live pane/process context | [Pane And Process Discovery Scope](gameterm-scene-pane-process-discovery-scope.md) | Implemented through explicit metadata |
| Agent/Workspace authored model | [Agent And Workspace Scope](gameterm-scene-agent-workspace-scope.md) | First-pass implemented |
| Workspace Discovery | [Workspace Discovery Scope](gameterm-scene-workspace-discovery-scope.md) | First-pass implemented |
| Generated layout and assets | [Visual Layout And Assets Scope](gameterm-scene-visual-layout-assets-scope.md) | First-pass implemented |
| Stabilization/refactor pass | [Stabilization Refactor Scope](gameterm-scene-stabilization-refactor-scope.md) | First pass complete |
| Refactor backlog | [Refactor Plan](gameterm-scene-refactor-plan.md) | Active backlog, do only scoped lanes |
| Smoke checklist | [Product Smoke Checklist](gameterm-scene-product-smoke.md) | Current smoke procedure |
| Smoke results | [Smoke Report](gameterm-scene-smoke-report.md) | Current live/deterministic evidence |
| Onboarding | [Onboarding](gameterm-scene-onboarding.md) | Current user workflow |

## Product Direction

Scene Mode should become a terminal-native workspace surface where the user can
see, inspect, and steer live work.

It should answer:

- What workspace am I in?
- What panes, processes, files, tasks, agents, and state matter right now?
- What actions are available?
- Why is an action blocked?
- What changed after a command, patch, or agent update?
- What should I inspect or do next?

Scene Mode is not a replacement for the shell, editor, mux, or agent runtime.
It is a stateful visual layer over them.

## Priority Stack

### Priority 0: VN Script Import

Status: first-pass implemented. The canonical `.rpy` subset importer is now
Rust-native and engine-agnostic at the product layer.

Goal: prove Scene Mode can host a real visual-novel-shaped demo by importing a
conservative visual-novel script subset into a valid Scene Mode fixture.

Why it matters: Scene Mode already has dialogue, choices, variables, guarded
state, story persistence, and action policy. A VN script import tests whether
those primitives can carry outside VN-style authoring formats without turning
GameTerm into a full VN engine.

Scope owners:

- [Ren'Py Demo Scope](gameterm-scene-renpy-demo-scope.md)
- [VN Script Import Scope](gameterm-scene-vn-script-import-scope.md)

Completed first slice:

- import a small GameTerm-authored Ren'Py-shaped fixture source
- preserve source/license attribution
- convert labels, dialogue, menus, jumps, simple assignments, and simple guards
- generate a valid Scene Mode fixture
- verify import, doctor, and runtime traversal in CI
- record open-license VN asset source policy
- move the importer from Python to Rust
- name the product layer around VN scripts, not a single source engine
- construct `VisualScene` directly in `gameterm-visual`
- expose a Rust example/CLI for fixture generation
- remove the Python helper so there is only one canonical importer

Verified behavior:

- `scene_vn_script_import` reads the GameTerm-owned `.rpy` fixture source.
- Generated choices use `policy.origin=vn_script_import`.
- `ci/gameterm-scene-verify.sh --all` covers Rust import, attribution, doctor
  validation, and runtime traversal.
- Legacy `renpy_import` remains accepted by validation for older scenes.

Deferred:

- full Ren'Py interpreter
- Python execution
- audio, transitions, rollback, screen language, and exact VN staging
- committing demo assets before attribution is fully represented

### Priority 0.5: VN Asset Intake

Status: first-pass implemented.

Goal: make approved VN character and background assets usable in Scene Mode
through local asset intake, stable sprite IDs, generated `sprites.json`, and
attribution.

Why it matters: VN Script Import can build the dialogue and choice structure,
but the user needs a concrete way to use open art packs without copying
unreviewed files into the repo or hardcoding local paths into scenes.

Scope owner:

- [VN Asset Intake Scope](gameterm-scene-vn-asset-intake-scope.md)

Completed first slice:

- validate the open asset catalog policy
- copy approved local files into the user's Scene asset cache
- generate stable VN sprite IDs
- generate `sprites.json`, bindings, and attribution files
- verify the manifest with the existing Scene doctor

Verified behavior:

- `scene_vn_asset_intake` reads the curated open asset catalog.
- Approved local character sprite files are copied into a chosen asset output
  root.
- Generated `sprites.json`, bindings, and attribution files are serialized by
  the Rust example.
- Local school background sources are copied when present and tracked in
  attribution.
- Sprite-parts sources report composition-required warnings instead of broken
  manifest entries.
- `ci/gameterm-scene-verify.sh --all` covers the intake path with repo-owned
  placeholder fixtures.

Deferred:

- automatic itch.io download/login
- committing third-party art before attribution is represented
- sprite-parts composition beyond warning/reporting

### Priority 0.75: VN Demo Install

Status: first-pass implemented.

Goal: combine Rust VN script import and Rust VN asset intake into one safe local
demo install workflow.

Why it matters: VN Script Import and VN Asset Intake now work independently.
The user still needs a single workflow that writes `default.json`,
`sprites.json`, bindings, and attribution into the Scene config directory
without hand-wiring commands or risking accidental overwrite.

Scope owner:

- [VN Demo Install Scope](gameterm-scene-vn-demo-install-scope.md)

Implemented first pass:

- `scene_vn_script_import` accepts `--bindings` and can reference VN sprite IDs
- `ci/gameterm-scene-vn-demo.sh` supports `generate`, `install`, and `doctor`
- generated output is validated and doctored before install
- installs refuse overwrites unless `--force` is passed
- verifier coverage checks generate, install, doctor, bindings, attribution,
  and overwrite protection

Deferred:

- automatic asset downloads
- full VN staging/expression changes per dialogue line
- app bundle installer integration

### Priority 0.8: VN Real Asset Run

Status: complete.

Goal: make the VN demo run with real local PNG assets, make doctor catch fake
image placeholders in strict mode, and add a repeatable `vn-demo` smoke
scenario.

Why it matters: the current VN pipeline proves Rust import, intake, bindings,
attribution, install safety, and doctor wiring, but the repo-safe fixture assets
are text placeholders. A real asset run proves Scene Mode can render an actual
VN-shaped demo without committing unclear-license art.

Scope owner:

- [VN Real Assets Run Scope](gameterm-scene-vn-real-assets-run-scope.md)

Completed slice:

- repo-safe VN fixture assets are real PNG files
- approved local PNG assets can flow through `--asset-source-root`
- opt-in strict image validation is available through doctor
- `vn-demo` smoke scenario opens and captures the generated VN demo
- live smoke is recorded with fixture-mode real PNG assets
- `ci/gameterm-scene-vn-image-export.sh` can flatten local PSD/image downloads
  into the current VN source-root layout
- live smoke is recorded with local downloaded PSD character art and
  local school backgrounds

Non-goals:

- automatic asset downloads
- DDLC/proprietary asset import
- committing user-downloaded third-party art
- renderer redesign for full VN staging

Next likely VN pass:

- staged VN presentation with expression/background changes from imported VN
  script state
- better normal-view placement/scaling for character and background assets
- clearer support for alternate local source ids and multi-character local
  asset roots

Scope owner:

- [VN Presentation Scope](gameterm-scene-vn-presentation-scope.md)

### Priority 1: Live Pane And Process Context

Status: complete for explicit metadata input and live mux auto-discovery.

Goal: connect Workspace Discovery to the active GameTerm session.

Why it mattered: Workspace Discovery knew the repo, but not enough about the
current pane/process to feel like a live operational surface.

Scope owner:

- [Pane And Process Discovery Scope](gameterm-scene-pane-process-discovery-scope.md)
- [Live Mux Discovery Scope](gameterm-scene-live-mux-discovery-scope.md)

Completed first slice:

- accept optional pane metadata input in the workspace helper
- represent pane cwd, mux window id, pane id, foreground process, and process
  phase as explicit Scene state when available
- keep missing metadata non-fatal
- verify with deterministic fixture metadata before live mux smoke

Verified behavior:

- `ci/gameterm-scene-workspace.sh inspect`, `discover`, and `patch` accept
  explicit pane/process metadata.
- `--pane-cwd` becomes the discovery cwd when `--cwd` is absent.
- Generated scenes include `discovered-pane`, `discovered-process`,
  `pane_context`, `active_pane_id`, `active_mux_window_id`, and
  `process_phase` when supplied.
- Generated patches update workspace/process metadata and write typed
  `process_state` when a foreground process is known.
- `ci/gameterm-scene-verify.sh --all` covers supplied and missing metadata.

Deferred:

- multi-pane process maps
- progress polling

### Priority 2: Normal View Product Polish

Status: complete for the first product-summary pass; second-pass layout work
is deferred.

Goal: make Scene Mode useful without opening the Tile Debugger.

Why it mattered: the debugger explained state well, but daily use needed the
common path visible in normal view.

Scope owner:

- [Normal View Polish Scope](gameterm-scene-normal-view-polish-scope.md)

Completed first slice:

- clarify selected entity, active layers, status, blockers, process/agent
  state, and choices in normal view
- keep small terminal windows readable
- add focused render tests and screenshot smoke where layout changes matter

Verified behavior:

- selected entity metadata, relationships, active layers, process state,
  variables, RPG state, and story-state status render in normal view
- choices are grouped by action kind
- focused tests cover selected entity, product-state summary, and grouped
  choices

Deferred:

- two-column layout
- terminal-size-specific layout planner
- screenshot assertions across several terminal sizes

### Priority 3: Command Selection Surface

Status: complete through the command policy second pass.

Goal: avoid turning generated workspaces into long flat choice lists.

Why it mattered: Workspace Discovery can produce many files and actions;
repeated use needed grouped, explicit action selection.

Scope owner:

- [Command Selection Scope](gameterm-scene-command-selection-scope.md)

Completed slices:

- group actions by selected entity, file, project, or process
- preserve existing choices and explicit activation semantics
- ensure `RunCommand` and `OpenFile` remain visibly different
- expose derived command option rows
- filter command options by text, action kind, risk, scope, and enabled state
- render a runtime command-selection view with policy rows

Verified behavior:

- normal view groups choices by action kind
- selected choice markers remain attached to the original choices
- `RunCommand` and `OpenFile` remain explicit, user-activated actions
- command-selection view preserves selected-entity context while moving
  between command choices
- focused tests cover grouped choice rendering, policy rendering, filtering,
  and command-selection activation

Deferred:

- explicit action group metadata
- GUI overlay command palette
- persistent command history

### Priority 4: Policy And Permission Boundaries

Status: complete through the command policy second pass.

Goal: make generated and future agent-proposed actions auditable before they
can run.

Why it mattered: command grouping increases action surface area; policy keeps
that surface explicit and safe.

Scope owner:

- [Policy Boundaries Scope](gameterm-scene-policy-boundaries-scope.md)

Completed slices:

- require explicit command shape through validation and doctor diagnostics
- warn when reusable or generated `RunCommand` choices omit cwd
- keep generated workspace `RunCommand` choices cwd-explicit
- preserve explicit activation for all command execution
- add optional action policy metadata with origin, risk, scope, confirmation,
  and summary
- derive policy defaults for old scenes without metadata
- render policy in normal view, Tile Debugger, and command-selection view
- generate deterministic policy metadata from Workspace Discovery

Verified behavior:

- `RunCommand` requires explicit argv and target
- doctor validates targets and warns about missing cwd
- verifier covers target diagnostics and missing-cwd policy warnings
- discovery docs state that generated workflows do not run commands or start
  agents automatically
- old fixtures and user scenes without policy still load
- generated workspace scenes include `policy.origin=workspace_discovery`
- command actions remain visibly distinct from inspect/open/navigate actions

Deferred:

- allowlist enforcement
- agent-proposed command approval UI
- workspace command sandboxing

### Priority 5: Persisted Workspace Sessions

Status: complete for helper-driven first pass; GUI session browser and default
state-directory wiring are deferred.

Goal: restore useful workspace state across repeated Scene Mode use.

Why it mattered: session persistence lets users preserve useful workspace state
without treating generated scene JSON as the only durable storage.

Scope owner:

- [Workspace Sessions Scope](gameterm-scene-workspace-sessions-scope.md)

Completed first slice:

- define what session state is worth saving
- avoid tracked or repo-local churn
- keep generated scenes reproducible from explicit inputs
- save, restore, validate, and inspect workspace session JSON through
  `ci/gameterm-scene-session.sh`
- document workspace-session commands in the Scene Mode docs
- verify overwrite protection and non-mutating restore

### Priority 6: Memory, Relationships, And Multi-Agent State

Status: complete for local explicit relationship and multi-agent fixture
passes; live concurrent agent orchestration is deferred.

Goal: make memory and multi-agent state useful as product behavior, not only
as model data.

Why it mattered: the schema could represent this state; the first product pass
needed it visible and verifiable in generated/local scenes.

Scope owners:

- [Memory Relationships Scope](gameterm-scene-memory-relationships-scope.md)
- [Multi-Agent Coordination Scope](gameterm-scene-multi-agent-coordination-scope.md)
- [Agent Task Bootstrap Scope](gameterm-scene-agent-task-bootstrap-scope.md)

Completed first slice:

- show relationship/memory state in the polished normal view
- connect multiple agent/process updates to live pane/process context
- keep task bootstrap explicit and user-confirmed
- generate and validate local workspace relationships
- support two-agent/two-task coordination fixture and scoped agent patches
- generate local task briefs without starting agents

### Priority 7: Packaging And Onboarding

Status: complete for command-first onboarding; app-bundle onboarding is
deferred.

Goal: make Scene Mode easy to use repeatedly without tribal knowledge.

Why it mattered: onboarding should describe the settled product path, not every
intermediate helper detail.

Scope owner:

- [Packaging Onboarding Scope](gameterm-scene-packaging-onboarding-scope.md)

Completed first slice:

- command-first onboarding workflow
- keep quickstart commands current
- document dry-run, validation, doctor, install, launch, smoke, and recovery
- verify onboarding docs reference real helper commands

## Refactor Backlog

Refactor work should remain subordinate to product priorities. Do it only when
it directly reduces risk for the next lane.

Allowed next refactor lanes:

- table-drive smoke scenario definitions if smoke edits become hard to review
- extract small helper registries from author/workspace scripts when adding
  new product actions
- move narrow runtime helpers out of `lib.rs` only when a lane already touches
  that area

Avoid:

- broad crate splits
- schema renames without migration
- visual redesign mixed with NFC moves
- unrelated macOS warning cleanup inside Scene Mode commits

## Next Recommended Work

Start the dogfood workspace product gate.

Reasoning:

- The first maintainable refactor pass and author-helper cleanup pass are
  complete.
- Scene Mode now needs a daily-use path so we can dogfood it while developing
  GameTerm.
- The existing workspace discovery, boot menu, default scene loading, and smoke
  harness are enough to scope a narrow dogfood profile without changing the
  Scene runtime schema.

Concrete next checklist:

1. Use [Dogfood Workspace Scope](gameterm-scene-dogfood-workspace-scope.md).
2. Add the dogfood workspace profile.
3. Add deterministic verifier coverage.
4. Add the dogfood smoke scenario.
5. Verify the boot/menu path loads the installed dogfood default scene.
6. Record live fullscreen dogfood smoke.
