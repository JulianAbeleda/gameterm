use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::{
    validate_state_entries, VisualPosition, VisualProcessState, VisualStateEntry,
    VisualStateEntryError,
};

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

impl crate::SceneRuntime {
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
}
