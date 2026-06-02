use crate::actions::{self, SceneActionOutcome};
use crate::conditions::{condition_guard_detail, conditions_match};
use crate::SceneRuntime;

impl SceneRuntime {
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
            let outcome = actions::scene_action_outcome(self, &choice);
            self.apply_action_outcome(outcome);
        }
    }

    pub(crate) fn apply_action_outcome(&mut self, outcome: SceneActionOutcome) {
        self.status = outcome.status;
        self.pending_action = outcome.pending_action;
        for event in outcome.events {
            self.record_runtime_event(event.kind, event.detail);
        }
        self.bump_generation();
    }
}
