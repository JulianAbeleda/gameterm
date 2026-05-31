use std::path::PathBuf;

use crate::conditions::conditions_match;
use crate::{
    relationship_key, validate_layers, validate_rpg_state, validate_state_entries,
    validate_state_operations, SceneAction, SceneActionKind, SceneRuntime, VisualActionRequest,
    VisualRuntimeEvent, VisualSceneError, VisualStat, VisualStateEntry, VisualStateEntryError,
    VisualStateOperation, VisualStateValue,
};

pub(crate) fn action_kind_name(kind: &SceneActionKind) -> String {
    match kind {
        SceneActionKind::Inspect => "Inspect",
        SceneActionKind::OpenFile { .. } => "OpenFile",
        SceneActionKind::RunCommand { .. } => "RunCommand",
        SceneActionKind::Navigate { .. } => "Navigate",
        SceneActionKind::ExportStoryState { .. } => "ExportStoryState",
        SceneActionKind::ImportStoryState { .. } => "ImportStoryState",
        SceneActionKind::AdvanceDialogue { .. } => "AdvanceDialogue",
        SceneActionKind::Resolve { .. } => "Resolve",
    }
    .to_string()
}

pub(crate) fn action_kind_detail(kind: &SceneActionKind) -> String {
    match kind {
        SceneActionKind::Inspect => "selected entity".to_string(),
        SceneActionKind::OpenFile { path } => format!("path={path}"),
        SceneActionKind::RunCommand { argv, cwd, target } => {
            let mut detail = format!("argv={}", argv.join(" "));
            if let Some(cwd) = cwd {
                detail.push_str(&format!(" cwd={cwd}"));
            }
            detail.push_str(&format!(" target={}", target.as_str()));
            detail
        }
        SceneActionKind::Navigate { target } => format!("target={target}"),
        SceneActionKind::ExportStoryState { path } => format!("path={path}"),
        SceneActionKind::ImportStoryState { path } => format!("path={path}"),
        SceneActionKind::AdvanceDialogue { target } => format!("target={target}"),
        SceneActionKind::Resolve { operations } => format!("operations={}", operations.len()),
    }
}

pub(crate) fn action_request_name(action: &VisualActionRequest) -> String {
    match action {
        VisualActionRequest::OpenFile { .. } => "OpenFile",
        VisualActionRequest::RunCommand { .. } => "RunCommand",
        VisualActionRequest::Navigate { .. } => "Navigate",
        VisualActionRequest::ExportStoryState { .. } => "ExportStoryState",
        VisualActionRequest::ImportStoryState { .. } => "ImportStoryState",
    }
    .to_string()
}

pub(crate) fn action_request_detail(action: &VisualActionRequest) -> String {
    match action {
        VisualActionRequest::OpenFile { path } => format!("path={}", path.display()),
        VisualActionRequest::RunCommand { argv, cwd, target } => {
            let mut detail = format!("argv={}", argv.join(" "));
            if let Some(cwd) = cwd {
                detail.push_str(&format!(" cwd={}", cwd.display()));
            }
            detail.push_str(&format!(" target={}", target.as_str()));
            detail
        }
        VisualActionRequest::Navigate { target } => format!("target={target}"),
        VisualActionRequest::ExportStoryState { path } => format!("path={}", path.display()),
        VisualActionRequest::ImportStoryState { path } => format!("path={}", path.display()),
    }
}

pub(crate) fn visual_state_operation_summary(operation: &VisualStateOperation) -> String {
    match operation {
        VisualStateOperation::SetVariable { key, value } => {
            format!("set {key}={}", value.as_debug_string())
        }
        VisualStateOperation::SetLayerState { layer_id, state } => {
            format!("layer {layer_id}->{state}")
        }
        VisualStateOperation::SelectEntity { entity_id } => format!("select {entity_id}"),
        VisualStateOperation::SetEntityFlags { entity_id, flags } => {
            format!("flags {entity_id}=[{}]", flags.join(","))
        }
        VisualStateOperation::SetEntityMetadata {
            entity_id,
            metadata,
        } => {
            format!("metadata {entity_id}={} pair(s)", metadata.len())
        }
        VisualStateOperation::SetEntityVisibility { entity_id, visible } => {
            format!("visible {entity_id}={visible}")
        }
        VisualStateOperation::AdvanceDialogueAndSetLayer {
            target,
            layer_id,
            state,
        } => {
            format!("dialogue {target} + layer {layer_id}->{state}")
        }
        VisualStateOperation::TriggerLayerTransition { layer_id, input } => {
            format!("transition {layer_id}:{input}")
        }
        VisualStateOperation::IncrementVariable { key, amount } => {
            format!("increment {key} by {amount}")
        }
        VisualStateOperation::ClearVariable { key } => format!("clear {key}"),
        VisualStateOperation::AddInventory { item } => {
            format!("add inventory {} x{}", item.item_id, item.count)
        }
        VisualStateOperation::RemoveInventory { item_id, count } => {
            format!("remove inventory {item_id} x{count}")
        }
        VisualStateOperation::SetStat {
            owner_id,
            key,
            value,
        } => {
            let prefix = owner_id
                .as_ref()
                .map(|owner_id| format!("{owner_id}:"))
                .unwrap_or_default();
            format!("set stat {prefix}{key}={}", value.as_debug_string())
        }
        VisualStateOperation::AdjustStat {
            owner_id,
            key,
            amount,
        } => {
            let prefix = owner_id
                .as_ref()
                .map(|owner_id| format!("{owner_id}:"))
                .unwrap_or_default();
            format!("adjust stat {prefix}{key} by {amount}")
        }
        VisualStateOperation::AdvanceQuest { quest_id, stage } => {
            format!("quest {quest_id} stage {stage}")
        }
        VisualStateOperation::CompleteQuest { quest_id } => {
            format!("quest {quest_id} complete")
        }
        VisualStateOperation::AppendQuestJournal { quest_id, .. } => {
            format!("quest {quest_id} journal")
        }
        VisualStateOperation::AdjustRelationship {
            source_id,
            target_id,
            kind,
            amount,
        } => {
            format!("relationship {source_id}:{target_id}:{kind} by {amount}")
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SceneActionOutcome {
    pub(crate) status: String,
    pub(crate) pending_action: Option<VisualActionRequest>,
    pub(crate) events: Vec<VisualRuntimeEvent>,
}

impl SceneActionOutcome {
    pub(crate) fn status(status: impl Into<String>) -> Self {
        Self {
            status: status.into(),
            pending_action: None,
            events: Vec::new(),
        }
    }

    pub(crate) fn pending(status: impl Into<String>, pending_action: VisualActionRequest) -> Self {
        Self {
            status: status.into(),
            pending_action: Some(pending_action),
            events: Vec::new(),
        }
    }

    fn with_event(mut self, kind: impl Into<String>, detail: impl Into<String>) -> Self {
        self.events.push(VisualRuntimeEvent {
            kind: kind.into(),
            detail: detail.into(),
        });
        self
    }
}

pub(crate) fn story_state_export_outcome(path: PathBuf) -> SceneActionOutcome {
    SceneActionOutcome::pending(
        format!("ExportStoryState ready: {}", path.display()),
        VisualActionRequest::ExportStoryState { path },
    )
}

pub(crate) fn story_state_import_outcome(path: PathBuf) -> SceneActionOutcome {
    SceneActionOutcome::pending(
        format!("ImportStoryState ready: {}", path.display()),
        VisualActionRequest::ImportStoryState { path },
    )
}

pub(crate) fn scene_action_outcome(
    runtime: &mut SceneRuntime,
    choice: &SceneAction,
) -> SceneActionOutcome {
    match &choice.kind {
        SceneActionKind::Inspect => runtime
            .selected_entity()
            .map(|entity| {
                SceneActionOutcome::status(format!("Inspecting {} ({})", entity.label, entity.id))
            })
            .unwrap_or_else(|| SceneActionOutcome::status("No entity selected")),
        SceneActionKind::OpenFile { path } => {
            let (status, pending_action) = runtime.open_file_action_status(path);
            SceneActionOutcome {
                status,
                pending_action,
                events: Vec::new(),
            }
        }
        SceneActionKind::RunCommand { argv, cwd, target } => SceneActionOutcome::pending(
            format!("RunCommand ready ({}): {}", target.as_str(), argv.join(" ")),
            VisualActionRequest::RunCommand {
                argv: argv.clone(),
                cwd: cwd.as_ref().map(std::path::PathBuf::from),
                target: *target,
            },
        ),
        SceneActionKind::Navigate { target } => SceneActionOutcome::pending(
            format!("Navigate ready: {target}"),
            VisualActionRequest::Navigate {
                target: target.clone(),
            },
        ),
        SceneActionKind::ExportStoryState { path } => {
            let path = runtime.resolve_action_path(path);
            story_state_export_outcome(path)
        }
        SceneActionKind::ImportStoryState { path } => {
            let path = runtime.resolve_action_path(path);
            story_state_import_outcome(path)
        }
        SceneActionKind::AdvanceDialogue { target } => {
            runtime.dialogue_index = *target;
            if let Some(line) = runtime.scene.dialogue_lines.get(*target).cloned() {
                runtime.dialogue_history.push(line.clone());
                SceneActionOutcome::status(format!("Dialogue advanced: {}", line.speaker))
                    .with_event("dialogue", format!("advanced to line {target}"))
            } else {
                SceneActionOutcome::status(format!("Dialogue target missing: {target}"))
            }
        }
        SceneActionKind::Resolve { operations } => {
            match runtime.apply_state_operations(&choice.label, operations) {
                Ok(count) => {
                    let summary = operations
                        .iter()
                        .map(visual_state_operation_summary)
                        .collect::<Vec<_>>()
                        .join(", ");
                    SceneActionOutcome::status(format!(
                        "Resolved {count} operation(s): {}",
                        choice.label
                    ))
                    .with_event(
                        "state",
                        format!("{} resolved {count} operation(s): {summary}", choice.label),
                    )
                }
                Err(err) => SceneActionOutcome::status(format!("Resolve failed: {err}"))
                    .with_event("state", format!("{} failed: {err}", choice.label)),
            }
        }
    }
}

pub(crate) fn apply_state_operations(
    runtime: &mut SceneRuntime,
    label: &str,
    operations: &[VisualStateOperation],
) -> Result<usize, VisualSceneError> {
    validate_state_operations(
        label,
        operations,
        &runtime.scene.entities,
        &runtime.scene.dialogue_lines,
        &runtime.scene.layers,
        &runtime.scene.rpg,
    )?;
    let mut variables = runtime.scene.variables.clone();
    let mut layers = runtime.scene.layers.clone();
    let mut rpg = runtime.scene.rpg.clone();
    let mut entities = runtime.scene.entities.clone();
    let mut selected_entity = runtime.selected_entity;
    let mut dialogue_index = runtime.dialogue_index;
    let mut dialogue_history = runtime.dialogue_history.clone();

    for operation in operations {
        match operation {
            VisualStateOperation::SetVariable { key, value } => {
                set_variable(&mut variables, key, value.clone());
            }
            VisualStateOperation::SetLayerState { layer_id, state } => {
                if let Some(layer) = layers.iter_mut().find(|layer| layer.layer_id == *layer_id) {
                    layer.state = state.trim().to_string();
                }
            }
            VisualStateOperation::SelectEntity { entity_id } => {
                let Some(index) = entities.iter().position(|entity| entity.id == *entity_id) else {
                    return Err(VisualSceneError::UnknownEntity {
                        label: label.to_string(),
                        entity_id: entity_id.clone(),
                    });
                };
                selected_entity = index;
            }
            VisualStateOperation::SetEntityFlags { entity_id, flags } => {
                let Some(entity) = entities.iter_mut().find(|entity| entity.id == *entity_id)
                else {
                    return Err(VisualSceneError::UnknownEntity {
                        label: label.to_string(),
                        entity_id: entity_id.clone(),
                    });
                };
                entity.state_flags = flags.clone();
            }
            VisualStateOperation::SetEntityMetadata {
                entity_id,
                metadata,
            } => {
                let Some(entity) = entities.iter_mut().find(|entity| entity.id == *entity_id)
                else {
                    return Err(VisualSceneError::UnknownEntity {
                        label: label.to_string(),
                        entity_id: entity_id.clone(),
                    });
                };
                entity.metadata = metadata.clone();
            }
            VisualStateOperation::SetEntityVisibility { entity_id, visible } => {
                let Some(entity) = entities.iter_mut().find(|entity| entity.id == *entity_id)
                else {
                    return Err(VisualSceneError::UnknownEntity {
                        label: label.to_string(),
                        entity_id: entity_id.clone(),
                    });
                };
                entity.visible = *visible;
            }
            VisualStateOperation::AdvanceDialogueAndSetLayer {
                target,
                layer_id,
                state,
            } => {
                dialogue_index = *target;
                if let Some(line) = runtime.scene.dialogue_lines.get(*target).cloned() {
                    dialogue_history.push(line);
                }
                if let Some(layer) = layers.iter_mut().find(|layer| layer.layer_id == *layer_id) {
                    layer.state = state.trim().to_string();
                }
            }
            VisualStateOperation::TriggerLayerTransition { layer_id, input } => {
                let Some(layer_index) = layers.iter().position(|layer| layer.layer_id == *layer_id)
                else {
                    return Err(VisualSceneError::UnknownLayer {
                        label: label.to_string(),
                        layer_id: layer_id.clone(),
                    });
                };
                let Some(transition) = layers[layer_index]
                    .transitions
                    .iter()
                    .find(|transition| transition.input == *input)
                    .cloned()
                else {
                    return Err(VisualSceneError::UnknownLayerTransition {
                        label: label.to_string(),
                        layer_id: layer_id.clone(),
                        input: input.clone(),
                    });
                };
                if !conditions_match(&transition.conditions, &variables, &rpg, None, None) {
                    return Err(VisualSceneError::LayerTransitionGuardFailed {
                        label: label.to_string(),
                        layer_id: layer_id.clone(),
                        input: input.clone(),
                    });
                }
                layers[layer_index].state = transition.target_state.trim().to_string();
            }
            VisualStateOperation::IncrementVariable { key, amount } => {
                increment_variable(&mut variables, key, *amount);
            }
            VisualStateOperation::ClearVariable { key } => {
                variables.retain(|entry| entry.key != *key);
            }
            VisualStateOperation::AddInventory { item } => {
                match rpg
                    .inventory
                    .iter_mut()
                    .find(|existing| existing.item_id == item.item_id)
                {
                    Some(existing) => existing.count = existing.count.saturating_add(item.count),
                    None => rpg.inventory.push(item.clone()),
                }
            }
            VisualStateOperation::RemoveInventory { item_id, count } => {
                let Some(item) = rpg
                    .inventory
                    .iter_mut()
                    .find(|existing| existing.item_id == *item_id)
                else {
                    return Err(VisualSceneError::UnknownInventoryItem {
                        label: label.to_string(),
                        item_id: item_id.clone(),
                    });
                };
                item.count = item.count.saturating_sub(*count);
                rpg.inventory.retain(|item| item.count > 0);
            }
            VisualStateOperation::SetStat {
                owner_id,
                key,
                value,
            } => match rpg
                .stats
                .iter_mut()
                .find(|stat| stat.owner_id == *owner_id && stat.key == *key)
            {
                Some(stat) => stat.value = value.clone(),
                None => rpg.stats.push(VisualStat {
                    owner_id: owner_id.clone(),
                    key: key.clone(),
                    value: value.clone(),
                }),
            },
            VisualStateOperation::AdjustStat {
                owner_id,
                key,
                amount,
            } => {
                let Some(stat) = rpg
                    .stats
                    .iter_mut()
                    .find(|stat| stat.owner_id == *owner_id && stat.key == *key)
                else {
                    return Err(VisualSceneError::UnknownStat {
                        label: label.to_string(),
                        key: key.clone(),
                    });
                };
                match &mut stat.value {
                    VisualStateValue::Number(value) => *value += amount,
                    _ => {
                        return Err(VisualSceneError::NonNumericStat {
                            label: label.to_string(),
                            key: key.clone(),
                        });
                    }
                }
            }
            VisualStateOperation::AdvanceQuest { quest_id, stage } => {
                let Some(quest) = rpg
                    .quests
                    .iter_mut()
                    .find(|quest| quest.quest_id == *quest_id)
                else {
                    return Err(VisualSceneError::UnknownQuest {
                        label: label.to_string(),
                        quest_id: quest_id.clone(),
                    });
                };
                quest.stage = *stage;
            }
            VisualStateOperation::CompleteQuest { quest_id } => {
                let Some(quest) = rpg
                    .quests
                    .iter_mut()
                    .find(|quest| quest.quest_id == *quest_id)
                else {
                    return Err(VisualSceneError::UnknownQuest {
                        label: label.to_string(),
                        quest_id: quest_id.clone(),
                    });
                };
                quest.completed = true;
            }
            VisualStateOperation::AppendQuestJournal { quest_id, text } => {
                let Some(quest) = rpg
                    .quests
                    .iter_mut()
                    .find(|quest| quest.quest_id == *quest_id)
                else {
                    return Err(VisualSceneError::UnknownQuest {
                        label: label.to_string(),
                        quest_id: quest_id.clone(),
                    });
                };
                if !quest.journal.is_empty() {
                    quest.journal.push('\n');
                }
                quest.journal.push_str(text);
            }
            VisualStateOperation::AdjustRelationship {
                source_id,
                target_id,
                kind,
                amount,
            } => {
                let Some(relationship) = rpg.relationships.iter_mut().find(|relationship| {
                    relationship.source_id == *source_id
                        && relationship.target_id == *target_id
                        && relationship.kind == *kind
                }) else {
                    return Err(VisualSceneError::UnknownRelationship {
                        label: label.to_string(),
                        relationship: relationship_key(source_id, target_id, kind),
                    });
                };
                relationship.value += amount;
            }
        }
    }

    validate_state_entries(&variables).map_err(|err| match err {
        VisualStateEntryError::EmptyKey => VisualSceneError::EmptyVariableKey,
        VisualStateEntryError::DuplicateKey(key) => VisualSceneError::DuplicateVariableKey(key),
    })?;
    validate_layers(&layers)?;
    validate_rpg_state(&rpg)?;

    runtime.scene.variables = variables;
    runtime.scene.layers = layers;
    runtime.scene.rpg = rpg;
    runtime.scene.entities = entities;
    runtime.selected_entity = selected_entity;
    runtime.dialogue_index = dialogue_index;
    runtime.dialogue_history = dialogue_history;
    Ok(operations.len())
}

fn set_variable(variables: &mut Vec<VisualStateEntry>, key: &str, value: VisualStateValue) {
    match variables.iter_mut().find(|entry| entry.key == key) {
        Some(entry) => entry.value = value,
        None => variables.push(VisualStateEntry {
            key: key.to_string(),
            value,
        }),
    }
}

fn increment_variable(variables: &mut Vec<VisualStateEntry>, key: &str, amount: i64) {
    match variables.iter_mut().find(|entry| entry.key == key) {
        Some(entry) => match &mut entry.value {
            VisualStateValue::Number(value) => *value += amount,
            _ => entry.value = VisualStateValue::Number(amount),
        },
        None => variables.push(VisualStateEntry {
            key: key.to_string(),
            value: VisualStateValue::Number(amount),
        }),
    }
}
