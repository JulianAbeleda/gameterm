use std::sync::mpsc;

use super::super::visual_stt::{SceneSttConfig, SceneSttSession, SceneSttState};
use super::super::visual_tts::{SceneTtsConfig, SceneTtsResult, SceneTtsState, SceneTtsWorker};
use super::visual_compose_dock::SceneComposeDock;
use super::visual_compose_result::PendingFirstVoiceReveal;
use super::visual_dialogue_scroll::SceneDialogueScrollback;
use super::visual_voice_debug::SceneVoiceDebugState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SceneComposeDebugBackend {
    RealCodex,
    FakeCodex,
}

impl SceneComposeDebugBackend {
    pub(super) fn toggle(&mut self) -> &'static str {
        *self = match self {
            Self::RealCodex => Self::FakeCodex,
            Self::FakeCodex => Self::RealCodex,
        };
        self.status()
    }

    pub(super) fn status(self) -> &'static str {
        match self {
            Self::RealCodex => "Compose debug backend: Codex",
            Self::FakeCodex => "Compose debug backend: Fake Codex",
        }
    }

    pub(super) fn is_fake(self) -> bool {
        matches!(self, Self::FakeCodex)
    }
}

pub(super) struct VisualOverlaySession {
    pub(super) compose_dock: SceneComposeDock,
    pub(super) dialogue_scroll: SceneDialogueScrollback,
    pub(super) compose_debug_backend: SceneComposeDebugBackend,
    pub(super) compose_backend_running: bool,
    pub(super) tts_worker: SceneTtsWorker,
    pub(super) tts_state: SceneTtsState,
    pub(super) sync_first_voice_reveal: bool,
    pub(super) first_voice_reveal_done: bool,
    pub(super) pending_first_voice_reveal: Option<PendingFirstVoiceReveal>,
    pub(super) stt_config: SceneSttConfig,
    pub(super) stt_state: SceneSttState,
    pub(super) stt_session: Option<SceneSttSession>,
    pub(super) last_idle_sprite: Option<String>,
}

impl VisualOverlaySession {
    pub(super) fn new(
        tts_config: SceneTtsConfig,
        tts_tx: mpsc::Sender<SceneTtsResult>,
        stt_config: SceneSttConfig,
    ) -> Self {
        let sync_first_voice_reveal = tts_config.can_play_audio();
        let tts_worker = SceneTtsWorker::new(tts_config, tts_tx);
        let stt_state = SceneSttState::default();
        let mut dialogue_scroll = SceneDialogueScrollback::default();
        dialogue_scroll.voice_debug = SceneVoiceDebugState::new(&stt_config, &stt_state);

        Self {
            compose_dock: SceneComposeDock::default(),
            dialogue_scroll,
            compose_debug_backend: SceneComposeDebugBackend::RealCodex,
            compose_backend_running: false,
            tts_worker,
            tts_state: SceneTtsState::default(),
            sync_first_voice_reveal,
            first_voice_reveal_done: false,
            pending_first_voice_reveal: None,
            stt_config,
            stt_state,
            stt_session: None,
            last_idle_sprite: None,
        }
    }
}
