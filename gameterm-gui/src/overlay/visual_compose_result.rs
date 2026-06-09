use crate::overlay::visual_compose::{ComposeBackendLabel, ComposeBackendResult};
use crate::overlay::visual_speech_blocks::{
    extract_compose_reply_blocks, ComposeReplyBlocks, SpeakableSegment, SpeakableSource,
};
use gameterm_visual::{SceneRuntime, VisualSceneDialoguePatch, VisualScenePatch};
use serde::Deserialize;

pub(super) const COMPOSE_OUTPUT_LIMIT: usize = 1200;

#[derive(Debug, Clone, PartialEq, Eq)]
enum StructuredComposeOutcome {
    NoReply,
    WithReply {
        speaker: String,
        dialogue_text: String,
    },
}

#[cfg(test)]
pub(super) fn compose_result_speakable_segments(
    result: &ComposeBackendResult,
) -> Vec<SpeakableSegment> {
    let Some((speaker, dialogue_text)) = compose_result_reply_preview(result) else {
        return Vec::new();
    };
    extract_compose_reply_blocks(
        Some(&speaker),
        &dialogue_text,
        SpeakableSource::ComposeReply,
    )
    .segments
}

#[cfg(test)]
fn compose_result_reply_preview(result: &ComposeBackendResult) -> Option<(String, String)> {
    if !result.succeeded() {
        return None;
    }

    if let Some(payload) = parse_structured_compose_payload(&result.stdout) {
        if let Some(value) = payload.patch.clone() {
            if let Ok(patch) = parse_structured_compose_patch(value) {
                if let Some(dialogue) = patch.dialogue {
                    return Some((dialogue.speaker, dialogue.text));
                }
            }
        }

        if payload.dialogue_text_is_present() {
            let speaker = payload
                .speaker
                .as_deref()
                .filter(|speaker| !speaker.trim().is_empty())
                .unwrap_or("Codex")
                .to_string();
            return Some((speaker, payload.text_or_default()));
        }

        return None;
    }

    let reply = sanitize_compose_output(&result.stdout);
    let reply = if reply.is_empty() {
        "The compose backend returned no output.".to_string()
    } else {
        reply
    };
    Some(("Codex".to_string(), reply))
}

pub(super) fn apply_compose_backend_result(
    runtime: &mut SceneRuntime,
    result: ComposeBackendResult,
    voice_block_sync: bool,
) -> Vec<SpeakableSegment> {
    if result.succeeded() {
        match apply_structured_compose_backend_result(runtime, &result) {
            Some(StructuredComposeOutcome::WithReply {
                speaker,
                dialogue_text,
            }) => {
                return stamp_runtime_blocks(runtime, &speaker, &dialogue_text, voice_block_sync);
            }
            Some(StructuredComposeOutcome::NoReply) => {
                runtime.mark_compose_succeeded("Scene", "");
                return Vec::new();
            }
            None => {}
        }

        let reply = sanitize_compose_output(&result.stdout);
        let reply = if reply.is_empty() {
            "The compose backend returned no output.".to_string()
        } else {
            reply
        };
        let patch = VisualScenePatch {
            scene_patch_version: VisualScenePatch::VERSION,
            updates: vec![],
            variables: vec![],
            selected_entity_id: None,
            process_state: None,
            dialogue: Some(VisualSceneDialoguePatch {
                speaker: "Codex".to_string(),
                text: reply.clone(),
                append_history: true,
            }),
            status: Some(result.label.succeeded_status().to_string()),
        };
        if let Err(err) = runtime.apply_scene_patch(patch) {
            runtime.mark_action_status(format!("Compose reply failed: {err}"));
        } else {
            return stamp_runtime_blocks(runtime, "Codex", &reply, voice_block_sync);
        }
        return Vec::new();
    }

    apply_compose_backend_failure_result(runtime, &result);
    Vec::new()
}

fn stamp_runtime_blocks(
    runtime: &mut SceneRuntime,
    speaker: &str,
    dialogue_text: &str,
    voice_block_sync: bool,
) -> Vec<SpeakableSegment> {
    // Display blocks come from the full reply text; the speech filter only
    // decides what is spoken. A reply that is entirely technical (a path, a
    // code block, JSON) must still land in the visible transcript even though
    // it produces no speakable segments.
    let ComposeReplyBlocks {
        display_blocks,
        mut segments,
        segment_block_ordinals,
    } = extract_compose_reply_blocks(Some(speaker), dialogue_text, SpeakableSource::ComposeReply);
    if display_blocks.is_empty() {
        runtime.mark_compose_succeeded(speaker, dialogue_text);
        return Vec::new();
    }
    let ids = runtime.mark_compose_succeeded_blocks(speaker, &display_blocks, !voice_block_sync);
    for (segment, block_ordinal) in segments.iter_mut().zip(segment_block_ordinals) {
        if let Some((turn_id, block_index)) = ids.get(block_ordinal).copied() {
            segment.turn_id = turn_id;
            segment.block_index = block_index;
        }
    }
    segments
}

pub(super) fn fake_codex_compose_result(prompt: String) -> ComposeBackendResult {
    let text = if prompt.trim().is_empty() {
        "Fake Codex is ready.".to_string()
    } else {
        format!("Fake Codex received: {}", prompt.trim())
    };
    let stdout = serde_json::json!({
        "speaker": "Fake Codex",
        "text": text,
        "status": "Fake Codex succeeded"
    })
    .to_string();
    ComposeBackendResult {
        prompt,
        stdout,
        stderr: String::new(),
        exit_code: Some(0),
        label: ComposeBackendLabel::Codex,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct StructuredComposePayload {
    #[serde(default)]
    speaker: Option<String>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    append_history: Option<bool>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    patch: Option<serde_json::Value>,
}

impl StructuredComposePayload {
    fn meaningful(&self) -> bool {
        self.speaker.is_some()
            || self
                .text
                .as_ref()
                .is_some_and(|text| !text.trim().is_empty())
            || self.append_history.is_some()
            || self.status.is_some()
            || self.patch.is_some()
    }

    fn text_or_default(&self) -> String {
        sanitize_compose_output(self.text.as_deref().unwrap_or(""))
    }

    fn dialogue_text_is_present(&self) -> bool {
        self.text
            .as_ref()
            .is_some_and(|text| !text.trim().is_empty())
    }
}

fn parse_structured_payload(raw: &str) -> Option<StructuredComposePayload> {
    serde_json::from_str::<StructuredComposePayload>(raw)
        .ok()
        .filter(|payload| payload.meaningful())
}

fn parse_structured_compose_payload(raw: &str) -> Option<StructuredComposePayload> {
    parse_structured_payload(raw.trim()).or_else(|| {
        raw.lines()
            .rev()
            .find_map(|line| parse_structured_payload(line.trim()))
    })
}

fn parse_structured_compose_patch(value: serde_json::Value) -> Result<VisualScenePatch, String> {
    let patch: VisualScenePatch = serde_json::from_value(value)
        .map_err(|err| format!("invalid compose patch json: {err}"))?;
    patch
        .validate()
        .map_err(|err| format!("compose patch invalid: {err}"))?;
    Ok(patch)
}

fn apply_structured_compose_backend_result(
    runtime: &mut SceneRuntime,
    result: &ComposeBackendResult,
) -> Option<StructuredComposeOutcome> {
    let mut payload = match parse_structured_compose_payload(&result.stdout) {
        Some(payload) => payload,
        None => return None,
    };

    let mut patch = match payload.patch.take() {
        Some(value) => match parse_structured_compose_patch(value) {
            Ok(patch) => patch,
            Err(err) => {
                runtime.mark_action_status(format!("Compose patch parse failed: {err}"));
                return None;
            }
        },
        None => VisualScenePatch {
            scene_patch_version: VisualScenePatch::VERSION,
            updates: vec![],
            variables: vec![],
            selected_entity_id: None,
            process_state: None,
            dialogue: None,
            status: None,
        },
    };

    let mut applied_dialogue = patch.dialogue.clone();

    if patch.dialogue.is_none() && payload.dialogue_text_is_present() {
        let speaker = payload
            .speaker
            .take()
            .filter(|speaker| !speaker.trim().is_empty())
            .unwrap_or_else(|| "Codex".to_string());

        let dialogue = VisualSceneDialoguePatch {
            speaker,
            text: payload.text_or_default(),
            append_history: payload.append_history.unwrap_or(true),
        };
        applied_dialogue = Some(dialogue.clone());
        patch.dialogue = Some(dialogue);
    }

    if patch.status.is_none() {
        patch.status = Some(
            payload
                .status
                .unwrap_or_else(|| result.label.succeeded_status().to_string()),
        );
    }

    if let Err(err) = runtime.apply_scene_patch(patch) {
        runtime.mark_action_status(format!("Compose patch failed: {err}"));
        return None;
    }

    if applied_dialogue.is_some() {
        return applied_dialogue
            .take()
            .map(|dialogue| StructuredComposeOutcome::WithReply {
                speaker: dialogue.speaker,
                dialogue_text: dialogue.text,
            });
    }

    Some(StructuredComposeOutcome::NoReply)
}

fn apply_compose_backend_failure_result(runtime: &mut SceneRuntime, result: &ComposeBackendResult) {
    let diagnostic = sanitize_compose_output(&result.stderr);
    let diagnostic = result.failure_dialogue(&diagnostic);
    let marked_diagnostic = diagnostic.clone();
    let patch = VisualScenePatch {
        scene_patch_version: VisualScenePatch::VERSION,
        updates: vec![],
        variables: vec![],
        selected_entity_id: None,
        process_state: None,
        dialogue: Some(VisualSceneDialoguePatch {
            speaker: "Scene".to_string(),
            text: diagnostic,
            append_history: true,
        }),
        status: Some(result.failure_status()),
    };
    if let Err(err) = runtime.apply_scene_patch(patch) {
        runtime.mark_action_status(format!("Compose reply failed: {err}"));
    } else {
        runtime.mark_compose_failed(&marked_diagnostic);
    }
}

pub(super) fn sanitize_compose_output(output: &str) -> String {
    let mut sanitized = String::new();
    let mut blank_pending = false;
    for ch in output.chars() {
        if sanitized.chars().count() >= COMPOSE_OUTPUT_LIMIT {
            break;
        }
        match ch {
            '\n' | '\r' => {
                if !blank_pending {
                    sanitized.push('\n');
                    blank_pending = true;
                }
            }
            '\t' => {
                sanitized.push(' ');
                blank_pending = false;
            }
            ch if ch.is_control() => {}
            ch => {
                sanitized.push(ch);
                blank_pending = false;
            }
        }
    }
    sanitized.trim().to_string()
}
