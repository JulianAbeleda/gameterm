use super::super::visual_stt::{SceneSttConfig, SceneSttResult, SceneSttState};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct SceneVoiceDebugState {
    pub(super) visible: bool,
    pub(super) test_mode: bool,
    pub(super) fake_codex_backend: bool,
    config_lines: Vec<String>,
    last_status: String,
    last_transcript: Option<String>,
    last_error: Option<String>,
}

impl SceneVoiceDebugState {
    pub(super) fn new(config: &SceneSttConfig, state: &SceneSttState) -> Self {
        Self {
            config_lines: config.diagnostics_lines(),
            last_status: state.last_status().to_string(),
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
            "Microphone status",
            &self.last_status,
        ));
        lines.push(self.menu_item_line_for_row(selected_row, 5, "Test TTS playback", "planned"));

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
            lines.extend(self.config_lines.iter().cloned());
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
