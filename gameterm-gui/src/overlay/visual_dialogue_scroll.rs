use super::visual_voice_debug::SceneVoiceDebugState;
use gameterm_visual::{SceneRuntime, VisualInput, VnDialogueScrollMetrics};
use termwiz::input::MouseButtons;
use termwiz::terminal::ScreenSize;

#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct SceneDialogueScrollback {
    pub(super) offset: usize,
    pub(super) voice_hold_active: bool,
    pub(super) voice_hold_disarmed: bool,
    pub(super) voice_debug: SceneVoiceDebugState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SceneVoiceHoldTransition {
    None,
    Start,
    Stop,
}

impl SceneDialogueScrollback {
    pub(super) fn reset_to_bottom(&mut self) {
        self.offset = 0;
    }

    pub(super) fn apply_voice_hold_level(&mut self, hold_active: bool) -> SceneVoiceHoldTransition {
        if hold_active {
            if self.voice_hold_active || self.voice_hold_disarmed {
                return SceneVoiceHoldTransition::None;
            }
            self.voice_hold_active = true;
            self.voice_hold_disarmed = true;
            return SceneVoiceHoldTransition::Start;
        }

        self.voice_hold_disarmed = false;
        if self.voice_hold_active {
            self.voice_hold_active = false;
            return SceneVoiceHoldTransition::Stop;
        }
        SceneVoiceHoldTransition::None
    }

    pub(super) fn mark_voice_hold_result_finished(&mut self) {
        self.voice_hold_active = false;
    }

    pub(super) fn cancel_voice_hold(&mut self) {
        self.voice_hold_active = false;
        self.voice_hold_disarmed = false;
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

#[cfg(test)]
mod tests {
    use super::{SceneDialogueScrollback, SceneVoiceHoldTransition};

    #[test]
    fn voice_hold_starts_once_per_physical_hold() {
        let mut scroll = SceneDialogueScrollback::default();

        assert_eq!(
            scroll.apply_voice_hold_level(true),
            SceneVoiceHoldTransition::Start
        );
        assert!(scroll.voice_hold_active);

        assert_eq!(
            scroll.apply_voice_hold_level(true),
            SceneVoiceHoldTransition::None
        );
        assert!(scroll.voice_hold_active);
    }

    #[test]
    fn voice_hold_result_does_not_restart_until_release() {
        let mut scroll = SceneDialogueScrollback::default();

        assert_eq!(
            scroll.apply_voice_hold_level(true),
            SceneVoiceHoldTransition::Start
        );
        scroll.mark_voice_hold_result_finished();
        assert!(!scroll.voice_hold_active);

        assert_eq!(
            scroll.apply_voice_hold_level(true),
            SceneVoiceHoldTransition::None
        );
        assert!(!scroll.voice_hold_active);

        assert_eq!(
            scroll.apply_voice_hold_level(false),
            SceneVoiceHoldTransition::None
        );
        assert_eq!(
            scroll.apply_voice_hold_level(true),
            SceneVoiceHoldTransition::Start
        );
        assert!(scroll.voice_hold_active);
    }

    #[test]
    fn voice_hold_release_stops_and_rearms() {
        let mut scroll = SceneDialogueScrollback::default();

        assert_eq!(
            scroll.apply_voice_hold_level(true),
            SceneVoiceHoldTransition::Start
        );
        assert_eq!(
            scroll.apply_voice_hold_level(false),
            SceneVoiceHoldTransition::Stop
        );
        assert!(!scroll.voice_hold_active);

        assert_eq!(
            scroll.apply_voice_hold_level(true),
            SceneVoiceHoldTransition::Start
        );
    }

    #[test]
    fn voice_hold_cancel_clears_active_and_disarmed_state() {
        let mut scroll = SceneDialogueScrollback::default();

        assert_eq!(
            scroll.apply_voice_hold_level(true),
            SceneVoiceHoldTransition::Start
        );
        scroll.cancel_voice_hold();
        assert!(!scroll.voice_hold_active);
        assert_eq!(
            scroll.apply_voice_hold_level(true),
            SceneVoiceHoldTransition::Start
        );
    }
}
