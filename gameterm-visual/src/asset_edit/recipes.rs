use super::io::{load_rgba_image, save_rgba_image};
use super::mask::SceneAssetMask;
use super::pixels::{
    composite_scaled, draw_ellipse, draw_line_in_region, erase_region, fill_region, multiply_alpha,
    normalized_point_to_pixel, parse_rgba, scale_region, tint_region, translate_region,
};
use super::roots::resolve_recipe_path;
use super::selection_ops::{
    apply_mask_polish, apply_transparency_mask, background_magic_mask, channel_matte_mask,
    color_range_mask, contiguous_magic_mask, decontaminate_light_edges, defringe_scene_asset_edges,
    global_magic_mask, multi_seed_contiguous_mask, polished_background_mask,
    restore_pixels_from_source_image,
};
use super::{
    content_bounds, inspect_scene_asset_image, rounded_f32, validate_scene_asset_feature_map,
    SceneAssetAnimationOutput, SceneAssetBackgroundSample, SceneAssetContinuityCheck,
    SceneAssetContinuityReport, SceneAssetContinuityStatus, SceneAssetDimensions,
    SceneAssetEditError, SceneAssetEditOperation, SceneAssetExportReport,
    SceneAssetExpressionOutput, SceneAssetFeatureMap, SceneAssetHairCleanupMode,
    SceneAssetMaskPolishOptions, SceneAssetPixelRect, SceneAssetRecipeBook,
    SceneAssetRestoreOptions,
};
use image::RgbaImage;
use std::path::{Path, PathBuf};

pub fn generate_scene_asset_expression(
    base_path: &Path,
    feature_map: &SceneAssetFeatureMap,
    recipe_book: &SceneAssetRecipeBook,
    expression: &str,
    recipe_base_dir: Option<&Path>,
    output_path: &Path,
    force: bool,
) -> Result<SceneAssetExpressionOutput, SceneAssetEditError> {
    if output_path.exists() && !force {
        return Err(SceneAssetEditError::OutputExists(
            output_path.display().to_string(),
        ));
    }
    let operations = recipe_book
        .expressions
        .get(expression)
        .ok_or_else(|| SceneAssetEditError::UnknownExpression(expression.to_string()))?;
    let mut image = load_rgba_image(base_path)?;
    validate_scene_asset_feature_map(feature_map, image.width(), image.height())?;
    for operation in operations {
        apply_operation(&mut image, feature_map, operation, recipe_base_dir)?;
    }
    save_rgba_image(&image, output_path, force)?;
    Ok(SceneAssetExpressionOutput {
        expression: expression.to_string(),
        output_path: output_path.display().to_string(),
        report: inspect_scene_asset_image(output_path)?,
    })
}

pub fn generate_scene_asset_animation(
    base_path: &Path,
    feature_map: &SceneAssetFeatureMap,
    recipe_book: &SceneAssetRecipeBook,
    animation: &str,
    recipe_base_dir: Option<&Path>,
    output_dir: &Path,
    character: &str,
    force: bool,
) -> Result<SceneAssetAnimationOutput, SceneAssetEditError> {
    let recipe = recipe_book
        .animations
        .get(animation)
        .ok_or_else(|| SceneAssetEditError::UnknownAnimation(animation.to_string()))?;
    if let Err(err) = std::fs::create_dir_all(output_dir) {
        return Err(SceneAssetEditError::ImageFile {
            path: output_dir.display().to_string(),
            message: err.to_string(),
        });
    }
    let mut frames = Vec::new();
    for (index, frame) in recipe.frames.iter().enumerate() {
        let output_file = frame
            .output
            .clone()
            .unwrap_or_else(|| format!("{character}-{animation}-{index}.png"));
        let output_path = output_dir.join(output_file);
        frames.push(generate_scene_asset_expression(
            base_path,
            feature_map,
            recipe_book,
            &frame.expression,
            recipe_base_dir,
            &output_path,
            force,
        )?);
    }
    Ok(SceneAssetAnimationOutput {
        animation: animation.to_string(),
        frames,
    })
}

pub fn export_scene_asset_source_images(
    source: &Path,
    output_source_root: &Path,
    source_id: &str,
    character: &str,
    expressions: &[String],
    force: bool,
) -> Result<SceneAssetExportReport, SceneAssetEditError> {
    if expressions.is_empty() {
        return Err(SceneAssetEditError::InvalidOperation(
            "at least one expression is required".to_string(),
        ));
    }
    let image = load_rgba_image(source)?;
    let target_dir = output_source_root.join(source_id);
    if let Err(err) = std::fs::create_dir_all(&target_dir) {
        return Err(SceneAssetEditError::ImageFile {
            path: target_dir.display().to_string(),
            message: err.to_string(),
        });
    }
    let mut outputs = Vec::new();
    for expression in expressions {
        if expression.trim().is_empty() {
            return Err(SceneAssetEditError::InvalidOperation(
                "expression names must not be empty".to_string(),
            ));
        }
        let output = target_dir.join(format!("{character}-{expression}.png"));
        save_rgba_image(&image, &output, force)?;
        outputs.push(output.display().to_string());
    }
    Ok(SceneAssetExportReport {
        source: source.display().to_string(),
        source_id: source_id.to_string(),
        character: character.to_string(),
        outputs,
    })
}

pub fn continuity_report_for_scene_asset_frames(
    frame_paths: &[PathBuf],
    drift_tolerance_px: u32,
) -> Result<SceneAssetContinuityReport, SceneAssetEditError> {
    let mut checks = Vec::new();
    let mut warnings = Vec::new();
    let mut dimensions: Option<SceneAssetDimensions> = None;
    let mut content_rects = Vec::new();
    let mut images = Vec::new();

    for path in frame_paths {
        let image = load_rgba_image(path)?;
        let dims = SceneAssetDimensions {
            width: image.width(),
            height: image.height(),
        };
        if let Some(expected) = dimensions {
            if expected != dims {
                checks.push(SceneAssetContinuityCheck {
                    name: "dimensions".to_string(),
                    status: SceneAssetContinuityStatus::Fail,
                    detail: format!(
                        "{} is {}x{}, expected {}x{}",
                        path.display(),
                        dims.width,
                        dims.height,
                        expected.width,
                        expected.height
                    ),
                });
            }
        } else {
            dimensions = Some(dims);
        }
        content_rects.push(content_bounds(&image));
        images.push(image);
    }

    if !checks
        .iter()
        .any(|check| check.name == "dimensions" && check.status == SceneAssetContinuityStatus::Fail)
    {
        checks.push(SceneAssetContinuityCheck {
            name: "dimensions".to_string(),
            status: SceneAssetContinuityStatus::Pass,
            detail: "all frames have matching dimensions".to_string(),
        });
    }

    let drift_status = continuity_drift_status(&content_rects, drift_tolerance_px);
    checks.push(drift_status);

    for index in 1..images.len() {
        let changed_ratio = changed_pixel_ratio(&images[index - 1], &images[index]);
        if changed_ratio == 0.0 {
            warnings.push(format!(
                "frame {} and {} are pixel-identical",
                index - 1,
                index
            ));
        } else if changed_ratio > 0.25 {
            warnings.push(format!(
                "frame {} and {} changed {:.3} of pixels; check continuity",
                index - 1,
                index,
                changed_ratio
            ));
        }
    }

    Ok(SceneAssetContinuityReport {
        frame_count: frame_paths.len(),
        dimensions,
        content_bounds: content_rects,
        checks,
        warnings,
    })
}

fn continuity_drift_status(
    rects: &[Option<SceneAssetPixelRect>],
    drift_tolerance_px: u32,
) -> SceneAssetContinuityCheck {
    let Some(first) = rects.iter().flatten().next().copied() else {
        return SceneAssetContinuityCheck {
            name: "content_bounds".to_string(),
            status: SceneAssetContinuityStatus::Warning,
            detail: "no visible content in any frame".to_string(),
        };
    };
    let mut max_drift = 0;
    for rect in rects.iter().flatten() {
        max_drift = max_drift
            .max(first.x.abs_diff(rect.x))
            .max(first.y.abs_diff(rect.y))
            .max(first.right().abs_diff(rect.right()))
            .max(first.bottom().abs_diff(rect.bottom()));
    }
    let status = if max_drift <= drift_tolerance_px {
        SceneAssetContinuityStatus::Pass
    } else {
        SceneAssetContinuityStatus::Warning
    };
    SceneAssetContinuityCheck {
        name: "content_bounds".to_string(),
        status,
        detail: format!("max drift {max_drift}px, tolerance {drift_tolerance_px}px"),
    }
}

fn changed_pixel_ratio(a: &RgbaImage, b: &RgbaImage) -> f32 {
    if a.dimensions() != b.dimensions() {
        return 1.0;
    }
    let changed = a
        .pixels()
        .zip(b.pixels())
        .filter(|(a, b)| a.0 != b.0)
        .count();
    rounded_f32(changed as f32 / (a.width() as usize * a.height() as usize).max(1) as f32)
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

fn default_magic_tolerance() -> u8 {
    24
}

fn default_background_sample() -> SceneAssetBackgroundSample {
    SceneAssetBackgroundSample::Corners
}
