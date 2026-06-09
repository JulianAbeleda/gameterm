use image::{ImageBuffer, Rgba, RgbaImage};
use serde::Serialize;
use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};

mod composite;
mod io;
mod mask;
mod model;
mod operation_support;
mod pipeline_args;
mod pixels;
mod review;
mod roots;

pub use composite::{
    composite_scene_asset_layers, create_scene_asset_state_manifest,
    load_scene_asset_state_manifest, render_scene_asset_state, render_scene_asset_state_sheet,
};
use io::{
    load_json, load_rgba_image, read_file, save_rgba_image, sha256_hex, write_file, write_json,
};
use mask::{mask_index, validate_polygon, SceneAssetMask};
pub use model::*;
pub use operation_support::scene_asset_operation_error_report;
use operation_support::{compare_protected_regions, operation_expectation_failures};
use pipeline_args::*;
use pixels::{
    blend_pixel, changed_pixel_count, color_channels, composite_scaled, crop_region, draw_ellipse,
    draw_line_in_region, draw_normalized_line, draw_normalized_path, draw_rect_outline,
    erase_region, fill_region, lerp, multiply_alpha, normalized_point_to_pixel,
    normalized_rect_arg, parse_rgba, paste_region, pixel_index, pixel_len, scale_region,
    tint_region, translate_region,
};
pub use review::write_scene_asset_review_preview;
use review::{operation_preview_output, write_scene_asset_operation_review_previews};
use roots::{
    pipeline_report_path, resolve_asset_accept_output_path, resolve_asset_accept_source_path,
    resolve_asset_operation_source_path, resolve_pipeline_input_path, resolve_pipeline_output_path,
    resolve_recipe_path,
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

pub fn run_scene_asset_pipeline(
    pipeline_path: &Path,
    roots: &SceneAssetPipelineRoots,
    options: SceneAssetPipelineRunOptions,
) -> Result<SceneAssetPipelineRunReport, SceneAssetEditError> {
    let pipeline: SceneAssetPipeline = load_json(pipeline_path)?;
    if pipeline.asset_pipeline_version != 1 {
        return Err(SceneAssetEditError::InvalidOperation(format!(
            "unsupported asset_pipeline_version {}; expected 1",
            pipeline.asset_pipeline_version
        )));
    }
    let mut current_source = resolve_pipeline_input_path(roots, &pipeline.input);
    let input_source = current_source.clone();
    if !current_source.is_file() {
        return Err(SceneAssetEditError::ImageFile {
            path: current_source.display().to_string(),
            message: "pipeline input does not exist".to_string(),
        });
    }

    let mut steps = Vec::with_capacity(pipeline.steps.len());
    for (index, step) in pipeline.steps.iter().enumerate() {
        let step_report =
            run_scene_asset_pipeline_step(index, step, roots, &current_source, &options)?;
        if step_report.advanced_source {
            if let Some(output_path) = &step_report.output_path {
                current_source = PathBuf::from(output_path);
            }
        }
        steps.push(step_report);
    }

    let input_label = input_source
        .strip_prefix(&roots.input_root)
        .map(|path| path.display().to_string())
        .unwrap_or_else(|_| pipeline.input.clone());
    Ok(SceneAssetPipelineRunReport {
        operation: "pipeline_run".to_string(),
        name: pipeline.name,
        input: input_label,
        final_source: current_source.display().to_string(),
        dry_run: options.dry_run,
        steps,
    })
}

fn run_scene_asset_pipeline_step(
    index: usize,
    step: &SceneAssetPipelineStep,
    roots: &SceneAssetPipelineRoots,
    current_source: &Path,
    options: &SceneAssetPipelineRunOptions,
) -> Result<SceneAssetPipelineStepReport, SceneAssetEditError> {
    let output_path = step
        .output
        .as_ref()
        .map(|output| resolve_pipeline_output_path(roots, output));
    let report_path = output_path.as_ref().map(pipeline_report_path);
    let advances_source = pipeline_command_advances_source(&step.command);

    if options.dry_run {
        validate_pipeline_command_name(&step.command)?;
        validate_scene_asset_pipeline_step_args(step, roots)?;
        if output_path.is_none() {
            return Err(SceneAssetEditError::InvalidOperation(format!(
                "pipeline step `{}` requires an output",
                step.command
            )));
        }
        if let Some(output_path) = &output_path {
            if output_path.exists() && !options.force {
                return Err(SceneAssetEditError::OutputExists(
                    output_path.display().to_string(),
                ));
            }
        }
        return Ok(SceneAssetPipelineStepReport {
            index,
            command: step.command.clone(),
            source: current_source.display().to_string(),
            output_path: output_path.as_ref().map(|path| path.display().to_string()),
            report_path: report_path.as_ref().map(|path| path.display().to_string()),
            advanced_source: advances_source,
            dry_run: true,
            report: None,
        });
    }

    let output_path = output_path.ok_or_else(|| {
        SceneAssetEditError::InvalidOperation(format!(
            "pipeline step `{}` requires an output",
            step.command
        ))
    })?;
    let report_path = report_path.expect("report path is derived from output path");
    let feature_map = load_pipeline_feature_map(roots, &step.args)?;
    let report = match step.command.as_str() {
        "sample" => {
            let report = sample_scene_asset_image(
                current_source,
                SceneAssetSampleOptions {
                    points: pipeline_points_arg(&step.args, "points", "point")?,
                    within_regions: pipeline_string_list_arg(&step.args, "within_regions")?,
                    within_polygons: pipeline_polygons_arg(&step.args, "within_polygons")?,
                },
                feature_map.as_ref(),
            )?;
            write_scene_asset_json(&output_path, &report, options.pretty, options.force)?;
            serde_json::to_value(report).map_err(|err| SceneAssetEditError::JsonFile {
                path: output_path.display().to_string(),
                message: err.to_string(),
            })?
        }
        "mask-preview" => {
            let report = preview_scene_asset_selection_mask(
                current_source,
                &output_path,
                SceneAssetMaskPreviewOptions {
                    mode: pipeline_mask_preview_mode_arg(&step.args)?,
                    seeds: pipeline_points_arg(&step.args, "seeds", "seed")?,
                    threshold: pipeline_u8_arg(&step.args, "threshold", 238)?,
                    neutrality: pipeline_u8_arg(&step.args, "neutrality", 28)?,
                    polish: pipeline_mask_polish_options(&step.args)?,
                },
                feature_map.as_ref(),
                options.force,
            )?;
            serde_json::to_value(report).map_err(|err| SceneAssetEditError::JsonFile {
                path: report_path.display().to_string(),
                message: err.to_string(),
            })?
        }
        "mask-export" => {
            let report = export_scene_asset_selection_mask(
                current_source,
                &output_path,
                SceneAssetMaskPreviewOptions {
                    mode: pipeline_mask_preview_mode_arg(&step.args)?,
                    seeds: pipeline_points_arg(&step.args, "seeds", "seed")?,
                    threshold: pipeline_u8_arg(&step.args, "threshold", 238)?,
                    neutrality: pipeline_u8_arg(&step.args, "neutrality", 28)?,
                    polish: pipeline_mask_polish_options(&step.args)?,
                },
                feature_map.as_ref(),
                options.force,
            )?;
            serde_json::to_value(report).map_err(|err| SceneAssetEditError::JsonFile {
                path: report_path.display().to_string(),
                message: err.to_string(),
            })?
        }
        "mask-apply-alpha" => {
            let mask_path = pipeline_required_asset_path_arg(roots, &step.args, "mask")?;
            let report = apply_scene_asset_mask_alpha(
                current_source,
                &mask_path,
                &output_path,
                pipeline_u8_arg(&step.args, "alpha", 0)?,
                options.force,
            )?;
            serde_json::to_value(report).map_err(|err| SceneAssetEditError::JsonFile {
                path: report_path.display().to_string(),
                message: err.to_string(),
            })?
        }
        "mask-composite" => {
            let mask_path = pipeline_required_asset_path_arg(roots, &step.args, "mask")?;
            let patch_path = pipeline_required_asset_path_arg(roots, &step.args, "patch")?;
            let report = composite_scene_asset_mask(
                current_source,
                &patch_path,
                &mask_path,
                &output_path,
                options.force,
            )?;
            serde_json::to_value(report).map_err(|err| SceneAssetEditError::JsonFile {
                path: report_path.display().to_string(),
                message: err.to_string(),
            })?
        }
        "remove-background" => {
            let report = make_scene_asset_background_transparent(
                current_source,
                &output_path,
                pipeline_u8_arg(&step.args, "tolerance", 24)?,
                pipeline_u32_arg(&step.args, "feather", 0)?,
                pipeline_background_sample_arg(&step.args, "sample")?,
                options.force,
            )?;
            serde_json::to_value(report).map_err(|err| SceneAssetEditError::JsonFile {
                path: report_path.display().to_string(),
                message: err.to_string(),
            })?
        }
        "remove-background-polished" => {
            let report = make_scene_asset_background_transparent_polished(
                current_source,
                &output_path,
                pipeline_mask_polish_options(&step.args)?,
                feature_map.as_ref(),
                options.force,
            )?;
            serde_json::to_value(report).map_err(|err| SceneAssetEditError::JsonFile {
                path: report_path.display().to_string(),
                message: err.to_string(),
            })?
        }
        "color-range-erase" => {
            let report = color_range_erase_scene_asset_image(
                current_source,
                &output_path,
                pipeline_mask_polish_options(&step.args)?,
                feature_map.as_ref(),
                options.force,
            )?;
            serde_json::to_value(report).map_err(|err| SceneAssetEditError::JsonFile {
                path: report_path.display().to_string(),
                message: err.to_string(),
            })?
        }
        "magic-erase-add" => {
            let seeds = pipeline_points_arg(&step.args, "seeds", "seed")?;
            let report = magic_erase_add_scene_asset_image(
                current_source,
                &output_path,
                &seeds,
                pipeline_mask_polish_options(&step.args)?,
                feature_map.as_ref(),
                options.force,
            )?;
            serde_json::to_value(report).map_err(|err| SceneAssetEditError::JsonFile {
                path: report_path.display().to_string(),
                message: err.to_string(),
            })?
        }
        "hair-cleanup" => {
            let report = cleanup_scene_asset_hair_edges(
                current_source,
                &output_path,
                SceneAssetHairCleanupMode::Decontaminate,
                pipeline_u32_arg(&step.args, "radius", 4)?,
                pipeline_f32_arg(&step.args, "strength", 0.85)?,
                feature_map.as_ref(),
                pipeline_string_arg(&step.args, "hair_region")?.as_deref(),
                options.force,
            )?;
            serde_json::to_value(report).map_err(|err| SceneAssetEditError::JsonFile {
                path: report_path.display().to_string(),
                message: err.to_string(),
            })?
        }
        "fill-region" => {
            let report = fill_scene_asset_region(
                current_source,
                &output_path,
                SceneAssetFillOptions {
                    color: pipeline_color_arg(&step.args, "color")?,
                    whole_image: pipeline_bool_arg(&step.args, "whole_image", false)?,
                    within_regions: pipeline_string_list_arg(&step.args, "within_regions")?,
                    within_polygons: pipeline_polygons_arg(&step.args, "within_polygons")?,
                    protect_regions: pipeline_string_list_arg(&step.args, "protect_regions")?,
                },
                feature_map.as_ref(),
                options.force,
            )?;
            serde_json::to_value(report).map_err(|err| SceneAssetEditError::JsonFile {
                path: report_path.display().to_string(),
                message: err.to_string(),
            })?
        }
        "sample-fill" => {
            let report = sample_fill_scene_asset_region(
                current_source,
                &output_path,
                SceneAssetSampleFillOptions {
                    sample_point: pipeline_required_point_arg(&step.args, "sample_point")?,
                    sample_radius: pipeline_u32_arg(&step.args, "sample_radius", 1)?,
                    within_regions: pipeline_string_list_arg(&step.args, "within_regions")?,
                    within_polygons: pipeline_polygons_arg(&step.args, "within_polygons")?,
                    protect_regions: pipeline_string_list_arg(&step.args, "protect_regions")?,
                },
                feature_map.as_ref(),
                options.force,
            )?;
            serde_json::to_value(report).map_err(|err| SceneAssetEditError::JsonFile {
                path: report_path.display().to_string(),
                message: err.to_string(),
            })?
        }
        "alpha-paint" => {
            let report = alpha_paint_scene_asset_region(
                current_source,
                &output_path,
                SceneAssetAlphaPaintOptions {
                    alpha: pipeline_u8_arg(&step.args, "alpha", 255)?,
                    whole_image: pipeline_bool_arg(&step.args, "whole_image", false)?,
                    within_regions: pipeline_string_list_arg(&step.args, "within_regions")?,
                    within_polygons: pipeline_polygons_arg(&step.args, "within_polygons")?,
                    protect_regions: pipeline_string_list_arg(&step.args, "protect_regions")?,
                },
                feature_map.as_ref(),
                options.force,
            )?;
            serde_json::to_value(report).map_err(|err| SceneAssetEditError::JsonFile {
                path: report_path.display().to_string(),
                message: err.to_string(),
            })?
        }
        "clone-stamp" => {
            let report = clone_stamp_scene_asset_region(
                current_source,
                &output_path,
                SceneAssetCloneStampOptions {
                    sample_origin: pipeline_required_point_arg(&step.args, "sample_origin")?,
                    target_origin: pipeline_required_point_arg(&step.args, "target_origin")?,
                    within_regions: pipeline_string_list_arg(&step.args, "within_regions")?,
                    within_polygons: pipeline_polygons_arg(&step.args, "within_polygons")?,
                    protect_regions: pipeline_string_list_arg(&step.args, "protect_regions")?,
                },
                feature_map.as_ref(),
                options.force,
            )?;
            serde_json::to_value(report).map_err(|err| SceneAssetEditError::JsonFile {
                path: report_path.display().to_string(),
                message: err.to_string(),
            })?
        }
        "draw-shape" => {
            let report = draw_scene_asset_shape(
                current_source,
                &output_path,
                SceneAssetDrawShapeOptions {
                    shape: pipeline_draw_shape_arg(&step.args)?,
                    color: pipeline_color_arg(&step.args, "color")?,
                    stroke_width: pipeline_u32_arg(&step.args, "stroke_width", 1)?,
                    fill: pipeline_bool_arg(&step.args, "fill", false)?,
                    rect: pipeline_rect_arg(&step.args, "rect")?,
                    points: pipeline_points_arg(&step.args, "points", "point")?,
                    protect_regions: pipeline_string_list_arg(&step.args, "protect_regions")?,
                },
                feature_map.as_ref(),
                options.force,
            )?;
            serde_json::to_value(report).map_err(|err| SceneAssetEditError::JsonFile {
                path: report_path.display().to_string(),
                message: err.to_string(),
            })?
        }
        "stroke-path" => {
            let report = stroke_scene_asset_path(
                current_source,
                &output_path,
                SceneAssetStrokePathOptions {
                    path: pipeline_path_points_arg(&step.args, "path")?,
                    color: pipeline_color_arg(&step.args, "color")?,
                    width: pipeline_u32_arg(&step.args, "width", 1)?,
                    closed: pipeline_bool_arg(&step.args, "closed", false)?,
                    protect_regions: pipeline_string_list_arg(&step.args, "protect_regions")?,
                },
                feature_map.as_ref(),
                options.force,
            )?;
            serde_json::to_value(report).map_err(|err| SceneAssetEditError::JsonFile {
                path: report_path.display().to_string(),
                message: err.to_string(),
            })?
        }
        "crop" => {
            let report = crop_scene_asset_image(
                current_source,
                &output_path,
                SceneAssetCropOptions {
                    rect: pipeline_rect_arg(&step.args, "rect")?,
                    content_bounds: pipeline_bool_arg(&step.args, "content_bounds", false)?,
                },
                options.force,
            )?;
            serde_json::to_value(report).map_err(|err| SceneAssetEditError::JsonFile {
                path: report_path.display().to_string(),
                message: err.to_string(),
            })?
        }
        "pad" => {
            let report = pad_scene_asset_image(
                current_source,
                &output_path,
                SceneAssetPadOptions {
                    width: pipeline_u32_required_arg(&step.args, "width")?,
                    height: pipeline_u32_required_arg(&step.args, "height")?,
                    anchor: pipeline_pad_anchor_arg(&step.args)?,
                    color: pipeline_optional_color_arg(&step.args, "color", [0, 0, 0, 0])?,
                },
                options.force,
            )?;
            serde_json::to_value(report).map_err(|err| SceneAssetEditError::JsonFile {
                path: report_path.display().to_string(),
                message: err.to_string(),
            })?
        }
        "transform" => {
            let (translate_x, translate_y) = pipeline_translate_arg(&step.args)?;
            let report = transform_scene_asset_image(
                current_source,
                &output_path,
                SceneAssetTransformOptions {
                    scale: pipeline_f32_arg(&step.args, "scale", 1.0)?,
                    translate_x,
                    translate_y,
                    flip_x: pipeline_bool_arg(&step.args, "flip_x", false)?,
                    flip_y: pipeline_bool_arg(&step.args, "flip_y", false)?,
                    resample: pipeline_resample_arg(&step.args)?,
                },
                options.force,
            )?;
            serde_json::to_value(report).map_err(|err| SceneAssetEditError::JsonFile {
                path: report_path.display().to_string(),
                message: err.to_string(),
            })?
        }
        "levels" => {
            let report = levels_scene_asset_image(
                current_source,
                &output_path,
                SceneAssetLevelsOptions {
                    channel: pipeline_channel_arg(&step.args)?,
                    black: pipeline_u8_arg(&step.args, "black", 0)?,
                    white: pipeline_u8_arg(&step.args, "white", 255)?,
                    gamma: pipeline_f32_arg(&step.args, "gamma", 1.0)?,
                    within_regions: pipeline_string_list_arg(&step.args, "within_regions")?,
                    within_polygons: pipeline_polygons_arg(&step.args, "within_polygons")?,
                    protect_regions: pipeline_string_list_arg(&step.args, "protect_regions")?,
                },
                feature_map.as_ref(),
                options.force,
            )?;
            serde_json::to_value(report).map_err(|err| SceneAssetEditError::JsonFile {
                path: report_path.display().to_string(),
                message: err.to_string(),
            })?
        }
        "brightness-contrast" => {
            let report = brightness_contrast_scene_asset_image(
                current_source,
                &output_path,
                SceneAssetBrightnessContrastOptions {
                    brightness: pipeline_f32_arg(&step.args, "brightness", 0.0)?,
                    contrast: pipeline_f32_arg(&step.args, "contrast", 0.0)?,
                    within_regions: pipeline_string_list_arg(&step.args, "within_regions")?,
                    within_polygons: pipeline_polygons_arg(&step.args, "within_polygons")?,
                    protect_regions: pipeline_string_list_arg(&step.args, "protect_regions")?,
                },
                feature_map.as_ref(),
                options.force,
            )?;
            serde_json::to_value(report).map_err(|err| SceneAssetEditError::JsonFile {
                path: report_path.display().to_string(),
                message: err.to_string(),
            })?
        }
        "hsl" => {
            let report = hsl_scene_asset_image(
                current_source,
                &output_path,
                SceneAssetHslOptions {
                    hue_degrees: pipeline_f32_arg(&step.args, "hue", 0.0)?,
                    saturation: pipeline_f32_arg(&step.args, "saturation", 0.0)?,
                    lightness: pipeline_f32_arg(&step.args, "lightness", 0.0)?,
                    within_regions: pipeline_string_list_arg(&step.args, "within_regions")?,
                    within_polygons: pipeline_polygons_arg(&step.args, "within_polygons")?,
                    protect_regions: pipeline_string_list_arg(&step.args, "protect_regions")?,
                },
                feature_map.as_ref(),
                options.force,
            )?;
            serde_json::to_value(report).map_err(|err| SceneAssetEditError::JsonFile {
                path: report_path.display().to_string(),
                message: err.to_string(),
            })?
        }
        "blur" => {
            let report = blur_scene_asset_image(
                current_source,
                &output_path,
                SceneAssetBlurOptions {
                    radius: pipeline_f32_arg(&step.args, "radius", 1.0)?,
                    within_regions: pipeline_string_list_arg(&step.args, "within_regions")?,
                    within_polygons: pipeline_polygons_arg(&step.args, "within_polygons")?,
                    protect_regions: pipeline_string_list_arg(&step.args, "protect_regions")?,
                },
                feature_map.as_ref(),
                options.force,
            )?;
            serde_json::to_value(report).map_err(|err| SceneAssetEditError::JsonFile {
                path: report_path.display().to_string(),
                message: err.to_string(),
            })?
        }
        "unsharp-mask" => {
            let report = unsharp_mask_scene_asset_image(
                current_source,
                &output_path,
                SceneAssetUnsharpMaskOptions {
                    radius: pipeline_f32_arg(&step.args, "radius", 1.0)?,
                    amount: pipeline_f32_arg(&step.args, "amount", 1.0)?,
                    threshold: pipeline_u8_arg(&step.args, "threshold", 0)?,
                    within_regions: pipeline_string_list_arg(&step.args, "within_regions")?,
                    within_polygons: pipeline_polygons_arg(&step.args, "within_polygons")?,
                    protect_regions: pipeline_string_list_arg(&step.args, "protect_regions")?,
                },
                feature_map.as_ref(),
                options.force,
            )?;
            serde_json::to_value(report).map_err(|err| SceneAssetEditError::JsonFile {
                path: report_path.display().to_string(),
                message: err.to_string(),
            })?
        }
        command => {
            return Err(SceneAssetEditError::InvalidOperation(format!(
                "unsupported pipeline command `{command}`"
            )));
        }
    };

    if step.command != "sample" {
        write_scene_asset_json(&report_path, &report, options.pretty, options.force)?;
    }

    Ok(SceneAssetPipelineStepReport {
        index,
        command: step.command.clone(),
        source: current_source.display().to_string(),
        output_path: Some(output_path.display().to_string()),
        report_path: Some(
            if step.command == "sample" {
                output_path
            } else {
                report_path
            }
            .display()
            .to_string(),
        ),
        advanced_source: advances_source,
        dry_run: false,
        report: Some(report),
    })
}

pub fn run_scene_asset_operation(
    operation_path: &Path,
    roots: &SceneAssetPipelineRoots,
    options: SceneAssetOperationRunOptions,
) -> Result<SceneAssetOperationRunReport, SceneAssetEditError> {
    let operation: SceneAssetOperation = load_json(operation_path)?;
    if operation.asset_operation_version != 1 {
        return Err(SceneAssetEditError::InvalidOperation(format!(
            "unsupported asset_operation_version {}; expected 1",
            operation.asset_operation_version
        )));
    }
    if operation.id.trim().is_empty() {
        return Err(SceneAssetEditError::InvalidOperation(
            "asset operation id is required".to_string(),
        ));
    }
    if operation.output.trim().is_empty() {
        return Err(SceneAssetEditError::InvalidOperation(
            "asset operation output is required".to_string(),
        ));
    }
    validate_pipeline_command_name(&operation.command)?;

    let source_path = resolve_asset_operation_source_path(roots, &operation.source);
    if !source_path.is_file() {
        return Err(SceneAssetEditError::ImageFile {
            path: source_path.display().to_string(),
            message: "operation source does not exist".to_string(),
        });
    }
    let before = inspect_scene_asset_image(&source_path)?;
    let requested_output_path = resolve_pipeline_output_path(roots, &operation.output);
    let step_output = if options.preview {
        operation_preview_output(&operation.id)
    } else {
        operation.output.clone()
    };
    let step = SceneAssetPipelineStep {
        command: operation.command.clone(),
        output: Some(step_output),
        args: operation.args.clone(),
    };
    let step_report = run_scene_asset_pipeline_step(
        0,
        &step,
        roots,
        &source_path,
        &SceneAssetPipelineRunOptions {
            force: options.force,
            dry_run: options.dry_run,
            pretty: options.pretty,
        },
    )?;
    let output_path = step_report.output_path.clone().unwrap_or_else(|| {
        resolve_pipeline_output_path(
            roots,
            step.output
                .as_deref()
                .expect("operation step output is always present"),
        )
        .display()
        .to_string()
    });
    let compare = if !options.dry_run && step_report.advanced_source {
        Some(compare_scene_asset_images(
            &source_path,
            Path::new(&output_path),
        )?)
    } else {
        None
    };
    let review_preview_paths = if options.preview
        && !options.dry_run
        && compare
            .as_ref()
            .is_some_and(|report| report.same_dimensions)
    {
        Some(write_scene_asset_operation_review_previews(
            roots,
            &operation.id,
            &source_path,
            Path::new(&output_path),
            options.force,
        )?)
    } else {
        None
    };
    let diff_preview_path = review_preview_paths
        .as_ref()
        .and_then(|paths| paths.overlay_diff.clone());
    let after = compare.as_ref().map(|report| report.after.clone());
    let protected_region_report = if !options.dry_run
        && step_report.advanced_source
        && !operation.expectations.must_preserve_regions.is_empty()
    {
        load_pipeline_feature_map(roots, &operation.args)?.map_or(Ok(None), |feature_map| {
            compare_protected_regions(
                &source_path,
                Path::new(&output_path),
                &feature_map,
                &operation.expectations.must_preserve_regions,
            )
            .map(Some)
        })?
    } else {
        None
    };
    let expectation_failures = operation_expectation_failures(
        &operation.expectations,
        compare.as_ref(),
        protected_region_report.as_ref(),
    );
    let status = if options.dry_run {
        "validated"
    } else if expectation_failures.is_empty() {
        "ok"
    } else {
        "expectation_failed"
    }
    .to_string();
    Ok(SceneAssetOperationRunReport {
        operation: "operation_run".to_string(),
        id: operation.id,
        command: operation.command,
        intent: operation.intent,
        source: source_path.display().to_string(),
        output_path,
        report_path: step_report.report_path.clone(),
        dry_run: options.dry_run,
        preview: options.preview,
        status,
        requested_output_path: options
            .preview
            .then_some(requested_output_path.display().to_string()),
        diff_preview_path,
        review_preview_paths,
        before: Some(before),
        after,
        compare,
        protected_region_report,
        expectation_failures,
        step: step_report,
    })
}

pub fn validate_scene_asset_operation(
    operation_path: &Path,
    roots: &SceneAssetPipelineRoots,
    force: bool,
) -> Result<SceneAssetOperationValidationReport, SceneAssetEditError> {
    let operation: SceneAssetOperation = load_json(operation_path)?;
    if operation.asset_operation_version != 1 {
        return Err(SceneAssetEditError::InvalidOperation(format!(
            "unsupported asset_operation_version {}; expected 1",
            operation.asset_operation_version
        )));
    }
    if operation.id.trim().is_empty() {
        return Err(SceneAssetEditError::InvalidOperation(
            "asset operation id is required".to_string(),
        ));
    }
    if operation.output.trim().is_empty() {
        return Err(SceneAssetEditError::InvalidOperation(
            "asset operation output is required".to_string(),
        ));
    }
    validate_pipeline_command_name(&operation.command)?;
    let source_path = resolve_asset_operation_source_path(roots, &operation.source);
    if !source_path.is_file() {
        return Err(SceneAssetEditError::ImageFile {
            path: source_path.display().to_string(),
            message: "operation source does not exist".to_string(),
        });
    }
    inspect_scene_asset_image(&source_path)?;
    let requested_output_path = resolve_pipeline_output_path(roots, &operation.output);
    if requested_output_path.exists() && !force {
        return Err(SceneAssetEditError::OutputExists(
            requested_output_path.display().to_string(),
        ));
    }
    let step = SceneAssetPipelineStep {
        command: operation.command.clone(),
        output: Some(operation.output.clone()),
        args: operation.args,
    };
    validate_scene_asset_pipeline_step_args(&step, roots)?;
    if !operation.expectations.must_preserve_regions.is_empty() {
        let feature_map = load_pipeline_feature_map(roots, &step.args)?;
        validate_pipeline_region_names(
            feature_map.as_ref(),
            &operation.expectations.must_preserve_regions,
        )?;
    }
    Ok(SceneAssetOperationValidationReport {
        operation: "validate_operation".to_string(),
        id: operation.id,
        status: "ok".to_string(),
        source_path: source_path.display().to_string(),
        requested_output_path: requested_output_path.display().to_string(),
        command: operation.command,
        warnings: Vec::new(),
    })
}

pub fn run_scene_asset_edit_session(
    session_path: &Path,
    roots: &SceneAssetPipelineRoots,
    options: SceneAssetOperationRunOptions,
) -> Result<SceneAssetEditSessionRunReport, SceneAssetEditError> {
    let session: SceneAssetEditSession = load_json(session_path)?;
    if session.asset_session_version != 1 {
        return Err(SceneAssetEditError::InvalidOperation(format!(
            "unsupported asset_session_version {}; expected 1",
            session.asset_session_version
        )));
    }
    if session.name.trim().is_empty() {
        return Err(SceneAssetEditError::InvalidOperation(
            "asset edit session name is required".to_string(),
        ));
    }
    if session.operations.is_empty() {
        return Err(SceneAssetEditError::InvalidOperation(
            "asset edit session requires at least one operation".to_string(),
        ));
    }
    let base_dir = session_path.parent();
    let mut operation_reports = Vec::with_capacity(session.operations.len());
    for operation in &session.operations {
        let operation_path = resolve_recipe_path(operation, base_dir);
        operation_reports.push(run_scene_asset_operation(
            &operation_path,
            roots,
            options.clone(),
        )?);
    }
    let final_output_path = operation_reports
        .last()
        .map(|report| report.output_path.clone());
    Ok(SceneAssetEditSessionRunReport {
        operation: "session_run".to_string(),
        name: session.name,
        dry_run: options.dry_run,
        current_source: session.current_source,
        accepted_outputs: session.accepted_outputs,
        final_output_path,
        operations: operation_reports,
    })
}

pub fn compare_scene_asset_images(
    before_path: &Path,
    after_path: &Path,
) -> Result<SceneAssetCompareReport, SceneAssetEditError> {
    let before = inspect_scene_asset_image(before_path)?;
    let after = inspect_scene_asset_image(after_path)?;
    let before_image = load_rgba_image(before_path)?;
    let after_image = load_rgba_image(after_path)?;
    if before_image.dimensions() != after_image.dimensions() {
        return Ok(SceneAssetCompareReport {
            operation: "compare".to_string(),
            before_path: before_path.display().to_string(),
            after_path: after_path.display().to_string(),
            same_dimensions: false,
            changed_pixels: pixel_len(&before_image).max(pixel_len(&after_image)),
            changed_pixel_ratio: 1.0,
            alpha_changed_pixels: 0,
            changed_bounds: None,
            before,
            after,
        });
    }

    let mut changed_pixels = 0;
    let mut alpha_changed_pixels = 0;
    let mut min_x = before_image.width();
    let mut min_y = before_image.height();
    let mut max_x = 0;
    let mut max_y = 0;
    for y in 0..before_image.height() {
        for x in 0..before_image.width() {
            let before_pixel = before_image.get_pixel(x, y);
            let after_pixel = after_image.get_pixel(x, y);
            if before_pixel.0 == after_pixel.0 {
                continue;
            }
            changed_pixels += 1;
            if before_pixel[3] != after_pixel[3] {
                alpha_changed_pixels += 1;
            }
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
    }
    let changed_bounds = (changed_pixels > 0).then_some(SceneAssetPixelRect {
        x: min_x,
        y: min_y,
        w: max_x.saturating_sub(min_x).saturating_add(1),
        h: max_y.saturating_sub(min_y).saturating_add(1),
    });
    let total_pixels = pixel_len(&before_image).max(1);
    Ok(SceneAssetCompareReport {
        operation: "compare".to_string(),
        before_path: before_path.display().to_string(),
        after_path: after_path.display().to_string(),
        same_dimensions: true,
        changed_pixels,
        changed_pixel_ratio: rounded_f32(changed_pixels as f32 / total_pixels as f32),
        alpha_changed_pixels,
        changed_bounds,
        before,
        after,
    })
}

pub fn accept_scene_asset_output(
    source: &Path,
    output: &Path,
    roots: &SceneAssetPipelineRoots,
    force: bool,
) -> Result<SceneAssetAcceptOutputReport, SceneAssetEditError> {
    let source_path = resolve_asset_accept_source_path(roots, source);
    if !source_path.is_file() {
        return Err(SceneAssetEditError::ImageFile {
            path: source_path.display().to_string(),
            message: "accepted source does not exist".to_string(),
        });
    }
    inspect_scene_asset_image(&source_path)?;
    let output_path = resolve_asset_accept_output_path(roots, output);
    let bytes = read_file(&source_path, "image")?;
    write_file(&output_path, &bytes, force)?;
    let image = inspect_scene_asset_image(&output_path)?;
    Ok(SceneAssetAcceptOutputReport {
        operation: "accept_output".to_string(),
        source_path: source_path.display().to_string(),
        output_path: output_path.display().to_string(),
        status: "ok".to_string(),
        image,
    })
}

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

pub fn magic_erase_scene_asset_image(
    source_path: &Path,
    output_path: &Path,
    seed: SceneAssetNormalizedPoint,
    tolerance: u8,
    contiguous: bool,
    feather: u32,
    force: bool,
) -> Result<SceneAssetSelectionReport, SceneAssetEditError> {
    if output_path.exists() && !force {
        return Err(SceneAssetEditError::OutputExists(
            output_path.display().to_string(),
        ));
    }
    let mut image = load_rgba_image(source_path)?;
    let seed = normalized_point_to_pixel(seed, image.width(), image.height())?;
    let mask = if contiguous {
        contiguous_magic_mask(&image, &[seed], tolerance)
    } else {
        global_magic_mask(&image, *image.get_pixel(seed.0, seed.1), tolerance)
    };
    let selected_pixels = selected_pixel_count(&mask);
    apply_transparency_mask(&mut image, &mask, feather);
    save_rgba_image(&image, output_path, force)?;
    Ok(selection_report(
        "magic_erase",
        source_path,
        output_path,
        selected_pixels,
        mask.len(),
        tolerance,
        feather,
    )?)
}

pub fn make_scene_asset_background_transparent(
    source_path: &Path,
    output_path: &Path,
    tolerance: u8,
    feather: u32,
    sample: SceneAssetBackgroundSample,
    force: bool,
) -> Result<SceneAssetSelectionReport, SceneAssetEditError> {
    if output_path.exists() && !force {
        return Err(SceneAssetEditError::OutputExists(
            output_path.display().to_string(),
        ));
    }
    let mut image = load_rgba_image(source_path)?;
    let mask = background_magic_mask(&image, tolerance, sample);
    let selected_pixels = selected_pixel_count(&mask);
    apply_transparency_mask(&mut image, &mask, feather);
    save_rgba_image(&image, output_path, force)?;
    Ok(selection_report(
        "remove_background",
        source_path,
        output_path,
        selected_pixels,
        mask.len(),
        tolerance,
        feather,
    )?)
}

pub fn make_scene_asset_background_transparent_polished(
    source_path: &Path,
    output_path: &Path,
    options: SceneAssetMaskPolishOptions,
    feature_map: Option<&SceneAssetFeatureMap>,
    force: bool,
) -> Result<SceneAssetSelectionReport, SceneAssetEditError> {
    if output_path.exists() && !force {
        return Err(SceneAssetEditError::OutputExists(
            output_path.display().to_string(),
        ));
    }
    let mut image = load_rgba_image(source_path)?;
    if let Some(feature_map) = feature_map {
        validate_scene_asset_feature_map(feature_map, image.width(), image.height())?;
    }
    let mask = polished_background_mask(&image, &options, feature_map)?;
    let selected_pixels = mask.selected_count();
    apply_transparency_mask(&mut image, mask.pixels(), options.feather);
    defringe_scene_asset_edges(&mut image, options.defringe);
    save_rgba_image(&image, output_path, force)?;
    let mut report = selection_report(
        "remove_background_polished",
        source_path,
        output_path,
        selected_pixels,
        mask.len(),
        options.tolerance,
        options.feather,
    )?;
    report.quality = Some(cutout_quality_report(&image, options.protect_regions.len()));
    Ok(report)
}

pub fn color_range_erase_scene_asset_image(
    source_path: &Path,
    output_path: &Path,
    options: SceneAssetMaskPolishOptions,
    feature_map: Option<&SceneAssetFeatureMap>,
    force: bool,
) -> Result<SceneAssetSelectionReport, SceneAssetEditError> {
    write_polished_selection_output(
        "color_range_erase",
        source_path,
        output_path,
        options,
        feature_map,
        force,
        |image, options| {
            Ok(SceneAssetMask::from_pixels(
                image.width(),
                image.height(),
                color_range_mask(image, options.tolerance, options.sample),
            ))
        },
    )
}

pub fn magic_erase_add_scene_asset_image(
    source_path: &Path,
    output_path: &Path,
    seeds: &[SceneAssetNormalizedPoint],
    options: SceneAssetMaskPolishOptions,
    feature_map: Option<&SceneAssetFeatureMap>,
    force: bool,
) -> Result<SceneAssetSelectionReport, SceneAssetEditError> {
    let seeds = seeds.to_vec();
    write_polished_selection_output(
        "magic_erase_add",
        source_path,
        output_path,
        options,
        feature_map,
        force,
        move |image, options| multi_seed_contiguous_mask(image, &seeds, options.tolerance),
    )
}

pub fn channel_matte_erase_scene_asset_image(
    source_path: &Path,
    output_path: &Path,
    threshold: u8,
    neutrality: u8,
    options: SceneAssetMaskPolishOptions,
    feature_map: Option<&SceneAssetFeatureMap>,
    force: bool,
) -> Result<SceneAssetSelectionReport, SceneAssetEditError> {
    write_polished_selection_output(
        "channel_matte_erase",
        source_path,
        output_path,
        options,
        feature_map,
        force,
        move |image, _options| {
            Ok(SceneAssetMask::from_pixels(
                image.width(),
                image.height(),
                channel_matte_mask(image, threshold, neutrality),
            ))
        },
    )
}

pub fn cleanup_scene_asset_hair_edges(
    source_path: &Path,
    output_path: &Path,
    mode: SceneAssetHairCleanupMode,
    radius: u32,
    strength: f32,
    feature_map: Option<&SceneAssetFeatureMap>,
    region: Option<&str>,
    force: bool,
) -> Result<SceneAssetHairCleanupReport, SceneAssetEditError> {
    if output_path.exists() && !force {
        return Err(SceneAssetEditError::OutputExists(
            output_path.display().to_string(),
        ));
    }
    let mut image = load_rgba_image(source_path)?;
    let pixel_region = match (feature_map, region) {
        (Some(feature_map), Some(region)) => {
            validate_scene_asset_feature_map(feature_map, image.width(), image.height())?;
            Some(feature_map.pixel_region(region, image.width(), image.height())?)
        }
        (None, Some(region)) => {
            return Err(SceneAssetEditError::InvalidOperation(format!(
                "--hair-region `{region}` requires --protect or --feature-map"
            )));
        }
        _ => None,
    };
    let changed_pixels = match mode {
        SceneAssetHairCleanupMode::Decontaminate => {
            decontaminate_light_edges(&mut image, radius, strength, pixel_region)
        }
    };
    save_rgba_image(&image, output_path, force)?;
    Ok(SceneAssetHairCleanupReport {
        operation: "hair_cleanup".to_string(),
        source: source_path.display().to_string(),
        output_path: output_path.display().to_string(),
        changed_pixels,
        radius,
        strength: rounded_f32(strength),
        region: region.map(ToString::to_string),
        report: inspect_scene_asset_image(output_path)?,
    })
}

pub fn restore_scene_asset_from_source(
    base_path: &Path,
    cutout_path: &Path,
    output_path: &Path,
    options: SceneAssetRestoreOptions,
    feature_map: Option<&SceneAssetFeatureMap>,
    force: bool,
) -> Result<SceneAssetRestoreReport, SceneAssetEditError> {
    if output_path.exists() && !force {
        return Err(SceneAssetEditError::OutputExists(
            output_path.display().to_string(),
        ));
    }
    let source = load_rgba_image(base_path)?;
    let mut cutout = load_rgba_image(cutout_path)?;
    if let Some(feature_map) = feature_map {
        validate_scene_asset_feature_map(feature_map, cutout.width(), cutout.height())?;
    }
    let restored_pixels =
        restore_pixels_from_source_image(&source, &mut cutout, &options, feature_map)?;
    save_rgba_image(&cutout, output_path, force)?;
    Ok(SceneAssetRestoreReport {
        operation: "restore_from_source".to_string(),
        base: base_path.display().to_string(),
        cutout: cutout_path.display().to_string(),
        output_path: output_path.display().to_string(),
        restored_pixels,
        filter: options.filter,
        report: inspect_scene_asset_image(output_path)?,
    })
}

pub fn preview_scene_asset_selection_mask(
    source_path: &Path,
    output_path: &Path,
    options: SceneAssetMaskPreviewOptions,
    feature_map: Option<&SceneAssetFeatureMap>,
    force: bool,
) -> Result<SceneAssetMaskPreviewReport, SceneAssetEditError> {
    if output_path.exists() && !force {
        return Err(SceneAssetEditError::OutputExists(
            output_path.display().to_string(),
        ));
    }
    let image = load_rgba_image(source_path)?;
    if let Some(feature_map) = feature_map {
        validate_scene_asset_feature_map(feature_map, image.width(), image.height())?;
    }
    let mask = preview_selection_mask(&image, &options, feature_map)?;
    let selected_pixels = mask.selected_count();
    let preview = selection_mask_preview_image(&image, &mask);
    save_rgba_image(&preview, output_path, force)?;
    Ok(SceneAssetMaskPreviewReport {
        operation: "mask_preview".to_string(),
        source: source_path.display().to_string(),
        output_path: output_path.display().to_string(),
        mode: options.mode,
        selected_pixels,
        total_pixels: mask.len(),
        report: inspect_scene_asset_image(output_path)?,
    })
}

pub fn export_scene_asset_selection_mask(
    source_path: &Path,
    output_path: &Path,
    options: SceneAssetMaskPreviewOptions,
    feature_map: Option<&SceneAssetFeatureMap>,
    force: bool,
) -> Result<SceneAssetMaskExportReport, SceneAssetEditError> {
    if output_path.exists() && !force {
        return Err(SceneAssetEditError::OutputExists(
            output_path.display().to_string(),
        ));
    }
    let image = load_rgba_image(source_path)?;
    if let Some(feature_map) = feature_map {
        validate_scene_asset_feature_map(feature_map, image.width(), image.height())?;
    }
    let mask = preview_selection_mask(&image, &options, feature_map)?;
    let selected_pixels = mask.selected_count();
    let selected_bounds = mask_selection_bounds(mask.width, mask.height, mask.pixels());
    let output = mask_export_image(&mask);
    save_rgba_image(&output, output_path, force)?;
    Ok(SceneAssetMaskExportReport {
        operation: "mask_export".to_string(),
        source: source_path.display().to_string(),
        output_path: output_path.display().to_string(),
        mode: options.mode,
        selected_pixels,
        total_pixels: mask.len(),
        selected_bounds,
        report: inspect_scene_asset_image(output_path)?,
    })
}

pub fn apply_scene_asset_mask_alpha(
    source_path: &Path,
    mask_path: &Path,
    output_path: &Path,
    alpha: u8,
    force: bool,
) -> Result<SceneAssetMaskApplyReport, SceneAssetEditError> {
    if output_path.exists() && !force {
        return Err(SceneAssetEditError::OutputExists(
            output_path.display().to_string(),
        ));
    }
    let mut image = load_rgba_image(source_path)?;
    let mask = load_scene_asset_mask(mask_path, image.width(), image.height())?;
    let selected_pixels = mask.selected_count();
    let mut changed_pixels = 0;
    for y in 0..image.height() {
        for x in 0..image.width() {
            if !mask.pixels()[mask_index(image.width(), x, y)] {
                continue;
            }
            let pixel = image.get_pixel_mut(x, y);
            if pixel[3] != alpha {
                pixel[3] = alpha;
                changed_pixels += 1;
            }
        }
    }
    save_rgba_image(&image, output_path, force)?;
    Ok(SceneAssetMaskApplyReport {
        operation: "mask_apply_alpha".to_string(),
        source: source_path.display().to_string(),
        mask: mask_path.display().to_string(),
        output_path: output_path.display().to_string(),
        selected_pixels,
        changed_pixels,
        alpha,
        report: inspect_scene_asset_image(output_path)?,
    })
}

pub fn composite_scene_asset_mask(
    source_path: &Path,
    patch_path: &Path,
    mask_path: &Path,
    output_path: &Path,
    force: bool,
) -> Result<SceneAssetMaskCompositeReport, SceneAssetEditError> {
    if output_path.exists() && !force {
        return Err(SceneAssetEditError::OutputExists(
            output_path.display().to_string(),
        ));
    }
    let mut image = load_rgba_image(source_path)?;
    let patch = load_rgba_image(patch_path)?;
    if image.dimensions() != patch.dimensions() {
        return Err(SceneAssetEditError::InvalidOperation(format!(
            "mask-composite source and patch dimensions differ: {}x{} vs {}x{}",
            image.width(),
            image.height(),
            patch.width(),
            patch.height()
        )));
    }
    let mask = load_scene_asset_mask(mask_path, image.width(), image.height())?;
    let selected_pixels = mask.selected_count();
    let mut changed_pixels = 0;
    let mut changed_mask = vec![false; pixel_len(&image)];
    for y in 0..image.height() {
        for x in 0..image.width() {
            let index = mask_index(image.width(), x, y);
            if !mask.pixels()[index] {
                continue;
            }
            let patch_pixel = patch.get_pixel(x, y);
            let target = image.get_pixel_mut(x, y);
            if target.0 != patch_pixel.0 {
                *target = *patch_pixel;
                changed_pixels += 1;
                changed_mask[index] = true;
            }
        }
    }
    let changed_bounds = mask_selection_bounds(image.width(), image.height(), &changed_mask);
    save_rgba_image(&image, output_path, force)?;
    Ok(SceneAssetMaskCompositeReport {
        operation: "mask_composite".to_string(),
        source: source_path.display().to_string(),
        patch: patch_path.display().to_string(),
        mask: mask_path.display().to_string(),
        output_path: output_path.display().to_string(),
        selected_pixels,
        changed_pixels,
        changed_bounds,
        report: inspect_scene_asset_image(output_path)?,
    })
}

pub fn report_scene_asset_points(
    source_path: &Path,
    points: &[SceneAssetNormalizedPoint],
) -> Result<SceneAssetPointReport, SceneAssetEditError> {
    if points.is_empty() {
        return Err(SceneAssetEditError::InvalidOperation(
            "point-report requires at least one --point".to_string(),
        ));
    }
    let image = load_rgba_image(source_path)?;
    let mut samples = Vec::with_capacity(points.len());
    for &point in points {
        let (pixel_x, pixel_y) = normalized_point_to_pixel(point, image.width(), image.height())?;
        samples.push(SceneAssetPointSample {
            point,
            pixel_x,
            pixel_y,
            rgba: image.get_pixel(pixel_x, pixel_y).0,
        });
    }
    Ok(SceneAssetPointReport {
        operation: "point_report".to_string(),
        source: source_path.display().to_string(),
        samples,
    })
}

pub fn sample_scene_asset_image(
    source_path: &Path,
    options: SceneAssetSampleOptions,
    feature_map: Option<&SceneAssetFeatureMap>,
) -> Result<SceneAssetSampleReport, SceneAssetEditError> {
    if options.points.is_empty()
        && options.within_regions.is_empty()
        && options.within_polygons.is_empty()
    {
        return Err(SceneAssetEditError::InvalidOperation(
            "sample requires --point, --within-regions, or --within-polygon".to_string(),
        ));
    }
    let image = load_rgba_image(source_path)?;
    if let Some(feature_map) = feature_map {
        validate_scene_asset_feature_map(feature_map, image.width(), image.height())?;
    }
    let mut points = Vec::with_capacity(options.points.len());
    for &point in &options.points {
        let (pixel_x, pixel_y) = normalized_point_to_pixel(point, image.width(), image.height())?;
        points.push(SceneAssetPointSample {
            point,
            pixel_x,
            pixel_y,
            rgba: image.get_pixel(pixel_x, pixel_y).0,
        });
    }
    let region = if options.within_regions.is_empty() && options.within_polygons.is_empty() {
        None
    } else {
        let mask = paint_bounds_mask(
            image.width(),
            image.height(),
            false,
            &options.within_regions,
            &options.within_polygons,
            &[],
            feature_map,
        )?;
        Some(sample_masked_region(&image, mask.pixels())?)
    };
    Ok(SceneAssetSampleReport {
        operation: "sample".to_string(),
        source: source_path.display().to_string(),
        points,
        region,
    })
}

pub fn preview_scene_asset_grid(
    source_path: &Path,
    output_path: &Path,
    options: SceneAssetGridPreviewOptions,
    force: bool,
) -> Result<SceneAssetGridPreviewReport, SceneAssetEditError> {
    if !options.step.is_finite() || options.step <= 0.0 || options.step > 1.0 {
        return Err(SceneAssetEditError::InvalidOperation(
            "grid step must be finite and inside 0..1".to_string(),
        ));
    }
    let mut image = load_rgba_image(source_path)?;
    let line = Rgba([255, 64, 64, 220]);
    let bold_line = Rgba([255, 220, 64, 255]);
    let mut value = 0.0;
    let mut index = 0usize;
    while value <= 1.0 + f32::EPSILON {
        let x = (value.min(1.0) * image.width().saturating_sub(1) as f32).round() as u32;
        let y = (value.min(1.0) * image.height().saturating_sub(1) as f32).round() as u32;
        let color = if index == 0 || (value - 0.5).abs() < options.step / 2.0 {
            bold_line
        } else {
            line
        };
        draw_vertical_line(&mut image, x, color);
        draw_horizontal_line(&mut image, y, color);
        value += options.step;
        index += 1;
    }
    save_rgba_image(&image, output_path, force)?;
    Ok(SceneAssetGridPreviewReport {
        operation: "grid_preview".to_string(),
        source: source_path.display().to_string(),
        output_path: output_path.display().to_string(),
        step: options.step,
        report: inspect_scene_asset_image(output_path)?,
    })
}

pub fn fill_scene_asset_region(
    source_path: &Path,
    output_path: &Path,
    options: SceneAssetFillOptions,
    feature_map: Option<&SceneAssetFeatureMap>,
    force: bool,
) -> Result<SceneAssetPaintReport, SceneAssetEditError> {
    let mut image = load_rgba_image(source_path)?;
    let mask = paint_bounds_mask(
        image.width(),
        image.height(),
        options.whole_image,
        &options.within_regions,
        &options.within_polygons,
        &options.protect_regions,
        feature_map,
    )?;
    let color = Rgba(options.color);
    let changed_pixels = paint_pixels(&mut image, mask.pixels(), |pixel| {
        if *pixel == color {
            false
        } else {
            *pixel = color;
            true
        }
    });
    save_rgba_image(&image, output_path, force)?;
    Ok(SceneAssetPaintReport {
        operation: "fill_region".to_string(),
        source: source_path.display().to_string(),
        output_path: output_path.display().to_string(),
        changed_pixels,
        report: inspect_scene_asset_image(output_path)?,
    })
}

pub fn sample_fill_scene_asset_region(
    source_path: &Path,
    output_path: &Path,
    options: SceneAssetSampleFillOptions,
    feature_map: Option<&SceneAssetFeatureMap>,
    force: bool,
) -> Result<SceneAssetPaintReport, SceneAssetEditError> {
    let mut image = load_rgba_image(source_path)?;
    let (sample_x, sample_y) =
        normalized_point_to_pixel(options.sample_point, image.width(), image.height())?;
    let color = median_sample_color(&image, sample_x, sample_y, options.sample_radius);
    let mask = paint_bounds_mask(
        image.width(),
        image.height(),
        false,
        &options.within_regions,
        &options.within_polygons,
        &options.protect_regions,
        feature_map,
    )?;
    let changed_pixels = paint_pixels(&mut image, mask.pixels(), |pixel| {
        if *pixel == color {
            false
        } else {
            *pixel = color;
            true
        }
    });
    save_rgba_image(&image, output_path, force)?;
    Ok(SceneAssetPaintReport {
        operation: "sample_fill".to_string(),
        source: source_path.display().to_string(),
        output_path: output_path.display().to_string(),
        changed_pixels,
        report: inspect_scene_asset_image(output_path)?,
    })
}

pub fn alpha_paint_scene_asset_region(
    source_path: &Path,
    output_path: &Path,
    options: SceneAssetAlphaPaintOptions,
    feature_map: Option<&SceneAssetFeatureMap>,
    force: bool,
) -> Result<SceneAssetPaintReport, SceneAssetEditError> {
    let mut image = load_rgba_image(source_path)?;
    let mask = paint_bounds_mask(
        image.width(),
        image.height(),
        options.whole_image,
        &options.within_regions,
        &options.within_polygons,
        &options.protect_regions,
        feature_map,
    )?;
    let changed_pixels = paint_pixels(&mut image, mask.pixels(), |pixel| {
        if pixel[3] == options.alpha {
            false
        } else {
            pixel[3] = options.alpha;
            true
        }
    });
    save_rgba_image(&image, output_path, force)?;
    Ok(SceneAssetPaintReport {
        operation: "alpha_paint".to_string(),
        source: source_path.display().to_string(),
        output_path: output_path.display().to_string(),
        changed_pixels,
        report: inspect_scene_asset_image(output_path)?,
    })
}

pub fn clone_stamp_scene_asset_region(
    source_path: &Path,
    output_path: &Path,
    options: SceneAssetCloneStampOptions,
    feature_map: Option<&SceneAssetFeatureMap>,
    force: bool,
) -> Result<SceneAssetPaintReport, SceneAssetEditError> {
    let mut image = load_rgba_image(source_path)?;
    let source = image.clone();
    let (sample_x, sample_y) =
        normalized_point_to_pixel(options.sample_origin, image.width(), image.height())?;
    let (target_x, target_y) =
        normalized_point_to_pixel(options.target_origin, image.width(), image.height())?;
    let dx = sample_x as i32 - target_x as i32;
    let dy = sample_y as i32 - target_y as i32;
    let mask = paint_bounds_mask(
        image.width(),
        image.height(),
        false,
        &options.within_regions,
        &options.within_polygons,
        &options.protect_regions,
        feature_map,
    )?;
    let mut changed_pixels = 0;
    for y in 0..image.height() {
        for x in 0..image.width() {
            if !mask.pixels()[mask_index(image.width(), x, y)] {
                continue;
            }
            let source_x = x as i32 + dx;
            let source_y = y as i32 + dy;
            if source_x < 0
                || source_y < 0
                || source_x >= image.width() as i32
                || source_y >= image.height() as i32
            {
                continue;
            }
            let replacement = *source.get_pixel(source_x as u32, source_y as u32);
            let pixel = image.get_pixel_mut(x, y);
            if *pixel != replacement {
                *pixel = replacement;
                changed_pixels += 1;
            }
        }
    }
    save_rgba_image(&image, output_path, force)?;
    Ok(SceneAssetPaintReport {
        operation: "clone_stamp".to_string(),
        source: source_path.display().to_string(),
        output_path: output_path.display().to_string(),
        changed_pixels,
        report: inspect_scene_asset_image(output_path)?,
    })
}

pub fn draw_scene_asset_shape(
    source_path: &Path,
    output_path: &Path,
    options: SceneAssetDrawShapeOptions,
    feature_map: Option<&SceneAssetFeatureMap>,
    force: bool,
) -> Result<SceneAssetPaintReport, SceneAssetEditError> {
    let mut image = load_rgba_image(source_path)?;
    let original = image.clone();
    let color = Rgba(options.color);
    match options.shape {
        SceneAssetDrawShapeKind::Rect => {
            let rect = normalized_rect_arg(options.rect, "draw-shape rect")?
                .to_pixels(image.width(), image.height());
            if options.fill {
                fill_region(&mut image, rect, color);
            }
            draw_rect_outline(&mut image, rect, color, options.stroke_width.max(1));
        }
        SceneAssetDrawShapeKind::Line => {
            if options.points.len() < 2 {
                return Err(SceneAssetEditError::InvalidOperation(
                    "draw-shape line requires at least two --point values".to_string(),
                ));
            }
            draw_normalized_line(
                &mut image,
                options.points[0],
                options.points[1],
                color,
                options.stroke_width.max(1),
            )?;
        }
        SceneAssetDrawShapeKind::Polygon => {
            validate_polygon(&options.points)?;
            if options.fill {
                let mut mask = SceneAssetMask::from_pixels(
                    image.width(),
                    image.height(),
                    vec![false; pixel_len(&image)],
                );
                mask.select_polygon(&options.points)?;
                paint_pixels(&mut image, mask.pixels(), |pixel| {
                    let before = *pixel;
                    blend_pixel(pixel, color, 1.0);
                    *pixel != before
                });
            }
            draw_normalized_path(
                &mut image,
                &options.points,
                color,
                options.stroke_width.max(1),
                true,
            )?;
        }
        SceneAssetDrawShapeKind::Ellipse => {
            let rect = normalized_rect_arg(options.rect, "draw-shape ellipse")?
                .to_pixels(image.width(), image.height());
            draw_ellipse(
                &mut image,
                rect,
                Some(color),
                options.fill.then_some(color),
                options.stroke_width.max(1),
            );
        }
    }
    restore_protected_regions(&original, &mut image, feature_map, &options.protect_regions)?;
    let changed_pixels = changed_pixel_count(&original, &image);
    save_rgba_image(&image, output_path, force)?;
    Ok(SceneAssetPaintReport {
        operation: "draw_shape".to_string(),
        source: source_path.display().to_string(),
        output_path: output_path.display().to_string(),
        changed_pixels,
        report: inspect_scene_asset_image(output_path)?,
    })
}

pub fn stroke_scene_asset_path(
    source_path: &Path,
    output_path: &Path,
    options: SceneAssetStrokePathOptions,
    feature_map: Option<&SceneAssetFeatureMap>,
    force: bool,
) -> Result<SceneAssetPaintReport, SceneAssetEditError> {
    let mut image = load_rgba_image(source_path)?;
    let original = image.clone();
    draw_normalized_path(
        &mut image,
        &options.path,
        Rgba(options.color),
        options.width.max(1),
        options.closed,
    )?;
    restore_protected_regions(&original, &mut image, feature_map, &options.protect_regions)?;
    let changed_pixels = changed_pixel_count(&original, &image);
    save_rgba_image(&image, output_path, force)?;
    Ok(SceneAssetPaintReport {
        operation: "stroke_path".to_string(),
        source: source_path.display().to_string(),
        output_path: output_path.display().to_string(),
        changed_pixels,
        report: inspect_scene_asset_image(output_path)?,
    })
}

pub fn crop_scene_asset_image(
    source_path: &Path,
    output_path: &Path,
    options: SceneAssetCropOptions,
    force: bool,
) -> Result<SceneAssetPaintReport, SceneAssetEditError> {
    let image = load_rgba_image(source_path)?;
    let rect = if options.content_bounds {
        content_bounds(&image).ok_or_else(|| {
            SceneAssetEditError::InvalidOperation(
                "crop --content-bounds found no visible pixels".to_string(),
            )
        })?
    } else {
        normalized_rect_arg(options.rect, "crop")?.to_pixels(image.width(), image.height())
    };
    let output = crop_region(&image, rect);
    save_rgba_image(&output, output_path, force)?;
    Ok(SceneAssetPaintReport {
        operation: "crop".to_string(),
        source: source_path.display().to_string(),
        output_path: output_path.display().to_string(),
        changed_pixels: output.width() as usize * output.height() as usize,
        report: inspect_scene_asset_image(output_path)?,
    })
}

pub fn pad_scene_asset_image(
    source_path: &Path,
    output_path: &Path,
    options: SceneAssetPadOptions,
    force: bool,
) -> Result<SceneAssetPaintReport, SceneAssetEditError> {
    let image = load_rgba_image(source_path)?;
    if options.width < image.width() || options.height < image.height() {
        return Err(SceneAssetEditError::InvalidOperation(
            "pad width and height must be greater than or equal to source dimensions".to_string(),
        ));
    }
    let mut output = ImageBuffer::from_pixel(options.width, options.height, Rgba(options.color));
    let x = match options.anchor {
        SceneAssetPadAnchor::TopLeft => 0,
        SceneAssetPadAnchor::Center | SceneAssetPadAnchor::BottomCenter => {
            (options.width - image.width()) / 2
        }
    } as i32;
    let y = match options.anchor {
        SceneAssetPadAnchor::TopLeft => 0,
        SceneAssetPadAnchor::Center => (options.height - image.height()) / 2,
        SceneAssetPadAnchor::BottomCenter => options.height - image.height(),
    } as i32;
    paste_region(&mut output, &image, x, y, 1.0);
    save_rgba_image(&output, output_path, force)?;
    Ok(SceneAssetPaintReport {
        operation: "pad".to_string(),
        source: source_path.display().to_string(),
        output_path: output_path.display().to_string(),
        changed_pixels: output.width() as usize * output.height() as usize,
        report: inspect_scene_asset_image(output_path)?,
    })
}

pub fn transform_scene_asset_image(
    source_path: &Path,
    output_path: &Path,
    options: SceneAssetTransformOptions,
    force: bool,
) -> Result<SceneAssetPaintReport, SceneAssetEditError> {
    if !options.scale.is_finite() || options.scale <= 0.0 {
        return Err(SceneAssetEditError::InvalidOperation(
            "transform scale must be finite and positive".to_string(),
        ));
    }
    let image = load_rgba_image(source_path)?;
    let original = image.clone();
    let mut content = image;
    if options.flip_x {
        content = image::imageops::flip_horizontal(&content);
    }
    if options.flip_y {
        content = image::imageops::flip_vertical(&content);
    }
    if (options.scale - 1.0).abs() > f32::EPSILON {
        let width = ((content.width() as f32 * options.scale).round() as u32).max(1);
        let height = ((content.height() as f32 * options.scale).round() as u32).max(1);
        content = image::imageops::resize(&content, width, height, options.resample.filter_type());
    }
    let mut output =
        ImageBuffer::from_pixel(original.width(), original.height(), Rgba([0, 0, 0, 0]));
    let x = (original.width() as i32 - content.width() as i32) / 2 + options.translate_x;
    let y = (original.height() as i32 - content.height() as i32) / 2 + options.translate_y;
    paste_region(&mut output, &content, x, y, 1.0);
    let changed_pixels = changed_pixel_count(&original, &output);
    save_rgba_image(&output, output_path, force)?;
    Ok(SceneAssetPaintReport {
        operation: "transform".to_string(),
        source: source_path.display().to_string(),
        output_path: output_path.display().to_string(),
        changed_pixels,
        report: inspect_scene_asset_image(output_path)?,
    })
}

pub fn levels_scene_asset_image(
    source_path: &Path,
    output_path: &Path,
    options: SceneAssetLevelsOptions,
    feature_map: Option<&SceneAssetFeatureMap>,
    force: bool,
) -> Result<SceneAssetPaintReport, SceneAssetEditError> {
    if options.white <= options.black || !options.gamma.is_finite() || options.gamma <= 0.0 {
        return Err(SceneAssetEditError::InvalidOperation(
            "levels requires white > black and positive finite gamma".to_string(),
        ));
    }
    let mut image = load_rgba_image(source_path)?;
    let original = image.clone();
    let mask = adjustment_mask(
        image.width(),
        image.height(),
        &options.within_regions,
        &options.within_polygons,
        &options.protect_regions,
        feature_map,
    )?;
    let black = options.black as f32;
    let range = (options.white - options.black) as f32;
    let gamma = 1.0 / options.gamma;
    apply_masked_pixels(&mut image, mask.pixels(), |pixel| {
        for &channel in color_channels(options.channel) {
            let normalized = ((pixel[channel] as f32 - black) / range).clamp(0.0, 1.0);
            pixel[channel] = (normalized.powf(gamma) * 255.0).round().clamp(0.0, 255.0) as u8;
        }
    });
    let changed_pixels = changed_pixel_count(&original, &image);
    save_rgba_image(&image, output_path, force)?;
    Ok(SceneAssetPaintReport {
        operation: "levels".to_string(),
        source: source_path.display().to_string(),
        output_path: output_path.display().to_string(),
        changed_pixels,
        report: inspect_scene_asset_image(output_path)?,
    })
}

pub fn brightness_contrast_scene_asset_image(
    source_path: &Path,
    output_path: &Path,
    options: SceneAssetBrightnessContrastOptions,
    feature_map: Option<&SceneAssetFeatureMap>,
    force: bool,
) -> Result<SceneAssetPaintReport, SceneAssetEditError> {
    if !options.brightness.is_finite() || !options.contrast.is_finite() || options.contrast <= -1.0
    {
        return Err(SceneAssetEditError::InvalidOperation(
            "brightness-contrast requires finite brightness and contrast > -1".to_string(),
        ));
    }
    let mut image = load_rgba_image(source_path)?;
    let original = image.clone();
    let mask = adjustment_mask(
        image.width(),
        image.height(),
        &options.within_regions,
        &options.within_polygons,
        &options.protect_regions,
        feature_map,
    )?;
    let factor = 1.0 + options.contrast;
    apply_masked_pixels(&mut image, mask.pixels(), |pixel| {
        for channel in 0..3 {
            pixel[channel] =
                (((pixel[channel] as f32 - 128.0) * factor) + 128.0 + options.brightness)
                    .round()
                    .clamp(0.0, 255.0) as u8;
        }
    });
    let changed_pixels = changed_pixel_count(&original, &image);
    save_rgba_image(&image, output_path, force)?;
    Ok(SceneAssetPaintReport {
        operation: "brightness_contrast".to_string(),
        source: source_path.display().to_string(),
        output_path: output_path.display().to_string(),
        changed_pixels,
        report: inspect_scene_asset_image(output_path)?,
    })
}

pub fn hsl_scene_asset_image(
    source_path: &Path,
    output_path: &Path,
    options: SceneAssetHslOptions,
    feature_map: Option<&SceneAssetFeatureMap>,
    force: bool,
) -> Result<SceneAssetPaintReport, SceneAssetEditError> {
    if !options.hue_degrees.is_finite()
        || !options.saturation.is_finite()
        || !options.lightness.is_finite()
    {
        return Err(SceneAssetEditError::InvalidOperation(
            "hsl values must be finite".to_string(),
        ));
    }
    let mut image = load_rgba_image(source_path)?;
    let original = image.clone();
    let mask = adjustment_mask(
        image.width(),
        image.height(),
        &options.within_regions,
        &options.within_polygons,
        &options.protect_regions,
        feature_map,
    )?;
    apply_masked_pixels(&mut image, mask.pixels(), |pixel| {
        let (mut h, mut s, mut l) = rgb_to_hsl(pixel[0], pixel[1], pixel[2]);
        h = (h + options.hue_degrees).rem_euclid(360.0);
        s = (s + options.saturation).clamp(0.0, 1.0);
        l = (l + options.lightness).clamp(0.0, 1.0);
        let [r, g, b] = hsl_to_rgb(h, s, l);
        pixel[0] = r;
        pixel[1] = g;
        pixel[2] = b;
    });
    let changed_pixels = changed_pixel_count(&original, &image);
    save_rgba_image(&image, output_path, force)?;
    Ok(SceneAssetPaintReport {
        operation: "hsl".to_string(),
        source: source_path.display().to_string(),
        output_path: output_path.display().to_string(),
        changed_pixels,
        report: inspect_scene_asset_image(output_path)?,
    })
}

pub fn blur_scene_asset_image(
    source_path: &Path,
    output_path: &Path,
    options: SceneAssetBlurOptions,
    feature_map: Option<&SceneAssetFeatureMap>,
    force: bool,
) -> Result<SceneAssetPaintReport, SceneAssetEditError> {
    if !options.radius.is_finite() || options.radius < 0.0 {
        return Err(SceneAssetEditError::InvalidOperation(
            "blur radius must be finite and non-negative".to_string(),
        ));
    }
    let mut image = load_rgba_image(source_path)?;
    let original = image.clone();
    let blurred = image::imageops::blur(&image, options.radius);
    let mask = adjustment_mask(
        image.width(),
        image.height(),
        &options.within_regions,
        &options.within_polygons,
        &options.protect_regions,
        feature_map,
    )?;
    copy_masked_pixels(&blurred, &mut image, mask.pixels());
    let changed_pixels = changed_pixel_count(&original, &image);
    save_rgba_image(&image, output_path, force)?;
    Ok(SceneAssetPaintReport {
        operation: "blur".to_string(),
        source: source_path.display().to_string(),
        output_path: output_path.display().to_string(),
        changed_pixels,
        report: inspect_scene_asset_image(output_path)?,
    })
}

pub fn unsharp_mask_scene_asset_image(
    source_path: &Path,
    output_path: &Path,
    options: SceneAssetUnsharpMaskOptions,
    feature_map: Option<&SceneAssetFeatureMap>,
    force: bool,
) -> Result<SceneAssetPaintReport, SceneAssetEditError> {
    if !options.radius.is_finite() || options.radius < 0.0 || !options.amount.is_finite() {
        return Err(SceneAssetEditError::InvalidOperation(
            "unsharp-mask radius and amount must be finite".to_string(),
        ));
    }
    let mut image = load_rgba_image(source_path)?;
    let original = image.clone();
    let blurred = image::imageops::blur(&image, options.radius);
    let mask = adjustment_mask(
        image.width(),
        image.height(),
        &options.within_regions,
        &options.within_polygons,
        &options.protect_regions,
        feature_map,
    )?;
    for y in 0..image.height() {
        for x in 0..image.width() {
            if !mask.pixels()[mask_index(image.width(), x, y)] {
                continue;
            }
            let source = original.get_pixel(x, y);
            let blurred = blurred.get_pixel(x, y);
            let pixel = image.get_pixel_mut(x, y);
            for channel in 0..3 {
                let diff = source[channel] as i16 - blurred[channel] as i16;
                if diff.unsigned_abs() as u8 >= options.threshold {
                    pixel[channel] = (source[channel] as f32 + options.amount * diff as f32)
                        .round()
                        .clamp(0.0, 255.0) as u8;
                }
            }
            pixel[3] = source[3];
        }
    }
    let changed_pixels = changed_pixel_count(&original, &image);
    save_rgba_image(&image, output_path, force)?;
    Ok(SceneAssetPaintReport {
        operation: "unsharp_mask".to_string(),
        source: source_path.display().to_string(),
        output_path: output_path.display().to_string(),
        changed_pixels,
        report: inspect_scene_asset_image(output_path)?,
    })
}

fn write_polished_selection_output(
    operation: &str,
    source_path: &Path,
    output_path: &Path,
    options: SceneAssetMaskPolishOptions,
    feature_map: Option<&SceneAssetFeatureMap>,
    force: bool,
    build_mask: impl FnOnce(
        &RgbaImage,
        &SceneAssetMaskPolishOptions,
    ) -> Result<SceneAssetMask, SceneAssetEditError>,
) -> Result<SceneAssetSelectionReport, SceneAssetEditError> {
    if output_path.exists() && !force {
        return Err(SceneAssetEditError::OutputExists(
            output_path.display().to_string(),
        ));
    }
    let mut image = load_rgba_image(source_path)?;
    if let Some(feature_map) = feature_map {
        validate_scene_asset_feature_map(feature_map, image.width(), image.height())?;
    }
    let mask = apply_mask_polish(build_mask(&image, &options)?, &options, feature_map)?;
    let selected_pixels = mask.selected_count();
    apply_transparency_mask(&mut image, mask.pixels(), options.feather);
    defringe_scene_asset_edges(&mut image, options.defringe);
    save_rgba_image(&image, output_path, force)?;
    let mut report = selection_report(
        operation,
        source_path,
        output_path,
        selected_pixels,
        mask.len(),
        options.tolerance,
        options.feather,
    )?;
    report.quality = Some(cutout_quality_report(&image, options.protect_regions.len()));
    Ok(report)
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

fn validate_pipeline_command_name(command: &str) -> Result<(), SceneAssetEditError> {
    match command {
        "sample"
        | "mask-preview"
        | "mask-export"
        | "mask-apply-alpha"
        | "mask-composite"
        | "remove-background"
        | "remove-background-polished"
        | "color-range-erase"
        | "magic-erase-add"
        | "hair-cleanup"
        | "fill-region"
        | "sample-fill"
        | "alpha-paint"
        | "clone-stamp"
        | "draw-shape"
        | "stroke-path"
        | "crop"
        | "pad"
        | "transform"
        | "levels"
        | "brightness-contrast"
        | "hsl"
        | "blur"
        | "unsharp-mask" => Ok(()),
        _ => Err(SceneAssetEditError::InvalidOperation(format!(
            "unsupported pipeline command `{command}`"
        ))),
    }
}

fn validate_scene_asset_pipeline_step_args(
    step: &SceneAssetPipelineStep,
    roots: &SceneAssetPipelineRoots,
) -> Result<(), SceneAssetEditError> {
    validate_pipeline_command_name(&step.command)?;
    let feature_map = load_pipeline_feature_map(roots, &step.args)?;
    match step.command.as_str() {
        "sample" => {
            let within_regions = pipeline_string_list_arg(&step.args, "within_regions")?;
            pipeline_points_arg(&step.args, "points", "point")?;
            pipeline_polygons_arg(&step.args, "within_polygons")?;
            validate_pipeline_region_names(feature_map.as_ref(), &within_regions)?;
        }
        "mask-preview" => {
            let mode = pipeline_mask_preview_mode_arg(&step.args)?;
            let seeds = pipeline_points_arg(&step.args, "seeds", "seed")?;
            if mode == SceneAssetMaskPreviewMode::MagicAdd && seeds.is_empty() {
                return Err(SceneAssetEditError::InvalidOperation(
                    "magic-add mask preview requires at least one seed".to_string(),
                ));
            }
            pipeline_u8_arg(&step.args, "threshold", 238)?;
            pipeline_u8_arg(&step.args, "neutrality", 28)?;
            let polish = pipeline_mask_polish_options(&step.args)?;
            validate_mask_polish_regions(feature_map.as_ref(), &polish)?;
        }
        "mask-export" => {
            let mode = pipeline_mask_preview_mode_arg(&step.args)?;
            let seeds = pipeline_points_arg(&step.args, "seeds", "seed")?;
            if mode == SceneAssetMaskPreviewMode::MagicAdd && seeds.is_empty() {
                return Err(SceneAssetEditError::InvalidOperation(
                    "mask-export with magic-add requires at least one seed".to_string(),
                ));
            }
            pipeline_u8_arg(&step.args, "threshold", 238)?;
            pipeline_u8_arg(&step.args, "neutrality", 28)?;
            let polish = pipeline_mask_polish_options(&step.args)?;
            validate_mask_polish_regions(feature_map.as_ref(), &polish)?;
        }
        "mask-apply-alpha" => {
            pipeline_required_asset_path_arg(roots, &step.args, "mask")?;
            pipeline_u8_arg(&step.args, "alpha", 0)?;
        }
        "mask-composite" => {
            pipeline_required_asset_path_arg(roots, &step.args, "mask")?;
            pipeline_required_asset_path_arg(roots, &step.args, "patch")?;
        }
        "remove-background" => {
            pipeline_u8_arg(&step.args, "tolerance", 24)?;
            pipeline_u32_arg(&step.args, "feather", 0)?;
            pipeline_background_sample_arg(&step.args, "sample")?;
        }
        "remove-background-polished" | "color-range-erase" => {
            let polish = pipeline_mask_polish_options(&step.args)?;
            validate_mask_polish_regions(feature_map.as_ref(), &polish)?;
        }
        "magic-erase-add" => {
            let seeds = pipeline_points_arg(&step.args, "seeds", "seed")?;
            if seeds.is_empty() {
                return Err(SceneAssetEditError::InvalidOperation(
                    "magic-erase-add requires at least one seed".to_string(),
                ));
            }
            let polish = pipeline_mask_polish_options(&step.args)?;
            validate_mask_polish_regions(feature_map.as_ref(), &polish)?;
        }
        "hair-cleanup" => {
            pipeline_u32_arg(&step.args, "radius", 4)?;
            pipeline_f32_arg(&step.args, "strength", 0.85)?;
            if let Some(region) = pipeline_string_arg(&step.args, "hair_region")? {
                validate_pipeline_region_names(feature_map.as_ref(), &[region])?;
            }
        }
        "fill-region" => {
            pipeline_color_arg(&step.args, "color")?;
            pipeline_bool_arg(&step.args, "whole_image", false)?;
            let within_regions = pipeline_string_list_arg(&step.args, "within_regions")?;
            pipeline_polygons_arg(&step.args, "within_polygons")?;
            let protect_regions = pipeline_string_list_arg(&step.args, "protect_regions")?;
            validate_pipeline_region_names(feature_map.as_ref(), &within_regions)?;
            validate_pipeline_region_names(feature_map.as_ref(), &protect_regions)?;
        }
        "sample-fill" => {
            pipeline_required_point_arg(&step.args, "sample_point")?;
            pipeline_u32_arg(&step.args, "sample_radius", 1)?;
            let within_regions = pipeline_string_list_arg(&step.args, "within_regions")?;
            pipeline_polygons_arg(&step.args, "within_polygons")?;
            let protect_regions = pipeline_string_list_arg(&step.args, "protect_regions")?;
            validate_pipeline_region_names(feature_map.as_ref(), &within_regions)?;
            validate_pipeline_region_names(feature_map.as_ref(), &protect_regions)?;
        }
        "alpha-paint" => {
            pipeline_u8_arg(&step.args, "alpha", 255)?;
            pipeline_bool_arg(&step.args, "whole_image", false)?;
            let within_regions = pipeline_string_list_arg(&step.args, "within_regions")?;
            pipeline_polygons_arg(&step.args, "within_polygons")?;
            let protect_regions = pipeline_string_list_arg(&step.args, "protect_regions")?;
            validate_pipeline_region_names(feature_map.as_ref(), &within_regions)?;
            validate_pipeline_region_names(feature_map.as_ref(), &protect_regions)?;
        }
        "clone-stamp" => {
            pipeline_required_point_arg(&step.args, "sample_origin")?;
            pipeline_required_point_arg(&step.args, "target_origin")?;
            let within_regions = pipeline_string_list_arg(&step.args, "within_regions")?;
            pipeline_polygons_arg(&step.args, "within_polygons")?;
            let protect_regions = pipeline_string_list_arg(&step.args, "protect_regions")?;
            validate_pipeline_region_names(feature_map.as_ref(), &within_regions)?;
            validate_pipeline_region_names(feature_map.as_ref(), &protect_regions)?;
        }
        "draw-shape" => {
            pipeline_draw_shape_arg(&step.args)?;
            pipeline_color_arg(&step.args, "color")?;
            pipeline_u32_arg(&step.args, "stroke_width", 1)?;
            pipeline_bool_arg(&step.args, "fill", false)?;
            pipeline_rect_arg(&step.args, "rect")?;
            pipeline_points_arg(&step.args, "points", "point")?;
            let protect_regions = pipeline_string_list_arg(&step.args, "protect_regions")?;
            validate_pipeline_region_names(feature_map.as_ref(), &protect_regions)?;
        }
        "stroke-path" => {
            pipeline_path_points_arg(&step.args, "path")?;
            pipeline_color_arg(&step.args, "color")?;
            pipeline_u32_arg(&step.args, "width", 1)?;
            pipeline_bool_arg(&step.args, "closed", false)?;
            let protect_regions = pipeline_string_list_arg(&step.args, "protect_regions")?;
            validate_pipeline_region_names(feature_map.as_ref(), &protect_regions)?;
        }
        "crop" => {
            pipeline_rect_arg(&step.args, "rect")?;
            pipeline_bool_arg(&step.args, "content_bounds", false)?;
        }
        "pad" => {
            pipeline_u32_required_arg(&step.args, "width")?;
            pipeline_u32_required_arg(&step.args, "height")?;
            pipeline_pad_anchor_arg(&step.args)?;
            pipeline_optional_color_arg(&step.args, "color", [0, 0, 0, 0])?;
        }
        "transform" => {
            pipeline_translate_arg(&step.args)?;
            pipeline_f32_arg(&step.args, "scale", 1.0)?;
            pipeline_bool_arg(&step.args, "flip_x", false)?;
            pipeline_bool_arg(&step.args, "flip_y", false)?;
            pipeline_resample_arg(&step.args)?;
        }
        "levels" => {
            pipeline_channel_arg(&step.args)?;
            pipeline_u8_arg(&step.args, "black", 0)?;
            pipeline_u8_arg(&step.args, "white", 255)?;
            pipeline_f32_arg(&step.args, "gamma", 1.0)?;
            validate_adjustment_regions(&step.args, feature_map.as_ref())?;
        }
        "brightness-contrast" => {
            pipeline_f32_arg(&step.args, "brightness", 0.0)?;
            pipeline_f32_arg(&step.args, "contrast", 0.0)?;
            validate_adjustment_regions(&step.args, feature_map.as_ref())?;
        }
        "hsl" => {
            pipeline_f32_arg(&step.args, "hue", 0.0)?;
            pipeline_f32_arg(&step.args, "saturation", 0.0)?;
            pipeline_f32_arg(&step.args, "lightness", 0.0)?;
            validate_adjustment_regions(&step.args, feature_map.as_ref())?;
        }
        "blur" => {
            pipeline_f32_arg(&step.args, "radius", 1.0)?;
            validate_adjustment_regions(&step.args, feature_map.as_ref())?;
        }
        "unsharp-mask" => {
            pipeline_f32_arg(&step.args, "radius", 1.0)?;
            pipeline_f32_arg(&step.args, "amount", 1.0)?;
            pipeline_u8_arg(&step.args, "threshold", 0)?;
            validate_adjustment_regions(&step.args, feature_map.as_ref())?;
        }
        command => {
            return Err(SceneAssetEditError::InvalidOperation(format!(
                "unsupported pipeline command `{command}`"
            )));
        }
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

fn selection_report(
    operation: &str,
    source_path: &Path,
    output_path: &Path,
    selected_pixels: usize,
    total_pixels: usize,
    tolerance: u8,
    feather: u32,
) -> Result<SceneAssetSelectionReport, SceneAssetEditError> {
    Ok(SceneAssetSelectionReport {
        operation: operation.to_string(),
        source: source_path.display().to_string(),
        output_path: output_path.display().to_string(),
        selected_pixels,
        total_pixels,
        tolerance,
        feather,
        quality: None,
        report: inspect_scene_asset_image(output_path)?,
    })
}

fn background_magic_mask(
    image: &RgbaImage,
    tolerance: u8,
    sample: SceneAssetBackgroundSample,
) -> Vec<bool> {
    let sample_colors = background_sample_colors(image, sample);
    let seeds = edge_seed_points_matching_samples(image, &sample_colors, tolerance);
    contiguous_magic_mask_with_samples(image, &seeds, &sample_colors, tolerance)
}

fn polished_background_mask(
    image: &RgbaImage,
    options: &SceneAssetMaskPolishOptions,
    feature_map: Option<&SceneAssetFeatureMap>,
) -> Result<SceneAssetMask, SceneAssetEditError> {
    apply_mask_polish(
        SceneAssetMask::from_pixels(
            image.width(),
            image.height(),
            background_magic_mask(image, options.tolerance, options.sample),
        ),
        options,
        feature_map,
    )
}

fn preview_selection_mask(
    image: &RgbaImage,
    options: &SceneAssetMaskPreviewOptions,
    feature_map: Option<&SceneAssetFeatureMap>,
) -> Result<SceneAssetMask, SceneAssetEditError> {
    let mask = match options.mode {
        SceneAssetMaskPreviewMode::Background => SceneAssetMask::from_pixels(
            image.width(),
            image.height(),
            background_magic_mask(image, options.polish.tolerance, options.polish.sample),
        ),
        SceneAssetMaskPreviewMode::ColorRange => SceneAssetMask::from_pixels(
            image.width(),
            image.height(),
            color_range_mask(image, options.polish.tolerance, options.polish.sample),
        ),
        SceneAssetMaskPreviewMode::MagicAdd => {
            multi_seed_contiguous_mask(image, &options.seeds, options.polish.tolerance)?
        }
        SceneAssetMaskPreviewMode::ChannelMatte => SceneAssetMask::from_pixels(
            image.width(),
            image.height(),
            channel_matte_mask(image, options.threshold, options.neutrality),
        ),
    };
    apply_mask_polish(mask, &options.polish, feature_map)
}

fn restore_pixels_from_source_image(
    source: &RgbaImage,
    target: &mut RgbaImage,
    options: &SceneAssetRestoreOptions,
    feature_map: Option<&SceneAssetFeatureMap>,
) -> Result<usize, SceneAssetEditError> {
    if source.dimensions() != target.dimensions() {
        return Err(SceneAssetEditError::InvalidOperation(format!(
            "restore source dimensions {}x{} must match cutout dimensions {}x{}",
            source.width(),
            source.height(),
            target.width(),
            target.height()
        )));
    }
    let mask = restore_mask(source.width(), source.height(), options, feature_map)?;
    let background_samples = match options.filter {
        SceneAssetRestoreFilter::All => Vec::new(),
        SceneAssetRestoreFilter::NonBackground => background_sample_colors(source, options.sample),
    };
    let mut restored_pixels = 0;
    for y in 0..source.height() {
        for x in 0..source.width() {
            if !mask.pixels()[mask_index(source.width(), x, y)] {
                continue;
            }
            let source_pixel = *source.get_pixel(x, y);
            if options.filter == SceneAssetRestoreFilter::NonBackground
                && pixel_matches_any(source_pixel, &background_samples, options.tolerance)
            {
                continue;
            }
            target.put_pixel(x, y, source_pixel);
            restored_pixels += 1;
        }
    }
    Ok(restored_pixels)
}

fn restore_mask(
    width: u32,
    height: u32,
    options: &SceneAssetRestoreOptions,
    feature_map: Option<&SceneAssetFeatureMap>,
) -> Result<SceneAssetMask, SceneAssetEditError> {
    if options.regions.is_empty() && options.polygons.is_empty() {
        return Err(SceneAssetEditError::InvalidOperation(
            "restore-from-source requires --restore-regions or --polygon".to_string(),
        ));
    }
    let mut mask =
        SceneAssetMask::from_pixels(width, height, vec![false; width as usize * height as usize]);
    if !options.regions.is_empty() {
        let Some(feature_map) = feature_map else {
            return Err(SceneAssetEditError::InvalidOperation(
                "--restore-regions requires --feature-map or --protect".to_string(),
            ));
        };
        for region in &options.regions {
            let region = region.trim();
            if region.is_empty() {
                continue;
            }
            mask.select_rect(feature_map.pixel_region(region, width, height)?);
        }
    }
    for polygon in &options.polygons {
        mask.select_polygon(polygon)?;
    }
    Ok(mask)
}

fn apply_mask_polish(
    mut mask: SceneAssetMask,
    options: &SceneAssetMaskPolishOptions,
    feature_map: Option<&SceneAssetFeatureMap>,
) -> Result<SceneAssetMask, SceneAssetEditError> {
    if options.erode > 0 {
        mask = mask.eroded(options.erode);
    }
    if options.dilate > 0 {
        mask = mask.dilated(options.dilate);
    }
    if options.open > 0 {
        mask = mask.opened(options.open);
    }
    if options.close > 0 {
        mask = mask.closed(options.close);
    }
    if options.remove_small > 0 {
        mask = mask.without_small_components(options.remove_small);
    }
    if options.fill_holes > 0 {
        mask = mask.with_filled_small_holes(options.fill_holes);
    }
    if !options.within_regions.is_empty() || !options.within_polygons.is_empty() {
        let bounds = selection_bounds_mask(mask.width, mask.height, options, feature_map)?;
        mask.intersect_pixels(bounds.pixels());
    }
    if let Some(feature_map) = feature_map {
        mask.protect_feature_regions(feature_map, &options.protect_regions)?;
    }
    Ok(mask)
}

fn selection_bounds_mask(
    width: u32,
    height: u32,
    options: &SceneAssetMaskPolishOptions,
    feature_map: Option<&SceneAssetFeatureMap>,
) -> Result<SceneAssetMask, SceneAssetEditError> {
    let mut mask =
        SceneAssetMask::from_pixels(width, height, vec![false; width as usize * height as usize]);
    if !options.within_regions.is_empty() {
        let Some(feature_map) = feature_map else {
            return Err(SceneAssetEditError::InvalidOperation(
                "--within-regions requires --feature-map or --protect".to_string(),
            ));
        };
        for region in &options.within_regions {
            let region = region.trim();
            if region.is_empty() {
                continue;
            }
            mask.select_rect(feature_map.pixel_region(region, width, height)?);
        }
    }
    for polygon in &options.within_polygons {
        mask.select_polygon(polygon)?;
    }
    Ok(mask)
}

fn selection_mask_preview_image(image: &RgbaImage, mask: &SceneAssetMask) -> RgbaImage {
    let mut output = ImageBuffer::from_pixel(image.width(), image.height(), Rgba([0, 0, 0, 255]));
    for y in 0..image.height() {
        for x in 0..image.width() {
            let source = *image.get_pixel(x, y);
            let selected = mask.pixels()[mask_index(image.width(), x, y)];
            let preview = if selected {
                Rgba([
                    lerp(source[0] as f32, 255.0, 0.72).round() as u8,
                    lerp(source[1] as f32, 40.0, 0.72).round() as u8,
                    lerp(source[2] as f32, 40.0, 0.72).round() as u8,
                    255,
                ])
            } else {
                Rgba([
                    (source[0] as f32 * 0.35).round() as u8,
                    (source[1] as f32 * 0.35).round() as u8,
                    (source[2] as f32 * 0.35).round() as u8,
                    255,
                ])
            };
            output.put_pixel(x, y, preview);
        }
    }
    output
}

fn mask_export_image(mask: &SceneAssetMask) -> RgbaImage {
    let mut image = RgbaImage::new(mask.width, mask.height);
    for y in 0..mask.height {
        for x in 0..mask.width {
            let selected = mask.pixels()[mask_index(mask.width, x, y)];
            let value = if selected { 255 } else { 0 };
            image.put_pixel(x, y, Rgba([value, value, value, 255]));
        }
    }
    image
}

fn load_scene_asset_mask(
    mask_path: &Path,
    expected_width: u32,
    expected_height: u32,
) -> Result<SceneAssetMask, SceneAssetEditError> {
    let mask_image = load_rgba_image(mask_path)?;
    if mask_image.width() != expected_width || mask_image.height() != expected_height {
        return Err(SceneAssetEditError::InvalidOperation(format!(
            "mask dimensions differ from source: {}x{} vs {}x{}",
            mask_image.width(),
            mask_image.height(),
            expected_width,
            expected_height
        )));
    }
    let pixels = mask_image
        .pixels()
        .map(|pixel| {
            let luminance = (pixel[0] as u16 + pixel[1] as u16 + pixel[2] as u16) / 3;
            let has_color = pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0;
            pixel[3] > 0 && (luminance >= 128 || (pixel[3] >= 128 && has_color))
        })
        .collect();
    Ok(SceneAssetMask::from_pixels(
        expected_width,
        expected_height,
        pixels,
    ))
}

fn mask_selection_bounds(width: u32, height: u32, pixels: &[bool]) -> Option<SceneAssetPixelRect> {
    let mut min_x = width;
    let mut min_y = height;
    let mut max_x = 0;
    let mut max_y = 0;
    let mut found = false;
    for y in 0..height {
        for x in 0..width {
            if !pixels[mask_index(width, x, y)] {
                continue;
            }
            found = true;
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
    }
    found.then_some(SceneAssetPixelRect {
        x: min_x,
        y: min_y,
        w: max_x.saturating_sub(min_x).saturating_add(1),
        h: max_y.saturating_sub(min_y).saturating_add(1),
    })
}

fn paint_bounds_mask(
    width: u32,
    height: u32,
    whole_image: bool,
    within_regions: &[String],
    within_polygons: &[Vec<SceneAssetNormalizedPoint>],
    protect_regions: &[String],
    feature_map: Option<&SceneAssetFeatureMap>,
) -> Result<SceneAssetMask, SceneAssetEditError> {
    if !whole_image && within_regions.is_empty() && within_polygons.is_empty() {
        return Err(SceneAssetEditError::InvalidOperation(
            "paint operations require --within-regions, --within-polygon, or --whole-image"
                .to_string(),
        ));
    }
    let mut mask =
        SceneAssetMask::from_pixels(width, height, vec![false; width as usize * height as usize]);
    if whole_image {
        mask.select_rect(SceneAssetPixelRect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        });
    }
    if !within_regions.is_empty() {
        let Some(feature_map) = feature_map else {
            return Err(SceneAssetEditError::InvalidOperation(
                "--within-regions requires --protect or --feature-map".to_string(),
            ));
        };
        for region in within_regions {
            let region = region.trim();
            if region.is_empty() {
                continue;
            }
            mask.select_rect(feature_map.pixel_region(region, width, height)?);
        }
    }
    for polygon in within_polygons {
        mask.select_polygon(polygon)?;
    }
    if !protect_regions.is_empty() {
        let Some(feature_map) = feature_map else {
            return Err(SceneAssetEditError::InvalidOperation(
                "--protect-regions requires --protect or --feature-map".to_string(),
            ));
        };
        mask.protect_feature_regions(feature_map, protect_regions)?;
    }
    Ok(mask)
}

fn adjustment_mask(
    width: u32,
    height: u32,
    within_regions: &[String],
    within_polygons: &[Vec<SceneAssetNormalizedPoint>],
    protect_regions: &[String],
    feature_map: Option<&SceneAssetFeatureMap>,
) -> Result<SceneAssetMask, SceneAssetEditError> {
    let whole_image = within_regions.is_empty() && within_polygons.is_empty();
    paint_bounds_mask(
        width,
        height,
        whole_image,
        within_regions,
        within_polygons,
        protect_regions,
        feature_map,
    )
}

fn paint_pixels(
    image: &mut RgbaImage,
    mask: &[bool],
    mut paint: impl FnMut(&mut Rgba<u8>) -> bool,
) -> usize {
    let mut changed_pixels = 0;
    for y in 0..image.height() {
        for x in 0..image.width() {
            if !mask[mask_index(image.width(), x, y)] {
                continue;
            }
            if paint(image.get_pixel_mut(x, y)) {
                changed_pixels += 1;
            }
        }
    }
    changed_pixels
}

fn apply_masked_pixels(image: &mut RgbaImage, mask: &[bool], mut apply: impl FnMut(&mut Rgba<u8>)) {
    for y in 0..image.height() {
        for x in 0..image.width() {
            if !mask[mask_index(image.width(), x, y)] {
                continue;
            }
            apply(image.get_pixel_mut(x, y));
        }
    }
}

fn copy_masked_pixels(source: &RgbaImage, target: &mut RgbaImage, mask: &[bool]) {
    for y in 0..target.height() {
        for x in 0..target.width() {
            if !mask[mask_index(target.width(), x, y)] {
                continue;
            }
            target.put_pixel(x, y, *source.get_pixel(x, y));
        }
    }
}

fn median_sample_color(image: &RgbaImage, x: u32, y: u32, radius: u32) -> Rgba<u8> {
    let min_x = x.saturating_sub(radius);
    let min_y = y.saturating_sub(radius);
    let max_x = x
        .saturating_add(radius)
        .min(image.width().saturating_sub(1));
    let max_y = y
        .saturating_add(radius)
        .min(image.height().saturating_sub(1));
    let mut channels = [Vec::<u8>::new(), Vec::new(), Vec::new(), Vec::new()];
    for sample_y in min_y..=max_y {
        for sample_x in min_x..=max_x {
            let pixel = image.get_pixel(sample_x, sample_y);
            for channel in 0..4 {
                channels[channel].push(pixel[channel]);
            }
        }
    }
    let mut rgba = [0; 4];
    for channel in 0..4 {
        channels[channel].sort_unstable();
        rgba[channel] = channels[channel][channels[channel].len() / 2];
    }
    Rgba(rgba)
}

fn sample_masked_region(
    image: &RgbaImage,
    mask: &[bool],
) -> Result<SceneAssetRegionSample, SceneAssetEditError> {
    let mut channels = [Vec::<u8>::new(), Vec::new(), Vec::new(), Vec::new()];
    let mut sum = [0u64; 4];
    let mut opaque_pixels = 0usize;
    for y in 0..image.height() {
        for x in 0..image.width() {
            if !mask[mask_index(image.width(), x, y)] {
                continue;
            }
            let pixel = image.get_pixel(x, y);
            if pixel[3] > 0 {
                opaque_pixels += 1;
            }
            for channel in 0..4 {
                channels[channel].push(pixel[channel]);
                sum[channel] += pixel[channel] as u64;
            }
        }
    }
    let pixel_count = channels[0].len();
    if pixel_count == 0 {
        return Err(SceneAssetEditError::InvalidOperation(
            "sample region selected zero pixels".to_string(),
        ));
    }
    let mut mean_rgba = [0.0; 4];
    let mut median_rgba = [0; 4];
    for channel in 0..4 {
        channels[channel].sort_unstable();
        mean_rgba[channel] = rounded_f32(sum[channel] as f32 / pixel_count as f32);
        median_rgba[channel] = channels[channel][pixel_count / 2];
    }
    Ok(SceneAssetRegionSample {
        pixel_count,
        mean_rgba,
        median_rgba,
        alpha_coverage: rounded_f32(opaque_pixels as f32 / pixel_count as f32),
    })
}

fn draw_vertical_line(image: &mut RgbaImage, x: u32, color: Rgba<u8>) {
    if x >= image.width() {
        return;
    }
    for y in 0..image.height() {
        image.put_pixel(x, y, color);
    }
}

fn draw_horizontal_line(image: &mut RgbaImage, y: u32, color: Rgba<u8>) {
    if y >= image.height() {
        return;
    }
    for x in 0..image.width() {
        image.put_pixel(x, y, color);
    }
}

fn color_range_mask(
    image: &RgbaImage,
    tolerance: u8,
    sample: SceneAssetBackgroundSample,
) -> Vec<bool> {
    let sample_colors = background_sample_colors(image, sample);
    image
        .pixels()
        .map(|pixel| pixel_matches_any(*pixel, &sample_colors, tolerance))
        .collect()
}

fn multi_seed_contiguous_mask(
    image: &RgbaImage,
    seeds: &[SceneAssetNormalizedPoint],
    tolerance: u8,
) -> Result<SceneAssetMask, SceneAssetEditError> {
    if seeds.is_empty() {
        return Err(SceneAssetEditError::InvalidOperation(
            "magic_erase_add requires at least one --seed".to_string(),
        ));
    }
    let mut output =
        SceneAssetMask::from_pixels(image.width(), image.height(), vec![false; pixel_len(image)]);
    for seed in seeds {
        let seed = normalized_point_to_pixel(*seed, image.width(), image.height())?;
        output.union_pixels(&contiguous_magic_mask(image, &[seed], tolerance));
    }
    Ok(output)
}

fn channel_matte_mask(image: &RgbaImage, threshold: u8, neutrality: u8) -> Vec<bool> {
    image
        .pixels()
        .map(|pixel| {
            let min = pixel[0].min(pixel[1]).min(pixel[2]);
            let max = pixel[0].max(pixel[1]).max(pixel[2]);
            let average = (pixel[0] as u16 + pixel[1] as u16 + pixel[2] as u16) / 3;
            average >= threshold as u16 && max.saturating_sub(min) <= neutrality
        })
        .collect()
}

fn background_sample_colors(
    image: &RgbaImage,
    sample: SceneAssetBackgroundSample,
) -> Vec<Rgba<u8>> {
    if image.width() == 0 || image.height() == 0 {
        return Vec::new();
    }
    match sample {
        SceneAssetBackgroundSample::Corners => {
            let max_x = image.width() - 1;
            let max_y = image.height() - 1;
            vec![
                *image.get_pixel(0, 0),
                *image.get_pixel(max_x, 0),
                *image.get_pixel(0, max_y),
                *image.get_pixel(max_x, max_y),
            ]
        }
        SceneAssetBackgroundSample::Edges => edge_points(image)
            .into_iter()
            .map(|(x, y)| *image.get_pixel(x, y))
            .collect(),
    }
}

fn edge_seed_points_matching_samples(
    image: &RgbaImage,
    sample_colors: &[Rgba<u8>],
    tolerance: u8,
) -> Vec<(u32, u32)> {
    edge_points(image)
        .into_iter()
        .filter(|&(x, y)| pixel_matches_any(*image.get_pixel(x, y), sample_colors, tolerance))
        .collect()
}

fn edge_points(image: &RgbaImage) -> Vec<(u32, u32)> {
    let mut points = Vec::new();
    if image.width() == 0 || image.height() == 0 {
        return points;
    }
    let max_x = image.width() - 1;
    let max_y = image.height() - 1;
    for x in 0..image.width() {
        points.push((x, 0));
        if max_y > 0 {
            points.push((x, max_y));
        }
    }
    for y in 1..max_y {
        points.push((0, y));
        if max_x > 0 {
            points.push((max_x, y));
        }
    }
    points
}

fn contiguous_magic_mask(image: &RgbaImage, seeds: &[(u32, u32)], tolerance: u8) -> Vec<bool> {
    let sample_colors = seeds
        .iter()
        .filter(|&&(x, y)| x < image.width() && y < image.height())
        .map(|&(x, y)| *image.get_pixel(x, y))
        .collect::<Vec<_>>();
    contiguous_magic_mask_with_samples(image, seeds, &sample_colors, tolerance)
}

fn contiguous_magic_mask_with_samples(
    image: &RgbaImage,
    seeds: &[(u32, u32)],
    sample_colors: &[Rgba<u8>],
    tolerance: u8,
) -> Vec<bool> {
    let len = pixel_len(image);
    let mut selected = vec![false; len];
    if sample_colors.is_empty() {
        return selected;
    }
    let mut queue = VecDeque::new();
    for &(x, y) in seeds {
        if x >= image.width() || y >= image.height() {
            continue;
        }
        if !pixel_matches_any(*image.get_pixel(x, y), sample_colors, tolerance) {
            continue;
        }
        let index = pixel_index(image, x, y);
        if !selected[index] {
            selected[index] = true;
            queue.push_back((x, y));
        }
    }
    while let Some((x, y)) = queue.pop_front() {
        for (nx, ny) in neighbor_points(image, x, y) {
            let index = pixel_index(image, nx, ny);
            if selected[index] {
                continue;
            }
            if pixel_matches_any(*image.get_pixel(nx, ny), sample_colors, tolerance) {
                selected[index] = true;
                queue.push_back((nx, ny));
            }
        }
    }
    selected
}

fn global_magic_mask(image: &RgbaImage, sample_color: Rgba<u8>, tolerance: u8) -> Vec<bool> {
    image
        .pixels()
        .map(|pixel| pixel_matches(*pixel, sample_color, tolerance))
        .collect()
}

fn apply_transparency_mask(image: &mut RgbaImage, mask: &[bool], feather: u32) {
    if mask.len() != pixel_len(image) {
        return;
    }
    let mut alpha_factors = vec![1.0f32; mask.len()];
    for (index, selected) in mask.iter().copied().enumerate() {
        if selected {
            alpha_factors[index] = 0.0;
        }
    }
    if feather > 0 {
        let radius = feather as i32;
        for y in 0..image.height() {
            for x in 0..image.width() {
                if !mask[pixel_index(image, x, y)] {
                    continue;
                }
                for dy in -radius..=radius {
                    for dx in -radius..=radius {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        if nx < 0
                            || ny < 0
                            || nx >= image.width() as i32
                            || ny >= image.height() as i32
                        {
                            continue;
                        }
                        let neighbor_index = pixel_index(image, nx as u32, ny as u32);
                        if mask[neighbor_index] {
                            continue;
                        }
                        let distance = dx.abs().max(dy.abs()) as f32;
                        let factor = (distance / (feather as f32 + 1.0)).clamp(0.0, 1.0);
                        alpha_factors[neighbor_index] = alpha_factors[neighbor_index].min(factor);
                    }
                }
            }
        }
    }
    for y in 0..image.height() {
        for x in 0..image.width() {
            let index = pixel_index(image, x, y);
            let factor = alpha_factors[index];
            if factor < 1.0 {
                let pixel = image.get_pixel_mut(x, y);
                pixel[3] = (pixel[3] as f32 * factor).round().clamp(0.0, 255.0) as u8;
            }
        }
    }
}

fn defringe_scene_asset_edges(image: &mut RgbaImage, mode: SceneAssetDefringeMode) {
    if mode == SceneAssetDefringeMode::None {
        return;
    }
    decontaminate_light_edges(image, 4, 0.75, None);
}

fn decontaminate_light_edges(
    image: &mut RgbaImage,
    radius: u32,
    strength: f32,
    region: Option<SceneAssetPixelRect>,
) -> usize {
    let radius = radius.max(1);
    let strength = strength.clamp(0.0, 1.0);
    let source = image.clone();
    let mut changed_pixels = 0;
    for y in 0..source.height() {
        for x in 0..source.width() {
            if let Some(region) = region {
                if x < region.x || x >= region.right() || y < region.y || y >= region.bottom() {
                    continue;
                }
            }
            let pixel = *source.get_pixel(x, y);
            if pixel[3] == 0
                || !is_light_fringe_pixel(pixel)
                || !has_low_alpha_neighbor(&source, x, y, radius.min(3))
            {
                continue;
            }
            let Some(replacement) = nearby_foreground_median(&source, x, y, radius) else {
                continue;
            };
            let output = image.get_pixel_mut(x, y);
            let before = [output[0], output[1], output[2]];
            for channel in 0..3 {
                output[channel] = lerp(
                    output[channel] as f32,
                    replacement[channel] as f32,
                    strength,
                )
                .round()
                .clamp(0.0, 255.0) as u8;
            }
            if before != [output[0], output[1], output[2]] {
                changed_pixels += 1;
            }
        }
    }
    changed_pixels
}

fn nearby_foreground_median(image: &RgbaImage, x: u32, y: u32, radius: u32) -> Option<[u8; 3]> {
    let radius = radius as i32;
    let mut colors = Vec::new();
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx < 0 || ny < 0 || nx >= image.width() as i32 || ny >= image.height() as i32 {
                continue;
            }
            let pixel = *image.get_pixel(nx as u32, ny as u32);
            if pixel[3] >= 220 && !is_light_fringe_pixel(pixel) {
                colors.push([pixel[0], pixel[1], pixel[2]]);
            }
        }
    }
    if colors.is_empty() {
        return None;
    }
    Some([
        median_channel(&colors, 0),
        median_channel(&colors, 1),
        median_channel(&colors, 2),
    ])
}

fn median_channel(colors: &[[u8; 3]], channel: usize) -> u8 {
    let mut values = colors
        .iter()
        .map(|color| color[channel])
        .collect::<Vec<_>>();
    values.sort_unstable();
    values[values.len() / 2]
}

fn has_low_alpha_neighbor(image: &RgbaImage, x: u32, y: u32, radius: u32) -> bool {
    let radius = radius as i32;
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            if dx == 0 && dy == 0 {
                continue;
            }
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx < 0 || ny < 0 || nx >= image.width() as i32 || ny >= image.height() as i32 {
                continue;
            }
            if image.get_pixel(nx as u32, ny as u32)[3] < 32 {
                return true;
            }
        }
    }
    false
}

fn is_light_fringe_pixel(pixel: Rgba<u8>) -> bool {
    let min = pixel[0].min(pixel[1]).min(pixel[2]);
    let max = pixel[0].max(pixel[1]).max(pixel[2]);
    let average = (pixel[0] as u16 + pixel[1] as u16 + pixel[2] as u16) / 3;
    min >= 200 && average >= 220 && max.saturating_sub(min) <= 60
}

fn cutout_quality_report(
    image: &RgbaImage,
    protected_regions: usize,
) -> SceneAssetCutoutQualityReport {
    let mut transparent_pixels = 0;
    let mut partial_alpha_pixels = 0;
    let mut light_edge_pixels = 0;
    for y in 0..image.height() {
        for x in 0..image.width() {
            let pixel = *image.get_pixel(x, y);
            if pixel[3] == 0 {
                transparent_pixels += 1;
            } else if pixel[3] < 255 {
                partial_alpha_pixels += 1;
            }
            if pixel[3] > 0
                && has_low_alpha_neighbor(image, x, y, 2)
                && is_light_fringe_pixel(pixel)
            {
                light_edge_pixels += 1;
            }
        }
    }
    SceneAssetCutoutQualityReport {
        protected_regions,
        transparent_pixels,
        partial_alpha_pixels,
        light_edge_pixels,
    }
}

fn neighbor_points(image: &RgbaImage, x: u32, y: u32) -> Vec<(u32, u32)> {
    let mut neighbors = Vec::with_capacity(4);
    if x > 0 {
        neighbors.push((x - 1, y));
    }
    if y > 0 {
        neighbors.push((x, y - 1));
    }
    if x + 1 < image.width() {
        neighbors.push((x + 1, y));
    }
    if y + 1 < image.height() {
        neighbors.push((x, y + 1));
    }
    neighbors
}

fn pixel_matches_any(pixel: Rgba<u8>, sample_colors: &[Rgba<u8>], tolerance: u8) -> bool {
    sample_colors
        .iter()
        .copied()
        .any(|sample| pixel_matches(pixel, sample, tolerance))
}

fn pixel_matches(pixel: Rgba<u8>, sample: Rgba<u8>, tolerance: u8) -> bool {
    let tolerance = tolerance as i16;
    (pixel[0] as i16 - sample[0] as i16).abs() <= tolerance
        && (pixel[1] as i16 - sample[1] as i16).abs() <= tolerance
        && (pixel[2] as i16 - sample[2] as i16).abs() <= tolerance
}

fn selected_pixel_count(mask: &[bool]) -> usize {
    mask.iter().filter(|selected| **selected).count()
}

fn restore_protected_regions(
    original: &RgbaImage,
    image: &mut RgbaImage,
    feature_map: Option<&SceneAssetFeatureMap>,
    protect_regions: &[String],
) -> Result<(), SceneAssetEditError> {
    if protect_regions.is_empty() {
        return Ok(());
    }
    let Some(feature_map) = feature_map else {
        return Err(SceneAssetEditError::InvalidOperation(
            "--protect-regions requires --protect or --feature-map".to_string(),
        ));
    };
    for region in protect_regions {
        let rect = feature_map.pixel_region(region, image.width(), image.height())?;
        for y in rect.y..rect.bottom().min(image.height()) {
            for x in rect.x..rect.right().min(image.width()) {
                image.put_pixel(x, y, *original.get_pixel(x, y));
            }
        }
    }
    Ok(())
}

fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let r = r as f32 / 255.0;
    let g = g as f32 / 255.0;
    let b = b as f32 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    if (max - min).abs() < f32::EPSILON {
        return (0.0, 0.0, l);
    }
    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };
    let h = if (max - r).abs() < f32::EPSILON {
        60.0 * (((g - b) / d).rem_euclid(6.0))
    } else if (max - g).abs() < f32::EPSILON {
        60.0 * (((b - r) / d) + 2.0)
    } else {
        60.0 * (((r - g) / d) + 4.0)
    };
    (h, s, l)
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> [u8; 3] {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let h_prime = h / 60.0;
    let x = c * (1.0 - (h_prime.rem_euclid(2.0) - 1.0).abs());
    let (r1, g1, b1) = if h_prime < 1.0 {
        (c, x, 0.0)
    } else if h_prime < 2.0 {
        (x, c, 0.0)
    } else if h_prime < 3.0 {
        (0.0, c, x)
    } else if h_prime < 4.0 {
        (0.0, x, c)
    } else if h_prime < 5.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    let m = l - c / 2.0;
    [
        ((r1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((g1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((b1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
    ]
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
