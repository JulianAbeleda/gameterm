use crate::{
    VisualCondition, VisualEntity, VisualProcessState, VisualRpgState, VisualStateEntry,
    VisualStateValue,
};

pub(crate) fn conditions_match(
    conditions: &[VisualCondition],
    variables: &[VisualStateEntry],
    rpg: &VisualRpgState,
    selected_entity: Option<&VisualEntity>,
    process_state: Option<&VisualProcessState>,
) -> bool {
    conditions.iter().all(|condition| {
        condition_value(condition, variables, rpg, selected_entity, process_state)
            .map(|value| value == condition.equals)
            .unwrap_or(false)
    })
}

fn condition_value(
    condition: &VisualCondition,
    variables: &[VisualStateEntry],
    rpg: &VisualRpgState,
    selected_entity: Option<&VisualEntity>,
    process_state: Option<&VisualProcessState>,
) -> Option<VisualStateValue> {
    match condition.source.as_deref().unwrap_or("variable") {
        "variable" => variables
            .iter()
            .find(|entry| entry.key == condition.variable)
            .map(|entry| entry.value.clone()),
        "inventory_count" => rpg
            .inventory
            .iter()
            .find(|item| item.item_id == condition.variable)
            .map(|item| VisualStateValue::Number(i64::from(item.count))),
        "inventory_has" => rpg
            .inventory
            .iter()
            .find(|item| item.item_id == condition.variable)
            .map(|item| VisualStateValue::Bool(item.count > 0))
            .or(Some(VisualStateValue::Bool(false))),
        "quest_stage" => rpg
            .quests
            .iter()
            .find(|quest| quest.quest_id == condition.variable)
            .map(|quest| VisualStateValue::Number(quest.stage)),
        "quest_completed" => rpg
            .quests
            .iter()
            .find(|quest| quest.quest_id == condition.variable)
            .map(|quest| VisualStateValue::Bool(quest.completed)),
        "stat" => {
            let (owner_id, key) = condition
                .variable
                .split_once(':')
                .map(|(owner_id, key)| (Some(owner_id), key))
                .unwrap_or((None, condition.variable.as_str()));
            rpg.stats
                .iter()
                .find(|stat| stat.owner_id.as_deref() == owner_id && stat.key == key)
                .map(|stat| stat.value.clone())
        }
        "agent_phase" => variables
            .iter()
            .find(|entry| entry.key == "agent_phase")
            .map(|entry| entry.value.clone()),
        "process_phase" => process_state
            .map(|state| VisualStateValue::Text(state.phase.as_str().to_string()))
            .or_else(|| {
                variables
                    .iter()
                    .find(|entry| entry.key == "agent_process_phase")
                    .map(|entry| entry.value.clone())
            }),
        "selected_entity_flag" => selected_entity.map(|entity| {
            VisualStateValue::Bool(
                entity
                    .state_flags
                    .iter()
                    .any(|flag| flag == &condition.variable),
            )
        }),
        "selected_entity_metadata" => selected_entity.and_then(|entity| {
            entity
                .metadata
                .iter()
                .find(|(key, _)| key == &condition.variable)
                .map(|(_, value)| VisualStateValue::Text(value.clone()))
        }),
        _ => None,
    }
}

pub(crate) fn condition_guard_detail(conditions: &[VisualCondition]) -> Option<String> {
    if conditions.is_empty() {
        return None;
    }

    Some(format!(
        "requires {}",
        conditions
            .iter()
            .map(|condition| format!(
                "{}{}={}",
                condition
                    .source
                    .as_ref()
                    .map(|source| format!("{source}:"))
                    .unwrap_or_default(),
                condition.variable,
                condition.equals.as_debug_string()
            ))
            .collect::<Vec<_>>()
            .join(", ")
    ))
}
