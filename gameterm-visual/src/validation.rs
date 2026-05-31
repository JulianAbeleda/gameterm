use std::collections::HashSet;

use crate::{
    VisualDialogueLine, VisualEntity, VisualLayerState, VisualRpgState, VisualSceneError,
    VisualStateEntry, VisualStateOperation,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum VisualStateEntryError {
    EmptyKey,
    DuplicateKey(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VisualDialogueLineError {
    EmptySpeaker { index: usize },
    EmptyText { index: usize },
}

pub(crate) fn validate_state_entries(
    entries: &[VisualStateEntry],
) -> Result<(), VisualStateEntryError> {
    let mut keys = HashSet::new();
    for entry in entries {
        if entry.key.trim().is_empty() {
            return Err(VisualStateEntryError::EmptyKey);
        }
        if !keys.insert(entry.key.as_str()) {
            return Err(VisualStateEntryError::DuplicateKey(entry.key.clone()));
        }
    }
    Ok(())
}

pub(crate) fn validate_dialogue_lines(
    lines: &[VisualDialogueLine],
) -> Result<(), VisualDialogueLineError> {
    for (index, line) in lines.iter().enumerate() {
        if line.speaker.trim().is_empty() {
            return Err(VisualDialogueLineError::EmptySpeaker { index });
        }
        if line.text.trim().is_empty() {
            return Err(VisualDialogueLineError::EmptyText { index });
        }
    }
    Ok(())
}

pub(crate) fn validate_layers(layers: &[VisualLayerState]) -> Result<(), VisualSceneError> {
    let mut layer_ids = HashSet::new();
    for layer in layers {
        if layer.layer_id.trim().is_empty() {
            return Err(VisualSceneError::EmptyLayerId);
        }
        if !layer_ids.insert(layer.layer_id.as_str()) {
            return Err(VisualSceneError::DuplicateLayerId(layer.layer_id.clone()));
        }
        if layer.state.trim().is_empty() {
            return Err(VisualSceneError::EmptyLayerState {
                layer_id: layer.layer_id.clone(),
            });
        }
        for binding in &layer.input_map {
            if binding.input.trim().is_empty() {
                return Err(VisualSceneError::EmptyLayerInputBindingInput {
                    layer_id: layer.layer_id.clone(),
                });
            }
            if binding.action.trim().is_empty() {
                return Err(VisualSceneError::EmptyLayerInputBindingAction {
                    layer_id: layer.layer_id.clone(),
                });
            }
            if !is_supported_mode_input(binding.input.trim()) {
                return Err(VisualSceneError::UnknownLayerInputBindingInput {
                    layer_id: layer.layer_id.clone(),
                    input: binding.input.clone(),
                });
            }
            if !is_supported_mode_input_action(binding.action.trim()) {
                return Err(VisualSceneError::UnknownLayerInputBindingAction {
                    layer_id: layer.layer_id.clone(),
                    action: binding.action.clone(),
                });
            }
            if binding
                .conditions
                .iter()
                .any(|condition| condition.variable.trim().is_empty())
            {
                return Err(VisualSceneError::EmptyLayerInputBindingConditionVariable {
                    layer_id: layer.layer_id.clone(),
                });
            }
        }
        for transition in &layer.transitions {
            if transition.input.trim().is_empty() {
                return Err(VisualSceneError::EmptyLayerTransitionInput {
                    layer_id: layer.layer_id.clone(),
                });
            }
            if transition.target_state.trim().is_empty() {
                return Err(VisualSceneError::EmptyLayerTransitionTarget {
                    layer_id: layer.layer_id.clone(),
                });
            }
            if !is_supported_mode_input(transition.input.trim()) {
                return Err(VisualSceneError::UnknownLayerTransitionInput {
                    layer_id: layer.layer_id.clone(),
                    input: transition.input.clone(),
                });
            }
            if transition
                .conditions
                .iter()
                .any(|condition| condition.variable.trim().is_empty())
            {
                return Err(VisualSceneError::EmptyLayerTransitionConditionVariable {
                    layer_id: layer.layer_id.clone(),
                });
            }
        }
    }
    Ok(())
}

pub(crate) fn validate_rpg_state(state: &VisualRpgState) -> Result<(), VisualSceneError> {
    let mut item_ids = HashSet::new();
    for item in &state.inventory {
        if item.item_id.trim().is_empty() {
            return Err(VisualSceneError::EmptyInventoryItemId);
        }
        if item.label.trim().is_empty() {
            return Err(VisualSceneError::EmptyInventoryLabel {
                item_id: item.item_id.clone(),
            });
        }
        if !item_ids.insert(item.item_id.as_str()) {
            return Err(VisualSceneError::DuplicateInventoryItemId(
                item.item_id.clone(),
            ));
        }
    }

    let mut stats = HashSet::new();
    for stat in &state.stats {
        if stat.key.trim().is_empty() {
            return Err(VisualSceneError::EmptyStatKey);
        }
        if matches!(stat.owner_id.as_ref(), Some(owner_id) if owner_id.trim().is_empty()) {
            return Err(VisualSceneError::EmptyStatOwnerId);
        }
        let key = format!(
            "{}:{}",
            stat.owner_id.as_deref().unwrap_or("scene"),
            stat.key
        );
        if !stats.insert(key.clone()) {
            return Err(VisualSceneError::DuplicateStatKey(key));
        }
    }

    let mut quest_ids = HashSet::new();
    for quest in &state.quests {
        if quest.quest_id.trim().is_empty() {
            return Err(VisualSceneError::EmptyQuestId);
        }
        if quest.label.trim().is_empty() {
            return Err(VisualSceneError::EmptyQuestLabel {
                quest_id: quest.quest_id.clone(),
            });
        }
        if !quest_ids.insert(quest.quest_id.as_str()) {
            return Err(VisualSceneError::DuplicateQuestId(quest.quest_id.clone()));
        }
    }

    let mut relationships = HashSet::new();
    for relationship in &state.relationships {
        if relationship.source_id.trim().is_empty() {
            return Err(VisualSceneError::EmptyRelationshipSourceId);
        }
        if relationship.target_id.trim().is_empty() {
            return Err(VisualSceneError::EmptyRelationshipTargetId);
        }
        if relationship.kind.trim().is_empty() {
            return Err(VisualSceneError::EmptyRelationshipKind);
        }
        let key = format!(
            "{}:{}:{}",
            relationship.source_id, relationship.target_id, relationship.kind
        );
        if !relationships.insert(key.clone()) {
            return Err(VisualSceneError::DuplicateRelationship(key));
        }
    }

    Ok(())
}

pub(crate) fn validate_state_operations(
    label: &str,
    operations: &[VisualStateOperation],
    entities: &[VisualEntity],
    dialogue_lines: &[VisualDialogueLine],
    layers: &[VisualLayerState],
    rpg: &VisualRpgState,
) -> Result<(), VisualSceneError> {
    if operations.is_empty() {
        return Err(VisualSceneError::EmptyResolveOperations {
            label: label.to_string(),
        });
    }
    for operation in operations {
        match operation {
            VisualStateOperation::SetVariable { key, .. }
            | VisualStateOperation::IncrementVariable { key, .. }
            | VisualStateOperation::ClearVariable { key } => {
                if key.trim().is_empty() {
                    return Err(VisualSceneError::EmptyResolveOperationKey {
                        label: label.to_string(),
                    });
                }
            }
            VisualStateOperation::SetLayerState { layer_id, state } => {
                if layer_id.trim().is_empty() || state.trim().is_empty() {
                    return Err(VisualSceneError::EmptyResolveOperationKey {
                        label: label.to_string(),
                    });
                }
                if !layers.iter().any(|layer| layer.layer_id == *layer_id) {
                    return Err(VisualSceneError::UnknownLayer {
                        label: label.to_string(),
                        layer_id: layer_id.clone(),
                    });
                }
            }
            VisualStateOperation::SelectEntity { entity_id }
            | VisualStateOperation::SetEntityVisibility { entity_id, .. } => {
                if entity_id.trim().is_empty() {
                    return Err(VisualSceneError::EmptyResolveOperationKey {
                        label: label.to_string(),
                    });
                }
                if !entities.iter().any(|entity| entity.id == *entity_id) {
                    return Err(VisualSceneError::UnknownEntity {
                        label: label.to_string(),
                        entity_id: entity_id.clone(),
                    });
                }
            }
            VisualStateOperation::SetEntityFlags { entity_id, flags } => {
                if entity_id.trim().is_empty() || flags.iter().any(|flag| flag.trim().is_empty()) {
                    return Err(VisualSceneError::EmptyResolveOperationKey {
                        label: label.to_string(),
                    });
                }
                if !entities.iter().any(|entity| entity.id == *entity_id) {
                    return Err(VisualSceneError::UnknownEntity {
                        label: label.to_string(),
                        entity_id: entity_id.clone(),
                    });
                }
            }
            VisualStateOperation::SetEntityMetadata {
                entity_id,
                metadata,
            } => {
                if entity_id.trim().is_empty()
                    || metadata.iter().any(|(key, _)| key.trim().is_empty())
                {
                    return Err(VisualSceneError::EmptyResolveOperationKey {
                        label: label.to_string(),
                    });
                }
                if !entities.iter().any(|entity| entity.id == *entity_id) {
                    return Err(VisualSceneError::UnknownEntity {
                        label: label.to_string(),
                        entity_id: entity_id.clone(),
                    });
                }
            }
            VisualStateOperation::AdvanceDialogueAndSetLayer {
                target,
                layer_id,
                state,
            } => {
                if layer_id.trim().is_empty() || state.trim().is_empty() {
                    return Err(VisualSceneError::EmptyResolveOperationKey {
                        label: label.to_string(),
                    });
                }
                if *target >= dialogue_lines.len() {
                    return Err(VisualSceneError::DialogueTargetOutOfBounds {
                        label: label.to_string(),
                        target: *target,
                    });
                }
                if !layers.iter().any(|layer| layer.layer_id == *layer_id) {
                    return Err(VisualSceneError::UnknownLayer {
                        label: label.to_string(),
                        layer_id: layer_id.clone(),
                    });
                }
            }
            VisualStateOperation::TriggerLayerTransition { layer_id, input } => {
                if layer_id.trim().is_empty() || input.trim().is_empty() {
                    return Err(VisualSceneError::EmptyResolveOperationKey {
                        label: label.to_string(),
                    });
                }
                let Some(layer) = layers.iter().find(|layer| layer.layer_id == *layer_id) else {
                    return Err(VisualSceneError::UnknownLayer {
                        label: label.to_string(),
                        layer_id: layer_id.clone(),
                    });
                };
                if !layer
                    .transitions
                    .iter()
                    .any(|transition| transition.input == *input)
                {
                    return Err(VisualSceneError::UnknownLayerTransition {
                        label: label.to_string(),
                        layer_id: layer_id.clone(),
                        input: input.clone(),
                    });
                }
            }
            VisualStateOperation::AddInventory { item } => {
                validate_rpg_state(&VisualRpgState {
                    inventory: vec![item.clone()],
                    stats: Vec::new(),
                    quests: Vec::new(),
                    relationships: Vec::new(),
                })?;
            }
            VisualStateOperation::RemoveInventory { item_id, .. } => {
                if item_id.trim().is_empty() {
                    return Err(VisualSceneError::EmptyResolveOperationKey {
                        label: label.to_string(),
                    });
                }
                if !rpg.inventory.iter().any(|item| item.item_id == *item_id) {
                    return Err(VisualSceneError::UnknownInventoryItem {
                        label: label.to_string(),
                        item_id: item_id.clone(),
                    });
                }
            }
            VisualStateOperation::SetStat { key, .. }
            | VisualStateOperation::AdjustStat { key, .. } => {
                if key.trim().is_empty() {
                    return Err(VisualSceneError::EmptyResolveOperationKey {
                        label: label.to_string(),
                    });
                }
            }
            VisualStateOperation::AdvanceQuest { quest_id, .. }
            | VisualStateOperation::CompleteQuest { quest_id }
            | VisualStateOperation::AppendQuestJournal { quest_id, .. } => {
                if quest_id.trim().is_empty() {
                    return Err(VisualSceneError::EmptyResolveOperationKey {
                        label: label.to_string(),
                    });
                }
                if !rpg.quests.iter().any(|quest| quest.quest_id == *quest_id) {
                    return Err(VisualSceneError::UnknownQuest {
                        label: label.to_string(),
                        quest_id: quest_id.clone(),
                    });
                }
            }
            VisualStateOperation::AdjustRelationship {
                source_id,
                target_id,
                kind,
                ..
            } => {
                if source_id.trim().is_empty()
                    || target_id.trim().is_empty()
                    || kind.trim().is_empty()
                {
                    return Err(VisualSceneError::EmptyResolveOperationKey {
                        label: label.to_string(),
                    });
                }
                if !rpg.relationships.iter().any(|relationship| {
                    relationship.source_id == *source_id
                        && relationship.target_id == *target_id
                        && relationship.kind == *kind
                }) {
                    return Err(VisualSceneError::UnknownRelationship {
                        label: label.to_string(),
                        relationship: relationship_key(source_id, target_id, kind),
                    });
                }
            }
        }
    }
    Ok(())
}

pub(crate) fn relationship_key(source_id: &str, target_id: &str, kind: &str) -> String {
    format!("{source_id}:{target_id}:{kind}")
}

pub(crate) fn is_supported_action_policy_origin(origin: &str) -> bool {
    matches!(
        origin,
        "authored" | "workspace_discovery" | "agent" | "runtime" | "fixture" | "unknown"
    )
}

pub(crate) fn is_supported_action_policy_risk(risk: &str) -> bool {
    matches!(
        risk,
        "inspect"
            | "open_file"
            | "navigate"
            | "state_change"
            | "story_io"
            | "command"
            | "agent_proposal"
            | "unknown"
    )
}

pub(crate) fn is_supported_action_policy_scope(scope: &str) -> bool {
    matches!(
        scope,
        "scene"
            | "selected_entity"
            | "workspace"
            | "pane"
            | "process"
            | "agent"
            | "external"
            | "unknown"
    )
}

pub(crate) fn is_supported_mode_input(input: &str) -> bool {
    matches!(
        input,
        "close" | "reload" | "toggle_debug" | "activate" | "next" | "previous" | "other"
    )
}

pub(crate) fn is_supported_mode_input_action(action: &str) -> bool {
    matches!(
        action,
        "close"
            | "reload"
            | "toggle_debug"
            | "activate_choice"
            | "select_next"
            | "select_previous"
            | "run_update_hooks"
            | "run_exit_hooks"
            | "export_story_state"
            | "import_story_state"
            | "ignore"
    )
}
