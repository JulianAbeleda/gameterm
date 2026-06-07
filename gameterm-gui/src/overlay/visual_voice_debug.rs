use super::super::visual_stt::{
    SceneMicDevice, SceneMicTestResult, SceneSttConfig, SceneSttResult, SceneSttState,
};

#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct SceneVoiceDebugState {
    pub(super) visible: bool,
    pub(super) test_mode: bool,
    pub(super) fake_codex_backend: bool,
    config_lines: Vec<String>,
    last_status: String,
    last_transcript: Option<String>,
    last_error: Option<String>,
    selected_microphone: String,
    microphone_devices: Vec<SceneMicDevice>,
    mic_test_running: bool,
    last_mic_test: Option<SceneMicTestResult>,
}

impl SceneVoiceDebugState {
    pub(super) fn new(config: &SceneSttConfig, state: &SceneSttState) -> Self {
        Self {
            config_lines: config.diagnostics_lines(),
            last_status: state.last_status().to_string(),
            selected_microphone: "system default".to_string(),
            ..Self::default()
        }
    }

    pub(super) fn toggle_diagnostics(&mut self) -> &'static str {
        self.toggle_visible()
    }

    pub(super) fn toggle_voice_test_mode(&mut self) -> &'static str {
        self.toggle_test_mode()
    }

    fn toggle_visible(&mut self) -> &'static str {
        self.visible = !self.visible;
        if self.visible {
            "Voice diagnostics shown"
        } else {
            "Voice diagnostics hidden"
        }
    }

    fn toggle_test_mode(&mut self) -> &'static str {
        self.visible = true;
        self.test_mode = !self.test_mode;
        if self.test_mode {
            "Voice test mode enabled"
        } else {
            "Voice test mode disabled"
        }
    }

    pub(super) fn sync_status(&mut self, status: &str) {
        self.last_status = status.to_string();
    }

    pub(super) fn apply_result(&mut self, result: &SceneSttResult) {
        self.last_status = result.status.clone();
        self.last_transcript = result.transcript.clone();
        self.last_error = result.error.clone();
    }

    pub(super) fn sync_microphones(
        &mut self,
        config: &SceneSttConfig,
        devices: &[SceneMicDevice],
        selected_label: &str,
    ) {
        self.config_lines = config.diagnostics_lines();
        self.microphone_devices = devices.to_vec();
        self.selected_microphone = selected_label.to_string();
    }

    pub(super) fn mark_mic_test_started(&mut self, selected_label: &str) {
        self.visible = true;
        self.mic_test_running = true;
        self.selected_microphone = selected_label.to_string();
        self.last_status = format!("Mic test listening: {selected_label}");
        self.last_mic_test = None;
    }

    pub(super) fn apply_mic_test_result(&mut self, result: SceneMicTestResult) {
        self.mic_test_running = false;
        self.last_status = result.status.clone();
        self.last_error = result.error.clone();
        self.last_mic_test = Some(result);
    }

    pub(super) fn render_voice_lines(&self, selected_row: usize) -> Vec<String> {
        let mut lines = Vec::new();
        lines.push("Voice".to_string());
        lines.push(self.menu_item_line_for_row(
            selected_row,
            1,
            "Scene voice diagnostics",
            if self.visible { "on" } else { "off" },
        ));
        lines.push(self.menu_item_line_for_row(
            selected_row,
            2,
            "Voice test mode",
            if self.test_mode { "on" } else { "off" },
        ));
        lines.push(self.menu_item_line_for_row(selected_row, 3, "TTS mute", "overlay state"));
        lines.push(self.menu_item_line_for_row(
            selected_row,
            4,
            "Microphone",
            &self.selected_microphone,
        ));
        lines.push(self.menu_item_line_for_row(
            selected_row,
            5,
            "Mic test",
            if self.mic_test_running {
                "listening"
            } else {
                "enter"
            },
        ));
        lines.push(self.menu_item_line_for_row(
            selected_row,
            6,
            "Microphone status",
            &self.last_status,
        ));
        lines.push(self.menu_item_line_for_row(selected_row, 7, "Test TTS playback", "planned"));

        if self.visible {
            lines.push(String::new());
            lines.push(format!(
                "Mode: {}",
                if self.test_mode {
                    "test recognition only"
                } else {
                    "compose transcript"
                }
            ));
            lines.push(format!("Status: {}", self.last_status));
            lines.push(format!("Selected mic: {}", self.selected_microphone));
            if !self.microphone_devices.is_empty() {
                lines.push("Available mics:".to_string());
                for device in &self.microphone_devices {
                    let marker = if device.is_default { " default" } else { "" };
                    lines.push(format!("  - {}{}", device.name, marker));
                }
            }
            lines.extend(self.config_lines.iter().cloned());
            if let Some(result) = self.last_mic_test.as_ref() {
                if let Some(peak) = result.peak {
                    let rms = result.rms.unwrap_or_default();
                    lines.push(format!("Last mic test: peak {peak:.3}, rms {rms:.3}"));
                }
            }
            if let Some(transcript) = self.last_transcript.as_deref() {
                lines.push(format!("Last transcript: {transcript}"));
            }
            if let Some(error) = self.last_error.as_deref() {
                lines.push(format!("Last error: {error}"));
            }
        }
        lines
    }

    pub(super) fn render_compose_lines(
        &self,
        selected_row: usize,
        history_len: usize,
    ) -> Vec<String> {
        vec![
            "Compose".to_string(),
            self.menu_line_for_row(
                selected_row,
                1,
                "Codex backend",
                if self.fake_codex_backend {
                    "Fake Codex"
                } else {
                    "Codex"
                },
            ),
            self.menu_line_for_row(selected_row, 2, "Clear dialogue history", "enter"),
            self.menu_line_for_row(selected_row, 3, "Compose running", "overlay session"),
            self.menu_line_for_row(selected_row, 4, "History entries", &history_len.to_string()),
            String::new(),
            "Fake Codex is controlled here; toggling it clears the dialogue box.".to_string(),
        ]
    }

    fn menu_item_line_for_row(
        &self,
        selected_row: usize,
        row: usize,
        label: &str,
        value: &str,
    ) -> String {
        let marker = if selected_row == row { ">" } else { " " };
        format!("{marker} {label:<28} {value}")
    }

    fn menu_line_for_row(
        &self,
        selected_row: usize,
        row: usize,
        label: &str,
        value: &str,
    ) -> String {
        let marker = if selected_row == row { ">" } else { " " };
        format!("{marker} {label:<28} {value}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct VoiceDebugMenuEffect {
    pub(super) handled: bool,
    pub(super) reset_compose_dialogue: bool,
}

impl VoiceDebugMenuEffect {
    pub(super) const IGNORED: Self = Self {
        handled: false,
        reset_compose_dialogue: false,
    };

    pub(super) const HANDLED: Self = Self {
        handled: true,
        reset_compose_dialogue: false,
    };

    pub(super) const RESET_COMPOSE_DIALOGUE: Self = Self {
        handled: true,
        reset_compose_dialogue: true,
    };
}
