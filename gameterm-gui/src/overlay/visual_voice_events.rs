use gameterm_visual::SceneRuntime;
use std::path::Path;
use std::sync::mpsc;

use super::super::visual_compose::{
    compose_running_status, spawn_compose_backend, ComposeBackendRequest, ComposeBackendResult,
};
use super::super::visual_stt::{SceneSttResult, SceneSttState};
use super::super::visual_tts::{SceneTtsEvent, SceneTtsResult, SceneTtsState};
use super::visual_compose_dock::SceneComposeDock;

pub(super) fn apply_tts_result(
    runtime: &mut SceneRuntime,
    tts_state: &mut SceneTtsState,
    result: SceneTtsResult,
) {
    match result.event {
        SceneTtsEvent::Started => {
            runtime.mark_compose_block_speaking(result.segment.turn_id, result.segment.block_index);
        }
        SceneTtsEvent::Finished => {
            runtime.mark_compose_block_done(result.segment.turn_id, result.segment.block_index);
        }
    }
    let status = tts_state.apply_result(&result);
    if result.succeeded() {
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
    compose_tx: &mpsc::Sender<ComposeBackendResult>,
    scene_path: &Path,
    pane_id: mux::pane::PaneId,
) {
    let status = stt_state.apply_result(&result);
    let succeeded = result.succeeded();
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
            compose_dock.mark_submitted(&prompt);
            runtime.mark_compose_running(compose_running_status(&prompt), &prompt);
            *compose_backend_running = true;
            spawn_compose_backend(
                ComposeBackendRequest {
                    prompt,
                    scene_path: Some(scene_path.display().to_string()),
                    pane_id: Some(pane_id),
                },
                compose_tx.clone(),
            );
        }
    } else if let Some(error) = result.error {
        runtime.mark_action_status(format!("{status}: {error}"));
    } else {
        runtime.mark_action_status(status);
    }
}
