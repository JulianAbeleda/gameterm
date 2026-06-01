# GameTerm Scene Mode Fixtures

These fixtures exercise Scene Mode without touching user config.

- `default.json`: normal scene with Inspect, OpenFile, Navigate, and explicit-argv RunCommand choices.
- `memory.json`: navigation target scene.
- `invalid.json`: syntactically valid JSON that fails scene validation.
- `patch-status.json`: valid in-memory scene patch for the default fixture.
- `patch-unknown-entity.json`: invalid patch that references a missing entity.
- `run-command-targets.json`: command target fixture for tab, right split, and down split live audits.
- `vertical-slice.json`: playable Scene Mode slice for dialogue, RPG state, and layered/process state.
- `authoring-loop.json`: in-app story-state save, mutate, and reload workflow fixture.
- `game-states.json`: common layered game-state fixture for dialogue, exploration, inventory, command, quest, and agent state.
- `chained-transitions.json`: guarded multi-step state transition fixture for dialogue, inventory, quest, command, stat, and agent state.
- `workspace-agent.json`: Agent/Workspace product slice for workspace, project, task, agent, process, and file entities.
- `multi-agent-coordination.json`: two-agent/two-task coordination fixture with ownership, waiting, blocking, and completion state.
- `renpy-demo-source.rpy`: GameTerm-owned `.rpy` source fixture for VN script import verification; not copied from Ren'Py.
- `renpy-demo.json`: generated Scene Mode import of the `.rpy` fixture source.
- `renpy-demo-attribution.json`: attribution/provenance manifest generated beside the imported demo scene.
- `renpy-demo-open-assets.json`: curated open-license VN asset source policy for DDLC-adjacent demo art.
- `sprites.json`: sprite manifest that points at bundled test assets.
- `sprites-missing.json`: sprite manifest with one intentionally missing sprite path.

## Scenario Ownership

| Fixture | Proves | Focused verifier/test coverage |
| --- | --- | --- |
| `default.json` | Baseline Scene Mode runtime actions, default mode, entity selection, and command requests. | `scene_fixture_default_loads_runtime_actions`, `run_author_helper_check`, fixture `basic` |
| `memory.json` | Navigation target loading and scene source replacement. | `scene_fixture_memory_loads_navigation_target`, fixture `navigate` |
| `invalid.json` | Parser accepts JSON but schema validation rejects invalid scene content. | `scene_fixture_invalid_is_rejected`, author `validate` negative check, fixture `invalid` |
| `patch-status.json` | Scene patch transport can update entity status, visibility, and process metadata. | `scene_patch_fixture_applies_to_default_scene`, `scene_patch_updates_entity_state_and_status` |
| `patch-unknown-entity.json` | Patch application fails without mutating state when an entity is missing. | `scene_patch_fixture_rejects_unknown_entity`, `scene_patch_rejects_unknown_entity_without_mutation` |
| `run-command-targets.json` | RunCommand target variants remain valid for tab, right split, and down split. | `run_command_action_emits_explicit_argv_request`, doctor target checks, fixture `run-command-targets` |
| `vertical-slice.json` | First-pass playable loop across dialogue, RPG state, layers, choices, and status. | `scene_fixture_vertical_slice_completes_product_loop`, smoke scenario `vertical-slice`, fixture `vertical-slice` |
| `authoring-loop.json` | Story-state export/import and authoring-mode status rendering. | `story_state_actions_emit_pending_requests`, smoke scenario `authoring-loop`, fixture `authoring-loop` |
| `game-states.json` | Common game computational modes: exploration, dialogue, inventory, command, quest, and agent state. | `scene_fixture_game_states_covers_common_modes`, fixture `game-states` |
| `chained-transitions.json` | Guarded deterministic transition chains over dialogue, inventory, quest, command, stat, and agent state. | `scene_fixture_chained_transitions_completes_state_chain`, fixture `chained-transitions` |
| `workspace-agent.json` | Agent/Workspace product loop across workspace, project, task, agent, process, and file entities. | `scene_fixture_workspace_agent_completes_product_loop`, smoke scenario `workspace-agent`, fixture `workspace-agent` |
| `multi-agent-coordination.json` | Multi-agent coordination across two agents, two tasks, relationship ownership, waiting, blocking, and completion state. | `scene_fixture_multi_agent_coordination_updates_independently`, fixture `multi-agent-coordination` |
| `renpy-demo.json` | VN script import path for labels, dialogue, menu choices, guards, policy metadata, and attribution. | `scene_fixture_renpy_demo_import_loads_story_choices`, importer check, fixture `renpy-demo` |
| `renpy-demo-open-assets.json` | Open-license asset source policy for optional VN demo art, including repo-safe and local-only boundaries. | script import and asset intake checks |
| `sprites.json` | Sprite manifest resolution against bundled assets. | `scene_fixture_sprite_manifest_resolves_relative_paths`, fixture `sprites` |
| `sprites-missing.json` | Missing sprite paths are reported without dropping valid sprite entries. | `scene_fixture_missing_sprite_manifest_keeps_valid_entries`, fixture `missing-sprite` |
