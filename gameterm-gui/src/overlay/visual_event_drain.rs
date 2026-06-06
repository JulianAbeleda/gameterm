use gameterm_visual::SceneRuntime;
use std::path::Path;
use std::sync::mpsc;

use super::super::visual_compose::ComposeBackendResult;
use super::super::visual_stt::{SceneSttResult, SceneSttSession, SceneSttState};
use super::super::visual_tts::{SceneTtsRequest, SceneTtsResult, SceneTtsState, SceneTtsWorker};
use super::visual_command_dispatch::RunCommandResult;
use super::visual_compose_dock::SceneComposeDock;
use super::visual_compose_result::{
    PendingFirstVoiceReveal, apply_compose_backend_result, compose_result_speakable_segments,
    should_delay_first_voice_reveal,
};
use super::visual_dialogue_scroll::SceneDialogueScrollback;
use super::visual_voice_events::{apply_stt_result, apply_tts_result};

pub(super) fn drain_command_results(
    command_rx: &mpsc::Receiver<RunCommandResult>,
    runtime: &mut Option<SceneRuntime>,
) -> bool {
    let mut needs_render = false;
    while let Ok(result) = command_rx.try_recv() {
        if let Some(runtime) = runtime.as_mut() {
            match result {
                RunCommandResult::Spawned {
                    argv,
                    target,
                    pane_id,
                } => {
                    runtime.mark_run_command_spawned(&argv, target, pane_id);
                }
                RunCommandResult::Failed {
                    argv,
                    target,
                    error,
                } => {
                    runtime.mark_run_command_failed(&argv, target, error);
                }
            }
            needs_render = true;
        }
    }
    needs_render
}

pub(super) fn drain_compose_results(
    compose_rx: &mpsc::Receiver<ComposeBackendResult>,
    runtime: &mut Option<SceneRuntime>,
    compose_backend_running: &mut bool,
    dialogue_scroll: &mut SceneDialogueScrollback,
    sync_first_voice_reveal: bool,
    first_voice_reveal_done: &mut bool,
    pending_first_voice_reveal: &mut Option<PendingFirstVoiceReveal>,
    tts_state: &SceneTtsState,
    tts_worker: &SceneTtsWorker,
) -> bool {
    let mut needs_render = false;
    while let Ok(result) = compose_rx.try_recv() {
        *compose_backend_running = false;
        if let Some(runtime) = runtime.as_mut() {
            dialogue_scroll.reset_to_bottom();
            let speakable_segments = compose_result_speakable_segments(&result);
            if should_delay_first_voice_reveal(
                sync_first_voice_reveal,
                *first_voice_reveal_done,
                pending_first_voice_reveal.is_some(),
                tts_state.is_muted(),
                &speakable_segments,
            ) {
                for segment in speakable_segments {
                    tts_worker.speak(SceneTtsRequest { segment });
                }
                *pending_first_voice_reveal = Some(PendingFirstVoiceReveal { result });
                runtime.mark_action_status("Voice preparing first reply");
            } else {
                let speakable_segments = apply_compose_backend_result(runtime, result);
                if !*first_voice_reveal_done && !speakable_segments.is_empty() {
                    *first_voice_reveal_done = true;
                }
                if !tts_state.is_muted() {
                    for segment in speakable_segments {
                        tts_worker.speak(SceneTtsRequest { segment });
                    }
                }
            }
            needs_render = true;
        }
    }
    needs_render
}

pub(super) fn drain_tts_results(
    tts_rx: &mpsc::Receiver<SceneTtsResult>,
    runtime: &mut Option<SceneRuntime>,
    tts_state: &mut SceneTtsState,
    dialogue_scroll: &mut SceneDialogueScrollback,
    pending_first_voice_reveal: &mut Option<PendingFirstVoiceReveal>,
    first_voice_reveal_done: &mut bool,
) -> bool {
    let mut needs_render = false;
    while let Ok(result) = tts_rx.try_recv() {
        if let Some(runtime) = runtime.as_mut() {
            if let Some(pending) = pending_first_voice_reveal.take() {
                apply_compose_backend_result(runtime, pending.result);
                *first_voice_reveal_done = true;
                dialogue_scroll.reset_to_bottom();
            }
            apply_tts_result(runtime, tts_state, result);
            needs_render = true;
        }
    }
    needs_render
}

pub(super) fn drain_stt_results(
    stt_rx: &mpsc::Receiver<SceneSttResult>,
    runtime: &mut Option<SceneRuntime>,
    stt_session: &mut Option<SceneSttSession>,
    dialogue_scroll: &mut SceneDialogueScrollback,
    compose_dock: &mut SceneComposeDock,
    stt_state: &mut SceneSttState,
    compose_backend_running: &mut bool,
    compose_tx: &mpsc::Sender<ComposeBackendResult>,
    scene_path: &Path,
    pane_id: mux::pane::PaneId,
) -> bool {
    let mut needs_render = false;
    while let Ok(result) = stt_rx.try_recv() {
        *stt_session = None;
        dialogue_scroll.voice_hold_active = false;
        if let Some(runtime) = runtime.as_mut() {
            dialogue_scroll.voice_debug.apply_result(&result);
            if dialogue_scroll.voice_debug.test_mode {
                let status = stt_state.apply_result(&result);
                if let Some(transcript) = result.transcript.as_deref() {
                    runtime.mark_action_status(format!("Voice test recognized: {transcript}"));
                } else if let Some(error) = result.error.as_deref() {
                    runtime.mark_action_status(format!("{status}: {error}"));
                } else {
                    runtime.mark_action_status(status);
                }
                dialogue_scroll
                    .voice_debug
                    .sync_status(stt_state.last_status());
            } else {
                apply_stt_result(
                    runtime,
                    compose_dock,
                    stt_state,
                    result,
                    compose_backend_running,
                    compose_tx,
                    scene_path,
                    pane_id,
                );
                dialogue_scroll
                    .voice_debug
                    .sync_status(stt_state.last_status());
            }
            dialogue_scroll.reset_to_bottom();
            needs_render = true;
        }
    }
    needs_render
}
