use super::SceneRuntime;
use crate::runtime_status::wrap_text;
use crate::vn_layout::{
    vn_overlay_layout, vn_overlay_layout_with_overrides, VnDialogueScrollMetrics, VnOverlayLayout,
    VnOverlayRect,
};
use crate::vn_text::{place_vn_overlay_text, truncate_to_screen, wrap_compose_transcript_for_vn};
use crate::{VisualDialogueLine, VisualView};

impl SceneRuntime {
    pub fn vn_dialogue_scroll_metrics(
        &self,
        cols: usize,
        rows: usize,
        dialogue_scroll_offset: usize,
    ) -> VnDialogueScrollMetrics {
        if self.scene.stage.is_empty() {
            return VnDialogueScrollMetrics::default();
        }
        let cols = cols.max(1);
        let rows = rows.max(1);
        let dialogue = self.active_dialogue_line();
        let layout = self.active_vn_overlay_layout(cols, rows, &dialogue);
        let dialogue_width = self.vn_dialogue_text_width(&layout);
        let dialogue_lines = self.render_vn_dialogue_lines(&dialogue, dialogue_width);
        self.vn_dialogue_scroll_metrics_for_line_count(
            dialogue_lines.len(),
            self.vn_dialogue_visible_rows(&layout),
            dialogue_scroll_offset,
        )
    }

    pub fn vn_dialogue_panel_rect(&self, cols: usize, rows: usize) -> Option<VnOverlayRect> {
        if self.scene.stage.is_empty() {
            return None;
        }
        let cols = cols.max(1);
        let rows = rows.max(1);
        let dialogue = self.active_dialogue_line();
        Some(
            self.active_vn_overlay_layout(cols, rows, &dialogue)
                .dialogue_panel,
        )
    }

    fn active_vn_overlay_layout(
        &self,
        cols: usize,
        rows: usize,
        dialogue: &VisualDialogueLine,
    ) -> VnOverlayLayout {
        match &self.vn_layout_debug {
            Some(overrides) => vn_overlay_layout_with_overrides(
                cols,
                rows,
                &dialogue.speaker,
                "Composer",
                overrides,
            ),
            None => vn_overlay_layout(cols, rows, &dialogue.speaker, "Composer"),
        }
    }

    fn vn_dialogue_text_width(&self, layout: &VnOverlayLayout) -> usize {
        const SCROLLBAR_GUTTER_COLS: usize = 2;
        layout
            .dialogue_panel
            .width
            .saturating_sub(layout.dialogue_text_inset_cols * 2)
            .saturating_sub(SCROLLBAR_GUTTER_COLS)
            .max(1)
    }

    fn vn_dialogue_visible_rows(&self, layout: &VnOverlayLayout) -> usize {
        layout
            .dialogue_panel
            .bottom()
            .saturating_sub(layout.dialogue_text_row)
    }

    fn vn_dialogue_scroll_metrics_for_line_count(
        &self,
        total_lines: usize,
        visible_rows: usize,
        dialogue_scroll_offset: usize,
    ) -> VnDialogueScrollMetrics {
        let max_scroll_offset = total_lines.saturating_sub(visible_rows);
        VnDialogueScrollMetrics {
            total_lines,
            visible_rows,
            scroll_offset: dialogue_scroll_offset.min(max_scroll_offset),
            max_scroll_offset,
        }
    }

    pub(super) fn render_staged_scene(
        &self,
        cols: usize,
        rows: usize,
        dialogue_scroll_offset: usize,
        voice_hold_active: bool,
    ) -> String {
        let cols = cols.max(1);
        let rows = rows.max(1);
        if self.view == VisualView::VnLayoutDebugger {
            return self.render_interactive_debugger(cols, rows);
        }
        let dialogue = self.active_dialogue_line();
        let layout = self.active_vn_overlay_layout(cols, rows, &dialogue);

        let mut screen = vec![" ".repeat(cols); rows];
        let dialogue_col = layout
            .dialogue_panel
            .col
            .saturating_add(layout.dialogue_text_inset_cols);
        let dialogue_width = self.vn_dialogue_text_width(&layout);
        let dialogue_lines = self.render_vn_dialogue_lines(&dialogue, dialogue_width);
        let dialogue_rows = self.vn_dialogue_visible_rows(&layout);
        let scroll_metrics = self.vn_dialogue_scroll_metrics_for_line_count(
            dialogue_lines.len(),
            dialogue_rows,
            dialogue_scroll_offset,
        );
        if !dialogue_lines.is_empty() {
            place_vn_overlay_text(
                &mut screen,
                cols,
                layout
                    .dialogue_nameplate_text
                    .row
                    .min(rows.saturating_sub(1)),
                layout.dialogue_nameplate_text.col,
                layout.dialogue_nameplate_text.width,
                &self.vn_dialogue_nameplate(&dialogue),
            );
        }
        let line_count = scroll_metrics.total_lines;
        let start = line_count.saturating_sub(
            scroll_metrics
                .visible_rows
                .saturating_add(scroll_metrics.scroll_offset),
        );
        for (idx, line) in dialogue_lines
            .into_iter()
            .skip(start)
            .take(scroll_metrics.visible_rows)
            .enumerate()
        {
            place_vn_overlay_text(
                &mut screen,
                cols,
                layout.dialogue_text_row.saturating_add(idx),
                dialogue_col,
                dialogue_width,
                &line,
            );
        }
        if !voice_hold_active {
            place_vn_overlay_text(
                &mut screen,
                cols,
                layout.voice_hold_indicator_text.row,
                layout.voice_hold_indicator_text.col,
                layout.voice_hold_indicator_text.width,
                "[off]",
            );
        }
        let frame = screen.join("\r\n") + "\r\n";
        truncate_to_screen(frame, cols, rows)
    }

    fn vn_dialogue_nameplate(&self, dialogue: &VisualDialogueLine) -> String {
        if self.compose_state.history.is_empty() {
            dialogue.speaker.clone()
        } else {
            self.compose_state
                .latest_assistant_speaker()
                .unwrap_or("Codex")
                .to_string()
        }
    }

    fn render_vn_dialogue_lines(
        &self,
        dialogue: &VisualDialogueLine,
        dialogue_width: usize,
    ) -> Vec<String> {
        let transcript = self.recent_compose_transcript_lines(dialogue_width);
        if transcript.is_empty() {
            if self.scene.stage.is_empty() {
                wrap_text(&dialogue.text, dialogue_width)
            } else {
                Vec::new()
            }
        } else {
            transcript
        }
    }

    fn recent_compose_transcript_lines(&self, dialogue_width: usize) -> Vec<String> {
        if self.compose_state.history.is_empty() {
            return Vec::new();
        }
        wrap_compose_transcript_for_vn(&self.compose_state.history, dialogue_width)
    }
}
