use gameterm_visual::{SceneRuntime, VisualInput, VisualInteractiveDebugMenu, VisualView};
use std::sync::mpsc;

use super::super::visual_stt::{SceneMicTestResult, spawn_mic_test};
use super::visual_overlay_session::VisualOverlaySession;
use super::visual_voice_debug::VoiceDebugMenuEffect;

pub(super) fn handle_scene_debug_session_input(
    runtime: &mut SceneRuntime,
    session: &mut VisualOverlaySession,
    input: VisualInput,
    mic_test_tx: &mpsc::Sender<SceneMicTestResult>,
) -> VoiceDebugMenuEffect {
    if runtime.view() != VisualView::VnLayoutDebugger {
        return VoiceDebugMenuEffect::IGNORED;
    }
    let is_action = matches!(
        input,
        VisualInput::Activate | VisualInput::Left | VisualInput::Right
    );
    if !is_action || runtime.interactive_debug_row() == 0 {
        return VoiceDebugMenuEffect::IGNORED;
    }

    match runtime.interactive_debug_menu() {
        VisualInteractiveDebugMenu::Voice => match runtime.interactive_debug_row() {
            1 => {
                runtime
                    .mark_action_status(session.dialogue_scroll.voice_debug.toggle_diagnostics());
                VoiceDebugMenuEffect::HANDLED
            }
            2 => {
                if session.stt_state.is_running() {
                    runtime.mark_action_status(
                        "Voice test mode toggle unavailable: voice is listening",
                    );
                } else {
                    runtime.mark_action_status(
                        session.dialogue_scroll.voice_debug.toggle_voice_test_mode(),
                    );
                }
                VoiceDebugMenuEffect::HANDLED
            }
            3 => {
                runtime.mark_action_status(session.tts_state.toggle_muted());
                session.sync_tts_debug();
                VoiceDebugMenuEffect::HANDLED
            }
            4 => {
                let delta = if input == VisualInput::Left { -1 } else { 1 };
                runtime.mark_action_status(session.cycle_selected_mic(delta));
                VoiceDebugMenuEffect::HANDLED
            }
            5 => {
                if session.mic_test_running {
                    runtime.mark_action_status("Mic test already running");
                } else {
                    session.mic_test_running = true;
                    let selected_label = session.selected_mic_label().to_string();
                    session
                        .dialogue_scroll
                        .voice_debug
                        .mark_mic_test_started(&selected_label);
                    runtime.mark_action_status(format!("Mic test listening: {selected_label}"));
                    spawn_mic_test(session.selected_mic_device(), mic_test_tx.clone());
                }
                VoiceDebugMenuEffect::HANDLED
            }
            7 => {
                runtime.mark_action_status(session.enqueue_tts_test());
                VoiceDebugMenuEffect::HANDLED
            }
            8 => {
                runtime.mark_action_status(session.interrupt_tts_queue());
                VoiceDebugMenuEffect::HANDLED
            }
            _ => VoiceDebugMenuEffect::IGNORED,
        },
        VisualInteractiveDebugMenu::Compose => match runtime.interactive_debug_row() {
            1 => {
                if session.compose_backend_running {
                    runtime.mark_action_status(
                        "Compose debug backend toggle unavailable: compose is running",
                    );
                    VoiceDebugMenuEffect::HANDLED
                } else {
                    let status = session.compose_debug_backend.toggle();
                    session.dialogue_scroll.voice_debug.fake_codex_backend =
                        session.compose_debug_backend.is_fake();
                    runtime.clear_compose_history();
                    session.interrupt_tts_queue();
                    runtime.mark_action_status(status);
                    VoiceDebugMenuEffect::RESET_COMPOSE_DIALOGUE
                }
            }
            2 => {
                runtime.clear_compose_history();
                session.interrupt_tts_queue();
                runtime.mark_action_status("Compose dialogue history cleared");
                VoiceDebugMenuEffect::RESET_COMPOSE_DIALOGUE
            }
            _ => VoiceDebugMenuEffect::IGNORED,
        },
        _ => VoiceDebugMenuEffect::IGNORED,
    }
}
