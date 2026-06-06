use super::visual_voice_debug::SceneVoiceDebugState;
use gameterm_visual::{SceneRuntime, VisualInput, VnDialogueScrollMetrics};
use termwiz::input::MouseButtons;
use termwiz::terminal::ScreenSize;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct SceneDialogueScrollback {
    pub(super) offset: usize,
    pub(super) voice_hold_active: bool,
    pub(super) voice_debug: SceneVoiceDebugState,
}

impl SceneDialogueScrollback {
    pub(super) fn reset_to_bottom(&mut self) {
        self.offset = 0;
    }

    fn scroll_up(&mut self, max_offset: usize) {
        self.offset = self.offset.saturating_add(1).min(max_offset);
    }

    fn scroll_up_by(&mut self, lines: usize, max_offset: usize) {
        self.offset = self.offset.saturating_add(lines).min(max_offset);
    }

    fn scroll_down(&mut self) {
        self.offset = self.offset.saturating_sub(1);
    }

    fn scroll_down_by(&mut self, lines: usize) {
        self.offset = self.offset.saturating_sub(lines);
    }

    pub(super) fn clamp(&mut self, max_offset: usize) {
        self.offset = self.offset.min(max_offset);
    }
}

pub(super) fn handle_dialogue_scroll_wheel(
    runtime: &SceneRuntime,
    scroll: &mut SceneDialogueScrollback,
    cols: usize,
    rows: usize,
    x: u16,
    y: u16,
    mouse_buttons: MouseButtons,
) -> bool {
    let Some(panel) = runtime.vn_dialogue_panel_rect(cols, rows) else {
        return false;
    };
    let mouse_col = x.saturating_sub(1) as usize;
    let mouse_row = y.saturating_sub(1) as usize;
    if mouse_col < panel.col
        || mouse_col >= panel.right()
        || mouse_row < panel.row
        || mouse_row >= panel.bottom()
    {
        return false;
    }

    let metrics = runtime.vn_dialogue_scroll_metrics(cols, rows, scroll.offset);
    apply_dialogue_scroll_wheel(scroll, metrics, mouse_buttons);
    true
}

pub(super) fn handle_dialogue_scroll_key(
    runtime: &SceneRuntime,
    scroll: &mut SceneDialogueScrollback,
    input: VisualInput,
    size: ScreenSize,
) -> bool {
    let metrics = runtime.vn_dialogue_scroll_metrics(size.cols, size.rows, scroll.offset);
    apply_dialogue_scroll_key(scroll, metrics, input)
}

pub(super) fn apply_dialogue_scroll_key(
    scroll: &mut SceneDialogueScrollback,
    metrics: VnDialogueScrollMetrics,
    input: VisualInput,
) -> bool {
    let page_lines = metrics.visible_rows.saturating_sub(1).max(1);
    match input {
        VisualInput::ScrollDialogueUp => {
            scroll.scroll_up_by(page_lines, metrics.max_scroll_offset);
            true
        }
        VisualInput::ScrollDialogueDown => {
            scroll.scroll_down_by(page_lines);
            scroll.clamp(metrics.max_scroll_offset);
            true
        }
        _ => false,
    }
}

pub(super) fn apply_dialogue_scroll_wheel(
    scroll: &mut SceneDialogueScrollback,
    metrics: VnDialogueScrollMetrics,
    mouse_buttons: MouseButtons,
) {
    if mouse_buttons.contains(MouseButtons::WHEEL_POSITIVE) {
        scroll.scroll_up(metrics.max_scroll_offset);
    } else {
        scroll.scroll_down();
    }
    scroll.clamp(metrics.max_scroll_offset);
}
