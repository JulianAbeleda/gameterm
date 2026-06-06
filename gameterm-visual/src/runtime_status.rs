use crate::{VisualLayerState, VisualProcessState, VisualScene, VisualStateEntry};

pub(crate) fn clip_summary_value(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let clipped = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() {
        format!("{clipped}...")
    } else {
        clipped
    }
}

pub(crate) fn format_state_summary(entries: &[VisualStateEntry], max_entries: usize) -> String {
    let mut parts = entries
        .iter()
        .take(max_entries)
        .map(|entry| {
            format!(
                "{}={}",
                entry.key,
                clip_summary_value(&entry.value.as_debug_string(), 32)
            )
        })
        .collect::<Vec<_>>();
    if entries.len() > max_entries {
        parts.push(format!("+{} more", entries.len() - max_entries));
    }
    parts.join(", ")
}

pub(crate) fn format_metadata_summary(metadata: &[(String, String)], max_entries: usize) -> String {
    let mut parts = metadata
        .iter()
        .take(max_entries)
        .map(|(key, value)| format!("{key}={}", clip_summary_value(value, 36)))
        .collect::<Vec<_>>();
    if metadata.len() > max_entries {
        parts.push(format!("+{} more", metadata.len() - max_entries));
    }
    parts.join(", ")
}

pub(crate) fn relationship_entity_label(scene: &VisualScene, entity_id: &str) -> String {
    scene
        .entities
        .iter()
        .find(|entity| entity.id == entity_id)
        .map(|entity| format!("{} ({})", entity.label, entity.id))
        .unwrap_or_else(|| entity_id.to_string())
}

#[cfg(test)]
pub(crate) fn format_relationship_metadata(metadata: &[(String, String)]) -> String {
    let metadata = format_metadata_summary(metadata, 2);
    if metadata.is_empty() {
        String::new()
    } else {
        format!(" [{metadata}]")
    }
}

pub(crate) fn format_relationship_summary(
    scene: &VisualScene,
    entity_id: &str,
    max_entries: usize,
) -> Option<String> {
    let outgoing = scene
        .rpg
        .relationships
        .iter()
        .filter(|relationship| relationship.source_id == entity_id)
        .collect::<Vec<_>>();
    let incoming = scene
        .rpg
        .relationships
        .iter()
        .filter(|relationship| relationship.target_id == entity_id)
        .collect::<Vec<_>>();
    if outgoing.is_empty() && incoming.is_empty() {
        return None;
    }

    let mut parts = vec![format!("out={} in={}", outgoing.len(), incoming.len())];
    for relationship in outgoing.iter().take(max_entries) {
        parts.push(format!(
            "-> {} {}",
            relationship_entity_label(scene, &relationship.target_id),
            relationship.kind
        ));
    }
    let remaining = max_entries.saturating_sub(outgoing.len().min(max_entries));
    for relationship in incoming.iter().take(remaining) {
        parts.push(format!(
            "<- {} {}",
            relationship_entity_label(scene, &relationship.source_id),
            relationship.kind
        ));
    }
    let shown = parts.len().saturating_sub(1);
    let total = outgoing.len() + incoming.len();
    if total > shown {
        parts.push(format!("+{} more", total - shown));
    }
    Some(parts.join(", "))
}

pub(crate) fn format_layer_summary(layers: &[VisualLayerState]) -> String {
    layers
        .iter()
        .take(4)
        .map(|layer| format!("{}={}", layer.layer_id, layer.state))
        .chain((layers.len() > 4).then(|| format!("+{} more", layers.len() - 4)))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn format_process_summary(process_state: &VisualProcessState) -> String {
    let mut parts = vec![process_state.phase.as_str().to_string()];
    if let Some(entity_id) = &process_state.entity_id {
        parts.push(format!("entity={entity_id}"));
    }
    if let Some(command) = &process_state.command {
        parts.push(format!("cmd={}", clip_summary_value(command, 36)));
    }
    if let Some(exit_code) = process_state.exit_code {
        parts.push(format!("exit={exit_code}"));
    }
    if let Some(message) = &process_state.message {
        parts.push(clip_summary_value(message, 48));
    }
    parts.join(", ")
}

pub(crate) fn clip_text(text: &str, max_chars: usize) -> String {
    text.chars().take(max_chars).collect()
}

pub(crate) fn wrap_text(text: &str, max_chars: usize) -> Vec<String> {
    let max_chars = max_chars.max(1);
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let word_len = word.chars().count();
        let current_len = current.chars().count();
        if current_len == 0 {
            current.push_str(word);
        } else if current_len + 1 + word_len <= max_chars {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current);
            current = word.to_string();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}
