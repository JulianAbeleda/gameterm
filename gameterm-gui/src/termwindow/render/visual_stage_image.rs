use crate::termwindow::render::paint::AllowImage;
use ::window::RectF;
use gameterm_visual::{VisualRenderStageDisplayable, VisualStagePlacement};
use std::sync::Arc;
use termwiz::image::ImageData;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum VisualImageScaleMode {
    Stretch,
    #[allow(dead_code)]
    FitCenter,
    FitBottomCenter,
    FillCenter,
    #[allow(dead_code)]
    IntegerFitCenter,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct VisualImageSourceSize {
    pub(super) width: f32,
    pub(super) height: f32,
}

pub(super) fn resolve_aspect_rect(
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

pub(super) fn stage_displayable_target_rect(
    displayable: &VisualRenderStageDisplayable,
    stage_rect: RectF,
) -> RectF {
    match displayable.placement {
        VisualStagePlacement::Fullscreen => stage_rect,
        VisualStagePlacement::Left | VisualStagePlacement::Center | VisualStagePlacement::Right => {
            let height = (stage_rect.size.height
                * super::visual_vn_panel::VN_STAGE_CHARACTER_HEIGHT_RATIO)
                .max(1.0);
            let width = (stage_rect.size.width
                * super::visual_vn_panel::VN_STAGE_CHARACTER_TARGET_WIDTH_RATIO)
                .max(1.0);
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

pub(super) fn stage_displayable_scale_mode(
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

pub(super) fn stage_displayable_placeholder_rect(
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

pub(super) fn visual_sprite_image_data<'a>(
    sprite_id: &str,
    allow_images: AllowImage,
    visual_sprites: Option<&'a crate::termwindow::render::VisualSpriteImages>,
) -> Option<&'a Arc<ImageData>> {
    if allow_images == AllowImage::No {
        return None;
    }

    visual_sprites.and_then(|sprites| sprites.sprites.get(sprite_id))
}
