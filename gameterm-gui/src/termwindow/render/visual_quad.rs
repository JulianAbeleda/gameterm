use crate::quad::{QuadTrait, TripleLayerQuadAllocator, TripleLayerQuadAllocatorTrait};
use crate::termwindow::render::paint::AllowImage;
use crate::termwindow::render::RenderScreenLineParams;
use crate::termwindow::TermWindow;
use ::window::bitmaps::atlas::Sprite;
use ::window::bitmaps::{TextureCoord, TextureRect, TextureSize};
use ::window::RectF;
use anyhow::Context;
use config::HsbTransform;
use gameterm_visual::{
    vn_overlay_layout, vn_overlay_layout_with_overrides, VisualRenderEntity, VisualRenderSnapshot,
    VisualRenderStageDisplayable, VisualRenderTile, VisualStagePlacement, VisualView,
    VnOverlayRect,
};
use std::sync::{Arc, LazyLock};
use termwiz::color::LinearRgba;
use termwiz::image::ImageData;

const VN_PANEL_FILL: LinearRgba = LinearRgba(0.102, 0.1137, 0.1333, 0.4627);
const VN_PANEL_BORDER: LinearRgba = LinearRgba(0.1608, 0.1725, 0.1961, 0.3608);
const VN_NAMEPLATE_FILL: LinearRgba = LinearRgba(0.102, 0.1137, 0.1333, 0.58);
const VN_NAMEPLATE_BORDER: LinearRgba = LinearRgba(0.1608, 0.1725, 0.1961, 0.42);
const VN_PANEL_BORDER_WIDTH_PX: f32 = 1.5;
const VN_DIALOGUE_PANEL_RADIUS_PX: f32 = 22.0;
const VN_COMPOSER_PANEL_RADIUS_PX: f32 = 18.0;
const VN_DIALOGUE_NAMEPLATE_RADIUS_PX: f32 = 13.0;
const VN_COMPOSER_NAMEPLATE_RADIUS_PX: f32 = 11.0;
const VN_PANEL_MIN_CORNER_SEGMENTS: usize = 12;
const VN_PANEL_MAX_CORNER_SEGMENTS: usize = 32;
const VN_PANEL_SLICE_PX: f32 = 32.0;
const VN_STAGE_CHARACTER_HEIGHT_RATIO: f32 = 0.78;
const VN_STAGE_CHARACTER_TARGET_WIDTH_RATIO: f32 = 0.34;
const VN_DIALOGUE_SCROLLBAR_TRACK_COLOR: LinearRgba = LinearRgba(0.86, 0.88, 0.94, 0.22);
const VN_DIALOGUE_SCROLLBAR_THUMB_COLOR: LinearRgba = LinearRgba(0.96, 0.97, 1.0, 0.74);
static VN_PANEL_TEXTURE_RENDERING: LazyLock<bool> = LazyLock::new(|| {
    std::env::var("GAMETERM_SCENE_VN_PANEL_TEXTURE")
        .map(|value| {
            matches!(
                value.to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
});
static VN_PANEL_IMAGE_DATA: LazyLock<Arc<ImageData>> = LazyLock::new(|| {
    Arc::new(ImageData::with_raw_data(
        include_bytes!("../../../../assets/gameterm-scene/vn-panel.png").to_vec(),
    ))
});

fn visual_placeholder_color(sprite: &str, alpha: f32, floor: f32) -> LinearRgba {
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

fn visual_selection_color(alpha: f32) -> LinearRgba {
    LinearRgba::with_components(1.0, 0.92, 0.34, alpha)
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct VnPanelStyle {
    fill: LinearRgba,
    border: LinearRgba,
    border_width: f32,
    radius: f32,
}

impl VnPanelStyle {
    fn dialogue_panel() -> Self {
        Self {
            fill: VN_PANEL_FILL,
            border: VN_PANEL_BORDER,
            border_width: VN_PANEL_BORDER_WIDTH_PX,
            radius: VN_DIALOGUE_PANEL_RADIUS_PX,
        }
    }

    fn composer_panel() -> Self {
        Self {
            fill: VN_PANEL_FILL,
            border: VN_PANEL_BORDER,
            border_width: VN_PANEL_BORDER_WIDTH_PX,
            radius: VN_COMPOSER_PANEL_RADIUS_PX,
        }
    }

    fn dialogue_nameplate() -> Self {
        Self {
            fill: VN_NAMEPLATE_FILL,
            border: VN_NAMEPLATE_BORDER,
            border_width: VN_PANEL_BORDER_WIDTH_PX,
            radius: VN_DIALOGUE_NAMEPLATE_RADIUS_PX,
        }
    }

    fn composer_nameplate() -> Self {
        Self {
            fill: VN_NAMEPLATE_FILL,
            border: VN_NAMEPLATE_BORDER,
            border_width: VN_PANEL_BORDER_WIDTH_PX,
            radius: VN_COMPOSER_NAMEPLATE_RADIUS_PX,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct RoundedRectPrimitive {
    rect: RectF,
    fill: LinearRgba,
    border: LinearRgba,
    border_width: f32,
    radius: f32,
}

impl RoundedRectPrimitive {
    fn new(rect: RectF, style: VnPanelStyle) -> Self {
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

    fn inner_rect(self) -> RectF {
        inset_rect(self.rect, self.border_width)
    }

    fn inner_radius(self) -> f32 {
        (self.radius - self.border_width).max(1.0)
    }
}

fn vn_panel_texture_rendering_enabled() -> bool {
    *VN_PANEL_TEXTURE_RENDERING
}

impl TermWindow {
    pub(super) fn populate_visual_stage(
        &self,
        snapshot: &VisualRenderSnapshot,
        layers: &mut TripleLayerQuadAllocator,
        params: &RenderScreenLineParams,
        cell_height: f32,
        hsv: Option<HsbTransform>,
    ) -> anyhow::Result<()> {
        if snapshot.stage.is_empty() {
            return Ok(());
        }
        // The tile debugger is a plain entity inspector; suppress all stage art.
        if matches!(snapshot.view, VisualView::TileDebugger) {
            return Ok(());
        }
        // The VN layout debugger keeps the panels and nameplates visible so the
        // boxes can be tuned live, but drops the background and character art.
        let suppress_stage_art = matches!(snapshot.view, VisualView::VnLayoutDebugger);

        let stage_rect = stage_viewport_rect(params, cell_height);
        if !suppress_stage_art {
            for displayable in &snapshot.stage {
                let rect = stage_displayable_target_rect(displayable, stage_rect);
                let scale_mode = stage_displayable_scale_mode(displayable);
                let layer_num = match displayable.placement {
                    VisualStagePlacement::Fullscreen => 0,
                    VisualStagePlacement::Left
                    | VisualStagePlacement::Center
                    | VisualStagePlacement::Right => 1,
                };
                if self.populate_visual_sprite_quad_with_scale(
                    &displayable.sprite,
                    layers,
                    layer_num,
                    rect,
                    scale_mode,
                    params,
                    hsv,
                )? {
                    continue;
                }
                let rect = stage_displayable_placeholder_rect(displayable, rect);
                let mut quad = self.filled_rectangle(
                    layers,
                    layer_num,
                    rect,
                    visual_placeholder_color(&displayable.sprite, 0.42, 0.24),
                )?;
                quad.set_hsv(hsv);
            }
        }
        self.populate_visual_vn_panels(layers, stage_rect, params, cell_height, hsv)?;

        Ok(())
    }

    fn populate_visual_vn_panels(
        &self,
        layers: &mut TripleLayerQuadAllocator,
        stage_rect: RectF,
        params: &RenderScreenLineParams,
        cell_height: f32,
        hsv: Option<HsbTransform>,
    ) -> anyhow::Result<()> {
        let cell_width = params.render_metrics.cell_size.width as f32;
        let speaker = params
            .visual_snapshot
            .map(|snapshot| snapshot.dialogue_speaker.as_str())
            .unwrap_or("Codex");
        let (layout_cols, layout_rows) = vn_overlay_layout_dims(
            params.visual_snapshot,
            params.dims.cols,
            params.dims.viewport_rows,
        );
        let layout = match params
            .visual_snapshot
            .and_then(|s| s.vn_layout_debug.as_ref())
        {
            Some(overrides) => vn_overlay_layout_with_overrides(
                layout_cols,
                layout_rows,
                speaker,
                "Composer",
                overrides,
            ),
            None => vn_overlay_layout(layout_cols, layout_rows, speaker, "Composer"),
        };
        for (rect, style) in vn_panel_rects(&layout, stage_rect, cell_width, cell_height)
            .into_iter()
            .zip(vn_panel_styles(&layout))
        {
            self.populate_vn_panel_surface(layers, 1, rect, cell_width, params, style, hsv)?;
        }
        for (rect, style) in vn_panel_nameplate_rects(&layout, stage_rect, cell_width, cell_height)
            .into_iter()
            .zip(vn_panel_nameplate_styles(&layout))
        {
            self.populate_vn_panel_surface(layers, 1, rect, cell_width, params, style, hsv)?;
        }
        if let Some((track, thumb)) = params.visual_snapshot.and_then(|snapshot| {
            vn_dialogue_scrollbar_rects(&layout, snapshot, stage_rect, cell_width, cell_height)
        }) {
            let mut track_quad =
                self.filled_rectangle(layers, 2, track, VN_DIALOGUE_SCROLLBAR_TRACK_COLOR)?;
            track_quad.set_hsv(hsv);
            let mut thumb_quad =
                self.filled_rectangle(layers, 2, thumb, VN_DIALOGUE_SCROLLBAR_THUMB_COLOR)?;
            thumb_quad.set_hsv(hsv);
        }
        Ok(())
    }

    fn populate_vn_panel_surface(
        &self,
        layers: &mut TripleLayerQuadAllocator,
        layer_num: usize,
        rect: RectF,
        cell_width: f32,
        params: &RenderScreenLineParams,
        style: VnPanelStyle,
        hsv: Option<HsbTransform>,
    ) -> anyhow::Result<()> {
        if vn_panel_texture_rendering_enabled()
            && self.populate_vn_panel_texture(layers, layer_num, rect, cell_width, params, hsv)?
        {
            return Ok(());
        }

        self.populate_rounded_rect_primitive(
            layers,
            layer_num,
            RoundedRectPrimitive::new(rect, style),
            hsv,
        )
    }

    fn populate_vn_panel_texture(
        &self,
        layers: &mut TripleLayerQuadAllocator,
        layer_num: usize,
        rect: RectF,
        cell_width: f32,
        params: &RenderScreenLineParams,
        hsv: Option<HsbTransform>,
    ) -> anyhow::Result<bool> {
        if self.allow_images == AllowImage::No {
            return Ok(false);
        }

        let Some(gl_state) = self.render_state.as_ref() else {
            return Ok(false);
        };
        let padding = params
            .render_metrics
            .cell_size
            .height
            .max(params.render_metrics.cell_size.width) as usize;
        let padding = if padding.is_power_of_two() {
            padding
        } else {
            padding.next_power_of_two()
        };
        let (sprite, next_due, _load_state) = gl_state
            .glyph_cache
            .borrow_mut()
            .cached_image(&VN_PANEL_IMAGE_DATA, Some(padding), self.allow_images)
            .context("cached vn panel image")?;
        self.update_next_frame_time(next_due);

        let screen_slices = vn_panel_screen_slices(rect, cell_width);
        let texture_slices = vn_panel_texture_slices();
        let left_offset = self.dimensions.pixel_width as f32 / 2.;
        let top_offset = self.dimensions.pixel_height as f32 / 2.;
        for (screen_rect, texture_rect) in screen_slices.into_iter().zip(texture_slices) {
            if screen_rect.size.width <= 0.0 || screen_rect.size.height <= 0.0 {
                continue;
            }
            let mut quad = layers.allocate(layer_num)?;
            quad.set_position(
                screen_rect.min_x() - left_offset,
                screen_rect.min_y() - top_offset,
                screen_rect.max_x() - left_offset,
                screen_rect.max_y() - top_offset,
            );
            quad.set_texture(sprite_texture_rect(&sprite, texture_rect));
            quad.set_fg_color(LinearRgba::with_components(1.0, 1.0, 1.0, 1.0));
            quad.set_hsv(hsv);
            quad.set_has_color(true);
        }

        Ok(true)
    }

    fn populate_rounded_rect_primitive(
        &self,
        layers: &mut TripleLayerQuadAllocator,
        layer_num: usize,
        primitive: RoundedRectPrimitive,
        hsv: Option<HsbTransform>,
    ) -> anyhow::Result<()> {
        for panel_rect in rounded_panel_rects(primitive.rect, primitive.radius) {
            let mut quad =
                self.filled_rectangle(layers, layer_num, panel_rect, primitive.border)?;
            quad.set_hsv(hsv);
        }

        for panel_rect in rounded_panel_rects(primitive.inner_rect(), primitive.inner_radius()) {
            let mut quad = self.filled_rectangle(layers, layer_num, panel_rect, primitive.fill)?;
            quad.set_hsv(hsv);
        }
        Ok(())
    }

    pub(super) fn populate_visual_tile(
        &self,
        tile: &VisualRenderTile,
        layers: &mut TripleLayerQuadAllocator,
        params: &RenderScreenLineParams,
        cell_width: f32,
        cell_height: f32,
        hsv: Option<HsbTransform>,
    ) -> anyhow::Result<()> {
        let rect: RectF = euclid::rect(
            params.left_pixel_x + tile.position.x as f32 * cell_width,
            params.top_pixel_y,
            cell_width,
            cell_height,
        );
        if self.populate_visual_sprite_quad(&tile.sprite, layers, 0, rect, &params, hsv)? {
            return Ok(());
        }
        let mut quad = self.filled_rectangle(
            layers,
            0,
            rect,
            visual_placeholder_color(&tile.sprite, 0.26, 0.18),
        )?;
        quad.set_hsv(hsv);
        Ok(())
    }

    pub(super) fn populate_visual_entity(
        &self,
        entity: &VisualRenderEntity,
        layers: &mut TripleLayerQuadAllocator,
        params: &RenderScreenLineParams,
        cell_width: f32,
        cell_height: f32,
        hsv: Option<HsbTransform>,
    ) -> anyhow::Result<()> {
        let inset = if entity.selected { 1.0 } else { 2.0 };
        let rect: RectF = euclid::rect(
            params.left_pixel_x + entity.position.x as f32 * cell_width + inset,
            params.top_pixel_y + inset,
            (cell_width - inset * 2.0).max(1.0),
            (cell_height - inset * 2.0).max(1.0),
        );
        if entity.selected {
            let mut quad = self.filled_rectangle(
                layers,
                1,
                euclid::rect(
                    params.left_pixel_x + entity.position.x as f32 * cell_width,
                    params.top_pixel_y,
                    cell_width,
                    cell_height,
                ),
                visual_selection_color(0.42),
            )?;
            quad.set_hsv(hsv);
            let outline = [
                euclid::rect(
                    params.left_pixel_x + entity.position.x as f32 * cell_width,
                    params.top_pixel_y,
                    cell_width,
                    1.5,
                ),
                euclid::rect(
                    params.left_pixel_x + entity.position.x as f32 * cell_width,
                    params.top_pixel_y + cell_height - 1.5,
                    cell_width,
                    1.5,
                ),
                euclid::rect(
                    params.left_pixel_x + entity.position.x as f32 * cell_width,
                    params.top_pixel_y,
                    1.5,
                    cell_height,
                ),
                euclid::rect(
                    params.left_pixel_x + (entity.position.x + 1) as f32 * cell_width - 1.5,
                    params.top_pixel_y,
                    1.5,
                    cell_height,
                ),
            ];
            for rect in outline {
                let mut quad =
                    self.filled_rectangle(layers, 2, rect, visual_selection_color(0.9))?;
                quad.set_hsv(hsv);
            }
        }
        if self.populate_visual_sprite_quad(&entity.sprite, layers, 2, rect, &params, hsv)? {
            return Ok(());
        }
        let placeholder_color = visual_placeholder_color(
            &entity.sprite,
            if entity.selected { 0.92 } else { 0.72 },
            if entity.selected { 0.86 } else { 0.66 },
        );
        let mut quad = self.filled_rectangle(layers, 2, rect, placeholder_color)?;
        quad.set_hsv(hsv);
        let marker_rect: RectF = euclid::rect(
            rect.origin.x + rect.size.width * 0.28,
            rect.origin.y + rect.size.height * 0.28,
            (rect.size.width * 0.44).max(1.0),
            (rect.size.height * 0.44).max(1.0),
        );
        let mut quad = self.filled_rectangle(
            layers,
            2,
            marker_rect,
            visual_placeholder_color(&entity.sprite, 0.95, 0.25),
        )?;
        quad.set_hsv(hsv);
        Ok(())
    }

    fn populate_visual_sprite_quad(
        &self,
        sprite_id: &str,
        layers: &mut TripleLayerQuadAllocator,
        layer_num: usize,
        rect: RectF,
        params: &RenderScreenLineParams,
        hsv: Option<HsbTransform>,
    ) -> anyhow::Result<bool> {
        self.populate_visual_sprite_quad_with_scale(
            sprite_id,
            layers,
            layer_num,
            rect,
            VisualImageScaleMode::Stretch,
            params,
            hsv,
        )
    }

    fn populate_visual_sprite_quad_with_scale(
        &self,
        sprite_id: &str,
        layers: &mut TripleLayerQuadAllocator,
        layer_num: usize,
        rect: RectF,
        scale_mode: VisualImageScaleMode,
        params: &RenderScreenLineParams,
        hsv: Option<HsbTransform>,
    ) -> anyhow::Result<bool> {
        let Some(image_data) =
            visual_sprite_image_data(sprite_id, self.allow_images, params.visual_sprites)
        else {
            return Ok(false);
        };

        let gl_state = self.render_state.as_ref().unwrap();
        let padding = params
            .render_metrics
            .cell_size
            .height
            .max(params.render_metrics.cell_size.width) as usize;
        let padding = if padding.is_power_of_two() {
            padding
        } else {
            padding.next_power_of_two()
        };
        let (sprite, next_due, _load_state) = gl_state
            .glyph_cache
            .borrow_mut()
            .cached_image(image_data, Some(padding), self.allow_images)
            .context("cached_image")?;
        self.update_next_frame_time(next_due);

        let source_size = VisualImageSourceSize {
            width: sprite.coords.size.width as f32,
            height: sprite.coords.size.height as f32,
        };
        let rect = resolve_aspect_rect(source_size, rect, scale_mode);

        let left_offset = self.dimensions.pixel_width as f32 / 2.;
        let top_offset = self.dimensions.pixel_height as f32 / 2.;
        let mut quad = layers.allocate(layer_num)?;
        quad.set_position(
            rect.min_x() - left_offset,
            rect.min_y() - top_offset,
            rect.max_x() - left_offset,
            rect.max_y() - top_offset,
        );
        quad.set_texture(sprite.texture_coords());
        quad.set_fg_color(LinearRgba::with_components(1.0, 1.0, 1.0, 1.0));
        quad.set_hsv(hsv);
        quad.set_has_color(true);
        Ok(true)
    }
}

fn stage_viewport_rect(params: &RenderScreenLineParams, cell_height: f32) -> RectF {
    euclid::rect(
        params.left_pixel_x,
        params.top_pixel_y,
        params.pixel_width,
        params.dims.viewport_rows as f32 * cell_height,
    )
}

fn vn_overlay_layout_dims(
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

fn vn_panel_rects(
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

fn vn_panel_nameplate_rects(
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

fn vn_overlay_rect_to_pixels(
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

fn vn_dialogue_scrollbar_rects(
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
    let track = euclid::rect(
        panel.max_x() - layout.dialogue_text_inset_cols as f32 * cell_width + track_width,
        track_top,
        track_width,
        track_height,
    );

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

fn vn_panel_styles(layout: &gameterm_visual::VnOverlayLayout) -> Vec<VnPanelStyle> {
    let mut styles = Vec::new();
    if layout.composer_panel.is_some() {
        styles.push(VnPanelStyle::composer_panel());
    }
    styles.push(VnPanelStyle::dialogue_panel());
    styles
}

fn vn_panel_nameplate_styles(layout: &gameterm_visual::VnOverlayLayout) -> Vec<VnPanelStyle> {
    let mut styles = Vec::new();
    if layout.composer_nameplate.is_some() {
        styles.push(VnPanelStyle::composer_nameplate());
    }
    styles.push(VnPanelStyle::dialogue_nameplate());
    styles
}

fn rounded_panel_corner_segments(radius: f32) -> usize {
    ((radius / 1.25).ceil() as usize)
        .max(VN_PANEL_MIN_CORNER_SEGMENTS)
        .min(VN_PANEL_MAX_CORNER_SEGMENTS)
}

fn rounded_panel_rects(rect: RectF, radius: f32) -> Vec<RectF> {
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

fn vn_panel_screen_slices(rect: RectF, cell_width: f32) -> Vec<RectF> {
    let margin = (cell_width * 3.0)
        .max(16.0)
        .min(rect.size.width / 2.0)
        .min(rect.size.height / 2.0);
    nine_slice_rects(rect, margin)
}

fn vn_panel_texture_slices() -> Vec<RectF> {
    let rect = euclid::rect(0.0, 0.0, 128.0, 128.0);
    nine_slice_rects(rect, VN_PANEL_SLICE_PX)
}

fn nine_slice_rects(rect: RectF, margin: f32) -> Vec<RectF> {
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

fn sprite_texture_rect(sprite: &Sprite, source_rect: RectF) -> TextureRect {
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

fn inset_rect(rect: RectF, inset: f32) -> RectF {
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum VisualImageScaleMode {
    Stretch,
    #[allow(dead_code)]
    FitCenter,
    FitBottomCenter,
    FillCenter,
    #[allow(dead_code)]
    IntegerFitCenter,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct VisualImageSourceSize {
    width: f32,
    height: f32,
}

fn resolve_aspect_rect(
    source: VisualImageSourceSize,
    target: RectF,
    mode: VisualImageScaleMode,
) -> RectF {
    if mode == VisualImageScaleMode::Stretch
        || source.width <= 0.0
        || source.height <= 0.0
        || target.size.width <= 0.0
        || target.size.height <= 0.0
    {
        return target;
    }

    let scale_x = target.size.width / source.width;
    let scale_y = target.size.height / source.height;
    let scale = match mode {
        VisualImageScaleMode::Stretch => unreachable!(),
        VisualImageScaleMode::FitCenter | VisualImageScaleMode::FitBottomCenter => {
            scale_x.min(scale_y)
        }
        VisualImageScaleMode::FillCenter => scale_x.max(scale_y),
        VisualImageScaleMode::IntegerFitCenter => {
            let fit = scale_x.min(scale_y);
            if fit >= 1.0 {
                fit.floor().max(1.0)
            } else {
                fit
            }
        }
    };

    let width = (source.width * scale).max(1.0);
    let height = (source.height * scale).max(1.0);
    let x = target.min_x() + (target.size.width - width) / 2.0;
    let y = match mode {
        VisualImageScaleMode::FitBottomCenter => target.max_y() - height,
        _ => target.min_y() + (target.size.height - height) / 2.0,
    };

    euclid::rect(x, y, width, height)
}

fn stage_displayable_target_rect(
    displayable: &VisualRenderStageDisplayable,
    stage_rect: RectF,
) -> RectF {
    match displayable.placement {
        VisualStagePlacement::Fullscreen => stage_rect,
        VisualStagePlacement::Left | VisualStagePlacement::Center | VisualStagePlacement::Right => {
            let height = (stage_rect.size.height * VN_STAGE_CHARACTER_HEIGHT_RATIO).max(1.0);
            let width = (stage_rect.size.width * VN_STAGE_CHARACTER_TARGET_WIDTH_RATIO).max(1.0);
            let center_x = match displayable.placement {
                VisualStagePlacement::Left => stage_rect.min_x() + stage_rect.size.width * 0.28,
                VisualStagePlacement::Center => stage_rect.min_x() + stage_rect.size.width * 0.50,
                VisualStagePlacement::Right => stage_rect.min_x() + stage_rect.size.width * 0.72,
                VisualStagePlacement::Fullscreen => unreachable!(),
            };
            euclid::rect(
                center_x - width / 2.0,
                stage_rect.max_y() - height,
                width,
                height,
            )
        }
    }
}

fn stage_displayable_scale_mode(
    displayable: &VisualRenderStageDisplayable,
) -> VisualImageScaleMode {
    match displayable.placement {
        // Fullscreen backgrounds use cover semantics. The generated rect may
        // exceed the viewport and rely on framebuffer clipping.
        VisualStagePlacement::Fullscreen => VisualImageScaleMode::FillCenter,
        VisualStagePlacement::Left | VisualStagePlacement::Center | VisualStagePlacement::Right => {
            VisualImageScaleMode::FitBottomCenter
        }
    }
}

fn stage_displayable_placeholder_rect(
    displayable: &VisualRenderStageDisplayable,
    target: RectF,
) -> RectF {
    let source = match displayable.placement {
        VisualStagePlacement::Fullscreen => VisualImageSourceSize {
            width: 16.0,
            height: 9.0,
        },
        VisualStagePlacement::Left | VisualStagePlacement::Center | VisualStagePlacement::Right => {
            VisualImageSourceSize {
                width: 1.0,
                height: 2.0,
            }
        }
    };
    resolve_aspect_rect(source, target, stage_displayable_scale_mode(displayable))
}

fn visual_sprite_image_data<'a>(
    sprite_id: &str,
    allow_images: AllowImage,
    visual_sprites: Option<&'a crate::termwindow::render::VisualSpriteImages>,
) -> Option<&'a Arc<ImageData>> {
    if allow_images == AllowImage::No {
        return None;
    }

    visual_sprites.and_then(|sprites| sprites.sprites.get(sprite_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::termwindow::render::VisualSpriteImages;
    use gameterm_visual::{SceneRuntime, VisualScene};
    use std::collections::HashMap;

    fn assert_approx_eq(left: f32, right: f32) {
        assert!(
            (left - right).abs() < 0.001,
            "expected {} to be approximately {}",
            left,
            right
        );
    }

    #[test]
    fn placeholder_color_is_deterministic_per_sprite() {
        let first = visual_placeholder_color("project_core", 0.72, 0.66);
        let second = visual_placeholder_color("project_core", 0.72, 0.66);
        let other = visual_placeholder_color("agent_idle", 0.72, 0.66);

        assert_eq!(first, second);
        assert_ne!(first, other);
    }

    #[test]
    fn visual_sprite_lookup_is_disabled_when_images_are_disabled() {
        let mut sprites = HashMap::new();
        sprites.insert(
            "task_tile".to_string(),
            Arc::new(ImageData::with_raw_data(vec![1, 2, 3])),
        );
        let images = VisualSpriteImages { sprites };

        assert!(visual_sprite_image_data("task_tile", AllowImage::No, Some(&images)).is_none());
    }

    #[test]
    fn visual_sprite_lookup_falls_back_when_sprite_is_missing() {
        let images = VisualSpriteImages {
            sprites: HashMap::new(),
        };

        assert!(visual_sprite_image_data("missing", AllowImage::Yes, Some(&images)).is_none());
    }

    #[test]
    fn visual_sprite_lookup_finds_available_sprite_when_images_are_allowed() {
        let mut sprites = HashMap::new();
        sprites.insert(
            "task_tile".to_string(),
            Arc::new(ImageData::with_raw_data(vec![1, 2, 3])),
        );
        let images = VisualSpriteImages { sprites };

        assert!(visual_sprite_image_data("task_tile", AllowImage::Yes, Some(&images)).is_some());
    }

    #[test]
    fn stage_displayable_rect_places_fullscreen_background() {
        let rect = euclid::rect(10.0, 20.0, 800.0, 600.0);
        let displayable = VisualRenderStageDisplayable {
            layer_id: "background".to_string(),
            tag: "background".to_string(),
            sprite: "vn.background.school_classroom".to_string(),
            placement: VisualStagePlacement::Fullscreen,
            layer_zorder: 0,
            zorder: 0,
        };

        assert_eq!(stage_displayable_target_rect(&displayable, rect), rect);
        assert_eq!(
            stage_displayable_scale_mode(&displayable),
            VisualImageScaleMode::FillCenter
        );
    }

    #[test]
    fn stage_displayable_target_rect_places_character_slots() {
        let stage = euclid::rect(0.0, 0.0, 1000.0, 800.0);
        let make_displayable = |placement| VisualRenderStageDisplayable {
            layer_id: "characters".to_string(),
            tag: "kiki".to_string(),
            sprite: "vn.character.kiki.neutral".to_string(),
            placement,
            layer_zorder: 10,
            zorder: 0,
        };

        let left =
            stage_displayable_target_rect(&make_displayable(VisualStagePlacement::Left), stage);
        let center =
            stage_displayable_target_rect(&make_displayable(VisualStagePlacement::Center), stage);
        let right =
            stage_displayable_target_rect(&make_displayable(VisualStagePlacement::Right), stage);

        assert_eq!(left.size.height, 624.0);
        assert_eq!(left.size.width, 340.0);
        assert_eq!(left.max_y(), stage.max_y());
        assert!(left.min_x() < center.min_x());
        assert!(center.min_x() < right.min_x());
    }

    #[test]
    fn fit_center_preserves_aspect_for_wide_source() {
        let source = VisualImageSourceSize {
            width: 800.0,
            height: 400.0,
        };
        let target = euclid::rect(10.0, 20.0, 300.0, 300.0);

        let rect = resolve_aspect_rect(source, target, VisualImageScaleMode::FitCenter);

        assert_approx_eq(rect.size.width / rect.size.height, 2.0);
        assert_approx_eq(rect.size.width, 300.0);
        assert_approx_eq(rect.size.height, 150.0);
        assert_approx_eq(rect.min_x(), 10.0);
        assert_approx_eq(rect.min_y(), 95.0);
    }

    #[test]
    fn fit_bottom_center_preserves_aspect_and_bottom_anchor() {
        let source = VisualImageSourceSize {
            width: 400.0,
            height: 800.0,
        };
        let target = euclid::rect(100.0, 50.0, 300.0, 600.0);

        let rect = resolve_aspect_rect(source, target, VisualImageScaleMode::FitBottomCenter);

        assert_approx_eq(rect.size.width / rect.size.height, 0.5);
        assert_approx_eq(rect.max_y(), target.max_y());
        assert!(rect.size.width <= target.size.width);
        assert!(rect.size.height <= target.size.height);
    }

    #[test]
    fn fill_center_preserves_aspect_and_covers_target() {
        let source = VisualImageSourceSize {
            width: 1600.0,
            height: 900.0,
        };
        let target = euclid::rect(0.0, 0.0, 800.0, 800.0);

        let rect = resolve_aspect_rect(source, target, VisualImageScaleMode::FillCenter);

        assert_approx_eq(rect.size.width / rect.size.height, 16.0 / 9.0);
        assert!(rect.size.width >= target.size.width);
        assert!(rect.size.height >= target.size.height);
        assert!(rect.min_x() < target.min_x());
    }

    #[test]
    fn integer_fit_center_uses_whole_scale_when_possible() {
        let source = VisualImageSourceSize {
            width: 64.0,
            height: 32.0,
        };
        let target = euclid::rect(0.0, 0.0, 170.0, 100.0);

        let rect = resolve_aspect_rect(source, target, VisualImageScaleMode::IntegerFitCenter);

        assert_approx_eq(rect.size.width, 128.0);
        assert_approx_eq(rect.size.height, 64.0);
        assert_approx_eq(rect.size.width / rect.size.height, 2.0);
    }

    #[test]
    fn resolve_aspect_rect_returns_target_for_degenerate_input() {
        let target = euclid::rect(10.0, 20.0, 300.0, 200.0);

        assert_eq!(
            resolve_aspect_rect(
                VisualImageSourceSize {
                    width: 0.0,
                    height: 100.0,
                },
                target,
                VisualImageScaleMode::FitCenter,
            ),
            target
        );
        assert_eq!(
            resolve_aspect_rect(
                VisualImageSourceSize {
                    width: 100.0,
                    height: -1.0,
                },
                target,
                VisualImageScaleMode::FillCenter,
            ),
            target
        );
        assert_eq!(
            resolve_aspect_rect(
                VisualImageSourceSize {
                    width: 100.0,
                    height: 100.0,
                },
                euclid::rect(0.0, 0.0, 0.0, 100.0),
                VisualImageScaleMode::FitBottomCenter,
            ),
            euclid::rect(0.0, 0.0, 0.0, 100.0)
        );
    }

    #[test]
    fn staged_character_rect_preserves_source_aspect_across_viewports() {
        let displayable = VisualRenderStageDisplayable {
            layer_id: "characters".to_string(),
            tag: "kiki".to_string(),
            sprite: "vn.character.kiki.neutral".to_string(),
            placement: VisualStagePlacement::Center,
            layer_zorder: 10,
            zorder: 0,
        };
        let source = VisualImageSourceSize {
            width: 512.0,
            height: 1024.0,
        };
        let fullscreen_target =
            stage_displayable_target_rect(&displayable, euclid::rect(0.0, 0.0, 1920.0, 1080.0));
        let windowed_target =
            stage_displayable_target_rect(&displayable, euclid::rect(0.0, 0.0, 1000.0, 560.0));

        let fullscreen = resolve_aspect_rect(
            source,
            fullscreen_target,
            stage_displayable_scale_mode(&displayable),
        );
        let windowed = resolve_aspect_rect(
            source,
            windowed_target,
            stage_displayable_scale_mode(&displayable),
        );

        assert_approx_eq(fullscreen.size.width / fullscreen.size.height, 0.5);
        assert_approx_eq(windowed.size.width / windowed.size.height, 0.5);
        assert_approx_eq(fullscreen.max_y(), fullscreen_target.max_y());
        assert_approx_eq(windowed.max_y(), windowed_target.max_y());
    }

    #[test]
    fn staged_character_placeholder_uses_portrait_fit() {
        let displayable = VisualRenderStageDisplayable {
            layer_id: "characters".to_string(),
            tag: "kiki".to_string(),
            sprite: "missing-kiki".to_string(),
            placement: VisualStagePlacement::Center,
            layer_zorder: 10,
            zorder: 0,
        };
        let target =
            stage_displayable_target_rect(&displayable, euclid::rect(0.0, 0.0, 1000.0, 560.0));

        let rect = stage_displayable_placeholder_rect(&displayable, target);

        assert_approx_eq(rect.size.width / rect.size.height, 0.5);
        assert_approx_eq(rect.max_y(), target.max_y());
        assert!(rect.size.width <= target.size.width);
        assert!(rect.size.height <= target.size.height);
    }

    #[test]
    fn fullscreen_background_uses_fill_policy() {
        let displayable = VisualRenderStageDisplayable {
            layer_id: "background".to_string(),
            tag: "background".to_string(),
            sprite: "vn.background.school_classroom".to_string(),
            placement: VisualStagePlacement::Fullscreen,
            layer_zorder: 0,
            zorder: 0,
        };
        let target =
            stage_displayable_target_rect(&displayable, euclid::rect(0.0, 0.0, 1000.0, 560.0));
        let source = VisualImageSourceSize {
            width: 1920.0,
            height: 1080.0,
        };

        let rect = resolve_aspect_rect(source, target, stage_displayable_scale_mode(&displayable));

        assert_eq!(
            stage_displayable_scale_mode(&displayable),
            VisualImageScaleMode::FillCenter
        );
        assert_approx_eq(rect.size.width / rect.size.height, 16.0 / 9.0);
        assert!(rect.size.width + 0.001 >= target.size.width);
        assert!(rect.size.height + 0.001 >= target.size.height);
    }

    #[test]
    fn fullscreen_background_placeholder_uses_cover_policy() {
        let displayable = VisualRenderStageDisplayable {
            layer_id: "background".to_string(),
            tag: "background".to_string(),
            sprite: "missing-background".to_string(),
            placement: VisualStagePlacement::Fullscreen,
            layer_zorder: 0,
            zorder: 0,
        };
        let target =
            stage_displayable_target_rect(&displayable, euclid::rect(0.0, 0.0, 1000.0, 560.0));

        let rect = stage_displayable_placeholder_rect(&displayable, target);

        assert_approx_eq(rect.size.width / rect.size.height, 16.0 / 9.0);
        assert!(rect.size.width + 0.001 >= target.size.width);
        assert!(rect.size.height + 0.001 >= target.size.height);
    }

    #[test]
    fn vn_panel_rects_include_dialogue_and_dock_for_large_viewports() {
        let stage = euclid::rect(0.0, 0.0, 1000.0, 800.0);
        let layout = vn_overlay_layout(100, 30, "Codex", "Composer");
        let rects = vn_panel_rects(&layout, stage, 10.0, 20.0);

        assert_eq!(rects.len(), 2);
        let dock = rects[0];
        let dialogue = rects[1];
        assert!(dialogue.min_y() < dock.min_y());
        assert!(dialogue.max_y() < dock.min_y());
        assert_eq!(dock.min_x(), 20.0);
        assert_eq!(dialogue.min_x(), 30.0);
        assert!(dock.size.width > dialogue.size.width);
    }

    #[test]
    fn vn_panel_rects_use_shared_fullscreen_proportions() {
        let stage = euclid::rect(0.0, 0.0, 1920.0, 1080.0);
        let layout = vn_overlay_layout(240, 60, "Codex", "Composer");
        let rects = vn_panel_rects(&layout, stage, 8.0, 18.0);

        assert_eq!(rects.len(), 2);
        let dialogue = rects[1];
        assert!((dialogue.min_x() - 336.0).abs() < 0.1);
        assert!((dialogue.min_y() - 72.0).abs() < 0.1);
        assert!((dialogue.size.width - 1248.0).abs() < 0.1);
        assert!((dialogue.max_y() - 720.0).abs() < 0.1);
    }

    #[test]
    fn vn_panel_nameplate_rects_attach_to_dialogue_and_dock() {
        let stage = euclid::rect(0.0, 0.0, 1920.0, 1080.0);
        let layout = vn_overlay_layout(240, 60, "Codex", "Composer");
        let rects = vn_panel_rects(&layout, stage, 8.0, 18.0);
        let nameplates = vn_panel_nameplate_rects(&layout, stage, 8.0, 18.0);

        assert_eq!(nameplates.len(), 2);
        for (nameplate, panel) in nameplates.iter().zip(rects.iter()) {
            assert!(nameplate.min_x() > panel.min_x());
            assert!(nameplate.max_x() < panel.max_x());
            assert_eq!(nameplate.max_y(), panel.min_y());
        }
        let expected_gap_px = 0.0;
        assert!((nameplates[0].max_y() - rects[0].min_y() - expected_gap_px).abs() < 0.1);
        assert!((nameplates[1].max_y() - rects[1].min_y() - expected_gap_px).abs() < 0.1);
        assert!(nameplates[1].min_y() > stage.min_y());
    }

    #[test]
    fn vn_panel_nameplate_and_panel_layout_uses_overlay_snapshot_rows() {
        let runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
        let mut snapshot = runtime.render_snapshot();
        snapshot.overlay_cols = Some(120);
        snapshot.overlay_rows = Some(24);

        let (cols, rows) = vn_overlay_layout_dims(Some(&snapshot), 120, 24);
        let layout = vn_overlay_layout(cols, rows, &snapshot.dialogue_speaker, "Composer");

        assert_eq!(cols, 120);
        assert_eq!(rows, 24);
        assert_eq!(layout.composer_panel.unwrap().row, 18);
        assert_eq!(layout.composer_panel.unwrap().height, 4);
    }

    #[test]
    fn vn_dialogue_scrollbar_rects_track_overflow_metrics() {
        let runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
        let mut snapshot = runtime.render_snapshot();
        snapshot.vn_dialogue_scroll = Some(gameterm_visual::VnDialogueScrollMetrics {
            total_lines: 40,
            visible_rows: 10,
            scroll_offset: 0,
            max_scroll_offset: 30,
        });
        let stage = euclid::rect(0.0, 0.0, 1000.0, 600.0);
        let layout = vn_overlay_layout(100, 30, "Codex", "Composer");

        let (track, bottom_thumb) =
            vn_dialogue_scrollbar_rects(&layout, &snapshot, stage, 10.0, 20.0).unwrap();

        assert!(
            track.min_x()
                > vn_overlay_rect_to_pixels(layout.dialogue_panel, stage, 10.0, 20.0).min_x()
        );
        assert_eq!(track.size.height, 200.0);
        assert!(bottom_thumb.min_y() > track.min_y());
        assert_eq!(bottom_thumb.max_y(), track.max_y());

        snapshot.vn_dialogue_scroll = Some(gameterm_visual::VnDialogueScrollMetrics {
            total_lines: 40,
            visible_rows: 10,
            scroll_offset: 30,
            max_scroll_offset: 30,
        });
        let (_, top_thumb) =
            vn_dialogue_scrollbar_rects(&layout, &snapshot, stage, 10.0, 20.0).unwrap();
        assert_eq!(top_thumb.min_y(), track.min_y());
        assert!(top_thumb.max_y() < bottom_thumb.min_y());
    }

    #[test]
    fn vn_dialogue_scrollbar_rects_skip_non_overflowing_dialogue() {
        let runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
        let mut snapshot = runtime.render_snapshot();
        snapshot.vn_dialogue_scroll = Some(gameterm_visual::VnDialogueScrollMetrics {
            total_lines: 3,
            visible_rows: 10,
            scroll_offset: 0,
            max_scroll_offset: 0,
        });
        let layout = vn_overlay_layout(100, 30, "Codex", "Composer");

        assert!(vn_dialogue_scrollbar_rects(
            &layout,
            &snapshot,
            euclid::rect(0.0, 0.0, 1000.0, 600.0),
            10.0,
            20.0,
        )
        .is_none());
    }

    #[test]
    fn rounded_panel_rects_approximate_corner_radius() {
        let rect = euclid::rect(10.0, 20.0, 100.0, 40.0);
        let rects = rounded_panel_rects(rect, 8.0);

        assert_eq!(rects.len(), 25);
        assert_eq!(rects[0].min_y(), 28.0);
        assert_eq!(rects[0].size.height, 24.0);
        assert!(rects[1].min_x() > rect.min_x());
        assert!(rects[1].size.width < rect.size.width);
        assert!(rects[23].min_x() < rects[1].min_x());
        assert!(rects[23].size.width > rects[1].size.width);
    }

    #[test]
    fn rounded_panel_rects_increase_detail_for_larger_radius() {
        let rect = euclid::rect(0.0, 0.0, 200.0, 100.0);

        assert!(rounded_panel_rects(rect, 28.0).len() > rounded_panel_rects(rect, 8.0).len());
        assert_eq!(
            rounded_panel_corner_segments(4.0),
            VN_PANEL_MIN_CORNER_SEGMENTS
        );
        assert_eq!(
            rounded_panel_corner_segments(80.0),
            VN_PANEL_MAX_CORNER_SEGMENTS
        );
    }

    #[test]
    fn rounded_panel_rects_preserve_bounds_and_clamp_radius() {
        let rect = euclid::rect(5.0, 7.0, 40.0, 16.0);
        let rects = rounded_panel_rects(rect, 80.0);

        for panel_rect in rects {
            assert!(panel_rect.min_x() >= rect.min_x());
            assert!(panel_rect.min_y() >= rect.min_y());
            assert!(panel_rect.max_x() <= rect.max_x());
            assert!(panel_rect.max_y() <= rect.max_y());
            assert!(panel_rect.size.width > 0.0);
            assert!(panel_rect.size.height > 0.0);
        }
    }

    #[test]
    fn rounded_rect_primitive_derives_inner_shape_from_style() {
        let rect = euclid::rect(4.0, 6.0, 120.0, 48.0);
        let primitive = RoundedRectPrimitive::new(rect, VnPanelStyle::dialogue_panel());

        assert_eq!(primitive.rect, rect);
        assert_eq!(primitive.fill, VN_PANEL_FILL);
        assert_eq!(primitive.border, VN_PANEL_BORDER);
        assert_eq!(primitive.border_width, VN_PANEL_BORDER_WIDTH_PX);
        assert_eq!(primitive.radius, VN_DIALOGUE_PANEL_RADIUS_PX);
        assert_eq!(
            primitive.inner_rect(),
            inset_rect(rect, VN_PANEL_BORDER_WIDTH_PX)
        );
        assert_eq!(
            primitive.inner_radius(),
            VN_DIALOGUE_PANEL_RADIUS_PX - VN_PANEL_BORDER_WIDTH_PX
        );

        let small = RoundedRectPrimitive::new(
            euclid::rect(0.0, 0.0, 12.0, 8.0),
            VnPanelStyle::dialogue_panel(),
        );
        assert_eq!(small.radius, 4.0);
    }

    #[test]
    fn vn_panel_styles_use_extracted_procedural_values() {
        let panel = VnPanelStyle::dialogue_panel();
        let composer = VnPanelStyle::composer_panel();
        let nameplate = VnPanelStyle::dialogue_nameplate();
        let composer_nameplate = VnPanelStyle::composer_nameplate();

        assert_eq!(panel.fill, LinearRgba(0.102, 0.1137, 0.1333, 0.4627));
        assert_eq!(panel.border, LinearRgba(0.1608, 0.1725, 0.1961, 0.3608));
        assert_eq!(panel.border_width, 1.5);
        assert_eq!(panel.radius, VN_DIALOGUE_PANEL_RADIUS_PX);
        assert_eq!(composer.radius, VN_COMPOSER_PANEL_RADIUS_PX);
        assert_eq!(nameplate.radius, VN_DIALOGUE_NAMEPLATE_RADIUS_PX);
        assert_eq!(composer_nameplate.radius, VN_COMPOSER_NAMEPLATE_RADIUS_PX);
        assert!(panel.radius > composer.radius);
        assert!(composer.radius > nameplate.radius);
        assert!(nameplate.radius > composer_nameplate.radius);
        assert_eq!(nameplate.fill.0, panel.fill.0);
        assert_eq!(nameplate.fill.1, panel.fill.1);
        assert_eq!(nameplate.fill.2, panel.fill.2);
        assert!(nameplate.fill.3 > panel.fill.3);
        assert!(nameplate.border.3 > panel.border.3);
    }

    #[test]
    fn nine_slice_rects_preserve_corners_and_stretch_center() {
        let rect = euclid::rect(10.0, 20.0, 100.0, 60.0);
        let rects = nine_slice_rects(rect, 12.0);

        assert_eq!(rects.len(), 9);
        assert_eq!(rects[0], euclid::rect(10.0, 20.0, 12.0, 12.0));
        assert_eq!(rects[2], euclid::rect(98.0, 20.0, 12.0, 12.0));
        assert_eq!(rects[4], euclid::rect(22.0, 32.0, 76.0, 36.0));
        assert_eq!(rects[8], euclid::rect(98.0, 68.0, 12.0, 12.0));
    }

    #[test]
    fn nine_slice_rects_clamp_margin_for_small_panels() {
        let rect = euclid::rect(0.0, 0.0, 20.0, 10.0);
        let rects = nine_slice_rects(rect, 12.0);

        assert_eq!(rects.len(), 9);
        for rect in rects {
            assert!(rect.size.width >= 0.0);
            assert!(rect.size.height >= 0.0);
        }
    }
}
