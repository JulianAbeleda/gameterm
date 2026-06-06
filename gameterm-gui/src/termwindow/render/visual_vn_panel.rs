use ::window::bitmaps::atlas::Sprite;
use ::window::bitmaps::{TextureCoord, TextureRect, TextureSize};
use ::window::RectF;
use gameterm_visual::{
    VisualRenderSnapshot, VnOverlayDebugOverrides, VnOverlayRect, VN_OVERLAY_NAMEPLATE_OPACITY,
    VN_OVERLAY_PANEL_OPACITY,
};
use std::sync::{Arc, LazyLock};
use termwiz::color::LinearRgba;
use termwiz::image::ImageData;

pub(super) const VN_PANEL_FILL: LinearRgba = LinearRgba(0.102, 0.1137, 0.1333, 0.4627);
pub(super) const VN_PANEL_BORDER: LinearRgba = LinearRgba(0.1608, 0.1725, 0.1961, 0.3608);
pub(super) const VN_NAMEPLATE_FILL: LinearRgba = LinearRgba(0.102, 0.1137, 0.1333, 0.58);
pub(super) const VN_NAMEPLATE_BORDER: LinearRgba = LinearRgba(0.1608, 0.1725, 0.1961, 0.42);
pub(super) const VN_PANEL_BORDER_WIDTH_PX: f32 = 1.5;
pub(super) const VN_DIALOGUE_PANEL_RADIUS_PX: f32 = 22.0;
pub(super) const VN_COMPOSER_PANEL_RADIUS_PX: f32 = 18.0;
pub(super) const VN_DIALOGUE_NAMEPLATE_RADIUS_PX: f32 = 13.0;
pub(super) const VN_COMPOSER_NAMEPLATE_RADIUS_PX: f32 = 11.0;
pub(super) const VN_PANEL_MIN_CORNER_SEGMENTS: usize = 12;
pub(super) const VN_PANEL_MAX_CORNER_SEGMENTS: usize = 32;
pub(super) const VN_PANEL_SLICE_PX: f32 = 32.0;
pub(super) const VN_STAGE_CHARACTER_HEIGHT_RATIO: f32 = 0.78;
pub(super) const VN_STAGE_CHARACTER_TARGET_WIDTH_RATIO: f32 = 0.34;
pub(super) const VN_DIALOGUE_SCROLLBAR_TRACK_COLOR: LinearRgba = LinearRgba(0.86, 0.88, 0.94, 0.22);
pub(super) const VN_DIALOGUE_SCROLLBAR_THUMB_COLOR: LinearRgba = LinearRgba(0.96, 0.97, 1.0, 0.74);
pub(super) const VN_VOICE_INDICATOR_OFF_FILL: LinearRgba = LinearRgba(0.0, 0.0, 0.0, 0.78);
pub(super) const VN_VOICE_INDICATOR_OFF_BORDER: LinearRgba = LinearRgba(0.9, 0.9, 0.9, 0.22);
pub(super) const VN_VOICE_INDICATOR_ON_FILL: LinearRgba = LinearRgba(0.08, 0.72, 0.25, 0.88);
pub(super) const VN_VOICE_INDICATOR_ON_BORDER: LinearRgba = LinearRgba(0.7, 1.0, 0.78, 0.36);
pub(super) static VN_PANEL_TEXTURE_RENDERING: LazyLock<bool> = LazyLock::new(|| {
    std::env::var("GAMETERM_SCENE_VN_PANEL_TEXTURE")
        .map(|value| {
            matches!(
                value.to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
});
pub(super) static VN_PANEL_IMAGE_DATA: LazyLock<Arc<ImageData>> = LazyLock::new(|| {
    Arc::new(ImageData::with_raw_data(
        include_bytes!("../../../../assets/gameterm-scene/vn-panel.png").to_vec(),
    ))
});

pub(super) fn visual_placeholder_color(sprite: &str, alpha: f32, floor: f32) -> LinearRgba {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in sprite.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }

    let channel = |shift: u32| {
        let raw = ((hash >> shift) & 0xff_u64) as f32 / 255.0;
        floor + raw * (1.0 - floor)
    };
    LinearRgba::with_components(channel(0), channel(8), channel(16), alpha)
}

pub(super) fn visual_selection_color(alpha: f32) -> LinearRgba {
    LinearRgba::with_components(1.0, 0.92, 0.34, alpha)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct VnPanelStyle {
    pub(super) fill: LinearRgba,
    pub(super) border: LinearRgba,
    pub(super) border_width: f32,
    pub(super) radius: f32,
}

impl VnPanelStyle {
    pub(super) fn dialogue_panel(opacity: f32) -> Self {
        Self {
            fill: with_alpha(VN_PANEL_FILL, opacity),
            border: with_scaled_alpha(VN_PANEL_BORDER, VN_OVERLAY_PANEL_OPACITY, opacity),
            border_width: VN_PANEL_BORDER_WIDTH_PX,
            radius: VN_DIALOGUE_PANEL_RADIUS_PX,
        }
    }

    pub(super) fn composer_panel(opacity: f32) -> Self {
        Self {
            fill: with_alpha(VN_PANEL_FILL, opacity),
            border: with_scaled_alpha(VN_PANEL_BORDER, VN_OVERLAY_PANEL_OPACITY, opacity),
            border_width: VN_PANEL_BORDER_WIDTH_PX,
            radius: VN_COMPOSER_PANEL_RADIUS_PX,
        }
    }

    pub(super) fn dialogue_nameplate(opacity: f32) -> Self {
        Self {
            fill: with_alpha(VN_NAMEPLATE_FILL, opacity),
            border: with_scaled_alpha(VN_NAMEPLATE_BORDER, VN_OVERLAY_NAMEPLATE_OPACITY, opacity),
            border_width: VN_PANEL_BORDER_WIDTH_PX,
            radius: VN_DIALOGUE_NAMEPLATE_RADIUS_PX,
        }
    }

    pub(super) fn composer_nameplate(opacity: f32) -> Self {
        Self {
            fill: with_alpha(VN_NAMEPLATE_FILL, opacity),
            border: with_scaled_alpha(VN_NAMEPLATE_BORDER, VN_OVERLAY_NAMEPLATE_OPACITY, opacity),
            border_width: VN_PANEL_BORDER_WIDTH_PX,
            radius: VN_COMPOSER_NAMEPLATE_RADIUS_PX,
        }
    }

    pub(super) fn voice_indicator(active: bool) -> Self {
        let (fill, border) = if active {
            (VN_VOICE_INDICATOR_ON_FILL, VN_VOICE_INDICATOR_ON_BORDER)
        } else {
            (VN_VOICE_INDICATOR_OFF_FILL, VN_VOICE_INDICATOR_OFF_BORDER)
        };
        Self {
            fill,
            border,
            border_width: VN_PANEL_BORDER_WIDTH_PX,
            radius: VN_COMPOSER_NAMEPLATE_RADIUS_PX,
        }
    }
}

pub(super) fn with_alpha(color: LinearRgba, alpha: f32) -> LinearRgba {
    LinearRgba(color.0, color.1, color.2, alpha.clamp(0.0, 1.0))
}

pub(super) fn with_scaled_alpha(color: LinearRgba, baseline: f32, alpha: f32) -> LinearRgba {
    let scale = if baseline <= 0.0 {
        1.0
    } else {
        color.3 / baseline
    };
    with_alpha(color, alpha * scale)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct RoundedRectPrimitive {
    pub(super) rect: RectF,
    pub(super) fill: LinearRgba,
    pub(super) border: LinearRgba,
    pub(super) border_width: f32,
    pub(super) radius: f32,
}

impl RoundedRectPrimitive {
    pub(super) fn new(rect: RectF, style: VnPanelStyle) -> Self {
        let radius = style
            .radius
            .max(4.0)
            .min(rect.size.width / 3.0)
            .min(rect.size.height / 2.0);
        Self {
            rect,
            fill: style.fill,
            border: style.border,
            border_width: style.border_width,
            radius,
        }
    }

    pub(super) fn inner_rect(self) -> RectF {
        inset_rect(self.rect, self.border_width)
    }

    pub(super) fn inner_radius(self) -> f32 {
        (self.radius - self.border_width).max(1.0)
    }
}

pub(super) fn vn_panel_texture_rendering_enabled() -> bool {
    *VN_PANEL_TEXTURE_RENDERING
}

pub(super) fn vn_overlay_layout_dims(
    snapshot: Option<&VisualRenderSnapshot>,
    fallback_cols: usize,
    fallback_rows: usize,
) -> (usize, usize) {
    let Some((cols, rows)) = snapshot.and_then(VisualRenderSnapshot::overlay_layout_dims) else {
        return (fallback_cols.max(1), fallback_rows.max(1));
    };

    debug_assert_eq!(
        fallback_cols, cols,
        "Mismatch in VN layout columns between text and visual layers"
    );
    debug_assert_eq!(
        fallback_rows, rows,
        "Mismatch in VN layout rows between text and visual layers",
    );

    (cols.max(1), rows.max(1))
}

pub(super) fn vn_panel_rects(
    layout: &gameterm_visual::VnOverlayLayout,
    stage_rect: RectF,
    cell_width: f32,
    cell_height: f32,
) -> Vec<RectF> {
    let mut rects = Vec::new();
    if let Some(composer_panel) = layout.composer_panel {
        rects.push(vn_overlay_rect_to_pixels(
            composer_panel,
            stage_rect,
            cell_width,
            cell_height,
        ));
    }
    rects.push(vn_overlay_rect_to_pixels(
        layout.dialogue_panel,
        stage_rect,
        cell_width,
        cell_height,
    ));
    rects
}

pub(super) fn vn_panel_nameplate_rects(
    layout: &gameterm_visual::VnOverlayLayout,
    stage_rect: RectF,
    cell_width: f32,
    cell_height: f32,
) -> Vec<RectF> {
    let mut rects = Vec::new();
    if let Some(composer_nameplate) = layout.composer_nameplate {
        rects.push(vn_overlay_rect_to_pixels(
            composer_nameplate,
            stage_rect,
            cell_width,
            cell_height,
        ));
    }
    rects.push(vn_overlay_rect_to_pixels(
        layout.dialogue_nameplate,
        stage_rect,
        cell_width,
        cell_height,
    ));
    rects
}

pub(super) fn vn_overlay_rect_to_pixels(
    rect: VnOverlayRect,
    stage_rect: RectF,
    cell_width: f32,
    cell_height: f32,
) -> RectF {
    euclid::rect(
        stage_rect.min_x() + rect.col as f32 * cell_width,
        stage_rect.min_y() + rect.row as f32 * cell_height,
        (rect.width as f32 * cell_width).max(cell_width),
        (rect.height as f32 * cell_height).max(cell_height),
    )
}

pub(super) fn vn_dialogue_scrollbar_rects(
    layout: &gameterm_visual::VnOverlayLayout,
    snapshot: &VisualRenderSnapshot,
    stage_rect: RectF,
    cell_width: f32,
    cell_height: f32,
) -> Option<(RectF, RectF)> {
    let metrics = snapshot.vn_dialogue_scroll?;
    if metrics.max_scroll_offset == 0 || metrics.visible_rows == 0 {
        return None;
    }

    let track_width = (cell_width * 0.32).clamp(3.0, 7.0);
    let panel =
        vn_overlay_rect_to_pixels(layout.dialogue_panel, stage_rect, cell_width, cell_height);
    let track_top = stage_rect.min_y() + layout.dialogue_text_row as f32 * cell_height;
    let track_height = metrics.visible_rows as f32 * cell_height;
    if track_height <= 0.0 {
        return None;
    }
    let gutter_width = layout.dialogue_text_inset_cols as f32 * cell_width;
    let track_x = panel.max_x() - (gutter_width * 0.5) - (track_width * 0.5);
    let track = euclid::rect(track_x, track_top, track_width, track_height);

    let thumb_height = ((track_height * metrics.visible_rows as f32)
        / metrics.total_lines.max(1) as f32)
        .max(cell_height)
        .min(track_height);
    let travel = (track_height - thumb_height).max(0.0);
    let distance_from_top = metrics
        .max_scroll_offset
        .saturating_sub(metrics.scroll_offset) as f32;
    let thumb_top = if metrics.max_scroll_offset == 0 {
        0.0
    } else {
        travel * distance_from_top / metrics.max_scroll_offset as f32
    };
    let thumb = euclid::rect(
        track.min_x(),
        track.min_y() + thumb_top,
        track.size.width,
        thumb_height,
    );
    Some((track, thumb))
}

pub(super) fn vn_panel_styles(
    layout: &gameterm_visual::VnOverlayLayout,
    overrides: Option<&VnOverlayDebugOverrides>,
) -> Vec<VnPanelStyle> {
    let mut styles = Vec::new();
    if layout.composer_panel.is_some() {
        styles.push(VnPanelStyle::composer_panel(
            overrides
                .map(|overrides| overrides.composer_panel_opacity)
                .unwrap_or(VN_OVERLAY_PANEL_OPACITY),
        ));
    }
    styles.push(VnPanelStyle::dialogue_panel(
        overrides
            .map(|overrides| overrides.dialogue_panel_opacity)
            .unwrap_or(VN_OVERLAY_PANEL_OPACITY),
    ));
    styles
}

pub(super) fn vn_panel_nameplate_styles(
    layout: &gameterm_visual::VnOverlayLayout,
    overrides: Option<&VnOverlayDebugOverrides>,
) -> Vec<VnPanelStyle> {
    let mut styles = Vec::new();
    if layout.composer_nameplate.is_some() {
        styles.push(VnPanelStyle::composer_nameplate(
            overrides
                .map(|overrides| overrides.composer_nameplate_opacity)
                .unwrap_or(VN_OVERLAY_NAMEPLATE_OPACITY),
        ));
    }
    styles.push(VnPanelStyle::dialogue_nameplate(
        overrides
            .map(|overrides| overrides.dialogue_nameplate_opacity)
            .unwrap_or(VN_OVERLAY_NAMEPLATE_OPACITY),
    ));
    styles
}

pub(super) fn rounded_panel_corner_segments(radius: f32) -> usize {
    ((radius / 1.25).ceil() as usize)
        .max(VN_PANEL_MIN_CORNER_SEGMENTS)
        .min(VN_PANEL_MAX_CORNER_SEGMENTS)
}

pub(super) fn rounded_panel_rects(rect: RectF, radius: f32) -> Vec<RectF> {
    let radius = radius
        .max(0.0)
        .min(rect.size.width / 2.0)
        .min(rect.size.height / 2.0);
    if radius <= 0.0 {
        return vec![rect];
    }

    let mut rects = Vec::new();
    let middle_height = (rect.size.height - radius * 2.0).max(0.0);
    if middle_height > 0.0 {
        rects.push(euclid::rect(
            rect.min_x(),
            rect.min_y() + radius,
            rect.size.width,
            middle_height,
        ));
    }

    let corner_segments = rounded_panel_corner_segments(radius);
    let strip_height = radius / corner_segments as f32;
    for segment in 0..corner_segments {
        let y0 = segment as f32 * strip_height;
        if y0 >= radius {
            break;
        }
        let y1 = ((segment + 1) as f32 * strip_height).min(radius);
        let height = y1 - y0;
        if height <= 0.0 {
            continue;
        }
        let sample_y = y0 + height * 0.5;
        let distance_from_center = radius - sample_y;
        let x_extent = (radius * radius - distance_from_center * distance_from_center)
            .max(0.0)
            .sqrt();
        let inset = (radius - x_extent).max(0.0);
        let width = (rect.size.width - inset * 2.0).max(1.0);
        rects.push(euclid::rect(
            rect.min_x() + inset,
            rect.min_y() + y0,
            width,
            height,
        ));
        rects.push(euclid::rect(
            rect.min_x() + inset,
            rect.max_y() - y1,
            width,
            height,
        ));
    }

    rects
}

pub(super) fn vn_panel_screen_slices(rect: RectF, cell_width: f32) -> Vec<RectF> {
    let margin = (cell_width * 3.0)
        .max(16.0)
        .min(rect.size.width / 2.0)
        .min(rect.size.height / 2.0);
    nine_slice_rects(rect, margin)
}

pub(super) fn vn_panel_texture_slices() -> Vec<RectF> {
    let rect = euclid::rect(0.0, 0.0, 128.0, 128.0);
    nine_slice_rects(rect, VN_PANEL_SLICE_PX)
}

pub(super) fn nine_slice_rects(rect: RectF, margin: f32) -> Vec<RectF> {
    let margin = margin
        .max(0.0)
        .min(rect.size.width / 2.0)
        .min(rect.size.height / 2.0);
    let x0 = rect.min_x();
    let x1 = rect.min_x() + margin;
    let x2 = rect.max_x() - margin;
    let x3 = rect.max_x();
    let y0 = rect.min_y();
    let y1 = rect.min_y() + margin;
    let y2 = rect.max_y() - margin;
    let y3 = rect.max_y();
    vec![
        euclid::rect(x0, y0, x1 - x0, y1 - y0),
        euclid::rect(x1, y0, x2 - x1, y1 - y0),
        euclid::rect(x2, y0, x3 - x2, y1 - y0),
        euclid::rect(x0, y1, x1 - x0, y2 - y1),
        euclid::rect(x1, y1, x2 - x1, y2 - y1),
        euclid::rect(x2, y1, x3 - x2, y2 - y1),
        euclid::rect(x0, y2, x1 - x0, y3 - y2),
        euclid::rect(x1, y2, x2 - x1, y3 - y2),
        euclid::rect(x2, y2, x3 - x2, y3 - y2),
    ]
}

pub(super) fn sprite_texture_rect(sprite: &Sprite, source_rect: RectF) -> TextureRect {
    let texture_width = sprite.texture.width() as f32;
    let texture_height = sprite.texture.height() as f32;
    TextureRect::new(
        TextureCoord::new(
            (sprite.coords.origin.x as f32 + source_rect.min_x()) / texture_width,
            (sprite.coords.origin.y as f32 + source_rect.min_y()) / texture_height,
        ),
        TextureSize::new(
            source_rect.size.width / texture_width,
            source_rect.size.height / texture_height,
        ),
    )
}

pub(super) fn inset_rect(rect: RectF, inset: f32) -> RectF {
    let inset = inset
        .max(0.0)
        .min(rect.size.width / 2.0)
        .min(rect.size.height / 2.0);
    euclid::rect(
        rect.min_x() + inset,
        rect.min_y() + inset,
        (rect.size.width - inset * 2.0).max(1.0),
        (rect.size.height - inset * 2.0).max(1.0),
    )
}
