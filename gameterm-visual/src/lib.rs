use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

mod actions;
mod conditions;
mod debug;
mod patch;
pub mod render;
mod runtime_input;
mod runtime_selection;
mod runtime_status;
mod runtime_status_methods;
mod schema;
mod story_state;
mod validation;
mod vn_asset_intake;
mod vn_script_import;
mod workspace_scene;

use actions::{action_kind_name, action_policy_summary, derived_action_policy};
use conditions::{condition_guard_detail, conditions_match};
pub use debug::VisualSceneDebugReport;
pub use patch::{
    VisualSceneDialoguePatch, VisualSceneEntityPatch, VisualScenePatch, VisualScenePatchError,
};
pub use render::{intersecting_entities_for_row, visible_tiles_for_row};
use runtime_input::default_mode_input_action;
use runtime_status::{
    clip_text, format_layer_summary, format_metadata_summary, format_process_summary,
    format_relationship_metadata, format_relationship_summary, format_state_summary,
    relationship_entity_label, wrap_text,
};
#[cfg(test)]
pub(crate) use schema::default_scene_mode;
pub(crate) use schema::is_default_scene_mode;
pub use schema::{
    VisualInputBinding, VisualLayerState, VisualLayerTransition, VisualLayerTransitionReport,
    VisualModeDescriptor, VisualModeLifecycle, VisualRuntimeEvent,
};
pub use story_state::{VisualStoryState, VisualStoryStateError};
pub(crate) use validation::{
    is_supported_action_policy_origin, is_supported_action_policy_risk,
    is_supported_action_policy_scope, is_supported_mode_input, is_supported_mode_input_action,
    relationship_key, validate_dialogue_lines, validate_layers, validate_rpg_state,
    validate_state_entries, validate_state_operations, VisualDialogueLineError,
    VisualStateEntryError,
};
pub use vn_asset_intake::{
    run_vn_asset_intake, VnAssetAttributionManifest, VnAssetBindingCharacter, VnAssetBindings,
    VnAssetCatalog, VnAssetCatalogPolicy, VnAssetCatalogSource, VnAssetIntakeError,
    VnAssetIntakeOptions, VnAssetIntakeReport, VnAssetIntakeWarning, VnAssetIntakeWarningKind,
    VnAssetUsedAsset,
};
pub use vn_script_import::{
    import_vn_script_scene, VnScriptAssetAttribution, VnScriptAttributionManifest, VnScriptDialect,
    VnScriptImportError, VnScriptImportOptions, VnScriptImportReport, VnScriptImportWarning,
    VnScriptImportWarningKind,
};
pub use workspace_scene::{
    generate_workspace_context_error_scene, generate_workspace_scene, ScenePaneContext,
    SceneWorkspaceContext, WorkspaceSceneReport,
};

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

fn is_empty_rpg_state(state: &VisualRpgState) -> bool {
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stage: Vec<VisualRenderStageDisplayable>,
    pub entities: Vec<VisualRenderEntity>,
    pub dialogue_speaker: String,
    pub dialogue: String,
    pub dialogue_index: Option<usize>,
    pub dialogue_history: Vec<VisualDialogueLine>,
    pub status: String,
    pub choices: Vec<String>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
enum VisualComposePhase {
    Idle,
    Running,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum VisualComposeRole {
    User,
    Assistant,
    System,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct VisualComposeMessage {
    role: VisualComposeRole,
    text: String,
}

#[derive(Debug, Clone)]
struct VisualComposeRuntimeState {
    phase: VisualComposePhase,
    history: Vec<VisualComposeMessage>,
    last_prompt: Option<String>,
    last_reply: Option<String>,
}

impl VisualComposeRuntimeState {
    fn new() -> Self {
        Self {
            phase: VisualComposePhase::Idle,
            history: Vec::new(),
            last_prompt: None,
            last_reply: None,
        }
    }

    fn push_message(&mut self, role: VisualComposeRole, text: String) {
        if text.trim().is_empty() {
            return;
        }
        const MAX_COMPOSE_HISTORY: usize = 20;
        self.history.push(VisualComposeMessage { role, text });
        if self.history.len() > MAX_COMPOSE_HISTORY {
            let excess = self.history.len() - MAX_COMPOSE_HISTORY;
            self.history.drain(0..excess);
        }
    }

    fn set_phase_and_history(&mut self, phase: VisualComposePhase) {
        self.phase = phase;
    }

    fn mark_running(&mut self, prompt: &str) {
        self.set_phase_and_history(VisualComposePhase::Running);
        self.last_prompt = Some(prompt.to_string());
        self.last_reply = None;
        self.push_message(VisualComposeRole::User, prompt.to_string());
    }

    fn mark_succeeded(&mut self, reply: &str) {
        self.set_phase_and_history(VisualComposePhase::Succeeded);
        self.last_reply = Some(reply.to_string());
        self.push_message(VisualComposeRole::Assistant, reply.to_string());
    }

    fn mark_failed(&mut self, reason: &str) {
        self.set_phase_and_history(VisualComposePhase::Failed);
        self.last_reply = Some(reason.to_string());
        self.push_message(VisualComposeRole::Error, reason.to_string());
    }
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

fn validate_stage(stage: &VisualStage) -> Result<(), VisualSceneError> {
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
    compose_state: VisualComposeRuntimeState,
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
            compose_state: VisualComposeRuntimeState::new(),
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

    pub fn command_options(&self) -> Vec<VisualCommandOption> {
        self.scene
            .choices
            .iter()
            .enumerate()
            .map(|(choice_index, choice)| {
                let policy = derived_action_policy(choice);
                let enabled = conditions_match(
                    &choice.conditions,
                    &self.scene.variables,
                    &self.scene.rpg,
                    self.selected_entity(),
                    self.last_process_state.as_ref(),
                );
                VisualCommandOption {
                    choice_index,
                    label: choice.label.clone(),
                    action_kind: action_kind_name(&choice.kind),
                    origin: policy.origin,
                    risk: policy.risk,
                    scope: policy.scope,
                    requires_confirmation: policy.requires_confirmation,
                    summary: policy.summary,
                    enabled,
                    guard_detail: condition_guard_detail(&choice.conditions),
                }
            })
            .collect()
    }

    pub fn filtered_command_options(
        &self,
        filter: &VisualCommandFilter,
    ) -> Vec<VisualCommandOption> {
        self.command_options()
            .into_iter()
            .filter(|option| command_option_matches_filter(option, filter))
            .collect()
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
        self.apply_mode_enter_status();
        self.bump_generation();
        Ok(())
    }
}

fn command_option_matches_filter(
    option: &VisualCommandOption,
    filter: &VisualCommandFilter,
) -> bool {
    if filter.enabled_only && !option.enabled {
        return false;
    }
    if let Some(action_kind) = &filter.action_kind {
        if option.action_kind != action_kind.trim() {
            return false;
        }
    }
    if let Some(risk) = &filter.risk {
        if option.risk != risk.trim() {
            return false;
        }
    }
    if let Some(scope) = &filter.scope {
        if option.scope != scope.trim() {
            return false;
        }
    }
    if let Some(query) = &filter.query {
        let query = query.trim().to_lowercase();
        if query.is_empty() {
            return true;
        }
        let haystack = format!(
            "{} {} {} {} {} {}",
            option.label,
            option.action_kind,
            option.origin,
            option.risk,
            option.scope,
            option.summary.as_deref().unwrap_or_default()
        )
        .to_lowercase();
        return haystack.contains(&query);
    }
    true
}

impl SceneRuntime {
    pub fn toggle_debugger(&mut self) {
        self.view = match self.view {
            VisualView::Scene => VisualView::TileDebugger,
            VisualView::CommandSelection => VisualView::TileDebugger,
            VisualView::TileDebugger => VisualView::Scene,
        };
        self.bump_generation();
    }

    pub fn show_command_selection(&mut self) {
        if self.view != VisualView::CommandSelection {
            self.view = VisualView::CommandSelection;
            self.bump_generation();
        }
    }

    pub fn hide_command_selection(&mut self) {
        if self.view == VisualView::CommandSelection {
            self.view = VisualView::Scene;
            self.bump_generation();
        }
    }

    pub fn toggle_command_selection(&mut self) {
        self.view = match self.view {
            VisualView::CommandSelection => VisualView::Scene,
            VisualView::Scene | VisualView::TileDebugger => VisualView::CommandSelection,
        };
        self.bump_generation();
    }

    pub fn run_mode_enter_hooks(&mut self) {
        if self.apply_mode_enter_status() {
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

    fn apply_mode_enter_status(&mut self) -> bool {
        if let Some(status) = &self.scene.mode.lifecycle.enter_status {
            self.status = status.clone();
            true
        } else {
            false
        }
    }
}

impl SceneRuntime {
    fn apply_state_operations(
        &mut self,
        label: &str,
        operations: &[VisualStateOperation],
    ) -> Result<usize, VisualSceneError> {
        actions::apply_state_operations(self, label, operations)
    }

    fn resolve_action_path(&self, path: &str) -> PathBuf {
        let raw_path = PathBuf::from(path);
        if raw_path.is_absolute() {
            raw_path
        } else {
            self.action_base_dir.join(raw_path)
        }
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
            VisualView::CommandSelection => self.render_command_selection(cols, rows),
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
            stage: self.render_stage(),
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

    fn render_tiles(&self) -> Vec<VisualRenderTile> {
        if !self.scene.stage.is_empty() {
            return Vec::new();
        }
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

    fn render_stage(&self) -> Vec<VisualRenderStageDisplayable> {
        let mut displayables = Vec::new();
        for layer in &self.scene.stage.layers {
            for displayable in &layer.displayables {
                if displayable.visible {
                    displayables.push(VisualRenderStageDisplayable {
                        layer_id: layer.layer_id.clone(),
                        tag: displayable.tag.clone(),
                        sprite: displayable.sprite.clone(),
                        placement: displayable.placement,
                        layer_zorder: layer.zorder,
                        zorder: displayable.zorder,
                    });
                }
            }
        }
        displayables.sort_by(|left, right| {
            left.layer_zorder
                .cmp(&right.layer_zorder)
                .then(left.zorder.cmp(&right.zorder))
                .then(left.layer_id.cmp(&right.layer_id))
                .then(left.tag.cmp(&right.tag))
        });
        displayables
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
        if !self.scene.stage.is_empty() {
            return self.render_staged_scene(cols, rows);
        }

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
            let metadata = format_metadata_summary(&entity.metadata, 4);
            if !metadata.is_empty() {
                out.push_str(&format!("Details: {metadata}\r\n"));
            }
            if let Some(summary) = format_relationship_summary(&self.scene, &entity.id, 3) {
                out.push_str(&format!("Relationships: {summary}\r\n"));
            }
        }
        out.push_str(&format!(
            "Mode: {} ({})\r\n",
            self.scene.mode.label, self.scene.mode.mode_id
        ));
        if !self.scene.layers.is_empty() {
            out.push_str(&format!(
                "Layers: {}\r\n",
                format_layer_summary(&self.scene.layers)
            ));
        }
        if let Some(process_state) = &self.last_process_state {
            out.push_str(&format!(
                "Process: {}\r\n",
                format_process_summary(process_state)
            ));
        }
        if !self.scene.variables.is_empty() {
            out.push_str(&format!(
                "State: {}\r\n",
                format_state_summary(&self.scene.variables, 5)
            ));
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
        let mut last_choice_group: Option<String> = None;
        for (idx, choice) in self.scene.choices.iter().enumerate() {
            let choice_group = action_kind_name(&choice.kind);
            if last_choice_group.as_deref() != Some(choice_group.as_str()) {
                out.push_str(&format!("[{choice_group}]\r\n"));
                last_choice_group = Some(choice_group);
            }
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
            out.push_str(&format!(
                "{marker} {}{}  {}\r\n",
                choice.label,
                guard,
                action_policy_summary(choice)
            ));
        }
        out.push_str(&format!("\r\nStatus: {}\r\n", self.status));
        truncate_to_screen(out, cols, rows)
    }

    fn render_staged_scene(&self, cols: usize, rows: usize) -> String {
        let cols = cols.max(1);
        let rows = rows.max(1);
        let dock_height = if rows >= 10 { 1 } else { 0 };
        let fullscreen_vn_layout = rows >= 40;
        let box_margin = if fullscreen_vn_layout {
            ((cols as f32) * 0.033).round() as usize
        } else {
            3
        }
        .min(cols.saturating_sub(1));
        let box_width = cols.saturating_sub(box_margin * 2).max(20);
        let box_height = if fullscreen_vn_layout {
            let panel_top = ((rows as f32) * 0.085).round() as usize;
            let panel_bottom = ((rows as f32) * 0.896).round() as usize;
            panel_bottom.saturating_sub(panel_top).max(7)
        } else if rows >= 18 {
            7
        } else {
            4
        }
        .min(rows.saturating_sub(dock_height).max(1));
        let top_budget = if fullscreen_vn_layout {
            ((rows as f32) * 0.085).round() as usize
        } else {
            rows.saturating_sub(box_height + dock_height)
        }
        .min(rows.saturating_sub(box_height + dock_height));
        let mut top = Vec::new();

        top.push(self.scene.title.clone());
        top.push(
            "Scene Mode  [enter: action] [tab: debugger] [r: reload] [esc/q: close]".to_string(),
        );
        if self.scene_source.load_status == VisualSceneLoadStatus::ReloadFailed {
            if let Some(error) = &self.scene_source.last_error {
                top.push(format!("Reload failed: {error}"));
            }
        }
        if let Some(entity) = self.selected_entity() {
            top.push(format!(
                "Selected: {} [{:?}] sprite={} flags={}",
                entity.label,
                entity.kind,
                entity.sprite,
                entity.state_flags.join(", ")
            ));
        }
        top.push(format!(
            "Mode: {} ({})",
            self.scene.mode.label, self.scene.mode.mode_id
        ));
        top.push(format!(
            "Stage: {} layer(s), {} displayable(s)",
            self.scene.stage.layers.len(),
            self.scene
                .stage
                .layers
                .iter()
                .map(|layer| layer.displayables.len())
                .sum::<usize>()
        ));
        if !self.scene.variables.is_empty() {
            top.push(format!(
                "State: {}",
                format_state_summary(&self.scene.variables, 4)
            ));
        }
        if !self.status.is_empty() {
            top.push(format!("Status: {}", self.status));
        }

        let mut frame = String::new();
        for line in top.into_iter().take(top_budget) {
            frame.push_str(&line);
            frame.push_str("\r\n");
        }
        let used_top_rows = frame.lines().count();
        for _ in used_top_rows..top_budget {
            frame.push_str("\r\n");
        }

        frame.push_str(&self.render_vn_dialogue_box(box_margin, box_width, box_height));
        if dock_height > 0 {
            frame.push_str(&self.render_vn_dock(cols, box_margin));
        }
        truncate_to_screen(frame, cols, rows)
    }

    fn render_vn_dialogue_box(&self, margin: usize, width: usize, height: usize) -> String {
        const VN_PANEL_TEXT_INSET: usize = 3;

        let width = width.max(6);
        let height = height.max(1);
        let inner_width = width.saturating_sub(VN_PANEL_TEXT_INSET * 2);
        let indent = " ".repeat(margin + VN_PANEL_TEXT_INSET);
        let dialogue = self.active_dialogue_line();
        let mut lines = Vec::new();
        lines.push(format!("{}:", dialogue.speaker));
        lines.extend(wrap_text(&dialogue.text, inner_width));

        if !self.scene.choices.is_empty() && height >= 6 {
            lines.push(String::new());
            for (idx, choice) in self.scene.choices.iter().take(2).enumerate() {
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
                lines.push(format!("{marker} {}{guard}", choice.label));
            }
        }

        let mut out = String::new();
        for idx in 0..height {
            let line = lines.get(idx).map(String::as_str).unwrap_or("");
            out.push_str(&format!(
                "{indent}{:<inner_width$}\r\n",
                clip_text(line, inner_width)
            ));
        }
        out
    }

    fn render_vn_dock(&self, cols: usize, margin: usize) -> String {
        const VN_PANEL_TEXT_INSET: usize = 3;

        let choice_count = self.scene.choices.len();
        let selected_choice = if choice_count == 0 {
            "none".to_string()
        } else {
            format!("{}/{}", self.selected_choice + 1, choice_count)
        };
        let dock = format!(
            " Compose: _ | scene={} | choice={} | dialogue={} | controls=enter/tab/r/esc ",
            self.scene.mode.mode_id,
            selected_choice,
            dialogue_index(&self.scene, self.dialogue_index)
                .map(|idx| (idx + 1).to_string())
                .unwrap_or_else(|| "static".to_string())
        );
        let indent_width = margin + VN_PANEL_TEXT_INSET;
        let width = cols.saturating_sub(indent_width * 2).max(1);
        let indent = " ".repeat(indent_width);
        format!("{indent}{:<width$}\r\n", clip_text(&dock, width))
    }

    fn render_command_selection(&self, cols: usize, rows: usize) -> String {
        let mut out = String::new();
        out.push_str("GameTerm Command Selection\r\n");
        out.push_str(
            "[arrows/hjkl: select command] [enter: action] [tab: debugger] [esc/q: close]\r\n\r\n",
        );
        out.push_str("Filter: none\r\n");
        if let Some(entity) = self.selected_entity() {
            out.push_str(&format!(
                "Selected entity: {} ({})\r\n",
                entity.label, entity.id
            ));
        }
        out.push_str("\r\nCommands:\r\n");
        for option in self.command_options() {
            let marker = if option.choice_index == self.selected_choice {
                ">"
            } else {
                " "
            };
            let lock = if option.enabled { "" } else { " locked" };
            let confirm = if option.requires_confirmation {
                " confirm=true"
            } else {
                ""
            };
            let guard = option
                .guard_detail
                .as_ref()
                .map(|detail| format!(" guard={detail}"))
                .unwrap_or_default();
            let summary = option
                .summary
                .as_ref()
                .map(|summary| format!(" summary={summary}"))
                .unwrap_or_default();
            out.push_str(&format!(
                "{marker} #{:02} {:<18} {:<14} {:<16} {:<14} {}{}{}{}{}\r\n",
                option.choice_index,
                option.action_kind,
                option.risk,
                option.origin,
                option.scope,
                option.label,
                lock,
                confirm,
                guard,
                summary
            ));
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
        if let Some(policy) = &report.selected_choice_policy {
            out.push_str(&format!("  Choice policy: {policy}\r\n"));
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
            for relationship in &report.rpg.relationships {
                out.push_str(&format!(
                    "    {} --{}({})--> {}{}\r\n",
                    relationship_entity_label(&self.scene, &relationship.source_id),
                    relationship.kind,
                    relationship.value,
                    relationship_entity_label(&self.scene, &relationship.target_id),
                    format_relationship_metadata(&relationship.metadata)
                ));
            }
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
            if let Some(summary) = format_relationship_summary(&self.scene, &entity.id, 8) {
                out.push_str("\r\nSelected relationships:\r\n");
                out.push_str(&format!("  {summary}\r\n"));
            }
        }
        truncate_to_screen(out, cols, rows)
    }
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
    use test_support::scene_fixture_path;

    mod test_support {
        use std::path::PathBuf;

        pub(super) fn scene_fixture_path(name: &str) -> PathBuf {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .join("ci")
                .join("fixtures")
                .join("gameterm-scene")
                .join(name)
        }
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
        let frame = runtime.render_text_frame(200, 80);
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
    fn relationships_are_visible_in_normal_view_and_debugger() {
        let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
        runtime.select_next_entity();
        runtime.select_next_entity();

        let normal = runtime.render_text_frame(120, 40);
        assert!(normal.contains("Relationships: out=1 in=0"));
        assert!(normal.contains("-> Render Scene (task-render) monitors"));

        let debugger = runtime.render_debugger(140, 80);
        assert!(debugger
            .contains("Audit Agent (agent-audit) --monitors(2)--> Render Scene (task-render)"));
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
        let scene =
            VisualScene::load_from_path(scene_fixture_path("workspace-agent.json")).unwrap();
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
            entry.key == "agent_phase"
                && entry.value == VisualStateValue::Text("completed".to_string())
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
            VisualScene::load_from_path(scene_fixture_path("multi-agent-coordination.json"))
                .unwrap();
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
            entry.key == "audit_phase"
                && entry.value == VisualStateValue::Text("completed".to_string())
        }));
        assert!(snapshot.variables.iter().any(|entry| {
            entry.key == "build_phase"
                && entry.value == VisualStateValue::Text("completed".to_string())
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
    fn scene_fixture_renpy_demo_import_loads_story_choices() {
        let scene = VisualScene::load_from_path(scene_fixture_path("renpy-demo.json")).unwrap();
        let mut runtime = SceneRuntime::new(scene).unwrap();

        let snapshot = runtime.render_snapshot();
        assert_eq!(snapshot.title, "VN Script Demo Import");
        assert!(snapshot.variables.iter().any(|entry| {
            entry.key == "source_dialect"
                && entry.value == VisualStateValue::Text("rpy".to_string())
        }));
        assert!(snapshot
            .choices
            .iter()
            .any(|choice| choice == "Ask about Scene Mode."));

        let options = runtime.command_options();
        assert!(options.iter().any(|option| {
            option.origin == "vn_script_import"
                && option.risk == "state_change"
                && option.scope == "scene"
        }));

        runtime.activate_choice();
        assert_eq!(
            runtime.active_dialogue_line().text,
            "Labels become dialogue targets, and menu items become Scene Mode choices."
        );
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
                        tag: "guide".to_string(),
                        sprite: "vn.character.guide.neutral".to_string(),
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
            vec!["background", "guide"]
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
        let runtime = SceneRuntime::new(scene).unwrap();

        let frame = runtime.render_text_frame(80, 24);
        assert!(frame.contains("Stage: 1 layer(s), 1 displayable(s)"));
        assert!(!frame.contains("+---"));
        assert!(!frame.contains("| Codex:"));
        assert!(frame.contains("Codex:"));
        assert!(frame.contains("transparent VN overlay"));
        assert!(frame.contains("Compose: _"));
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
                    tag: "guide".to_string(),
                    sprite: " ".to_string(),
                    placement: VisualStagePlacement::Center,
                    zorder: 0,
                    visible: true,
                }],
            }],
        };
        assert!(matches!(
            empty_sprite.validate(),
            Err(VisualSceneError::EmptyStageDisplayableSprite { tag }) if tag == "guide"
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

        runtime.toggle_debugger();
        let frame = runtime.render_text_frame(200, 80);
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

        runtime.mark_compose_failed("backend offline");
        assert_eq!(runtime.compose_state.phase, VisualComposePhase::Failed);
        assert_eq!(
            runtime.compose_state.history[2].role,
            VisualComposeRole::Error
        );

        let report = runtime.debug_report();
        assert!(report.transition_history.iter().any(
            |event| event.kind == "compose" && event.detail.contains("submit: inspect roadmap")
        ));
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

        assert!(frame.contains(
            "Choices:\r\n[Inspect]\r\n> Inspect selected entity  origin=unknown risk=inspect scope=selected_entity"
        ));
        assert!(frame.contains(
            "[OpenFile]\r\n  Open MIGRATION.md  origin=unknown risk=open_file scope=scene"
        ));
        assert!(frame.contains(
            "[RunCommand]\r\n  Run cargo check -p gameterm-visual  origin=unknown risk=command scope=workspace confirm=true"
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

        runtime.toggle_debugger();
        let frame = runtime.render_text_frame(100, 40);
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
