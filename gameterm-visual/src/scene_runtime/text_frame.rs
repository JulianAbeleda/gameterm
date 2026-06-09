use super::actions::{action_kind_name, action_policy_summary};
use super::SceneRuntime;
#[cfg(test)]
use crate::conditions::condition_guard_detail;
use crate::conditions::conditions_match;
use crate::runtime_status::{
    format_layer_summary, format_metadata_summary, format_process_summary,
    format_relationship_summary, format_state_summary,
};
#[cfg(test)]
use crate::runtime_status::{format_relationship_metadata, relationship_entity_label};
use crate::vn_text::truncate_to_screen;
use crate::{VisualEntityKind, VisualSceneLoadStatus, VisualView};

impl SceneRuntime {
    pub fn render_text_frame(&self, cols: usize, rows: usize) -> String {
        self.render_text_frame_with_dialogue_scroll(cols, rows, 0)
    }

    pub fn render_text_frame_with_dialogue_scroll(
        &self,
        cols: usize,
        rows: usize,
        dialogue_scroll_offset: usize,
    ) -> String {
        self.render_text_frame_with_dialogue_scroll_and_voice_hold(
            cols,
            rows,
            dialogue_scroll_offset,
            false,
        )
    }

    pub fn render_text_frame_with_dialogue_scroll_and_voice_hold(
        &self,
        cols: usize,
        rows: usize,
        dialogue_scroll_offset: usize,
        voice_hold_active: bool,
    ) -> String {
        match self.view {
            VisualView::Scene => {
                self.render_scene(cols, rows, dialogue_scroll_offset, voice_hold_active)
            }
            VisualView::CommandSelection => self.render_command_selection(cols, rows),
            VisualView::TileDebugger => self.render_interactive_debugger(cols, rows),
            VisualView::VnLayoutDebugger => self.render_interactive_debugger(cols, rows),
        }
    }

    fn render_scene(
        &self,
        cols: usize,
        rows: usize,
        dialogue_scroll_offset: usize,
        voice_hold_active: bool,
    ) -> String {
        if !self.scene.stage.is_empty() {
            return self.render_staged_scene(cols, rows, dialogue_scroll_offset, voice_hold_active);
        }

        let mut out = String::new();
        out.push_str(&format!("{}\r\n", self.scene.title));
        out.push_str("Scene Mode  [arrows/hjkl: select] [enter: action] [tab: debugger] [r: reload] [esc/q: close]\r\n\r\n");
        if self.scene_source.load_status == VisualSceneLoadStatus::ReloadFailed {
            if let Some(error) = &self.scene_source.last_error {
                out.push_str(&format!("Reload failed: {error}\r\n\r\n"));
            }
        }

        let mut grid = vec![vec!['.'; self.scene.width]; self.scene.height];
        for (idx, entity) in self.scene.entities.iter().enumerate() {
            let glyph = match entity.kind {
                VisualEntityKind::Agent => 'A',
                VisualEntityKind::Memory => 'M',
                VisualEntityKind::Principle => 'P',
                VisualEntityKind::Project => 'R',
                VisualEntityKind::Task => 'T',
            };
            grid[entity.position.y][entity.position.x] = if idx == self.selected_entity {
                '@'
            } else {
                glyph
            };
        }

        let available_grid_rows = rows.saturating_sub(13).min(self.scene.height);
        for row in grid.into_iter().take(available_grid_rows) {
            out.push_str("  ");
            for ch in row {
                out.push(ch);
                out.push(' ');
            }
            out.push_str("\r\n");
        }

        out.push_str("\r\n");
        if let Some(entity) = self.selected_entity() {
            out.push_str(&format!(
                "Selected: {} [{:?}] sprite={} flags={}\r\n",
                entity.label,
                entity.kind,
                entity.sprite,
                entity.state_flags.join(", ")
            ));
            let metadata = format_metadata_summary(&entity.metadata, 4);
            if !metadata.is_empty() {
                out.push_str(&format!("Details: {metadata}\r\n"));
            }
            if let Some(summary) = format_relationship_summary(&self.scene, &entity.id, 3) {
                out.push_str(&format!("Relationships: {summary}\r\n"));
            }
        }
        out.push_str(&format!(
            "Mode: {} ({})\r\n",
            self.scene.mode.label, self.scene.mode.mode_id
        ));
        if !self.scene.layers.is_empty() {
            out.push_str(&format!(
                "Layers: {}\r\n",
                format_layer_summary(&self.scene.layers)
            ));
        }
        if let Some(process_state) = &self.last_process_state {
            out.push_str(&format!(
                "Process: {}\r\n",
                format_process_summary(process_state)
            ));
        }
        if !self.scene.variables.is_empty() {
            out.push_str(&format!(
                "State: {}\r\n",
                format_state_summary(&self.scene.variables, 5)
            ));
        }
        if !self.scene.rpg.is_empty() {
            out.push_str(&format!(
                "RPG: inventory={} stats={} quests={} relationships={}\r\n",
                self.scene.rpg.inventory.len(),
                self.scene.rpg.stats.len(),
                self.scene.rpg.quests.len(),
                self.scene.rpg.relationships.len()
            ));
        }
        if let Some(action) = &self.last_story_state_action {
            match &self.last_story_state_path {
                Some(path) => {
                    out.push_str(&format!("Story State: {action} {}\r\n", path.display()))
                }
                None => out.push_str(&format!("Story State: {action}\r\n")),
            }
        } else if self.scene.mode.mode_id == "authoring" {
            out.push_str(&format!(
                "Story State: default {}\r\n",
                self.default_story_state_path().display()
            ));
        }
        out.push_str(&format!(
            "{}: {}\r\n\r\n",
            self.active_dialogue_line().speaker,
            self.active_dialogue_line().text
        ));

        out.push_str("Choices:\r\n");
        let mut last_choice_group: Option<String> = None;
        for (idx, choice) in self.scene.choices.iter().enumerate() {
            let choice_group = action_kind_name(&choice.kind);
            if last_choice_group.as_deref() != Some(choice_group.as_str()) {
                out.push_str(&format!("[{choice_group}]\r\n"));
                last_choice_group = Some(choice_group);
            }
            let marker = if idx == self.selected_choice {
                ">"
            } else {
                " "
            };
            let guard = if conditions_match(
                &choice.conditions,
                &self.scene.variables,
                &self.scene.rpg,
                self.selected_entity(),
                self.last_process_state.as_ref(),
            ) {
                ""
            } else {
                " [locked]"
            };
            out.push_str(&format!(
                "{marker} {}{}  {}\r\n",
                choice.label,
                guard,
                action_policy_summary(choice)
            ));
        }
        out.push_str(&format!("\r\nStatus: {}\r\n", self.status));
        truncate_to_screen(out, cols, rows)
    }

    fn render_command_selection(&self, cols: usize, rows: usize) -> String {
        let mut out = String::new();
        out.push_str("GameTerm Command Selection\r\n");
        out.push_str(
            "[arrows/hjkl: select command] [enter: action] [tab: debugger] [esc/q: close]\r\n\r\n",
        );
        out.push_str("Filter: none\r\n");
        if let Some(entity) = self.selected_entity() {
            out.push_str(&format!(
                "Selected entity: {} ({})\r\n",
                entity.label, entity.id
            ));
        }
        out.push_str("\r\nCommands:\r\n");
        for option in self.command_options() {
            let marker = if option.choice_index == self.selected_choice {
                ">"
            } else {
                " "
            };
            let lock = if option.enabled { "" } else { " locked" };
            let confirm = if option.requires_confirmation {
                " confirm=true"
            } else {
                ""
            };
            let guard = option
                .guard_detail
                .as_ref()
                .map(|detail| format!(" guard={detail}"))
                .unwrap_or_default();
            let summary = option
                .summary
                .as_ref()
                .map(|summary| format!(" summary={summary}"))
                .unwrap_or_default();
            out.push_str(&format!(
                "{marker} #{:02} {:<18} {:<14} {:<16} {:<14} {}{}{}{}{}\r\n",
                option.choice_index,
                option.action_kind,
                option.risk,
                option.origin,
                option.scope,
                option.label,
                lock,
                confirm,
                guard,
                summary
            ));
        }
        out.push_str(&format!("\r\nStatus: {}\r\n", self.status));
        truncate_to_screen(out, cols, rows)
    }

    #[cfg(test)]
    pub(super) fn render_debugger(&self, cols: usize, rows: usize) -> String {
        let report = self.debug_report();
        let mut out = String::new();
        out.push_str("GameTerm Tile Debugger\r\n");
        out.push_str("[tab: scene] [arrows/hjkl: select entity] [esc/q: close]\r\n\r\n");
        out.push_str("Source:\r\n");
        out.push_str(&format!("  Scene path: {}\r\n", report.scene_path));
        out.push_str(&format!("  Load status: {}\r\n", report.load_status));
        out.push_str(&format!("  Reload counter: {}\r\n", report.reload_count));
        out.push_str(&format!(
            "  Action base dir: {}\r\n",
            report.action_base_dir
        ));
        if let Some(error) = &report.last_error {
            out.push_str(&format!("  Error: {error}\r\n"));
        }
        out.push_str("\r\nMode:\r\n");
        out.push_str(&format!(
            "  Active: {} ({})\r\n",
            report.active_mode_label, report.active_mode_id
        ));
        if !report.active_mode_description.is_empty() {
            out.push_str(&format!(
                "  Description: {}\r\n",
                report.active_mode_description
            ));
        }
        if let Some(profile) = &report.active_mode_scene_profile {
            out.push_str(&format!("  Scene profile: {profile}\r\n"));
        }
        if !report.active_mode_allowed_actions.is_empty() {
            out.push_str(&format!(
                "  Allowed actions: {}\r\n",
                report.active_mode_allowed_actions.join(", ")
            ));
        }
        if let Some(transition) = &report.active_mode_default_transition {
            out.push_str(&format!("  Default transition: {transition}\r\n"));
        }
        if !report.active_mode_lifecycle.is_empty() {
            out.push_str("  Lifecycle:");
            if report.active_mode_lifecycle.enter_status.is_some() {
                out.push_str(" enter");
            }
            if report.active_mode_lifecycle.update_status.is_some() {
                out.push_str(" update");
            }
            if report.active_mode_lifecycle.exit_status.is_some() {
                out.push_str(" exit");
            }
            out.push_str("\r\n");
        }
        if !report.active_mode_input_map.is_empty() {
            out.push_str("  Input map:\r\n");
            for binding in &report.active_mode_input_map {
                let guard = condition_guard_detail(&binding.conditions)
                    .map(|detail| format!(" ({detail})"))
                    .unwrap_or_default();
                out.push_str(&format!(
                    "    {} -> {}{}\r\n",
                    binding.input, binding.action, guard
                ));
            }
        }
        if !report.active_layers.is_empty() {
            out.push_str("  Layers:\r\n");
            for layer in &report.active_layers {
                let label = layer
                    .label
                    .as_ref()
                    .map(|label| format!(" label={label}"))
                    .unwrap_or_default();
                out.push_str(&format!(
                    "    {} state={}{}\r\n",
                    layer.layer_id, layer.state, label
                ));
            }
        }
        if let Some(layer) = &report.last_input_layer {
            out.push_str(&format!("  Last input layer: {layer}\r\n"));
        }
        if let Some(transition) = &report.last_layer_transition {
            out.push_str(&format!(
                "  Last transition: {} {} {} -> {} ({})\r\n",
                transition.layer_id,
                transition.input,
                transition.from_state,
                transition.target_state,
                transition.result
            ));
        }
        if !report.transition_history.is_empty() {
            out.push_str("  History:\r\n");
            for event in &report.transition_history {
                out.push_str(&format!("    {}: {}\r\n", event.kind, event.detail));
            }
        }
        if let Some(mode) = &report.selected_entity_mode {
            out.push_str(&format!("  Selected entity mode: {mode}\r\n"));
        }
        if !report.variables.is_empty() {
            out.push_str("\r\nState:\r\n");
            for variable in &report.variables {
                out.push_str(&format!(
                    "  {}: {}\r\n",
                    variable.key,
                    variable.value.as_debug_string()
                ));
            }
        }
        out.push_str("\r\nAction:\r\n");
        out.push_str(&format!("  Status: {}\r\n", report.status));
        out.push_str(&format!(
            "  Selected choice: {}\r\n",
            report.selected_choice
        ));
        if let Some(label) = &report.selected_choice_label {
            out.push_str(&format!("  Choice label: {label}\r\n"));
        }
        if let Some(kind) = &report.selected_choice_kind {
            out.push_str(&format!("  Choice kind: {kind}\r\n"));
        }
        if let Some(detail) = &report.selected_choice_detail {
            out.push_str(&format!("  Choice detail: {detail}\r\n"));
        }
        if let Some(policy) = &report.selected_choice_policy {
            out.push_str(&format!("  Choice policy: {policy}\r\n"));
        }
        out.push_str(&format!(
            "  Choice enabled: {}\r\n",
            report.selected_choice_enabled
        ));
        if let Some(detail) = &report.selected_choice_guard_detail {
            out.push_str(&format!("  Choice guard: {detail}\r\n"));
        }
        match (&report.pending_action_kind, &report.pending_action_detail) {
            (Some(kind), Some(detail)) => {
                out.push_str(&format!("  Pending action: {kind} {detail}\r\n"));
            }
            (Some(kind), None) => {
                out.push_str(&format!("  Pending action: {kind}\r\n"));
            }
            _ => out.push_str("  Pending action: none\r\n"),
        }
        if let Some(process_state) = &report.process_state {
            out.push_str(&format!("  Process phase: {:?}\r\n", process_state.phase));
            if let Some(entity_id) = &process_state.entity_id {
                out.push_str(&format!("  Process entity: {entity_id}\r\n"));
            }
            if let Some(command) = &process_state.command {
                out.push_str(&format!("  Process command: {command}\r\n"));
            }
            if let Some(exit_code) = process_state.exit_code {
                out.push_str(&format!("  Process exit code: {exit_code}\r\n"));
            }
            if let Some(message) = &process_state.message {
                out.push_str(&format!("  Process message: {message}\r\n"));
            }
        }
        match (
            &report.last_story_state_action,
            &report.last_story_state_path,
        ) {
            (Some(action), Some(path)) => {
                out.push_str(&format!("  Last story state: {action} {path}\r\n"));
            }
            (Some(action), None) => {
                out.push_str(&format!("  Last story state: {action}\r\n"));
            }
            _ => out.push_str("  Last story state: none\r\n"),
        }
        match (
            &report.last_patch_transport,
            report.last_patch_source_pane_id,
        ) {
            (Some(transport), Some(pane_id)) => {
                out.push_str(&format!(
                    "  Last patch: {transport} from pane {pane_id}\r\n"
                ));
            }
            (Some(transport), None) => {
                out.push_str(&format!("  Last patch: {transport}\r\n"));
            }
            (None, _) => out.push_str("  Last patch: none\r\n"),
        }
        out.push_str("\r\nDialogue:\r\n");
        match report.dialogue_index {
            Some(index) => out.push_str(&format!(
                "  Active line: {} of {}\r\n",
                index + 1,
                report.dialogue_line_count
            )),
            None => out.push_str("  Active line: legacy\r\n"),
        }
        out.push_str(&format!(
            "  History entries: {}\r\n",
            report.dialogue_history.len()
        ));
        if !report.rpg.is_empty() {
            out.push_str("\r\nRPG:\r\n");
            out.push_str(&format!(
                "  Inventory items: {}\r\n",
                report.rpg.inventory.len()
            ));
            out.push_str(&format!("  Stats: {}\r\n", report.rpg.stats.len()));
            out.push_str(&format!("  Quests: {}\r\n", report.rpg.quests.len()));
            out.push_str(&format!(
                "  Relationships: {}\r\n",
                report.rpg.relationships.len()
            ));
            for relationship in &report.rpg.relationships {
                out.push_str(&format!(
                    "    {} --{}({})--> {}{}\r\n",
                    relationship_entity_label(&self.scene, &relationship.source_id),
                    relationship.kind,
                    relationship.value,
                    relationship_entity_label(&self.scene, &relationship.target_id),
                    format_relationship_metadata(&relationship.metadata)
                ));
            }
        }
        out.push_str("\r\n");
        out.push_str(&format!(
            "scene={} background={} size={}x{} entities={} choices={}\r\n\r\n",
            report.title,
            report.background,
            report.width,
            report.height,
            report.entity_count,
            report.choice_count
        ));
        out.push_str("Layer order:\r\n");
        out.push_str("  0 background\r\n  1 tile grid\r\n  2 entity sprites\r\n  3 selection/relations\r\n  4 dialogue\r\n  5 debug overlay\r\n\r\n");
        out.push_str("Entities:\r\n");
        for (idx, entity) in self.scene.entities.iter().enumerate() {
            let marker = if idx == self.selected_entity {
                ">"
            } else {
                " "
            };
            out.push_str(&format!(
                "{marker} id={} kind={:?} pos={},{} sprite={} flags={}\r\n",
                entity.id,
                entity.kind,
                entity.position.x,
                entity.position.y,
                entity.sprite,
                entity.state_flags.join(", ")
            ));
        }
        if let Some(entity) = self.selected_entity() {
            out.push_str("\r\nSelected metadata:\r\n");
            out.push_str(&format!(
                "  label: {}\r\n  kind: {:?}\r\n  sprite: {}\r\n  flags: {}\r\n",
                entity.label,
                entity.kind,
                entity.sprite,
                entity.state_flags.join(", ")
            ));
            for (key, value) in &entity.metadata {
                out.push_str(&format!("  {key}: {value}\r\n"));
            }
            if let Some(summary) = format_relationship_summary(&self.scene, &entity.id, 8) {
                out.push_str("\r\nSelected relationships:\r\n");
                out.push_str(&format!("  {summary}\r\n"));
            }
        }
        truncate_to_screen(out, cols, rows)
    }
}
