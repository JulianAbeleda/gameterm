use serde::{Deserialize, Serialize};

use crate::actions::{
    action_kind_detail, action_kind_name, action_request_detail, action_request_name,
};
use crate::conditions::{condition_guard_detail, conditions_match};
use crate::{
    dialogue_index, entity_mode, SceneRuntime, VisualDialogueLine, VisualInputBinding,
    VisualLayerState, VisualLayerTransitionReport, VisualModeLifecycle, VisualProcessState,
    VisualRpgState, VisualRuntimeEvent, VisualStateEntry,
};

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

impl SceneRuntime {
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
}
