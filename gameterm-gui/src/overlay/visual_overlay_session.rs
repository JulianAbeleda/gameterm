use std::sync::mpsc;

use super::super::visual_stt::{
    scene_microphone_devices, SceneMicDevice, SceneSttConfig, SceneSttSession, SceneSttState,
};
use super::super::visual_tts::{SceneTtsConfig, SceneTtsResult, SceneTtsState, SceneTtsWorker};
use super::visual_compose_dock::SceneComposeDock;
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
    pub(super) stt_config: SceneSttConfig,
    pub(super) stt_state: SceneSttState,
    pub(super) stt_session: Option<SceneSttSession>,
    pub(super) mic_devices: Vec<SceneMicDevice>,
    pub(super) selected_mic_index: usize,
    pub(super) mic_test_running: bool,
    pub(super) last_idle_sprite: Option<String>,
}

impl VisualOverlaySession {
    pub(super) fn new(
        tts_config: SceneTtsConfig,
        tts_tx: mpsc::Sender<SceneTtsResult>,
        stt_config: SceneSttConfig,
    ) -> Self {
        let tts_worker = SceneTtsWorker::new(tts_config, tts_tx);
        let mic_devices = scene_microphone_devices().unwrap_or_default();
        let stt_state = SceneSttState::default();
        let mut dialogue_scroll = SceneDialogueScrollback::default();
        dialogue_scroll.voice_debug = SceneVoiceDebugState::new(&stt_config, &stt_state);
        dialogue_scroll.voice_debug.sync_microphones(
            &stt_config,
            &mic_devices,
            selected_mic_label(&mic_devices, 0),
        );

        Self {
            compose_dock: SceneComposeDock::default(),
            dialogue_scroll,
            compose_debug_backend: SceneComposeDebugBackend::RealCodex,
            compose_backend_running: false,
            tts_worker,
            tts_state: SceneTtsState::default(),
            stt_config,
            stt_state,
            stt_session: None,
            mic_devices,
            selected_mic_index: 0,
            mic_test_running: false,
            last_idle_sprite: None,
        }
    }

    #[cfg(test)]
    pub(super) fn new_with_mic_devices(
        tts_config: SceneTtsConfig,
        tts_tx: mpsc::Sender<SceneTtsResult>,
        stt_config: SceneSttConfig,
        mic_devices: Vec<SceneMicDevice>,
    ) -> Self {
        let mut session = Self::new(tts_config, tts_tx, stt_config);
        session.mic_devices = mic_devices;
        session.selected_mic_index = 0;
        session.sync_voice_microphones();
        session
    }

    pub(super) fn selected_mic_label(&self) -> &str {
        selected_mic_label(&self.mic_devices, self.selected_mic_index)
    }

    pub(super) fn selected_mic_device(&self) -> Option<String> {
        if self.selected_mic_index == 0 {
            None
        } else {
            self.mic_devices
                .get(self.selected_mic_index - 1)
                .map(|device| device.name.clone())
        }
    }

    pub(super) fn selected_stt_config(&self) -> SceneSttConfig {
        self.stt_config
            .with_input_device(self.selected_mic_device())
    }

    pub(super) fn cycle_selected_mic(&mut self, delta: isize) -> String {
        let count = self.mic_devices.len() + 1;
        if count == 0 {
            self.selected_mic_index = 0;
        } else if delta >= 0 {
            self.selected_mic_index = (self.selected_mic_index + delta as usize) % count;
        } else {
            let delta = delta.unsigned_abs() % count;
            self.selected_mic_index = (self.selected_mic_index + count - delta) % count;
        }
        self.sync_voice_microphones();
        format!("Microphone selected: {}", self.selected_mic_label())
    }

    pub(super) fn sync_voice_microphones(&mut self) {
        let config = self.selected_stt_config();
        let label = self.selected_mic_label().to_string();
        self.dialogue_scroll
            .voice_debug
            .sync_microphones(&config, &self.mic_devices, &label);
    }
}

fn selected_mic_label(devices: &[SceneMicDevice], index: usize) -> &str {
    if index == 0 {
        "system default"
    } else {
        devices
            .get(index - 1)
            .map(|device| device.name.as_str())
            .unwrap_or("system default")
    }
}
