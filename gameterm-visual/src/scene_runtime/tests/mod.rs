use super::*;
use crate::*;
mod test_support;
use test_support::{branching_dialogue_scene, scene_fixture_path, snapshot_for_filtering};

#[test]
fn demo_scene_validates() {
    k9::assert_ok!(VisualScene::demo().validate());
}

#[test]
fn scene_without_mode_uses_default_workspace_mode() {
    let scene = VisualScene::from_json(
        r#"{
                "title": "Legacy Scene",
                "background": "floor",
                "width": 2,
                "height": 2,
                "entities": [],
                "dialogue_speaker": "Narrator",
                "dialogue": "No explicit mode.",
                "choices": []
            }"#,
    )
    .unwrap();

    assert_eq!(scene.mode, default_scene_mode());
}

#[test]
fn scene_rejects_empty_mode_id() {
    let mut scene = VisualScene::demo();
    scene.mode.mode_id = " ".to_string();

    assert!(matches!(
        scene.validate(),
        Err(VisualSceneError::EmptyModeId)
    ));
}

#[test]
fn scene_rejects_empty_mode_label() {
    let mut scene = VisualScene::demo();
    scene.mode.label = " ".to_string();

    assert!(matches!(
        scene.validate(),
        Err(VisualSceneError::EmptyModeLabel)
    ));
}

#[test]
fn scene_rejects_empty_mode_allowed_action() {
    let mut scene = VisualScene::demo();
    scene.mode.allowed_actions.push(" ".to_string());

    assert!(matches!(
        scene.validate(),
        Err(VisualSceneError::EmptyModeAllowedAction)
    ));
}

#[test]
fn scene_rejects_empty_mode_lifecycle_status() {
    let mut scene = VisualScene::demo();
    scene.mode.lifecycle.update_status = Some(" ".to_string());

    assert_eq!(
        scene.validate(),
        Err(VisualSceneError::EmptyModeLifecycleStatus)
    );
}

#[test]
fn mode_lifecycle_hooks_update_status_and_generation() {
    let mut scene = VisualScene::demo();
    scene.mode.lifecycle = VisualModeLifecycle {
        enter_status: Some("Entered conversation".to_string()),
        update_status: Some("Conversation update".to_string()),
        exit_status: Some("Exited conversation".to_string()),
    };
    let mut runtime = SceneRuntime::new(scene).unwrap();

    assert_eq!(runtime.render_snapshot().status, "Entered conversation");
    let entered_generation = runtime.generation();

    runtime.run_mode_update_hooks();
    assert!(runtime.generation() > entered_generation);
    assert_eq!(runtime.render_snapshot().status, "Conversation update");

    runtime.run_mode_exit_hooks();
    assert_eq!(runtime.render_snapshot().status, "Exited conversation");
    assert_eq!(
        runtime
            .debug_report()
            .active_mode_lifecycle
            .update_status
            .as_deref(),
        Some("Conversation update")
    );

    let frame = runtime.render_debugger(200, 80);
    assert!(frame.contains("Lifecycle: enter update exit"));
}

#[test]
fn scene_rejects_empty_variable_key() {
    let mut scene = VisualScene::demo();
    scene.variables.push(VisualStateEntry {
        key: " ".to_string(),
        value: VisualStateValue::Text("bad".to_string()),
    });

    assert!(matches!(
        scene.validate(),
        Err(VisualSceneError::EmptyVariableKey)
    ));
}

#[test]
fn scene_rejects_duplicate_variable_key() {
    let mut scene = VisualScene::demo();
    scene.variables.push(VisualStateEntry {
        key: "workspace_level".to_string(),
        value: VisualStateValue::Number(2),
    });

    assert!(matches!(
        scene.validate(),
        Err(VisualSceneError::DuplicateVariableKey(key)) if key == "workspace_level"
    ));
}

#[test]
fn rpg_state_is_visible_in_snapshot_and_debugger() {
    let runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
    let snapshot = runtime.render_snapshot();

    assert_eq!(snapshot.rpg.inventory.len(), 1);
    assert_eq!(snapshot.rpg.inventory[0].item_id, "scene-token");
    assert_eq!(snapshot.rpg.stats.len(), 1);
    assert_eq!(snapshot.rpg.quests[0].quest_id, "verify-scene-runtime");
    assert_eq!(snapshot.rpg.relationships[0].kind, "monitors");

    let report = runtime.debug_report();
    assert_eq!(report.rpg, snapshot.rpg);

    let frame = runtime.render_debugger(120, 48);
    assert!(frame.contains("RPG:"));
    assert!(frame.contains("Inventory items: 1"));
    assert!(frame.contains("Relationships: 1"));
}

#[test]
fn relationships_are_visible_in_normal_view_and_debugger() {
    let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
    runtime.select_next_entity();
    runtime.select_next_entity();

    let normal = runtime.render_text_frame(120, 40);
    assert!(normal.contains("Relationships: out=1 in=0"));
    assert!(normal.contains("-> Render Scene (task-render) monitors"));

    let debugger = runtime.render_debugger(140, 80);
    assert!(
        debugger.contains("Audit Agent (agent-audit) --monitors(2)--> Render Scene (task-render)")
    );
    assert!(debugger.contains("Selected relationships:"));
}

#[test]
fn resolve_action_updates_story_and_rpg_state_atomically() {
    let mut scene = VisualScene::demo();
    scene.choices.insert(
        0,
        SceneAction {
            label: "Resolve quest reward".to_string(),
            kind: SceneActionKind::Resolve {
                operations: vec![
                    VisualStateOperation::SetVariable {
                        key: "quest_reward_claimed".to_string(),
                        value: VisualStateValue::Bool(true),
                    },
                    VisualStateOperation::AddInventory {
                        item: VisualInventoryItem {
                            item_id: "memory-key".to_string(),
                            label: "Memory Key".to_string(),
                            count: 1,
                            metadata: Vec::new(),
                        },
                    },
                    VisualStateOperation::AdvanceQuest {
                        quest_id: "verify-scene-runtime".to_string(),
                        stage: 2,
                    },
                ],
            },
            policy: None,
            conditions: Vec::new(),
        },
    );
    let mut runtime = SceneRuntime::new(scene).unwrap();

    runtime.activate_choice();

    let snapshot = runtime.render_snapshot();
    assert_eq!(
        snapshot.status,
        "Resolved 3 operation(s): Resolve quest reward"
    );
    assert!(snapshot.variables.iter().any(|entry| {
        entry.key == "quest_reward_claimed" && entry.value == VisualStateValue::Bool(true)
    }));
    assert!(snapshot
        .rpg
        .inventory
        .iter()
        .any(|item| item.item_id == "memory-key" && item.count == 1));
    assert_eq!(snapshot.rpg.quests[0].stage, 2);
}

#[test]
fn resolve_action_failure_does_not_partially_mutate_state() {
    let mut scene = VisualScene::demo();
    scene.choices.insert(
        0,
        SceneAction {
            label: "Broken reward".to_string(),
            kind: SceneActionKind::Resolve {
                operations: vec![VisualStateOperation::SetVariable {
                    key: "should_not_apply".to_string(),
                    value: VisualStateValue::Bool(true),
                }],
            },
            policy: None,
            conditions: Vec::new(),
        },
    );
    let mut runtime = SceneRuntime::new(scene).unwrap();
    runtime.scene.choices[0].kind = SceneActionKind::Resolve {
        operations: vec![
            VisualStateOperation::SetVariable {
                key: "should_not_apply".to_string(),
                value: VisualStateValue::Bool(true),
            },
            VisualStateOperation::AdvanceQuest {
                quest_id: "missing-quest".to_string(),
                stage: 9,
            },
        ],
    };

    runtime.activate_choice();

    let snapshot = runtime.render_snapshot();
    assert!(snapshot
        .status
        .starts_with("Resolve failed: Resolve action `Broken reward` references unknown quest"));
    assert!(!snapshot
        .variables
        .iter()
        .any(|entry| entry.key == "should_not_apply"));
}

#[test]
fn resolve_action_updates_layer_state_atomically() {
    let mut scene = VisualScene::demo();
    scene.layers = vec![VisualLayerState {
        layer_id: "story".to_string(),
        state: "dialogue".to_string(),
        label: Some("Story".to_string()),
        input_map: Vec::new(),
        transitions: Vec::new(),
    }];
    scene.choices.insert(
        0,
        SceneAction {
            label: "Complete story beat".to_string(),
            kind: SceneActionKind::Resolve {
                operations: vec![
                    VisualStateOperation::SetLayerState {
                        layer_id: "story".to_string(),
                        state: "resolved".to_string(),
                    },
                    VisualStateOperation::SetVariable {
                        key: "story_resolved".to_string(),
                        value: VisualStateValue::Bool(true),
                    },
                ],
            },
            policy: None,
            conditions: Vec::new(),
        },
    );
    let mut runtime = SceneRuntime::new(scene).unwrap();

    runtime.activate_choice();

    let snapshot = runtime.render_snapshot();
    assert_eq!(
        snapshot.status,
        "Resolved 2 operation(s): Complete story beat"
    );
    assert_eq!(snapshot.active_layers[0].state, "resolved");
    assert!(snapshot.variables.iter().any(|entry| {
        entry.key == "story_resolved" && entry.value == VisualStateValue::Bool(true)
    }));
}

#[test]
fn resolve_action_rejects_unknown_layer_without_mutation() {
    let mut scene = VisualScene::demo();
    scene.layers = vec![VisualLayerState {
        layer_id: "story".to_string(),
        state: "dialogue".to_string(),
        label: Some("Story".to_string()),
        input_map: Vec::new(),
        transitions: Vec::new(),
    }];
    scene.choices.insert(
        0,
        SceneAction {
            label: "Broken layer transition".to_string(),
            kind: SceneActionKind::Resolve {
                operations: vec![
                    VisualStateOperation::SetVariable {
                        key: "should_not_apply".to_string(),
                        value: VisualStateValue::Bool(true),
                    },
                    VisualStateOperation::SetLayerState {
                        layer_id: "missing".to_string(),
                        state: "resolved".to_string(),
                    },
                ],
            },
            policy: None,
            conditions: Vec::new(),
        },
    );

    assert_eq!(
        scene.validate(),
        Err(VisualSceneError::UnknownLayer {
            label: "Broken layer transition".to_string(),
            layer_id: "missing".to_string()
        })
    );

    scene.choices[0].kind = SceneActionKind::Resolve {
        operations: vec![
            VisualStateOperation::SetVariable {
                key: "should_not_apply".to_string(),
                value: VisualStateValue::Bool(true),
            },
            VisualStateOperation::SetLayerState {
                layer_id: "story".to_string(),
                state: "resolved".to_string(),
            },
            VisualStateOperation::AdjustStat {
                owner_id: Some("project-gameterm".to_string()),
                key: "missing".to_string(),
                amount: 1,
            },
        ],
    };
    let mut runtime = SceneRuntime::new(scene).unwrap();

    runtime.activate_choice();

    let snapshot = runtime.render_snapshot();
    assert!(snapshot.status.starts_with(
        "Resolve failed: Resolve action `Broken layer transition` references unknown stat"
    ));
    assert_eq!(snapshot.active_layers[0].state, "dialogue");
    assert!(!snapshot
        .variables
        .iter()
        .any(|entry| entry.key == "should_not_apply"));
}

#[test]
fn resolve_action_updates_existing_numeric_values() {
    let mut scene = VisualScene::demo();
    scene.choices.insert(
        0,
        SceneAction {
            label: "Resolve counters".to_string(),
            kind: SceneActionKind::Resolve {
                operations: vec![
                    VisualStateOperation::IncrementVariable {
                        key: "workspace_level".to_string(),
                        amount: 3,
                    },
                    VisualStateOperation::AdjustStat {
                        owner_id: Some("project-gameterm".to_string()),
                        key: "focus".to_string(),
                        amount: 2,
                    },
                    VisualStateOperation::AdjustRelationship {
                        source_id: "agent-audit".to_string(),
                        target_id: "task-render".to_string(),
                        kind: "monitors".to_string(),
                        amount: 1,
                    },
                ],
            },
            policy: None,
            conditions: Vec::new(),
        },
    );
    let mut runtime = SceneRuntime::new(scene).unwrap();

    runtime.activate_choice();

    let snapshot = runtime.render_snapshot();
    assert!(snapshot.variables.iter().any(|entry| {
        entry.key == "workspace_level" && entry.value == VisualStateValue::Number(4)
    }));
    assert_eq!(snapshot.rpg.stats[0].value, VisualStateValue::Number(5));
    assert_eq!(snapshot.rpg.relationships[0].value, 3);
}

#[test]
fn resolve_action_updates_entity_and_dialogue_state() {
    let mut scene = branching_dialogue_scene();
    scene.layers = vec![VisualLayerState {
        layer_id: "story".to_string(),
        state: "dialogue".to_string(),
        label: None,
        input_map: Vec::new(),
        transitions: Vec::new(),
    }];
    scene.choices.insert(
        0,
        SceneAction {
            label: "Resolve entity state".to_string(),
            kind: SceneActionKind::Resolve {
                operations: vec![
                    VisualStateOperation::SelectEntity {
                        entity_id: "task-render".to_string(),
                    },
                    VisualStateOperation::SetEntityFlags {
                        entity_id: "task-render".to_string(),
                        flags: vec!["focused".to_string(), "ready".to_string()],
                    },
                    VisualStateOperation::SetEntityMetadata {
                        entity_id: "task-render".to_string(),
                        metadata: vec![("mode".to_string(), "command".to_string())],
                    },
                    VisualStateOperation::SetEntityVisibility {
                        entity_id: "task-render".to_string(),
                        visible: false,
                    },
                    VisualStateOperation::AdvanceDialogueAndSetLayer {
                        target: 1,
                        layer_id: "story".to_string(),
                        state: "exploration".to_string(),
                    },
                ],
            },
            policy: None,
            conditions: Vec::new(),
        },
    );
    let mut runtime = SceneRuntime::new(scene).unwrap();

    runtime.activate_choice();

    let snapshot = runtime.render_snapshot();
    let report = runtime.debug_report();
    assert_eq!(snapshot.selected_entity_id.as_deref(), Some("task-render"));
    assert!(!snapshot
        .entities
        .iter()
        .any(|entity| entity.id == "task-render"));
    assert_eq!(report.selected_entity_flags, ["focused", "ready"]);
    assert_eq!(
        report.selected_entity_metadata,
        [("mode".to_string(), "command".to_string())]
    );
    assert_eq!(snapshot.dialogue_speaker, "Guide");
    assert_eq!(snapshot.active_layers[0].state, "exploration");
    assert!(report.transition_history.iter().any(|event| event
        .detail
        .contains("select task-render, flags task-render=[focused,ready]")));
}

#[test]
fn resolve_action_triggers_layer_transition_with_rollback_on_guard_failure() {
    let mut scene = VisualScene::demo();
    scene.layers = vec![VisualLayerState {
        layer_id: "story".to_string(),
        state: "dialogue".to_string(),
        label: None,
        input_map: Vec::new(),
        transitions: vec![VisualLayerTransition {
            input: "activate".to_string(),
            target_state: "exploration".to_string(),
            conditions: vec![VisualCondition {
                source: None,
                variable: "route_open".to_string(),
                equals: VisualStateValue::Bool(true),
            }],
        }],
    }];
    scene.choices.insert(
        0,
        SceneAction {
            label: "Blocked transition".to_string(),
            kind: SceneActionKind::Resolve {
                operations: vec![
                    VisualStateOperation::SetVariable {
                        key: "should_rollback".to_string(),
                        value: VisualStateValue::Bool(true),
                    },
                    VisualStateOperation::TriggerLayerTransition {
                        layer_id: "story".to_string(),
                        input: "activate".to_string(),
                    },
                ],
            },
            policy: None,
            conditions: Vec::new(),
        },
    );
    let mut runtime = SceneRuntime::new(scene).unwrap();

    runtime.activate_choice();

    let snapshot = runtime.render_snapshot();
    assert!(snapshot.status.starts_with(
        "Resolve failed: Resolve action `Blocked transition` blocked by layer transition guard"
    ));
    assert_eq!(snapshot.active_layers[0].state, "dialogue");
    assert!(!snapshot
        .variables
        .iter()
        .any(|entry| entry.key == "should_rollback"));
}

#[test]
fn scene_rejects_empty_resolve_action() {
    let mut scene = VisualScene::demo();
    scene.choices[0].kind = SceneActionKind::Resolve {
        operations: Vec::new(),
    };

    assert_eq!(
        scene.validate(),
        Err(VisualSceneError::EmptyResolveOperations {
            label: "Inspect selected entity".to_string()
        })
    );
}

#[test]
fn scene_rejects_duplicate_inventory_item_id() {
    let mut scene = VisualScene::demo();
    scene.rpg.inventory.push(scene.rpg.inventory[0].clone());

    assert!(matches!(
        scene.validate(),
        Err(VisualSceneError::DuplicateInventoryItemId(id)) if id == "scene-token"
    ));
}

#[test]
fn scene_rejects_empty_stat_key() {
    let mut scene = VisualScene::demo();
    scene.rpg.stats[0].key = " ".to_string();

    assert_eq!(scene.validate(), Err(VisualSceneError::EmptyStatKey));
}

#[test]
fn scene_rejects_duplicate_quest_id() {
    let mut scene = VisualScene::demo();
    scene.rpg.quests.push(scene.rpg.quests[0].clone());

    assert!(matches!(
        scene.validate(),
        Err(VisualSceneError::DuplicateQuestId(id)) if id == "verify-scene-runtime"
    ));
}

#[test]
fn scene_rejects_duplicate_relationship() {
    let mut scene = VisualScene::demo();
    scene
        .rpg
        .relationships
        .push(scene.rpg.relationships[0].clone());

    assert!(matches!(
        scene.validate(),
        Err(VisualSceneError::DuplicateRelationship(key))
            if key == "agent-audit:task-render:monitors"
    ));
}

#[test]
fn scene_rejects_unknown_relationship_entities() {
    let mut scene = VisualScene::demo();
    scene.rpg.relationships[0].source_id = "missing-agent".to_string();

    assert!(matches!(
        scene.validate(),
        Err(VisualSceneError::UnknownRelationshipSourceId(id)) if id == "missing-agent"
    ));

    let mut scene = VisualScene::demo();
    scene.rpg.relationships[0].target_id = "missing-task".to_string();

    assert!(matches!(
        scene.validate(),
        Err(VisualSceneError::UnknownRelationshipTargetId(id)) if id == "missing-task"
    ));
}

#[test]
fn scene_rejects_empty_choice_condition_variable() {
    let mut scene = VisualScene::demo();
    scene.choices[0].conditions = vec![VisualCondition {
        source: None,
        variable: " ".to_string(),
        equals: VisualStateValue::Bool(true),
    }];

    assert!(matches!(
        scene.validate(),
        Err(VisualSceneError::EmptyConditionVariable { label }) if label == "Inspect selected entity"
    ));
}

#[test]
fn scene_rejects_empty_dialogue_line_speaker() {
    let mut scene = branching_dialogue_scene();
    scene.dialogue_lines[0].speaker = " ".to_string();

    assert!(matches!(
        scene.validate(),
        Err(VisualSceneError::EmptyDialogueSpeaker { index }) if index == 0
    ));
}

#[test]
fn scene_rejects_empty_dialogue_line_text() {
    let mut scene = branching_dialogue_scene();
    scene.dialogue_lines[1].text = " ".to_string();

    assert!(matches!(
        scene.validate(),
        Err(VisualSceneError::EmptyDialogueText { index }) if index == 1
    ));
}

#[test]
fn scene_rejects_dialogue_choice_target_out_of_bounds() {
    let mut scene = branching_dialogue_scene();
    scene.choices[0].kind = SceneActionKind::AdvanceDialogue { target: 99 };

    assert!(matches!(
        scene.validate(),
        Err(VisualSceneError::DialogueTargetOutOfBounds { label, target })
            if label == "Choose workspace" && target == 99
    ));
}

#[test]
fn scene_fixture_default_loads_runtime_actions() {
    let scene = VisualScene::load_from_path(scene_fixture_path("default.json")).unwrap();
    let runtime = SceneRuntime::new(scene).unwrap();
    let snapshot = runtime.render_snapshot();

    assert_eq!(snapshot.title, "Scene Harness Default");
    assert_eq!(snapshot.active_mode.mode_id, "workspace");
    assert!(snapshot.variables.is_empty());
    assert!(snapshot
        .choices
        .iter()
        .any(|choice| choice == "Open scene docs"));
    assert!(snapshot
        .choices
        .iter()
        .any(|choice| choice == "Navigate to memory"));
}

#[test]
fn scene_fixture_memory_loads_navigation_target() {
    let scene = VisualScene::load_from_path(scene_fixture_path("memory.json")).unwrap();
    let runtime = SceneRuntime::new(scene).unwrap();
    let snapshot = runtime.render_snapshot();

    assert_eq!(snapshot.title, "Scene Harness Memory");
    assert_eq!(
        snapshot.selected_entity_id.as_deref(),
        Some("memory-navigation")
    );
}

#[test]
fn scene_fixture_layered_mode_loads_active_layers() {
    let scene = VisualScene::load_from_path(scene_fixture_path("layered-mode.json")).unwrap();
    let runtime = SceneRuntime::new(scene).unwrap();
    let snapshot = runtime.render_snapshot();

    assert_eq!(snapshot.title, "Scene Harness Layered Mode");
    assert_eq!(snapshot.active_layers.len(), 2);
    assert_eq!(snapshot.active_layers[0].layer_id, "ui");
    assert_eq!(snapshot.active_layers[1].layer_id, "story");
}

#[test]
fn scene_fixture_vertical_slice_completes_product_loop() {
    let scene = VisualScene::load_from_path(scene_fixture_path("vertical-slice.json")).unwrap();
    let mut runtime = SceneRuntime::new(scene).unwrap();

    assert_eq!(runtime.render_snapshot().title, "Scene Vertical Slice");
    assert_eq!(runtime.render_snapshot().active_layers.len(), 3);

    runtime.activate_choice();
    runtime.select_next_choice();
    runtime.activate_choice();
    runtime.select_next_choice();
    runtime.activate_choice();
    runtime.select_next_choice();
    runtime.activate_choice();

    let state = runtime.export_story_state();
    assert!(state.variables.iter().any(|entry| {
        entry.key == "agent_phase" && entry.value == VisualStateValue::Text("complete".to_string())
    }));
    assert!(state
        .rpg
        .inventory
        .iter()
        .any(|item| item.item_id == "launch-kit" && item.count == 1));
    assert_eq!(state.rpg.stats[0].value, VisualStateValue::Number(3));
    assert!(state.rpg.quests[0].completed);
    assert!(state.rpg.quests[0]
        .journal
        .contains("Prepared the launch kit."));
    assert_eq!(state.rpg.relationships[0].value, 2);
    assert_eq!(state.dialogue_index, Some(2));

    runtime
        .apply_scene_patch(VisualScenePatch {
            scene_patch_version: VisualScenePatch::VERSION,
            updates: vec![VisualSceneEntityPatch {
                entity_id: "build-task".to_string(),
                label: Some("Launch Check Complete".to_string()),
                position: None,
                sprite: None,
                visible: None,
                state_flags: Some(vec!["succeeded".to_string()]),
                metadata: Some(vec![("process".to_string(), "complete".to_string())]),
            }],
            variables: vec![],
            selected_entity_id: Some("build-task".to_string()),
            process_state: Some(VisualProcessState {
                entity_id: Some("build-task".to_string()),
                phase: VisualProcessPhase::Succeeded,
                command: Some("true".to_string()),
                exit_code: Some(0),
                message: Some("Vertical slice process succeeded".to_string()),
            }),
            dialogue: None,
            status: Some("Vertical slice complete".to_string()),
        })
        .unwrap();

    let report = runtime.debug_report();
    assert_eq!(report.selected_entity_id.as_deref(), Some("build-task"));
    assert_eq!(
        report.process_state.as_ref().map(|state| state.phase),
        Some(VisualProcessPhase::Succeeded)
    );
}

#[test]
fn scene_fixture_workspace_agent_completes_product_loop() {
    let scene = VisualScene::load_from_path(scene_fixture_path("workspace-agent.json")).unwrap();
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let mut runtime = SceneRuntime::new_with_source_and_action_base_dir(
        scene,
        VisualSceneSource::new("workspace-agent.json", VisualSceneLoadStatus::Loaded, 0),
        &repo_root,
    )
    .unwrap();

    let snapshot = runtime.render_snapshot();
    assert_eq!(snapshot.title, "Scene Agent Workspace");
    assert_eq!(snapshot.active_layers.len(), 4);
    assert_eq!(snapshot.status, "Workspace overview ready");
    assert!(snapshot
        .entities
        .iter()
        .any(|entity| entity.id == "scene-agent"
            && entity.state_flags.iter().any(|flag| flag == "agent_idle")));

    runtime.select_next_choice();
    runtime.select_next_choice();
    runtime.activate_choice();
    runtime.select_next_choice();
    runtime.activate_choice();
    runtime.select_next_choice();
    runtime.activate_choice();
    runtime.select_next_choice();
    runtime.activate_choice();

    let snapshot = runtime.render_snapshot();
    assert_eq!(
        snapshot.selected_entity_id.as_deref(),
        Some("scene-agent-workspace-task")
    );
    assert!(snapshot.variables.iter().any(|entry| {
        entry.key == "agent_phase" && entry.value == VisualStateValue::Text("completed".to_string())
    }));
    assert!(snapshot.variables.iter().any(|entry| {
        entry.key == "agent_process_phase"
            && entry.value == VisualStateValue::Text("succeeded".to_string())
    }));
    assert!(snapshot.variables.iter().any(|entry| {
        entry.key == "review_ready" && entry.value == VisualStateValue::Bool(true)
    }));
    assert!(snapshot
        .active_layers
        .iter()
        .any(|layer| layer.layer_id == "workspace" && layer.state == "review"));
    assert!(snapshot
        .active_layers
        .iter()
        .any(|layer| layer.layer_id == "agent" && layer.state == "complete"));
    assert!(snapshot
        .active_layers
        .iter()
        .any(|layer| layer.layer_id == "process" && layer.state == "succeeded"));

    runtime.select_next_choice();
    runtime.activate_choice();
    assert!(runtime
        .render_snapshot()
        .status
        .starts_with("OpenFile ready: "));

    runtime.select_next_choice();
    runtime.activate_choice();
    assert_eq!(
        runtime.take_pending_action(),
        Some(VisualActionRequest::RunCommand {
            argv: vec![
                "ci/gameterm-scene-verify.sh".to_string(),
                "--fixture".to_string(),
                "workspace-agent".to_string(),
            ],
            cwd: Some(PathBuf::from(".")),
            target: RunCommandTarget::SplitDown,
        })
    );
}

#[test]
fn scene_fixture_multi_agent_coordination_updates_independently() {
    let scene =
        VisualScene::load_from_path(scene_fixture_path("multi-agent-coordination.json")).unwrap();
    let mut runtime = SceneRuntime::new(scene).unwrap();

    let snapshot = runtime.render_snapshot();
    assert_eq!(snapshot.title, "Scene Multi-Agent Coordination");
    assert_eq!(snapshot.entities.len(), 5);
    assert_eq!(snapshot.rpg.relationships.len(), 4);
    assert!(snapshot
        .rpg
        .relationships
        .iter()
        .any(|relationship| relationship.source_id == "agent-audit"
            && relationship.target_id == "task-build"
            && relationship.kind == "waits_for"));

    runtime.activate_choice();
    runtime.select_next_choice();
    runtime.activate_choice();
    runtime.select_next_choice();
    runtime.activate_choice();
    runtime.select_next_choice();
    runtime.activate_choice();

    let snapshot = runtime.render_snapshot();
    assert!(snapshot.variables.iter().any(|entry| {
        entry.key == "audit_phase" && entry.value == VisualStateValue::Text("completed".to_string())
    }));
    assert!(snapshot.variables.iter().any(|entry| {
        entry.key == "build_phase" && entry.value == VisualStateValue::Text("completed".to_string())
    }));
    assert!(snapshot.variables.iter().any(|entry| {
        entry.key == "blocked_count" && entry.value == VisualStateValue::Number(1)
    }));
    assert!(snapshot
        .entities
        .iter()
        .any(|entity| entity.id == "agent-audit"
            && entity
                .state_flags
                .iter()
                .any(|flag| flag == "agent_completed")));
    assert!(snapshot
        .entities
        .iter()
        .any(|entity| entity.id == "agent-build"
            && entity
                .state_flags
                .iter()
                .any(|flag| flag == "agent_completed")));
    assert_eq!(snapshot.selected_entity_id.as_deref(), Some("task-review"));

    runtime
        .apply_scene_patch(VisualScenePatch {
            scene_patch_version: VisualScenePatch::VERSION,
            updates: vec![VisualSceneEntityPatch {
                entity_id: "agent-audit".to_string(),
                label: None,
                position: None,
                sprite: None,
                visible: None,
                state_flags: Some(vec!["agent".to_string(), "agent_blocked".to_string()]),
                metadata: Some(vec![
                    ("agent_phase".to_string(), "blocked".to_string()),
                    ("agent_task_id".to_string(), "task-review".to_string()),
                    ("blocked_by".to_string(), "task-build".to_string()),
                ]),
            }],
            variables: vec![VisualStateEntry {
                key: "active_agent_id".to_string(),
                value: VisualStateValue::Text("agent-audit".to_string()),
            }],
            selected_entity_id: Some("agent-audit".to_string()),
            process_state: Some(VisualProcessState {
                entity_id: Some("agent-audit".to_string()),
                phase: VisualProcessPhase::Blocked,
                command: Some("agent:blocked".to_string()),
                exit_code: None,
                message: Some("Waiting for build output".to_string()),
            }),
            dialogue: None,
            status: Some("agent-audit blocked for task-review".to_string()),
        })
        .unwrap();

    let snapshot = runtime.render_snapshot();
    assert!(snapshot
        .entities
        .iter()
        .any(|entity| entity.id == "agent-build"
            && entity
                .state_flags
                .iter()
                .any(|flag| flag == "agent_completed")));
    assert!(snapshot
        .entities
        .iter()
        .any(|entity| entity.id == "agent-audit"
            && entity
                .state_flags
                .iter()
                .any(|flag| flag == "agent_blocked")));
}

#[test]
fn render_snapshot_uses_stage_displayables_when_present() {
    let mut scene = VisualScene::demo();
    scene.stage = VisualStage {
        layers: vec![
            VisualStageLayer {
                layer_id: "characters".to_string(),
                zorder: 10,
                displayables: vec![VisualStageDisplayable {
                    tag: "kiki".to_string(),
                    sprite: "vn.character.kiki.neutral".to_string(),
                    placement: VisualStagePlacement::Center,
                    zorder: 0,
                    visible: true,
                }],
            },
            VisualStageLayer {
                layer_id: "background".to_string(),
                zorder: 0,
                displayables: vec![VisualStageDisplayable {
                    tag: "background".to_string(),
                    sprite: "vn.background.school_classroom".to_string(),
                    placement: VisualStagePlacement::Fullscreen,
                    zorder: 0,
                    visible: true,
                }],
            },
        ],
    };
    let runtime = SceneRuntime::new(scene).unwrap();

    let snapshot = runtime.render_snapshot();
    assert!(snapshot.tiles.is_empty());
    assert_eq!(
        snapshot
            .stage
            .iter()
            .map(|displayable| displayable.tag.as_str())
            .collect::<Vec<_>>(),
        vec!["background", "kiki"]
    );
    assert_eq!(
        snapshot.stage[0].placement,
        VisualStagePlacement::Fullscreen
    );
    assert_eq!(snapshot.stage[1].placement, VisualStagePlacement::Center);
}

#[test]
fn staged_scene_renders_vn_dialogue_box_and_compose_dock() {
    let mut scene = VisualScene::demo();
    scene.stage = VisualStage {
        layers: vec![VisualStageLayer {
            layer_id: "background".to_string(),
            zorder: 0,
            displayables: vec![VisualStageDisplayable {
                tag: "background".to_string(),
                sprite: "vn.background.school_classroom".to_string(),
                placement: VisualStagePlacement::Fullscreen,
                zorder: 0,
                visible: true,
            }],
        }],
    };
    scene.dialogue_speaker = "Codex".to_string();
    scene.dialogue = "This line belongs in the transparent VN overlay.".to_string();
    let mut runtime = SceneRuntime::new(scene).unwrap();
    runtime.mark_compose_running("Compose running", "inspect scene");
    runtime.mark_compose_succeeded("Codex", "This line belongs in the transparent VN overlay.");

    let frame = runtime.render_text_frame(80, 24);
    assert!(!frame.contains("Stage: 1 layer(s), 1 displayable(s)"));
    assert!(!frame.contains("Scene Mode  ["));
    assert!(!frame.contains("+---"));
    assert!(!frame.contains("| Codex:"));
    assert!(frame.contains("Codex"));
    assert!(!frame.contains("Codex:"));
    assert!(frame.contains("transparent VN overlay"));
    assert!(!frame.contains("Compose: _"));
    let lines = frame.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 24);
    assert!(lines.iter().all(|line| line.chars().count() == 80));
}

#[test]
fn vn_overlay_layout_derives_panels_and_nameplates() {
    let layout = vn_overlay_layout(240, 60, "Codex", "Composer");
    let composer = layout.composer_panel.unwrap();
    let composer_nameplate = layout.composer_nameplate.unwrap();

    assert!(layout.fullscreen);
    assert!(composer.col < layout.dialogue_panel.col);
    assert!(composer.width > layout.dialogue_panel.width);
    assert!(layout.dialogue_panel.row < composer.row);
    assert!(layout.dialogue_panel.bottom() < composer.row);
    assert_eq!(
        layout.dialogue_nameplate.col,
        layout.dialogue_panel.col + VN_OVERLAY_DIALOGUE_NAMEPLATE_INSET_COLS
    );
    assert_eq!(
        layout.dialogue_nameplate.height,
        VN_OVERLAY_DIALOGUE_NAMEPLATE_HEIGHT_ROWS.min(layout.dialogue_panel.height.max(1)),
    );
    assert_eq!(
        layout.dialogue_nameplate.row
            + layout.dialogue_nameplate.height
            + VN_OVERLAY_NAMEPLATE_OFFSET_ROWS,
        layout.dialogue_panel.row
    );
    assert_eq!(
        layout.dialogue_nameplate_text.col,
        layout.dialogue_nameplate.col + VN_OVERLAY_DIALOGUE_NAMEPLATE_TEXT_INSET_COLS
    );
    assert_eq!(
        layout.dialogue_nameplate_text.row,
        layout.dialogue_nameplate.row + VN_OVERLAY_DIALOGUE_NAMEPLATE_TEXT_INSET_ROWS
    );
    assert_eq!(
        composer_nameplate.col,
        composer.col + VN_OVERLAY_COMPOSER_NAMEPLATE_INSET_COLS
    );
    assert_eq!(
        composer_nameplate.height,
        VN_OVERLAY_COMPOSER_NAMEPLATE_HEIGHT_ROWS.min(composer.height)
    );
    assert_eq!(
        composer_nameplate.row + composer_nameplate.height + VN_OVERLAY_NAMEPLATE_OFFSET_ROWS,
        composer.row
    );
    assert_eq!(
        layout.composer_nameplate.as_ref().unwrap().height,
        VN_OVERLAY_COMPOSER_NAMEPLATE_HEIGHT_ROWS.min(composer.height)
    );
    assert_eq!(
        layout.composer_nameplate_text.unwrap().col,
        composer_nameplate.col + VN_OVERLAY_COMPOSER_NAMEPLATE_TEXT_INSET_COLS
    );
    assert_eq!(
        layout.composer_nameplate_text.unwrap().row,
        composer_nameplate.row + VN_OVERLAY_COMPOSER_NAMEPLATE_TEXT_INSET_ROWS
    );
    assert!(layout.composer_text_row.unwrap() > composer.row);
    assert!(layout.composer_text_row.unwrap() < composer.bottom());
}

#[test]
fn vn_overlay_layout_adapts_to_windowed_rows() {
    let compact = vn_overlay_layout(120, 24, "Codex", "Composer");
    let large_window = vn_overlay_layout(120, 30, "Codex", "Composer");

    let compact_composer = compact.composer_panel.unwrap();
    let large_composer = large_window.composer_panel.unwrap();

    assert!(!compact.fullscreen);
    assert_eq!(compact_composer.height, 4);
    assert_eq!(large_composer.height, 4);
    assert_eq!(
        compact.composer_text_row,
        Some(compact_composer.row + VN_OVERLAY_COMPOSER_TEXT_INSET_ROWS)
    );
    assert_eq!(
        large_window.composer_text_row,
        Some(large_composer.row + VN_OVERLAY_COMPOSER_TEXT_INSET_ROWS)
    );
    assert_eq!(compact_composer.row, 18);
    assert_eq!(large_composer.row, 24);
}

#[test]
fn vn_overlay_nameplates_do_not_overlap_at_fullscreen_threshold() {
    let layout = vn_overlay_layout(120, VN_OVERLAY_FULLSCREEN_MIN_ROWS, "Codex", "Composer");
    let composer = layout.composer_panel.unwrap();
    let composer_nameplate = layout.composer_nameplate.unwrap();

    assert_eq!(
        layout.dialogue_nameplate.row + layout.dialogue_nameplate.height,
        layout.dialogue_panel.row
    );
    assert_eq!(
        composer_nameplate.row + composer_nameplate.height,
        composer.row
    );
}

#[test]
fn vn_overlay_debug_overrides_preserve_nameplate_separation() {
    let mut overrides = VnOverlayDebugOverrides::default();
    overrides.dialogue_top_ratio = 0.0;
    let layout = vn_overlay_layout_with_overrides(
        120,
        VN_OVERLAY_FULLSCREEN_MIN_ROWS,
        "Codex",
        "Composer",
        &overrides,
    );

    assert_eq!(
        layout.dialogue_nameplate.row + layout.dialogue_nameplate.height,
        layout.dialogue_panel.row
    );
}

#[test]
fn vn_overlay_debug_overrides_separate_nameplate_and_text_positions() {
    let mut overrides = VnOverlayDebugOverrides::default();
    overrides.dialogue_nameplate_height_rows = 4;
    overrides.composer_nameplate_height_rows = 2;
    overrides.dialogue_nameplate_inset_cols = 6;
    overrides.composer_nameplate_inset_cols = 3;
    overrides.dialogue_nameplate_text_inset_cols = 2;
    overrides.composer_nameplate_text_inset_cols = 4;
    overrides.dialogue_nameplate_text_inset_rows = 1;
    overrides.composer_nameplate_text_inset_rows = 0;
    overrides.dialogue_text_inset_cols = 9;
    overrides.composer_text_inset_cols = 5;
    overrides.dialogue_text_inset_rows = 3;
    overrides.composer_text_inset_rows = 2;

    let layout = vn_overlay_layout_with_overrides(160, 60, "Narrator", "Composer", &overrides);
    let composer = layout.composer_panel.unwrap();
    let composer_nameplate = layout.composer_nameplate.unwrap();
    let composer_nameplate_text = layout.composer_nameplate_text.unwrap();

    assert_eq!(layout.dialogue_nameplate.height, 4);
    assert_eq!(composer_nameplate.height, 2);
    assert_eq!(layout.dialogue_nameplate.col, layout.dialogue_panel.col + 6);
    assert_eq!(composer_nameplate.col, composer.col + 3);
    assert_eq!(
        layout.dialogue_nameplate_text.col,
        layout.dialogue_nameplate.col + 2
    );
    assert_eq!(
        layout.dialogue_nameplate_text.row,
        layout.dialogue_nameplate.row + 1
    );
    assert_eq!(composer_nameplate_text.col, composer_nameplate.col + 4);
    assert_eq!(composer_nameplate_text.row, composer_nameplate.row);
    assert_eq!(layout.dialogue_text_inset_cols, 9);
    assert_eq!(layout.composer_text_inset_cols, 5);
    assert_eq!(
        layout.dialogue_text_row,
        layout.dialogue_panel.row + overrides.dialogue_text_inset_rows
    );
    assert_eq!(
        layout.composer_text_row,
        Some(composer.row + overrides.composer_text_inset_rows)
    );
    assert_eq!(
        layout.dialogue_nameplate.row + layout.dialogue_nameplate.height,
        layout.dialogue_panel.row
    );
    assert_eq!(
        composer_nameplate.row + composer_nameplate.height,
        composer.row
    );
}

#[test]
fn vn_layout_debug_overrides_adjust_and_select() {
    let mut overrides = VnOverlayDebugOverrides::default();
    let baseline = overrides.dialogue_margin_ratio;
    overrides.adjust(1);
    assert!(overrides.dialogue_margin_ratio > baseline);
    overrides.adjust(-1);
    assert!((overrides.dialogue_margin_ratio - baseline).abs() < 1e-6);

    overrides.select_next();
    assert_eq!(overrides.selected_param, 1);
    overrides.select_prev();
    assert_eq!(overrides.selected_param, 0);
    overrides.select_prev();
    assert_eq!(
        overrides.selected_param,
        VnOverlayDebugOverrides::PARAM_COUNT - 1
    );
}

#[test]
fn vn_layout_debug_overrides_adjust_opacity_params() {
    let mut overrides = VnOverlayDebugOverrides::default();

    overrides.selected_param = 17;
    overrides.adjust(-1);
    assert!(overrides.dialogue_panel_opacity < VN_OVERLAY_PANEL_OPACITY);

    overrides.selected_param = 18;
    overrides.adjust(1);
    assert!(overrides.composer_panel_opacity > VN_OVERLAY_PANEL_OPACITY);

    overrides.selected_param = 19;
    overrides.adjust(-1);
    assert!(overrides.dialogue_nameplate_opacity < VN_OVERLAY_NAMEPLATE_OPACITY);

    overrides.selected_param = 20;
    overrides.adjust(1);
    assert!(overrides.composer_nameplate_opacity > VN_OVERLAY_NAMEPLATE_OPACITY);
}

#[test]
fn vn_layout_debug_overrides_text_edit_commits_typed_value() {
    let mut overrides = VnOverlayDebugOverrides::default();
    overrides.begin_edit();
    assert!(overrides.editing_buffer.is_some());
    // Clear the prefilled value, then type a new one.
    for _ in 0..16 {
        overrides.pop_char();
    }
    for c in "0.250".chars() {
        overrides.push_char(c);
    }
    // Non-numeric input is rejected.
    overrides.push_char('x');
    overrides.commit_edit();
    assert!(overrides.editing_buffer.is_none());
    assert!((overrides.dialogue_margin_ratio - 0.250).abs() < 1e-6);
}

#[test]
fn vn_layout_debug_overrides_text_edit_cancel_keeps_value() {
    let mut overrides = VnOverlayDebugOverrides::default();
    let baseline = overrides.dialogue_margin_ratio;
    overrides.begin_edit();
    for _ in 0..16 {
        overrides.pop_char();
    }
    for c in "0.400".chars() {
        overrides.push_char(c);
    }
    overrides.cancel_edit();
    assert!(overrides.editing_buffer.is_none());
    assert!((overrides.dialogue_margin_ratio - baseline).abs() < 1e-6);
}

#[test]
fn staged_scene_view_hides_debug_chrome() {
    let mut scene = VisualScene::demo();
    scene.dialogue = "Normal scene line.".to_string();
    scene.dialogue_speaker = "Narrator".to_string();
    scene.dialogue_lines.clear();
    scene.choices.clear();
    scene.stage = VisualStage {
        layers: vec![VisualStageLayer {
            layer_id: "background".to_string(),
            zorder: 0,
            displayables: vec![VisualStageDisplayable {
                tag: "background".to_string(),
                sprite: "vn.background.school_classroom".to_string(),
                placement: VisualStagePlacement::Fullscreen,
                zorder: 0,
                visible: true,
            }],
        }],
    };
    let mut runtime = SceneRuntime::new(scene).unwrap();
    runtime.mark_compose_running("Compose running", "hello");
    runtime.mark_compose_succeeded("Codex", "Normal scene line.");

    let frame = runtime.render_text_frame(160, 48);

    assert!(!frame.contains("Scene Mode  ["));
    assert!(!frame.contains("Selected:"));
    assert!(!frame.contains("Stage:"));
    assert!(frame.contains("Normal scene line."));
    assert!(frame.contains("Codex"));
}

#[test]
fn staged_scene_view_fills_blank_rows_to_clear_stale_text() {
    let mut scene = VisualScene::demo();
    scene.dialogue = "Normal scene line.".to_string();
    scene.dialogue_speaker = "Narrator".to_string();
    scene.dialogue_lines.clear();
    scene.choices.clear();
    scene.stage = VisualStage {
        layers: vec![VisualStageLayer {
            layer_id: "background".to_string(),
            zorder: 0,
            displayables: vec![VisualStageDisplayable {
                tag: "background".to_string(),
                sprite: "vn.background.school_classroom".to_string(),
                placement: VisualStagePlacement::Fullscreen,
                zorder: 0,
                visible: true,
            }],
        }],
    };
    let runtime = SceneRuntime::new(scene).unwrap();

    let frame = runtime.render_text_frame(24, 12);
    let lines = frame.lines().collect::<Vec<_>>();

    assert_eq!(lines[0].chars().count(), 24);
    assert!(lines[0].chars().all(|ch| ch == ' '));
}

#[test]
fn interactive_debugger_separates_scene_and_tile_menus() {
    let mut scene = VisualScene::demo();
    scene.dialogue = "Live tuning line.".to_string();
    scene.dialogue_speaker = "Narrator".to_string();
    scene.dialogue_lines.clear();
    scene.choices.clear();
    scene.stage = VisualStage {
        layers: vec![VisualStageLayer {
            layer_id: "background".to_string(),
            zorder: 0,
            displayables: vec![VisualStageDisplayable {
                tag: "background".to_string(),
                sprite: "vn.background.school_classroom".to_string(),
                placement: VisualStagePlacement::Fullscreen,
                zorder: 0,
                visible: true,
            }],
        }],
    };
    let mut runtime = SceneRuntime::new(scene).unwrap();
    runtime.mark_compose_running("Compose running", "debug layout");
    runtime.mark_compose_succeeded("Codex", "Live tuning line.");
    runtime.toggle_debugger();
    assert_eq!(runtime.view(), VisualView::VnLayoutDebugger);
    assert_eq!(
        runtime.interactive_debug_menu(),
        VisualInteractiveDebugMenu::SceneLayout
    );

    let frame = runtime.render_text_frame(200, 60);
    assert!(frame.contains("Debug 2"));
    assert!(frame.contains("> Sections [Scene Layout]"));
    assert!(frame.contains("dialogue_margin_ratio"));
    assert!(frame.contains("Voice"));
    assert!(frame.contains("Compose"));
    assert!(!frame.contains("Tile Debug Menu"));
    assert!(!frame.contains("Entities:"));
    assert!(!frame.contains("Live tuning line."));

    runtime.handle_input(VisualInput::Left);
    assert_eq!(
        runtime.interactive_debug_menu(),
        VisualInteractiveDebugMenu::Runtime
    );
    let frame = runtime.render_text_frame(200, 60);
    assert!(!frame.contains("Tile Debug Menu"));
    assert!(!frame.contains("Entities:"));
    runtime.handle_input(VisualInput::Right);
    assert_eq!(
        runtime.interactive_debug_menu(),
        VisualInteractiveDebugMenu::SceneLayout
    );

    runtime.handle_input(VisualInput::Activate);
    assert!(runtime
        .vn_layout_debug
        .as_ref()
        .and_then(|debug| debug.editing_buffer.as_ref())
        .is_none());
    runtime.handle_input(VisualInput::Next);
    runtime.handle_input(VisualInput::Activate);
    assert!(runtime
        .vn_layout_debug
        .as_ref()
        .and_then(|debug| debug.editing_buffer.as_ref())
        .is_some());
}

#[test]
fn vn_layout_debug_overrides_edit_clamps_out_of_range() {
    let mut overrides = VnOverlayDebugOverrides::default();
    overrides.begin_edit();
    for _ in 0..16 {
        overrides.pop_char();
    }
    for c in "9.999".chars() {
        overrides.push_char(c);
    }
    overrides.commit_edit();
    assert!(overrides.dialogue_margin_ratio <= 0.45);
}

#[test]
fn vn_layout_debug_opacity_edit_clamps_to_unit_interval() {
    let mut overrides = VnOverlayDebugOverrides {
        selected_param: 17,
        ..VnOverlayDebugOverrides::default()
    };
    overrides.begin_edit();
    for _ in 0..16 {
        overrides.pop_char();
    }
    for c in "9.999".chars() {
        overrides.push_char(c);
    }
    overrides.commit_edit();
    assert_eq!(overrides.dialogue_panel_opacity, 1.0);

    overrides.selected_param = 20;
    overrides.begin_edit();
    for _ in 0..16 {
        overrides.pop_char();
    }
    for c in "-1.000".chars() {
        overrides.push_char(c);
    }
    overrides.commit_edit();
    assert_eq!(overrides.composer_nameplate_opacity, 0.0);
}

#[test]
fn staged_scene_renders_dialogue_text_on_layout_row() {
    let mut scene = VisualScene::demo();
    scene.dialogue = "This is the active layout row.".to_string();
    scene.dialogue_speaker = "Narrator".to_string();
    scene.dialogue_lines.clear();
    scene.choices.clear();
    let runtime = SceneRuntime::new(scene).unwrap();
    let cols = 90;
    let rows = 30;
    let layout = vn_overlay_layout(cols, rows, "Narrator", "Composer");
    let frame = runtime.render_text_frame(cols, rows);
    let lines: Vec<&str> = frame.split("\r\n").collect();
    assert!(!lines.is_empty());
    let layout_text_row = layout.dialogue_text_row.min(lines.len().saturating_sub(1));
    let expected = "This is the active layout row.";
    let matched_row = lines.iter().position(|line| line.contains(expected));
    let matched_row = match matched_row {
        Some(row) => row,
        None => panic!("expected dialogue text in rendered frame"),
    };
    assert!(
        matched_row == layout_text_row || matched_row + 1 == layout_text_row,
        "expected dialogue text at layout row {} or {} but found {}",
        layout_text_row,
        layout_text_row.saturating_sub(1),
        matched_row
    );
    assert_ne!(layout.dialogue_text_row, 0);
}

#[test]
fn scene_rejects_invalid_stage_fields() {
    let mut empty_layer = VisualScene::demo();
    empty_layer.stage = VisualStage {
        layers: vec![VisualStageLayer {
            layer_id: " ".to_string(),
            zorder: 0,
            displayables: Vec::new(),
        }],
    };
    assert!(matches!(
        empty_layer.validate(),
        Err(VisualSceneError::EmptyStageLayerId)
    ));

    let mut empty_sprite = VisualScene::demo();
    empty_sprite.stage = VisualStage {
        layers: vec![VisualStageLayer {
            layer_id: "characters".to_string(),
            zorder: 0,
            displayables: vec![VisualStageDisplayable {
                tag: "kiki".to_string(),
                sprite: " ".to_string(),
                placement: VisualStagePlacement::Center,
                zorder: 0,
                visible: true,
            }],
        }],
    };
    assert!(matches!(
        empty_sprite.validate(),
        Err(VisualSceneError::EmptyStageDisplayableSprite { tag }) if tag == "kiki"
    ));
}

#[test]
fn scene_fixture_game_states_covers_common_modes() {
    let scene = VisualScene::load_from_path(scene_fixture_path("game-states.json")).unwrap();
    let mut runtime = SceneRuntime::new(scene).unwrap();

    assert_eq!(runtime.render_snapshot().active_mode.mode_id, "gameplay");
    assert_eq!(runtime.render_snapshot().active_layers.len(), 6);
    assert_eq!(runtime.render_snapshot().status, "Entered gameplay state");

    runtime.activate_choice();
    runtime.select_next_choice();
    runtime.activate_choice();
    runtime.select_next_choice();
    runtime.activate_choice();
    runtime.select_next_choice();
    runtime.activate_choice();

    let snapshot = runtime.render_snapshot();
    assert!(snapshot.variables.iter().any(|entry| {
        entry.key == "intro_complete" && entry.value == VisualStateValue::Bool(true)
    }));
    assert!(snapshot.variables.iter().any(|entry| {
        entry.key == "agent_phase" && entry.value == VisualStateValue::Text("running".to_string())
    }));
    assert!(snapshot
        .active_layers
        .iter()
        .any(|layer| layer.layer_id == "story" && layer.state == "exploration"));
    assert!(snapshot
        .active_layers
        .iter()
        .any(|layer| layer.layer_id == "ui" && layer.state == "inventory"));
    assert!(snapshot
        .active_layers
        .iter()
        .any(|layer| layer.layer_id == "combat" && layer.state == "command"));
    assert!(snapshot
        .active_layers
        .iter()
        .any(|layer| layer.layer_id == "agent" && layer.state == "running"));
    assert_eq!(snapshot.rpg.quests[0].stage, 2);
    assert_eq!(snapshot.rpg.stats[0].value, VisualStateValue::Number(9));
}

#[test]
fn scene_fixture_chained_transitions_completes_state_chain() {
    let scene =
        VisualScene::load_from_path(scene_fixture_path("chained-transitions.json")).unwrap();
    let mut runtime = SceneRuntime::new(scene).unwrap();

    runtime.activate_choice();
    runtime.select_next_choice();
    runtime.activate_choice();
    runtime.select_next_choice();
    runtime.activate_choice();
    runtime.select_next_choice();
    runtime.activate_choice();

    let snapshot = runtime.render_snapshot();
    assert!(snapshot.variables.iter().any(|entry| {
        entry.key == "agent_phase" && entry.value == VisualStateValue::Text("running".to_string())
    }));
    assert!(snapshot
        .active_layers
        .iter()
        .any(|layer| layer.layer_id == "story" && layer.state == "exploration"));
    assert!(snapshot
        .active_layers
        .iter()
        .any(|layer| layer.layer_id == "ui" && layer.state == "inventory"));
    assert!(snapshot
        .active_layers
        .iter()
        .any(|layer| layer.layer_id == "quest" && layer.state == "route-open"));
    assert!(snapshot
        .active_layers
        .iter()
        .any(|layer| layer.layer_id == "command" && layer.state == "issued"));
    assert!(snapshot
        .active_layers
        .iter()
        .any(|layer| layer.layer_id == "agent" && layer.state == "running"));
    assert_eq!(snapshot.rpg.quests[0].stage, 2);
    assert_eq!(snapshot.rpg.stats[0].value, VisualStateValue::Number(9));
}

#[test]
fn scene_fixture_invalid_is_rejected() {
    assert!(matches!(
        VisualScene::load_from_path(scene_fixture_path("invalid.json")),
        Err(VisualSceneError::EmptyScene)
    ));
}

#[test]
fn scene_fixture_sprite_manifest_resolves_relative_paths() {
    let manifest_path = scene_fixture_path("sprites.json");
    let manifest = VisualSpriteManifest::load_from_path(&manifest_path).unwrap();
    let status = manifest.resolve_against(&manifest_path);

    assert!(status.sprites.iter().any(|sprite| {
        sprite.id == "project_core"
            && sprite
                .path
                .ends_with("assets/gameterm-scene/project-core.png")
    }));
    assert!(status.warnings.is_empty());
}

#[test]
fn scene_fixture_missing_sprite_manifest_keeps_valid_entries() {
    let manifest_path = scene_fixture_path("sprites-missing.json");
    let manifest = VisualSpriteManifest::load_from_path(&manifest_path).unwrap();
    let status = manifest.resolve_against(&manifest_path);

    assert_eq!(status.sprites.len(), 2);
    assert!(status
        .sprites
        .iter()
        .any(|sprite| sprite.id == "workspace-map"));
    assert!(status
        .sprites
        .iter()
        .any(|sprite| sprite.path.ends_with("sprites/missing-project-core.png")));
}

#[test]
fn duplicate_entity_ids_are_rejected() {
    let mut scene = VisualScene::demo();
    scene.entities[1].id = scene.entities[0].id.clone();
    assert!(matches!(
        scene.validate(),
        Err(VisualSceneError::DuplicateEntityId(_))
    ));
}

#[test]
fn runtime_toggles_debugger() {
    let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
    assert_eq!(runtime.view(), VisualView::Scene);
    runtime.toggle_debugger();
    assert_eq!(runtime.view(), VisualView::VnLayoutDebugger);
    runtime.toggle_debugger();
    assert_eq!(runtime.view(), VisualView::Scene);
}

#[test]
fn mode_toggle_debug_input_changes_view_and_generation() {
    let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
    let initial_generation = runtime.generation();

    // Tab in a scene now opens the Tab-cycle mode screens (starting at
    // Character Select). The layout debugger is reached via Main Menu ->
    // Settings, and `toggle_debugger()` still opens it directly.
    let outcome = runtime.handle_input(VisualInput::ToggleDebug);

    assert_eq!(outcome, VisualModeOutcome::Continue);
    assert_eq!(runtime.view(), VisualView::CharacterSelect);
    assert!(runtime.generation() > initial_generation);

    runtime.toggle_debugger();
    assert_eq!(runtime.view(), VisualView::VnLayoutDebugger);
}

#[test]
fn mode_input_map_can_enter_and_exit_command_selection_view() {
    let mut scene = VisualScene::demo();
    scene.mode.input_map = vec![
        VisualInputBinding {
            input: "other".to_string(),
            action: "toggle_command_selection".to_string(),
            conditions: Vec::new(),
        },
        VisualInputBinding {
            input: "reload".to_string(),
            action: "hide_command_selection".to_string(),
            conditions: Vec::new(),
        },
    ];
    let mut runtime = SceneRuntime::new(scene).unwrap();

    let outcome = runtime.handle_input(VisualInput::Other);

    assert_eq!(outcome, VisualModeOutcome::Continue);
    assert_eq!(runtime.view(), VisualView::CommandSelection);

    let outcome = runtime.handle_input(VisualInput::Reload);

    assert_eq!(outcome, VisualModeOutcome::Continue);
    assert_eq!(runtime.view(), VisualView::Scene);
}

#[test]
fn mode_next_input_advances_selection() {
    let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
    let initial_generation = runtime.generation();

    let outcome = runtime.handle_input(VisualInput::Next);

    assert_eq!(outcome, VisualModeOutcome::Continue);
    assert!(runtime.generation() > initial_generation);
    assert_eq!(
        runtime.render_snapshot().selected_entity_id.as_deref(),
        Some("task-render")
    );
    assert_eq!(runtime.render_snapshot().selected_choice, 1);
}

#[test]
fn mode_previous_input_wraps_selection() {
    let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();

    let outcome = runtime.handle_input(VisualInput::Previous);

    assert_eq!(outcome, VisualModeOutcome::Continue);
    assert_eq!(
        runtime.render_snapshot().selected_entity_id.as_deref(),
        Some("agent-audit")
    );
    assert_eq!(runtime.render_snapshot().selected_choice, 2);
}

#[test]
fn mode_activate_input_updates_status() {
    let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();

    let outcome = runtime.handle_input(VisualInput::Activate);

    assert_eq!(outcome, VisualModeOutcome::Continue);
    assert_eq!(
        runtime.render_snapshot().status,
        "Inspecting GameTerm (project-gameterm)"
    );
}

#[test]
fn mode_input_map_remaps_input_action() {
    let mut scene = VisualScene::demo();
    scene.mode.input_map = vec![VisualInputBinding {
        input: "next".to_string(),
        action: "activate_choice".to_string(),
        conditions: Vec::new(),
    }];
    let mut runtime = SceneRuntime::new(scene).unwrap();

    let outcome = runtime.handle_input(VisualInput::Next);

    assert_eq!(outcome, VisualModeOutcome::Continue);
    assert_eq!(
        runtime.render_snapshot().status,
        "Inspecting GameTerm (project-gameterm)"
    );
    assert_eq!(
        runtime.render_snapshot().selected_entity_id.as_deref(),
        Some("project-gameterm")
    );
}

#[test]
fn guarded_mode_input_map_blocks_action_when_variable_mismatches() {
    let mut scene = VisualScene::demo();
    scene.mode.input_map = vec![VisualInputBinding {
        input: "other".to_string(),
        action: "toggle_debug".to_string(),
        conditions: vec![VisualCondition {
            source: None,
            variable: "active_track".to_string(),
            equals: VisualStateValue::Text("memory".to_string()),
        }],
    }];
    let mut runtime = SceneRuntime::new(scene).unwrap();

    let outcome = runtime.handle_input(VisualInput::Other);

    assert_eq!(outcome, VisualModeOutcome::Continue);
    assert_eq!(runtime.view(), VisualView::Scene);
    assert_eq!(
        runtime.render_snapshot().status,
        "Input unavailable: requires active_track=memory"
    );
}

#[test]
fn mode_input_map_is_visible_in_debugger() {
    let mut scene = VisualScene::demo();
    scene.mode.input_map = vec![VisualInputBinding {
        input: "other".to_string(),
        action: "run_update_hooks".to_string(),
        conditions: Vec::new(),
    }];
    scene.mode.lifecycle.update_status = Some("Polled mode".to_string());
    let mut runtime = SceneRuntime::new(scene).unwrap();

    let outcome = runtime.handle_input(VisualInput::Other);

    assert_eq!(outcome, VisualModeOutcome::Continue);
    assert_eq!(runtime.render_snapshot().status, "Polled mode");
    assert_eq!(
        runtime.debug_report().active_mode_input_map[0].action,
        "run_update_hooks"
    );

    let frame = runtime.render_debugger(200, 80);
    assert!(frame.contains("Input map:"));
    assert!(frame.contains("other -> run_update_hooks"));
}

#[test]
fn scene_rejects_empty_mode_input_map_fields() {
    let mut scene = VisualScene::demo();
    scene.mode.input_map = vec![VisualInputBinding {
        input: " ".to_string(),
        action: "ignore".to_string(),
        conditions: Vec::new(),
    }];

    assert_eq!(
        scene.validate(),
        Err(VisualSceneError::EmptyModeInputBindingInput)
    );

    scene.mode.input_map[0].input = "other".to_string();
    scene.mode.input_map[0].action = " ".to_string();

    assert_eq!(
        scene.validate(),
        Err(VisualSceneError::EmptyModeInputBindingAction)
    );
}

#[test]
fn scene_rejects_unknown_mode_input_map_values() {
    let mut scene = VisualScene::demo();
    scene.mode.input_map = vec![VisualInputBinding {
        input: "space".to_string(),
        action: "ignore".to_string(),
        conditions: Vec::new(),
    }];

    assert_eq!(
        scene.validate(),
        Err(VisualSceneError::UnknownModeInputBindingInput(
            "space".to_string()
        ))
    );

    scene.mode.input_map[0].input = "other".to_string();
    scene.mode.input_map[0].action = "jump".to_string();

    assert_eq!(
        scene.validate(),
        Err(VisualSceneError::UnknownModeInputBindingAction(
            "jump".to_string()
        ))
    );
}

#[test]
fn layered_state_defaults_empty_for_existing_scenes() {
    let scene = VisualScene::from_json(
        r#"{
              "title": "Legacy",
              "background": "floor",
              "width": 2,
              "height": 2,
              "entities": [],
              "dialogue_speaker": "System",
              "dialogue": "Ready",
              "choices": []
            }"#,
    )
    .unwrap();
    let runtime = SceneRuntime::new(scene).unwrap();

    assert!(runtime.render_snapshot().active_layers.is_empty());
    assert!(runtime.debug_report().active_layers.is_empty());
}

#[test]
fn layered_state_transition_updates_layer_and_debug_report() {
    let mut scene = VisualScene::demo();
    scene.layers = vec![VisualLayerState {
        layer_id: "story".to_string(),
        state: "dialogue".to_string(),
        label: Some("Story".to_string()),
        input_map: Vec::new(),
        transitions: vec![VisualLayerTransition {
            input: "activate".to_string(),
            target_state: "choice".to_string(),
            conditions: Vec::new(),
        }],
    }];
    let mut runtime = SceneRuntime::new(scene).unwrap();

    let outcome = runtime.handle_input(VisualInput::Activate);

    assert_eq!(outcome, VisualModeOutcome::Continue);
    assert_eq!(runtime.render_snapshot().active_layers[0].state, "choice");
    assert_eq!(
        runtime.debug_report().last_input_layer.as_deref(),
        Some("story")
    );
    assert_eq!(
        runtime
            .debug_report()
            .last_layer_transition
            .as_ref()
            .map(|transition| transition.result.as_str()),
        Some("transitioned")
    );

    let frame = runtime.render_debugger(120, 40);
    assert!(frame.contains("Layers:"));
    assert!(frame.contains("story state=choice label=Story"));
    assert!(frame.contains("Last transition: story activate dialogue -> choice (transitioned)"));
}

#[test]
fn guarded_layer_transition_fails_without_mutation() {
    let mut scene = VisualScene::demo();
    scene.layers = vec![VisualLayerState {
        layer_id: "agent".to_string(),
        state: "idle".to_string(),
        label: None,
        input_map: Vec::new(),
        transitions: vec![VisualLayerTransition {
            input: "other".to_string(),
            target_state: "running".to_string(),
            conditions: vec![VisualCondition {
                source: None,
                variable: "active_track".to_string(),
                equals: VisualStateValue::Text("agent".to_string()),
            }],
        }],
    }];
    let mut runtime = SceneRuntime::new(scene).unwrap();

    let outcome = runtime.handle_input(VisualInput::Other);

    assert_eq!(outcome, VisualModeOutcome::Continue);
    assert_eq!(runtime.render_snapshot().active_layers[0].state, "idle");
    assert_eq!(
        runtime.render_snapshot().status,
        "Layer transition unavailable: agent requires active_track=agent"
    );
    assert_eq!(
        runtime
            .debug_report()
            .last_layer_transition
            .as_ref()
            .map(|transition| transition.result.as_str()),
        Some("guard_failed")
    );
}

#[test]
fn transition_history_records_recent_runtime_events() {
    let mut scene = VisualScene::demo();
    scene.layers = vec![VisualLayerState {
        layer_id: "story".to_string(),
        state: "dialogue".to_string(),
        label: Some("Story".to_string()),
        input_map: Vec::new(),
        transitions: vec![VisualLayerTransition {
            input: "activate".to_string(),
            target_state: "choice".to_string(),
            conditions: Vec::new(),
        }],
    }];
    let mut runtime = SceneRuntime::new(scene).unwrap();

    runtime.handle_input(VisualInput::Activate);
    runtime.mark_scene_patch_failed("mux", Some(7), "bad patch");

    let report = runtime.debug_report();
    assert!(report
        .transition_history
        .iter()
        .any(|event| { event.kind == "transition" && event.detail == "story dialogue -> choice" }));
    assert!(report
        .transition_history
        .iter()
        .any(|event| { event.kind == "patch" && event.detail == "mux failed: bad patch" }));

    let frame = runtime.render_debugger(120, 40);
    assert!(frame.contains("History:"));
    assert!(frame.contains("transition: story dialogue -> choice"));
}

fn staged_compose_scene() -> VisualScene {
    let mut scene = VisualScene::demo();
    scene.dialogue_speaker = "Narrator".to_string();
    scene.dialogue = "placeholder narrator line".to_string();
    scene.dialogue_lines = vec![VisualDialogueLine {
        speaker: "Narrator".to_string(),
        text: "placeholder narrator line".to_string(),
        portrait: None,
        metadata: Vec::new(),
    }];
    scene.choices = vec![
        SceneAction {
            label: "Ask about Scene Mode.".to_string(),
            kind: SceneActionKind::Inspect,
            policy: None,
            conditions: Vec::new(),
        },
        SceneAction {
            label: "End the demo.".to_string(),
            kind: SceneActionKind::Inspect,
            policy: None,
            conditions: Vec::new(),
        },
    ];
    scene.stage = VisualStage {
        layers: vec![VisualStageLayer {
            layer_id: "background".to_string(),
            zorder: 0,
            displayables: vec![VisualStageDisplayable {
                tag: "background".to_string(),
                sprite: "vn.background.school_classroom".to_string(),
                placement: VisualStagePlacement::Fullscreen,
                zorder: 0,
                visible: true,
            }],
        }],
    };
    scene
}

#[test]
fn compose_runtime_records_turn_status_and_history() {
    let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();

    runtime.mark_compose_running("Compose running: inspect roadmap", "inspect roadmap");
    assert_eq!(
        runtime.compose_state.last_prompt.as_deref(),
        Some("inspect roadmap")
    );
    assert_eq!(runtime.compose_state.phase, VisualComposePhase::Running);

    runtime.mark_compose_succeeded("Codex", "I can inspect the route.");
    assert_eq!(runtime.compose_state.phase, VisualComposePhase::Succeeded);
    assert_eq!(
        runtime.compose_state.last_reply.as_deref(),
        Some("I can inspect the route.")
    );
    assert_eq!(runtime.compose_state.history.len(), 2);
    assert_eq!(
        runtime.compose_state.history[0].role,
        VisualComposeRole::User
    );
    assert_eq!(
        runtime.compose_state.history[0].text,
        "inspect roadmap".to_string()
    );
    assert_eq!(
        runtime.compose_state.history[1].role,
        VisualComposeRole::Assistant
    );
    assert_eq!(runtime.compose_state.history[0].turn_id, 1);
    assert_eq!(runtime.compose_state.history[1].turn_id, 1);
    assert_eq!(runtime.compose_state.history[0].block_index, 0);
    assert_eq!(runtime.compose_state.history[1].block_index, 1);

    runtime.mark_compose_failed("backend offline");
    assert_eq!(runtime.compose_state.phase, VisualComposePhase::Failed);
    assert_eq!(
        runtime.compose_state.history[2].role,
        VisualComposeRole::Error
    );

    let report = runtime.debug_report();
    assert!(report
        .transition_history
        .iter()
        .any(|event| event.kind == "compose" && event.detail.contains("submit: inspect roadmap")));
}

#[test]
fn staged_scene_renders_compose_prompt_above_reply() {
    let mut runtime = SceneRuntime::new(staged_compose_scene()).unwrap();

    runtime.mark_compose_running("Compose running: say hi", "say hi");
    let frame = runtime.render_text_frame(100, 30);

    assert!(frame.contains("> say hi"));
    assert!(!frame.contains("last: say hi"));

    runtime.mark_compose_succeeded("Codex", "Hello from Scene Mode.");
    let frame = runtime.render_text_frame(100, 30);
    let prompt_idx = frame.find("> say hi").unwrap();
    let reply_idx = frame.find("Hello from Scene Mode.").unwrap();

    assert!(prompt_idx < reply_idx);
}

#[test]
fn staged_scene_formats_structured_compose_json_as_dialogue_text() {
    let mut runtime = SceneRuntime::new(staged_compose_scene()).unwrap();

    runtime.mark_compose_running("Compose running", "summarize");
    runtime.mark_compose_succeeded(
        "Codex",
        r#"{"speaker":"Codex","text":"Here is the readable answer.","status":"ok"}"#,
    );
    let frame = runtime.render_text_frame(100, 30);

    assert!(frame.contains("Here is the readable answer."));
    assert!(!frame.contains("\"speaker\""));
    assert!(!frame.contains("\"status\""));
}

#[test]
fn staged_scene_compose_blocks_fake_stream_until_voice_or_tick_reveals() {
    let mut runtime = SceneRuntime::new(staged_compose_scene()).unwrap();

    runtime.mark_compose_running("Compose running", "plan");
    let block_ids = runtime.mark_compose_succeeded_blocks(
        "Codex",
        &[
            "First reply block has enough words to reveal slowly.".to_string(),
            "Second reply block waits for its own voice turn.".to_string(),
        ],
        false,
    );
    assert_eq!(block_ids.len(), 2);

    let frame = runtime.render_text_frame(100, 30);
    assert!(frame.contains("> plan"));
    assert!(!frame.contains("First reply block"));
    assert!(!frame.contains("Second reply block"));

    assert!(runtime.advance_compose_reveal(12));
    let frame = runtime.render_text_frame(100, 30);
    assert!(frame.contains("First reply"));
    assert!(!frame.contains("reveal slowly."));
    assert!(!frame.contains("Second reply block"));

    assert!(runtime.advance_compose_reveal(12));
    let frame = runtime.render_text_frame(100, 30);
    assert!(frame.contains("First reply block"));
    assert!(!frame.contains("Second reply block"));

    runtime.mark_compose_block_done(block_ids[0].0, block_ids[0].1);
    runtime.mark_compose_block_speaking(block_ids[1].0, block_ids[1].1);
    let frame = runtime.render_text_frame(100, 30);
    assert!(frame.contains("First reply block has enough words to reveal slowly."));
    assert!(frame.contains("Second reply block"));
}

#[test]
fn staged_scene_ignores_stale_voice_events_without_hiding_text() {
    let mut runtime = SceneRuntime::new(staged_compose_scene()).unwrap();

    runtime.mark_compose_running("Compose running", "plan");
    let block_ids = runtime.mark_compose_succeeded_blocks(
        "Codex",
        &[
            "Visible reply block one.".to_string(),
            "Visible reply block two.".to_string(),
        ],
        false,
    );
    assert_eq!(block_ids.len(), 2);

    runtime.mark_compose_block_speaking(999, 0);
    runtime.mark_compose_block_done(block_ids[0].0, 99);

    let frame = runtime.render_text_frame(100, 30);
    assert!(!frame.contains("Visible reply block one."));
    assert!(!frame.contains("Visible reply block two."));
    assert!(runtime.advance_compose_reveal(usize::MAX));
    assert!(runtime.advance_compose_reveal(usize::MAX));
    let frame = runtime.render_text_frame(100, 30);
    assert!(frame.contains("Visible reply block one."));
    assert!(frame.contains("Visible reply block two."));

    runtime.mark_compose_running("Compose running", "next prompt");
    let frame = runtime.render_text_frame(100, 30);
    assert!(frame.contains("Visible reply block one."));
    assert!(frame.contains("Visible reply block two."));
    assert!(frame.contains("> next prompt"));

    let report = runtime.debug_report();
    assert!(!report
        .transition_history
        .iter()
        .any(|event| event.kind == "compose" && event.detail.contains("turn=999")));
    assert!(!report
        .transition_history
        .iter()
        .any(|event| event.kind == "compose" && event.detail.contains("block=99")));
}

#[test]
fn staged_scene_future_turns_render_while_previous_voice_blocks_are_unfinished() {
    let mut runtime = SceneRuntime::new(staged_compose_scene()).unwrap();

    runtime.mark_compose_running("Compose running", "first");
    let first_ids = runtime.mark_compose_succeeded_blocks(
        "Codex",
        &["First turn reply waiting on voice.".to_string()],
        false,
    );
    assert_eq!(first_ids.len(), 1);

    let mut previous_turn_id = first_ids[0].0;
    for idx in 2..=5 {
        runtime.mark_compose_running("Compose running", &format!("future prompt {idx}"));
        let ids = runtime.mark_compose_succeeded_blocks(
            "Codex",
            &[format!("Future turn {idx} reply should still render.")],
            false,
        );
        assert_eq!(ids.len(), 1);
        assert_ne!(previous_turn_id, ids[0].0);
        previous_turn_id = ids[0].0;
    }

    let frame = runtime.render_text_frame(100, 30);
    assert!(frame.contains("> future prompt 5"));
    assert!(runtime.advance_compose_reveal(usize::MAX));
    let frame = runtime.render_text_frame(100, 30);
    assert!(frame.contains("Future turn 5 reply should still render."));
}

#[test]
fn staged_scene_splits_flattened_numbered_reply_sections() {
    let mut runtime = SceneRuntime::new(staged_compose_scene()).unwrap();

    runtime.mark_compose_running("Compose running", "plan");
    runtime.mark_compose_succeeded(
            "Codex",
            "The plan is simple. **1. First Step** Do the first thing carefully. **2. Second Step** Then verify the output.",
        );
    let frame = runtime.render_text_frame(100, 30);
    let first_idx = frame.find("1. First Step").unwrap();
    let body_idx = frame.find("Do the first thing carefully.").unwrap();
    let second_idx = frame.find("2. Second Step").unwrap();

    assert!(first_idx < body_idx);
    assert!(body_idx < second_idx);
    assert!(!frame.contains("**"));
}

#[test]
fn staged_scene_uses_compose_speaker_for_nameplate_and_can_clear_history() {
    let mut runtime = SceneRuntime::new(staged_compose_scene()).unwrap();

    runtime.mark_compose_running("Fake Codex running: say hi", "say hi");
    runtime.mark_compose_succeeded("Fake Codex", "Fake reply.");
    let frame = runtime.render_text_frame(100, 30);

    assert!(frame.contains("Fake Codex"));
    assert!(frame.contains("Fake reply."));
    assert!(!frame.contains("Narrator"));

    runtime.clear_compose_history();
    let frame = runtime.render_text_frame(100, 30);

    assert!(!frame.contains("Fake reply."));
    assert!(!frame.contains("> say hi"));
}

#[test]
fn staged_scene_keeps_empty_compose_dialogue_clean() {
    let runtime = SceneRuntime::new(staged_compose_scene()).unwrap();
    let frame = runtime.render_text_frame(100, 30);

    assert!(!frame.contains("Narrator"));
    assert!(!frame.contains("placeholder narrator line"));
    assert!(!frame.contains("Ask about Scene Mode."));
    assert!(!frame.contains("End the demo."));
}

#[test]
fn staged_scene_voice_hold_indicator_renders_off_label_only_when_idle() {
    let runtime = SceneRuntime::new(staged_compose_scene()).unwrap();
    let idle = runtime.render_text_frame_with_dialogue_scroll_and_voice_hold(100, 30, 0, false);
    let active = runtime.render_text_frame_with_dialogue_scroll_and_voice_hold(100, 30, 0, true);
    let layout = vn_overlay_layout(100, 30, "Narrator", "Composer");
    let row = idle
        .lines()
        .nth(layout.voice_hold_indicator_text.row)
        .unwrap();

    assert!(row.contains("[off]"));
    assert!(!active.contains("[off]"));
}

#[test]
fn staged_scene_compose_transcript_scrolls_to_latest_lines() {
    let mut runtime = SceneRuntime::new(staged_compose_scene()).unwrap();

    for idx in 0..8 {
        runtime.mark_compose_running("Compose running", &format!("old prompt {idx}"));
        runtime.mark_compose_succeeded(
            "Codex",
            &format!(
                "old reply {idx} {}",
                "with enough words to wrap inside the dialogue box ".repeat(2)
            ),
        );
    }
    runtime.mark_compose_running("Compose running", "latest prompt");
    runtime.mark_compose_succeeded("Codex", "latest reply should remain visible");

    let frame = runtime.render_text_frame(80, 24);

    assert!(!frame.contains("old prompt 0"));
    assert!(frame.contains("> latest prompt"));
    assert!(frame.contains("latest reply should remain visible"));
}

#[test]
fn staged_scene_compose_transcript_scroll_offset_shows_older_lines() {
    let mut runtime = SceneRuntime::new(staged_compose_scene()).unwrap();

    for idx in 0..8 {
        runtime.mark_compose_running("Compose running", &format!("old prompt {idx}"));
        runtime.mark_compose_succeeded(
            "Codex",
            &format!(
                "old reply {idx} {}",
                "with enough words to wrap inside the dialogue box ".repeat(2)
            ),
        );
    }
    runtime.mark_compose_running("Compose running", "latest prompt");
    runtime.mark_compose_succeeded("Codex", "latest reply should remain visible");

    let metrics = runtime.vn_dialogue_scroll_metrics(80, 24, usize::MAX);
    assert!(metrics.max_scroll_offset > 0);

    let frame = runtime.render_text_frame_with_dialogue_scroll(80, 24, metrics.max_scroll_offset);

    assert!(frame.contains("> old prompt 0"));
    assert!(!frame.contains("latest reply should remain visible"));
}

#[test]
fn staged_scene_reports_dialogue_scroll_metrics_for_overflowing_transcript() {
    let mut runtime = SceneRuntime::new(staged_compose_scene()).unwrap();

    for idx in 0..8 {
        runtime.mark_compose_running("Compose running", &format!("prompt {idx}"));
        runtime.mark_compose_succeeded(
            "Codex",
            &format!(
                "reply {idx} {}",
                "with enough words to overflow the dialogue panel ".repeat(3)
            ),
        );
    }

    assert!(
        runtime
            .vn_dialogue_scroll_metrics(80, 24, 0)
            .max_scroll_offset
            > 0
    );
}

#[test]
fn staged_scene_dialogue_scroll_metrics_clamp_offset() {
    let mut runtime = SceneRuntime::new(staged_compose_scene()).unwrap();

    for idx in 0..6 {
        runtime.mark_compose_running("Compose running", &format!("prompt {idx}"));
        runtime.mark_compose_succeeded("Codex", &"reply ".repeat(40));
    }

    let metrics = runtime.vn_dialogue_scroll_metrics(80, 24, usize::MAX);

    assert!(metrics.total_lines > metrics.visible_rows);
    assert_eq!(metrics.scroll_offset, metrics.max_scroll_offset);
}

#[test]
fn compose_runtime_history_is_capped() {
    let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();

    for idx in 0..30 {
        runtime.mark_compose_running("Compose running", &format!("prompt {idx}"));
    }

    assert_eq!(runtime.compose_state.history.len(), 20);
    assert_eq!(runtime.compose_state.history[0].text, "prompt 10");
    assert_eq!(runtime.compose_state.history[19].text, "prompt 29");
}

#[test]
fn compose_backend_prompt_includes_recent_turn_context_for_followups() {
    let mut runtime = SceneRuntime::new(staged_compose_scene()).unwrap();

    assert_eq!(runtime.compose_backend_prompt("hello"), "hello");

    runtime.mark_compose_running("Compose running", "whats the weather today?");
    runtime.mark_compose_succeeded("Codex", "What city or ZIP code should I check?");

    let prompt = runtime.compose_backend_prompt("11249");

    assert!(prompt.contains("GameTerm Scene Mode conversation context follows."));
    assert!(prompt.contains("Latest user prompt:\n11249"));
    assert!(prompt.contains("User: whats the weather today?"));
    assert!(prompt.contains("Codex: What city or ZIP code should I check?"));
    assert!(prompt.contains("If the latest user prompt is a fragment"));
    assert!(!prompt.contains("User: 11249"));
}

#[test]
fn layered_input_map_owns_input_before_mode_map() {
    let mut scene = VisualScene::demo();
    scene.mode.input_map = vec![VisualInputBinding {
        input: "other".to_string(),
        action: "toggle_debug".to_string(),
        conditions: Vec::new(),
    }];
    scene.layers = vec![VisualLayerState {
        layer_id: "ui".to_string(),
        state: "scene".to_string(),
        label: None,
        input_map: vec![VisualInputBinding {
            input: "other".to_string(),
            action: "run_update_hooks".to_string(),
            conditions: Vec::new(),
        }],
        transitions: Vec::new(),
    }];
    scene.mode.lifecycle.update_status = Some("Layer handled update".to_string());
    let mut runtime = SceneRuntime::new(scene).unwrap();

    let outcome = runtime.handle_input(VisualInput::Other);

    assert_eq!(outcome, VisualModeOutcome::Continue);
    assert_eq!(runtime.view(), VisualView::Scene);
    assert_eq!(runtime.render_snapshot().status, "Layer handled update");
    assert_eq!(
        runtime.debug_report().last_input_layer.as_deref(),
        Some("ui")
    );
}

#[test]
fn scene_rejects_invalid_layer_state() {
    let mut scene = VisualScene::demo();
    scene.layers = vec![VisualLayerState {
        layer_id: "story".to_string(),
        state: " ".to_string(),
        label: None,
        input_map: Vec::new(),
        transitions: Vec::new(),
    }];

    assert_eq!(
        scene.validate(),
        Err(VisualSceneError::EmptyLayerState {
            layer_id: "story".to_string()
        })
    );
}

#[test]
fn guarded_choice_blocks_action_when_variable_mismatches() {
    let mut scene = VisualScene::demo();
    scene.choices[0].conditions = vec![VisualCondition {
        source: None,
        variable: "conversation_unlocked".to_string(),
        equals: VisualStateValue::Bool(false),
    }];
    let mut runtime = SceneRuntime::new(scene).unwrap();
    let initial_generation = runtime.generation();

    runtime.activate_choice();
    let snapshot = runtime.render_snapshot();

    assert!(runtime.generation() > initial_generation);
    assert_eq!(
        snapshot.status,
        "Choice unavailable: requires conversation_unlocked=false"
    );
    assert_eq!(runtime.take_pending_action(), None);
}

#[test]
fn guarded_choice_allows_action_when_variable_matches() {
    let mut scene = VisualScene::demo();
    scene.choices[0].conditions = vec![VisualCondition {
        source: None,
        variable: "conversation_unlocked".to_string(),
        equals: VisualStateValue::Bool(true),
    }];
    let mut runtime = SceneRuntime::new(scene).unwrap();

    runtime.activate_choice();

    assert_eq!(
        runtime.render_snapshot().status,
        "Inspecting GameTerm (project-gameterm)"
    );
}

#[test]
fn guarded_choice_state_is_visible_in_debugger() {
    let mut scene = VisualScene::demo();
    scene.choices[0].conditions = vec![VisualCondition {
        source: None,
        variable: "workspace_level".to_string(),
        equals: VisualStateValue::Number(2),
    }];
    let runtime = SceneRuntime::new(scene).unwrap();

    let report = runtime.debug_report();

    assert!(!report.selected_choice_enabled);
    assert_eq!(
        report.selected_choice_guard_detail.as_deref(),
        Some("requires workspace_level=2")
    );

    let frame = runtime.render_debugger(100, 40);
    assert!(frame.contains("Choice enabled: false"));
    assert!(frame.contains("Choice guard: requires workspace_level=2"));
}

#[test]
fn guarded_choice_renders_locked_marker() {
    let mut scene = VisualScene::demo();
    scene.choices[0].conditions = vec![VisualCondition {
        source: None,
        variable: "active_track".to_string(),
        equals: VisualStateValue::Text("memory".to_string()),
    }];
    let runtime = SceneRuntime::new(scene).unwrap();

    let frame = runtime.render_text_frame(120, 40);

    assert!(frame.contains("> Inspect selected entity [locked]"));
}

#[test]
fn rpg_condition_sources_guard_choices() {
    let mut scene = VisualScene::demo();
    scene.choices[0].conditions = vec![
        VisualCondition {
            source: Some("inventory_count".to_string()),
            variable: "scene-token".to_string(),
            equals: VisualStateValue::Number(1),
        },
        VisualCondition {
            source: Some("quest_stage".to_string()),
            variable: "verify-scene-runtime".to_string(),
            equals: VisualStateValue::Number(1),
        },
        VisualCondition {
            source: Some("quest_completed".to_string()),
            variable: "verify-scene-runtime".to_string(),
            equals: VisualStateValue::Bool(false),
        },
        VisualCondition {
            source: Some("stat".to_string()),
            variable: "project-gameterm:focus".to_string(),
            equals: VisualStateValue::Number(3),
        },
        VisualCondition {
            source: Some("agent_phase".to_string()),
            variable: "ignored".to_string(),
            equals: VisualStateValue::Text("running".to_string()),
        },
        VisualCondition {
            source: Some("process_phase".to_string()),
            variable: "ignored".to_string(),
            equals: VisualStateValue::Text("succeeded".to_string()),
        },
        VisualCondition {
            source: Some("selected_entity_flag".to_string()),
            variable: "active".to_string(),
            equals: VisualStateValue::Bool(true),
        },
        VisualCondition {
            source: Some("selected_entity_metadata".to_string()),
            variable: "mode".to_string(),
            equals: VisualStateValue::Text("hard-fork".to_string()),
        },
    ];
    scene.variables.push(VisualStateEntry {
        key: "agent_phase".to_string(),
        value: VisualStateValue::Text("running".to_string()),
    });
    let mut runtime = SceneRuntime::new(scene).unwrap();
    runtime.last_process_state = Some(VisualProcessState {
        phase: VisualProcessPhase::Succeeded,
        entity_id: Some("agent-audit".to_string()),
        command: Some("agent:completed".to_string()),
        exit_code: Some(0),
        message: Some("Done".to_string()),
    });

    runtime.activate_choice();

    assert_eq!(
        runtime.render_snapshot().status,
        "Inspecting GameTerm (project-gameterm)"
    );

    runtime.scene.choices[0].conditions[0].equals = VisualStateValue::Number(2);
    runtime.activate_choice();

    assert_eq!(
            runtime.render_snapshot().status,
            "Choice unavailable: requires inventory_count:scene-token=2, quest_stage:verify-scene-runtime=1, quest_completed:verify-scene-runtime=false, stat:project-gameterm:focus=3, agent_phase:ignored=running, process_phase:ignored=succeeded, selected_entity_flag:active=true, selected_entity_metadata:mode=hard-fork"
        );
}

#[test]
fn dialogue_lines_override_legacy_dialogue_in_snapshot() {
    let runtime = SceneRuntime::new(branching_dialogue_scene()).unwrap();
    let snapshot = runtime.render_snapshot();

    assert_eq!(snapshot.dialogue_speaker, "Guide");
    assert_eq!(snapshot.dialogue, "Choose a route.");
    assert_eq!(snapshot.dialogue_index, Some(0));
    assert_eq!(snapshot.dialogue_history.len(), 1);
    assert_eq!(snapshot.dialogue_history[0].metadata[0].1, "start");
}

#[test]
fn advance_dialogue_choice_updates_runtime_history() {
    let mut runtime = SceneRuntime::new(branching_dialogue_scene()).unwrap();

    runtime.activate_choice();
    let snapshot = runtime.render_snapshot();

    assert_eq!(snapshot.dialogue_index, Some(1));
    assert_eq!(snapshot.dialogue, "Workspace branch selected.");
    assert_eq!(snapshot.dialogue_history.len(), 2);
    assert_eq!(snapshot.status, "Dialogue advanced: Guide");
}

#[test]
fn guarded_branching_choice_blocks_unavailable_dialogue_path() {
    let mut runtime = SceneRuntime::new(branching_dialogue_scene()).unwrap();
    runtime.select_next_choice();

    runtime.activate_choice();
    let snapshot = runtime.render_snapshot();

    assert_eq!(snapshot.dialogue_index, Some(0));
    assert_eq!(
        snapshot.status,
        "Choice unavailable: requires active_track=memory"
    );
    assert_eq!(snapshot.dialogue_history.len(), 1);
}

#[test]
fn dialogue_runtime_state_is_visible_in_debugger() {
    let mut runtime = SceneRuntime::new(branching_dialogue_scene()).unwrap();
    runtime.activate_choice();
    let report = runtime.debug_report();

    assert_eq!(report.dialogue_index, Some(1));
    assert_eq!(report.dialogue_line_count, 3);
    assert_eq!(report.dialogue_history.len(), 2);
    assert_eq!(
        report.selected_choice_kind.as_deref(),
        Some("AdvanceDialogue")
    );
    assert_eq!(report.selected_choice_detail.as_deref(), Some("target=1"));

    let frame = runtime.render_debugger(120, 40);
    assert!(frame.contains("Active line: 2 of 3"));
    assert!(frame.contains("History entries: 2"));
    assert!(frame.contains("Choice kind: AdvanceDialogue"));
}

#[test]
fn story_state_export_includes_variables_and_dialogue_position() {
    let mut runtime = SceneRuntime::new(branching_dialogue_scene()).unwrap();
    runtime.activate_choice();

    let state = runtime.export_story_state();

    assert_eq!(state.story_state_version, VisualStoryState::VERSION);
    assert!(state.variables.contains(&VisualStateEntry {
        key: "active_track".to_string(),
        value: VisualStateValue::Text("visual-state".to_string()),
    }));
    assert_eq!(state.dialogue_index, Some(1));
    assert_eq!(state.dialogue_history.len(), 2);
    assert_eq!(state.rpg.inventory.len(), 1);

    let json = runtime.story_state_json_pretty().unwrap();
    assert!(json.contains("\"story_state_version\": 1"));
    assert!(json.contains("\"dialogue_index\": 1"));
}

#[test]
fn story_state_import_restores_variables_and_dialogue() {
    let mut source = SceneRuntime::new(branching_dialogue_scene()).unwrap();
    source.activate_choice();
    let mut state = source.export_story_state();
    state.variables.push(VisualStateEntry {
        key: "quest_stage".to_string(),
        value: VisualStateValue::Number(3),
    });
    let mut target = SceneRuntime::new(branching_dialogue_scene()).unwrap();
    let initial_generation = target.generation();

    target.import_story_state(state).unwrap();
    let snapshot = target.render_snapshot();

    assert!(target.generation() > initial_generation);
    assert_eq!(snapshot.dialogue_index, Some(1));
    assert_eq!(snapshot.dialogue, "Workspace branch selected.");
    assert!(snapshot.variables.contains(&VisualStateEntry {
        key: "quest_stage".to_string(),
        value: VisualStateValue::Number(3),
    }));
    assert_eq!(snapshot.rpg.inventory.len(), 1);
    assert_eq!(snapshot.status, "Imported story state");
}

#[test]
fn story_state_import_rejects_out_of_bounds_dialogue_without_mutation() {
    let mut runtime = SceneRuntime::new(branching_dialogue_scene()).unwrap();
    let before = runtime.render_snapshot();
    let mut state = runtime.export_story_state();
    state.dialogue_index = Some(99);

    assert_eq!(
        runtime.import_story_state(state),
        Err(VisualStoryStateError::DialogueIndexOutOfBounds { target: 99 })
    );
    assert_eq!(runtime.render_snapshot(), before);
}

#[test]
fn story_state_rejects_duplicate_variable_key() {
    let state = VisualStoryState {
        story_state_version: VisualStoryState::VERSION,
        variables: vec![
            VisualStateEntry {
                key: "quest_stage".to_string(),
                value: VisualStateValue::Number(1),
            },
            VisualStateEntry {
                key: "quest_stage".to_string(),
                value: VisualStateValue::Number(2),
            },
        ],
        rpg: VisualRpgState::default(),
        dialogue_index: None,
        dialogue_history: vec![],
    };

    assert!(matches!(
        state.validate(),
        Err(VisualStoryStateError::DuplicateVariableKey(key)) if key == "quest_stage"
    ));
}

#[test]
fn story_state_rejects_empty_history_dialogue_text() {
    let state = VisualStoryState {
        story_state_version: VisualStoryState::VERSION,
        variables: vec![],
        rpg: VisualRpgState::default(),
        dialogue_index: None,
        dialogue_history: vec![VisualDialogueLine {
            speaker: "Guide".to_string(),
            text: " ".to_string(),
            portrait: None,
            metadata: vec![],
        }],
    };

    assert_eq!(
        state.validate(),
        Err(VisualStoryStateError::EmptyDialogueText { index: 0 })
    );
}

#[test]
fn runtime_snapshot_includes_scene_source_status() {
    let source = VisualSceneSource::new("bundled default", VisualSceneLoadStatus::Bundled, 1);
    let runtime = SceneRuntime::new_with_source(VisualScene::demo(), source.clone()).unwrap();
    let snapshot = runtime.render_snapshot();

    assert_eq!(snapshot.scene_source, source);
}

#[test]
fn open_file_action_resolves_relative_path_against_base_dir() {
    let dir = tempfile::tempdir().unwrap();
    let docs_dir = dir.path().join("docs");
    std::fs::create_dir(&docs_dir).unwrap();
    std::fs::write(docs_dir.join("scene.md"), "scene docs").unwrap();
    let mut scene = VisualScene::demo();
    scene.choices = vec![SceneAction {
        label: "Open scene docs".to_string(),
        kind: SceneActionKind::OpenFile {
            path: "docs/scene.md".to_string(),
        },
        policy: None,
        conditions: vec![],
    }];
    let mut runtime = SceneRuntime::new_with_source_and_action_base_dir(
        scene,
        VisualSceneSource::new("/tmp/default.json", VisualSceneLoadStatus::Loaded, 1),
        dir.path(),
    )
    .unwrap();

    runtime.activate_choice();
    let snapshot = runtime.render_snapshot();

    assert!(snapshot.status.starts_with("OpenFile ready: "));
    assert!(snapshot.status.contains("docs/scene.md"));
    assert_eq!(
        runtime.take_pending_action(),
        Some(VisualActionRequest::OpenFile {
            path: docs_dir.join("scene.md")
        })
    );
    assert_eq!(runtime.take_pending_action(), None);
}

#[test]
fn open_file_action_reports_missing_target() {
    let dir = tempfile::tempdir().unwrap();
    let mut scene = VisualScene::demo();
    scene.choices = vec![SceneAction {
        label: "Open missing docs".to_string(),
        kind: SceneActionKind::OpenFile {
            path: "missing.md".to_string(),
        },
        policy: None,
        conditions: vec![],
    }];
    let mut runtime = SceneRuntime::new_with_source_and_action_base_dir(
        scene,
        VisualSceneSource::new("/tmp/default.json", VisualSceneLoadStatus::Loaded, 1),
        dir.path(),
    )
    .unwrap();

    runtime.activate_choice();
    let snapshot = runtime.render_snapshot();

    assert!(snapshot.status.starts_with("OpenFile missing: "));
    assert!(snapshot.status.contains("missing.md"));
    assert_eq!(runtime.take_pending_action(), None);
}

#[test]
fn open_file_action_reports_directory_target() {
    let dir = tempfile::tempdir().unwrap();
    let mut scene = VisualScene::demo();
    scene.choices = vec![SceneAction {
        label: "Open directory".to_string(),
        kind: SceneActionKind::OpenFile {
            path: ".".to_string(),
        },
        policy: None,
        conditions: vec![],
    }];
    let mut runtime = SceneRuntime::new_with_source_and_action_base_dir(
        scene,
        VisualSceneSource::new("/tmp/default.json", VisualSceneLoadStatus::Loaded, 1),
        dir.path(),
    )
    .unwrap();

    runtime.activate_choice();
    let snapshot = runtime.render_snapshot();

    assert!(snapshot
        .status
        .starts_with("OpenFile target is not a file: "));
    assert_eq!(runtime.take_pending_action(), None);
}

#[test]
fn open_file_dispatched_status_updates_generation() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("scene.md");
    std::fs::write(&path, "scene docs").unwrap();
    let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
    let generation_before = runtime.generation();

    runtime.mark_open_file_dispatched(&path);
    let snapshot = runtime.render_snapshot();

    assert!(snapshot.status.starts_with("OpenFile opening: "));
    assert!(runtime.generation() > generation_before);
}

#[test]
fn story_state_actions_emit_pending_requests() {
    let dir = tempfile::tempdir().unwrap();
    let mut scene = VisualScene::demo();
    scene.choices = vec![
        SceneAction {
            label: "Export story".to_string(),
            kind: SceneActionKind::ExportStoryState {
                path: "state/story.json".to_string(),
            },
            policy: None,
            conditions: vec![],
        },
        SceneAction {
            label: "Import story".to_string(),
            kind: SceneActionKind::ImportStoryState {
                path: "state/story.json".to_string(),
            },
            policy: None,
            conditions: vec![],
        },
    ];
    let mut runtime = SceneRuntime::new_with_source_and_action_base_dir(
        scene,
        VisualSceneSource::new("/tmp/default.json", VisualSceneLoadStatus::Loaded, 1),
        dir.path(),
    )
    .unwrap();
    let state_path = dir.path().join("state/story.json");

    runtime.activate_choice();
    assert_eq!(
        runtime.take_pending_action(),
        Some(VisualActionRequest::ExportStoryState {
            path: state_path.clone()
        })
    );
    assert!(runtime
        .render_snapshot()
        .status
        .starts_with("ExportStoryState ready: "));

    runtime.select_next_choice();
    runtime.activate_choice();
    assert_eq!(
        runtime.take_pending_action(),
        Some(VisualActionRequest::ImportStoryState { path: state_path })
    );
}

#[test]
fn story_state_input_map_uses_default_scene_state_path() {
    let mut scene = VisualScene::demo();
    scene.mode.input_map = vec![VisualInputBinding {
        input: "other".to_string(),
        action: "export_story_state".to_string(),
        conditions: vec![],
    }];
    let mut runtime = SceneRuntime::new_with_source(
        scene,
        VisualSceneSource::new(
            "/tmp/gameterm/scenes/default.json",
            VisualSceneLoadStatus::Loaded,
            1,
        ),
    )
    .unwrap();

    runtime.handle_input(VisualInput::Other);

    assert_eq!(
        runtime.take_pending_action(),
        Some(VisualActionRequest::ExportStoryState {
            path: PathBuf::from("/tmp/gameterm/scenes/default.story.json")
        })
    );
}

#[test]
fn story_state_status_helpers_update_debug_report() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("story.json");
    let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();

    runtime.mark_story_state_exported(&path);
    let report = runtime.debug_report();
    assert_eq!(report.last_story_state_action.as_deref(), Some("export"));
    assert_eq!(
        report.last_story_state_path,
        Some(path.display().to_string())
    );

    let frame = runtime.render_debugger(120, 40);
    assert!(frame.contains("Last story state: export"));
}

#[test]
fn authoring_mode_renders_story_state_path_in_scene_view() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("story.json");
    let mut scene = VisualScene::demo();
    scene.mode.mode_id = "authoring".to_string();
    scene.mode.label = "Authoring".to_string();
    let mut runtime = SceneRuntime::new_with_source_and_action_base_dir(
        scene,
        VisualSceneSource::new("/tmp/default.json", VisualSceneLoadStatus::Loaded, 1),
        dir.path(),
    )
    .unwrap();

    let frame = runtime.render_text_frame(120, 40);
    assert!(frame.contains("Story State: default /tmp/default.story.json"));

    runtime.mark_story_state_imported(&path);
    let frame = runtime.render_text_frame(120, 40);
    assert!(frame.contains(&format!("Story State: import {}", path.display())));
}

#[test]
fn scene_rejects_empty_story_state_action_path() {
    let mut scene = VisualScene::demo();
    scene.choices = vec![SceneAction {
        label: "Export story".to_string(),
        kind: SceneActionKind::ExportStoryState {
            path: " ".to_string(),
        },
        policy: None,
        conditions: vec![],
    }];

    assert_eq!(
        scene.validate(),
        Err(VisualSceneError::EmptyStoryStatePath {
            label: "Export story".to_string()
        })
    );
}

#[test]
fn navigate_action_emits_pending_request() {
    let mut scene = VisualScene::demo();
    scene.choices = vec![SceneAction {
        label: "Go to memory".to_string(),
        kind: SceneActionKind::Navigate {
            target: "memory.json".to_string(),
        },
        policy: None,
        conditions: vec![],
    }];
    let mut runtime = SceneRuntime::new(scene).unwrap();

    runtime.activate_choice();
    let snapshot = runtime.render_snapshot();

    assert_eq!(snapshot.status, "Navigate ready: memory.json");
    assert_eq!(
        runtime.take_pending_action(),
        Some(VisualActionRequest::Navigate {
            target: "memory.json".to_string()
        })
    );
}

#[test]
fn run_command_action_emits_explicit_argv_request() {
    let mut scene = VisualScene::demo();
    scene.choices = vec![SceneAction {
        label: "Run true".to_string(),
        kind: SceneActionKind::RunCommand {
            argv: vec!["true".to_string()],
            cwd: Some("/tmp".to_string()),
            target: RunCommandTarget::SplitRight,
        },
        policy: None,
        conditions: vec![],
    }];
    let mut runtime = SceneRuntime::new(scene).unwrap();

    runtime.activate_choice();
    let snapshot = runtime.render_snapshot();

    assert_eq!(snapshot.status, "RunCommand ready (split_right): true");
    assert_eq!(
        runtime.take_pending_action(),
        Some(VisualActionRequest::RunCommand {
            argv: vec!["true".to_string()],
            cwd: Some(PathBuf::from("/tmp")),
            target: RunCommandTarget::SplitRight,
        })
    );
}

#[test]
fn action_status_compatibility_covers_pending_requests() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("scene.md");
    std::fs::write(&file_path, "scene docs").unwrap();
    let mut scene = VisualScene::demo();
    scene.choices = vec![
        SceneAction {
            label: "Open docs".to_string(),
            kind: SceneActionKind::OpenFile {
                path: "scene.md".to_string(),
            },
            policy: None,
            conditions: vec![],
        },
        SceneAction {
            label: "Run true".to_string(),
            kind: SceneActionKind::RunCommand {
                argv: vec!["true".to_string()],
                cwd: Some("/tmp".to_string()),
                target: RunCommandTarget::SplitRight,
            },
            policy: None,
            conditions: vec![],
        },
        SceneAction {
            label: "Navigate".to_string(),
            kind: SceneActionKind::Navigate {
                target: "memory.json".to_string(),
            },
            policy: None,
            conditions: vec![],
        },
        SceneAction {
            label: "Export".to_string(),
            kind: SceneActionKind::ExportStoryState {
                path: "story.json".to_string(),
            },
            policy: None,
            conditions: vec![],
        },
        SceneAction {
            label: "Import".to_string(),
            kind: SceneActionKind::ImportStoryState {
                path: "story.json".to_string(),
            },
            policy: None,
            conditions: vec![],
        },
    ];
    let mut runtime = SceneRuntime::new_with_source_and_action_base_dir(
        scene,
        VisualSceneSource::new("/tmp/default.json", VisualSceneLoadStatus::Loaded, 1),
        dir.path(),
    )
    .unwrap();

    runtime.activate_choice();
    assert_eq!(
        runtime.render_snapshot().status,
        format!("OpenFile ready: {}", file_path.display())
    );
    assert_eq!(
        runtime.take_pending_action(),
        Some(VisualActionRequest::OpenFile {
            path: file_path.clone()
        })
    );

    runtime.select_next_choice();
    runtime.activate_choice();
    assert_eq!(
        runtime.render_snapshot().status,
        "RunCommand ready (split_right): true"
    );
    assert_eq!(
        runtime.take_pending_action(),
        Some(VisualActionRequest::RunCommand {
            argv: vec!["true".to_string()],
            cwd: Some(PathBuf::from("/tmp")),
            target: RunCommandTarget::SplitRight,
        })
    );

    runtime.select_next_choice();
    runtime.activate_choice();
    assert_eq!(
        runtime.render_snapshot().status,
        "Navigate ready: memory.json"
    );
    assert_eq!(
        runtime.take_pending_action(),
        Some(VisualActionRequest::Navigate {
            target: "memory.json".to_string()
        })
    );

    runtime.select_next_choice();
    runtime.activate_choice();
    assert_eq!(
        runtime.render_snapshot().status,
        format!(
            "ExportStoryState ready: {}",
            dir.path().join("story.json").display()
        )
    );

    runtime.select_next_choice();
    runtime.activate_choice();
    assert_eq!(
        runtime.render_snapshot().status,
        format!(
            "ImportStoryState ready: {}",
            dir.path().join("story.json").display()
        )
    );
}

#[test]
fn run_command_action_requires_explicit_argv() {
    let mut scene = VisualScene::demo();
    scene.choices = vec![SceneAction {
        label: "Run empty".to_string(),
        kind: SceneActionKind::RunCommand {
            argv: Vec::new(),
            cwd: None,
            target: RunCommandTarget::Tab,
        },
        policy: None,
        conditions: vec![],
    }];

    assert!(matches!(
        scene.validate(),
        Err(VisualSceneError::EmptyRunCommand { .. })
    ));
}

#[test]
fn run_command_status_helpers_update_debug_report() {
    let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
    let argv = vec!["true".to_string()];

    runtime.mark_run_command_spawning(&argv, RunCommandTarget::Tab);
    assert_eq!(
        runtime.debug_report().status,
        "RunCommand opening tab: true"
    );

    runtime.mark_run_command_spawned(&argv, RunCommandTarget::SplitDown, 123);
    assert_eq!(
        runtime.debug_report().status,
        "RunCommand opened split_down pane 123: true"
    );

    runtime.mark_run_command_failed(&argv, RunCommandTarget::SplitRight, "spawn failed");
    assert_eq!(
        runtime.debug_report().status,
        "RunCommand failed (split_right): true: spawn failed"
    );
}

#[test]
fn reload_failure_updates_source_status_and_preserves_scene() {
    let mut runtime = SceneRuntime::new_with_source(
        VisualScene::demo(),
        VisualSceneSource::new("/tmp/default.json", VisualSceneLoadStatus::Loaded, 1),
    )
    .unwrap();
    let selected_before = runtime.render_snapshot().selected_entity_id;

    runtime.mark_reload_failed(2, "bad scene json".to_string());
    let snapshot = runtime.render_snapshot();

    assert_eq!(snapshot.selected_entity_id, selected_before);
    assert_eq!(
        snapshot.scene_source.load_status,
        VisualSceneLoadStatus::ReloadFailed
    );
    assert_eq!(snapshot.scene_source.reload_count, 2);
    assert_eq!(
        snapshot.scene_source.last_error.as_deref(),
        Some("bad scene json")
    );
    assert!(snapshot.status.contains("Reload failed"));
}

#[test]
fn reload_success_preserves_selected_entity_id() {
    let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
    runtime.select_next_entity();
    assert_eq!(
        runtime.render_snapshot().selected_entity_id.as_deref(),
        Some("task-render")
    );

    let mut reloaded = VisualScene::demo();
    reloaded.entities.swap(0, 1);
    runtime
        .replace_scene_preserving_state(
            reloaded,
            VisualSceneSource::new("/tmp/default.json", VisualSceneLoadStatus::Loaded, 2),
        )
        .unwrap();

    assert_eq!(
        runtime.render_snapshot().selected_entity_id.as_deref(),
        Some("task-render")
    );
    assert_eq!(runtime.render_snapshot().scene_source.reload_count, 2);
}

#[test]
fn reload_success_resets_missing_selected_entity() {
    let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
    runtime.select_next_entity();

    let mut reloaded = VisualScene::demo();
    reloaded
        .entities
        .retain(|entity| entity.id != "task-render");
    reloaded
        .rpg
        .relationships
        .retain(|relationship| relationship.target_id != "task-render");
    runtime
        .replace_scene_preserving_state(
            reloaded,
            VisualSceneSource::new("/tmp/default.json", VisualSceneLoadStatus::Loaded, 2),
        )
        .unwrap();

    assert_eq!(
        runtime.render_snapshot().selected_entity_id.as_deref(),
        Some("project-gameterm")
    );
}

#[test]
fn mode_close_input_exits() {
    let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
    let initial_generation = runtime.generation();

    let outcome = runtime.handle_input(VisualInput::Close);

    assert_eq!(outcome, VisualModeOutcome::Exit);
    assert_eq!(runtime.generation(), initial_generation);
}

#[test]
fn mode_reload_input_is_ignored_by_runtime() {
    let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
    let initial = runtime.render_snapshot();

    let outcome = runtime.handle_input(VisualInput::Reload);

    assert_eq!(outcome, VisualModeOutcome::Continue);
    assert_eq!(runtime.render_snapshot(), initial);
}

#[test]
fn mode_other_input_is_ignored() {
    let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
    let initial = runtime.render_snapshot();

    let outcome = runtime.handle_input(VisualInput::Other);

    assert_eq!(outcome, VisualModeOutcome::Continue);
    assert_eq!(runtime.render_snapshot(), initial);
}

#[test]
fn scene_frame_contains_selected_entity() {
    let runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
    let frame = runtime.render_text_frame(200, 80);
    assert!(frame.contains("Selected: GameTerm"));
}

#[test]
fn scene_frame_contains_product_state_summary() {
    let mut scene = VisualScene::demo();
    scene.variables.extend([
        VisualStateEntry {
            key: "workspace_root".to_string(),
            value: VisualStateValue::Text("/tmp/gameterm".to_string()),
        },
        VisualStateEntry {
            key: "repo_status".to_string(),
            value: VisualStateValue::Text("dirty".to_string()),
        },
        VisualStateEntry {
            key: "active_pane_id".to_string(),
            value: VisualStateValue::Number(231),
        },
        VisualStateEntry {
            key: "process_phase".to_string(),
            value: VisualStateValue::Text("running".to_string()),
        },
    ]);
    scene.layers.push(VisualLayerState {
        layer_id: "process".to_string(),
        state: "running".to_string(),
        label: Some("Process".to_string()),
        transitions: Vec::new(),
        input_map: Vec::new(),
    });
    scene.entities[0].metadata.extend([
        ("entity_type".to_string(), "workspace".to_string()),
        ("root".to_string(), "/tmp/gameterm".to_string()),
    ]);
    let mut runtime = SceneRuntime::new(scene).unwrap();
    runtime.last_process_state = Some(VisualProcessState {
        entity_id: Some("task-render".to_string()),
        phase: VisualProcessPhase::Running,
        command: Some("cargo test -p gameterm-visual".to_string()),
        exit_code: None,
        message: Some("Verification running".to_string()),
    });

    let frame = runtime.render_text_frame(120, 40);

    assert!(frame.contains("Details: repo=JulianAbeleda/gameterm"));
    assert!(frame.contains("entity_type=workspace"));
    assert!(frame.contains("Layers: process=running"));
    assert!(frame.contains("Process: running, entity=task-render"));
    assert!(frame.contains("State: conversation_unlocked=true"));
    assert!(frame.contains("workspace_root=/tmp/gameterm"));
    assert!(frame.contains("Choices:"));
}

#[test]
fn scene_frame_groups_choices_by_action_kind() {
    let runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
    let frame = runtime.render_text_frame(120, 40);

    assert!(frame.contains("Choices:"));
    assert!(frame.contains("[Inspect]"));
    assert!(frame
        .contains("> Inspect selected entity  origin=unknown risk=inspect scope=selected_entity"));
    assert!(frame.contains("[OpenFile]"));
    assert!(frame.contains("  Open MIGRATION.md  origin=unknown risk=open_file scope=scene"));
    assert!(frame.contains("[RunCommand]"));
    assert!(frame.contains(
            "  Run cargo check -p gameterm-visual  origin=unknown risk=command scope=workspace confirm=true"
        ));
}

#[test]
fn action_policy_metadata_renders_in_scene_and_debugger() {
    let mut scene = VisualScene::demo();
    scene.choices[0].policy = Some(SceneActionPolicy {
        origin: "workspace_discovery".to_string(),
        risk: "inspect".to_string(),
        scope: "workspace".to_string(),
        requires_confirmation: false,
        summary: Some("Inspect generated workspace state".to_string()),
    });
    let runtime = SceneRuntime::new(scene).unwrap();

    let frame = runtime.render_text_frame(200, 80);
    assert!(frame.contains(
            "origin=workspace_discovery risk=inspect scope=workspace summary=Inspect generated workspace state"
        ));

    let debug = runtime.render_debugger(120, 80);
    assert!(debug.contains(
            "Choice policy: origin=workspace_discovery risk=inspect scope=workspace summary=Inspect generated workspace state"
        ));
}

#[test]
fn scene_rejects_invalid_action_policy_values() {
    let mut scene = VisualScene::demo();
    scene.choices[0].policy = Some(SceneActionPolicy {
        origin: "workspace-discovery".to_string(),
        risk: "inspect".to_string(),
        scope: "workspace".to_string(),
        requires_confirmation: false,
        summary: None,
    });

    assert!(matches!(
        SceneRuntime::new(scene),
        Err(VisualSceneError::UnknownActionPolicyOrigin { .. })
    ));
}

#[test]
fn command_options_include_policy_and_original_choice_index() {
    let mut scene = VisualScene::demo();
    scene.choices[1].policy = Some(SceneActionPolicy {
        origin: "workspace_discovery".to_string(),
        risk: "open_file".to_string(),
        scope: "workspace".to_string(),
        requires_confirmation: false,
        summary: Some("Open discovered migration notes".to_string()),
    });
    scene.choices[1].conditions.push(VisualCondition {
        source: None,
        variable: "missing_flag".to_string(),
        equals: VisualStateValue::Bool(true),
    });
    let runtime = SceneRuntime::new(scene).unwrap();

    let options = runtime.command_options();

    assert_eq!(options[1].choice_index, 1);
    assert_eq!(options[1].label, "Open MIGRATION.md");
    assert_eq!(options[1].action_kind, "OpenFile");
    assert_eq!(options[1].origin, "workspace_discovery");
    assert_eq!(options[1].risk, "open_file");
    assert_eq!(options[1].scope, "workspace");
    assert_eq!(
        options[1].summary.as_deref(),
        Some("Open discovered migration notes")
    );
    assert!(!options[1].requires_confirmation);
    assert!(!options[1].enabled);
    assert_eq!(
        options[1].guard_detail.as_deref(),
        Some("requires missing_flag=true")
    );
}

#[test]
fn command_options_filter_by_text_kind_risk_scope_and_enabled_state() {
    let mut scene = VisualScene::demo();
    scene.choices[0].policy = Some(SceneActionPolicy {
        origin: "authored".to_string(),
        risk: "inspect".to_string(),
        scope: "selected_entity".to_string(),
        requires_confirmation: false,
        summary: Some("Inspect the selected entity".to_string()),
    });
    scene.choices[2].policy = Some(SceneActionPolicy {
        origin: "workspace_discovery".to_string(),
        risk: "command".to_string(),
        scope: "workspace".to_string(),
        requires_confirmation: true,
        summary: Some("Run verification".to_string()),
    });
    scene.choices[2].conditions.push(VisualCondition {
        source: None,
        variable: "missing_flag".to_string(),
        equals: VisualStateValue::Bool(true),
    });
    let runtime = SceneRuntime::new(scene).unwrap();

    let inspect_options = runtime.filtered_command_options(&VisualCommandFilter {
        query: Some("selected".to_string()),
        action_kind: Some("Inspect".to_string()),
        risk: Some("inspect".to_string()),
        scope: Some("selected_entity".to_string()),
        enabled_only: true,
    });
    let enabled_command_options = runtime.filtered_command_options(&VisualCommandFilter {
        query: Some("verification".to_string()),
        action_kind: Some("RunCommand".to_string()),
        risk: Some("command".to_string()),
        scope: Some("workspace".to_string()),
        enabled_only: true,
    });
    let all_command_options = runtime.filtered_command_options(&VisualCommandFilter {
        query: Some("verification".to_string()),
        action_kind: Some("RunCommand".to_string()),
        risk: Some("command".to_string()),
        scope: Some("workspace".to_string()),
        enabled_only: false,
    });

    assert_eq!(
        inspect_options
            .iter()
            .map(|option| option.choice_index)
            .collect::<Vec<_>>(),
        vec![0]
    );
    assert!(enabled_command_options.is_empty());
    assert_eq!(all_command_options.len(), 1);
    assert_eq!(all_command_options[0].choice_index, 2);
    assert!(!all_command_options[0].enabled);
}

#[test]
fn command_selection_view_renders_policy_rows() {
    let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
    runtime.show_command_selection();

    let frame = runtime.render_text_frame(200, 80);

    assert!(frame.contains("GameTerm Command Selection"));
    assert!(frame.contains("> #00 Inspect"));
    assert!(frame.contains("inspect"));
    assert!(frame.contains("unknown"));
    assert!(frame.contains("selected_entity"));
    assert!(frame.contains("Inspect selected entity"));
    assert!(frame.contains("#02 RunCommand"));
    assert!(frame.contains("command"));
    assert!(frame.contains("confirm=true"));
}

#[test]
fn command_selection_input_preserves_entity_and_activates_selected_choice() {
    let mut scene = VisualScene::demo();
    scene.mode.input_map = vec![VisualInputBinding {
        input: "other".to_string(),
        action: "toggle_command_selection".to_string(),
        conditions: Vec::new(),
    }];
    let mut runtime = SceneRuntime::new(scene).unwrap();
    runtime.handle_input(VisualInput::Other);
    let selected_entity = runtime.render_snapshot().selected_entity_id;

    runtime.handle_input(VisualInput::Next);
    runtime.handle_input(VisualInput::Next);

    assert_eq!(runtime.view(), VisualView::CommandSelection);
    assert_eq!(
        runtime.render_snapshot().selected_entity_id,
        selected_entity
    );
    assert_eq!(runtime.render_snapshot().selected_choice, 2);

    runtime.handle_input(VisualInput::Activate);

    assert!(matches!(
        runtime.take_pending_action(),
        Some(VisualActionRequest::RunCommand { .. })
    ));
}

#[test]
fn debugger_frame_contains_scene_source_status() {
    let source = VisualSceneSource::new("/tmp/default.json", VisualSceneLoadStatus::Loaded, 3);
    let mut runtime = SceneRuntime::new_with_source(VisualScene::demo(), source).unwrap();
    runtime.activate_choice();
    let frame = runtime.render_debugger(200, 80);

    assert!(frame.contains("Scene path: /tmp/default.json"));
    assert!(frame.contains("Load status: loaded"));
    assert!(frame.contains("Reload counter: 3"));
    assert!(frame.contains("Active: Workspace (workspace)"));
    assert!(frame.contains("Status: Inspecting GameTerm"));
    assert!(frame.contains("Choice kind: Inspect"));
    assert!(frame.contains("Pending action: none"));
}

#[test]
fn debug_report_contains_authoring_state() {
    let source = VisualSceneSource::new("/tmp/default.json", VisualSceneLoadStatus::Loaded, 3);
    let mut runtime = SceneRuntime::new_with_source(VisualScene::demo(), source).unwrap();
    runtime.select_next_entity();
    runtime.select_next_choice();
    runtime.activate_choice();

    let report = runtime.debug_report();

    assert_eq!(report.scene_path, "/tmp/default.json");
    assert_eq!(report.load_status, "loaded");
    assert_eq!(report.reload_count, 3);
    assert!(!report.action_base_dir.is_empty());
    assert_eq!(report.active_mode_id, "workspace");
    assert_eq!(report.active_mode_label, "Workspace");
    assert_eq!(
        report.active_mode_description,
        "Project and process-oriented Scene Mode workspace"
    );
    assert_eq!(report.active_mode_scene_profile.as_deref(), Some("scene"));
    assert!(report
        .active_mode_allowed_actions
        .contains(&"Inspect".to_string()));
    assert_eq!(report.active_mode_default_transition, None);
    assert!(report
        .variables
        .iter()
        .any(|entry| entry.key == "workspace_level" && entry.value == VisualStateValue::Number(1)));
    assert_eq!(report.title, "GameTerm Scene Mode");
    assert_eq!(report.background, "workspace-map");
    assert_eq!(report.width, 18);
    assert_eq!(report.height, 9);
    assert_eq!(report.entity_count, 3);
    assert_eq!(report.choice_count, 3);
    assert_eq!(report.selected_entity_id.as_deref(), Some("task-render"));
    assert_eq!(report.selected_entity_mode.as_deref(), None);
    assert_eq!(
        report.selected_entity_label.as_deref(),
        Some("Render Scene")
    );
    assert_eq!(report.selected_entity_kind.as_deref(), Some("Task"));
    assert_eq!(report.selected_entity_sprite.as_deref(), Some("task_tile"));
    assert_eq!(report.selected_entity_flags, vec!["running"]);
    assert!(report
        .selected_entity_metadata
        .iter()
        .any(|(key, value)| key == "reference" && value == "visual novel scene flow"));
    assert_eq!(report.selected_choice, 1);
    assert_eq!(
        report.selected_choice_label.as_deref(),
        Some("Open MIGRATION.md")
    );
    assert_eq!(report.selected_choice_kind.as_deref(), Some("OpenFile"));
    assert_eq!(
        report.selected_choice_detail.as_deref(),
        Some("path=MIGRATION.md")
    );
    assert_eq!(report.pending_action_kind.as_deref(), None);
    assert_eq!(report.pending_action_detail.as_deref(), None);
    assert_eq!(report.last_patch_transport.as_deref(), None);
    assert_eq!(report.last_patch_source_pane_id, None);
    assert!(report.status.starts_with("OpenFile "));
}

#[test]
fn debug_report_contains_pending_action_state() {
    let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
    runtime.select_next_choice();
    runtime.select_next_choice();
    runtime.activate_choice();

    let report = runtime.debug_report();

    assert_eq!(report.selected_choice, 2);
    assert_eq!(
        report.selected_choice_label.as_deref(),
        Some("Run cargo check -p gameterm-visual")
    );
    assert_eq!(report.selected_choice_kind.as_deref(), Some("RunCommand"));
    assert_eq!(
        report.selected_choice_detail.as_deref(),
        Some("argv=cargo check -p gameterm-visual target=tab")
    );
    assert_eq!(report.pending_action_kind.as_deref(), Some("RunCommand"));
    assert_eq!(
        report.pending_action_detail.as_deref(),
        Some("argv=cargo check -p gameterm-visual target=tab")
    );

    let frame = runtime.render_debugger(100, 32);
    assert!(frame.contains("Choice label: Run cargo check -p gameterm-visual"));
    assert!(frame.contains("Choice kind: RunCommand"));
    assert!(
        frame.contains("Pending action: RunCommand argv=cargo check -p gameterm-visual target=tab")
    );
}

#[test]
fn truncate_to_screen_clips_rows_and_columns() {
    let frame = truncate_to_screen("abcdef\n123456\nxyz".to_string(), 3, 2);
    assert_eq!(frame, "abc\r\n123\r\n");
}

#[test]
fn truncate_to_screen_pads_to_full_screen_frame() {
    let frame = truncate_to_screen("a\nbc".to_string(), 4, 3);
    assert_eq!(frame, "a   \r\nbc  \r\n    \r\n");
}

#[test]
fn valid_scene_json_loads_from_path() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("scene.json");
    std::fs::write(
        &path,
        r#"{
                "title": "Loaded Scene",
                "background": "test",
                "width": 2,
                "height": 2,
                "entities": [{
                    "id": "task-one",
                    "kind": "Task",
                    "label": "Task One",
                    "position": { "x": 1, "y": 1 },
                    "sprite": "task"
                }],
                "dialogue_speaker": "Loader",
                "dialogue": "Loaded from disk.",
                "choices": [{
                    "label": "Open docs",
                    "kind": { "OpenFile": { "path": "docs/gameterm-scene-mode.md" } }
                }]
            }"#,
    )
    .unwrap();

    let scene = VisualScene::load_from_path(path).unwrap();
    assert_eq!(scene.title, "Loaded Scene");
    assert_eq!(scene.entities[0].id, "task-one");
    assert!(matches!(
        scene.choices[0].kind,
        SceneActionKind::OpenFile { .. }
    ));
}

#[test]
fn malformed_json_returns_scene_json_error() {
    assert!(matches!(
        VisualScene::from_json("{"),
        Err(VisualSceneError::Json(_))
    ));
}

#[test]
fn valid_sprite_manifest_resolves_relative_paths() {
    let manifest = VisualSpriteManifest::from_json(
        r#"{
                "sprites": [
                    { "id": "project_core", "path": "sprites/project.png" },
                    { "id": "agent_idle", "path": "/tmp/agent.png" }
                ]
            }"#,
    )
    .unwrap();

    let status = manifest.resolve_against("/tmp/gameterm/scenes/sprites.json");

    assert_eq!(status.sprites.len(), 2);
    assert_eq!(status.sprites[0].id, "project_core");
    assert_eq!(
        status.sprites[0].path,
        "/tmp/gameterm/scenes/sprites/project.png"
    );
    assert_eq!(status.sprites[1].path, "/tmp/agent.png");
    assert!(status.warnings.is_empty());
}

#[test]
fn duplicate_sprite_ids_are_rejected() {
    assert!(matches!(
        VisualSpriteManifest::from_json(
            r#"{
                    "sprites": [
                        { "id": "task_tile", "path": "a.png" },
                        { "id": "task_tile", "path": "b.png" }
                    ]
                }"#
        ),
        Err(VisualSpriteManifestError::DuplicateSpriteId(_))
    ));
}

#[test]
fn empty_sprite_id_is_rejected() {
    assert!(matches!(
        VisualSpriteManifest::from_json(r#"{ "sprites": [{ "id": " ", "path": "sprite.png" }] }"#),
        Err(VisualSpriteManifestError::EmptySpriteId)
    ));
}

#[test]
fn empty_sprite_path_is_rejected() {
    assert!(matches!(
        VisualSpriteManifest::from_json(r#"{ "sprites": [{ "id": "task", "path": "" }] }"#),
        Err(VisualSpriteManifestError::EmptySpritePath { .. })
    ));
}

#[test]
fn out_of_bounds_entity_is_rejected() {
    let mut scene = VisualScene::demo();
    scene.entities[0].position.x = scene.width;
    assert!(matches!(
        scene.validate(),
        Err(VisualSceneError::EntityOutOfBounds { .. })
    ));
}

#[test]
fn snapshot_includes_all_demo_entities() {
    let runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
    let snapshot = runtime.render_snapshot();
    assert_eq!(snapshot.entities.len(), 3);
    assert_eq!(snapshot.entities[0].id, "project-gameterm");
    assert_eq!(snapshot.tiles.len(), snapshot.width * snapshot.height);
}

#[test]
fn snapshot_marks_selected_entity() {
    let runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
    let snapshot = runtime.render_snapshot();
    assert_eq!(
        snapshot.selected_entity_id.as_deref(),
        Some("project-gameterm")
    );
    assert_eq!(
        snapshot
            .entities
            .iter()
            .filter(|entity| entity.selected)
            .count(),
        1
    );
    assert!(snapshot.entities[0].selected);
}

#[test]
fn selection_changes_increment_generation() {
    let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
    let initial_generation = runtime.generation();
    runtime.select_next_entity();
    assert!(runtime.generation() > initial_generation);
    assert_eq!(
        runtime.render_snapshot().selected_entity_id.as_deref(),
        Some("task-render")
    );
}

#[test]
fn snapshot_generation_is_stable_without_state_changes() {
    let runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
    let first = runtime.render_snapshot();
    let second = runtime.render_snapshot();
    assert_eq!(first.generation, second.generation);
}

#[test]
fn activating_choice_updates_snapshot_status() {
    let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
    let initial = runtime.render_snapshot();

    runtime.activate_choice();
    let activated = runtime.render_snapshot();

    assert!(activated.generation > initial.generation);
    assert_ne!(activated.status, initial.status);
    assert_eq!(activated.status, "Inspecting GameTerm (project-gameterm)");
}

#[test]
fn empty_entities_render_without_selection() {
    let mut scene = VisualScene::demo();
    scene.entities.clear();
    scene.rpg.relationships.clear();

    let runtime = SceneRuntime::new(scene).unwrap();
    let snapshot = runtime.render_snapshot();

    assert_eq!(snapshot.selected_entity_id, None);
    assert!(snapshot.entities.is_empty());
    assert_eq!(snapshot.tiles.len(), snapshot.width * snapshot.height);
}

#[test]
fn empty_choices_do_not_change_generation_on_activate() {
    let mut scene = VisualScene::demo();
    scene.choices.clear();

    let mut runtime = SceneRuntime::new(scene).unwrap();
    let initial = runtime.render_snapshot();

    runtime.activate_choice();
    let activated = runtime.render_snapshot();

    assert_eq!(activated.generation, initial.generation);
    assert_eq!(activated.status, initial.status);
    assert!(activated.choices.is_empty());
}

#[test]
fn snapshot_layer_ordering_is_deterministic() {
    let runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
    let first = runtime.render_snapshot();
    let second = runtime.render_snapshot();
    assert_eq!(first.tiles, second.tiles);
    assert_eq!(first.entities, second.entities);
    assert!(first
        .tiles
        .iter()
        .all(|tile| tile.layer == VisualRenderLayer::Tile));
    assert!(first
        .entities
        .iter()
        .all(|entity| entity.layer == VisualRenderLayer::Entity));
}

#[test]
fn visible_tiles_for_row_matches_only_requested_row() {
    let snapshot = snapshot_for_filtering();
    let tiles = visible_tiles_for_row(&snapshot, 1, 0..snapshot.width);

    assert_eq!(tiles.len(), 3);
    assert_eq!(tiles[0].sprite, "left");
    assert_eq!(tiles[1].sprite, "middle");
    assert_eq!(tiles[2].sprite, "right");
    assert!(tiles.iter().all(|tile| tile.position.y == 1));
}

#[test]
fn visible_tiles_for_row_clips_to_viewport_columns() {
    let snapshot = snapshot_for_filtering();
    let tiles = visible_tiles_for_row(&snapshot, 1, 1..99);

    assert_eq!(tiles.len(), 2);
    assert_eq!(tiles[0].position.x, 1);
    assert_eq!(tiles[1].position.x, 3);
}

#[test]
fn intersecting_entities_for_row_matches_row_and_columns() {
    let snapshot = snapshot_for_filtering();
    let entities = intersecting_entities_for_row(&snapshot, 1, 1..4);

    assert_eq!(entities.len(), 1);
    assert_eq!(entities[0].id, "row-one-right");
    assert_eq!(entities[0].position, VisualPosition { x: 3, y: 1 });
}

#[test]
fn row_filter_helpers_return_empty_for_empty_data() {
    let mut snapshot = snapshot_for_filtering();
    snapshot.tiles.clear();
    snapshot.entities.clear();

    assert!(visible_tiles_for_row(&snapshot, 1, 0..snapshot.width).is_empty());
    assert!(intersecting_entities_for_row(&snapshot, 1, 0..snapshot.width).is_empty());
    assert!(visible_tiles_for_row(&snapshot, snapshot.height, 0..snapshot.width).is_empty());
    assert!(
        intersecting_entities_for_row(&snapshot, snapshot.height, 0..snapshot.width).is_empty()
    );
}

#[test]
fn scene_patch_updates_entity_state_and_status() {
    let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
    let initial_generation = runtime.generation();
    let patch = VisualScenePatch {
        scene_patch_version: VisualScenePatch::VERSION,
        updates: vec![VisualSceneEntityPatch {
            entity_id: "task-render".to_string(),
            label: None,
            position: None,
            sprite: None,
            visible: None,
            state_flags: Some(vec!["running".to_string(), "verified".to_string()]),
            metadata: Some(vec![("status".to_string(), "tests passed".to_string())]),
        }],
        variables: vec![],
        selected_entity_id: None,
        process_state: None,
        dialogue: None,
        status: Some("Verification passed".to_string()),
    };

    runtime.apply_scene_patch(patch).unwrap();

    assert!(runtime.generation() > initial_generation);
    assert_eq!(runtime.debug_report().status, "Verification passed");
    let entity = runtime
        .render_snapshot()
        .entities
        .into_iter()
        .find(|entity| entity.id == "task-render")
        .unwrap();
    assert_eq!(entity.state_flags, vec!["running", "verified"]);
    assert!(runtime
        .debug_report()
        .selected_entity_metadata
        .iter()
        .all(|(key, _)| key != "status"));
}

#[test]
fn scene_patch_source_is_reported_in_debugger() {
    let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
    let patch = VisualScenePatch {
        scene_patch_version: VisualScenePatch::VERSION,
        updates: vec![],
        variables: vec![],
        selected_entity_id: None,
        process_state: None,
        dialogue: None,
        status: Some("Source tracked".to_string()),
    };

    runtime
        .apply_scene_patch_with_source(patch, Some("mux".to_string()), Some(42))
        .unwrap();

    let report = runtime.debug_report();
    assert_eq!(report.last_patch_transport.as_deref(), Some("mux"));
    assert_eq!(report.last_patch_source_pane_id, Some(42));

    let frame = runtime.render_debugger(100, 40);
    assert!(frame.contains("Last patch: mux from pane 42"));
}

#[test]
fn scene_patch_updates_active_dialogue() {
    let mut runtime = SceneRuntime::new(branching_dialogue_scene()).unwrap();
    let patch = VisualScenePatch {
        scene_patch_version: VisualScenePatch::VERSION,
        updates: vec![],
        variables: vec![],
        selected_entity_id: None,
        process_state: None,
        dialogue: Some(VisualSceneDialoguePatch {
            speaker: "Codex".to_string(),
            text: "I can inspect the workspace from Scene Mode.".to_string(),
            append_history: false,
        }),
        status: Some("Compose succeeded".to_string()),
    };

    runtime.apply_scene_patch(patch).unwrap();

    let snapshot = runtime.render_snapshot();
    assert_eq!(snapshot.dialogue_speaker, "Codex");
    assert_eq!(
        snapshot.dialogue,
        "I can inspect the workspace from Scene Mode."
    );
    assert_eq!(snapshot.status, "Compose succeeded");
}

#[test]
fn scene_patch_appends_dialogue_history_when_requested() {
    let mut runtime = SceneRuntime::new(branching_dialogue_scene()).unwrap();
    let initial_history_len = runtime.render_snapshot().dialogue_history.len();
    let patch = VisualScenePatch {
        scene_patch_version: VisualScenePatch::VERSION,
        updates: vec![],
        variables: vec![],
        selected_entity_id: None,
        process_state: None,
        dialogue: Some(VisualSceneDialoguePatch {
            speaker: "Codex".to_string(),
            text: "The next reply is now part of the conversation.".to_string(),
            append_history: true,
        }),
        status: None,
    };

    runtime.apply_scene_patch(patch).unwrap();

    let snapshot = runtime.render_snapshot();
    assert_eq!(snapshot.dialogue_speaker, "Codex");
    assert_eq!(
        snapshot.dialogue,
        "The next reply is now part of the conversation."
    );
    assert_eq!(snapshot.dialogue_history.len(), initial_history_len + 1);
    assert_eq!(snapshot.dialogue_history.last().unwrap().speaker, "Codex");
}

#[test]
fn scene_patch_rejects_empty_dialogue_fields() {
    let patch = VisualScenePatch {
        scene_patch_version: VisualScenePatch::VERSION,
        updates: vec![],
        variables: vec![],
        selected_entity_id: None,
        process_state: None,
        dialogue: Some(VisualSceneDialoguePatch {
            speaker: " ".to_string(),
            text: "reply".to_string(),
            append_history: false,
        }),
        status: None,
    };

    assert!(matches!(
        patch.validate(),
        Err(VisualScenePatchError::EmptyDialogueSpeaker)
    ));

    let patch = VisualScenePatch {
        dialogue: Some(VisualSceneDialoguePatch {
            speaker: "Codex".to_string(),
            text: " ".to_string(),
            append_history: false,
        }),
        ..patch
    };

    assert!(matches!(
        patch.validate(),
        Err(VisualScenePatchError::EmptyDialogueText)
    ));
}

#[test]
fn scene_patch_updates_process_state() {
    let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
    let patch = VisualScenePatch {
        scene_patch_version: VisualScenePatch::VERSION,
        updates: vec![],
        variables: vec![],
        selected_entity_id: None,
        process_state: Some(VisualProcessState {
            entity_id: Some("task-render".to_string()),
            phase: VisualProcessPhase::Running,
            command: Some("cargo check".to_string()),
            exit_code: None,
            message: Some("checking workspace".to_string()),
        }),
        dialogue: None,
        status: Some("Process running: cargo check".to_string()),
    };

    runtime.apply_scene_patch(patch).unwrap();
    let report = runtime.debug_report();

    assert_eq!(
        report.process_state.as_ref().map(|state| state.phase),
        Some(VisualProcessPhase::Running)
    );
    assert_eq!(
        report
            .process_state
            .as_ref()
            .and_then(|state| state.entity_id.as_deref()),
        Some("task-render")
    );
    let frame = runtime.render_debugger(100, 40);
    assert!(frame.contains("Process phase: Running"));
    assert!(frame.contains("Process command: cargo check"));
}

#[test]
fn scene_patch_rejects_unknown_process_entity_without_mutation() {
    let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
    let before = runtime.render_snapshot();
    let patch = VisualScenePatch {
        scene_patch_version: VisualScenePatch::VERSION,
        updates: vec![],
        variables: vec![],
        selected_entity_id: None,
        process_state: Some(VisualProcessState {
            entity_id: Some("missing".to_string()),
            phase: VisualProcessPhase::Running,
            command: Some("cargo check".to_string()),
            exit_code: None,
            message: None,
        }),
        dialogue: None,
        status: Some("Should not apply".to_string()),
    };

    assert_eq!(
        runtime.apply_scene_patch(patch),
        Err(VisualScenePatchError::UnknownProcessEntityId(
            "missing".to_string()
        ))
    );
    assert_eq!(runtime.render_snapshot(), before);
    assert_eq!(runtime.debug_report().process_state, None);
}

#[test]
fn scene_patch_updates_typed_variables() {
    let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
    let patch = VisualScenePatch {
        scene_patch_version: VisualScenePatch::VERSION,
        updates: vec![],
        variables: vec![
            VisualStateEntry {
                key: "conversation_unlocked".to_string(),
                value: VisualStateValue::Bool(false),
            },
            VisualStateEntry {
                key: "quest_stage".to_string(),
                value: VisualStateValue::Number(2),
            },
            VisualStateEntry {
                key: "active_track".to_string(),
                value: VisualStateValue::Text("agent".to_string()),
            },
        ],
        selected_entity_id: None,
        process_state: None,
        dialogue: None,
        status: None,
    };

    runtime.apply_scene_patch(patch).unwrap();
    let report = runtime.debug_report();

    assert_eq!(
        report.status,
        "Applied scene patch: 0 entity update(s), 3 variable update(s)"
    );
    assert!(report.variables.contains(&VisualStateEntry {
        key: "conversation_unlocked".to_string(),
        value: VisualStateValue::Bool(false),
    }));
    assert!(report.variables.contains(&VisualStateEntry {
        key: "quest_stage".to_string(),
        value: VisualStateValue::Number(2),
    }));
    assert!(report.variables.contains(&VisualStateEntry {
        key: "active_track".to_string(),
        value: VisualStateValue::Text("agent".to_string()),
    }));
}

#[test]
fn scene_patch_rejects_duplicate_variable_key() {
    let patch = VisualScenePatch {
        scene_patch_version: VisualScenePatch::VERSION,
        updates: vec![],
        variables: vec![
            VisualStateEntry {
                key: "quest_stage".to_string(),
                value: VisualStateValue::Number(1),
            },
            VisualStateEntry {
                key: "quest_stage".to_string(),
                value: VisualStateValue::Number(2),
            },
        ],
        selected_entity_id: None,
        process_state: None,
        dialogue: None,
        status: None,
    };

    assert!(matches!(
        patch.validate(),
        Err(VisualScenePatchError::DuplicateVariableKey(key)) if key == "quest_stage"
    ));
}

#[test]
fn scene_patch_failure_source_is_reported() {
    let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();

    runtime.mark_scene_patch_failed("mux", Some(99), "bad patch");

    let report = runtime.debug_report();
    assert_eq!(report.last_patch_transport.as_deref(), Some("mux"));
    assert_eq!(report.last_patch_source_pane_id, Some(99));
    assert_eq!(
        report.status,
        "Scene patch failed from mux pane 99: bad patch"
    );
}

#[test]
fn scene_patch_rejects_unknown_entity_without_mutation() {
    let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
    let before = runtime.render_snapshot();
    let patch = VisualScenePatch {
        scene_patch_version: VisualScenePatch::VERSION,
        updates: vec![VisualSceneEntityPatch {
            entity_id: "missing".to_string(),
            label: None,
            position: None,
            sprite: None,
            visible: None,
            state_flags: Some(vec!["bad".to_string()]),
            metadata: None,
        }],
        variables: vec![],
        selected_entity_id: None,
        process_state: None,
        dialogue: None,
        status: Some("Should not apply".to_string()),
    };

    assert_eq!(
        runtime.apply_scene_patch(patch),
        Err(VisualScenePatchError::UnknownEntityId(
            "missing".to_string()
        ))
    );
    assert_eq!(runtime.render_snapshot(), before);
}

#[test]
fn scene_patch_updates_entity_visual_state() {
    let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
    let initial_generation = runtime.generation();
    let patch = VisualScenePatch {
        scene_patch_version: VisualScenePatch::VERSION,
        updates: vec![VisualSceneEntityPatch {
            entity_id: "task-render".to_string(),
            label: Some("Render Verified".to_string()),
            position: Some(VisualPosition { x: 5, y: 6 }),
            sprite: Some("task_tile_done".to_string()),
            visible: None,
            state_flags: None,
            metadata: None,
        }],
        variables: vec![],
        selected_entity_id: None,
        process_state: None,
        dialogue: None,
        status: Some("Visual state patched".to_string()),
    };

    runtime.apply_scene_patch(patch).unwrap();

    assert!(runtime.generation() > initial_generation);
    let entity = runtime
        .render_snapshot()
        .entities
        .into_iter()
        .find(|entity| entity.id == "task-render")
        .unwrap();
    assert_eq!(entity.label, "Render Verified");
    assert_eq!(entity.position, VisualPosition { x: 5, y: 6 });
    assert_eq!(entity.sprite, "task_tile_done");
}

#[test]
fn scene_patch_rejects_out_of_bounds_position_without_mutation() {
    let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
    let before = runtime.render_snapshot();
    let patch = VisualScenePatch {
        scene_patch_version: VisualScenePatch::VERSION,
        updates: vec![VisualSceneEntityPatch {
            entity_id: "task-render".to_string(),
            label: None,
            position: Some(VisualPosition { x: 99, y: 6 }),
            sprite: None,
            visible: None,
            state_flags: None,
            metadata: None,
        }],
        variables: vec![],
        selected_entity_id: None,
        process_state: None,
        dialogue: None,
        status: Some("Should not apply".to_string()),
    };

    assert_eq!(
        runtime.apply_scene_patch(patch),
        Err(VisualScenePatchError::EntityOutOfBounds {
            entity_id: "task-render".to_string(),
            x: 99,
            y: 6,
        })
    );
    assert_eq!(runtime.render_snapshot(), before);
}

#[test]
fn scene_patch_rejects_empty_selected_entity_id() {
    let patch = VisualScenePatch {
        scene_patch_version: VisualScenePatch::VERSION,
        updates: vec![],
        variables: vec![],
        selected_entity_id: Some(" ".to_string()),
        process_state: None,
        dialogue: None,
        status: Some("Should not apply".to_string()),
    };

    assert_eq!(
        patch.validate(),
        Err(VisualScenePatchError::EmptySelectedEntityId)
    );
}

#[test]
fn scene_patch_updates_visibility_and_focus() {
    let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
    let patch = VisualScenePatch {
        scene_patch_version: VisualScenePatch::VERSION,
        updates: vec![VisualSceneEntityPatch {
            entity_id: "task-render".to_string(),
            label: None,
            position: None,
            sprite: None,
            visible: Some(false),
            state_flags: None,
            metadata: None,
        }],
        variables: vec![],
        selected_entity_id: Some("agent-audit".to_string()),
        process_state: None,
        dialogue: None,
        status: Some("Visibility patched".to_string()),
    };

    runtime.apply_scene_patch(patch).unwrap();
    let snapshot = runtime.render_snapshot();

    assert_eq!(snapshot.selected_entity_id.as_deref(), Some("agent-audit"));
    assert!(snapshot
        .entities
        .iter()
        .all(|entity| entity.id != "task-render"));
}

#[test]
fn scene_patch_fixture_applies_to_default_scene() {
    let scene = VisualScene::load_from_path(scene_fixture_path("default.json")).unwrap();
    let patch = VisualScenePatch::load_from_path(scene_fixture_path("patch-status.json")).unwrap();
    let mut runtime = SceneRuntime::new(scene).unwrap();

    runtime.apply_scene_patch(patch).unwrap();
    let report = runtime.debug_report();

    assert_eq!(report.status, "Fixture patch applied");
    assert_eq!(
        report.selected_entity_id.as_deref(),
        Some("project-harness")
    );
    assert_eq!(report.selected_entity_flags, vec!["loaded", "verified"]);
    assert!(report
        .selected_entity_metadata
        .contains(&("status".to_string(), "patched".to_string())));
}

#[test]
fn scene_patch_fixture_rejects_unknown_entity() {
    let scene = VisualScene::load_from_path(scene_fixture_path("default.json")).unwrap();
    let patch =
        VisualScenePatch::load_from_path(scene_fixture_path("patch-unknown-entity.json")).unwrap();
    let mut runtime = SceneRuntime::new(scene).unwrap();

    assert!(matches!(
        runtime.apply_scene_patch(patch),
        Err(VisualScenePatchError::UnknownEntityId(id)) if id == "missing-entity"
    ));
}
