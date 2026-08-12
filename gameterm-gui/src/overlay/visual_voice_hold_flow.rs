use gameterm_visual::SceneRuntime;
use std::sync::mpsc;

use super::super::visual_stt::{spawn_stt_backend, SceneSttResult};
use super::super::visual_voice_hold::scene_voice_hold_active;
use super::visual_dialogue_scroll::SceneVoiceHoldTransition;
use super::visual_overlay_session::VisualOverlaySession;

pub(super) fn reconcile_scene_voice_hold_state(
    pane_id: mux::pane::PaneId,
    runtime: &mut Option<SceneRuntime>,
    session: &mut VisualOverlaySession,
    stt_tx: &mpsc::Sender<SceneSttResult>,
) -> bool {
    let hold_active = scene_voice_hold_active(pane_id);
    let transition = session.dialogue_scroll.apply_voice_hold_level(hold_active);

    let Some(runtime) = runtime.as_mut() else {
        return !matches!(transition, SceneVoiceHoldTransition::None);
    };

    match transition {
        SceneVoiceHoldTransition::None => false,
        SceneVoiceHoldTransition::Start => {
            if !session.stt_state.is_running() {
                let request_id = session.next_stt_request_id();
                runtime.mark_action_status(session.stt_state.mark_started());
                if session.dialogue_scroll.voice_debug.test_mode {
                    runtime.mark_action_status("Voice test listening");
                }
                session
                    .dialogue_scroll
                    .voice_debug
                    .sync_status(session.stt_state.last_status());
                session.stt_session = Some(spawn_stt_backend(
                    request_id,
                    session.selected_stt_config(),
                    stt_tx.clone(),
                ));
            }
            true
        }
        SceneVoiceHoldTransition::Stop => {
            if session.stt_state.is_running() {
                if let Some(stt_session) = session.stt_session.take() {
                    stt_session.stop();
                }
                runtime.mark_action_status(session.stt_state.mark_processing());
                if session.dialogue_scroll.voice_debug.test_mode {
                    runtime.mark_action_status("Voice test processing");
                }
                session
                    .dialogue_scroll
                    .voice_debug
                    .sync_status(session.stt_state.last_status());
            }
            true
        }
    }
}
