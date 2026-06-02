use crate::quad::{QuadTrait, TripleLayerQuadAllocator, TripleLayerQuadAllocatorTrait};
use crate::termwindow::render::paint::AllowImage;
use crate::termwindow::render::RenderScreenLineParams;
use crate::termwindow::TermWindow;
use ::window::RectF;
use anyhow::Context;
use config::HsbTransform;
use gameterm_visual::{
    VisualRenderEntity, VisualRenderSnapshot, VisualRenderStageDisplayable, VisualRenderTile,
    VisualStagePlacement,
};
use std::sync::Arc;
use termwiz::color::LinearRgba;
use termwiz::image::ImageData;

const VN_PANEL_ALPHA: f32 = 0.62;
const VN_PANEL_BORDER_ALPHA: f32 = 0.30;
const VN_FULLSCREEN_PANEL_MIN_ROWS: usize = 40;
const VN_FULLSCREEN_PANEL_SIDE_MARGIN: f32 = 0.033;
const VN_FULLSCREEN_PANEL_TOP: f32 = 0.085;
const VN_FULLSCREEN_PANEL_BOTTOM: f32 = 0.896;

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

        let stage_rect = stage_viewport_rect(params, cell_height);
        for displayable in &snapshot.stage {
            let rect = stage_displayable_rect(displayable, stage_rect);
            let layer_num = match displayable.placement {
                VisualStagePlacement::Fullscreen => 0,
                VisualStagePlacement::Left
                | VisualStagePlacement::Center
                | VisualStagePlacement::Right => 1,
            };
            if self.populate_visual_sprite_quad(
                &displayable.sprite,
                layers,
                layer_num,
                rect,
                params,
                hsv,
            )? {
                continue;
            }
            let mut quad = self.filled_rectangle(
                layers,
                layer_num,
                rect,
                visual_placeholder_color(&displayable.sprite, 0.42, 0.24),
            )?;
            quad.set_hsv(hsv);
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
        for rect in vn_panel_rects(stage_rect, params.dims.viewport_rows, cell_width, cell_height) {
            self.populate_rounded_vn_panel(layers, 1, rect, cell_width, hsv)?;
        }
        Ok(())
    }

    fn populate_rounded_vn_panel(
        &self,
        layers: &mut TripleLayerQuadAllocator,
        layer_num: usize,
        rect: RectF,
        cell_width: f32,
        hsv: Option<HsbTransform>,
    ) -> anyhow::Result<()> {
        let radius = (cell_width * 1.4)
            .max(4.0)
            .min(rect.size.width / 3.0)
            .min(rect.size.height / 2.0);
        let border = 1.5;
        let border_color = LinearRgba::with_components(0.92, 0.96, 1.0, VN_PANEL_BORDER_ALPHA);
        for panel_rect in rounded_panel_rects(rect, radius) {
            let mut quad = self.filled_rectangle(layers, layer_num, panel_rect, border_color)?;
            quad.set_hsv(hsv);
        }

        let inner = inset_rect(rect, border);
        let inner_radius = (radius - border).max(1.0);
        let fill_color = LinearRgba::with_components(0.025, 0.028, 0.034, VN_PANEL_ALPHA);
        for panel_rect in rounded_panel_rects(inner, inner_radius) {
            let mut quad = self.filled_rectangle(layers, layer_num, panel_rect, fill_color)?;
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

fn vn_panel_rects(
    stage_rect: RectF,
    viewport_rows: usize,
    cell_width: f32,
    cell_height: f32,
) -> Vec<RectF> {
    let gap = (cell_height * 0.35).max(4.0);
    let dock_rows = if viewport_rows >= 10 { 2.0 } else { 0.0 };
    let fullscreen_vn_layout = viewport_rows >= VN_FULLSCREEN_PANEL_MIN_ROWS;
    let horizontal_margin = if fullscreen_vn_layout {
        (stage_rect.size.width * VN_FULLSCREEN_PANEL_SIDE_MARGIN).max(cell_width)
    } else {
        (cell_width * 2.0).max(12.0)
    };
    let mut rects = Vec::new();
    let panel_width = (stage_rect.size.width - horizontal_margin * 2.0).max(cell_width * 10.0);

    if dock_rows > 0.0 {
        let dock_height = (cell_height * dock_rows).max(cell_height);
        let dock_rect = euclid::rect(
            stage_rect.min_x() + horizontal_margin,
            stage_rect.max_y() - dock_height,
            panel_width,
            dock_height,
        );
        rects.push(dock_rect);
    }

    let (dialogue_top, dialogue_bottom) = if fullscreen_vn_layout {
        let top = stage_rect.min_y() + stage_rect.size.height * VN_FULLSCREEN_PANEL_TOP;
        let bottom = stage_rect.min_y() + stage_rect.size.height * VN_FULLSCREEN_PANEL_BOTTOM;
        (top, bottom)
    } else {
        let dialogue_rows = if viewport_rows >= 18 { 7.0 } else { 4.0 };
        let bottom = rects
            .first()
            .map(|dock_rect| dock_rect.min_y() - gap)
            .unwrap_or(stage_rect.max_y() - gap);
        let height = (cell_height * dialogue_rows - gap).max(cell_height * 3.0);
        (bottom - height, bottom)
    };
    let dialogue_height = (dialogue_bottom - dialogue_top).max(cell_height * 3.0);
    let dialogue_rect = euclid::rect(
        stage_rect.min_x() + horizontal_margin,
        dialogue_top,
        panel_width,
        dialogue_height,
    );
    rects.push(dialogue_rect);
    rects
}

fn rounded_panel_rects(rect: RectF, radius: f32) -> Vec<RectF> {
    let radius = radius
        .max(0.0)
        .min(rect.size.width / 2.0)
        .min(rect.size.height / 2.0);
    if radius <= 0.0 {
        return vec![rect];
    }
    vec![
        euclid::rect(
            rect.min_x() + radius,
            rect.min_y(),
            (rect.size.width - radius * 2.0).max(1.0),
            rect.size.height,
        ),
        euclid::rect(
            rect.min_x(),
            rect.min_y() + radius,
            rect.size.width,
            (rect.size.height - radius * 2.0).max(1.0),
        ),
    ]
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

fn stage_displayable_rect(
    displayable: &VisualRenderStageDisplayable,
    stage_rect: RectF,
) -> RectF {
    match displayable.placement {
        VisualStagePlacement::Fullscreen => stage_rect,
        VisualStagePlacement::Left | VisualStagePlacement::Center | VisualStagePlacement::Right => {
            let height = (stage_rect.size.height * 0.78).max(1.0);
            let width = height;
            let center_x = match displayable.placement {
                VisualStagePlacement::Left => stage_rect.min_x() + stage_rect.size.width * 0.28,
                VisualStagePlacement::Center => {
                    stage_rect.min_x() + stage_rect.size.width * 0.50
                }
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
    use std::collections::HashMap;

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

        assert_eq!(stage_displayable_rect(&displayable, rect), rect);
    }

    #[test]
    fn stage_displayable_rect_places_character_slots() {
        let stage = euclid::rect(0.0, 0.0, 1000.0, 800.0);
        let make_displayable = |placement| VisualRenderStageDisplayable {
            layer_id: "characters".to_string(),
            tag: "guide".to_string(),
            sprite: "vn.character.guide.neutral".to_string(),
            placement,
            layer_zorder: 10,
            zorder: 0,
        };

        let left = stage_displayable_rect(&make_displayable(VisualStagePlacement::Left), stage);
        let center =
            stage_displayable_rect(&make_displayable(VisualStagePlacement::Center), stage);
        let right = stage_displayable_rect(&make_displayable(VisualStagePlacement::Right), stage);

        assert_eq!(left.size.height, 624.0);
        assert_eq!(left.size.width, 624.0);
        assert_eq!(left.max_y(), stage.max_y());
        assert!(left.min_x() < center.min_x());
        assert!(center.min_x() < right.min_x());
    }

    #[test]
    fn vn_panel_rects_include_dialogue_and_dock_for_large_viewports() {
        let stage = euclid::rect(0.0, 0.0, 1000.0, 800.0);
        let rects = vn_panel_rects(stage, 30, 10.0, 20.0);

        assert_eq!(rects.len(), 2);
        let dock = rects[0];
        let dialogue = rects[1];
        assert!(dialogue.min_y() < dock.min_y());
        assert!(dialogue.max_y() < dock.min_y());
        assert_eq!(dock.min_x(), 20.0);
        assert_eq!(dialogue.min_x(), 20.0);
        assert_eq!(dock.size.width, dialogue.size.width);
    }

    #[test]
    fn vn_panel_rects_use_fate_style_fullscreen_proportions() {
        let stage = euclid::rect(0.0, 0.0, 1920.0, 1080.0);
        let rects = vn_panel_rects(stage, 60, 8.0, 18.0);

        assert_eq!(rects.len(), 2);
        let dialogue = rects[1];
        assert!((dialogue.min_x() - 63.36).abs() < 0.1);
        assert!((dialogue.min_y() - 91.8).abs() < 0.1);
        assert!((dialogue.size.width - 1793.28).abs() < 0.1);
        assert!((dialogue.max_y() - 967.68).abs() < 0.1);
    }

    #[test]
    fn rounded_panel_rects_leave_corner_cutouts() {
        let rect = euclid::rect(10.0, 20.0, 100.0, 40.0);
        let rects = rounded_panel_rects(rect, 8.0);

        assert_eq!(rects.len(), 2);
        assert_eq!(rects[0].min_x(), 18.0);
        assert_eq!(rects[0].size.width, 84.0);
        assert_eq!(rects[1].min_y(), 28.0);
        assert_eq!(rects[1].size.height, 24.0);
    }
}
