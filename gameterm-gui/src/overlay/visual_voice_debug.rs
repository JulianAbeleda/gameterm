use super::super::visual_stt::{SceneSttConfig, SceneSttResult, SceneSttState};
use super::SceneComposeDebugBackend;
use gameterm_visual::SceneRuntime;
use termwiz::input::{KeyCode, Modifiers};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct SceneVoiceDebugState {
    pub(super) menu_open: bool,
    pub(super) selected_item: usize,
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

    pub(super) fn open_menu(&mut self) -> &'static str {
        self.menu_open = true;
        self.visible = true;
        "Voice debug menu opened"
    }

    pub(super) fn close_menu(&mut self) -> &'static str {
        self.menu_open = false;
        "Voice debug menu closed"
    }

    pub(super) fn select_next(&mut self) {
        self.selected_item = (self.selected_item + 1).min(Self::MENU_ITEM_COUNT - 1);
    }

    pub(super) fn select_previous(&mut self) {
        self.selected_item = self.selected_item.saturating_sub(1);
    }

    pub(super) fn toggle_selected(&mut self) -> &'static str {
        match self.selected_item {
            Self::MENU_ITEM_DIAGNOSTICS => self.toggle_visible(),
            Self::MENU_ITEM_TEST_MODE => self.toggle_test_mode(),
            _ => "Voice debug menu unchanged",
        }
    }

    fn toggle_visible(&mut self) -> &'static str {
        self.visible = !self.visible;
        if self.visible {
            "Voice diagnostics shown"
        } else {
            "Voice diagnostics hidden"
        }
    }

    pub(super) const MENU_ITEM_COUNT: usize = 3;
    pub(super) const MENU_ITEM_DIAGNOSTICS: usize = 0;
    pub(super) const MENU_ITEM_TEST_MODE: usize = 1;
    pub(super) const MENU_ITEM_FAKE_CODEX: usize = 2;

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

    pub(super) fn render_lines(&self) -> Vec<String> {
        if !self.menu_open {
            return Vec::new();
        }
        let mut lines = Vec::new();
        lines.push("Scene Voice Debug".to_string());
        lines.push(
            "[jk: select] [enter: toggle] [tab/esc: debug] [cmd+shift: hold mic]".to_string(),
        );
        lines.push(String::new());
        lines.push(self.menu_item_line(
            Self::MENU_ITEM_DIAGNOSTICS,
            "Scene voice diagnostics",
            if self.visible { "on" } else { "off" },
        ));
        lines.push(self.menu_item_line(
            Self::MENU_ITEM_TEST_MODE,
            "Voice test mode",
            if self.test_mode { "on" } else { "off" },
        ));
        lines.push(self.menu_item_line(
            Self::MENU_ITEM_FAKE_CODEX,
            "Fake Codex backend",
            if self.fake_codex_backend { "on" } else { "off" },
        ));

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

    fn menu_item_line(&self, item: usize, label: &str, value: &str) -> String {
        let marker = if self.selected_item == item { ">" } else { " " };
        format!("{marker} {label:<28} {value}")
    }
}

pub(super) fn is_voice_debug_menu_open_key(key: KeyCode, modifiers: Modifiers) -> bool {
    matches!(key, KeyCode::Char('v') | KeyCode::Char('V')) && modifiers == Modifiers::NONE
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

pub(super) fn handle_voice_debug_menu_key(
    key: KeyCode,
    runtime: &mut SceneRuntime,
    voice_debug: &mut SceneVoiceDebugState,
    voice_running: bool,
    compose_running: bool,
    compose_debug_backend: &mut SceneComposeDebugBackend,
) -> VoiceDebugMenuEffect {
    match key {
        KeyCode::Escape | KeyCode::Tab => {
            runtime.mark_action_status(voice_debug.close_menu());
            VoiceDebugMenuEffect::HANDLED
        }
        KeyCode::DownArrow | KeyCode::Char('j') | KeyCode::Char('J') => {
            voice_debug.select_next();
            VoiceDebugMenuEffect::HANDLED
        }
        KeyCode::UpArrow | KeyCode::Char('k') | KeyCode::Char('K') => {
            voice_debug.select_previous();
            VoiceDebugMenuEffect::HANDLED
        }
        KeyCode::Enter => {
            if voice_running
                && voice_debug.selected_item == SceneVoiceDebugState::MENU_ITEM_TEST_MODE
            {
                runtime
                    .mark_action_status("Voice test mode toggle unavailable: voice is listening");
            } else if compose_running
                && voice_debug.selected_item == SceneVoiceDebugState::MENU_ITEM_FAKE_CODEX
            {
                runtime.mark_action_status(
                    "Compose debug backend toggle unavailable: compose is running",
                );
            } else if voice_debug.selected_item == SceneVoiceDebugState::MENU_ITEM_FAKE_CODEX {
                let status = compose_debug_backend.toggle();
                voice_debug.fake_codex_backend = compose_debug_backend.is_fake();
                runtime.clear_compose_history();
                runtime.mark_action_status(status);
                return VoiceDebugMenuEffect::RESET_COMPOSE_DIALOGUE;
            } else {
                runtime.mark_action_status(voice_debug.toggle_selected());
            }
            VoiceDebugMenuEffect::HANDLED
        }
        _ => VoiceDebugMenuEffect::IGNORED,
    }
}
