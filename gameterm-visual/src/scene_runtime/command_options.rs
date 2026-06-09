use super::actions::{action_kind_name, derived_action_policy};
use super::SceneRuntime;
use crate::conditions::{condition_guard_detail, conditions_match};
use crate::{VisualCommandFilter, VisualCommandOption};

impl SceneRuntime {
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
