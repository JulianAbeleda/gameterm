use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub mod render;
pub use render::{intersecting_entities_for_row, visible_tiles_for_row};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualPosition {
    pub x: usize,
    pub y: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VisualEntityKind {
    Agent,
    Memory,
    Principle,
    Project,
    Task,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualEntity {
    pub id: String,
    pub kind: VisualEntityKind,
    pub label: String,
    pub position: VisualPosition,
    pub sprite: String,
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub visible: bool,
    #[serde(default)]
    pub state_flags: Vec<String>,
    #[serde(default)]
    pub metadata: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SceneActionKind {
    Inspect,
    OpenFile {
        path: String,
    },
    RunCommand {
        argv: Vec<String>,
        #[serde(default)]
        cwd: Option<String>,
        #[serde(default)]
        target: RunCommandTarget,
    },
    Navigate {
        target: String,
    },
    ExportStoryState {
        path: String,
    },
    ImportStoryState {
        path: String,
    },
    AdvanceDialogue {
        target: usize,
    },
    Resolve {
        operations: Vec<VisualStateOperation>,
    },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunCommandTarget {
    #[default]
    Tab,
    SplitRight,
    SplitDown,
}

impl RunCommandTarget {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Tab => "tab",
            Self::SplitRight => "split_right",
            Self::SplitDown => "split_down",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneAction {
    pub label: String,
    pub kind: SceneActionKind,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<VisualCondition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VisualStateValue {
    Bool(bool),
    Number(i64),
    Text(String),
}

impl VisualStateValue {
    pub fn as_debug_string(&self) -> String {
        match self {
            Self::Bool(value) => value.to_string(),
            Self::Number(value) => value.to_string(),
            Self::Text(value) => value.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualStateEntry {
    pub key: String,
    pub value: VisualStateValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualDialogueLine {
    pub speaker: String,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub portrait: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub metadata: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualCondition {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    pub variable: String,
    pub equals: VisualStateValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VisualStateOperation {
    SetVariable {
        key: String,
        value: VisualStateValue,
    },
    SetLayerState {
        layer_id: String,
        state: String,
    },
    SelectEntity {
        entity_id: String,
    },
    SetEntityFlags {
        entity_id: String,
        flags: Vec<String>,
    },
    SetEntityMetadata {
        entity_id: String,
        metadata: Vec<(String, String)>,
    },
    SetEntityVisibility {
        entity_id: String,
        visible: bool,
    },
    AdvanceDialogueAndSetLayer {
        target: usize,
        layer_id: String,
        state: String,
    },
    TriggerLayerTransition {
        layer_id: String,
        input: String,
    },
    IncrementVariable {
        key: String,
        amount: i64,
    },
    ClearVariable {
        key: String,
    },
    AddInventory {
        item: VisualInventoryItem,
    },
    RemoveInventory {
        item_id: String,
        count: u32,
    },
    SetStat {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        owner_id: Option<String>,
        key: String,
        value: VisualStateValue,
    },
    AdjustStat {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        owner_id: Option<String>,
        key: String,
        amount: i64,
    },
    AdvanceQuest {
        quest_id: String,
        stage: i64,
    },
    CompleteQuest {
        quest_id: String,
    },
    AppendQuestJournal {
        quest_id: String,
        text: String,
    },
    AdjustRelationship {
        source_id: String,
        target_id: String,
        kind: String,
        amount: i64,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualRpgState {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inventory: Vec<VisualInventoryItem>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stats: Vec<VisualStat>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub quests: Vec<VisualQuest>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relationships: Vec<VisualRelationship>,
}

impl VisualRpgState {
    pub fn is_empty(&self) -> bool {
        self.inventory.is_empty()
            && self.stats.is_empty()
            && self.quests.is_empty()
            && self.relationships.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualInventoryItem {
    pub item_id: String,
    pub label: String,
    pub count: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub metadata: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualStat {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<String>,
    pub key: String,
    pub value: VisualStateValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualQuest {
    pub quest_id: String,
    pub label: String,
    pub stage: i64,
    #[serde(default)]
    pub completed: bool,
    #[serde(default)]
    pub journal: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualRelationship {
    pub source_id: String,
    pub target_id: String,
    pub kind: String,
    pub value: i64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub metadata: Vec<(String, String)>,
}

fn is_empty_rpg_state(state: &VisualRpgState) -> bool {
    state.is_empty()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualModeDescriptor {
    pub mode_id: String,
    pub label: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub scene_profile: Option<String>,
    #[serde(default)]
    pub allowed_actions: Vec<String>,
    #[serde(default)]
    pub default_transition: Option<String>,
    #[serde(default, skip_serializing_if = "VisualModeLifecycle::is_empty")]
    pub lifecycle: VisualModeLifecycle,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub input_map: Vec<VisualInputBinding>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualModeLifecycle {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enter_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub update_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_status: Option<String>,
}

impl VisualModeLifecycle {
    pub fn is_empty(&self) -> bool {
        self.enter_status.is_none() && self.update_status.is_none() && self.exit_status.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualInputBinding {
    pub input: String,
    pub action: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<VisualCondition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualLayerState {
    pub layer_id: String,
    pub state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub input_map: Vec<VisualInputBinding>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub transitions: Vec<VisualLayerTransition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualLayerTransition {
    pub input: String,
    pub target_state: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<VisualCondition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualLayerTransitionReport {
    pub layer_id: String,
    pub input: String,
    pub from_state: String,
    pub target_state: String,
    pub result: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualRuntimeEvent {
    pub kind: String,
    pub detail: String,
}

impl Default for VisualModeDescriptor {
    fn default() -> Self {
        default_scene_mode()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualScene {
    pub title: String,
    pub background: String,
    pub width: usize,
    pub height: usize,
    #[serde(default, skip_serializing_if = "is_default_scene_mode")]
    pub mode: VisualModeDescriptor,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layers: Vec<VisualLayerState>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variables: Vec<VisualStateEntry>,
    #[serde(default, skip_serializing_if = "is_empty_rpg_state")]
    pub rpg: VisualRpgState,
    pub entities: Vec<VisualEntity>,
    pub dialogue_speaker: String,
    pub dialogue: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dialogue_lines: Vec<VisualDialogueLine>,
    pub choices: Vec<SceneAction>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualSpriteDefinition {
    pub id: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualSpriteManifest {
    pub sprites: Vec<VisualSpriteDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualResolvedSprite {
    pub id: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualSpriteManifestStatus {
    pub manifest_path: Option<String>,
    pub sprites: Vec<VisualResolvedSprite>,
    pub warnings: Vec<String>,
}

impl VisualSpriteManifestStatus {
    pub fn missing(path: impl AsRef<Path>) -> Self {
        Self {
            manifest_path: Some(path.as_ref().display().to_string()),
            sprites: Vec::new(),
            warnings: vec![format!(
                "sprite manifest not found at {}",
                path.as_ref().display()
            )],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VisualRenderLayer {
    Background,
    Tile,
    Entity,
    Selection,
    Dialogue,
    Debug,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualRenderTile {
    pub position: VisualPosition,
    pub sprite: String,
    pub layer: VisualRenderLayer,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualRenderEntity {
    pub id: String,
    pub kind: VisualEntityKind,
    pub label: String,
    pub position: VisualPosition,
    pub sprite: String,
    pub layer: VisualRenderLayer,
    pub selected: bool,
    pub state_flags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualRenderSnapshot {
    pub generation: u64,
    pub view: VisualView,
    pub scene_source: VisualSceneSource,
    pub active_mode: VisualModeDescriptor,
    pub active_layers: Vec<VisualLayerState>,
    pub selected_entity_mode: Option<String>,
    pub variables: Vec<VisualStateEntry>,
    pub rpg: VisualRpgState,
    pub title: String,
    pub background: String,
    pub width: usize,
    pub height: usize,
    pub selected_entity_id: Option<String>,
    pub selected_choice: usize,
    pub tiles: Vec<VisualRenderTile>,
    pub entities: Vec<VisualRenderEntity>,
    pub dialogue_speaker: String,
    pub dialogue: String,
    pub dialogue_index: Option<usize>,
    pub dialogue_history: Vec<VisualDialogueLine>,
    pub status: String,
    pub choices: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualSceneDebugReport {
    pub scene_path: String,
    pub load_status: String,
    pub reload_count: u64,
    pub last_error: Option<String>,
    pub action_base_dir: String,
    pub active_mode_id: String,
    pub active_mode_label: String,
    pub active_mode_description: String,
    pub active_mode_scene_profile: Option<String>,
    pub active_mode_allowed_actions: Vec<String>,
    pub active_mode_default_transition: Option<String>,
    pub active_mode_lifecycle: VisualModeLifecycle,
    pub active_mode_input_map: Vec<VisualInputBinding>,
    pub active_layers: Vec<VisualLayerState>,
    pub last_input_layer: Option<String>,
    pub last_layer_transition: Option<VisualLayerTransitionReport>,
    pub transition_history: Vec<VisualRuntimeEvent>,
    pub selected_entity_mode: Option<String>,
    pub variables: Vec<VisualStateEntry>,
    pub rpg: VisualRpgState,
    pub title: String,
    pub background: String,
    pub width: usize,
    pub height: usize,
    pub entity_count: usize,
    pub choice_count: usize,
    pub selected_entity_id: Option<String>,
    pub selected_entity_label: Option<String>,
    pub selected_entity_kind: Option<String>,
    pub selected_entity_sprite: Option<String>,
    pub selected_entity_flags: Vec<String>,
    pub selected_entity_metadata: Vec<(String, String)>,
    pub dialogue_index: Option<usize>,
    pub dialogue_line_count: usize,
    pub dialogue_history: Vec<VisualDialogueLine>,
    pub selected_choice: usize,
    pub selected_choice_label: Option<String>,
    pub selected_choice_kind: Option<String>,
    pub selected_choice_detail: Option<String>,
    pub selected_choice_enabled: bool,
    pub selected_choice_guard_detail: Option<String>,
    pub pending_action_kind: Option<String>,
    pub pending_action_detail: Option<String>,
    pub process_state: Option<VisualProcessState>,
    pub last_story_state_action: Option<String>,
    pub last_story_state_path: Option<String>,
    pub last_patch_transport: Option<String>,
    pub last_patch_source_pane_id: Option<usize>,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualScenePatch {
    pub scene_patch_version: u32,
    #[serde(default)]
    pub updates: Vec<VisualSceneEntityPatch>,
    #[serde(default)]
    pub variables: Vec<VisualStateEntry>,
    #[serde(default)]
    pub selected_entity_id: Option<String>,
    #[serde(default)]
    pub process_state: Option<VisualProcessState>,
    #[serde(default)]
    pub status: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualProcessState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<String>,
    pub phase: VisualProcessPhase,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisualProcessPhase {
    Queued,
    Running,
    Blocked,
    Succeeded,
    Failed,
}

impl VisualProcessPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Blocked => "blocked",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualStoryState {
    pub story_state_version: u32,
    #[serde(default)]
    pub variables: Vec<VisualStateEntry>,
    #[serde(default, skip_serializing_if = "is_empty_rpg_state")]
    pub rpg: VisualRpgState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dialogue_index: Option<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dialogue_history: Vec<VisualDialogueLine>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualSceneEntityPatch {
    pub entity_id: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub position: Option<VisualPosition>,
    #[serde(default)]
    pub sprite: Option<String>,
    #[serde(default)]
    pub visible: Option<bool>,
    #[serde(default)]
    pub state_flags: Option<Vec<String>>,
    #[serde(default)]
    pub metadata: Option<Vec<(String, String)>>,
}

fn default_true() -> bool {
    true
}

fn is_true(value: &bool) -> bool {
    *value
}

fn default_scene_mode() -> VisualModeDescriptor {
    VisualModeDescriptor {
        mode_id: "workspace".to_string(),
        label: "Workspace".to_string(),
        description: "Default Scene Mode workspace context".to_string(),
        scene_profile: Some("scene".to_string()),
        allowed_actions: vec![
            "Inspect".to_string(),
            "OpenFile".to_string(),
            "RunCommand".to_string(),
            "Navigate".to_string(),
        ],
        default_transition: None,
        lifecycle: VisualModeLifecycle::default(),
        input_map: Vec::new(),
    }
}

fn is_default_scene_mode(mode: &VisualModeDescriptor) -> bool {
    mode == &default_scene_mode()
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum VisualStateEntryError {
    EmptyKey,
    DuplicateKey(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VisualDialogueLineError {
    EmptySpeaker { index: usize },
    EmptyText { index: usize },
}

fn validate_state_entries(entries: &[VisualStateEntry]) -> Result<(), VisualStateEntryError> {
    let mut keys = HashSet::new();
    for entry in entries {
        if entry.key.trim().is_empty() {
            return Err(VisualStateEntryError::EmptyKey);
        }
        if !keys.insert(entry.key.as_str()) {
            return Err(VisualStateEntryError::DuplicateKey(entry.key.clone()));
        }
    }
    Ok(())
}

fn validate_dialogue_lines(lines: &[VisualDialogueLine]) -> Result<(), VisualDialogueLineError> {
    for (index, line) in lines.iter().enumerate() {
        if line.speaker.trim().is_empty() {
            return Err(VisualDialogueLineError::EmptySpeaker { index });
        }
        if line.text.trim().is_empty() {
            return Err(VisualDialogueLineError::EmptyText { index });
        }
    }
    Ok(())
}

fn validate_layers(layers: &[VisualLayerState]) -> Result<(), VisualSceneError> {
    let mut layer_ids = HashSet::new();
    for layer in layers {
        if layer.layer_id.trim().is_empty() {
            return Err(VisualSceneError::EmptyLayerId);
        }
        if !layer_ids.insert(layer.layer_id.as_str()) {
            return Err(VisualSceneError::DuplicateLayerId(layer.layer_id.clone()));
        }
        if layer.state.trim().is_empty() {
            return Err(VisualSceneError::EmptyLayerState {
                layer_id: layer.layer_id.clone(),
            });
        }
        for binding in &layer.input_map {
            if binding.input.trim().is_empty() {
                return Err(VisualSceneError::EmptyLayerInputBindingInput {
                    layer_id: layer.layer_id.clone(),
                });
            }
            if binding.action.trim().is_empty() {
                return Err(VisualSceneError::EmptyLayerInputBindingAction {
                    layer_id: layer.layer_id.clone(),
                });
            }
            if !is_supported_mode_input(binding.input.trim()) {
                return Err(VisualSceneError::UnknownLayerInputBindingInput {
                    layer_id: layer.layer_id.clone(),
                    input: binding.input.clone(),
                });
            }
            if !is_supported_mode_input_action(binding.action.trim()) {
                return Err(VisualSceneError::UnknownLayerInputBindingAction {
                    layer_id: layer.layer_id.clone(),
                    action: binding.action.clone(),
                });
            }
            if binding
                .conditions
                .iter()
                .any(|condition| condition.variable.trim().is_empty())
            {
                return Err(VisualSceneError::EmptyLayerInputBindingConditionVariable {
                    layer_id: layer.layer_id.clone(),
                });
            }
        }
        for transition in &layer.transitions {
            if transition.input.trim().is_empty() {
                return Err(VisualSceneError::EmptyLayerTransitionInput {
                    layer_id: layer.layer_id.clone(),
                });
            }
            if transition.target_state.trim().is_empty() {
                return Err(VisualSceneError::EmptyLayerTransitionTarget {
                    layer_id: layer.layer_id.clone(),
                });
            }
            if !is_supported_mode_input(transition.input.trim()) {
                return Err(VisualSceneError::UnknownLayerTransitionInput {
                    layer_id: layer.layer_id.clone(),
                    input: transition.input.clone(),
                });
            }
            if transition
                .conditions
                .iter()
                .any(|condition| condition.variable.trim().is_empty())
            {
                return Err(VisualSceneError::EmptyLayerTransitionConditionVariable {
                    layer_id: layer.layer_id.clone(),
                });
            }
        }
    }
    Ok(())
}

fn validate_rpg_state(state: &VisualRpgState) -> Result<(), VisualSceneError> {
    let mut item_ids = HashSet::new();
    for item in &state.inventory {
        if item.item_id.trim().is_empty() {
            return Err(VisualSceneError::EmptyInventoryItemId);
        }
        if item.label.trim().is_empty() {
            return Err(VisualSceneError::EmptyInventoryLabel {
                item_id: item.item_id.clone(),
            });
        }
        if !item_ids.insert(item.item_id.as_str()) {
            return Err(VisualSceneError::DuplicateInventoryItemId(
                item.item_id.clone(),
            ));
        }
    }

    let mut stats = HashSet::new();
    for stat in &state.stats {
        if stat.key.trim().is_empty() {
            return Err(VisualSceneError::EmptyStatKey);
        }
        if matches!(stat.owner_id.as_ref(), Some(owner_id) if owner_id.trim().is_empty()) {
            return Err(VisualSceneError::EmptyStatOwnerId);
        }
        let key = format!(
            "{}:{}",
            stat.owner_id.as_deref().unwrap_or("scene"),
            stat.key
        );
        if !stats.insert(key.clone()) {
            return Err(VisualSceneError::DuplicateStatKey(key));
        }
    }

    let mut quest_ids = HashSet::new();
    for quest in &state.quests {
        if quest.quest_id.trim().is_empty() {
            return Err(VisualSceneError::EmptyQuestId);
        }
        if quest.label.trim().is_empty() {
            return Err(VisualSceneError::EmptyQuestLabel {
                quest_id: quest.quest_id.clone(),
            });
        }
        if !quest_ids.insert(quest.quest_id.as_str()) {
            return Err(VisualSceneError::DuplicateQuestId(quest.quest_id.clone()));
        }
    }

    let mut relationships = HashSet::new();
    for relationship in &state.relationships {
        if relationship.source_id.trim().is_empty() {
            return Err(VisualSceneError::EmptyRelationshipSourceId);
        }
        if relationship.target_id.trim().is_empty() {
            return Err(VisualSceneError::EmptyRelationshipTargetId);
        }
        if relationship.kind.trim().is_empty() {
            return Err(VisualSceneError::EmptyRelationshipKind);
        }
        let key = format!(
            "{}:{}:{}",
            relationship.source_id, relationship.target_id, relationship.kind
        );
        if !relationships.insert(key.clone()) {
            return Err(VisualSceneError::DuplicateRelationship(key));
        }
    }

    Ok(())
}

fn validate_state_operations(
    label: &str,
    operations: &[VisualStateOperation],
    entities: &[VisualEntity],
    dialogue_lines: &[VisualDialogueLine],
    layers: &[VisualLayerState],
    rpg: &VisualRpgState,
) -> Result<(), VisualSceneError> {
    if operations.is_empty() {
        return Err(VisualSceneError::EmptyResolveOperations {
            label: label.to_string(),
        });
    }
    for operation in operations {
        match operation {
            VisualStateOperation::SetVariable { key, .. }
            | VisualStateOperation::IncrementVariable { key, .. }
            | VisualStateOperation::ClearVariable { key } => {
                if key.trim().is_empty() {
                    return Err(VisualSceneError::EmptyResolveOperationKey {
                        label: label.to_string(),
                    });
                }
            }
            VisualStateOperation::SetLayerState { layer_id, state } => {
                if layer_id.trim().is_empty() || state.trim().is_empty() {
                    return Err(VisualSceneError::EmptyResolveOperationKey {
                        label: label.to_string(),
                    });
                }
                if !layers.iter().any(|layer| layer.layer_id == *layer_id) {
                    return Err(VisualSceneError::UnknownLayer {
                        label: label.to_string(),
                        layer_id: layer_id.clone(),
                    });
                }
            }
            VisualStateOperation::SelectEntity { entity_id }
            | VisualStateOperation::SetEntityVisibility { entity_id, .. } => {
                if entity_id.trim().is_empty() {
                    return Err(VisualSceneError::EmptyResolveOperationKey {
                        label: label.to_string(),
                    });
                }
                if !entities.iter().any(|entity| entity.id == *entity_id) {
                    return Err(VisualSceneError::UnknownEntity {
                        label: label.to_string(),
                        entity_id: entity_id.clone(),
                    });
                }
            }
            VisualStateOperation::SetEntityFlags { entity_id, flags } => {
                if entity_id.trim().is_empty() || flags.iter().any(|flag| flag.trim().is_empty()) {
                    return Err(VisualSceneError::EmptyResolveOperationKey {
                        label: label.to_string(),
                    });
                }
                if !entities.iter().any(|entity| entity.id == *entity_id) {
                    return Err(VisualSceneError::UnknownEntity {
                        label: label.to_string(),
                        entity_id: entity_id.clone(),
                    });
                }
            }
            VisualStateOperation::SetEntityMetadata {
                entity_id,
                metadata,
            } => {
                if entity_id.trim().is_empty()
                    || metadata.iter().any(|(key, _)| key.trim().is_empty())
                {
                    return Err(VisualSceneError::EmptyResolveOperationKey {
                        label: label.to_string(),
                    });
                }
                if !entities.iter().any(|entity| entity.id == *entity_id) {
                    return Err(VisualSceneError::UnknownEntity {
                        label: label.to_string(),
                        entity_id: entity_id.clone(),
                    });
                }
            }
            VisualStateOperation::AdvanceDialogueAndSetLayer {
                target,
                layer_id,
                state,
            } => {
                if layer_id.trim().is_empty() || state.trim().is_empty() {
                    return Err(VisualSceneError::EmptyResolveOperationKey {
                        label: label.to_string(),
                    });
                }
                if *target >= dialogue_lines.len() {
                    return Err(VisualSceneError::DialogueTargetOutOfBounds {
                        label: label.to_string(),
                        target: *target,
                    });
                }
                if !layers.iter().any(|layer| layer.layer_id == *layer_id) {
                    return Err(VisualSceneError::UnknownLayer {
                        label: label.to_string(),
                        layer_id: layer_id.clone(),
                    });
                }
            }
            VisualStateOperation::TriggerLayerTransition { layer_id, input } => {
                if layer_id.trim().is_empty() || input.trim().is_empty() {
                    return Err(VisualSceneError::EmptyResolveOperationKey {
                        label: label.to_string(),
                    });
                }
                let Some(layer) = layers.iter().find(|layer| layer.layer_id == *layer_id) else {
                    return Err(VisualSceneError::UnknownLayer {
                        label: label.to_string(),
                        layer_id: layer_id.clone(),
                    });
                };
                if !layer
                    .transitions
                    .iter()
                    .any(|transition| transition.input == *input)
                {
                    return Err(VisualSceneError::UnknownLayerTransition {
                        label: label.to_string(),
                        layer_id: layer_id.clone(),
                        input: input.clone(),
                    });
                }
            }
            VisualStateOperation::AddInventory { item } => {
                validate_rpg_state(&VisualRpgState {
                    inventory: vec![item.clone()],
                    stats: Vec::new(),
                    quests: Vec::new(),
                    relationships: Vec::new(),
                })?;
            }
            VisualStateOperation::RemoveInventory { item_id, .. } => {
                if item_id.trim().is_empty() {
                    return Err(VisualSceneError::EmptyResolveOperationKey {
                        label: label.to_string(),
                    });
                }
                if !rpg.inventory.iter().any(|item| item.item_id == *item_id) {
                    return Err(VisualSceneError::UnknownInventoryItem {
                        label: label.to_string(),
                        item_id: item_id.clone(),
                    });
                }
            }
            VisualStateOperation::SetStat { key, .. }
            | VisualStateOperation::AdjustStat { key, .. } => {
                if key.trim().is_empty() {
                    return Err(VisualSceneError::EmptyResolveOperationKey {
                        label: label.to_string(),
                    });
                }
            }
            VisualStateOperation::AdvanceQuest { quest_id, .. }
            | VisualStateOperation::CompleteQuest { quest_id }
            | VisualStateOperation::AppendQuestJournal { quest_id, .. } => {
                if quest_id.trim().is_empty() {
                    return Err(VisualSceneError::EmptyResolveOperationKey {
                        label: label.to_string(),
                    });
                }
                if !rpg.quests.iter().any(|quest| quest.quest_id == *quest_id) {
                    return Err(VisualSceneError::UnknownQuest {
                        label: label.to_string(),
                        quest_id: quest_id.clone(),
                    });
                }
            }
            VisualStateOperation::AdjustRelationship {
                source_id,
                target_id,
                kind,
                ..
            } => {
                if source_id.trim().is_empty()
                    || target_id.trim().is_empty()
                    || kind.trim().is_empty()
                {
                    return Err(VisualSceneError::EmptyResolveOperationKey {
                        label: label.to_string(),
                    });
                }
                if !rpg.relationships.iter().any(|relationship| {
                    relationship.source_id == *source_id
                        && relationship.target_id == *target_id
                        && relationship.kind == *kind
                }) {
                    return Err(VisualSceneError::UnknownRelationship {
                        label: label.to_string(),
                        relationship: relationship_key(source_id, target_id, kind),
                    });
                }
            }
        }
    }
    Ok(())
}

fn relationship_key(source_id: &str, target_id: &str, kind: &str) -> String {
    format!("{source_id}:{target_id}:{kind}")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VisualSceneLoadStatus {
    Bundled,
    Loaded,
    ReloadFailed,
    Invalid,
}

impl VisualSceneLoadStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bundled => "bundled",
            Self::Loaded => "loaded",
            Self::ReloadFailed => "reload_failed",
            Self::Invalid => "invalid",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualSceneSource {
    pub scene_path: String,
    pub load_status: VisualSceneLoadStatus,
    pub reload_count: u64,
    pub last_error: Option<String>,
}

impl VisualSceneSource {
    pub fn new(
        scene_path: impl Into<String>,
        load_status: VisualSceneLoadStatus,
        reload_count: u64,
    ) -> Self {
        Self {
            scene_path: scene_path.into(),
            load_status,
            reload_count,
            last_error: None,
        }
    }

    pub fn invalid(scene_path: impl Into<String>, reload_count: u64, error: String) -> Self {
        Self {
            scene_path: scene_path.into(),
            load_status: VisualSceneLoadStatus::Invalid,
            reload_count,
            last_error: Some(error),
        }
    }

    pub fn reload_failed(&self, reload_count: u64, error: String) -> Self {
        Self {
            scene_path: self.scene_path.clone(),
            load_status: VisualSceneLoadStatus::ReloadFailed,
            reload_count,
            last_error: Some(error),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualInput {
    Close,
    Reload,
    ToggleDebug,
    Activate,
    Next,
    Previous,
    Other,
}

impl VisualInput {
    fn binding_key(self) -> &'static str {
        match self {
            Self::Close => "close",
            Self::Reload => "reload",
            Self::ToggleDebug => "toggle_debug",
            Self::Activate => "activate",
            Self::Next => "next",
            Self::Previous => "previous",
            Self::Other => "other",
        }
    }
}

fn is_supported_mode_input(input: &str) -> bool {
    matches!(
        input,
        "close" | "reload" | "toggle_debug" | "activate" | "next" | "previous" | "other"
    )
}

fn is_supported_mode_input_action(action: &str) -> bool {
    matches!(
        action,
        "close"
            | "reload"
            | "toggle_debug"
            | "activate_choice"
            | "select_next"
            | "select_previous"
            | "run_update_hooks"
            | "run_exit_hooks"
            | "export_story_state"
            | "import_story_state"
            | "ignore"
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualModeOutcome {
    Continue,
    Exit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VisualActionRequest {
    OpenFile {
        path: PathBuf,
    },
    RunCommand {
        argv: Vec<String>,
        cwd: Option<PathBuf>,
        target: RunCommandTarget,
    },
    Navigate {
        target: String,
    },
    ExportStoryState {
        path: PathBuf,
    },
    ImportStoryState {
        path: PathBuf,
    },
}

pub trait VisualMode {
    fn generation(&self) -> u64;
    fn render_snapshot(&self) -> VisualRenderSnapshot;
    fn render_text_frame(&self, cols: usize, rows: usize) -> String;
    fn handle_input(&mut self, input: VisualInput) -> VisualModeOutcome;
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VisualSceneError {
    #[error("scene dimensions must be non-zero")]
    EmptyScene,
    #[error("duplicate entity id `{0}`")]
    DuplicateEntityId(String),
    #[error("entity `{id}` is outside scene bounds at {x},{y}")]
    EntityOutOfBounds { id: String, x: usize, y: usize },
    #[error("RunCommand action `{label}` must provide a non-empty argv array")]
    EmptyRunCommand { label: String },
    #[error("RunCommand action `{label}` has an empty cwd")]
    EmptyRunCommandCwd { label: String },
    #[error("StoryState action `{label}` must provide a non-empty path")]
    EmptyStoryStatePath { label: String },
    #[error("Resolve action `{label}` must include at least one operation")]
    EmptyResolveOperations { label: String },
    #[error("Resolve action `{label}` has an empty operation key")]
    EmptyResolveOperationKey { label: String },
    #[error("Resolve action `{label}` references unknown inventory item `{item_id}`")]
    UnknownInventoryItem { label: String, item_id: String },
    #[error("Resolve action `{label}` references unknown stat `{key}`")]
    UnknownStat { label: String, key: String },
    #[error("Resolve action `{label}` references non-numeric stat `{key}`")]
    NonNumericStat { label: String, key: String },
    #[error("Resolve action `{label}` references unknown quest `{quest_id}`")]
    UnknownQuest { label: String, quest_id: String },
    #[error("Resolve action `{label}` references unknown relationship `{relationship}`")]
    UnknownRelationship { label: String, relationship: String },
    #[error("Resolve action `{label}` references unknown layer `{layer_id}`")]
    UnknownLayer { label: String, layer_id: String },
    #[error("Resolve action `{label}` references unknown entity `{entity_id}`")]
    UnknownEntity { label: String, entity_id: String },
    #[error("Resolve action `{label}` references unknown layer transition `{layer_id}:{input}`")]
    UnknownLayerTransition {
        label: String,
        layer_id: String,
        input: String,
    },
    #[error("Resolve action `{label}` blocked by layer transition guard `{layer_id}:{input}`")]
    LayerTransitionGuardFailed {
        label: String,
        layer_id: String,
        input: String,
    },
    #[error("scene mode id must be non-empty")]
    EmptyModeId,
    #[error("scene mode label must be non-empty")]
    EmptyModeLabel,
    #[error("scene mode allowed action must be non-empty")]
    EmptyModeAllowedAction,
    #[error("scene mode lifecycle status must be non-empty when provided")]
    EmptyModeLifecycleStatus,
    #[error("scene mode input binding input must be non-empty")]
    EmptyModeInputBindingInput,
    #[error("scene mode input binding action must be non-empty")]
    EmptyModeInputBindingAction,
    #[error("unknown scene mode input binding input `{0}`")]
    UnknownModeInputBindingInput(String),
    #[error("unknown scene mode input binding action `{0}`")]
    UnknownModeInputBindingAction(String),
    #[error("scene mode input binding condition variable must be non-empty")]
    EmptyModeInputBindingConditionVariable,
    #[error("scene layer id must be non-empty")]
    EmptyLayerId,
    #[error("duplicate scene layer id `{0}`")]
    DuplicateLayerId(String),
    #[error("scene layer `{layer_id}` state must be non-empty")]
    EmptyLayerState { layer_id: String },
    #[error("scene layer `{layer_id}` input binding input must be non-empty")]
    EmptyLayerInputBindingInput { layer_id: String },
    #[error("scene layer `{layer_id}` input binding action must be non-empty")]
    EmptyLayerInputBindingAction { layer_id: String },
    #[error("scene layer `{layer_id}` has unknown input binding input `{input}`")]
    UnknownLayerInputBindingInput { layer_id: String, input: String },
    #[error("scene layer `{layer_id}` has unknown input binding action `{action}`")]
    UnknownLayerInputBindingAction { layer_id: String, action: String },
    #[error("scene layer `{layer_id}` input binding condition variable must be non-empty")]
    EmptyLayerInputBindingConditionVariable { layer_id: String },
    #[error("scene layer `{layer_id}` transition input must be non-empty")]
    EmptyLayerTransitionInput { layer_id: String },
    #[error("scene layer `{layer_id}` transition target state must be non-empty")]
    EmptyLayerTransitionTarget { layer_id: String },
    #[error("scene layer `{layer_id}` has unknown transition input `{input}`")]
    UnknownLayerTransitionInput { layer_id: String, input: String },
    #[error("scene layer `{layer_id}` transition condition variable must be non-empty")]
    EmptyLayerTransitionConditionVariable { layer_id: String },
    #[error("scene variable key must be non-empty")]
    EmptyVariableKey,
    #[error("duplicate scene variable key `{0}`")]
    DuplicateVariableKey(String),
    #[error("choice action `{label}` has an empty condition variable")]
    EmptyConditionVariable { label: String },
    #[error("dialogue line {index} must provide a non-empty speaker")]
    EmptyDialogueSpeaker { index: usize },
    #[error("dialogue line {index} must provide non-empty text")]
    EmptyDialogueText { index: usize },
    #[error("choice action `{label}` references missing dialogue line {target}")]
    DialogueTargetOutOfBounds { label: String, target: usize },
    #[error("inventory item id must be non-empty")]
    EmptyInventoryItemId,
    #[error("inventory item `{item_id}` label must be non-empty")]
    EmptyInventoryLabel { item_id: String },
    #[error("duplicate inventory item id `{0}`")]
    DuplicateInventoryItemId(String),
    #[error("stat key must be non-empty")]
    EmptyStatKey,
    #[error("stat owner id must be non-empty when provided")]
    EmptyStatOwnerId,
    #[error("duplicate stat `{0}`")]
    DuplicateStatKey(String),
    #[error("quest id must be non-empty")]
    EmptyQuestId,
    #[error("quest `{quest_id}` label must be non-empty")]
    EmptyQuestLabel { quest_id: String },
    #[error("duplicate quest id `{0}`")]
    DuplicateQuestId(String),
    #[error("relationship source id must be non-empty")]
    EmptyRelationshipSourceId,
    #[error("relationship target id must be non-empty")]
    EmptyRelationshipTargetId,
    #[error("relationship kind must be non-empty")]
    EmptyRelationshipKind,
    #[error("duplicate relationship `{0}`")]
    DuplicateRelationship(String),
    #[error("scene json error: {0}")]
    Json(String),
    #[error("scene file error for `{path}`: {message}")]
    File { path: String, message: String },
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VisualSpriteManifestError {
    #[error("sprite manifest json error: {0}")]
    Json(String),
    #[error("sprite manifest file error for `{path}`: {message}")]
    File { path: String, message: String },
    #[error("sprite id must be non-empty")]
    EmptySpriteId,
    #[error("sprite path for `{id}` must be non-empty")]
    EmptySpritePath { id: String },
    #[error("duplicate sprite id `{0}`")]
    DuplicateSpriteId(String),
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VisualScenePatchError {
    #[error("scene patch json error: {0}")]
    Json(String),
    #[error("scene patch file error for `{path}`: {message}")]
    File { path: String, message: String },
    #[error("unsupported scene patch version `{0}`")]
    UnsupportedVersion(u32),
    #[error("scene patch must contain at least one entity update or a status")]
    EmptyPatch,
    #[error("scene patch entity id must be non-empty")]
    EmptyEntityId,
    #[error("scene patch selected entity id must be non-empty")]
    EmptySelectedEntityId,
    #[error("scene patch references unknown entity id `{0}`")]
    UnknownEntityId(String),
    #[error("scene patch entity `{entity_id}` position {x},{y} is outside scene bounds")]
    EntityOutOfBounds {
        entity_id: String,
        x: usize,
        y: usize,
    },
    #[error("scene patch entity `{entity_id}` label must be non-empty")]
    EmptyLabel { entity_id: String },
    #[error("scene patch entity `{entity_id}` sprite must be non-empty")]
    EmptySprite { entity_id: String },
    #[error("scene patch metadata for `{entity_id}` contains an empty key")]
    EmptyMetadataKey { entity_id: String },
    #[error("scene patch variable key must be non-empty")]
    EmptyVariableKey,
    #[error("scene patch contains duplicate variable key `{0}`")]
    DuplicateVariableKey(String),
    #[error("scene patch process entity id must be non-empty")]
    EmptyProcessEntityId,
    #[error("scene patch process command must be non-empty")]
    EmptyProcessCommand,
    #[error("scene patch process message must be non-empty")]
    EmptyProcessMessage,
    #[error("scene patch process references unknown entity id `{0}`")]
    UnknownProcessEntityId(String),
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VisualStoryStateError {
    #[error("story state json error: {0}")]
    Json(String),
    #[error("story state file error for `{path}`: {message}")]
    File { path: String, message: String },
    #[error("unsupported story state version `{0}`")]
    UnsupportedVersion(u32),
    #[error("story state variable key must be non-empty")]
    EmptyVariableKey,
    #[error("story state contains duplicate variable key `{0}`")]
    DuplicateVariableKey(String),
    #[error("story state dialogue line {index} must provide a non-empty speaker")]
    EmptyDialogueSpeaker { index: usize },
    #[error("story state dialogue line {index} must provide non-empty text")]
    EmptyDialogueText { index: usize },
    #[error("story state RPG state is invalid: {0}")]
    InvalidRpgState(String),
    #[error("story state references missing dialogue line {target}")]
    DialogueIndexOutOfBounds { target: usize },
}

impl VisualStoryState {
    pub const VERSION: u32 = 1;

    pub fn from_json(json: &str) -> Result<Self, VisualStoryStateError> {
        let state: Self = serde_json::from_str(json)
            .map_err(|err| VisualStoryStateError::Json(err.to_string()))?;
        state.validate()?;
        Ok(state)
    }

    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self, VisualStoryStateError> {
        let path = path.as_ref();
        let json = std::fs::read_to_string(path).map_err(|err| VisualStoryStateError::File {
            path: path.display().to_string(),
            message: err.to_string(),
        })?;
        Self::from_json(&json)
    }

    pub fn validate(&self) -> Result<(), VisualStoryStateError> {
        if self.story_state_version != Self::VERSION {
            return Err(VisualStoryStateError::UnsupportedVersion(
                self.story_state_version,
            ));
        }
        validate_state_entries(&self.variables).map_err(|err| match err {
            VisualStateEntryError::EmptyKey => VisualStoryStateError::EmptyVariableKey,
            VisualStateEntryError::DuplicateKey(key) => {
                VisualStoryStateError::DuplicateVariableKey(key)
            }
        })?;
        validate_rpg_state(&self.rpg)
            .map_err(|err| VisualStoryStateError::InvalidRpgState(err.to_string()))?;
        validate_dialogue_lines(&self.dialogue_history).map_err(|err| match err {
            VisualDialogueLineError::EmptySpeaker { index } => {
                VisualStoryStateError::EmptyDialogueSpeaker { index }
            }
            VisualDialogueLineError::EmptyText { index } => {
                VisualStoryStateError::EmptyDialogueText { index }
            }
        })?;
        Ok(())
    }
}

impl VisualScenePatch {
    pub const VERSION: u32 = 1;

    pub fn from_json(json: &str) -> Result<Self, VisualScenePatchError> {
        let patch: Self = serde_json::from_str(json)
            .map_err(|err| VisualScenePatchError::Json(err.to_string()))?;
        patch.validate()?;
        Ok(patch)
    }

    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self, VisualScenePatchError> {
        let path = path.as_ref();
        let json = std::fs::read_to_string(path).map_err(|err| VisualScenePatchError::File {
            path: path.display().to_string(),
            message: err.to_string(),
        })?;
        Self::from_json(&json)
    }

    pub fn validate(&self) -> Result<(), VisualScenePatchError> {
        if self.scene_patch_version != Self::VERSION {
            return Err(VisualScenePatchError::UnsupportedVersion(
                self.scene_patch_version,
            ));
        }
        if self.updates.is_empty()
            && self.variables.is_empty()
            && self.process_state.is_none()
            && self.status.is_none()
        {
            return Err(VisualScenePatchError::EmptyPatch);
        }
        validate_state_entries(&self.variables).map_err(|err| match err {
            VisualStateEntryError::EmptyKey => VisualScenePatchError::EmptyVariableKey,
            VisualStateEntryError::DuplicateKey(key) => {
                VisualScenePatchError::DuplicateVariableKey(key)
            }
        })?;
        for update in &self.updates {
            if update.entity_id.trim().is_empty() {
                return Err(VisualScenePatchError::EmptyEntityId);
            }
            if matches!(update.label.as_ref(), Some(label) if label.trim().is_empty()) {
                return Err(VisualScenePatchError::EmptyLabel {
                    entity_id: update.entity_id.clone(),
                });
            }
            if matches!(update.sprite.as_ref(), Some(sprite) if sprite.trim().is_empty()) {
                return Err(VisualScenePatchError::EmptySprite {
                    entity_id: update.entity_id.clone(),
                });
            }
            if let Some(metadata) = &update.metadata {
                if metadata.iter().any(|(key, _)| key.trim().is_empty()) {
                    return Err(VisualScenePatchError::EmptyMetadataKey {
                        entity_id: update.entity_id.clone(),
                    });
                }
            }
        }
        if matches!(self.selected_entity_id.as_ref(), Some(id) if id.trim().is_empty()) {
            return Err(VisualScenePatchError::EmptySelectedEntityId);
        }
        if let Some(process_state) = &self.process_state {
            if matches!(process_state.entity_id.as_ref(), Some(id) if id.trim().is_empty()) {
                return Err(VisualScenePatchError::EmptyProcessEntityId);
            }
            if matches!(process_state.command.as_ref(), Some(command) if command.trim().is_empty())
            {
                return Err(VisualScenePatchError::EmptyProcessCommand);
            }
            if matches!(process_state.message.as_ref(), Some(message) if message.trim().is_empty())
            {
                return Err(VisualScenePatchError::EmptyProcessMessage);
            }
        }
        Ok(())
    }
}

impl VisualSpriteManifest {
    pub fn from_json(json: &str) -> Result<Self, VisualSpriteManifestError> {
        let manifest: Self = serde_json::from_str(json)
            .map_err(|err| VisualSpriteManifestError::Json(err.to_string()))?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self, VisualSpriteManifestError> {
        let path = path.as_ref();
        let json =
            std::fs::read_to_string(path).map_err(|err| VisualSpriteManifestError::File {
                path: path.display().to_string(),
                message: err.to_string(),
            })?;
        Self::from_json(&json)
    }

    pub fn validate(&self) -> Result<(), VisualSpriteManifestError> {
        let mut ids = HashSet::new();
        for sprite in &self.sprites {
            if sprite.id.trim().is_empty() {
                return Err(VisualSpriteManifestError::EmptySpriteId);
            }
            if sprite.path.trim().is_empty() {
                return Err(VisualSpriteManifestError::EmptySpritePath {
                    id: sprite.id.clone(),
                });
            }
            if !ids.insert(sprite.id.as_str()) {
                return Err(VisualSpriteManifestError::DuplicateSpriteId(
                    sprite.id.clone(),
                ));
            }
        }
        Ok(())
    }

    pub fn resolve_against(&self, manifest_path: impl AsRef<Path>) -> VisualSpriteManifestStatus {
        let manifest_path = manifest_path.as_ref();
        let base_dir = manifest_path.parent().unwrap_or_else(|| Path::new(""));
        let sprites = self
            .sprites
            .iter()
            .map(|sprite| {
                let path = PathBuf::from(&sprite.path);
                let path = if path.is_absolute() {
                    path
                } else {
                    base_dir.join(path)
                };
                VisualResolvedSprite {
                    id: sprite.id.clone(),
                    path: path.display().to_string(),
                }
            })
            .collect();

        VisualSpriteManifestStatus {
            manifest_path: Some(manifest_path.display().to_string()),
            sprites,
            warnings: Vec::new(),
        }
    }
}

impl VisualScene {
    pub fn from_json(json: &str) -> Result<Self, VisualSceneError> {
        let scene: Self =
            serde_json::from_str(json).map_err(|err| VisualSceneError::Json(err.to_string()))?;
        scene.validate()?;
        Ok(scene)
    }

    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self, VisualSceneError> {
        let path = path.as_ref();
        let json = std::fs::read_to_string(path).map_err(|err| VisualSceneError::File {
            path: path.display().to_string(),
            message: err.to_string(),
        })?;
        Self::from_json(&json)
    }

    pub fn validate(&self) -> Result<(), VisualSceneError> {
        if self.width == 0 || self.height == 0 {
            return Err(VisualSceneError::EmptyScene);
        }
        if self.mode.mode_id.trim().is_empty() {
            return Err(VisualSceneError::EmptyModeId);
        }
        if self.mode.label.trim().is_empty() {
            return Err(VisualSceneError::EmptyModeLabel);
        }
        if self
            .mode
            .allowed_actions
            .iter()
            .any(|action| action.trim().is_empty())
        {
            return Err(VisualSceneError::EmptyModeAllowedAction);
        }
        if self
            .mode
            .lifecycle
            .enter_status
            .as_ref()
            .is_some_and(|status| status.trim().is_empty())
            || self
                .mode
                .lifecycle
                .update_status
                .as_ref()
                .is_some_and(|status| status.trim().is_empty())
            || self
                .mode
                .lifecycle
                .exit_status
                .as_ref()
                .is_some_and(|status| status.trim().is_empty())
        {
            return Err(VisualSceneError::EmptyModeLifecycleStatus);
        }
        for binding in &self.mode.input_map {
            if binding.input.trim().is_empty() {
                return Err(VisualSceneError::EmptyModeInputBindingInput);
            }
            if binding.action.trim().is_empty() {
                return Err(VisualSceneError::EmptyModeInputBindingAction);
            }
            if !is_supported_mode_input(binding.input.trim()) {
                return Err(VisualSceneError::UnknownModeInputBindingInput(
                    binding.input.clone(),
                ));
            }
            if !is_supported_mode_input_action(binding.action.trim()) {
                return Err(VisualSceneError::UnknownModeInputBindingAction(
                    binding.action.clone(),
                ));
            }
            if binding
                .conditions
                .iter()
                .any(|condition| condition.variable.trim().is_empty())
            {
                return Err(VisualSceneError::EmptyModeInputBindingConditionVariable);
            }
        }
        validate_layers(&self.layers)?;
        validate_state_entries(&self.variables).map_err(|err| match err {
            VisualStateEntryError::EmptyKey => VisualSceneError::EmptyVariableKey,
            VisualStateEntryError::DuplicateKey(key) => VisualSceneError::DuplicateVariableKey(key),
        })?;
        validate_rpg_state(&self.rpg)?;

        let mut ids = HashSet::new();
        for entity in &self.entities {
            if !ids.insert(entity.id.as_str()) {
                return Err(VisualSceneError::DuplicateEntityId(entity.id.clone()));
            }
            if entity.position.x >= self.width || entity.position.y >= self.height {
                return Err(VisualSceneError::EntityOutOfBounds {
                    id: entity.id.clone(),
                    x: entity.position.x,
                    y: entity.position.y,
                });
            }
        }

        validate_dialogue_lines(&self.dialogue_lines).map_err(|err| match err {
            VisualDialogueLineError::EmptySpeaker { index } => {
                VisualSceneError::EmptyDialogueSpeaker { index }
            }
            VisualDialogueLineError::EmptyText { index } => {
                VisualSceneError::EmptyDialogueText { index }
            }
        })?;

        for choice in &self.choices {
            for condition in &choice.conditions {
                if condition.variable.trim().is_empty() {
                    return Err(VisualSceneError::EmptyConditionVariable {
                        label: choice.label.clone(),
                    });
                }
            }
            if let SceneActionKind::RunCommand { argv, cwd, .. } = &choice.kind {
                if argv.is_empty() || argv.iter().any(|arg| arg.trim().is_empty()) {
                    return Err(VisualSceneError::EmptyRunCommand {
                        label: choice.label.clone(),
                    });
                }
                if matches!(cwd.as_ref(), Some(cwd) if cwd.trim().is_empty()) {
                    return Err(VisualSceneError::EmptyRunCommandCwd {
                        label: choice.label.clone(),
                    });
                }
            }
            if let SceneActionKind::ExportStoryState { path }
            | SceneActionKind::ImportStoryState { path } = &choice.kind
            {
                if path.trim().is_empty() {
                    return Err(VisualSceneError::EmptyStoryStatePath {
                        label: choice.label.clone(),
                    });
                }
            }
            if let SceneActionKind::AdvanceDialogue { target } = &choice.kind {
                if *target >= self.dialogue_lines.len() {
                    return Err(VisualSceneError::DialogueTargetOutOfBounds {
                        label: choice.label.clone(),
                        target: *target,
                    });
                }
            }
            if let SceneActionKind::Resolve { operations } = &choice.kind {
                validate_state_operations(
                    &choice.label,
                    operations,
                    &self.entities,
                    &self.dialogue_lines,
                    &self.layers,
                    &self.rpg,
                )?;
            }
        }

        Ok(())
    }

    pub fn demo() -> Self {
        Self {
            title: "GameTerm Scene Mode".to_string(),
            background: "workspace-map".to_string(),
            width: 18,
            height: 9,
            mode: VisualModeDescriptor {
                mode_id: "workspace".to_string(),
                label: "Workspace".to_string(),
                description: "Project and process-oriented Scene Mode workspace".to_string(),
                scene_profile: Some("scene".to_string()),
                allowed_actions: vec![
                    "Inspect".to_string(),
                    "OpenFile".to_string(),
                    "RunCommand".to_string(),
                    "Navigate".to_string(),
                    "ExportStoryState".to_string(),
                    "ImportStoryState".to_string(),
                ],
                default_transition: None,
                lifecycle: VisualModeLifecycle::default(),
                input_map: Vec::new(),
            },
            layers: Vec::new(),
            variables: vec![
                VisualStateEntry {
                    key: "conversation_unlocked".to_string(),
                    value: VisualStateValue::Bool(true),
                },
                VisualStateEntry {
                    key: "workspace_level".to_string(),
                    value: VisualStateValue::Number(1),
                },
                VisualStateEntry {
                    key: "active_track".to_string(),
                    value: VisualStateValue::Text("visual-state".to_string()),
                },
            ],
            rpg: VisualRpgState {
                inventory: vec![VisualInventoryItem {
                    item_id: "scene-token".to_string(),
                    label: "Scene Token".to_string(),
                    count: 1,
                    metadata: vec![("source".to_string(), "demo".to_string())],
                }],
                stats: vec![VisualStat {
                    owner_id: Some("project-gameterm".to_string()),
                    key: "focus".to_string(),
                    value: VisualStateValue::Number(3),
                }],
                quests: vec![VisualQuest {
                    quest_id: "verify-scene-runtime".to_string(),
                    label: "Verify Scene Runtime".to_string(),
                    stage: 1,
                    completed: false,
                    journal: "Keep Scene Mode state visible and testable.".to_string(),
                }],
                relationships: vec![VisualRelationship {
                    source_id: "agent-audit".to_string(),
                    target_id: "task-render".to_string(),
                    kind: "monitors".to_string(),
                    value: 2,
                    metadata: vec![],
                }],
            },
            entities: vec![
                VisualEntity {
                    id: "project-gameterm".to_string(),
                    kind: VisualEntityKind::Project,
                    label: "GameTerm".to_string(),
                    position: VisualPosition { x: 3, y: 2 },
                    sprite: "project_core".to_string(),
                    visible: true,
                    state_flags: vec!["active".to_string()],
                    metadata: vec![
                        ("repo".to_string(), "JulianAbeleda/gameterm".to_string()),
                        ("mode".to_string(), "hard-fork".to_string()),
                    ],
                },
                VisualEntity {
                    id: "task-render".to_string(),
                    kind: VisualEntityKind::Task,
                    label: "Render Scene".to_string(),
                    position: VisualPosition { x: 9, y: 4 },
                    sprite: "task_tile".to_string(),
                    visible: true,
                    state_flags: vec!["running".to_string()],
                    metadata: vec![
                        ("reference".to_string(), "Ren'Py scene flow".to_string()),
                        ("reference".to_string(), "mGBA PPU/debug split".to_string()),
                    ],
                },
                VisualEntity {
                    id: "agent-audit".to_string(),
                    kind: VisualEntityKind::Agent,
                    label: "Audit Agent".to_string(),
                    position: VisualPosition { x: 14, y: 2 },
                    sprite: "agent_idle".to_string(),
                    visible: true,
                    state_flags: vec!["watching".to_string()],
                    metadata: vec![("role".to_string(), "review scene state".to_string())],
                },
            ],
            dialogue_speaker: "GameTerm".to_string(),
            dialogue: "Scene Mode renders project state as symbolic entities while preserving terminal control.".to_string(),
            dialogue_lines: vec![],
            choices: vec![
                SceneAction {
                    label: "Inspect selected entity".to_string(),
                    kind: SceneActionKind::Inspect,
                    conditions: vec![],
                },
                SceneAction {
                    label: "Open MIGRATION.md".to_string(),
                    kind: SceneActionKind::OpenFile {
                        path: "MIGRATION.md".to_string(),
                    },
                    conditions: vec![],
                },
                SceneAction {
                    label: "Run cargo check -p gameterm-visual".to_string(),
                    kind: SceneActionKind::RunCommand {
                        argv: vec![
                            "cargo".to_string(),
                            "check".to_string(),
                            "-p".to_string(),
                            "gameterm-visual".to_string(),
                        ],
                        cwd: None,
                        target: RunCommandTarget::Tab,
                    },
                    conditions: vec![],
                },
            ],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VisualView {
    Scene,
    TileDebugger,
}

#[derive(Debug, Clone)]
pub struct SceneRuntime {
    scene: VisualScene,
    scene_source: VisualSceneSource,
    action_base_dir: PathBuf,
    selected_entity: usize,
    selected_choice: usize,
    dialogue_index: usize,
    dialogue_history: Vec<VisualDialogueLine>,
    view: VisualView,
    status: String,
    generation: u64,
    pending_action: Option<VisualActionRequest>,
    last_process_state: Option<VisualProcessState>,
    last_story_state_action: Option<String>,
    last_story_state_path: Option<PathBuf>,
    last_patch_transport: Option<String>,
    last_patch_source_pane_id: Option<usize>,
    last_input_layer: Option<String>,
    last_layer_transition: Option<VisualLayerTransitionReport>,
    transition_history: Vec<VisualRuntimeEvent>,
}

impl SceneRuntime {
    pub fn new(scene: VisualScene) -> Result<Self, VisualSceneError> {
        Self::new_with_source(
            scene,
            VisualSceneSource::new("runtime scene", VisualSceneLoadStatus::Loaded, 0),
        )
    }

    pub fn new_with_source(
        scene: VisualScene,
        scene_source: VisualSceneSource,
    ) -> Result<Self, VisualSceneError> {
        let action_base_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self::new_with_source_and_action_base_dir(scene, scene_source, action_base_dir)
    }

    pub fn new_with_source_and_action_base_dir(
        scene: VisualScene,
        scene_source: VisualSceneSource,
        action_base_dir: impl Into<PathBuf>,
    ) -> Result<Self, VisualSceneError> {
        scene.validate()?;
        let dialogue_history = initial_dialogue_history(&scene, 0);
        let mut runtime = Self {
            scene,
            scene_source,
            action_base_dir: action_base_dir.into(),
            selected_entity: 0,
            selected_choice: 0,
            dialogue_index: 0,
            dialogue_history,
            view: VisualView::Scene,
            status: "Ready".to_string(),
            generation: 0,
            pending_action: None,
            last_process_state: None,
            last_story_state_action: None,
            last_story_state_path: None,
            last_patch_transport: None,
            last_patch_source_pane_id: None,
            last_input_layer: None,
            last_layer_transition: None,
            transition_history: Vec::new(),
        };
        runtime.run_mode_enter_hooks();
        Ok(runtime)
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn view(&self) -> VisualView {
        self.view
    }

    pub fn scene_source(&self) -> &VisualSceneSource {
        &self.scene_source
    }

    pub fn action_base_dir(&self) -> &Path {
        &self.action_base_dir
    }

    pub fn scene(&self) -> &VisualScene {
        &self.scene
    }

    pub fn scene_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.scene)
    }

    pub fn export_story_state(&self) -> VisualStoryState {
        VisualStoryState {
            story_state_version: VisualStoryState::VERSION,
            variables: self.scene.variables.clone(),
            rpg: self.scene.rpg.clone(),
            dialogue_index: dialogue_index(&self.scene, self.dialogue_index),
            dialogue_history: self.dialogue_history.clone(),
        }
    }

    pub fn story_state_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.export_story_state())
    }

    pub fn import_story_state(
        &mut self,
        state: VisualStoryState,
    ) -> Result<(), VisualStoryStateError> {
        state.validate()?;
        if let Some(target) = state.dialogue_index {
            if target >= self.scene.dialogue_lines.len() {
                return Err(VisualStoryStateError::DialogueIndexOutOfBounds { target });
            }
        }

        let dialogue_index = state.dialogue_index.unwrap_or(0);
        let dialogue_history = if state.dialogue_history.is_empty() {
            initial_dialogue_history(&self.scene, dialogue_index)
        } else {
            state.dialogue_history
        };

        self.scene.variables = state.variables;
        self.scene.rpg = state.rpg;
        self.dialogue_index = dialogue_index;
        self.dialogue_history = dialogue_history;
        self.status = "Imported story state".to_string();
        self.bump_generation();
        Ok(())
    }

    pub fn take_pending_action(&mut self) -> Option<VisualActionRequest> {
        self.pending_action.take()
    }

    pub fn mark_reload_failed(&mut self, reload_count: u64, error: String) {
        self.scene_source = self.scene_source.reload_failed(reload_count, error.clone());
        self.status = format!("Reload failed: {error}");
        self.bump_generation();
    }

    pub fn replace_scene_preserving_state(
        &mut self,
        scene: VisualScene,
        scene_source: VisualSceneSource,
    ) -> Result<(), VisualSceneError> {
        scene.validate()?;
        let selected_entity_id = self.selected_entity().map(|entity| entity.id.clone());
        let dialogue_index = if scene.dialogue_lines.is_empty() {
            0
        } else {
            self.dialogue_index.min(scene.dialogue_lines.len() - 1)
        };
        let dialogue_history = initial_dialogue_history(&scene, dialogue_index);
        self.scene = scene;
        self.scene_source = scene_source;
        self.dialogue_index = dialogue_index;
        self.dialogue_history = dialogue_history;
        self.last_input_layer = None;
        self.last_layer_transition = None;
        self.record_runtime_event("scene", "reloaded preserving runtime state");
        self.status = "Ready".to_string();

        self.selected_entity = selected_entity_id
            .and_then(|id| {
                self.scene
                    .entities
                    .iter()
                    .position(|entity| entity.id == id)
            })
            .unwrap_or(0);
        if self.scene.entities.is_empty() {
            self.selected_entity = 0;
        }
        if self.scene.choices.is_empty() {
            self.selected_choice = 0;
        } else if self.selected_choice >= self.scene.choices.len() {
            self.selected_choice = self.scene.choices.len() - 1;
        }
        if let Some(status) = &self.scene.mode.lifecycle.enter_status {
            self.status = status.clone();
        }
        self.bump_generation();
        Ok(())
    }

    pub fn toggle_debugger(&mut self) {
        self.view = match self.view {
            VisualView::Scene => VisualView::TileDebugger,
            VisualView::TileDebugger => VisualView::Scene,
        };
        self.bump_generation();
    }

    pub fn run_mode_enter_hooks(&mut self) {
        if let Some(status) = &self.scene.mode.lifecycle.enter_status {
            self.status = status.clone();
            self.bump_generation();
        }
    }

    pub fn run_mode_update_hooks(&mut self) {
        if let Some(status) = &self.scene.mode.lifecycle.update_status {
            self.status = status.clone();
            self.record_runtime_event("lifecycle", "mode update");
            self.bump_generation();
        }
    }

    pub fn run_mode_exit_hooks(&mut self) {
        if let Some(status) = &self.scene.mode.lifecycle.exit_status {
            self.status = status.clone();
            self.record_runtime_event("lifecycle", "mode exit");
            self.bump_generation();
        }
    }

    fn run_mode_input_action(&mut self, action: &str) -> VisualModeOutcome {
        match action {
            "close" => VisualModeOutcome::Exit,
            "reload" | "ignore" => VisualModeOutcome::Continue,
            "toggle_debug" => {
                self.toggle_debugger();
                VisualModeOutcome::Continue
            }
            "activate_choice" => {
                self.activate_choice();
                VisualModeOutcome::Continue
            }
            "select_next" => {
                self.select_next_entity();
                self.select_next_choice();
                VisualModeOutcome::Continue
            }
            "select_previous" => {
                self.select_prev_entity();
                self.select_prev_choice();
                VisualModeOutcome::Continue
            }
            "run_update_hooks" => {
                self.run_mode_update_hooks();
                VisualModeOutcome::Continue
            }
            "run_exit_hooks" => {
                self.run_mode_exit_hooks();
                VisualModeOutcome::Continue
            }
            "export_story_state" => {
                let path = self.default_story_state_path();
                self.request_story_state_export(path);
                VisualModeOutcome::Continue
            }
            "import_story_state" => {
                let path = self.default_story_state_path();
                self.request_story_state_import(path);
                VisualModeOutcome::Continue
            }
            _ => VisualModeOutcome::Continue,
        }
    }

    fn handle_layer_input(&mut self, input: VisualInput) -> Option<VisualModeOutcome> {
        let input_key = input.binding_key();
        for layer_index in 0..self.scene.layers.len() {
            let transition = self.scene.layers[layer_index]
                .transitions
                .iter()
                .find(|transition| transition.input.trim() == input_key)
                .cloned();
            if let Some(transition) = transition {
                let layer_id = self.scene.layers[layer_index].layer_id.clone();
                let from_state = self.scene.layers[layer_index].state.clone();
                let target_state = transition.target_state.trim().to_string();
                self.last_input_layer = Some(layer_id.clone());
                if !conditions_match(
                    &transition.conditions,
                    &self.scene.variables,
                    &self.scene.rpg,
                    self.selected_entity(),
                    self.last_process_state.as_ref(),
                ) {
                    self.last_layer_transition = Some(VisualLayerTransitionReport {
                        layer_id: layer_id.clone(),
                        input: input_key.to_string(),
                        from_state: from_state.clone(),
                        target_state: target_state.clone(),
                        result: "guard_failed".to_string(),
                    });
                    self.status = format!(
                        "Layer transition unavailable: {} {}",
                        layer_id,
                        condition_guard_detail(&transition.conditions)
                            .unwrap_or_else(|| "guard condition not met".to_string())
                    );
                    self.record_runtime_event(
                        "transition",
                        format!("{layer_id} {from_state} -> {target_state} blocked"),
                    );
                    self.bump_generation();
                    return Some(VisualModeOutcome::Continue);
                }
                self.scene.layers[layer_index].state = target_state.clone();
                self.last_layer_transition = Some(VisualLayerTransitionReport {
                    layer_id: layer_id.clone(),
                    input: input_key.to_string(),
                    from_state: from_state.clone(),
                    target_state: target_state.clone(),
                    result: "transitioned".to_string(),
                });
                self.status =
                    format!("Layer {layer_id} transitioned: {from_state} -> {target_state}");
                self.record_runtime_event(
                    "transition",
                    format!("{layer_id} {from_state} -> {target_state}"),
                );
                self.bump_generation();
                return Some(VisualModeOutcome::Continue);
            }

            let binding = self.scene.layers[layer_index]
                .input_map
                .iter()
                .find(|binding| binding.input.trim() == input_key)
                .cloned();
            if let Some(binding) = binding {
                let layer_id = self.scene.layers[layer_index].layer_id.clone();
                self.last_input_layer = Some(layer_id.clone());
                if !conditions_match(
                    &binding.conditions,
                    &self.scene.variables,
                    &self.scene.rpg,
                    self.selected_entity(),
                    self.last_process_state.as_ref(),
                ) {
                    self.status = format!(
                        "Layer input unavailable: {} {}",
                        layer_id,
                        condition_guard_detail(&binding.conditions)
                            .unwrap_or_else(|| "guard condition not met".to_string())
                    );
                    self.record_runtime_event("input", format!("{layer_id} {input_key} blocked"));
                    self.bump_generation();
                    return Some(VisualModeOutcome::Continue);
                }
                self.record_runtime_event("input", format!("{layer_id} {input_key}"));
                return Some(self.run_mode_input_action(binding.action.trim()));
            }
        }
        None
    }

    pub fn select_next_entity(&mut self) {
        if self.scene.entities.len() > 1 {
            self.selected_entity = (self.selected_entity + 1) % self.scene.entities.len();
            self.bump_generation();
        }
    }

    pub fn select_prev_entity(&mut self) {
        if self.scene.entities.len() > 1 {
            self.selected_entity = if self.selected_entity == 0 {
                self.scene.entities.len() - 1
            } else {
                self.selected_entity - 1
            };
            self.bump_generation();
        }
    }

    pub fn select_next_choice(&mut self) {
        if self.scene.choices.len() > 1 {
            self.selected_choice = (self.selected_choice + 1) % self.scene.choices.len();
            self.bump_generation();
        }
    }

    pub fn select_prev_choice(&mut self) {
        if self.scene.choices.len() > 1 {
            self.selected_choice = if self.selected_choice == 0 {
                self.scene.choices.len() - 1
            } else {
                self.selected_choice - 1
            };
            self.bump_generation();
        }
    }

    pub fn activate_choice(&mut self) {
        self.pending_action = None;
        if let Some(choice) = self.scene.choices.get(self.selected_choice).cloned() {
            if !conditions_match(
                &choice.conditions,
                &self.scene.variables,
                &self.scene.rpg,
                self.selected_entity(),
                self.last_process_state.as_ref(),
            ) {
                self.status = format!(
                    "Choice unavailable: {}",
                    condition_guard_detail(&choice.conditions)
                        .unwrap_or_else(|| "guard condition not met".to_string())
                );
                self.bump_generation();
                return;
            }
            let mut pending_action = None;
            self.status = match &choice.kind {
                SceneActionKind::Inspect => self
                    .selected_entity()
                    .map(|entity| format!("Inspecting {} ({})", entity.label, entity.id))
                    .unwrap_or_else(|| "No entity selected".to_string()),
                SceneActionKind::OpenFile { path } => {
                    let (status, action) = self.open_file_action_status(path);
                    pending_action = action;
                    status
                }
                SceneActionKind::RunCommand { argv, cwd, target } => {
                    pending_action = Some(VisualActionRequest::RunCommand {
                        argv: argv.clone(),
                        cwd: cwd.as_ref().map(PathBuf::from),
                        target: *target,
                    });
                    format!("RunCommand ready ({}): {}", target.as_str(), argv.join(" "))
                }
                SceneActionKind::Navigate { target } => {
                    pending_action = Some(VisualActionRequest::Navigate {
                        target: target.clone(),
                    });
                    format!("Navigate ready: {target}")
                }
                SceneActionKind::ExportStoryState { path } => {
                    let path = self.resolve_action_path(path);
                    pending_action =
                        Some(VisualActionRequest::ExportStoryState { path: path.clone() });
                    format!("ExportStoryState ready: {}", path.display())
                }
                SceneActionKind::ImportStoryState { path } => {
                    let path = self.resolve_action_path(path);
                    pending_action =
                        Some(VisualActionRequest::ImportStoryState { path: path.clone() });
                    format!("ImportStoryState ready: {}", path.display())
                }
                SceneActionKind::AdvanceDialogue { target } => {
                    self.dialogue_index = *target;
                    if let Some(line) = self.scene.dialogue_lines.get(*target).cloned() {
                        self.dialogue_history.push(line.clone());
                        self.record_runtime_event("dialogue", format!("advanced to line {target}"));
                        format!("Dialogue advanced: {}", line.speaker)
                    } else {
                        format!("Dialogue target missing: {target}")
                    }
                }
                SceneActionKind::Resolve { operations } => {
                    match self.apply_state_operations(&choice.label, operations) {
                        Ok(count) => {
                            let summary = operations
                                .iter()
                                .map(visual_state_operation_summary)
                                .collect::<Vec<_>>()
                                .join(", ");
                            self.record_runtime_event(
                                "state",
                                format!(
                                    "{} resolved {count} operation(s): {summary}",
                                    choice.label
                                ),
                            );
                            format!("Resolved {count} operation(s): {}", choice.label)
                        }
                        Err(err) => {
                            self.record_runtime_event(
                                "state",
                                format!("{} failed: {err}", choice.label),
                            );
                            format!("Resolve failed: {err}")
                        }
                    }
                }
            };
            self.pending_action = pending_action;
            self.bump_generation();
        }
    }

    fn apply_state_operations(
        &mut self,
        label: &str,
        operations: &[VisualStateOperation],
    ) -> Result<usize, VisualSceneError> {
        validate_state_operations(
            label,
            operations,
            &self.scene.entities,
            &self.scene.dialogue_lines,
            &self.scene.layers,
            &self.scene.rpg,
        )?;
        let mut variables = self.scene.variables.clone();
        let mut layers = self.scene.layers.clone();
        let mut rpg = self.scene.rpg.clone();
        let mut entities = self.scene.entities.clone();
        let mut selected_entity = self.selected_entity;
        let mut dialogue_index = self.dialogue_index;
        let mut dialogue_history = self.dialogue_history.clone();

        for operation in operations {
            match operation {
                VisualStateOperation::SetVariable { key, value } => {
                    set_variable(&mut variables, key, value.clone());
                }
                VisualStateOperation::SetLayerState { layer_id, state } => {
                    if let Some(layer) = layers.iter_mut().find(|layer| layer.layer_id == *layer_id)
                    {
                        layer.state = state.trim().to_string();
                    }
                }
                VisualStateOperation::SelectEntity { entity_id } => {
                    let Some(index) = entities.iter().position(|entity| entity.id == *entity_id)
                    else {
                        return Err(VisualSceneError::UnknownEntity {
                            label: label.to_string(),
                            entity_id: entity_id.clone(),
                        });
                    };
                    selected_entity = index;
                }
                VisualStateOperation::SetEntityFlags { entity_id, flags } => {
                    let Some(entity) = entities.iter_mut().find(|entity| entity.id == *entity_id)
                    else {
                        return Err(VisualSceneError::UnknownEntity {
                            label: label.to_string(),
                            entity_id: entity_id.clone(),
                        });
                    };
                    entity.state_flags = flags.clone();
                }
                VisualStateOperation::SetEntityMetadata {
                    entity_id,
                    metadata,
                } => {
                    let Some(entity) = entities.iter_mut().find(|entity| entity.id == *entity_id)
                    else {
                        return Err(VisualSceneError::UnknownEntity {
                            label: label.to_string(),
                            entity_id: entity_id.clone(),
                        });
                    };
                    entity.metadata = metadata.clone();
                }
                VisualStateOperation::SetEntityVisibility { entity_id, visible } => {
                    let Some(entity) = entities.iter_mut().find(|entity| entity.id == *entity_id)
                    else {
                        return Err(VisualSceneError::UnknownEntity {
                            label: label.to_string(),
                            entity_id: entity_id.clone(),
                        });
                    };
                    entity.visible = *visible;
                }
                VisualStateOperation::AdvanceDialogueAndSetLayer {
                    target,
                    layer_id,
                    state,
                } => {
                    dialogue_index = *target;
                    if let Some(line) = self.scene.dialogue_lines.get(*target).cloned() {
                        dialogue_history.push(line);
                    }
                    if let Some(layer) = layers.iter_mut().find(|layer| layer.layer_id == *layer_id)
                    {
                        layer.state = state.trim().to_string();
                    }
                }
                VisualStateOperation::TriggerLayerTransition { layer_id, input } => {
                    let Some(layer_index) =
                        layers.iter().position(|layer| layer.layer_id == *layer_id)
                    else {
                        return Err(VisualSceneError::UnknownLayer {
                            label: label.to_string(),
                            layer_id: layer_id.clone(),
                        });
                    };
                    let Some(transition) = layers[layer_index]
                        .transitions
                        .iter()
                        .find(|transition| transition.input == *input)
                        .cloned()
                    else {
                        return Err(VisualSceneError::UnknownLayerTransition {
                            label: label.to_string(),
                            layer_id: layer_id.clone(),
                            input: input.clone(),
                        });
                    };
                    if !conditions_match(&transition.conditions, &variables, &rpg, None, None) {
                        return Err(VisualSceneError::LayerTransitionGuardFailed {
                            label: label.to_string(),
                            layer_id: layer_id.clone(),
                            input: input.clone(),
                        });
                    }
                    layers[layer_index].state = transition.target_state.trim().to_string();
                }
                VisualStateOperation::IncrementVariable { key, amount } => {
                    increment_variable(&mut variables, key, *amount);
                }
                VisualStateOperation::ClearVariable { key } => {
                    variables.retain(|entry| entry.key != *key);
                }
                VisualStateOperation::AddInventory { item } => {
                    match rpg
                        .inventory
                        .iter_mut()
                        .find(|existing| existing.item_id == item.item_id)
                    {
                        Some(existing) => {
                            existing.count = existing.count.saturating_add(item.count)
                        }
                        None => rpg.inventory.push(item.clone()),
                    }
                }
                VisualStateOperation::RemoveInventory { item_id, count } => {
                    let Some(item) = rpg
                        .inventory
                        .iter_mut()
                        .find(|existing| existing.item_id == *item_id)
                    else {
                        return Err(VisualSceneError::UnknownInventoryItem {
                            label: label.to_string(),
                            item_id: item_id.clone(),
                        });
                    };
                    item.count = item.count.saturating_sub(*count);
                    rpg.inventory.retain(|item| item.count > 0);
                }
                VisualStateOperation::SetStat {
                    owner_id,
                    key,
                    value,
                } => match rpg
                    .stats
                    .iter_mut()
                    .find(|stat| stat.owner_id == *owner_id && stat.key == *key)
                {
                    Some(stat) => stat.value = value.clone(),
                    None => rpg.stats.push(VisualStat {
                        owner_id: owner_id.clone(),
                        key: key.clone(),
                        value: value.clone(),
                    }),
                },
                VisualStateOperation::AdjustStat {
                    owner_id,
                    key,
                    amount,
                } => {
                    let Some(stat) = rpg
                        .stats
                        .iter_mut()
                        .find(|stat| stat.owner_id == *owner_id && stat.key == *key)
                    else {
                        return Err(VisualSceneError::UnknownStat {
                            label: label.to_string(),
                            key: key.clone(),
                        });
                    };
                    match &mut stat.value {
                        VisualStateValue::Number(value) => *value += amount,
                        _ => {
                            return Err(VisualSceneError::NonNumericStat {
                                label: label.to_string(),
                                key: key.clone(),
                            });
                        }
                    }
                }
                VisualStateOperation::AdvanceQuest { quest_id, stage } => {
                    let Some(quest) = rpg
                        .quests
                        .iter_mut()
                        .find(|quest| quest.quest_id == *quest_id)
                    else {
                        return Err(VisualSceneError::UnknownQuest {
                            label: label.to_string(),
                            quest_id: quest_id.clone(),
                        });
                    };
                    quest.stage = *stage;
                }
                VisualStateOperation::CompleteQuest { quest_id } => {
                    let Some(quest) = rpg
                        .quests
                        .iter_mut()
                        .find(|quest| quest.quest_id == *quest_id)
                    else {
                        return Err(VisualSceneError::UnknownQuest {
                            label: label.to_string(),
                            quest_id: quest_id.clone(),
                        });
                    };
                    quest.completed = true;
                }
                VisualStateOperation::AppendQuestJournal { quest_id, text } => {
                    let Some(quest) = rpg
                        .quests
                        .iter_mut()
                        .find(|quest| quest.quest_id == *quest_id)
                    else {
                        return Err(VisualSceneError::UnknownQuest {
                            label: label.to_string(),
                            quest_id: quest_id.clone(),
                        });
                    };
                    if !quest.journal.is_empty() {
                        quest.journal.push('\n');
                    }
                    quest.journal.push_str(text);
                }
                VisualStateOperation::AdjustRelationship {
                    source_id,
                    target_id,
                    kind,
                    amount,
                } => {
                    let Some(relationship) = rpg.relationships.iter_mut().find(|relationship| {
                        relationship.source_id == *source_id
                            && relationship.target_id == *target_id
                            && relationship.kind == *kind
                    }) else {
                        return Err(VisualSceneError::UnknownRelationship {
                            label: label.to_string(),
                            relationship: relationship_key(source_id, target_id, kind),
                        });
                    };
                    relationship.value += amount;
                }
            }
        }

        validate_state_entries(&variables).map_err(|err| match err {
            VisualStateEntryError::EmptyKey => VisualSceneError::EmptyVariableKey,
            VisualStateEntryError::DuplicateKey(key) => VisualSceneError::DuplicateVariableKey(key),
        })?;
        validate_layers(&layers)?;
        validate_rpg_state(&rpg)?;

        self.scene.variables = variables;
        self.scene.layers = layers;
        self.scene.rpg = rpg;
        self.scene.entities = entities;
        self.selected_entity = selected_entity;
        self.dialogue_index = dialogue_index;
        self.dialogue_history = dialogue_history;
        Ok(operations.len())
    }

    pub fn mark_action_status(&mut self, status: impl Into<String>) {
        self.status = status.into();
        self.bump_generation();
    }

    fn resolve_action_path(&self, path: &str) -> PathBuf {
        let raw_path = PathBuf::from(path);
        if raw_path.is_absolute() {
            raw_path
        } else {
            self.action_base_dir.join(raw_path)
        }
    }

    fn default_story_state_path(&self) -> PathBuf {
        let scene_path = PathBuf::from(&self.scene_source.scene_path);
        if scene_path.file_name().is_some() {
            scene_path.with_extension("story.json")
        } else {
            self.action_base_dir.join("gameterm-scene.story.json")
        }
    }

    fn request_story_state_export(&mut self, path: PathBuf) {
        self.pending_action = Some(VisualActionRequest::ExportStoryState { path: path.clone() });
        self.status = format!("ExportStoryState ready: {}", path.display());
        self.bump_generation();
    }

    fn request_story_state_import(&mut self, path: PathBuf) {
        self.pending_action = Some(VisualActionRequest::ImportStoryState { path: path.clone() });
        self.status = format!("ImportStoryState ready: {}", path.display());
        self.bump_generation();
    }

    pub fn mark_story_state_exported(&mut self, path: &Path) {
        self.last_story_state_action = Some("export".to_string());
        self.last_story_state_path = Some(path.to_path_buf());
        self.status = format!("Story state exported: {}", path.display());
        self.record_runtime_event("story_state", format!("export {}", path.display()));
        self.bump_generation();
    }

    pub fn mark_story_state_imported(&mut self, path: &Path) {
        self.last_story_state_action = Some("import".to_string());
        self.last_story_state_path = Some(path.to_path_buf());
        self.status = format!("Story state imported: {}", path.display());
        self.record_runtime_event("story_state", format!("import {}", path.display()));
        self.bump_generation();
    }

    pub fn mark_story_state_failed(
        &mut self,
        action: impl Into<String>,
        path: &Path,
        error: impl std::fmt::Display,
    ) {
        let action = action.into();
        self.last_story_state_action = Some(action.clone());
        self.last_story_state_path = Some(path.to_path_buf());
        self.status = format!("Story state {action} failed: {}: {error}", path.display());
        self.record_runtime_event("story_state", format!("{action} failed {}", path.display()));
        self.bump_generation();
    }

    pub fn apply_scene_patch(
        &mut self,
        patch: VisualScenePatch,
    ) -> Result<(), VisualScenePatchError> {
        self.apply_scene_patch_with_source(patch, None, None)
    }

    pub fn apply_scene_patch_with_source(
        &mut self,
        patch: VisualScenePatch,
        transport: Option<String>,
        source_pane_id: Option<usize>,
    ) -> Result<(), VisualScenePatchError> {
        patch.validate()?;
        for update in &patch.updates {
            let Some(entity) = self
                .scene
                .entities
                .iter()
                .find(|entity| entity.id == update.entity_id)
            else {
                return Err(VisualScenePatchError::UnknownEntityId(
                    update.entity_id.clone(),
                ));
            };
            if let Some(position) = update.position {
                if position.x >= self.scene.width || position.y >= self.scene.height {
                    return Err(VisualScenePatchError::EntityOutOfBounds {
                        entity_id: entity.id.clone(),
                        x: position.x,
                        y: position.y,
                    });
                }
            }
        }
        let selected_entity = patch.selected_entity_id.as_ref().map(|id| {
            self.scene
                .entities
                .iter()
                .position(|entity| entity.id == *id)
                .ok_or_else(|| VisualScenePatchError::UnknownEntityId(id.clone()))
        });
        let selected_entity = match selected_entity {
            Some(result) => Some(result?),
            None => None,
        };
        if let Some(process_state) = &patch.process_state {
            if let Some(entity_id) = &process_state.entity_id {
                if !self
                    .scene
                    .entities
                    .iter()
                    .any(|entity| entity.id == *entity_id)
                {
                    return Err(VisualScenePatchError::UnknownProcessEntityId(
                        entity_id.clone(),
                    ));
                }
            }
        }

        let update_count = patch.updates.len();
        let variable_count = patch.variables.len();
        let process_state = patch.process_state;
        for update in patch.updates {
            if let Some(entity) = self
                .scene
                .entities
                .iter_mut()
                .find(|entity| entity.id == update.entity_id)
            {
                if let Some(label) = update.label {
                    entity.label = label;
                }
                if let Some(position) = update.position {
                    entity.position = position;
                }
                if let Some(sprite) = update.sprite {
                    entity.sprite = sprite;
                }
                if let Some(visible) = update.visible {
                    entity.visible = visible;
                }
                if let Some(state_flags) = update.state_flags {
                    entity.state_flags = state_flags;
                }
                if let Some(metadata) = update.metadata {
                    entity.metadata = metadata;
                }
            }
        }
        for variable in patch.variables {
            match self
                .scene
                .variables
                .iter_mut()
                .find(|entry| entry.key == variable.key)
            {
                Some(entry) => entry.value = variable.value,
                None => self.scene.variables.push(variable),
            }
        }
        if let Some(selected_entity) = selected_entity {
            self.selected_entity = selected_entity;
        }
        if let Some(process_state) = process_state {
            self.last_process_state = Some(process_state);
        }

        self.status = patch.status.unwrap_or_else(|| {
            format!(
                "Applied scene patch: {} entity update(s), {} variable update(s)",
                update_count, variable_count
            )
        });
        self.last_patch_transport = transport;
        self.last_patch_source_pane_id = source_pane_id;
        self.record_runtime_event(
            "patch",
            format!(
                "{} entity update(s), {} variable update(s)",
                update_count, variable_count
            ),
        );
        self.bump_generation();
        Ok(())
    }

    pub fn mark_scene_patch_failed(
        &mut self,
        transport: impl Into<String>,
        source_pane_id: Option<usize>,
        error: impl std::fmt::Display,
    ) {
        let transport = transport.into();
        self.last_patch_transport = Some(transport.clone());
        self.last_patch_source_pane_id = source_pane_id;
        self.status = match source_pane_id {
            Some(pane_id) => format!("Scene patch failed from {transport} pane {pane_id}: {error}"),
            None => format!("Scene patch failed from {transport}: {error}"),
        };
        self.record_runtime_event("patch", format!("{transport} failed: {error}"));
        self.bump_generation();
    }

    pub fn mark_open_file_dispatched(&mut self, path: &Path) {
        self.status = format!("OpenFile opening: {}", path.display());
        self.bump_generation();
    }

    pub fn mark_run_command_spawning(&mut self, argv: &[String], target: RunCommandTarget) {
        self.status = format!("RunCommand opening {}: {}", target.as_str(), argv.join(" "));
        self.bump_generation();
    }

    pub fn mark_run_command_spawned(
        &mut self,
        argv: &[String],
        target: RunCommandTarget,
        pane_id: usize,
    ) {
        self.status = format!(
            "RunCommand opened {} pane {pane_id}: {}",
            target.as_str(),
            argv.join(" ")
        );
        self.bump_generation();
    }

    pub fn mark_run_command_failed(
        &mut self,
        argv: &[String],
        target: RunCommandTarget,
        error: impl std::fmt::Display,
    ) {
        self.status = format!(
            "RunCommand failed ({}): {}: {error}",
            target.as_str(),
            argv.join(" ")
        );
        self.bump_generation();
    }

    pub fn selected_entity(&self) -> Option<&VisualEntity> {
        self.scene.entities.get(self.selected_entity)
    }

    pub fn active_dialogue_line(&self) -> VisualDialogueLine {
        active_dialogue_line(&self.scene, self.dialogue_index)
    }

    fn open_file_action_status(&self, path: &str) -> (String, Option<VisualActionRequest>) {
        let resolved = self.resolve_action_path(path);
        let display_path = resolved.display();
        match std::fs::metadata(&resolved) {
            Ok(metadata) if metadata.is_file() => (
                format!("OpenFile ready: {display_path}"),
                Some(VisualActionRequest::OpenFile { path: resolved }),
            ),
            Ok(_) => (
                format!("OpenFile target is not a file: {display_path}"),
                None,
            ),
            Err(err) => (format!("OpenFile missing: {display_path}: {err}"), None),
        }
    }

    pub fn render_text_frame(&self, cols: usize, rows: usize) -> String {
        match self.view {
            VisualView::Scene => self.render_scene(cols, rows),
            VisualView::TileDebugger => self.render_debugger(cols, rows),
        }
    }

    pub fn render_snapshot(&self) -> VisualRenderSnapshot {
        let selected_entity_id = self.selected_entity().map(|entity| entity.id.clone());
        let selected_entity_mode = self.selected_entity().and_then(entity_mode);
        let active_dialogue = self.active_dialogue_line();
        VisualRenderSnapshot {
            generation: self.generation,
            view: self.view,
            scene_source: self.scene_source.clone(),
            active_mode: self.scene.mode.clone(),
            active_layers: self.scene.layers.clone(),
            selected_entity_mode,
            variables: self.scene.variables.clone(),
            rpg: self.scene.rpg.clone(),
            title: self.scene.title.clone(),
            background: self.scene.background.clone(),
            width: self.scene.width,
            height: self.scene.height,
            selected_entity_id,
            selected_choice: self.selected_choice,
            tiles: self.render_tiles(),
            entities: self.render_entities(),
            dialogue_speaker: active_dialogue.speaker,
            dialogue: active_dialogue.text,
            dialogue_index: dialogue_index(&self.scene, self.dialogue_index),
            dialogue_history: self.dialogue_history.clone(),
            status: self.status.clone(),
            choices: self
                .scene
                .choices
                .iter()
                .map(|choice| choice.label.clone())
                .collect(),
        }
    }

    pub fn debug_report(&self) -> VisualSceneDebugReport {
        let selected_entity = self.selected_entity();
        let selected_choice = self.scene.choices.get(self.selected_choice);
        let selected_choice_enabled = selected_choice
            .map(|choice| {
                conditions_match(
                    &choice.conditions,
                    &self.scene.variables,
                    &self.scene.rpg,
                    self.selected_entity(),
                    self.last_process_state.as_ref(),
                )
            })
            .unwrap_or(false);
        let pending_action = self.pending_action.as_ref();
        VisualSceneDebugReport {
            scene_path: self.scene_source.scene_path.clone(),
            load_status: self.scene_source.load_status.as_str().to_string(),
            reload_count: self.scene_source.reload_count,
            last_error: self.scene_source.last_error.clone(),
            action_base_dir: self.action_base_dir.display().to_string(),
            active_mode_id: self.scene.mode.mode_id.clone(),
            active_mode_label: self.scene.mode.label.clone(),
            active_mode_description: self.scene.mode.description.clone(),
            active_mode_scene_profile: self.scene.mode.scene_profile.clone(),
            active_mode_allowed_actions: self.scene.mode.allowed_actions.clone(),
            active_mode_default_transition: self.scene.mode.default_transition.clone(),
            active_mode_lifecycle: self.scene.mode.lifecycle.clone(),
            active_mode_input_map: self.scene.mode.input_map.clone(),
            active_layers: self.scene.layers.clone(),
            last_input_layer: self.last_input_layer.clone(),
            last_layer_transition: self.last_layer_transition.clone(),
            transition_history: self.transition_history.clone(),
            selected_entity_mode: selected_entity.and_then(entity_mode),
            variables: self.scene.variables.clone(),
            rpg: self.scene.rpg.clone(),
            title: self.scene.title.clone(),
            background: self.scene.background.clone(),
            width: self.scene.width,
            height: self.scene.height,
            entity_count: self.scene.entities.len(),
            choice_count: self.scene.choices.len(),
            selected_entity_id: selected_entity.map(|entity| entity.id.clone()),
            selected_entity_label: selected_entity.map(|entity| entity.label.clone()),
            selected_entity_kind: selected_entity.map(|entity| format!("{:?}", entity.kind)),
            selected_entity_sprite: selected_entity.map(|entity| entity.sprite.clone()),
            selected_entity_flags: selected_entity
                .map(|entity| entity.state_flags.clone())
                .unwrap_or_default(),
            selected_entity_metadata: selected_entity
                .map(|entity| entity.metadata.clone())
                .unwrap_or_default(),
            dialogue_index: dialogue_index(&self.scene, self.dialogue_index),
            dialogue_line_count: self.scene.dialogue_lines.len(),
            dialogue_history: self.dialogue_history.clone(),
            selected_choice: self.selected_choice,
            selected_choice_label: selected_choice.map(|choice| choice.label.clone()),
            selected_choice_kind: selected_choice.map(|choice| action_kind_name(&choice.kind)),
            selected_choice_detail: selected_choice.map(|choice| action_kind_detail(&choice.kind)),
            selected_choice_enabled,
            selected_choice_guard_detail: selected_choice
                .and_then(|choice| condition_guard_detail(&choice.conditions)),
            pending_action_kind: pending_action.map(action_request_name),
            pending_action_detail: pending_action.map(action_request_detail),
            process_state: self.last_process_state.clone(),
            last_story_state_action: self.last_story_state_action.clone(),
            last_story_state_path: self
                .last_story_state_path
                .as_ref()
                .map(|path| path.display().to_string()),
            last_patch_transport: self.last_patch_transport.clone(),
            last_patch_source_pane_id: self.last_patch_source_pane_id,
            status: self.status.clone(),
        }
    }

    fn render_tiles(&self) -> Vec<VisualRenderTile> {
        let mut tiles = Vec::with_capacity(self.scene.width * self.scene.height);
        for y in 0..self.scene.height {
            for x in 0..self.scene.width {
                tiles.push(VisualRenderTile {
                    position: VisualPosition { x, y },
                    sprite: self.scene.background.clone(),
                    layer: VisualRenderLayer::Tile,
                });
            }
        }
        tiles
    }

    fn render_entities(&self) -> Vec<VisualRenderEntity> {
        self.scene
            .entities
            .iter()
            .enumerate()
            .filter(|(_, entity)| entity.visible)
            .map(|(idx, entity)| VisualRenderEntity {
                id: entity.id.clone(),
                kind: entity.kind.clone(),
                label: entity.label.clone(),
                position: entity.position,
                sprite: entity.sprite.clone(),
                layer: VisualRenderLayer::Entity,
                selected: idx == self.selected_entity,
                state_flags: entity.state_flags.clone(),
            })
            .collect()
    }

    fn bump_generation(&mut self) {
        self.generation = self.generation.saturating_add(1);
    }

    fn record_runtime_event(&mut self, kind: impl Into<String>, detail: impl Into<String>) {
        self.transition_history.push(VisualRuntimeEvent {
            kind: kind.into(),
            detail: detail.into(),
        });
        const MAX_RUNTIME_EVENTS: usize = 8;
        if self.transition_history.len() > MAX_RUNTIME_EVENTS {
            let excess = self.transition_history.len() - MAX_RUNTIME_EVENTS;
            self.transition_history.drain(0..excess);
        }
    }

    fn render_scene(&self, cols: usize, rows: usize) -> String {
        let mut out = String::new();
        out.push_str(&format!("{}\r\n", self.scene.title));
        out.push_str("Scene Mode  [arrows/hjkl: select] [enter: action] [tab: debugger] [r: reload] [esc/q: close]\r\n\r\n");
        if self.scene_source.load_status == VisualSceneLoadStatus::ReloadFailed {
            if let Some(error) = &self.scene_source.last_error {
                out.push_str(&format!("Reload failed: {error}\r\n\r\n"));
            }
        }

        let mut grid = vec![vec!['.'; self.scene.width]; self.scene.height];
        for (idx, entity) in self.scene.entities.iter().enumerate() {
            let glyph = match entity.kind {
                VisualEntityKind::Agent => 'A',
                VisualEntityKind::Memory => 'M',
                VisualEntityKind::Principle => 'P',
                VisualEntityKind::Project => 'R',
                VisualEntityKind::Task => 'T',
            };
            grid[entity.position.y][entity.position.x] = if idx == self.selected_entity {
                '@'
            } else {
                glyph
            };
        }

        let available_grid_rows = rows.saturating_sub(13).min(self.scene.height);
        for row in grid.into_iter().take(available_grid_rows) {
            out.push_str("  ");
            for ch in row {
                out.push(ch);
                out.push(' ');
            }
            out.push_str("\r\n");
        }

        out.push_str("\r\n");
        if let Some(entity) = self.selected_entity() {
            out.push_str(&format!(
                "Selected: {} [{:?}] sprite={} flags={}\r\n",
                entity.label,
                entity.kind,
                entity.sprite,
                entity.state_flags.join(", ")
            ));
        }
        out.push_str(&format!(
            "Mode: {} ({})\r\n",
            self.scene.mode.label, self.scene.mode.mode_id
        ));
        if !self.scene.variables.is_empty() {
            out.push_str("State: ");
            for (idx, variable) in self.scene.variables.iter().enumerate() {
                if idx > 0 {
                    out.push_str(", ");
                }
                out.push_str(&format!(
                    "{}={}",
                    variable.key,
                    variable.value.as_debug_string()
                ));
            }
            out.push_str("\r\n");
        }
        if !self.scene.rpg.is_empty() {
            out.push_str(&format!(
                "RPG: inventory={} stats={} quests={} relationships={}\r\n",
                self.scene.rpg.inventory.len(),
                self.scene.rpg.stats.len(),
                self.scene.rpg.quests.len(),
                self.scene.rpg.relationships.len()
            ));
        }
        if let Some(action) = &self.last_story_state_action {
            match &self.last_story_state_path {
                Some(path) => {
                    out.push_str(&format!("Story State: {action} {}\r\n", path.display()))
                }
                None => out.push_str(&format!("Story State: {action}\r\n")),
            }
        } else if self.scene.mode.mode_id == "authoring" {
            out.push_str(&format!(
                "Story State: default {}\r\n",
                self.default_story_state_path().display()
            ));
        }
        out.push_str(&format!(
            "{}: {}\r\n\r\n",
            self.active_dialogue_line().speaker,
            self.active_dialogue_line().text
        ));

        out.push_str("Choices:\r\n");
        for (idx, choice) in self.scene.choices.iter().enumerate() {
            let marker = if idx == self.selected_choice {
                ">"
            } else {
                " "
            };
            let guard = if conditions_match(
                &choice.conditions,
                &self.scene.variables,
                &self.scene.rpg,
                self.selected_entity(),
                self.last_process_state.as_ref(),
            ) {
                ""
            } else {
                " [locked]"
            };
            out.push_str(&format!("{marker} {}{}\r\n", choice.label, guard));
        }
        out.push_str(&format!("\r\nStatus: {}\r\n", self.status));
        truncate_to_screen(out, cols, rows)
    }

    fn render_debugger(&self, cols: usize, rows: usize) -> String {
        let report = self.debug_report();
        let mut out = String::new();
        out.push_str("GameTerm Tile Debugger\r\n");
        out.push_str("[tab: scene] [arrows/hjkl: select entity] [esc/q: close]\r\n\r\n");
        out.push_str("Source:\r\n");
        out.push_str(&format!("  Scene path: {}\r\n", report.scene_path));
        out.push_str(&format!("  Load status: {}\r\n", report.load_status));
        out.push_str(&format!("  Reload counter: {}\r\n", report.reload_count));
        out.push_str(&format!(
            "  Action base dir: {}\r\n",
            report.action_base_dir
        ));
        if let Some(error) = &report.last_error {
            out.push_str(&format!("  Error: {error}\r\n"));
        }
        out.push_str("\r\nMode:\r\n");
        out.push_str(&format!(
            "  Active: {} ({})\r\n",
            report.active_mode_label, report.active_mode_id
        ));
        if !report.active_mode_description.is_empty() {
            out.push_str(&format!(
                "  Description: {}\r\n",
                report.active_mode_description
            ));
        }
        if let Some(profile) = &report.active_mode_scene_profile {
            out.push_str(&format!("  Scene profile: {profile}\r\n"));
        }
        if !report.active_mode_allowed_actions.is_empty() {
            out.push_str(&format!(
                "  Allowed actions: {}\r\n",
                report.active_mode_allowed_actions.join(", ")
            ));
        }
        if let Some(transition) = &report.active_mode_default_transition {
            out.push_str(&format!("  Default transition: {transition}\r\n"));
        }
        if !report.active_mode_lifecycle.is_empty() {
            out.push_str("  Lifecycle:");
            if report.active_mode_lifecycle.enter_status.is_some() {
                out.push_str(" enter");
            }
            if report.active_mode_lifecycle.update_status.is_some() {
                out.push_str(" update");
            }
            if report.active_mode_lifecycle.exit_status.is_some() {
                out.push_str(" exit");
            }
            out.push_str("\r\n");
        }
        if !report.active_mode_input_map.is_empty() {
            out.push_str("  Input map:\r\n");
            for binding in &report.active_mode_input_map {
                let guard = condition_guard_detail(&binding.conditions)
                    .map(|detail| format!(" ({detail})"))
                    .unwrap_or_default();
                out.push_str(&format!(
                    "    {} -> {}{}\r\n",
                    binding.input, binding.action, guard
                ));
            }
        }
        if !report.active_layers.is_empty() {
            out.push_str("  Layers:\r\n");
            for layer in &report.active_layers {
                let label = layer
                    .label
                    .as_ref()
                    .map(|label| format!(" label={label}"))
                    .unwrap_or_default();
                out.push_str(&format!(
                    "    {} state={}{}\r\n",
                    layer.layer_id, layer.state, label
                ));
            }
        }
        if let Some(layer) = &report.last_input_layer {
            out.push_str(&format!("  Last input layer: {layer}\r\n"));
        }
        if let Some(transition) = &report.last_layer_transition {
            out.push_str(&format!(
                "  Last transition: {} {} {} -> {} ({})\r\n",
                transition.layer_id,
                transition.input,
                transition.from_state,
                transition.target_state,
                transition.result
            ));
        }
        if !report.transition_history.is_empty() {
            out.push_str("  History:\r\n");
            for event in &report.transition_history {
                out.push_str(&format!("    {}: {}\r\n", event.kind, event.detail));
            }
        }
        if let Some(mode) = &report.selected_entity_mode {
            out.push_str(&format!("  Selected entity mode: {mode}\r\n"));
        }
        if !report.variables.is_empty() {
            out.push_str("\r\nState:\r\n");
            for variable in &report.variables {
                out.push_str(&format!(
                    "  {}: {}\r\n",
                    variable.key,
                    variable.value.as_debug_string()
                ));
            }
        }
        out.push_str("\r\nAction:\r\n");
        out.push_str(&format!("  Status: {}\r\n", report.status));
        out.push_str(&format!(
            "  Selected choice: {}\r\n",
            report.selected_choice
        ));
        if let Some(label) = &report.selected_choice_label {
            out.push_str(&format!("  Choice label: {label}\r\n"));
        }
        if let Some(kind) = &report.selected_choice_kind {
            out.push_str(&format!("  Choice kind: {kind}\r\n"));
        }
        if let Some(detail) = &report.selected_choice_detail {
            out.push_str(&format!("  Choice detail: {detail}\r\n"));
        }
        out.push_str(&format!(
            "  Choice enabled: {}\r\n",
            report.selected_choice_enabled
        ));
        if let Some(detail) = &report.selected_choice_guard_detail {
            out.push_str(&format!("  Choice guard: {detail}\r\n"));
        }
        match (&report.pending_action_kind, &report.pending_action_detail) {
            (Some(kind), Some(detail)) => {
                out.push_str(&format!("  Pending action: {kind} {detail}\r\n"));
            }
            (Some(kind), None) => {
                out.push_str(&format!("  Pending action: {kind}\r\n"));
            }
            _ => out.push_str("  Pending action: none\r\n"),
        }
        if let Some(process_state) = &report.process_state {
            out.push_str(&format!("  Process phase: {:?}\r\n", process_state.phase));
            if let Some(entity_id) = &process_state.entity_id {
                out.push_str(&format!("  Process entity: {entity_id}\r\n"));
            }
            if let Some(command) = &process_state.command {
                out.push_str(&format!("  Process command: {command}\r\n"));
            }
            if let Some(exit_code) = process_state.exit_code {
                out.push_str(&format!("  Process exit code: {exit_code}\r\n"));
            }
            if let Some(message) = &process_state.message {
                out.push_str(&format!("  Process message: {message}\r\n"));
            }
        }
        match (
            &report.last_story_state_action,
            &report.last_story_state_path,
        ) {
            (Some(action), Some(path)) => {
                out.push_str(&format!("  Last story state: {action} {path}\r\n"));
            }
            (Some(action), None) => {
                out.push_str(&format!("  Last story state: {action}\r\n"));
            }
            _ => out.push_str("  Last story state: none\r\n"),
        }
        match (
            &report.last_patch_transport,
            report.last_patch_source_pane_id,
        ) {
            (Some(transport), Some(pane_id)) => {
                out.push_str(&format!(
                    "  Last patch: {transport} from pane {pane_id}\r\n"
                ));
            }
            (Some(transport), None) => {
                out.push_str(&format!("  Last patch: {transport}\r\n"));
            }
            (None, _) => out.push_str("  Last patch: none\r\n"),
        }
        out.push_str("\r\nDialogue:\r\n");
        match report.dialogue_index {
            Some(index) => out.push_str(&format!(
                "  Active line: {} of {}\r\n",
                index + 1,
                report.dialogue_line_count
            )),
            None => out.push_str("  Active line: legacy\r\n"),
        }
        out.push_str(&format!(
            "  History entries: {}\r\n",
            report.dialogue_history.len()
        ));
        if !report.rpg.is_empty() {
            out.push_str("\r\nRPG:\r\n");
            out.push_str(&format!(
                "  Inventory items: {}\r\n",
                report.rpg.inventory.len()
            ));
            out.push_str(&format!("  Stats: {}\r\n", report.rpg.stats.len()));
            out.push_str(&format!("  Quests: {}\r\n", report.rpg.quests.len()));
            out.push_str(&format!(
                "  Relationships: {}\r\n",
                report.rpg.relationships.len()
            ));
        }
        out.push_str("\r\n");
        out.push_str(&format!(
            "scene={} background={} size={}x{} entities={} choices={}\r\n\r\n",
            report.title,
            report.background,
            report.width,
            report.height,
            report.entity_count,
            report.choice_count
        ));
        out.push_str("Layer order:\r\n");
        out.push_str("  0 background\r\n  1 tile grid\r\n  2 entity sprites\r\n  3 selection/relations\r\n  4 dialogue\r\n  5 debug overlay\r\n\r\n");
        out.push_str("Entities:\r\n");
        for (idx, entity) in self.scene.entities.iter().enumerate() {
            let marker = if idx == self.selected_entity {
                ">"
            } else {
                " "
            };
            out.push_str(&format!(
                "{marker} id={} kind={:?} pos={},{} sprite={} flags={}\r\n",
                entity.id,
                entity.kind,
                entity.position.x,
                entity.position.y,
                entity.sprite,
                entity.state_flags.join(", ")
            ));
        }
        if let Some(entity) = self.selected_entity() {
            out.push_str("\r\nSelected metadata:\r\n");
            out.push_str(&format!(
                "  label: {}\r\n  kind: {:?}\r\n  sprite: {}\r\n  flags: {}\r\n",
                entity.label,
                entity.kind,
                entity.sprite,
                entity.state_flags.join(", ")
            ));
            for (key, value) in &entity.metadata {
                out.push_str(&format!("  {key}: {value}\r\n"));
            }
        }
        truncate_to_screen(out, cols, rows)
    }
}

fn action_kind_name(kind: &SceneActionKind) -> String {
    match kind {
        SceneActionKind::Inspect => "Inspect",
        SceneActionKind::OpenFile { .. } => "OpenFile",
        SceneActionKind::RunCommand { .. } => "RunCommand",
        SceneActionKind::Navigate { .. } => "Navigate",
        SceneActionKind::ExportStoryState { .. } => "ExportStoryState",
        SceneActionKind::ImportStoryState { .. } => "ImportStoryState",
        SceneActionKind::AdvanceDialogue { .. } => "AdvanceDialogue",
        SceneActionKind::Resolve { .. } => "Resolve",
    }
    .to_string()
}

fn action_kind_detail(kind: &SceneActionKind) -> String {
    match kind {
        SceneActionKind::Inspect => "selected entity".to_string(),
        SceneActionKind::OpenFile { path } => format!("path={path}"),
        SceneActionKind::RunCommand { argv, cwd, target } => {
            let mut detail = format!("argv={}", argv.join(" "));
            if let Some(cwd) = cwd {
                detail.push_str(&format!(" cwd={cwd}"));
            }
            detail.push_str(&format!(" target={}", target.as_str()));
            detail
        }
        SceneActionKind::Navigate { target } => format!("target={target}"),
        SceneActionKind::ExportStoryState { path } => format!("path={path}"),
        SceneActionKind::ImportStoryState { path } => format!("path={path}"),
        SceneActionKind::AdvanceDialogue { target } => format!("target={target}"),
        SceneActionKind::Resolve { operations } => format!("operations={}", operations.len()),
    }
}

fn action_request_name(action: &VisualActionRequest) -> String {
    match action {
        VisualActionRequest::OpenFile { .. } => "OpenFile",
        VisualActionRequest::RunCommand { .. } => "RunCommand",
        VisualActionRequest::Navigate { .. } => "Navigate",
        VisualActionRequest::ExportStoryState { .. } => "ExportStoryState",
        VisualActionRequest::ImportStoryState { .. } => "ImportStoryState",
    }
    .to_string()
}

fn action_request_detail(action: &VisualActionRequest) -> String {
    match action {
        VisualActionRequest::OpenFile { path } => format!("path={}", path.display()),
        VisualActionRequest::RunCommand { argv, cwd, target } => {
            let mut detail = format!("argv={}", argv.join(" "));
            if let Some(cwd) = cwd {
                detail.push_str(&format!(" cwd={}", cwd.display()));
            }
            detail.push_str(&format!(" target={}", target.as_str()));
            detail
        }
        VisualActionRequest::Navigate { target } => format!("target={target}"),
        VisualActionRequest::ExportStoryState { path } => format!("path={}", path.display()),
        VisualActionRequest::ImportStoryState { path } => format!("path={}", path.display()),
    }
}

fn visual_state_operation_summary(operation: &VisualStateOperation) -> String {
    match operation {
        VisualStateOperation::SetVariable { key, value } => {
            format!("set {key}={}", value.as_debug_string())
        }
        VisualStateOperation::SetLayerState { layer_id, state } => {
            format!("layer {layer_id}->{state}")
        }
        VisualStateOperation::SelectEntity { entity_id } => format!("select {entity_id}"),
        VisualStateOperation::SetEntityFlags { entity_id, flags } => {
            format!("flags {entity_id}=[{}]", flags.join(","))
        }
        VisualStateOperation::SetEntityMetadata {
            entity_id,
            metadata,
        } => {
            format!("metadata {entity_id}={} pair(s)", metadata.len())
        }
        VisualStateOperation::SetEntityVisibility { entity_id, visible } => {
            format!("visible {entity_id}={visible}")
        }
        VisualStateOperation::AdvanceDialogueAndSetLayer {
            target,
            layer_id,
            state,
        } => {
            format!("dialogue {target} + layer {layer_id}->{state}")
        }
        VisualStateOperation::TriggerLayerTransition { layer_id, input } => {
            format!("transition {layer_id}:{input}")
        }
        VisualStateOperation::IncrementVariable { key, amount } => {
            format!("increment {key} by {amount}")
        }
        VisualStateOperation::ClearVariable { key } => format!("clear {key}"),
        VisualStateOperation::AddInventory { item } => {
            format!("add inventory {} x{}", item.item_id, item.count)
        }
        VisualStateOperation::RemoveInventory { item_id, count } => {
            format!("remove inventory {item_id} x{count}")
        }
        VisualStateOperation::SetStat {
            owner_id,
            key,
            value,
        } => {
            let prefix = owner_id
                .as_ref()
                .map(|owner_id| format!("{owner_id}:"))
                .unwrap_or_default();
            format!("set stat {prefix}{key}={}", value.as_debug_string())
        }
        VisualStateOperation::AdjustStat {
            owner_id,
            key,
            amount,
        } => {
            let prefix = owner_id
                .as_ref()
                .map(|owner_id| format!("{owner_id}:"))
                .unwrap_or_default();
            format!("adjust stat {prefix}{key} by {amount}")
        }
        VisualStateOperation::AdvanceQuest { quest_id, stage } => {
            format!("quest {quest_id} stage {stage}")
        }
        VisualStateOperation::CompleteQuest { quest_id } => {
            format!("quest {quest_id} complete")
        }
        VisualStateOperation::AppendQuestJournal { quest_id, .. } => {
            format!("quest {quest_id} journal")
        }
        VisualStateOperation::AdjustRelationship {
            source_id,
            target_id,
            kind,
            amount,
        } => {
            format!("relationship {source_id}:{target_id}:{kind} by {amount}")
        }
    }
}

fn conditions_match(
    conditions: &[VisualCondition],
    variables: &[VisualStateEntry],
    rpg: &VisualRpgState,
    selected_entity: Option<&VisualEntity>,
    process_state: Option<&VisualProcessState>,
) -> bool {
    conditions.iter().all(|condition| {
        condition_value(condition, variables, rpg, selected_entity, process_state)
            .map(|value| value == condition.equals)
            .unwrap_or(false)
    })
}

fn condition_value(
    condition: &VisualCondition,
    variables: &[VisualStateEntry],
    rpg: &VisualRpgState,
    selected_entity: Option<&VisualEntity>,
    process_state: Option<&VisualProcessState>,
) -> Option<VisualStateValue> {
    match condition.source.as_deref().unwrap_or("variable") {
        "variable" => variables
            .iter()
            .find(|entry| entry.key == condition.variable)
            .map(|entry| entry.value.clone()),
        "inventory_count" => rpg
            .inventory
            .iter()
            .find(|item| item.item_id == condition.variable)
            .map(|item| VisualStateValue::Number(i64::from(item.count))),
        "inventory_has" => rpg
            .inventory
            .iter()
            .find(|item| item.item_id == condition.variable)
            .map(|item| VisualStateValue::Bool(item.count > 0))
            .or(Some(VisualStateValue::Bool(false))),
        "quest_stage" => rpg
            .quests
            .iter()
            .find(|quest| quest.quest_id == condition.variable)
            .map(|quest| VisualStateValue::Number(quest.stage)),
        "quest_completed" => rpg
            .quests
            .iter()
            .find(|quest| quest.quest_id == condition.variable)
            .map(|quest| VisualStateValue::Bool(quest.completed)),
        "stat" => {
            let (owner_id, key) = condition
                .variable
                .split_once(':')
                .map(|(owner_id, key)| (Some(owner_id), key))
                .unwrap_or((None, condition.variable.as_str()));
            rpg.stats
                .iter()
                .find(|stat| stat.owner_id.as_deref() == owner_id && stat.key == key)
                .map(|stat| stat.value.clone())
        }
        "agent_phase" => variables
            .iter()
            .find(|entry| entry.key == "agent_phase")
            .map(|entry| entry.value.clone()),
        "process_phase" => process_state
            .map(|state| VisualStateValue::Text(state.phase.as_str().to_string()))
            .or_else(|| {
                variables
                    .iter()
                    .find(|entry| entry.key == "agent_process_phase")
                    .map(|entry| entry.value.clone())
            }),
        "selected_entity_flag" => selected_entity.map(|entity| {
            VisualStateValue::Bool(
                entity
                    .state_flags
                    .iter()
                    .any(|flag| flag == &condition.variable),
            )
        }),
        "selected_entity_metadata" => selected_entity.and_then(|entity| {
            entity
                .metadata
                .iter()
                .find(|(key, _)| key == &condition.variable)
                .map(|(_, value)| VisualStateValue::Text(value.clone()))
        }),
        _ => None,
    }
}

fn set_variable(variables: &mut Vec<VisualStateEntry>, key: &str, value: VisualStateValue) {
    match variables.iter_mut().find(|entry| entry.key == key) {
        Some(entry) => entry.value = value,
        None => variables.push(VisualStateEntry {
            key: key.to_string(),
            value,
        }),
    }
}

fn increment_variable(variables: &mut Vec<VisualStateEntry>, key: &str, amount: i64) {
    match variables.iter_mut().find(|entry| entry.key == key) {
        Some(entry) => match &mut entry.value {
            VisualStateValue::Number(value) => *value += amount,
            _ => entry.value = VisualStateValue::Number(amount),
        },
        None => variables.push(VisualStateEntry {
            key: key.to_string(),
            value: VisualStateValue::Number(amount),
        }),
    }
}

fn condition_guard_detail(conditions: &[VisualCondition]) -> Option<String> {
    if conditions.is_empty() {
        return None;
    }

    Some(format!(
        "requires {}",
        conditions
            .iter()
            .map(|condition| format!(
                "{}{}={}",
                condition
                    .source
                    .as_ref()
                    .map(|source| format!("{source}:"))
                    .unwrap_or_default(),
                condition.variable,
                condition.equals.as_debug_string()
            ))
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

fn dialogue_index(scene: &VisualScene, index: usize) -> Option<usize> {
    if scene.dialogue_lines.is_empty() {
        None
    } else {
        Some(index.min(scene.dialogue_lines.len() - 1))
    }
}

fn active_dialogue_line(scene: &VisualScene, index: usize) -> VisualDialogueLine {
    dialogue_index(scene, index)
        .and_then(|index| scene.dialogue_lines.get(index).cloned())
        .unwrap_or_else(|| VisualDialogueLine {
            speaker: scene.dialogue_speaker.clone(),
            text: scene.dialogue.clone(),
            portrait: None,
            metadata: Vec::new(),
        })
}

fn initial_dialogue_history(scene: &VisualScene, index: usize) -> Vec<VisualDialogueLine> {
    if scene.dialogue_lines.is_empty() {
        Vec::new()
    } else {
        vec![active_dialogue_line(scene, index)]
    }
}

fn entity_mode(entity: &VisualEntity) -> Option<String> {
    entity
        .metadata
        .iter()
        .find(|(key, value)| key == "mode" && !value.trim().is_empty())
        .map(|(_, value)| value.clone())
}

impl VisualMode for SceneRuntime {
    fn generation(&self) -> u64 {
        SceneRuntime::generation(self)
    }

    fn render_snapshot(&self) -> VisualRenderSnapshot {
        SceneRuntime::render_snapshot(self)
    }

    fn render_text_frame(&self, cols: usize, rows: usize) -> String {
        SceneRuntime::render_text_frame(self, cols, rows)
    }

    fn handle_input(&mut self, input: VisualInput) -> VisualModeOutcome {
        if let Some(outcome) = self.handle_layer_input(input) {
            return outcome;
        }

        if let Some(binding) = self
            .scene
            .mode
            .input_map
            .iter()
            .find(|binding| binding.input.trim() == input.binding_key())
            .cloned()
        {
            if !conditions_match(
                &binding.conditions,
                &self.scene.variables,
                &self.scene.rpg,
                self.selected_entity(),
                self.last_process_state.as_ref(),
            ) {
                self.status = format!(
                    "Input unavailable: {}",
                    condition_guard_detail(&binding.conditions)
                        .unwrap_or_else(|| "guard condition not met".to_string())
                );
                self.bump_generation();
                return VisualModeOutcome::Continue;
            }
            return self.run_mode_input_action(binding.action.trim());
        }

        self.run_mode_input_action(default_mode_input_action(input))
    }
}

fn default_mode_input_action(input: VisualInput) -> &'static str {
    match input {
        VisualInput::Close => "close",
        VisualInput::Reload => "reload",
        VisualInput::ToggleDebug => "toggle_debug",
        VisualInput::Activate => "activate_choice",
        VisualInput::Next => "select_next",
        VisualInput::Previous => "select_previous",
        VisualInput::Other => "ignore",
    }
}

pub fn truncate_to_screen(text: String, cols: usize, rows: usize) -> String {
    let max_cols = cols.max(1);
    text.lines()
        .take(rows.max(1))
        .map(|line| {
            let mut clipped = line.chars().take(max_cols).collect::<String>();
            clipped.push_str("\r\n");
            clipped
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scene_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("ci")
            .join("fixtures")
            .join("gameterm-scene")
            .join(name)
    }

    fn snapshot_for_filtering() -> VisualRenderSnapshot {
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
        }
    }

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

        runtime.toggle_debugger();
        let frame = runtime.render_text_frame(120, 40);
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
        let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
        let snapshot = runtime.render_snapshot();

        assert_eq!(snapshot.rpg.inventory.len(), 1);
        assert_eq!(snapshot.rpg.inventory[0].item_id, "scene-token");
        assert_eq!(snapshot.rpg.stats.len(), 1);
        assert_eq!(snapshot.rpg.quests[0].quest_id, "verify-scene-runtime");
        assert_eq!(snapshot.rpg.relationships[0].kind, "monitors");

        let report = runtime.debug_report();
        assert_eq!(report.rpg, snapshot.rpg);

        runtime.toggle_debugger();
        let frame = runtime.render_text_frame(120, 48);
        assert!(frame.contains("RPG:"));
        assert!(frame.contains("Inventory items: 1"));
        assert!(frame.contains("Relationships: 1"));
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
        assert!(snapshot.status.starts_with(
            "Resolve failed: Resolve action `Broken reward` references unknown quest"
        ));
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

    fn branching_dialogue_scene() -> VisualScene {
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
                conditions: vec![VisualCondition {
                    source: None,
                    variable: "active_track".to_string(),
                    equals: VisualStateValue::Text("visual-state".to_string()),
                }],
            },
            SceneAction {
                label: "Choose memory".to_string(),
                kind: SceneActionKind::AdvanceDialogue { target: 2 },
                conditions: vec![VisualCondition {
                    source: None,
                    variable: "active_track".to_string(),
                    equals: VisualStateValue::Text("memory".to_string()),
                }],
            },
        ];
        scene
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
            entry.key == "agent_phase"
                && entry.value == VisualStateValue::Text("complete".to_string())
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
            entry.key == "agent_phase"
                && entry.value == VisualStateValue::Text("running".to_string())
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
            entry.key == "agent_phase"
                && entry.value == VisualStateValue::Text("running".to_string())
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
        assert_eq!(runtime.view(), VisualView::TileDebugger);
    }

    #[test]
    fn mode_toggle_debug_input_changes_view_and_generation() {
        let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
        let initial_generation = runtime.generation();

        let outcome = runtime.handle_input(VisualInput::ToggleDebug);

        assert_eq!(outcome, VisualModeOutcome::Continue);
        assert_eq!(runtime.view(), VisualView::TileDebugger);
        assert!(runtime.generation() > initial_generation);
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

        runtime.toggle_debugger();
        let frame = runtime.render_text_frame(120, 40);
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

        runtime.toggle_debugger();
        let frame = runtime.render_text_frame(120, 40);
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
        assert!(report.transition_history.iter().any(|event| {
            event.kind == "transition" && event.detail == "story dialogue -> choice"
        }));
        assert!(report
            .transition_history
            .iter()
            .any(|event| { event.kind == "patch" && event.detail == "mux failed: bad patch" }));

        runtime.toggle_debugger();
        let frame = runtime.render_text_frame(120, 40);
        assert!(frame.contains("History:"));
        assert!(frame.contains("transition: story dialogue -> choice"));
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
        let mut runtime = SceneRuntime::new(scene).unwrap();

        let report = runtime.debug_report();

        assert!(!report.selected_choice_enabled);
        assert_eq!(
            report.selected_choice_guard_detail.as_deref(),
            Some("requires workspace_level=2")
        );

        runtime.toggle_debugger();
        let frame = runtime.render_text_frame(100, 40);
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

        runtime.toggle_debugger();
        let frame = runtime.render_text_frame(120, 40);
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
                conditions: vec![],
            },
            SceneAction {
                label: "Import story".to_string(),
                kind: SceneActionKind::ImportStoryState {
                    path: "state/story.json".to_string(),
                },
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

        runtime.toggle_debugger();
        let frame = runtime.render_text_frame(120, 40);
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
    fn run_command_action_requires_explicit_argv() {
        let mut scene = VisualScene::demo();
        scene.choices = vec![SceneAction {
            label: "Run empty".to_string(),
            kind: SceneActionKind::RunCommand {
                argv: Vec::new(),
                cwd: None,
                target: RunCommandTarget::Tab,
            },
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
    fn debugger_frame_contains_scene_source_status() {
        let source = VisualSceneSource::new("/tmp/default.json", VisualSceneLoadStatus::Loaded, 3);
        let mut runtime = SceneRuntime::new_with_source(VisualScene::demo(), source).unwrap();
        runtime.toggle_debugger();
        runtime.activate_choice();
        let frame = runtime.render_text_frame(200, 80);

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
            .any(|entry| entry.key == "workspace_level"
                && entry.value == VisualStateValue::Number(1)));
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
            .any(|(key, value)| key == "reference" && value == "Ren'Py scene flow"));
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

        runtime.toggle_debugger();
        let frame = runtime.render_text_frame(100, 32);
        assert!(frame.contains("Choice label: Run cargo check -p gameterm-visual"));
        assert!(frame.contains("Choice kind: RunCommand"));
        assert!(frame
            .contains("Pending action: RunCommand argv=cargo check -p gameterm-visual target=tab"));
    }

    #[test]
    fn truncate_to_screen_clips_rows_and_columns() {
        let frame = truncate_to_screen("abcdef\n123456\nxyz".to_string(), 3, 2);
        assert_eq!(frame, "abc\r\n123\r\n");
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
            VisualSpriteManifest::from_json(
                r#"{ "sprites": [{ "id": " ", "path": "sprite.png" }] }"#
            ),
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
            status: Some("Source tracked".to_string()),
        };

        runtime
            .apply_scene_patch_with_source(patch, Some("mux".to_string()), Some(42))
            .unwrap();

        let report = runtime.debug_report();
        assert_eq!(report.last_patch_transport.as_deref(), Some("mux"));
        assert_eq!(report.last_patch_source_pane_id, Some(42));

        runtime.toggle_debugger();
        let frame = runtime.render_text_frame(100, 40);
        assert!(frame.contains("Last patch: mux from pane 42"));
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
        runtime.toggle_debugger();
        let frame = runtime.render_text_frame(100, 40);
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
        let patch =
            VisualScenePatch::load_from_path(scene_fixture_path("patch-status.json")).unwrap();
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
            VisualScenePatch::load_from_path(scene_fixture_path("patch-unknown-entity.json"))
                .unwrap();
        let mut runtime = SceneRuntime::new(scene).unwrap();

        assert!(matches!(
            runtime.apply_scene_patch(patch),
            Err(VisualScenePatchError::UnknownEntityId(id)) if id == "missing-entity"
        ));
    }
}
