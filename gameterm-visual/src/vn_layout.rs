use serde::{Deserialize, Serialize};

pub const VN_OVERLAY_FULLSCREEN_MIN_ROWS: usize = 40;
pub const VN_OVERLAY_SIDE_MARGIN_RATIO: f32 = 0.125;
pub const VN_OVERLAY_COMPOSER_SIDE_MARGIN_RATIO: f32 = 0.025;
pub const VN_OVERLAY_DIALOGUE_TOP_RATIO: f32 = 0.075;
pub const VN_OVERLAY_DIALOGUE_BOTTOM_RATIO: f32 = 0.66;
pub const VN_OVERLAY_DIALOGUE_TEXT_INSET_COLS: usize = 3;
pub const VN_OVERLAY_COMPOSER_TEXT_INSET_COLS: usize = 1;
pub const VN_OVERLAY_DIALOGUE_NAMEPLATE_INSET_COLS: usize = 2;
pub const VN_OVERLAY_COMPOSER_NAMEPLATE_INSET_COLS: usize = 2;
pub const VN_OVERLAY_NAMEPLATE_TEXT_INSET_COLS: usize = 3;
pub const VN_OVERLAY_NAMEPLATE_TEXT_INSET_ROWS: usize =
    VN_OVERLAY_NAMEPLATE_HEIGHT_ROWS.saturating_sub(1);
pub const VN_OVERLAY_DIALOGUE_NAMEPLATE_TEXT_INSET_COLS: usize =
    VN_OVERLAY_NAMEPLATE_TEXT_INSET_COLS;
pub const VN_OVERLAY_COMPOSER_NAMEPLATE_TEXT_INSET_COLS: usize =
    VN_OVERLAY_NAMEPLATE_TEXT_INSET_COLS;
pub const VN_OVERLAY_DIALOGUE_NAMEPLATE_TEXT_INSET_ROWS: usize =
    VN_OVERLAY_NAMEPLATE_TEXT_INSET_ROWS;
pub const VN_OVERLAY_COMPOSER_NAMEPLATE_TEXT_INSET_ROWS: usize =
    VN_OVERLAY_NAMEPLATE_TEXT_INSET_ROWS;
pub const VN_OVERLAY_DIALOGUE_TEXT_INSET_ROWS: usize = 2;
pub const VN_OVERLAY_COMPOSER_TEXT_INSET_ROWS: usize = 2;
pub const VN_OVERLAY_NAMEPLATE_OFFSET_ROWS: usize = 0;
pub const VN_OVERLAY_NAMEPLATE_HEIGHT_ROWS: usize = 3;
pub const VN_OVERLAY_DIALOGUE_NAMEPLATE_HEIGHT_ROWS: usize = VN_OVERLAY_NAMEPLATE_HEIGHT_ROWS;
pub const VN_OVERLAY_COMPOSER_NAMEPLATE_HEIGHT_ROWS: usize = VN_OVERLAY_NAMEPLATE_HEIGHT_ROWS;
pub const VN_OVERLAY_PANEL_OPACITY: f32 = 0.4627;
pub const VN_OVERLAY_NAMEPLATE_OPACITY: f32 = 0.58;
pub const VN_OVERLAY_VOICE_INDICATOR_WIDTH_COLS: usize = 7;
pub const VN_OVERLAY_VOICE_INDICATOR_HEIGHT_ROWS: usize = 2;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct VnOverlayDebugOverrides {
    pub dialogue_margin_ratio: f32,
    pub composer_margin_ratio: f32,
    pub dialogue_top_ratio: f32,
    pub dialogue_bottom_ratio: f32,
    pub composer_height_rows: usize,
    pub dialogue_nameplate_height_rows: usize,
    pub composer_nameplate_height_rows: usize,
    pub dialogue_nameplate_inset_cols: usize,
    pub composer_nameplate_inset_cols: usize,
    pub dialogue_nameplate_text_inset_cols: usize,
    pub composer_nameplate_text_inset_cols: usize,
    pub dialogue_nameplate_text_inset_rows: usize,
    pub composer_nameplate_text_inset_rows: usize,
    pub dialogue_text_inset_cols: usize,
    pub composer_text_inset_cols: usize,
    pub dialogue_text_inset_rows: usize,
    pub composer_text_inset_rows: usize,
    pub dialogue_panel_opacity: f32,
    pub composer_panel_opacity: f32,
    pub dialogue_nameplate_opacity: f32,
    pub composer_nameplate_opacity: f32,
    pub selected_param: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub editing_buffer: Option<String>,
}

impl Default for VnOverlayDebugOverrides {
    fn default() -> Self {
        Self {
            dialogue_margin_ratio: VN_OVERLAY_SIDE_MARGIN_RATIO,
            composer_margin_ratio: VN_OVERLAY_COMPOSER_SIDE_MARGIN_RATIO,
            dialogue_top_ratio: VN_OVERLAY_DIALOGUE_TOP_RATIO,
            dialogue_bottom_ratio: VN_OVERLAY_DIALOGUE_BOTTOM_RATIO,
            composer_height_rows: 7,
            dialogue_nameplate_height_rows: VN_OVERLAY_DIALOGUE_NAMEPLATE_HEIGHT_ROWS,
            composer_nameplate_height_rows: VN_OVERLAY_COMPOSER_NAMEPLATE_HEIGHT_ROWS,
            dialogue_nameplate_inset_cols: VN_OVERLAY_DIALOGUE_NAMEPLATE_INSET_COLS,
            composer_nameplate_inset_cols: VN_OVERLAY_COMPOSER_NAMEPLATE_INSET_COLS,
            dialogue_nameplate_text_inset_cols: VN_OVERLAY_DIALOGUE_NAMEPLATE_TEXT_INSET_COLS,
            composer_nameplate_text_inset_cols: VN_OVERLAY_COMPOSER_NAMEPLATE_TEXT_INSET_COLS,
            dialogue_nameplate_text_inset_rows: VN_OVERLAY_DIALOGUE_NAMEPLATE_TEXT_INSET_ROWS,
            composer_nameplate_text_inset_rows: VN_OVERLAY_COMPOSER_NAMEPLATE_TEXT_INSET_ROWS,
            dialogue_text_inset_cols: VN_OVERLAY_DIALOGUE_TEXT_INSET_COLS,
            composer_text_inset_cols: VN_OVERLAY_COMPOSER_TEXT_INSET_COLS,
            dialogue_text_inset_rows: VN_OVERLAY_DIALOGUE_TEXT_INSET_ROWS,
            composer_text_inset_rows: VN_OVERLAY_COMPOSER_TEXT_INSET_ROWS,
            dialogue_panel_opacity: VN_OVERLAY_PANEL_OPACITY,
            composer_panel_opacity: VN_OVERLAY_PANEL_OPACITY,
            dialogue_nameplate_opacity: VN_OVERLAY_NAMEPLATE_OPACITY,
            composer_nameplate_opacity: VN_OVERLAY_NAMEPLATE_OPACITY,
            selected_param: 0,
            editing_buffer: None,
        }
    }
}

impl VnOverlayDebugOverrides {
    pub const PARAM_COUNT: usize = 21;

    pub fn select_next(&mut self) {
        self.selected_param = (self.selected_param + 1) % Self::PARAM_COUNT;
    }

    pub fn select_prev(&mut self) {
        self.selected_param = if self.selected_param == 0 {
            Self::PARAM_COUNT - 1
        } else {
            self.selected_param - 1
        };
    }

    pub fn adjust(&mut self, delta: i32) {
        match self.selected_param {
            0 => {
                self.dialogue_margin_ratio =
                    (self.dialogue_margin_ratio + delta as f32 * 0.005).clamp(0.0, 0.45)
            }
            1 => {
                self.composer_margin_ratio =
                    (self.composer_margin_ratio + delta as f32 * 0.005).clamp(0.0, 0.20)
            }
            2 => {
                self.dialogue_top_ratio =
                    (self.dialogue_top_ratio + delta as f32 * 0.005).clamp(0.0, 0.30)
            }
            3 => {
                self.dialogue_bottom_ratio =
                    (self.dialogue_bottom_ratio + delta as f32 * 0.010).clamp(0.30, 0.99)
            }
            4 => {
                if delta > 0 {
                    self.composer_height_rows = self.composer_height_rows.saturating_add(1).min(20);
                } else {
                    self.composer_height_rows = self.composer_height_rows.saturating_sub(1).max(1);
                }
            }
            5 => {
                adjust_usize(&mut self.dialogue_nameplate_height_rows, delta, 1, 8);
            }
            6 => {
                adjust_usize(&mut self.composer_nameplate_height_rows, delta, 1, 8);
            }
            7 => {
                adjust_usize(&mut self.dialogue_nameplate_inset_cols, delta, 0, 40);
            }
            8 => {
                adjust_usize(&mut self.composer_nameplate_inset_cols, delta, 0, 40);
            }
            9 => {
                adjust_usize(&mut self.dialogue_nameplate_text_inset_cols, delta, 0, 40);
            }
            10 => {
                adjust_usize(&mut self.composer_nameplate_text_inset_cols, delta, 0, 40);
            }
            11 => {
                adjust_usize(&mut self.dialogue_nameplate_text_inset_rows, delta, 0, 8);
            }
            12 => {
                adjust_usize(&mut self.composer_nameplate_text_inset_rows, delta, 0, 8);
            }
            13 => {
                adjust_usize(&mut self.dialogue_text_inset_cols, delta, 0, 40);
            }
            14 => {
                adjust_usize(&mut self.composer_text_inset_cols, delta, 0, 40);
            }
            15 => {
                adjust_usize(&mut self.dialogue_text_inset_rows, delta, 0, 12);
            }
            16 => {
                adjust_usize(&mut self.composer_text_inset_rows, delta, 0, 12);
            }
            17 => {
                self.dialogue_panel_opacity = adjust_opacity(self.dialogue_panel_opacity, delta);
            }
            18 => {
                self.composer_panel_opacity = adjust_opacity(self.composer_panel_opacity, delta);
            }
            19 => {
                self.dialogue_nameplate_opacity =
                    adjust_opacity(self.dialogue_nameplate_opacity, delta);
            }
            20 => {
                self.composer_nameplate_opacity =
                    adjust_opacity(self.composer_nameplate_opacity, delta);
            }
            _ => {}
        }
    }

    pub fn param_label(idx: usize) -> &'static str {
        match idx {
            0 => "dialogue_margin_ratio",
            1 => "composer_margin_ratio",
            2 => "dialogue_top_ratio",
            3 => "dialogue_bottom_ratio",
            4 => "composer_height_rows",
            5 => "dialogue_nameplate_height_rows",
            6 => "composer_nameplate_height_rows",
            7 => "dialogue_nameplate_inset_cols",
            8 => "composer_nameplate_inset_cols",
            9 => "dialogue_nameplate_text_inset_cols",
            10 => "composer_nameplate_text_inset_cols",
            11 => "dialogue_nameplate_text_inset_rows",
            12 => "composer_nameplate_text_inset_rows",
            13 => "dialogue_text_inset_cols",
            14 => "composer_text_inset_cols",
            15 => "dialogue_text_inset_rows",
            16 => "composer_text_inset_rows",
            17 => "dialogue_panel_opacity",
            18 => "composer_panel_opacity",
            19 => "dialogue_nameplate_opacity",
            20 => "composer_nameplate_opacity",
            _ => "?",
        }
    }

    pub fn param_desc(idx: usize) -> &'static str {
        match idx {
            0 => "dialogue side margins",
            1 => "composer side margins",
            2 => "dialogue panel top",
            3 => "dialogue panel bottom",
            4 => "composer height (fullscreen)",
            5 => "dialogue nameplate height",
            6 => "composer nameplate height",
            7 => "dialogue nameplate left inset",
            8 => "composer nameplate left inset",
            9 => "dialogue nameplate label left inset",
            10 => "composer nameplate label left inset",
            11 => "dialogue nameplate label top inset",
            12 => "composer nameplate label top inset",
            13 => "dialogue text left inset",
            14 => "composer text left inset",
            15 => "dialogue text top inset",
            16 => "composer text top inset",
            17 => "dialogue box opacity",
            18 => "composer box opacity",
            19 => "dialogue nameplate opacity",
            20 => "composer nameplate opacity",
            _ => "",
        }
    }

    pub fn param_value_str(&self, idx: usize) -> String {
        match idx {
            0 => format!("{:.3}", self.dialogue_margin_ratio),
            1 => format!("{:.3}", self.composer_margin_ratio),
            2 => format!("{:.3}", self.dialogue_top_ratio),
            3 => format!("{:.3}", self.dialogue_bottom_ratio),
            4 => self.composer_height_rows.to_string(),
            5 => self.dialogue_nameplate_height_rows.to_string(),
            6 => self.composer_nameplate_height_rows.to_string(),
            7 => self.dialogue_nameplate_inset_cols.to_string(),
            8 => self.composer_nameplate_inset_cols.to_string(),
            9 => self.dialogue_nameplate_text_inset_cols.to_string(),
            10 => self.composer_nameplate_text_inset_cols.to_string(),
            11 => self.dialogue_nameplate_text_inset_rows.to_string(),
            12 => self.composer_nameplate_text_inset_rows.to_string(),
            13 => self.dialogue_text_inset_cols.to_string(),
            14 => self.composer_text_inset_cols.to_string(),
            15 => self.dialogue_text_inset_rows.to_string(),
            16 => self.composer_text_inset_rows.to_string(),
            17 => format!("{:.3}", self.dialogue_panel_opacity),
            18 => format!("{:.3}", self.composer_panel_opacity),
            19 => format!("{:.3}", self.dialogue_nameplate_opacity),
            20 => format!("{:.3}", self.composer_nameplate_opacity),
            _ => String::new(),
        }
    }

    pub fn begin_edit(&mut self) {
        self.editing_buffer = Some(self.param_value_str(self.selected_param));
    }

    pub fn cancel_edit(&mut self) {
        self.editing_buffer = None;
    }

    pub fn push_char(&mut self, c: char) {
        if let Some(ref mut buf) = self.editing_buffer {
            if c.is_ascii_digit()
                || (c == '.' && !buf.contains('.'))
                || (c == '-' && buf.is_empty())
            {
                buf.push(c);
            }
        }
    }

    pub fn pop_char(&mut self) {
        if let Some(ref mut buf) = self.editing_buffer {
            buf.pop();
        }
    }

    pub fn commit_edit(&mut self) {
        let buf = match self.editing_buffer.take() {
            Some(b) => b,
            None => return,
        };
        match self.selected_param {
            0 => {
                if let Ok(v) = buf.parse::<f32>() {
                    self.dialogue_margin_ratio = v.clamp(0.0, 0.45);
                }
            }
            1 => {
                if let Ok(v) = buf.parse::<f32>() {
                    self.composer_margin_ratio = v.clamp(0.0, 0.20);
                }
            }
            2 => {
                if let Ok(v) = buf.parse::<f32>() {
                    self.dialogue_top_ratio = v.clamp(0.0, 0.30);
                }
            }
            3 => {
                if let Ok(v) = buf.parse::<f32>() {
                    self.dialogue_bottom_ratio = v.clamp(0.30, 0.99);
                }
            }
            4 => {
                if let Ok(v) = buf.parse::<usize>() {
                    self.composer_height_rows = v.clamp(1, 20);
                }
            }
            5 => {
                if let Ok(v) = buf.parse::<usize>() {
                    self.dialogue_nameplate_height_rows = v.clamp(1, 8);
                }
            }
            6 => {
                if let Ok(v) = buf.parse::<usize>() {
                    self.composer_nameplate_height_rows = v.clamp(1, 8);
                }
            }
            7 => {
                if let Ok(v) = buf.parse::<usize>() {
                    self.dialogue_nameplate_inset_cols = v.clamp(0, 40);
                }
            }
            8 => {
                if let Ok(v) = buf.parse::<usize>() {
                    self.composer_nameplate_inset_cols = v.clamp(0, 40);
                }
            }
            9 => {
                if let Ok(v) = buf.parse::<usize>() {
                    self.dialogue_nameplate_text_inset_cols = v.clamp(0, 40);
                }
            }
            10 => {
                if let Ok(v) = buf.parse::<usize>() {
                    self.composer_nameplate_text_inset_cols = v.clamp(0, 40);
                }
            }
            11 => {
                if let Ok(v) = buf.parse::<usize>() {
                    self.dialogue_nameplate_text_inset_rows = v.clamp(0, 8);
                }
            }
            12 => {
                if let Ok(v) = buf.parse::<usize>() {
                    self.composer_nameplate_text_inset_rows = v.clamp(0, 8);
                }
            }
            13 => {
                if let Ok(v) = buf.parse::<usize>() {
                    self.dialogue_text_inset_cols = v.clamp(0, 40);
                }
            }
            14 => {
                if let Ok(v) = buf.parse::<usize>() {
                    self.composer_text_inset_cols = v.clamp(0, 40);
                }
            }
            15 => {
                if let Ok(v) = buf.parse::<usize>() {
                    self.dialogue_text_inset_rows = v.clamp(0, 12);
                }
            }
            16 => {
                if let Ok(v) = buf.parse::<usize>() {
                    self.composer_text_inset_rows = v.clamp(0, 12);
                }
            }
            17 => {
                if let Ok(v) = buf.parse::<f32>() {
                    self.dialogue_panel_opacity = clamp_opacity(v);
                }
            }
            18 => {
                if let Ok(v) = buf.parse::<f32>() {
                    self.composer_panel_opacity = clamp_opacity(v);
                }
            }
            19 => {
                if let Ok(v) = buf.parse::<f32>() {
                    self.dialogue_nameplate_opacity = clamp_opacity(v);
                }
            }
            20 => {
                if let Ok(v) = buf.parse::<f32>() {
                    self.composer_nameplate_opacity = clamp_opacity(v);
                }
            }
            _ => {}
        }
    }
}

fn adjust_opacity(value: f32, delta: i32) -> f32 {
    clamp_opacity(value + delta as f32 * 0.025)
}

fn clamp_opacity(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
}

fn adjust_usize(value: &mut usize, delta: i32, min: usize, max: usize) {
    if delta > 0 {
        *value = value.saturating_add(1).min(max);
    } else {
        *value = value.saturating_sub(1).max(min);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct VnOverlayRect {
    pub col: usize,
    pub row: usize,
    pub width: usize,
    pub height: usize,
}

impl VnOverlayRect {
    pub fn right(self) -> usize {
        self.col.saturating_add(self.width)
    }

    pub fn bottom(self) -> usize {
        self.row.saturating_add(self.height)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VnOverlayLayout {
    pub fullscreen: bool,
    pub dialogue_panel: VnOverlayRect,
    pub dialogue_nameplate: VnOverlayRect,
    pub dialogue_nameplate_text: VnOverlayRect,
    pub composer_panel: Option<VnOverlayRect>,
    pub composer_nameplate: Option<VnOverlayRect>,
    pub composer_nameplate_text: Option<VnOverlayRect>,
    pub dialogue_text_inset_cols: usize,
    pub composer_text_inset_cols: usize,
    pub dialogue_text_row: usize,
    pub composer_text_row: Option<usize>,
    pub voice_hold_indicator: VnOverlayRect,
    pub voice_hold_indicator_text: VnOverlayRect,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VnDialogueScrollMetrics {
    pub total_lines: usize,
    pub visible_rows: usize,
    pub scroll_offset: usize,
    pub max_scroll_offset: usize,
}

pub fn vn_overlay_layout(
    cols: usize,
    rows: usize,
    dialogue_label: &str,
    composer_label: &str,
) -> VnOverlayLayout {
    vn_overlay_layout_inner(
        cols,
        rows,
        dialogue_label,
        composer_label,
        VN_OVERLAY_SIDE_MARGIN_RATIO,
        VN_OVERLAY_COMPOSER_SIDE_MARGIN_RATIO,
        VN_OVERLAY_DIALOGUE_TOP_RATIO,
        VN_OVERLAY_DIALOGUE_BOTTOM_RATIO,
        7,
        VN_OVERLAY_DIALOGUE_NAMEPLATE_HEIGHT_ROWS,
        VN_OVERLAY_COMPOSER_NAMEPLATE_HEIGHT_ROWS,
        VN_OVERLAY_DIALOGUE_NAMEPLATE_INSET_COLS,
        VN_OVERLAY_COMPOSER_NAMEPLATE_INSET_COLS,
        VN_OVERLAY_DIALOGUE_NAMEPLATE_TEXT_INSET_COLS,
        VN_OVERLAY_COMPOSER_NAMEPLATE_TEXT_INSET_COLS,
        VN_OVERLAY_DIALOGUE_NAMEPLATE_TEXT_INSET_ROWS,
        VN_OVERLAY_COMPOSER_NAMEPLATE_TEXT_INSET_ROWS,
        VN_OVERLAY_DIALOGUE_TEXT_INSET_COLS,
        VN_OVERLAY_COMPOSER_TEXT_INSET_COLS,
        VN_OVERLAY_DIALOGUE_TEXT_INSET_ROWS,
        VN_OVERLAY_COMPOSER_TEXT_INSET_ROWS,
    )
}

pub fn vn_overlay_layout_with_overrides(
    cols: usize,
    rows: usize,
    dialogue_label: &str,
    composer_label: &str,
    overrides: &VnOverlayDebugOverrides,
) -> VnOverlayLayout {
    vn_overlay_layout_inner(
        cols,
        rows,
        dialogue_label,
        composer_label,
        overrides.dialogue_margin_ratio,
        overrides.composer_margin_ratio,
        overrides.dialogue_top_ratio,
        overrides.dialogue_bottom_ratio,
        overrides.composer_height_rows,
        overrides.dialogue_nameplate_height_rows,
        overrides.composer_nameplate_height_rows,
        overrides.dialogue_nameplate_inset_cols,
        overrides.composer_nameplate_inset_cols,
        overrides.dialogue_nameplate_text_inset_cols,
        overrides.composer_nameplate_text_inset_cols,
        overrides.dialogue_nameplate_text_inset_rows,
        overrides.composer_nameplate_text_inset_rows,
        overrides.dialogue_text_inset_cols,
        overrides.composer_text_inset_cols,
        overrides.dialogue_text_inset_rows,
        overrides.composer_text_inset_rows,
    )
}

#[allow(clippy::too_many_arguments)]
fn vn_overlay_layout_inner(
    cols: usize,
    rows: usize,
    dialogue_label: &str,
    composer_label: &str,
    dialogue_margin_ratio: f32,
    composer_margin_ratio: f32,
    dialogue_top_ratio: f32,
    dialogue_bottom_ratio: f32,
    fullscreen_composer_height: usize,
    dialogue_nameplate_height_rows: usize,
    composer_nameplate_height_rows: usize,
    dialogue_nameplate_inset_cols: usize,
    composer_nameplate_inset_cols: usize,
    dialogue_nameplate_text_inset_cols: usize,
    composer_nameplate_text_inset_cols: usize,
    dialogue_nameplate_text_inset_rows: usize,
    composer_nameplate_text_inset_rows: usize,
    dialogue_text_inset_cols: usize,
    composer_text_inset_cols: usize,
    dialogue_text_inset_rows: usize,
    composer_text_inset_rows: usize,
) -> VnOverlayLayout {
    let cols = cols.max(1);
    let rows = rows.max(1);
    let fullscreen = rows >= VN_OVERLAY_FULLSCREEN_MIN_ROWS;
    let margin = if fullscreen {
        ((cols as f32) * dialogue_margin_ratio).round() as usize
    } else {
        3
    }
    .min(cols.saturating_sub(1));
    let panel_width = cols.saturating_sub(margin * 2).max(1);
    let composer_margin = if fullscreen {
        ((cols as f32) * composer_margin_ratio).round() as usize
    } else {
        2
    }
    .min(cols.saturating_sub(1));
    let composer_width = cols.saturating_sub(composer_margin * 2).max(1);
    let composer_height = if rows >= 40 {
        fullscreen_composer_height
    } else if rows >= 18 {
        4
    } else if rows >= 10 {
        2
    } else {
        0
    };
    let composer_gap = usize::from(composer_height > 0);
    let composer_panel = if composer_height > 0 {
        Some(VnOverlayRect {
            col: composer_margin,
            row: rows.saturating_sub(composer_height + 2),
            width: composer_width,
            height: composer_height,
        })
    } else {
        None
    };

    let min_panel_top = dialogue_nameplate_height_rows
        .min(rows.saturating_sub(1))
        .saturating_add(VN_OVERLAY_NAMEPLATE_OFFSET_ROWS)
        .min(rows.saturating_sub(1));
    let dialogue_panel = if fullscreen {
        let top = ((rows as f32) * dialogue_top_ratio).round() as usize;
        let top = top.max(min_panel_top);
        let bottom = ((rows as f32) * dialogue_bottom_ratio).round() as usize;
        let reserved_bottom = composer_panel
            .map(|panel| panel.row.saturating_sub(composer_gap))
            .unwrap_or(rows);
        let bottom = bottom.min(reserved_bottom).max(top + 4);
        let row = top.min(rows.saturating_sub(1));
        VnOverlayRect {
            col: margin,
            row,
            width: panel_width,
            height: bottom.saturating_sub(row).max(4),
        }
    } else {
        let height = if rows >= 18 { 7 } else { 4 };
        let bottom = composer_panel
            .map(|panel| panel.row.saturating_sub(composer_gap))
            .unwrap_or(rows.saturating_sub(1))
            .max(1);
        let row = bottom.saturating_sub(height);
        VnOverlayRect {
            col: margin,
            row,
            width: panel_width,
            height: height.min(rows.saturating_sub(row).max(1)),
        }
    };

    let dialogue_nameplate = vn_overlay_nameplate_rect(
        &dialogue_panel,
        dialogue_label,
        dialogue_nameplate_height_rows,
        dialogue_nameplate_inset_cols,
    );
    let composer_nameplate = composer_panel.map(|panel| {
        vn_overlay_nameplate_rect(
            &panel,
            composer_label,
            composer_nameplate_height_rows,
            composer_nameplate_inset_cols,
        )
    });
    let dialogue_nameplate_text = vn_overlay_nameplate_text_rect(
        &dialogue_nameplate,
        dialogue_nameplate_text_inset_cols,
        dialogue_nameplate_text_inset_rows,
    );
    let composer_nameplate_text = composer_nameplate.map(|nameplate| {
        vn_overlay_nameplate_text_rect(
            &nameplate,
            composer_nameplate_text_inset_cols,
            composer_nameplate_text_inset_rows,
        )
    });
    let voice_hold_indicator = vn_overlay_voice_hold_indicator_rect(cols, rows);
    let voice_hold_indicator_text = VnOverlayRect {
        col: voice_hold_indicator
            .col
            .saturating_add(1)
            .min(cols.saturating_sub(1)),
        row: voice_hold_indicator
            .row
            .saturating_add(voice_hold_indicator.height.saturating_sub(1))
            .min(rows.saturating_sub(1)),
        width: voice_hold_indicator.width.saturating_sub(2).max(1),
        height: 1,
    };

    VnOverlayLayout {
        fullscreen,
        dialogue_text_row: dialogue_panel
            .row
            .saturating_add(dialogue_text_inset_rows)
            .min(dialogue_panel.bottom().saturating_sub(1)),
        composer_text_row: composer_panel.map(|panel| {
            panel
                .row
                .saturating_add(composer_text_inset_rows)
                .min(panel.bottom().saturating_sub(1))
        }),
        dialogue_panel,
        dialogue_nameplate,
        dialogue_nameplate_text,
        composer_panel,
        composer_nameplate,
        composer_nameplate_text,
        dialogue_text_inset_cols,
        composer_text_inset_cols,
        voice_hold_indicator,
        voice_hold_indicator_text,
    }
}

pub fn vn_overlay_side_margin(cols: usize, rows: usize) -> usize {
    let margin = if rows >= VN_OVERLAY_FULLSCREEN_MIN_ROWS {
        ((cols as f32) * VN_OVERLAY_SIDE_MARGIN_RATIO).round() as usize
    } else {
        3
    };
    margin.min(cols.saturating_sub(1))
}

fn vn_overlay_nameplate_rect(
    panel: &VnOverlayRect,
    label: &str,
    nameplate_height_rows: usize,
    nameplate_inset_cols: usize,
) -> VnOverlayRect {
    let inset = nameplate_inset_cols.min(panel.width.saturating_sub(1));
    let available_width = panel.width.saturating_sub(inset).max(1);
    let label_width = label.chars().count().max(1);
    let width = (label_width + 6).clamp(1, available_width.min(32));
    let height = nameplate_height_rows.min(panel.height.max(1));
    let row_offset = height.saturating_add(VN_OVERLAY_NAMEPLATE_OFFSET_ROWS);
    VnOverlayRect {
        col: panel.col.saturating_add(inset),
        row: panel.row.saturating_sub(row_offset),
        width,
        height,
    }
}

fn vn_overlay_voice_hold_indicator_rect(cols: usize, rows: usize) -> VnOverlayRect {
    let cols = cols.max(1);
    let rows = rows.max(1);
    let width = VN_OVERLAY_VOICE_INDICATOR_WIDTH_COLS.min(cols).max(1);
    let height = VN_OVERLAY_VOICE_INDICATOR_HEIGHT_ROWS.min(rows).max(1);
    let bottom_margin = usize::from(rows > height);
    VnOverlayRect {
        col: usize::from(cols > 1),
        row: rows.saturating_sub(height.saturating_add(bottom_margin)),
        width,
        height,
    }
}

fn vn_overlay_nameplate_text_rect(
    nameplate: &VnOverlayRect,
    text_inset_cols: usize,
    text_inset_rows: usize,
) -> VnOverlayRect {
    let col_inset = text_inset_cols.min(nameplate.width.saturating_sub(1));
    let row_inset = text_inset_rows.min(nameplate.height.saturating_sub(1));
    VnOverlayRect {
        col: nameplate.col.saturating_add(col_inset),
        row: nameplate.row.saturating_add(row_inset),
        width: nameplate
            .width
            .saturating_sub(col_inset.saturating_add(1))
            .max(1),
        height: 1,
    }
}
