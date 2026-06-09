use std::path::{Path, PathBuf};

mod actions;
mod command_options;
mod debug;
mod debug_menu;
mod dialogue;
mod input;
mod lifecycle;
mod patch;
mod selection;
mod snapshot;
mod status_methods;
mod story_state;
mod text_frame;
mod vn_frame;

pub use self::debug::VisualSceneDebugReport;
use self::dialogue::{dialogue_index, initial_dialogue_history};
use self::input::default_mode_input_action;
pub use self::patch::{
    VisualSceneDialoguePatch, VisualSceneEntityPatch, VisualScenePatch, VisualScenePatchError,
};
pub use self::story_state::{VisualStoryState, VisualStoryStateError};
use crate::compose_state::VisualComposeRuntimeState;
use crate::conditions::{condition_guard_detail, conditions_match};
use crate::vn_layout::VnOverlayDebugOverrides;
use crate::{
    VisualActionRequest, VisualDialogueLine, VisualEntity, VisualInput, VisualInteractiveDebugMenu,
    VisualLayerTransitionReport, VisualMode, VisualModeOutcome, VisualProcessState,
    VisualRenderSnapshot, VisualRuntimeEvent, VisualScene, VisualSceneError, VisualSceneLoadStatus,
    VisualSceneSource, VisualStateOperation, VisualView,
};

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
    vn_layout_debug: Option<VnOverlayDebugOverrides>,
    interactive_debug_menu: VisualInteractiveDebugMenu,
    debug_selected_row: usize,
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
            vn_layout_debug: None,
            interactive_debug_menu: VisualInteractiveDebugMenu::SceneLayout,
            debug_selected_row: 0,
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

    pub fn vn_layout_debug_overrides(&self) -> Option<&VnOverlayDebugOverrides> {
        self.vn_layout_debug.as_ref()
    }

    pub fn interactive_debug_menu(&self) -> VisualInteractiveDebugMenu {
        self.interactive_debug_menu
    }

    pub fn interactive_debug_row(&self) -> usize {
        self.debug_selected_row
    }

    pub fn compose_history_len(&self) -> usize {
        self.compose_state.history.len()
    }

    pub fn set_vn_layout_debug_overrides(&mut self, overrides: VnOverlayDebugOverrides) {
        self.vn_layout_debug = Some(overrides);
        self.bump_generation();
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
        if self.view == VisualView::VnLayoutDebugger {
            return self.handle_vn_layout_debug_input(input);
        }

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

#[cfg(test)]
mod tests;
