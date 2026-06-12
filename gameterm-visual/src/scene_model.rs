use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::schema::{
    is_default_scene_mode, VisualLayerState, VisualModeDescriptor, VisualModeLifecycle,
};
use crate::validation::{
    is_supported_action_policy_origin, is_supported_action_policy_risk,
    is_supported_action_policy_scope, is_supported_mode_input, is_supported_mode_input_action,
    validate_dialogue_lines, validate_layers, validate_rpg_state, validate_state_entries,
    validate_state_operations, VisualDialogueLineError, VisualStateEntryError,
};
use crate::vn_layout::{VnDialogueScrollMetrics, VnOverlayDebugOverrides};
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

fn is_empty_stage(stage: &VisualStage) -> bool {
    stage.is_empty()
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualStage {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layers: Vec<VisualStageLayer>,
}

impl VisualStage {
    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualStageLayer {
    pub layer_id: String,
    #[serde(default)]
    pub zorder: i32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub displayables: Vec<VisualStageDisplayable>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualStageDisplayable {
    pub tag: String,
    pub sprite: String,
    #[serde(default)]
    pub placement: VisualStagePlacement,
    #[serde(default)]
    pub zorder: i32,
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub visible: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisualStagePlacement {
    Fullscreen,
    Left,
    #[default]
    Center,
    Right,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy: Option<SceneActionPolicy>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<VisualCondition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneActionPolicy {
    pub origin: String,
    pub risk: String,
    pub scope: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub requires_confirmation: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualCommandFilter {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub risk: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub enabled_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualCommandOption {
    pub choice_index: usize,
    pub label: String,
    pub action_kind: String,
    pub origin: String,
    pub risk: String,
    pub scope: String,
    pub requires_confirmation: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guard_detail: Option<String>,
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

pub(crate) fn is_empty_rpg_state(state: &VisualRpgState) -> bool {
    state.is_empty()
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
    #[serde(default, skip_serializing_if = "is_empty_stage")]
    pub stage: VisualStage,
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
pub struct VisualRenderStageDisplayable {
    pub layer_id: String,
    pub tag: String,
    pub sprite: String,
    pub placement: VisualStagePlacement,
    pub layer_zorder: i32,
    pub zorder: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stage: Vec<VisualRenderStageDisplayable>,
    pub entities: Vec<VisualRenderEntity>,
    pub dialogue_speaker: String,
    pub dialogue: String,
    pub dialogue_index: Option<usize>,
    pub dialogue_history: Vec<VisualDialogueLine>,
    pub status: String,
    pub choices: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overlay_cols: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overlay_rows: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vn_dialogue_scroll: Option<VnDialogueScrollMetrics>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vn_layout_debug: Option<VnOverlayDebugOverrides>,
    #[serde(default = "default_interactive_debug_menu")]
    pub interactive_debug_menu: VisualInteractiveDebugMenu,
    #[serde(default, skip_serializing_if = "is_false")]
    pub vn_voice_hold_active: bool,
}

fn default_interactive_debug_menu() -> VisualInteractiveDebugMenu {
    VisualInteractiveDebugMenu::SceneLayout
}

impl VisualRenderSnapshot {
    pub fn overlay_layout_dims(&self) -> Option<(usize, usize)> {
        self.overlay_cols
            .and_then(|cols| self.overlay_rows.map(|rows| (cols, rows)))
    }
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

fn default_true() -> bool {
    true
}

fn is_true(value: &bool) -> bool {
    *value
}

fn is_false(value: &bool) -> bool {
    !*value
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
    Left,
    Right,
    ScrollDialogueUp,
    ScrollDialogueDown,
    Char(char),
    Backspace,
    Other,
}

impl VisualInput {
    pub(crate) fn binding_key(self) -> &'static str {
        match self {
            Self::Close => "close",
            Self::Reload => "reload",
            Self::ToggleDebug => "toggle_debug",
            Self::Activate => "activate",
            Self::Next => "next",
            Self::Previous => "previous",
            Self::Left => "left",
            Self::Right => "right",
            Self::ScrollDialogueUp | Self::ScrollDialogueDown => "other",
            Self::Char(_) | Self::Backspace | Self::Other => "other",
        }
    }
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
    #[error("scene stage layer id must be non-empty")]
    EmptyStageLayerId,
    #[error("duplicate scene stage layer id `{0}`")]
    DuplicateStageLayerId(String),
    #[error("scene stage displayable tag must be non-empty")]
    EmptyStageDisplayableTag,
    #[error("scene stage displayable `{tag}` sprite must be non-empty")]
    EmptyStageDisplayableSprite { tag: String },
    #[error("duplicate scene stage displayable tag `{layer_id}:{tag}`")]
    DuplicateStageDisplayableTag { layer_id: String, tag: String },
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
    #[error("choice action `{label}` policy origin must be non-empty")]
    EmptyActionPolicyOrigin { label: String },
    #[error("choice action `{label}` policy risk must be non-empty")]
    EmptyActionPolicyRisk { label: String },
    #[error("choice action `{label}` policy scope must be non-empty")]
    EmptyActionPolicyScope { label: String },
    #[error("choice action `{label}` has unknown policy origin `{origin}`")]
    UnknownActionPolicyOrigin { label: String, origin: String },
    #[error("choice action `{label}` has unknown policy risk `{risk}`")]
    UnknownActionPolicyRisk { label: String, risk: String },
    #[error("choice action `{label}` has unknown policy scope `{scope}`")]
    UnknownActionPolicyScope { label: String, scope: String },
    #[error("choice action `{label}` policy summary must be non-empty when provided")]
    EmptyActionPolicySummary { label: String },
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
    #[error("relationship source id `{0}` does not reference a scene entity")]
    UnknownRelationshipSourceId(String),
    #[error("relationship target id `{0}` does not reference a scene entity")]
    UnknownRelationshipTargetId(String),
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

pub(crate) fn validate_stage(stage: &VisualStage) -> Result<(), VisualSceneError> {
    let mut layer_ids = HashSet::new();
    for layer in &stage.layers {
        if layer.layer_id.trim().is_empty() {
            return Err(VisualSceneError::EmptyStageLayerId);
        }
        if !layer_ids.insert(layer.layer_id.as_str()) {
            return Err(VisualSceneError::DuplicateStageLayerId(
                layer.layer_id.clone(),
            ));
        }

        let mut tags = HashSet::new();
        for displayable in &layer.displayables {
            if displayable.tag.trim().is_empty() {
                return Err(VisualSceneError::EmptyStageDisplayableTag);
            }
            if displayable.sprite.trim().is_empty() {
                return Err(VisualSceneError::EmptyStageDisplayableSprite {
                    tag: displayable.tag.clone(),
                });
            }
            if !tags.insert(displayable.tag.as_str()) {
                return Err(VisualSceneError::DuplicateStageDisplayableTag {
                    layer_id: layer.layer_id.clone(),
                    tag: displayable.tag.clone(),
                });
            }
        }
    }
    Ok(())
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
        validate_stage(&self.stage)?;
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
        for relationship in &self.rpg.relationships {
            if !ids.contains(relationship.source_id.as_str()) {
                return Err(VisualSceneError::UnknownRelationshipSourceId(
                    relationship.source_id.clone(),
                ));
            }
            if !ids.contains(relationship.target_id.as_str()) {
                return Err(VisualSceneError::UnknownRelationshipTargetId(
                    relationship.target_id.clone(),
                ));
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
            if let Some(policy) = &choice.policy {
                let origin = policy.origin.trim();
                let risk = policy.risk.trim();
                let scope = policy.scope.trim();
                if origin.is_empty() {
                    return Err(VisualSceneError::EmptyActionPolicyOrigin {
                        label: choice.label.clone(),
                    });
                }
                if risk.is_empty() {
                    return Err(VisualSceneError::EmptyActionPolicyRisk {
                        label: choice.label.clone(),
                    });
                }
                if scope.is_empty() {
                    return Err(VisualSceneError::EmptyActionPolicyScope {
                        label: choice.label.clone(),
                    });
                }
                if !is_supported_action_policy_origin(origin) {
                    return Err(VisualSceneError::UnknownActionPolicyOrigin {
                        label: choice.label.clone(),
                        origin: policy.origin.clone(),
                    });
                }
                if !is_supported_action_policy_risk(risk) {
                    return Err(VisualSceneError::UnknownActionPolicyRisk {
                        label: choice.label.clone(),
                        risk: policy.risk.clone(),
                    });
                }
                if !is_supported_action_policy_scope(scope) {
                    return Err(VisualSceneError::UnknownActionPolicyScope {
                        label: choice.label.clone(),
                        scope: policy.scope.clone(),
                    });
                }
                if policy
                    .summary
                    .as_ref()
                    .is_some_and(|summary| summary.trim().is_empty())
                {
                    return Err(VisualSceneError::EmptyActionPolicySummary {
                        label: choice.label.clone(),
                    });
                }
            }
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
            stage: VisualStage::default(),
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
                        ("reference".to_string(), "visual novel scene flow".to_string()),
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
                    policy: None,
                    conditions: vec![],
                },
                SceneAction {
                    label: "Open MIGRATION.md".to_string(),
                    kind: SceneActionKind::OpenFile {
                        path: "MIGRATION.md".to_string(),
                    },
                    policy: None,
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
                    policy: None,
                    conditions: vec![],
                },
            ],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VisualView {
    Scene,
    CommandSelection,
    TileDebugger,
    VnLayoutDebugger,
    /// Cozy-game navigation shell: a boot "press start" screen, the main menu,
    /// and the in-scene Tab-cycle mode screens.
    Boot,
    MainMenu,
    CharacterSelect,
    StageSelect,
    SettingMode,
}

impl VisualView {
    /// True for the navigation-shell screens (boot, menu, and mode cycle),
    /// which own keyboard input directly instead of the compose dock.
    pub fn is_shell(self) -> bool {
        matches!(
            self,
            Self::Boot
                | Self::MainMenu
                | Self::CharacterSelect
                | Self::StageSelect
                | Self::SettingMode
        )
    }

    /// True for the Tab-cycle mode screens reached from within a scene.
    pub fn is_mode_cycle(self) -> bool {
        matches!(
            self,
            Self::CharacterSelect | Self::StageSelect | Self::SettingMode
        )
    }

    /// True for the screens eligible for the plain-backdrop preference
    /// (`scene_shell_backdrop = "Black"`): the navigation shell and the
    /// debug panes. The preference itself lives in the GUI config; this only
    /// names which screens it applies to.
    pub fn uses_plain_backdrop(self) -> bool {
        self.is_shell() || matches!(self, Self::TileDebugger | Self::VnLayoutDebugger)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VisualInteractiveDebugMenu {
    TileDebugMenu,
    #[serde(alias = "SceneModeDebugMenu")]
    SceneLayout,
    Text,
    Voice,
    Compose,
    Runtime,
}

impl VisualInteractiveDebugMenu {
    pub const SCENE_SECTIONS: [Self; 5] = [
        Self::SceneLayout,
        Self::Text,
        Self::Voice,
        Self::Compose,
        Self::Runtime,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::TileDebugMenu => "Tile",
            Self::SceneLayout => "Scene Layout",
            Self::Text => "Text",
            Self::Voice => "Voice",
            Self::Compose => "Compose",
            Self::Runtime => "Runtime",
        }
    }

    pub(crate) fn next_scene_section(self) -> Self {
        let idx = Self::SCENE_SECTIONS
            .iter()
            .position(|section| *section == self)
            .unwrap_or(0);
        Self::SCENE_SECTIONS[(idx + 1) % Self::SCENE_SECTIONS.len()]
    }

    pub(crate) fn previous_scene_section(self) -> Self {
        let idx = Self::SCENE_SECTIONS
            .iter()
            .position(|section| *section == self)
            .unwrap_or(0);
        Self::SCENE_SECTIONS[(idx + Self::SCENE_SECTIONS.len() - 1) % Self::SCENE_SECTIONS.len()]
    }
}
