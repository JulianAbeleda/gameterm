use std::path::PathBuf;

use super::{
    default_scene_mode, SceneAction, SceneActionKind, VisualCondition, VisualDialogueLine,
    VisualEntityKind, VisualPosition, VisualRenderEntity, VisualRenderLayer, VisualRenderSnapshot,
    VisualRenderTile, VisualRpgState, VisualScene, VisualSceneLoadStatus, VisualSceneSource,
    VisualStateValue, VisualView,
};

pub(super) fn scene_fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("ci")
        .join("fixtures")
        .join("gameterm-scene")
        .join(name)
}

pub(super) fn snapshot_for_filtering() -> VisualRenderSnapshot {
    VisualRenderSnapshot {
        generation: 7,
        view: VisualView::Scene,
        scene_source: VisualSceneSource::new("fixture", VisualSceneLoadStatus::Loaded, 1),
        active_mode: default_scene_mode(),
        active_layers: Vec::new(),
        selected_entity_mode: None,
        variables: Vec::new(),
        rpg: VisualRpgState::default(),
        title: "Filter Fixture".to_string(),
        background: "floor".to_string(),
        width: 4,
        height: 3,
        selected_entity_id: None,
        selected_choice: 0,
        tiles: vec![
            VisualRenderTile {
                position: VisualPosition { x: 0, y: 1 },
                sprite: "left".to_string(),
                layer: VisualRenderLayer::Tile,
            },
            VisualRenderTile {
                position: VisualPosition { x: 1, y: 1 },
                sprite: "middle".to_string(),
                layer: VisualRenderLayer::Tile,
            },
            VisualRenderTile {
                position: VisualPosition { x: 3, y: 1 },
                sprite: "right".to_string(),
                layer: VisualRenderLayer::Tile,
            },
            VisualRenderTile {
                position: VisualPosition { x: 1, y: 2 },
                sprite: "other-row".to_string(),
                layer: VisualRenderLayer::Tile,
            },
        ],
        stage: Vec::new(),
        entities: vec![
            VisualRenderEntity {
                id: "row-one-left".to_string(),
                kind: VisualEntityKind::Task,
                label: "Row One Left".to_string(),
                position: VisualPosition { x: 0, y: 1 },
                sprite: "task".to_string(),
                layer: VisualRenderLayer::Entity,
                selected: false,
                state_flags: Vec::new(),
            },
            VisualRenderEntity {
                id: "row-one-right".to_string(),
                kind: VisualEntityKind::Agent,
                label: "Row One Right".to_string(),
                position: VisualPosition { x: 3, y: 1 },
                sprite: "agent".to_string(),
                layer: VisualRenderLayer::Entity,
                selected: false,
                state_flags: Vec::new(),
            },
            VisualRenderEntity {
                id: "row-two".to_string(),
                kind: VisualEntityKind::Memory,
                label: "Row Two".to_string(),
                position: VisualPosition { x: 1, y: 2 },
                sprite: "memory".to_string(),
                layer: VisualRenderLayer::Entity,
                selected: false,
                state_flags: Vec::new(),
            },
        ],
        dialogue_speaker: String::new(),
        dialogue: String::new(),
        dialogue_index: None,
        dialogue_history: Vec::new(),
        status: String::new(),
        choices: Vec::new(),
        overlay_cols: None,
        overlay_rows: None,
        vn_dialogue_scroll: None,
        vn_layout_debug: None,
    }
}

pub(super) fn branching_dialogue_scene() -> VisualScene {
    let mut scene = VisualScene::demo();
    scene.dialogue_speaker = "Narrator".to_string();
    scene.dialogue = "Legacy fallback".to_string();
    scene.dialogue_lines = vec![
        VisualDialogueLine {
            speaker: "Guide".to_string(),
            text: "Choose a route.".to_string(),
            portrait: Some("guide_neutral".to_string()),
            metadata: vec![("node".to_string(), "start".to_string())],
        },
        VisualDialogueLine {
            speaker: "Guide".to_string(),
            text: "Workspace branch selected.".to_string(),
            portrait: Some("guide_work".to_string()),
            metadata: vec![("node".to_string(), "workspace".to_string())],
        },
        VisualDialogueLine {
            speaker: "Guide".to_string(),
            text: "Memory branch selected.".to_string(),
            portrait: Some("guide_memory".to_string()),
            metadata: vec![("node".to_string(), "memory".to_string())],
        },
    ];
    scene.choices = vec![
        SceneAction {
            label: "Choose workspace".to_string(),
            kind: SceneActionKind::AdvanceDialogue { target: 1 },
            policy: None,
            conditions: vec![VisualCondition {
                source: None,
                variable: "active_track".to_string(),
                equals: VisualStateValue::Text("visual-state".to_string()),
            }],
        },
        SceneAction {
            label: "Choose memory".to_string(),
            kind: SceneActionKind::AdvanceDialogue { target: 2 },
            policy: None,
            conditions: vec![VisualCondition {
                source: None,
                variable: "active_track".to_string(),
                equals: VisualStateValue::Text("memory".to_string()),
            }],
        },
    ];
    scene
}
