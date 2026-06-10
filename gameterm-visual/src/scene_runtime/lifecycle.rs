use super::{initial_dialogue_history, SceneRuntime};
use crate::vn_layout::VnOverlayDebugOverrides;
use crate::{
    VisualActionRequest, VisualInteractiveDebugMenu, VisualScene, VisualSceneError,
    VisualSceneSource, VisualView,
};

impl SceneRuntime {
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

    pub fn toggle_debugger(&mut self) {
        match self.view {
            VisualView::VnLayoutDebugger => {
                self.view = VisualView::Scene;
                self.bump_generation();
            }
            _ => self.open_layout_debugger(),
        }
    }

    /// Enter the layout debugger from any view. Shared by the Tab toggle and
    /// the main-menu Settings entry.
    pub(super) fn open_layout_debugger(&mut self) {
        if self.vn_layout_debug.is_none() {
            self.vn_layout_debug = Some(VnOverlayDebugOverrides::default());
        }
        self.interactive_debug_menu = VisualInteractiveDebugMenu::SceneLayout;
        self.debug_selected_row = self.debug_selected_row.min(self.debug_menu_row_count());
        self.view = VisualView::VnLayoutDebugger;
        self.bump_generation();
    }

    pub(super) fn show_interactive_debug_menu(&mut self, menu: VisualInteractiveDebugMenu) {
        if self.interactive_debug_menu != menu {
            self.interactive_debug_menu = menu;
            self.debug_selected_row = 0;
            self.bump_generation();
        }
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
        // Shell/menu views never reach command selection; only toggle from the
        // scene-side views.
        self.view = match self.view {
            VisualView::CommandSelection => VisualView::Scene,
            _ => VisualView::CommandSelection,
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
