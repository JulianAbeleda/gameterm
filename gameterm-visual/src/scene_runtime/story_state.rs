use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::{actions, dialogue_index, initial_dialogue_history};
use crate::{
    is_empty_rpg_state, validate_dialogue_lines, validate_rpg_state, validate_state_entries,
    VisualDialogueLine, VisualDialogueLineError, VisualRpgState, VisualStateEntry,
    VisualStateEntryError,
};

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

impl super::SceneRuntime {
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

    pub(crate) fn default_story_state_path(&self) -> PathBuf {
        let scene_path = PathBuf::from(&self.scene_source.scene_path);
        if scene_path.file_name().is_some() {
            scene_path.with_extension("story.json")
        } else {
            self.action_base_dir.join("gameterm-scene.story.json")
        }
    }

    pub(crate) fn request_story_state_export(&mut self, path: PathBuf) {
        self.apply_action_outcome(actions::story_state_export_outcome(path));
    }

    pub(crate) fn request_story_state_import(&mut self, path: PathBuf) {
        self.apply_action_outcome(actions::story_state_import_outcome(path));
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
}
