use super::SceneRuntime;
use crate::vn_layout::VnOverlayDebugOverrides;
use crate::vn_text::truncate_to_screen;
use crate::{VisualInput, VisualInteractiveDebugMenu, VisualModeOutcome, VisualView};

impl SceneRuntime {
    fn debug_menu_row_count_for(&self, section: VisualInteractiveDebugMenu) -> usize {
        match section {
            VisualInteractiveDebugMenu::SceneLayout => VnOverlayDebugOverrides::PARAM_COUNT,
            VisualInteractiveDebugMenu::Text => 6,
            VisualInteractiveDebugMenu::Voice => 8,
            VisualInteractiveDebugMenu::Compose => 4,
            VisualInteractiveDebugMenu::Runtime => 6,
            VisualInteractiveDebugMenu::TileDebugMenu => 0,
        }
    }

    pub(super) fn debug_menu_row_count(&self) -> usize {
        self.debug_menu_row_count_for(self.interactive_debug_menu)
    }

    fn selected_debug_marker(&self, row: usize) -> &'static str {
        if self.debug_selected_row == row {
            ">"
        } else {
            " "
        }
    }

    fn debug_section_tabs(&self) -> String {
        VisualInteractiveDebugMenu::SCENE_SECTIONS
            .iter()
            .map(|section| {
                if *section == self.interactive_debug_menu {
                    format!("[{}]", section.label())
                } else {
                    format!(" {} ", section.label())
                }
            })
            .collect::<Vec<_>>()
            .join(" | ")
    }

    fn vn_layout_debug_menu_lines(&self) -> Vec<String> {
        let overrides = self.vn_layout_debug.clone().unwrap_or_default();
        let editing = overrides.editing_buffer.is_some();
        let mut lines = Vec::new();
        lines.push(format!(
            "{} Sections {}",
            self.selected_debug_marker(0),
            self.debug_section_tabs()
        ));
        lines.push("  left/right on this row changes section".to_string());
        lines.push(String::new());
        lines.push("Scene Layout".to_string());
        for i in 0..VnOverlayDebugOverrides::PARAM_COUNT {
            let row = i + 1;
            let marker = self.selected_debug_marker(row);
            let value = if i == overrides.selected_param && editing {
                format!("{}_", overrides.editing_buffer.as_deref().unwrap_or(""))
            } else {
                overrides.param_value_str(i)
            };
            lines.push(format!(
                "{} {:<22} {:>7}",
                marker,
                VnOverlayDebugOverrides::param_label(i),
                value,
            ));
        }
        lines
    }

    fn static_debug_menu_lines(
        &self,
        title: &str,
        rows: Vec<(String, String)>,
        details: &[String],
    ) -> Vec<String> {
        let mut lines = Vec::new();
        lines.push(format!(
            "{} Sections {}",
            self.selected_debug_marker(0),
            self.debug_section_tabs()
        ));
        lines.push("  left/right on this row changes section".to_string());
        lines.push(String::new());
        lines.push(title.to_string());
        for (idx, (label, value)) in rows.iter().enumerate() {
            lines.push(format!(
                "{} {:<28} {}",
                self.selected_debug_marker(idx + 1),
                label,
                value
            ));
        }
        if !details.is_empty() {
            lines.push(String::new());
            lines.extend(details.iter().cloned());
        }
        lines
    }

    fn text_debug_menu_lines(&self) -> Vec<String> {
        self.static_debug_menu_lines(
            "Text",
            vec![
                ("Dialogue font scale".to_string(), "planned".to_string()),
                ("Composer font scale".to_string(), "planned".to_string()),
                ("Nameplate font scale".to_string(), "planned".to_string()),
                (
                    "Dialogue text offset".to_string(),
                    "use Scene Layout text insets".to_string(),
                ),
                (
                    "Composer text offset".to_string(),
                    "use Scene Layout text insets".to_string(),
                ),
                (
                    "Scrollback rows".to_string(),
                    "auto from dialogue box height".to_string(),
                ),
            ],
            &[String::from(
                "Text primitives are staged here; current adjustable text positions live in Scene Layout.",
            )],
        )
    }

    fn voice_debug_menu_lines(&self) -> Vec<String> {
        self.static_debug_menu_lines(
            "Voice",
            vec![
                (
                    "Scene voice diagnostics".to_string(),
                    "overlay session".to_string(),
                ),
                ("Voice test mode".to_string(), "overlay session".to_string()),
                ("TTS mute".to_string(), "overlay session".to_string()),
                ("Microphone".to_string(), "overlay session".to_string()),
                ("Mic test".to_string(), "enter".to_string()),
                (
                    "Microphone status".to_string(),
                    "overlay session".to_string(),
                ),
                ("Test TTS playback".to_string(), "enter".to_string()),
                ("Stop TTS playback".to_string(), "enter".to_string()),
            ],
            &[String::from(
                "Use enter or left/right on a voice row to toggle supported items.",
            )],
        )
    }

    fn compose_debug_menu_lines(&self) -> Vec<String> {
        self.static_debug_menu_lines(
            "Compose",
            vec![
                ("Codex backend".to_string(), "overlay session".to_string()),
                ("Clear dialogue history".to_string(), "enter".to_string()),
                ("Compose running".to_string(), "overlay session".to_string()),
                (
                    "History entries".to_string(),
                    self.compose_state.history.len().to_string(),
                ),
            ],
            &[String::from(
                "Fake Codex is controlled here; toggling it clears the dialogue box.",
            )],
        )
    }

    fn runtime_debug_menu_lines(&self) -> Vec<String> {
        let scene_path = if self.scene_source.scene_path.is_empty() {
            "(bundled)".to_string()
        } else {
            self.scene_source.scene_path.clone()
        };
        self.static_debug_menu_lines(
            "Runtime",
            vec![
                ("Status".to_string(), self.status.clone()),
                ("Generation".to_string(), self.generation.to_string()),
                ("Scene source".to_string(), scene_path),
                (
                    "Sprite stage layers".to_string(),
                    self.scene.stage.layers.len().to_string(),
                ),
                ("Choices".to_string(), self.scene.choices.len().to_string()),
                (
                    "RPG/tile layer".to_string(),
                    "disabled in staged VN mode".to_string(),
                ),
            ],
            &[],
        )
    }

    pub(super) fn render_interactive_debugger(&self, cols: usize, rows: usize) -> String {
        let mut out = String::new();
        out.push_str("  Debug 2\r\n");
        let editing = self
            .vn_layout_debug
            .as_ref()
            .map_or(false, |d| d.editing_buffer.is_some());
        if editing {
            out.push_str("  [enter: confirm] [esc: cancel]\r\n\r\n");
        } else {
            out.push_str("  [tab/esc: scene] [up/down: row] [left/right: section or value] [enter: action/edit] [r: reset]\r\n\r\n");
        }
        for line in match self.interactive_debug_menu {
            VisualInteractiveDebugMenu::SceneLayout => self.vn_layout_debug_menu_lines(),
            VisualInteractiveDebugMenu::Text => self.text_debug_menu_lines(),
            VisualInteractiveDebugMenu::Voice => self.voice_debug_menu_lines(),
            VisualInteractiveDebugMenu::Compose => self.compose_debug_menu_lines(),
            VisualInteractiveDebugMenu::Runtime => self.runtime_debug_menu_lines(),
            VisualInteractiveDebugMenu::TileDebugMenu => self.static_debug_menu_lines(
                "Tile",
                vec![(
                    "Tile debug menu".to_string(),
                    "disabled for VN Scene Mode".to_string(),
                )],
                &[],
            ),
        } {
            out.push_str("  ");
            out.push_str(&line);
            out.push_str("\r\n");
        }
        truncate_to_screen(out, cols, rows)
    }

    pub(super) fn handle_vn_layout_debug_input(&mut self, input: VisualInput) -> VisualModeOutcome {
        let editing = self
            .vn_layout_debug
            .as_ref()
            .map_or(false, |d| d.editing_buffer.is_some());
        match input {
            VisualInput::Close if editing => {
                if let Some(ref mut d) = self.vn_layout_debug {
                    d.cancel_edit();
                }
            }
            VisualInput::Close | VisualInput::ToggleDebug => {
                self.view = VisualView::Scene;
            }
            VisualInput::Right if !editing && self.debug_selected_row == 0 => {
                self.show_interactive_debug_menu(self.interactive_debug_menu.next_scene_section());
            }
            VisualInput::Left if !editing && self.debug_selected_row == 0 => {
                self.show_interactive_debug_menu(
                    self.interactive_debug_menu.previous_scene_section(),
                );
            }
            VisualInput::Activate
                if self.interactive_debug_menu == VisualInteractiveDebugMenu::SceneLayout
                    && editing =>
            {
                if let Some(ref mut d) = self.vn_layout_debug {
                    d.commit_edit();
                }
            }
            VisualInput::Activate
                if self.interactive_debug_menu == VisualInteractiveDebugMenu::SceneLayout
                    && self.debug_selected_row > 0 =>
            {
                if let Some(ref mut d) = self.vn_layout_debug {
                    d.selected_param = self.debug_selected_row.saturating_sub(1);
                    d.begin_edit();
                }
            }
            VisualInput::Backspace
                if self.interactive_debug_menu == VisualInteractiveDebugMenu::SceneLayout =>
            {
                if let Some(ref mut d) = self.vn_layout_debug {
                    d.pop_char();
                }
            }
            VisualInput::Next if !editing => {
                self.debug_selected_row =
                    (self.debug_selected_row + 1).min(self.debug_menu_row_count());
                self.sync_layout_selected_param();
            }
            VisualInput::Previous if !editing => {
                self.debug_selected_row = self.debug_selected_row.saturating_sub(1);
                self.sync_layout_selected_param();
            }
            VisualInput::Right
                if self.interactive_debug_menu == VisualInteractiveDebugMenu::SceneLayout
                    && self.debug_selected_row > 0
                    && !editing =>
            {
                if let Some(ref mut d) = self.vn_layout_debug {
                    d.selected_param = self.debug_selected_row.saturating_sub(1);
                    d.adjust(1);
                }
            }
            VisualInput::Left
                if self.interactive_debug_menu == VisualInteractiveDebugMenu::SceneLayout
                    && self.debug_selected_row > 0
                    && !editing =>
            {
                if let Some(ref mut d) = self.vn_layout_debug {
                    d.selected_param = self.debug_selected_row.saturating_sub(1);
                    d.adjust(-1);
                }
            }
            VisualInput::Char('+') | VisualInput::Char('=')
                if self.interactive_debug_menu == VisualInteractiveDebugMenu::SceneLayout
                    && self.debug_selected_row > 0
                    && !editing =>
            {
                if let Some(ref mut d) = self.vn_layout_debug {
                    d.selected_param = self.debug_selected_row.saturating_sub(1);
                    d.adjust(1);
                }
            }
            VisualInput::Char('-') | VisualInput::Char('_')
                if self.interactive_debug_menu == VisualInteractiveDebugMenu::SceneLayout
                    && self.debug_selected_row > 0
                    && !editing =>
            {
                if let Some(ref mut d) = self.vn_layout_debug {
                    d.selected_param = self.debug_selected_row.saturating_sub(1);
                    d.adjust(-1);
                }
            }
            VisualInput::Char(c)
                if self.interactive_debug_menu == VisualInteractiveDebugMenu::SceneLayout
                    && editing =>
            {
                if let Some(ref mut d) = self.vn_layout_debug {
                    d.push_char(c);
                }
            }
            VisualInput::Reload
                if self.interactive_debug_menu == VisualInteractiveDebugMenu::SceneLayout
                    && !editing =>
            {
                self.vn_layout_debug = Some(VnOverlayDebugOverrides::default());
                self.sync_layout_selected_param();
            }
            _ => return VisualModeOutcome::Continue,
        }
        self.bump_generation();
        VisualModeOutcome::Continue
    }

    fn sync_layout_selected_param(&mut self) {
        if self.interactive_debug_menu == VisualInteractiveDebugMenu::SceneLayout
            && self.debug_selected_row > 0
        {
            if let Some(ref mut d) = self.vn_layout_debug {
                d.selected_param = self
                    .debug_selected_row
                    .saturating_sub(1)
                    .min(VnOverlayDebugOverrides::PARAM_COUNT - 1);
            }
        }
    }
}
