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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualScene {
    pub title: String,
    pub background: String,
    pub width: usize,
    pub height: usize,
    pub entities: Vec<VisualEntity>,
    pub dialogue_speaker: String,
    pub dialogue: String,
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
    pub selected_choice: usize,
    pub selected_choice_label: Option<String>,
    pub selected_choice_kind: Option<String>,
    pub selected_choice_detail: Option<String>,
    pub pending_action_kind: Option<String>,
    pub pending_action_detail: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualScenePatch {
    pub scene_patch_version: u32,
    #[serde(default)]
    pub updates: Vec<VisualSceneEntityPatch>,
    #[serde(default)]
    pub status: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualSceneEntityPatch {
    pub entity_id: String,
    #[serde(default)]
    pub state_flags: Option<Vec<String>>,
    #[serde(default)]
    pub metadata: Option<Vec<(String, String)>>,
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
    #[error("scene patch references unknown entity id `{0}`")]
    UnknownEntityId(String),
    #[error("scene patch metadata for `{entity_id}` contains an empty key")]
    EmptyMetadataKey { entity_id: String },
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
        if self.updates.is_empty() && self.status.is_none() {
            return Err(VisualScenePatchError::EmptyPatch);
        }
        for update in &self.updates {
            if update.entity_id.trim().is_empty() {
                return Err(VisualScenePatchError::EmptyEntityId);
            }
            if let Some(metadata) = &update.metadata {
                if metadata.iter().any(|(key, _)| key.trim().is_empty()) {
                    return Err(VisualScenePatchError::EmptyMetadataKey {
                        entity_id: update.entity_id.clone(),
                    });
                }
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

        for choice in &self.choices {
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
        }

        Ok(())
    }

    pub fn demo() -> Self {
        Self {
            title: "GameTerm Scene Mode".to_string(),
            background: "workspace-map".to_string(),
            width: 18,
            height: 9,
            entities: vec![
                VisualEntity {
                    id: "project-gameterm".to_string(),
                    kind: VisualEntityKind::Project,
                    label: "GameTerm".to_string(),
                    position: VisualPosition { x: 3, y: 2 },
                    sprite: "project_core".to_string(),
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
                    state_flags: vec!["watching".to_string()],
                    metadata: vec![("role".to_string(), "review scene state".to_string())],
                },
            ],
            dialogue_speaker: "GameTerm".to_string(),
            dialogue: "Scene Mode renders project state as symbolic entities while preserving terminal control.".to_string(),
            choices: vec![
                SceneAction {
                    label: "Inspect selected entity".to_string(),
                    kind: SceneActionKind::Inspect,
                },
                SceneAction {
                    label: "Open MIGRATION.md".to_string(),
                    kind: SceneActionKind::OpenFile {
                        path: "MIGRATION.md".to_string(),
                    },
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
    view: VisualView,
    status: String,
    generation: u64,
    pending_action: Option<VisualActionRequest>,
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
        Ok(Self {
            scene,
            scene_source,
            action_base_dir: action_base_dir.into(),
            selected_entity: 0,
            selected_choice: 0,
            view: VisualView::Scene,
            status: "Ready".to_string(),
            generation: 0,
            pending_action: None,
        })
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
        self.scene = scene;
        self.scene_source = scene_source;
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
        if let Some(choice) = self.scene.choices.get(self.selected_choice) {
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
            };
            self.pending_action = pending_action;
            self.bump_generation();
        }
    }

    pub fn mark_action_status(&mut self, status: impl Into<String>) {
        self.status = status.into();
        self.bump_generation();
    }

    pub fn apply_scene_patch(
        &mut self,
        patch: VisualScenePatch,
    ) -> Result<(), VisualScenePatchError> {
        patch.validate()?;
        for update in &patch.updates {
            if !self
                .scene
                .entities
                .iter()
                .any(|entity| entity.id == update.entity_id)
            {
                return Err(VisualScenePatchError::UnknownEntityId(
                    update.entity_id.clone(),
                ));
            }
        }

        let update_count = patch.updates.len();
        for update in patch.updates {
            if let Some(entity) = self
                .scene
                .entities
                .iter_mut()
                .find(|entity| entity.id == update.entity_id)
            {
                if let Some(state_flags) = update.state_flags {
                    entity.state_flags = state_flags;
                }
                if let Some(metadata) = update.metadata {
                    entity.metadata = metadata;
                }
            }
        }

        self.status = patch
            .status
            .unwrap_or_else(|| format!("Applied scene patch: {} entity update(s)", update_count));
        self.bump_generation();
        Ok(())
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

    fn open_file_action_status(&self, path: &str) -> (String, Option<VisualActionRequest>) {
        let raw_path = PathBuf::from(path);
        let resolved = if raw_path.is_absolute() {
            raw_path
        } else {
            self.action_base_dir.join(raw_path)
        };
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
        VisualRenderSnapshot {
            generation: self.generation,
            view: self.view,
            scene_source: self.scene_source.clone(),
            title: self.scene.title.clone(),
            background: self.scene.background.clone(),
            width: self.scene.width,
            height: self.scene.height,
            selected_entity_id,
            selected_choice: self.selected_choice,
            tiles: self.render_tiles(),
            entities: self.render_entities(),
            dialogue_speaker: self.scene.dialogue_speaker.clone(),
            dialogue: self.scene.dialogue.clone(),
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
        let pending_action = self.pending_action.as_ref();
        VisualSceneDebugReport {
            scene_path: self.scene_source.scene_path.clone(),
            load_status: self.scene_source.load_status.as_str().to_string(),
            reload_count: self.scene_source.reload_count,
            last_error: self.scene_source.last_error.clone(),
            action_base_dir: self.action_base_dir.display().to_string(),
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
            selected_choice: self.selected_choice,
            selected_choice_label: selected_choice.map(|choice| choice.label.clone()),
            selected_choice_kind: selected_choice.map(|choice| action_kind_name(&choice.kind)),
            selected_choice_detail: selected_choice.map(|choice| action_kind_detail(&choice.kind)),
            pending_action_kind: pending_action.map(action_request_name),
            pending_action_detail: pending_action.map(action_request_detail),
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
            "{}: {}\r\n\r\n",
            self.scene.dialogue_speaker, self.scene.dialogue
        ));

        out.push_str("Choices:\r\n");
        for (idx, choice) in self.scene.choices.iter().enumerate() {
            let marker = if idx == self.selected_choice {
                ">"
            } else {
                " "
            };
            out.push_str(&format!("{marker} {}\r\n", choice.label));
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
        match (&report.pending_action_kind, &report.pending_action_detail) {
            (Some(kind), Some(detail)) => {
                out.push_str(&format!("  Pending action: {kind} {detail}\r\n"));
            }
            (Some(kind), None) => {
                out.push_str(&format!("  Pending action: {kind}\r\n"));
            }
            _ => out.push_str("  Pending action: none\r\n"),
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
    }
}

fn action_request_name(action: &VisualActionRequest) -> String {
    match action {
        VisualActionRequest::OpenFile { .. } => "OpenFile",
        VisualActionRequest::RunCommand { .. } => "RunCommand",
        VisualActionRequest::Navigate { .. } => "Navigate",
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
    }
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
        match input {
            VisualInput::Close => VisualModeOutcome::Exit,
            VisualInput::Reload => VisualModeOutcome::Continue,
            VisualInput::ToggleDebug => {
                self.toggle_debugger();
                VisualModeOutcome::Continue
            }
            VisualInput::Activate => {
                self.activate_choice();
                VisualModeOutcome::Continue
            }
            VisualInput::Next => {
                self.select_next_entity();
                self.select_next_choice();
                VisualModeOutcome::Continue
            }
            VisualInput::Previous => {
                self.select_prev_entity();
                self.select_prev_choice();
                VisualModeOutcome::Continue
            }
            VisualInput::Other => VisualModeOutcome::Continue,
        }
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
            status: String::new(),
            choices: Vec::new(),
        }
    }

    #[test]
    fn demo_scene_validates() {
        k9::assert_ok!(VisualScene::demo().validate());
    }

    #[test]
    fn scene_fixture_default_loads_runtime_actions() {
        let scene = VisualScene::load_from_path(scene_fixture_path("default.json")).unwrap();
        let runtime = SceneRuntime::new(scene).unwrap();
        let snapshot = runtime.render_snapshot();

        assert_eq!(snapshot.title, "Scene Harness Default");
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
    fn navigate_action_emits_pending_request() {
        let mut scene = VisualScene::demo();
        scene.choices = vec![SceneAction {
            label: "Go to memory".to_string(),
            kind: SceneActionKind::Navigate {
                target: "memory.json".to_string(),
            },
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
        let frame = runtime.render_text_frame(80, 24);
        assert!(frame.contains("Selected: GameTerm"));
    }

    #[test]
    fn debugger_frame_contains_scene_source_status() {
        let source = VisualSceneSource::new("/tmp/default.json", VisualSceneLoadStatus::Loaded, 3);
        let mut runtime = SceneRuntime::new_with_source(VisualScene::demo(), source).unwrap();
        runtime.toggle_debugger();
        runtime.activate_choice();
        let frame = runtime.render_text_frame(80, 24);

        assert!(frame.contains("Scene path: /tmp/default.json"));
        assert!(frame.contains("Load status: loaded"));
        assert!(frame.contains("Reload counter: 3"));
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
        assert_eq!(report.title, "GameTerm Scene Mode");
        assert_eq!(report.background, "workspace-map");
        assert_eq!(report.width, 18);
        assert_eq!(report.height, 9);
        assert_eq!(report.entity_count, 3);
        assert_eq!(report.choice_count, 3);
        assert_eq!(report.selected_entity_id.as_deref(), Some("task-render"));
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
                state_flags: Some(vec!["running".to_string(), "verified".to_string()]),
                metadata: Some(vec![("status".to_string(), "tests passed".to_string())]),
            }],
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
    fn scene_patch_rejects_unknown_entity_without_mutation() {
        let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
        let before = runtime.render_snapshot();
        let patch = VisualScenePatch {
            scene_patch_version: VisualScenePatch::VERSION,
            updates: vec![VisualSceneEntityPatch {
                entity_id: "missing".to_string(),
                state_flags: Some(vec!["bad".to_string()]),
                metadata: None,
            }],
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
