use image::RgbaImage;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;

mod composite;
mod io;
mod mask;
mod model;
mod operation_support;
mod paint_ops;
mod pipeline_args;
mod pixels;
mod recipes;
mod review;
mod roots;
mod runner;
mod selection_ops;
mod transform_ops;

pub use composite::{
    composite_scene_asset_layers, create_scene_asset_state_manifest,
    load_scene_asset_state_manifest, render_scene_asset_state, render_scene_asset_state_sheet,
};
use io::{load_json, load_rgba_image, read_file, sha256_hex, write_json};
use mask::SceneAssetMask;
pub use model::*;
pub use operation_support::scene_asset_operation_error_report;
pub use paint_ops::{
    alpha_paint_scene_asset_region, clone_stamp_scene_asset_region, draw_scene_asset_shape,
    fill_scene_asset_region, preview_scene_asset_grid, report_scene_asset_points,
    sample_fill_scene_asset_region, sample_scene_asset_image, stroke_scene_asset_path,
};
use pixels::{
    composite_scaled, draw_ellipse, draw_line_in_region, erase_region, fill_region, multiply_alpha,
    normalized_point_to_pixel, parse_rgba, scale_region, tint_region, translate_region,
};
pub use recipes::{
    continuity_report_for_scene_asset_frames, export_scene_asset_source_images,
    generate_scene_asset_animation, generate_scene_asset_expression,
};
pub use review::write_scene_asset_review_preview;
use roots::resolve_recipe_path;
pub use runner::{
    accept_scene_asset_output, compare_scene_asset_images, run_scene_asset_edit_session,
    run_scene_asset_operation, run_scene_asset_pipeline, validate_scene_asset_operation,
};
use selection_ops::{
    apply_mask_polish, apply_transparency_mask, background_magic_mask, channel_matte_mask,
    color_range_mask, contiguous_magic_mask, decontaminate_light_edges, defringe_scene_asset_edges,
    global_magic_mask, multi_seed_contiguous_mask, polished_background_mask,
    restore_pixels_from_source_image,
};
pub use selection_ops::{
    apply_scene_asset_mask_alpha, channel_matte_erase_scene_asset_image,
    cleanup_scene_asset_hair_edges, color_range_erase_scene_asset_image,
    composite_scene_asset_mask, export_scene_asset_selection_mask,
    magic_erase_add_scene_asset_image, magic_erase_scene_asset_image,
    make_scene_asset_background_transparent, make_scene_asset_background_transparent_polished,
    preview_scene_asset_selection_mask, restore_scene_asset_from_source,
};
pub use transform_ops::{
    blur_scene_asset_image, brightness_contrast_scene_asset_image, crop_scene_asset_image,
    hsl_scene_asset_image, levels_scene_asset_image, pad_scene_asset_image,
    transform_scene_asset_image, unsharp_mask_scene_asset_image,
};

pub fn inspect_scene_asset_image(
    path: &Path,
) -> Result<SceneAssetImageReport, SceneAssetEditError> {
    let bytes = read_file(path, "image")?;
    let format = image::guess_format(&bytes)
        .map(|format| format!("{format:?}"))
        .unwrap_or_else(|_| "unknown".to_string());
    let image = image::load_from_memory(&bytes).map_err(|err| SceneAssetEditError::ImageFile {
        path: path.display().to_string(),
        message: err.to_string(),
    })?;
    let color_type = format!("{:?}", image.color());
    let has_alpha = image.color().has_alpha();
    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();
    let transparent_count = rgba.pixels().filter(|pixel| pixel[3] == 0).count();
    let pixel_count = (width as usize).saturating_mul(height as usize).max(1);
    Ok(SceneAssetImageReport {
        asset_document_version: 1,
        source: path.display().to_string(),
        width,
        height,
        color_type: format!("{format}:{color_type}"),
        has_alpha,
        transparent_pixel_ratio: rounded_f32(transparent_count as f32 / pixel_count as f32),
        content_bounds: content_bounds(&rgba),
        sha256: sha256_hex(&bytes),
    })
}

pub fn default_scene_asset_feature_map(
    image_path: &Path,
    character: &str,
    base: Option<String>,
) -> Result<SceneAssetFeatureMap, SceneAssetEditError> {
    let report = inspect_scene_asset_image(image_path)?;
    let content = report.content_bounds.unwrap_or(SceneAssetPixelRect {
        x: 0,
        y: 0,
        w: report.width,
        h: report.height,
    });
    let mut regions = BTreeMap::new();
    let mut anchors = BTreeMap::new();

    let fx = |value: f32| rounded_f32((content.x as f32 / report.width.max(1) as f32) + value);
    let fy = |value: f32| rounded_f32((content.y as f32 / report.height.max(1) as f32) + value);
    let fw = |value: f32| rounded_f32(value);
    let fh = |value: f32| rounded_f32(value);

    regions.insert(
        "left_eye".to_string(),
        SceneAssetNormalizedRect {
            x: fx(0.34),
            y: fy(0.25),
            w: fw(0.11),
            h: fh(0.07),
        },
    );
    regions.insert(
        "right_eye".to_string(),
        SceneAssetNormalizedRect {
            x: fx(0.53),
            y: fy(0.25),
            w: fw(0.11),
            h: fh(0.07),
        },
    );
    regions.insert(
        "brows".to_string(),
        SceneAssetNormalizedRect {
            x: fx(0.32),
            y: fy(0.21),
            w: fw(0.36),
            h: fh(0.06),
        },
    );
    regions.insert(
        "mouth".to_string(),
        SceneAssetNormalizedRect {
            x: fx(0.45),
            y: fy(0.43),
            w: fw(0.10),
            h: fh(0.05),
        },
    );
    regions.insert(
        "torso".to_string(),
        SceneAssetNormalizedRect {
            x: fx(0.25),
            y: fy(0.56),
            w: fw(0.50),
            h: fh(0.36),
        },
    );
    anchors.insert(
        "head_center".to_string(),
        SceneAssetNormalizedPoint {
            x: fx(0.50),
            y: fy(0.33),
        },
    );
    anchors.insert(
        "mouth_center".to_string(),
        SceneAssetNormalizedPoint {
            x: fx(0.50),
            y: fy(0.455),
        },
    );
    anchors.insert(
        "feet_bottom".to_string(),
        SceneAssetNormalizedPoint {
            x: fx(0.50),
            y: fy(0.99),
        },
    );

    Ok(SceneAssetFeatureMap {
        feature_map_version: 1,
        character: character.to_string(),
        base: base.unwrap_or_else(|| image_path.display().to_string()),
        regions,
        anchors,
    })
}

pub fn validate_scene_asset_feature_map(
    map: &SceneAssetFeatureMap,
    image_width: u32,
    image_height: u32,
) -> Result<(), SceneAssetEditError> {
    if map.feature_map_version != 1 {
        return Err(SceneAssetEditError::InvalidFeatureMap(format!(
            "unsupported feature_map_version {}",
            map.feature_map_version
        )));
    }
    if map.character.trim().is_empty() {
        return Err(SceneAssetEditError::InvalidFeatureMap(
            "character must not be empty".to_string(),
        ));
    }
    if map.base.trim().is_empty() {
        return Err(SceneAssetEditError::InvalidFeatureMap(
            "base must not be empty".to_string(),
        ));
    }
    for (name, rect) in &map.regions {
        if name.trim().is_empty() {
            return Err(SceneAssetEditError::InvalidFeatureMap(
                "region name must not be empty".to_string(),
            ));
        }
        validate_normalized_rect(name, *rect)?;
        let pixel_rect = rect.to_pixels(image_width, image_height);
        if pixel_rect.w == 0 || pixel_rect.h == 0 {
            return Err(SceneAssetEditError::InvalidFeatureMap(format!(
                "region `{name}` maps to an empty pixel rectangle"
            )));
        }
    }
    for (name, point) in &map.anchors {
        if name.trim().is_empty() {
            return Err(SceneAssetEditError::InvalidFeatureMap(
                "anchor name must not be empty".to_string(),
            ));
        }
        if !point.x.is_finite()
            || !point.y.is_finite()
            || point.x < 0.0
            || point.y < 0.0
            || point.x > 1.0
            || point.y > 1.0
        {
            return Err(SceneAssetEditError::InvalidFeatureMap(format!(
                "anchor `{name}` must be inside 0..1"
            )));
        }
    }
    Ok(())
}

pub fn load_scene_asset_feature_map(
    path: &Path,
) -> Result<SceneAssetFeatureMap, SceneAssetEditError> {
    load_json(path)
}

pub fn load_scene_asset_recipe_book(
    path: &Path,
) -> Result<SceneAssetRecipeBook, SceneAssetEditError> {
    load_json(path)
}

pub fn write_scene_asset_json(
    path: &Path,
    value: &impl Serialize,
    pretty: bool,
    force: bool,
) -> Result<(), SceneAssetEditError> {
    write_json(path, value, pretty, force)
}

fn apply_operation(
    image: &mut RgbaImage,
    feature_map: &SceneAssetFeatureMap,
    operation: &SceneAssetEditOperation,
    recipe_base_dir: Option<&Path>,
) -> Result<(), SceneAssetEditError> {
    match operation {
        SceneAssetEditOperation::EraseRegion { region, soften } => {
            let rect = feature_map.pixel_region(region, image.width(), image.height())?;
            erase_region(image, rect, *soften);
        }
        SceneAssetEditOperation::FillRegion { region, color } => {
            let rect = feature_map.pixel_region(region, image.width(), image.height())?;
            fill_region(image, rect, parse_rgba(color)?);
        }
        SceneAssetEditOperation::DrawLine {
            region,
            from,
            to,
            color,
            width,
        } => {
            let rect = feature_map.pixel_region(region, image.width(), image.height())?;
            draw_line_in_region(image, rect, *from, *to, parse_rgba(color)?, *width);
        }
        SceneAssetEditOperation::DrawPolyline {
            region,
            points,
            color,
            width,
        } => {
            let rect = feature_map.pixel_region(region, image.width(), image.height())?;
            for pair in points.windows(2) {
                draw_line_in_region(image, rect, pair[0], pair[1], parse_rgba(color)?, *width);
            }
        }
        SceneAssetEditOperation::DrawEllipse {
            region,
            stroke,
            fill,
            width,
        } => {
            let rect = feature_map.pixel_region(region, image.width(), image.height())?;
            draw_ellipse(
                image,
                rect,
                stroke.as_deref().map(parse_rgba).transpose()?,
                fill.as_deref().map(parse_rgba).transpose()?,
                *width,
            );
        }
        SceneAssetEditOperation::CompositePng {
            region,
            path,
            opacity,
        } => {
            let rect = feature_map.pixel_region(region, image.width(), image.height())?;
            let overlay_path = resolve_recipe_path(path, recipe_base_dir);
            let overlay = load_rgba_image(&overlay_path)?;
            composite_scaled(image, rect, &overlay, *opacity);
        }
        SceneAssetEditOperation::TranslateRegion { region, dx, dy } => {
            let rect = feature_map.pixel_region(region, image.width(), image.height())?;
            translate_region(image, rect, *dx, *dy);
        }
        SceneAssetEditOperation::ScaleRegion { region, sx, sy } => {
            let rect = feature_map.pixel_region(region, image.width(), image.height())?;
            scale_region(image, rect, *sx, *sy)?;
        }
        SceneAssetEditOperation::Opacity { region, alpha } => {
            let rect = feature_map.pixel_region(region, image.width(), image.height())?;
            multiply_alpha(image, rect, *alpha);
        }
        SceneAssetEditOperation::ColorTint {
            region,
            color,
            amount,
        } => {
            let rect = feature_map.pixel_region(region, image.width(), image.height())?;
            tint_region(image, rect, parse_rgba(color)?, *amount);
        }
        SceneAssetEditOperation::MagicErase {
            seed,
            tolerance,
            contiguous,
            feather,
        } => {
            let seed = normalized_point_to_pixel(*seed, image.width(), image.height())?;
            let mask = if *contiguous {
                contiguous_magic_mask(image, &[seed], *tolerance)
            } else {
                global_magic_mask(image, *image.get_pixel(seed.0, seed.1), *tolerance)
            };
            apply_transparency_mask(image, &mask, *feather);
        }
        SceneAssetEditOperation::RemoveBackground {
            tolerance,
            feather,
            sample,
        } => {
            let mask = background_magic_mask(image, *tolerance, *sample);
            apply_transparency_mask(image, &mask, *feather);
        }
        SceneAssetEditOperation::RemoveBackgroundPolished {
            tolerance,
            feather,
            sample,
            erode,
            dilate,
            open,
            close,
            remove_small,
            fill_holes,
            defringe,
            protect_regions,
            within_regions,
            within_polygons,
        } => {
            let options = SceneAssetMaskPolishOptions {
                tolerance: *tolerance,
                feather: *feather,
                sample: *sample,
                erode: *erode,
                dilate: *dilate,
                open: *open,
                close: *close,
                remove_small: *remove_small,
                fill_holes: *fill_holes,
                defringe: *defringe,
                protect_regions: protect_regions.clone(),
                within_regions: within_regions.clone(),
                within_polygons: within_polygons.clone(),
            };
            let mask = polished_background_mask(image, &options, Some(feature_map))?;
            apply_transparency_mask(image, mask.pixels(), *feather);
            defringe_scene_asset_edges(image, *defringe);
        }
        SceneAssetEditOperation::ColorRangeErase {
            tolerance,
            feather,
            sample,
            erode,
            dilate,
            open,
            close,
            remove_small,
            fill_holes,
            defringe,
            protect_regions,
            within_regions,
            within_polygons,
        } => {
            let options = SceneAssetMaskPolishOptions {
                tolerance: *tolerance,
                feather: *feather,
                sample: *sample,
                erode: *erode,
                dilate: *dilate,
                open: *open,
                close: *close,
                remove_small: *remove_small,
                fill_holes: *fill_holes,
                defringe: *defringe,
                protect_regions: protect_regions.clone(),
                within_regions: within_regions.clone(),
                within_polygons: within_polygons.clone(),
            };
            let mask = apply_mask_polish(
                SceneAssetMask::from_pixels(
                    image.width(),
                    image.height(),
                    color_range_mask(image, *tolerance, *sample),
                ),
                &options,
                Some(feature_map),
            )?;
            apply_transparency_mask(image, mask.pixels(), *feather);
            defringe_scene_asset_edges(image, *defringe);
        }
        SceneAssetEditOperation::MagicEraseAdd {
            seeds,
            tolerance,
            feather,
            erode,
            dilate,
            open,
            close,
            remove_small,
            fill_holes,
            defringe,
            protect_regions,
            within_regions,
            within_polygons,
        } => {
            let options = SceneAssetMaskPolishOptions {
                tolerance: *tolerance,
                feather: *feather,
                sample: default_background_sample(),
                erode: *erode,
                dilate: *dilate,
                open: *open,
                close: *close,
                remove_small: *remove_small,
                fill_holes: *fill_holes,
                defringe: *defringe,
                protect_regions: protect_regions.clone(),
                within_regions: within_regions.clone(),
                within_polygons: within_polygons.clone(),
            };
            let mask = apply_mask_polish(
                multi_seed_contiguous_mask(image, seeds, *tolerance)?,
                &options,
                Some(feature_map),
            )?;
            apply_transparency_mask(image, mask.pixels(), *feather);
            defringe_scene_asset_edges(image, *defringe);
        }
        SceneAssetEditOperation::ChannelMatteErase {
            threshold,
            neutrality,
            feather,
            erode,
            dilate,
            open,
            close,
            remove_small,
            fill_holes,
            defringe,
            protect_regions,
            within_regions,
            within_polygons,
        } => {
            let options = SceneAssetMaskPolishOptions {
                tolerance: default_magic_tolerance(),
                feather: *feather,
                sample: default_background_sample(),
                erode: *erode,
                dilate: *dilate,
                open: *open,
                close: *close,
                remove_small: *remove_small,
                fill_holes: *fill_holes,
                defringe: *defringe,
                protect_regions: protect_regions.clone(),
                within_regions: within_regions.clone(),
                within_polygons: within_polygons.clone(),
            };
            let mask = apply_mask_polish(
                SceneAssetMask::from_pixels(
                    image.width(),
                    image.height(),
                    channel_matte_mask(image, *threshold, *neutrality),
                ),
                &options,
                Some(feature_map),
            )?;
            apply_transparency_mask(image, mask.pixels(), *feather);
            defringe_scene_asset_edges(image, *defringe);
        }
        SceneAssetEditOperation::HairCleanup {
            mode,
            radius,
            strength,
            region,
        } => {
            let pixel_region = region
                .as_deref()
                .map(|region| feature_map.pixel_region(region, image.width(), image.height()))
                .transpose()?;
            match mode {
                SceneAssetHairCleanupMode::Decontaminate => {
                    decontaminate_light_edges(image, *radius, *strength, pixel_region);
                }
            }
        }
        SceneAssetEditOperation::RestoreFromSource {
            path,
            regions,
            polygons,
            filter,
            tolerance,
            sample,
        } => {
            let source_path = resolve_recipe_path(path, recipe_base_dir);
            let source = load_rgba_image(&source_path)?;
            restore_pixels_from_source_image(
                &source,
                image,
                &SceneAssetRestoreOptions {
                    regions: regions.clone(),
                    polygons: polygons.clone(),
                    filter: *filter,
                    tolerance: *tolerance,
                    sample: *sample,
                },
                Some(feature_map),
            )?;
        }
    }
    Ok(())
}

fn validate_normalized_rect(
    name: &str,
    rect: SceneAssetNormalizedRect,
) -> Result<(), SceneAssetEditError> {
    if !rect.x.is_finite()
        || !rect.y.is_finite()
        || !rect.w.is_finite()
        || !rect.h.is_finite()
        || rect.x < 0.0
        || rect.y < 0.0
        || rect.w <= 0.0
        || rect.h <= 0.0
        || rect.x + rect.w > 1.0
        || rect.y + rect.h > 1.0
    {
        return Err(SceneAssetEditError::InvalidFeatureMap(format!(
            "region `{name}` must be finite and inside 0..1"
        )));
    }
    Ok(())
}

fn content_bounds(image: &RgbaImage) -> Option<SceneAssetPixelRect> {
    let mut min_x = image.width();
    let mut min_y = image.height();
    let mut max_x = 0;
    let mut max_y = 0;
    let mut found = false;
    for (x, y, pixel) in image.enumerate_pixels() {
        if pixel[3] == 0 {
            continue;
        }
        found = true;
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }
    found.then_some(SceneAssetPixelRect {
        x: min_x,
        y: min_y,
        w: max_x.saturating_sub(min_x).saturating_add(1),
        h: max_y.saturating_sub(min_y).saturating_add(1),
    })
}

fn default_magic_tolerance() -> u8 {
    24
}

fn default_background_sample() -> SceneAssetBackgroundSample {
    SceneAssetBackgroundSample::Corners
}

fn rounded_f32(value: f32) -> f32 {
    (value * 10_000.0).round() / 10_000.0
}

#[cfg(test)]
mod tests;
