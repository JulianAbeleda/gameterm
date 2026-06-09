use super::io::{load_rgba_image, save_rgba_image};
use super::{
    apply_operation, content_bounds, inspect_scene_asset_image, rounded_f32,
    validate_scene_asset_feature_map, SceneAssetAnimationOutput, SceneAssetContinuityCheck,
    SceneAssetContinuityReport, SceneAssetContinuityStatus, SceneAssetDimensions,
    SceneAssetEditError, SceneAssetExportReport, SceneAssetExpressionOutput, SceneAssetFeatureMap,
    SceneAssetPixelRect, SceneAssetRecipeBook,
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
