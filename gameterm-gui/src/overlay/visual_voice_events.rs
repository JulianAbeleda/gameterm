use gameterm_visual::SceneRuntime;
use std::path::Path;
use std::sync::mpsc;

use super::super::visual_compose::{
    compose_running_status, spawn_compose_backend, ComposeBackendCancel, ComposeBackendRequest,
    ComposeBackendResult,
};
use super::super::visual_stt::{SceneSttResult, SceneSttState};
use super::super::visual_tts::{SceneTtsEvent, SceneTtsResult, SceneTtsState};
use super::super::visual_voice_trace::{trace_voice_event, SceneVoiceTraceEvent};
use super::visual_compose_dock::SceneComposeDock;

pub(super) fn apply_tts_result(
    runtime: &mut SceneRuntime,
    tts_state: &mut SceneTtsState,
    result: SceneTtsResult,
) {
    let accepted = tts_state.accepts_result(&result);
    let mut trace = SceneVoiceTraceEvent::new(if accepted {
        "tts_result_applied"
    } else {
        "stale_tts_event_ignored"
    })
    .with_text(result.segment.text.clone());
    trace.turn_id = Some(result.segment.turn_id);
    trace.block_index = Some(result.segment.block_index);
    trace.generation = Some(result.generation);
    trace.speaker = result.segment.speaker.clone();
    trace.status = Some(result.status.clone());
    trace.error = result.error.clone();
    trace.output_path = result
        .output_path
        .as_ref()
        .map(|path| path.display().to_string());
    trace.timing = Some(serde_json::json!({
        "translation_ms": result.timing.translation_ms,
        "query_ms": result.timing.query_ms,
        "synthesis_ms": result.timing.synthesis_ms,
        "player_ms": result.timing.player_ms,
        "total_ms": result.timing.total_ms,
    }));
    trace_voice_event(trace);
    if accepted {
        match result.event {
            SceneTtsEvent::Started => {
                runtime.mark_compose_block_speaking(
                    result.segment.turn_id,
                    result.segment.block_index,
                );
            }
            SceneTtsEvent::Finished => {
                runtime.mark_compose_block_done(result.segment.turn_id, result.segment.block_index);
            }
        }
    }
    let status = tts_state.apply_result(&result);
    if !accepted {
        runtime.mark_action_status(status);
    } else if result.succeeded() {
        runtime.mark_action_status(status);
    } else if let Some(error) = result.error {
        runtime.mark_action_status(format!("{status}: {error}"));
    } else {
        runtime.mark_action_status(status);
    }
}

pub(super) fn apply_stt_result(
    runtime: &mut SceneRuntime,
    compose_dock: &mut SceneComposeDock,
    stt_state: &mut SceneSttState,
    result: SceneSttResult,
    compose_backend_running: &mut bool,
    compose_cancel: &mut Option<ComposeBackendCancel>,
    compose_model: Option<&str>,
    compose_tx: &mpsc::Sender<ComposeBackendResult>,
    scene_path: &Path,
    pane_id: mux::pane::PaneId,
) {
    let status = stt_state.apply_result(&result);
    let succeeded = result.succeeded();
    let mut trace = SceneVoiceTraceEvent::new("stt_result_applied");
    trace.status = Some(status.clone());
    trace.error = result.error.clone();
    trace.text = result.transcript.clone();
    trace.text_sha256 = result
        .transcript
        .as_deref()
        .map(super::super::visual_voice_trace::text_sha256);
    trace_voice_event(trace);
    if succeeded {
        let Some(transcript) = result.transcript else {
            return;
        };
        compose_dock.insert_transcript(&transcript);
        runtime.mark_action_status(status);
        if result.auto_submit {
            let prompt = compose_dock.buffer.trim().to_string();
            if prompt.is_empty() {
                return;
            }
            if *compose_backend_running {
                runtime.mark_action_status("Voice transcript ready: compose busy");
                return;
            }
            let backend_prompt = runtime.compose_backend_prompt(&prompt);
            compose_dock.mark_submitted(&prompt);
            runtime.mark_compose_running(compose_running_status(&prompt), &prompt);
            *compose_backend_running = true;
            *compose_cancel = Some(spawn_compose_backend(
                ComposeBackendRequest {
                    prompt,
                    backend_prompt,
                    scene_path: Some(scene_path.display().to_string()),
                    pane_id: Some(pane_id),
                    model_override: compose_model.map(str::to_string),
                },
                compose_tx.clone(),
            ));
            compose_dock.begin_backend_wait();
        }
    } else if let Some(error) = result.error {
        runtime.mark_action_status(format!("{status}: {error}"));
    } else {
        runtime.mark_action_status(status);
    }
}
