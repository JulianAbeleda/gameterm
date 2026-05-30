use crate::quad::{QuadTrait, TripleLayerQuadAllocator, TripleLayerQuadAllocatorTrait};
use crate::termwindow::render::paint::AllowImage;
use crate::termwindow::render::RenderScreenLineParams;
use crate::termwindow::TermWindow;
use ::window::RectF;
use anyhow::Context;
use config::HsbTransform;
use gameterm_visual::{VisualRenderEntity, VisualRenderTile};
use std::sync::Arc;
use termwiz::color::LinearRgba;
use termwiz::image::ImageData;

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
}
