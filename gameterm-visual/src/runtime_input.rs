use crate::conditions::{condition_guard_detail, conditions_match};
use crate::{actions, SceneRuntime, VisualInput, VisualModeOutcome, VisualView};

impl SceneRuntime {
    pub(super) fn run_mode_input_action(&mut self, action: &str) -> VisualModeOutcome {
        match action {
            "close" => VisualModeOutcome::Exit,
            "reload" | "ignore" => VisualModeOutcome::Continue,
            "toggle_debug" => {
                self.toggle_debugger();
                VisualModeOutcome::Continue
            }
            "show_command_selection" => {
                self.show_command_selection();
                VisualModeOutcome::Continue
            }
            "hide_command_selection" => {
                self.hide_command_selection();
                VisualModeOutcome::Continue
            }
            "toggle_command_selection" => {
                self.toggle_command_selection();
                VisualModeOutcome::Continue
            }
            "activate_choice" => {
                self.activate_choice();
                VisualModeOutcome::Continue
            }
            "select_next" => {
                if self.view != VisualView::CommandSelection {
                    self.select_next_entity();
                }
                self.select_next_choice();
                VisualModeOutcome::Continue
            }
            "select_previous" => {
                if self.view != VisualView::CommandSelection {
                    self.select_prev_entity();
                }
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

    pub(super) fn handle_layer_input(&mut self, input: VisualInput) -> Option<VisualModeOutcome> {
        let input_key = input.binding_key();
        for layer_index in 0..self.scene.layers.len() {
            let transition = self.scene.layers[layer_index]
                .transitions
                .iter()
                .find(|transition| transition.input.trim() == input_key)
                .cloned();
            if let Some(transition) = transition {
                let layer_id = self.scene.layers[layer_index].layer_id.clone();
                let selected_entity = self.selected_entity().cloned();
                let process_state = self.last_process_state.clone();
                self.last_input_layer = Some(layer_id.clone());
                let transition_result = actions::apply_layer_transition_at(
                    &mut self.scene.layers,
                    layer_index,
                    input_key,
                    &self.scene.variables,
                    &self.scene.rpg,
                    selected_entity.as_ref(),
                    process_state.as_ref(),
                )
                .expect("transition was found before applying it");
                match transition_result {
                    Err(report) => {
                        let layer_id = report.layer_id.clone();
                        let from_state = report.from_state.clone();
                        let target_state = report.target_state.clone();
                        self.last_layer_transition = Some(report);
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
                    Ok(report) => {
                        let layer_id = report.layer_id.clone();
                        let from_state = report.from_state.clone();
                        let target_state = report.target_state.clone();
                        self.last_layer_transition = Some(report);
                        self.status = format!(
                            "Layer {layer_id} transitioned: {from_state} -> {target_state}"
                        );
                        self.record_runtime_event(
                            "transition",
                            format!("{layer_id} {from_state} -> {target_state}"),
                        );
                        self.bump_generation();
                        return Some(VisualModeOutcome::Continue);
                    }
                }
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
}

pub(crate) fn default_mode_input_action(input: VisualInput) -> &'static str {
    match input {
        VisualInput::Close => "close",
        VisualInput::Reload => "reload",
        VisualInput::ToggleDebug => "toggle_debug",
        VisualInput::Activate => "activate_choice",
        VisualInput::Next => "select_next",
        VisualInput::Previous => "select_previous",
        VisualInput::Left => "select_previous",
        VisualInput::Right => "select_next",
        VisualInput::Char(_) | VisualInput::Backspace | VisualInput::Other => "ignore",
    }
}
