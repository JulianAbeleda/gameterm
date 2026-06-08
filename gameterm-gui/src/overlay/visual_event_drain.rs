use gameterm_visual::SceneRuntime;
use std::path::Path;
use std::sync::mpsc;

use super::super::visual_compose::ComposeBackendResult;
use super::super::visual_stt::{SceneMicTestResult, SceneSttResult};
use super::super::visual_tts::SceneTtsResult;
use super::visual_command_dispatch::RunCommandResult;
use super::visual_compose_result::apply_compose_backend_result;
use super::visual_dialogue_scroll::SceneDialogueScrollback;
use super::visual_overlay_session::VisualOverlaySession;
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
    session: &mut VisualOverlaySession,
) -> bool {
    let mut needs_render = false;
    while let Ok(result) = compose_rx.try_recv() {
        session.compose_backend_running = false;
        if let Some(runtime) = runtime.as_mut() {
            session.dialogue_scroll.reset_to_bottom();
            let voice_block_sync = !session.tts_state.is_muted();
            let speakable_segments =
                apply_compose_backend_result(runtime, result, voice_block_sync);
            if !session.tts_state.is_muted() {
                session.enqueue_tts_segments(speakable_segments);
            } else {
                session.sync_tts_debug();
            }
            needs_render = true;
        }
    }
    needs_render
}

pub(super) fn drain_tts_results(
    tts_rx: &mpsc::Receiver<SceneTtsResult>,
    runtime: &mut Option<SceneRuntime>,
    session: &mut VisualOverlaySession,
) -> bool {
    let mut needs_render = false;
    while let Ok(result) = tts_rx.try_recv() {
        if let Some(runtime) = runtime.as_mut() {
            apply_tts_result(runtime, &mut session.tts_state, result);
            session.sync_tts_debug();
            needs_render = true;
        }
    }
    needs_render
}

pub(super) fn drain_stt_results(
    stt_rx: &mpsc::Receiver<SceneSttResult>,
    runtime: &mut Option<SceneRuntime>,
    session: &mut VisualOverlaySession,
    compose_tx: &mpsc::Sender<ComposeBackendResult>,
    scene_path: &Path,
    pane_id: mux::pane::PaneId,
) -> bool {
    let mut needs_render = false;
    while let Ok(result) = stt_rx.try_recv() {
        session.stt_session = None;
        session.dialogue_scroll.mark_voice_hold_result_finished();
        if let Some(runtime) = runtime.as_mut() {
            session.dialogue_scroll.voice_debug.apply_result(&result);
            if session.dialogue_scroll.voice_debug.test_mode {
                let status = session.stt_state.apply_result(&result);
                if let Some(transcript) = result.transcript.as_deref() {
                    runtime.mark_action_status(format!("Voice test recognized: {transcript}"));
                } else if let Some(error) = result.error.as_deref() {
                    runtime.mark_action_status(format!("{status}: {error}"));
                } else {
                    runtime.mark_action_status(status);
                }
                session
                    .dialogue_scroll
                    .voice_debug
                    .sync_status(session.stt_state.last_status());
            } else {
                let will_auto_submit = result.succeeded()
                    && result.auto_submit
                    && result
                        .transcript
                        .as_deref()
                        .is_some_and(|transcript| !transcript.trim().is_empty())
                    && !session.compose_backend_running;
                if will_auto_submit {
                    session.interrupt_tts_queue();
                }
                apply_stt_result(
                    runtime,
                    &mut session.compose_dock,
                    &mut session.stt_state,
                    result,
                    &mut session.compose_backend_running,
                    compose_tx,
                    scene_path,
                    pane_id,
                );
                session
                    .dialogue_scroll
                    .voice_debug
                    .sync_status(session.stt_state.last_status());
            }
            session.dialogue_scroll.reset_to_bottom();
            needs_render = true;
        }
    }
    needs_render
}

pub(super) fn drain_mic_test_results(
    mic_test_rx: &mpsc::Receiver<SceneMicTestResult>,
    runtime: &mut Option<SceneRuntime>,
    dialogue_scroll: &mut SceneDialogueScrollback,
    mic_test_running: &mut bool,
) -> bool {
    let mut needs_render = false;
    while let Ok(result) = mic_test_rx.try_recv() {
        *mic_test_running = false;
        if let Some(runtime) = runtime.as_mut() {
            let status = if let Some(error) = result.error.as_deref() {
                format!("{}: {error}", result.status)
            } else {
                result.status.clone()
            };
            dialogue_scroll.voice_debug.apply_mic_test_result(result);
            runtime.mark_action_status(status);
            needs_render = true;
        }
    }
    needs_render
}
