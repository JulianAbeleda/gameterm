use image::imageops::FilterType;
use image::{ImageBuffer, Rgba, RgbaImage};
use serde::Serialize;
use std::collections::{BTreeMap, VecDeque};
use std::convert::TryFrom;
use std::path::{Path, PathBuf};

mod io;
mod model;
mod roots;

use io::{
    load_json, load_rgba_image, read_file, save_rgba_image, sha256_hex, write_file, write_json,
};
pub use model::*;
use roots::{
    pipeline_report_path, resolve_asset_accept_output_path, resolve_asset_accept_source_path,
    resolve_asset_operation_source_path, resolve_asset_prefixed_path, resolve_pipeline_input_path,
    resolve_pipeline_output_path, resolve_recipe_path,
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

pub fn scene_asset_operation_error_report(
    error: &SceneAssetEditError,
) -> SceneAssetOperationErrorReport {
    let (code, hint) = match error {
        SceneAssetEditError::ImageFile { message, .. }
            if message.contains("operation source does not exist") =>
        {
            (
                "missing_source",
                Some("Check the operation `source` path and input root.".to_string()),
            )
        }
        SceneAssetEditError::ImageFile { .. } => (
            "image_file",
            Some(
                "Check that the image path exists and is a readable PNG or supported image file."
                    .to_string(),
            ),
        ),
        SceneAssetEditError::JsonFile { .. } => (
            "json_file",
            Some("Check that the JSON path exists and is readable.".to_string()),
        ),
        SceneAssetEditError::JsonParse { .. } => (
            "json_parse",
            Some("Validate the JSON syntax and schema version.".to_string()),
        ),
        SceneAssetEditError::UnknownRegion(_) => (
            "unknown_region",
            Some("Run map-template or choose a region that exists in the feature map.".to_string()),
        ),
        SceneAssetEditError::InvalidFeatureMap(_) => (
            "invalid_feature_map",
            Some("Run validate-map against the image and feature map.".to_string()),
        ),
        SceneAssetEditError::InvalidColor(_) => (
            "invalid_color",
            Some("Use #rrggbb or #rrggbbaa color syntax.".to_string()),
        ),
        SceneAssetEditError::UnknownExpression(_) => (
            "unknown_expression",
            Some("Choose an expression listed in the recipe book.".to_string()),
        ),
        SceneAssetEditError::UnknownAnimation(_) => (
            "unknown_animation",
            Some("Choose an animation listed in the recipe book.".to_string()),
        ),
        SceneAssetEditError::InvalidOperation(message)
            if message.contains("unsupported pipeline command") =>
        {
            (
                "unknown_command",
                Some(
                    "Choose one of the supported scene_asset_edit operation commands.".to_string(),
                ),
            )
        }
        SceneAssetEditError::InvalidOperation(message)
            if message.contains("is required") || message.contains("requires") =>
        {
            (
                "missing_argument",
                Some("Add the required field or argument to the operation JSON.".to_string()),
            )
        }
        SceneAssetEditError::InvalidOperation(message) if message.contains("must be") => (
            "invalid_argument",
            Some("Adjust the argument type or value in the operation JSON.".to_string()),
        ),
        SceneAssetEditError::InvalidOperation(_) => (
            "invalid_operation",
            Some(
                "Inspect the operation command and args, then run with --dry-run first."
                    .to_string(),
            ),
        ),
        SceneAssetEditError::OutputExists(_) => (
            "output_exists",
            Some(
                "Choose a new output path or pass --force when overwriting is intended."
                    .to_string(),
            ),
        ),
    };
    SceneAssetOperationErrorReport {
        operation: "operation_error".to_string(),
        status: "error".to_string(),
        code: code.to_string(),
        message: error.to_string(),
        hint,
    }
}

pub fn write_scene_asset_review_preview(
    before_path: &Path,
    after_path: &Path,
    output_path: &Path,
    mode: SceneAssetReviewPreviewMode,
    force: bool,
) -> Result<SceneAssetReviewPreviewReport, SceneAssetEditError> {
    let before = load_rgba_image(before_path)?;
    let after = load_rgba_image(after_path)?;
    if before.dimensions() != after.dimensions() {
        return Err(SceneAssetEditError::InvalidOperation(format!(
            "cannot write review preview for different dimensions: {} vs {}",
            before_path.display(),
            after_path.display()
        )));
    }
    let preview = scene_asset_review_preview_image(&before, &after, mode);
    save_rgba_image(&preview, output_path, force)?;
    Ok(SceneAssetReviewPreviewReport {
        operation: "review_preview".to_string(),
        before_path: before_path.display().to_string(),
        after_path: after_path.display().to_string(),
        output_path: output_path.display().to_string(),
        mode,
        report: inspect_scene_asset_image(output_path)?,
    })
}

fn write_scene_asset_operation_review_previews(
    roots: &SceneAssetPipelineRoots,
    id: &str,
    before_path: &Path,
    after_path: &Path,
    force: bool,
) -> Result<SceneAssetReviewPreviewPaths, SceneAssetEditError> {
    let raw_diff = resolve_pipeline_output_path(roots, &operation_review_output(id, "raw-diff"));
    let overlay_diff = resolve_pipeline_output_path(roots, &operation_diff_preview_output(id));
    let alpha_diff =
        resolve_pipeline_output_path(roots, &operation_review_output(id, "alpha-diff"));
    let checkerboard =
        resolve_pipeline_output_path(roots, &operation_review_output(id, "checkerboard"));
    let dark = resolve_pipeline_output_path(roots, &operation_review_output(id, "dark-preview"));
    let contact_sheet = resolve_pipeline_output_path(roots, &operation_review_output(id, "review"));

    write_scene_asset_review_preview(
        before_path,
        after_path,
        &raw_diff,
        SceneAssetReviewPreviewMode::RawDiff,
        force,
    )?;
    write_scene_asset_review_preview(
        before_path,
        after_path,
        &overlay_diff,
        SceneAssetReviewPreviewMode::OverlayDiff,
        force,
    )?;
    write_scene_asset_review_preview(
        before_path,
        after_path,
        &alpha_diff,
        SceneAssetReviewPreviewMode::AlphaDiff,
        force,
    )?;
    write_scene_asset_review_preview(
        before_path,
        after_path,
        &checkerboard,
        SceneAssetReviewPreviewMode::Checkerboard,
        force,
    )?;
    write_scene_asset_review_preview(
        before_path,
        after_path,
        &dark,
        SceneAssetReviewPreviewMode::Dark,
        force,
    )?;
    write_scene_asset_review_preview(
        before_path,
        after_path,
        &contact_sheet,
        SceneAssetReviewPreviewMode::ContactSheet,
        force,
    )?;

    Ok(SceneAssetReviewPreviewPaths {
        raw_diff: Some(raw_diff.display().to_string()),
        overlay_diff: Some(overlay_diff.display().to_string()),
        alpha_diff: Some(alpha_diff.display().to_string()),
        checkerboard: Some(checkerboard.display().to_string()),
        dark: Some(dark.display().to_string()),
        contact_sheet: Some(contact_sheet.display().to_string()),
    })
}

fn scene_asset_review_preview_image(
    before: &RgbaImage,
    after: &RgbaImage,
    mode: SceneAssetReviewPreviewMode,
) -> RgbaImage {
    match mode {
        SceneAssetReviewPreviewMode::RawDiff => raw_diff_preview_image(before, after),
        SceneAssetReviewPreviewMode::OverlayDiff => overlay_diff_preview_image(before, after),
        SceneAssetReviewPreviewMode::AlphaDiff => alpha_diff_preview_image(before, after),
        SceneAssetReviewPreviewMode::Checkerboard => backdrop_preview_image(after, true),
        SceneAssetReviewPreviewMode::Dark => backdrop_preview_image(after, false),
        SceneAssetReviewPreviewMode::ContactSheet => contact_sheet_preview_image(before, after),
    }
}

fn raw_diff_preview_image(before: &RgbaImage, after: &RgbaImage) -> RgbaImage {
    let mut preview = ImageBuffer::from_pixel(after.width(), after.height(), Rgba([0, 0, 0, 0]));
    for y in 0..after.height() {
        for x in 0..after.width() {
            if before.get_pixel(x, y).0 != after.get_pixel(x, y).0 {
                preview.put_pixel(x, y, Rgba([255, 48, 96, 255]));
            }
        }
    }
    preview
}

fn overlay_diff_preview_image(before: &RgbaImage, after: &RgbaImage) -> RgbaImage {
    let mut preview = after.clone();
    for y in 0..after.height() {
        for x in 0..after.width() {
            let before_pixel = before.get_pixel(x, y);
            let after_pixel = after.get_pixel(x, y);
            if before_pixel.0 == after_pixel.0 {
                let dimmed = [
                    (after_pixel[0] as f32 * 0.55).round() as u8,
                    (after_pixel[1] as f32 * 0.55).round() as u8,
                    (after_pixel[2] as f32 * 0.55).round() as u8,
                    after_pixel[3],
                ];
                preview.put_pixel(x, y, Rgba(dimmed));
            } else {
                preview.put_pixel(x, y, Rgba([255, 48, 96, 255]));
            }
        }
    }
    preview
}

fn alpha_diff_preview_image(before: &RgbaImage, after: &RgbaImage) -> RgbaImage {
    let mut preview = ImageBuffer::from_pixel(after.width(), after.height(), Rgba([0, 0, 0, 0]));
    for y in 0..after.height() {
        for x in 0..after.width() {
            let before_pixel = before.get_pixel(x, y);
            let after_pixel = after.get_pixel(x, y);
            if before_pixel[3] != after_pixel[3] {
                preview.put_pixel(x, y, Rgba([64, 220, 255, 255]));
            } else if before_pixel.0 != after_pixel.0 {
                preview.put_pixel(x, y, Rgba([255, 48, 96, 180]));
            }
        }
    }
    preview
}

fn backdrop_preview_image(after: &RgbaImage, checkerboard: bool) -> RgbaImage {
    let mut preview = RgbaImage::new(after.width(), after.height());
    for y in 0..after.height() {
        for x in 0..after.width() {
            let base = if checkerboard {
                let tile = ((x / 8) + (y / 8)) % 2 == 0;
                if tile {
                    Rgba([220, 220, 220, 255])
                } else {
                    Rgba([150, 150, 150, 255])
                }
            } else {
                Rgba([24, 24, 28, 255])
            };
            preview.put_pixel(x, y, alpha_blend(base, *after.get_pixel(x, y)));
        }
    }
    preview
}

fn contact_sheet_preview_image(before: &RgbaImage, after: &RgbaImage) -> RgbaImage {
    let raw_diff = raw_diff_preview_image(before, after);
    let mut sheet = RgbaImage::new(before.width().saturating_mul(3), before.height());
    copy_image_at(before, &mut sheet, 0, 0);
    copy_image_at(after, &mut sheet, before.width(), 0);
    copy_image_at(&raw_diff, &mut sheet, before.width().saturating_mul(2), 0);
    sheet
}

fn copy_image_at(source: &RgbaImage, target: &mut RgbaImage, offset_x: u32, offset_y: u32) {
    for y in 0..source.height() {
        for x in 0..source.width() {
            let tx = offset_x.saturating_add(x);
            let ty = offset_y.saturating_add(y);
            if tx < target.width() && ty < target.height() {
                target.put_pixel(tx, ty, *source.get_pixel(x, y));
            }
        }
    }
}

fn alpha_blend(base: Rgba<u8>, top: Rgba<u8>) -> Rgba<u8> {
    let alpha = top[3] as f32 / 255.0;
    let inv_alpha = 1.0 - alpha;
    Rgba([
        (top[0] as f32 * alpha + base[0] as f32 * inv_alpha).round() as u8,
        (top[1] as f32 * alpha + base[1] as f32 * inv_alpha).round() as u8,
        (top[2] as f32 * alpha + base[2] as f32 * inv_alpha).round() as u8,
        255,
    ])
}

fn operation_preview_output(id: &str) -> String {
    format!("{}.preview.png", sanitize_operation_id(id))
}

fn operation_diff_preview_output(id: &str) -> String {
    format!("{}.diff.png", sanitize_operation_id(id))
}

fn operation_review_output(id: &str, suffix: &str) -> String {
    format!("{}.{}.png", sanitize_operation_id(id), suffix)
}

fn sanitize_operation_id(id: &str) -> String {
    let sanitized = id
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>();
    let trimmed = sanitized.trim_matches('-');
    if trimmed.is_empty() {
        "operation".to_string()
    } else {
        trimmed.to_string()
    }
}

fn operation_expectation_failures(
    expectations: &SceneAssetOperationExpectations,
    compare: Option<&SceneAssetCompareReport>,
    protected_region_report: Option<&SceneAssetProtectedRegionReport>,
) -> Vec<String> {
    let mut failures = Vec::new();
    if let (Some(max_ratio), Some(compare)) = (expectations.max_changed_pixel_ratio, compare) {
        if compare.changed_pixel_ratio > max_ratio {
            failures.push(format!(
                "changed_pixel_ratio {} exceeded max_changed_pixel_ratio {}",
                compare.changed_pixel_ratio, max_ratio
            ));
        }
    }
    if !expectations.must_preserve_regions.is_empty() {
        match protected_region_report {
            Some(report) => {
                let max_changed = expectations
                    .max_changed_pixels_in_protected_regions
                    .unwrap_or(0);
                if report.changed_pixels > max_changed {
                    failures.push(format!(
                        "protected regions changed {} pixels, exceeding max_changed_pixels_in_protected_regions {}",
                        report.changed_pixels, max_changed
                    ));
                }
            }
            None => failures.push(
                "must_preserve_regions requires a feature map and comparable output".to_string(),
            ),
        }
    }
    failures
}

fn compare_protected_regions(
    before_path: &Path,
    after_path: &Path,
    feature_map: &SceneAssetFeatureMap,
    regions: &[String],
) -> Result<SceneAssetProtectedRegionReport, SceneAssetEditError> {
    let before = load_rgba_image(before_path)?;
    let after = load_rgba_image(after_path)?;
    if before.dimensions() != after.dimensions() {
        return Err(SceneAssetEditError::InvalidOperation(format!(
            "protected-region compare dimensions differ: {}x{} vs {}x{}",
            before.width(),
            before.height(),
            after.width(),
            after.height()
        )));
    }
    validate_scene_asset_feature_map(feature_map, before.width(), before.height())?;
    let mut changed_regions = Vec::new();
    let mut changed_pixels = 0;
    for region in regions {
        let rect = feature_map.pixel_region(region, before.width(), before.height())?;
        let mut region_changed_pixels = 0;
        for y in rect.y..rect.bottom().min(before.height()) {
            for x in rect.x..rect.right().min(before.width()) {
                if before.get_pixel(x, y).0 != after.get_pixel(x, y).0 {
                    region_changed_pixels += 1;
                }
            }
        }
        changed_pixels += region_changed_pixels;
        if region_changed_pixels > 0 {
            changed_regions.push(SceneAssetProtectedRegionChange {
                region: region.clone(),
                changed_pixels: region_changed_pixels,
            });
        }
    }
    Ok(SceneAssetProtectedRegionReport {
        checked_regions: regions.to_vec(),
        changed_pixels,
        changed_regions,
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

pub fn composite_scene_asset_layers(
    output_path: &Path,
    options: SceneAssetCompositeOptions,
    base_dir: Option<&Path>,
    force: bool,
) -> Result<SceneAssetCompositeReport, SceneAssetEditError> {
    if options.layers.is_empty() {
        return Err(SceneAssetEditError::InvalidOperation(
            "composite requires at least one layer".to_string(),
        ));
    }
    let mut loaded = Vec::with_capacity(options.layers.len());
    for layer in &options.layers {
        if !layer.opacity.is_finite() {
            return Err(SceneAssetEditError::InvalidOperation(
                "layer opacity must be finite".to_string(),
            ));
        }
        let path = resolve_recipe_path(&layer.path, base_dir);
        loaded.push((layer, load_rgba_image(&path)?));
    }
    let width = options.width.unwrap_or_else(|| loaded[0].1.width());
    let height = options.height.unwrap_or_else(|| loaded[0].1.height());
    let mut output = ImageBuffer::from_pixel(width, height, Rgba([0, 0, 0, 0]));
    for (layer, image) in loaded {
        paste_layer(
            &mut output,
            &image,
            layer.x_offset,
            layer.y_offset,
            layer.opacity,
            layer.blend,
        );
    }
    save_rgba_image(&output, output_path, force)?;
    Ok(SceneAssetCompositeReport {
        operation: "composite".to_string(),
        output_path: output_path.display().to_string(),
        layer_count: options.layers.len(),
        report: inspect_scene_asset_image(output_path)?,
    })
}

pub fn create_scene_asset_state_manifest(
    base_path: &Path,
    output_path: &Path,
    options: SceneAssetStateManifestOptions,
    force: bool,
) -> Result<SceneAssetStateManifest, SceneAssetEditError> {
    inspect_scene_asset_image(base_path)?;
    let mut parts = BTreeMap::new();
    for (part_name, files) in options.parts {
        if files.is_empty() {
            return Err(SceneAssetEditError::InvalidOperation(format!(
                "state part `{part_name}` requires at least one state file"
            )));
        }
        let mut states = BTreeMap::new();
        for file in files {
            let state_name = Path::new(&file)
                .file_stem()
                .map(|stem| stem.to_string_lossy().to_string())
                .unwrap_or_else(|| file.clone());
            states.insert(state_name, file);
        }
        let default = states.keys().next().cloned().unwrap_or_default();
        parts.insert(part_name, SceneAssetStatePart { default, states });
    }
    let manifest = SceneAssetStateManifest {
        asset_state_version: 1,
        character: options.character,
        base: base_path.display().to_string(),
        parts,
    };
    write_scene_asset_json(output_path, &manifest, true, force)?;
    Ok(manifest)
}

pub fn load_scene_asset_state_manifest(
    path: &Path,
) -> Result<SceneAssetStateManifest, SceneAssetEditError> {
    load_json(path)
}

pub fn render_scene_asset_state(
    manifest_path: &Path,
    output_path: &Path,
    options: SceneAssetStateRenderOptions,
    force: bool,
) -> Result<SceneAssetCompositeReport, SceneAssetEditError> {
    let manifest = load_scene_asset_state_manifest(manifest_path)?;
    let base_dir = manifest_path.parent();
    let composite = state_composite_options(&manifest, &options.states)?;
    composite_scene_asset_layers(output_path, composite, base_dir, force)
}

pub fn render_scene_asset_state_sheet(
    manifest_path: &Path,
    frames_path: &Path,
    output_path: &Path,
    index_path: &Path,
    force: bool,
) -> Result<SceneAssetCompositeReport, SceneAssetEditError> {
    let manifest = load_scene_asset_state_manifest(manifest_path)?;
    let frames: Vec<SceneAssetStateSheetFrame> = load_json(frames_path)?;
    if frames.is_empty() {
        return Err(SceneAssetEditError::InvalidOperation(
            "state-sheet requires at least one frame".to_string(),
        ));
    }
    let base_dir = manifest_path.parent();
    let base_path = resolve_recipe_path(&manifest.base, base_dir);
    let base = load_rgba_image(&base_path)?;
    let mut sheet = ImageBuffer::from_pixel(
        base.width() * frames.len() as u32,
        base.height(),
        Rgba([0, 0, 0, 0]),
    );
    let mut index_frames = Vec::new();
    for (index, frame) in frames.iter().enumerate() {
        let composite = state_composite_options(&manifest, &frame.states)?;
        let rendered = render_composite_to_image(composite, base_dir)?;
        let x = base.width() * index as u32;
        paste_region(&mut sheet, &rendered, x as i32, 0, 1.0);
        index_frames.push(SceneAssetStateSheetIndexFrame {
            index,
            label: frame.label.clone(),
            x,
            y: 0,
            w: base.width(),
            h: base.height(),
            states: frame.states.clone(),
        });
    }
    save_rgba_image(&sheet, output_path, force)?;
    write_scene_asset_json(
        index_path,
        &SceneAssetStateSheetIndex {
            frames: index_frames,
        },
        true,
        force,
    )?;
    Ok(SceneAssetCompositeReport {
        operation: "state_sheet".to_string(),
        output_path: output_path.display().to_string(),
        layer_count: frames.len(),
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

fn pipeline_command_advances_source(command: &str) -> bool {
    !matches!(command, "sample" | "mask-preview" | "mask-export")
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

fn validate_mask_polish_regions(
    feature_map: Option<&SceneAssetFeatureMap>,
    polish: &SceneAssetMaskPolishOptions,
) -> Result<(), SceneAssetEditError> {
    validate_pipeline_region_names(feature_map, &polish.within_regions)?;
    validate_pipeline_region_names(feature_map, &polish.protect_regions)
}

fn validate_adjustment_regions(
    args: &BTreeMap<String, serde_json::Value>,
    feature_map: Option<&SceneAssetFeatureMap>,
) -> Result<(), SceneAssetEditError> {
    let within_regions = pipeline_string_list_arg(args, "within_regions")?;
    pipeline_polygons_arg(args, "within_polygons")?;
    let protect_regions = pipeline_string_list_arg(args, "protect_regions")?;
    validate_pipeline_region_names(feature_map, &within_regions)?;
    validate_pipeline_region_names(feature_map, &protect_regions)
}

fn validate_pipeline_region_names(
    feature_map: Option<&SceneAssetFeatureMap>,
    regions: &[String],
) -> Result<(), SceneAssetEditError> {
    if regions.is_empty() {
        return Ok(());
    }
    let Some(feature_map) = feature_map else {
        return Err(SceneAssetEditError::InvalidOperation(
            "feature map is required when using named regions".to_string(),
        ));
    };
    for region in regions {
        if !feature_map.regions.contains_key(region) {
            return Err(SceneAssetEditError::UnknownRegion(region.clone()));
        }
    }
    Ok(())
}

fn load_pipeline_feature_map(
    roots: &SceneAssetPipelineRoots,
    args: &BTreeMap<String, serde_json::Value>,
) -> Result<Option<SceneAssetFeatureMap>, SceneAssetEditError> {
    let path = pipeline_string_arg(args, "protect")?
        .or(pipeline_string_arg(args, "feature_map")?)
        .map(|path| resolve_pipeline_input_path(roots, &path));
    path.map(|path| load_scene_asset_feature_map(&path))
        .transpose()
}

fn pipeline_arg<'a>(
    args: &'a BTreeMap<String, serde_json::Value>,
    key: &str,
) -> Option<&'a serde_json::Value> {
    args.get(key).or_else(|| {
        let kebab = key.replace('_', "-");
        args.get(&kebab)
    })
}

fn pipeline_string_arg(
    args: &BTreeMap<String, serde_json::Value>,
    key: &str,
) -> Result<Option<String>, SceneAssetEditError> {
    let Some(value) = pipeline_arg(args, key) else {
        return Ok(None);
    };
    value
        .as_str()
        .map(|value| Some(value.to_string()))
        .ok_or_else(|| {
            SceneAssetEditError::InvalidOperation(format!("pipeline arg `{key}` must be a string"))
        })
}

fn pipeline_required_asset_path_arg(
    roots: &SceneAssetPipelineRoots,
    args: &BTreeMap<String, serde_json::Value>,
    key: &str,
) -> Result<PathBuf, SceneAssetEditError> {
    let path = pipeline_string_arg(args, key)?.ok_or_else(|| {
        SceneAssetEditError::InvalidOperation(format!("pipeline arg `{key}` is required"))
    })?;
    Ok(resolve_asset_prefixed_path(
        roots,
        Path::new(&path),
        &roots.transformation_root,
    ))
}

fn pipeline_string_list_arg(
    args: &BTreeMap<String, serde_json::Value>,
    key: &str,
) -> Result<Vec<String>, SceneAssetEditError> {
    let Some(value) = pipeline_arg(args, key) else {
        return Ok(Vec::new());
    };
    match value {
        serde_json::Value::String(text) => Ok(text
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
            .collect()),
        serde_json::Value::Array(values) => values
            .iter()
            .map(|value| {
                value.as_str().map(ToString::to_string).ok_or_else(|| {
                    SceneAssetEditError::InvalidOperation(format!(
                        "pipeline arg `{key}` entries must be strings"
                    ))
                })
            })
            .collect(),
        _ => Err(SceneAssetEditError::InvalidOperation(format!(
            "pipeline arg `{key}` must be a string or string array"
        ))),
    }
}

fn pipeline_u8_arg(
    args: &BTreeMap<String, serde_json::Value>,
    key: &str,
    default: u8,
) -> Result<u8, SceneAssetEditError> {
    let Some(value) = pipeline_arg(args, key) else {
        return Ok(default);
    };
    let number = value.as_u64().ok_or_else(|| {
        SceneAssetEditError::InvalidOperation(format!("pipeline arg `{key}` must be a number"))
    })?;
    u8::try_from(number).map_err(|_| {
        SceneAssetEditError::InvalidOperation(format!("pipeline arg `{key}` must fit in u8"))
    })
}

fn pipeline_u32_arg(
    args: &BTreeMap<String, serde_json::Value>,
    key: &str,
    default: u32,
) -> Result<u32, SceneAssetEditError> {
    let Some(value) = pipeline_arg(args, key) else {
        return Ok(default);
    };
    let number = value.as_u64().ok_or_else(|| {
        SceneAssetEditError::InvalidOperation(format!("pipeline arg `{key}` must be a number"))
    })?;
    u32::try_from(number).map_err(|_| {
        SceneAssetEditError::InvalidOperation(format!("pipeline arg `{key}` must fit in u32"))
    })
}

fn pipeline_u32_required_arg(
    args: &BTreeMap<String, serde_json::Value>,
    key: &str,
) -> Result<u32, SceneAssetEditError> {
    if pipeline_arg(args, key).is_none() {
        return Err(SceneAssetEditError::InvalidOperation(format!(
            "pipeline arg `{key}` is required"
        )));
    }
    pipeline_u32_arg(args, key, 0)
}

fn pipeline_usize_arg(
    args: &BTreeMap<String, serde_json::Value>,
    key: &str,
    default: usize,
) -> Result<usize, SceneAssetEditError> {
    let Some(value) = pipeline_arg(args, key) else {
        return Ok(default);
    };
    let number = value.as_u64().ok_or_else(|| {
        SceneAssetEditError::InvalidOperation(format!("pipeline arg `{key}` must be a number"))
    })?;
    usize::try_from(number).map_err(|_| {
        SceneAssetEditError::InvalidOperation(format!("pipeline arg `{key}` must fit in usize"))
    })
}

fn pipeline_f32_arg(
    args: &BTreeMap<String, serde_json::Value>,
    key: &str,
    default: f32,
) -> Result<f32, SceneAssetEditError> {
    let Some(value) = pipeline_arg(args, key) else {
        return Ok(default);
    };
    let number = value.as_f64().ok_or_else(|| {
        SceneAssetEditError::InvalidOperation(format!("pipeline arg `{key}` must be a number"))
    })?;
    Ok(number as f32)
}

fn pipeline_bool_arg(
    args: &BTreeMap<String, serde_json::Value>,
    key: &str,
    default: bool,
) -> Result<bool, SceneAssetEditError> {
    let Some(value) = pipeline_arg(args, key) else {
        return Ok(default);
    };
    value.as_bool().ok_or_else(|| {
        SceneAssetEditError::InvalidOperation(format!("pipeline arg `{key}` must be a boolean"))
    })
}

fn pipeline_color_arg(
    args: &BTreeMap<String, serde_json::Value>,
    key: &str,
) -> Result<[u8; 4], SceneAssetEditError> {
    let Some(color) = pipeline_string_arg(args, key)? else {
        return Err(SceneAssetEditError::InvalidOperation(format!(
            "pipeline arg `{key}` is required"
        )));
    };
    Ok(parse_rgba(&color)?.0)
}

fn pipeline_optional_color_arg(
    args: &BTreeMap<String, serde_json::Value>,
    key: &str,
    default: [u8; 4],
) -> Result<[u8; 4], SceneAssetEditError> {
    let Some(color) = pipeline_string_arg(args, key)? else {
        return Ok(default);
    };
    Ok(parse_rgba(&color)?.0)
}

fn pipeline_background_sample_arg(
    args: &BTreeMap<String, serde_json::Value>,
    key: &str,
) -> Result<SceneAssetBackgroundSample, SceneAssetEditError> {
    match pipeline_string_arg(args, key)?
        .as_deref()
        .unwrap_or("corners")
    {
        "corners" => Ok(SceneAssetBackgroundSample::Corners),
        "edges" => Ok(SceneAssetBackgroundSample::Edges),
        value => Err(SceneAssetEditError::InvalidOperation(format!(
            "pipeline arg `{key}` value `{value}` is invalid; expected corners or edges"
        ))),
    }
}

fn pipeline_pad_anchor_arg(
    args: &BTreeMap<String, serde_json::Value>,
) -> Result<SceneAssetPadAnchor, SceneAssetEditError> {
    match pipeline_string_arg(args, "anchor")?
        .as_deref()
        .unwrap_or("center")
    {
        "center" => Ok(SceneAssetPadAnchor::Center),
        "bottom-center" | "bottom_center" => Ok(SceneAssetPadAnchor::BottomCenter),
        "top-left" | "top_left" => Ok(SceneAssetPadAnchor::TopLeft),
        value => Err(SceneAssetEditError::InvalidOperation(format!(
            "pipeline arg `anchor` value `{value}` is invalid"
        ))),
    }
}

fn pipeline_resample_arg(
    args: &BTreeMap<String, serde_json::Value>,
) -> Result<SceneAssetResampleFilter, SceneAssetEditError> {
    match pipeline_string_arg(args, "resample")?
        .as_deref()
        .unwrap_or("lanczos3")
    {
        "nearest" => Ok(SceneAssetResampleFilter::Nearest),
        "lanczos3" => Ok(SceneAssetResampleFilter::Lanczos3),
        value => Err(SceneAssetEditError::InvalidOperation(format!(
            "pipeline arg `resample` value `{value}` is invalid"
        ))),
    }
}

fn pipeline_channel_arg(
    args: &BTreeMap<String, serde_json::Value>,
) -> Result<SceneAssetColorChannel, SceneAssetEditError> {
    match pipeline_string_arg(args, "channel")?
        .as_deref()
        .unwrap_or("rgb")
    {
        "rgb" => Ok(SceneAssetColorChannel::Rgb),
        "r" => Ok(SceneAssetColorChannel::R),
        "g" => Ok(SceneAssetColorChannel::G),
        "b" => Ok(SceneAssetColorChannel::B),
        "a" => Ok(SceneAssetColorChannel::A),
        value => Err(SceneAssetEditError::InvalidOperation(format!(
            "pipeline arg `channel` value `{value}` is invalid"
        ))),
    }
}

fn pipeline_translate_arg(
    args: &BTreeMap<String, serde_json::Value>,
) -> Result<(i32, i32), SceneAssetEditError> {
    let Some(value) = pipeline_arg(args, "translate") else {
        return Ok((0, 0));
    };
    match value {
        serde_json::Value::String(text) => parse_i32_pair_text(text, "translate"),
        serde_json::Value::Array(values) if values.len() == 2 => {
            let x = values[0].as_i64().ok_or_else(|| {
                SceneAssetEditError::InvalidOperation(
                    "pipeline translate x must be a number".to_string(),
                )
            })?;
            let y = values[1].as_i64().ok_or_else(|| {
                SceneAssetEditError::InvalidOperation(
                    "pipeline translate y must be a number".to_string(),
                )
            })?;
            Ok((x as i32, y as i32))
        }
        _ => Err(SceneAssetEditError::InvalidOperation(
            "pipeline translate must be `X,Y` or [X,Y]".to_string(),
        )),
    }
}

fn pipeline_draw_shape_arg(
    args: &BTreeMap<String, serde_json::Value>,
) -> Result<SceneAssetDrawShapeKind, SceneAssetEditError> {
    match pipeline_string_arg(args, "shape")?
        .as_deref()
        .unwrap_or("line")
    {
        "rect" => Ok(SceneAssetDrawShapeKind::Rect),
        "line" => Ok(SceneAssetDrawShapeKind::Line),
        "polygon" => Ok(SceneAssetDrawShapeKind::Polygon),
        "ellipse" | "circle" => Ok(SceneAssetDrawShapeKind::Ellipse),
        value => Err(SceneAssetEditError::InvalidOperation(format!(
            "pipeline arg `shape` value `{value}` is invalid"
        ))),
    }
}

fn pipeline_rect_arg(
    args: &BTreeMap<String, serde_json::Value>,
    key: &str,
) -> Result<Option<SceneAssetNormalizedRect>, SceneAssetEditError> {
    let Some(value) = pipeline_arg(args, key) else {
        return Ok(None);
    };
    match value {
        serde_json::Value::String(text) => parse_normalized_rect_text(text, key).map(Some),
        serde_json::Value::Object(object) => {
            let x = object.get("x").and_then(serde_json::Value::as_f64);
            let y = object.get("y").and_then(serde_json::Value::as_f64);
            let w = object.get("w").and_then(serde_json::Value::as_f64);
            let h = object.get("h").and_then(serde_json::Value::as_f64);
            match (x, y, w, h) {
                (Some(x), Some(y), Some(w), Some(h)) => Ok(Some(SceneAssetNormalizedRect {
                    x: x as f32,
                    y: y as f32,
                    w: w as f32,
                    h: h as f32,
                })),
                _ => Err(SceneAssetEditError::InvalidOperation(format!(
                    "pipeline rect `{key}` must include numeric x, y, w, and h"
                ))),
            }
        }
        _ => Err(SceneAssetEditError::InvalidOperation(format!(
            "pipeline rect `{key}` must be `X,Y,W,H` or an object"
        ))),
    }
}

fn pipeline_mask_preview_mode_arg(
    args: &BTreeMap<String, serde_json::Value>,
) -> Result<SceneAssetMaskPreviewMode, SceneAssetEditError> {
    match pipeline_string_arg(args, "selection_mode")?
        .as_deref()
        .unwrap_or("background")
    {
        "background" => Ok(SceneAssetMaskPreviewMode::Background),
        "color-range" | "color_range" => Ok(SceneAssetMaskPreviewMode::ColorRange),
        "magic-add" | "magic_add" => Ok(SceneAssetMaskPreviewMode::MagicAdd),
        "channel-matte" | "channel_matte" => Ok(SceneAssetMaskPreviewMode::ChannelMatte),
        value => Err(SceneAssetEditError::InvalidOperation(format!(
            "pipeline arg `selection_mode` value `{value}` is invalid"
        ))),
    }
}

fn pipeline_mask_polish_options(
    args: &BTreeMap<String, serde_json::Value>,
) -> Result<SceneAssetMaskPolishOptions, SceneAssetEditError> {
    Ok(SceneAssetMaskPolishOptions {
        tolerance: pipeline_u8_arg(args, "tolerance", 24)?,
        feather: pipeline_u32_arg(args, "feather", 0)?,
        sample: pipeline_background_sample_arg(args, "sample")?,
        erode: pipeline_u32_arg(args, "erode", 0)?,
        dilate: pipeline_u32_arg(args, "dilate", 0)?,
        open: pipeline_u32_arg(args, "open", 0)?,
        close: pipeline_u32_arg(args, "close", 0)?,
        remove_small: pipeline_usize_arg(args, "remove_small", 0)?,
        fill_holes: pipeline_usize_arg(args, "fill_holes", 0)?,
        defringe: match pipeline_string_arg(args, "defringe")?
            .as_deref()
            .unwrap_or("none")
        {
            "none" => SceneAssetDefringeMode::None,
            "white" => SceneAssetDefringeMode::White,
            value => {
                return Err(SceneAssetEditError::InvalidOperation(format!(
                    "pipeline arg `defringe` value `{value}` is invalid"
                )))
            }
        },
        protect_regions: pipeline_string_list_arg(args, "protect_regions")?,
        within_regions: pipeline_string_list_arg(args, "within_regions")?,
        within_polygons: pipeline_polygons_arg(args, "within_polygons")?,
    })
}

fn pipeline_points_arg(
    args: &BTreeMap<String, serde_json::Value>,
    plural_key: &str,
    singular_key: &str,
) -> Result<Vec<SceneAssetNormalizedPoint>, SceneAssetEditError> {
    let mut points = Vec::new();
    if let Some(value) = pipeline_arg(args, plural_key) {
        match value {
            serde_json::Value::Array(values) => {
                for value in values {
                    points.push(pipeline_point_value(value, plural_key)?);
                }
            }
            _ => points.push(pipeline_point_value(value, plural_key)?),
        }
    }
    if let Some(value) = pipeline_arg(args, singular_key) {
        points.push(pipeline_point_value(value, singular_key)?);
    }
    Ok(points)
}

fn pipeline_required_point_arg(
    args: &BTreeMap<String, serde_json::Value>,
    key: &str,
) -> Result<SceneAssetNormalizedPoint, SceneAssetEditError> {
    let Some(value) = pipeline_arg(args, key) else {
        return Err(SceneAssetEditError::InvalidOperation(format!(
            "pipeline arg `{key}` is required"
        )));
    };
    pipeline_point_value(value, key)
}

fn pipeline_point_value(
    value: &serde_json::Value,
    key: &str,
) -> Result<SceneAssetNormalizedPoint, SceneAssetEditError> {
    match value {
        serde_json::Value::String(text) => parse_normalized_point_text(text, key),
        serde_json::Value::Object(object) => {
            let x = object.get("x").and_then(serde_json::Value::as_f64);
            let y = object.get("y").and_then(serde_json::Value::as_f64);
            match (x, y) {
                (Some(x), Some(y)) => Ok(SceneAssetNormalizedPoint {
                    x: x as f32,
                    y: y as f32,
                }),
                _ => Err(SceneAssetEditError::InvalidOperation(format!(
                    "pipeline point `{key}` must include numeric x and y"
                ))),
            }
        }
        _ => Err(SceneAssetEditError::InvalidOperation(format!(
            "pipeline point `{key}` must be `X,Y` or {{\"x\":N,\"y\":N}}"
        ))),
    }
}

fn pipeline_polygons_arg(
    args: &BTreeMap<String, serde_json::Value>,
    key: &str,
) -> Result<Vec<Vec<SceneAssetNormalizedPoint>>, SceneAssetEditError> {
    let mut polygons = Vec::new();
    if let Some(value) = pipeline_arg(args, key) {
        match value {
            serde_json::Value::Array(values) => {
                for value in values {
                    polygons.push(pipeline_polygon_value(value, key)?);
                }
            }
            _ => polygons.push(pipeline_polygon_value(value, key)?),
        }
    }
    if let Some(value) = pipeline_arg(args, "within_polygon") {
        polygons.push(pipeline_polygon_value(value, "within_polygon")?);
    }
    Ok(polygons)
}

fn pipeline_polygon_value(
    value: &serde_json::Value,
    key: &str,
) -> Result<Vec<SceneAssetNormalizedPoint>, SceneAssetEditError> {
    match value {
        serde_json::Value::String(text) => parse_normalized_polygon_text(text, key),
        serde_json::Value::Array(values) => values
            .iter()
            .map(|value| pipeline_point_value(value, key))
            .collect(),
        _ => Err(SceneAssetEditError::InvalidOperation(format!(
            "pipeline polygon `{key}` must be a string or point array"
        ))),
    }
}

fn pipeline_path_points_arg(
    args: &BTreeMap<String, serde_json::Value>,
    key: &str,
) -> Result<Vec<SceneAssetNormalizedPoint>, SceneAssetEditError> {
    let Some(value) = pipeline_arg(args, key) else {
        return Err(SceneAssetEditError::InvalidOperation(format!(
            "pipeline arg `{key}` is required"
        )));
    };
    let points = match value {
        serde_json::Value::String(text) => text
            .split(';')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|point| parse_normalized_point_text(point, key))
            .collect::<Result<Vec<_>, _>>()?,
        serde_json::Value::Array(values) => values
            .iter()
            .map(|value| pipeline_point_value(value, key))
            .collect::<Result<Vec<_>, _>>()?,
        _ => {
            return Err(SceneAssetEditError::InvalidOperation(format!(
                "pipeline path `{key}` must be a string or point array"
            )))
        }
    };
    if points.len() < 2 {
        return Err(SceneAssetEditError::InvalidOperation(format!(
            "pipeline path `{key}` requires at least two points"
        )));
    }
    Ok(points)
}

fn parse_normalized_polygon_text(
    value: &str,
    key: &str,
) -> Result<Vec<SceneAssetNormalizedPoint>, SceneAssetEditError> {
    let points = value
        .split(';')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|point| parse_normalized_point_text(point, key))
        .collect::<Result<Vec<_>, _>>()?;
    validate_polygon(&points)?;
    Ok(points)
}

fn parse_normalized_rect_text(
    value: &str,
    key: &str,
) -> Result<SceneAssetNormalizedRect, SceneAssetEditError> {
    let parts = value.split(',').map(str::trim).collect::<Vec<_>>();
    if parts.len() != 4 {
        return Err(SceneAssetEditError::InvalidOperation(format!(
            "pipeline rect `{key}` value `{value}` is invalid; expected X,Y,W,H"
        )));
    }
    let parse = |index: usize| {
        parts[index].parse::<f32>().map_err(|err| {
            SceneAssetEditError::InvalidOperation(format!(
                "pipeline rect `{key}` value `{}` is invalid: {err}",
                parts[index]
            ))
        })
    };
    Ok(SceneAssetNormalizedRect {
        x: parse(0)?,
        y: parse(1)?,
        w: parse(2)?,
        h: parse(3)?,
    })
}

fn parse_i32_pair_text(value: &str, key: &str) -> Result<(i32, i32), SceneAssetEditError> {
    let (x, y) = value.split_once(',').ok_or_else(|| {
        SceneAssetEditError::InvalidOperation(format!(
            "pipeline `{key}` value `{value}` is invalid; expected X,Y"
        ))
    })?;
    let x = x.trim().parse::<i32>().map_err(|err| {
        SceneAssetEditError::InvalidOperation(format!(
            "pipeline `{key}` x value `{x}` is invalid: {err}"
        ))
    })?;
    let y = y.trim().parse::<i32>().map_err(|err| {
        SceneAssetEditError::InvalidOperation(format!(
            "pipeline `{key}` y value `{y}` is invalid: {err}"
        ))
    })?;
    Ok((x, y))
}

fn parse_normalized_point_text(
    value: &str,
    key: &str,
) -> Result<SceneAssetNormalizedPoint, SceneAssetEditError> {
    let (x, y) = value.split_once(',').ok_or_else(|| {
        SceneAssetEditError::InvalidOperation(format!(
            "pipeline point `{key}` value `{value}` is invalid; expected X,Y"
        ))
    })?;
    let x = x.trim().parse::<f32>().map_err(|err| {
        SceneAssetEditError::InvalidOperation(format!(
            "pipeline point `{key}` x value `{x}` is invalid: {err}"
        ))
    })?;
    let y = y.trim().parse::<f32>().map_err(|err| {
        SceneAssetEditError::InvalidOperation(format!(
            "pipeline point `{key}` y value `{y}` is invalid: {err}"
        ))
    })?;
    Ok(SceneAssetNormalizedPoint { x, y })
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

fn normalized_point_to_pixel(
    point: SceneAssetNormalizedPoint,
    width: u32,
    height: u32,
) -> Result<(u32, u32), SceneAssetEditError> {
    if !point.x.is_finite()
        || !point.y.is_finite()
        || point.x < 0.0
        || point.y < 0.0
        || point.x > 1.0
        || point.y > 1.0
    {
        return Err(SceneAssetEditError::InvalidOperation(
            "seed point must be finite and inside 0..1".to_string(),
        ));
    }
    if width == 0 || height == 0 {
        return Err(SceneAssetEditError::InvalidOperation(
            "image dimensions must be non-zero".to_string(),
        ));
    }
    Ok((
        (point.x * width.saturating_sub(1) as f32).round() as u32,
        (point.y * height.saturating_sub(1) as f32).round() as u32,
    ))
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct SceneAssetMask {
    width: u32,
    height: u32,
    pixels: Vec<bool>,
}

impl SceneAssetMask {
    fn from_pixels(width: u32, height: u32, pixels: Vec<bool>) -> Self {
        debug_assert_eq!(pixels.len(), width as usize * height as usize);
        Self {
            width,
            height,
            pixels,
        }
    }

    fn pixels(&self) -> &[bool] {
        &self.pixels
    }

    fn len(&self) -> usize {
        self.pixels.len()
    }

    fn selected_count(&self) -> usize {
        selected_pixel_count(&self.pixels)
    }

    fn union_pixels(&mut self, pixels: &[bool]) {
        if pixels.len() != self.pixels.len() {
            return;
        }
        for (target, selected) in self.pixels.iter_mut().zip(pixels.iter().copied()) {
            *target |= selected;
        }
    }

    fn intersect_pixels(&mut self, pixels: &[bool]) {
        if pixels.len() != self.pixels.len() {
            return;
        }
        for (target, selected) in self.pixels.iter_mut().zip(pixels.iter().copied()) {
            *target &= selected;
        }
    }

    fn eroded(&self, radius: u32) -> Self {
        if radius == 0 {
            return self.clone();
        }
        let radius = radius as i32;
        let mut pixels = vec![false; self.pixels.len()];
        for y in 0..self.height {
            for x in 0..self.width {
                let index = mask_index(self.width, x, y);
                if !self.pixels[index] {
                    continue;
                }
                let mut keep = true;
                'neighbors: for dy in -radius..=radius {
                    for dx in -radius..=radius {
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        if nx < 0 || ny < 0 || nx >= self.width as i32 || ny >= self.height as i32 {
                            continue;
                        }
                        if !self.pixels[mask_index(self.width, nx as u32, ny as u32)] {
                            keep = false;
                            break 'neighbors;
                        }
                    }
                }
                pixels[index] = keep;
            }
        }
        Self::from_pixels(self.width, self.height, pixels)
    }

    fn dilated(&self, radius: u32) -> Self {
        if radius == 0 {
            return self.clone();
        }
        let radius = radius as i32;
        let mut pixels = vec![false; self.pixels.len()];
        for y in 0..self.height {
            for x in 0..self.width {
                let mut selected = false;
                'neighbors: for dy in -radius..=radius {
                    for dx in -radius..=radius {
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        if nx < 0 || ny < 0 || nx >= self.width as i32 || ny >= self.height as i32 {
                            continue;
                        }
                        if self.pixels[mask_index(self.width, nx as u32, ny as u32)] {
                            selected = true;
                            break 'neighbors;
                        }
                    }
                }
                pixels[mask_index(self.width, x, y)] = selected;
            }
        }
        Self::from_pixels(self.width, self.height, pixels)
    }

    fn opened(&self, radius: u32) -> Self {
        self.eroded(radius).dilated(radius)
    }

    fn closed(&self, radius: u32) -> Self {
        self.dilated(radius).eroded(radius)
    }

    fn without_small_components(&self, min_size: usize) -> Self {
        if min_size == 0 {
            return self.clone();
        }
        let mut pixels = self.pixels.clone();
        for component in self.selected_components() {
            if component.len() < min_size {
                for index in component {
                    pixels[index] = false;
                }
            }
        }
        Self::from_pixels(self.width, self.height, pixels)
    }

    fn with_filled_small_holes(&self, max_size: usize) -> Self {
        if max_size == 0 {
            return self.clone();
        }
        let mut visited = vec![false; self.pixels.len()];
        let mut pixels = self.pixels.clone();
        for y in 0..self.height {
            for x in 0..self.width {
                let start = mask_index(self.width, x, y);
                if self.pixels[start] || visited[start] {
                    continue;
                }
                let (component, touches_edge) = self.unselected_component(start, &mut visited);
                if !touches_edge && component.len() <= max_size {
                    for index in component {
                        pixels[index] = true;
                    }
                }
            }
        }
        Self::from_pixels(self.width, self.height, pixels)
    }

    fn protect_feature_regions(
        &mut self,
        feature_map: &SceneAssetFeatureMap,
        region_names: &[String],
    ) -> Result<(), SceneAssetEditError> {
        for region in region_names {
            let trimmed = region.trim();
            if trimmed.is_empty() {
                continue;
            }
            self.protect_rect(feature_map.pixel_region(trimmed, self.width, self.height)?);
        }
        Ok(())
    }

    fn protect_rect(&mut self, rect: SceneAssetPixelRect) {
        for y in rect.y..rect.bottom().min(self.height) {
            for x in rect.x..rect.right().min(self.width) {
                self.pixels[mask_index(self.width, x, y)] = false;
            }
        }
    }

    fn select_rect(&mut self, rect: SceneAssetPixelRect) {
        for y in rect.y..rect.bottom().min(self.height) {
            for x in rect.x..rect.right().min(self.width) {
                self.pixels[mask_index(self.width, x, y)] = true;
            }
        }
    }

    fn select_polygon(
        &mut self,
        polygon: &[SceneAssetNormalizedPoint],
    ) -> Result<(), SceneAssetEditError> {
        validate_polygon(polygon)?;
        for y in 0..self.height {
            for x in 0..self.width {
                let point = SceneAssetNormalizedPoint {
                    x: (x as f32 + 0.5) / self.width.max(1) as f32,
                    y: (y as f32 + 0.5) / self.height.max(1) as f32,
                };
                if point_in_polygon(point, polygon) {
                    self.pixels[mask_index(self.width, x, y)] = true;
                }
            }
        }
        Ok(())
    }

    fn selected_components(&self) -> Vec<Vec<usize>> {
        let mut visited = vec![false; self.pixels.len()];
        let mut components = Vec::new();
        for y in 0..self.height {
            for x in 0..self.width {
                let start = mask_index(self.width, x, y);
                if !self.pixels[start] || visited[start] {
                    continue;
                }
                let mut component = Vec::new();
                let mut queue = VecDeque::from([start]);
                visited[start] = true;
                while let Some(index) = queue.pop_front() {
                    component.push(index);
                    let (cx, cy) = mask_xy(self.width, index);
                    for (nx, ny) in mask_neighbors(self.width, self.height, cx, cy) {
                        let neighbor_index = mask_index(self.width, nx, ny);
                        if self.pixels[neighbor_index] && !visited[neighbor_index] {
                            visited[neighbor_index] = true;
                            queue.push_back(neighbor_index);
                        }
                    }
                }
                components.push(component);
            }
        }
        components
    }

    fn unselected_component(&self, start: usize, visited: &mut [bool]) -> (Vec<usize>, bool) {
        let mut component = Vec::new();
        let mut touches_edge = false;
        let mut queue = VecDeque::from([start]);
        visited[start] = true;
        while let Some(index) = queue.pop_front() {
            component.push(index);
            let (x, y) = mask_xy(self.width, index);
            touches_edge |= x == 0 || y == 0 || x + 1 == self.width || y + 1 == self.height;
            for (nx, ny) in mask_neighbors(self.width, self.height, x, y) {
                let neighbor_index = mask_index(self.width, nx, ny);
                if !self.pixels[neighbor_index] && !visited[neighbor_index] {
                    visited[neighbor_index] = true;
                    queue.push_back(neighbor_index);
                }
            }
        }
        (component, touches_edge)
    }
}

fn mask_index(width: u32, x: u32, y: u32) -> usize {
    y as usize * width as usize + x as usize
}

fn mask_xy(width: u32, index: usize) -> (u32, u32) {
    (
        (index % width as usize) as u32,
        (index / width as usize) as u32,
    )
}

fn mask_neighbors(width: u32, height: u32, x: u32, y: u32) -> Vec<(u32, u32)> {
    let mut neighbors = Vec::with_capacity(4);
    if x > 0 {
        neighbors.push((x - 1, y));
    }
    if y > 0 {
        neighbors.push((x, y - 1));
    }
    if x + 1 < width {
        neighbors.push((x + 1, y));
    }
    if y + 1 < height {
        neighbors.push((x, y + 1));
    }
    neighbors
}

fn validate_polygon(polygon: &[SceneAssetNormalizedPoint]) -> Result<(), SceneAssetEditError> {
    if polygon.len() < 3 {
        return Err(SceneAssetEditError::InvalidOperation(
            "restore polygon requires at least three points".to_string(),
        ));
    }
    for point in polygon {
        if !point.x.is_finite()
            || !point.y.is_finite()
            || point.x < 0.0
            || point.y < 0.0
            || point.x > 1.0
            || point.y > 1.0
        {
            return Err(SceneAssetEditError::InvalidOperation(
                "restore polygon points must be finite and inside 0..1".to_string(),
            ));
        }
    }
    Ok(())
}

fn point_in_polygon(
    point: SceneAssetNormalizedPoint,
    polygon: &[SceneAssetNormalizedPoint],
) -> bool {
    let mut inside = false;
    let mut previous = polygon[polygon.len() - 1];
    for &current in polygon {
        let crosses_y = (current.y > point.y) != (previous.y > point.y);
        if crosses_y {
            let slope = (previous.x - current.x) / (previous.y - current.y);
            let intersect_x = slope * (point.y - current.y) + current.x;
            if point.x < intersect_x {
                inside = !inside;
            }
        }
        previous = current;
    }
    inside
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

fn pixel_len(image: &RgbaImage) -> usize {
    image.width() as usize * image.height() as usize
}

fn pixel_index(image: &RgbaImage, x: u32, y: u32) -> usize {
    y as usize * image.width() as usize + x as usize
}

fn parse_rgba(color: &str) -> Result<Rgba<u8>, SceneAssetEditError> {
    let trimmed = color.trim();
    let hex = trimmed.strip_prefix('#').unwrap_or(trimmed);
    if hex.len() != 6 && hex.len() != 8 {
        return Err(SceneAssetEditError::InvalidColor(color.to_string()));
    }
    let r = parse_hex_byte(color, &hex[0..2])?;
    let g = parse_hex_byte(color, &hex[2..4])?;
    let b = parse_hex_byte(color, &hex[4..6])?;
    let a = if hex.len() == 8 {
        parse_hex_byte(color, &hex[6..8])?
    } else {
        255
    };
    Ok(Rgba([r, g, b, a]))
}

fn parse_hex_byte(color: &str, text: &str) -> Result<u8, SceneAssetEditError> {
    u8::from_str_radix(text, 16).map_err(|_| SceneAssetEditError::InvalidColor(color.to_string()))
}

fn erase_region(image: &mut RgbaImage, rect: SceneAssetPixelRect, soften: u32) {
    for y in rect.y..rect.bottom().min(image.height()) {
        for x in rect.x..rect.right().min(image.width()) {
            let pixel = image.get_pixel_mut(x, y);
            if soften == 0 {
                pixel[3] = 0;
                continue;
            }
            let edge = (x - rect.x)
                .min(rect.right().saturating_sub(x + 1))
                .min(y - rect.y)
                .min(rect.bottom().saturating_sub(y + 1));
            if edge >= soften {
                pixel[3] = 0;
            } else {
                let factor = edge as f32 / soften.max(1) as f32;
                pixel[3] = (pixel[3] as f32 * factor).round() as u8;
            }
        }
    }
}

fn fill_region(image: &mut RgbaImage, rect: SceneAssetPixelRect, color: Rgba<u8>) {
    for y in rect.y..rect.bottom().min(image.height()) {
        for x in rect.x..rect.right().min(image.width()) {
            blend_pixel(image.get_pixel_mut(x, y), color, 1.0);
        }
    }
}

fn draw_line_in_region(
    image: &mut RgbaImage,
    rect: SceneAssetPixelRect,
    from: SceneAssetNormalizedPoint,
    to: SceneAssetNormalizedPoint,
    color: Rgba<u8>,
    width: u32,
) {
    let start = point_in_rect(rect, from);
    let end = point_in_rect(rect, to);
    draw_line(image, start, end, color, width.max(1));
}

fn draw_line(image: &mut RgbaImage, from: (i32, i32), to: (i32, i32), color: Rgba<u8>, width: u32) {
    let dx = (to.0 - from.0).abs();
    let dy = -(to.1 - from.1).abs();
    let sx = if from.0 < to.0 { 1 } else { -1 };
    let sy = if from.1 < to.1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut x = from.0;
    let mut y = from.1;
    loop {
        draw_disk(image, x, y, width as i32, color);
        if x == to.0 && y == to.1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

fn draw_normalized_line(
    image: &mut RgbaImage,
    from: SceneAssetNormalizedPoint,
    to: SceneAssetNormalizedPoint,
    color: Rgba<u8>,
    width: u32,
) -> Result<(), SceneAssetEditError> {
    let from = normalized_point_to_pixel(from, image.width(), image.height())?;
    let to = normalized_point_to_pixel(to, image.width(), image.height())?;
    draw_line(
        image,
        (from.0 as i32, from.1 as i32),
        (to.0 as i32, to.1 as i32),
        color,
        width,
    );
    Ok(())
}

fn draw_normalized_path(
    image: &mut RgbaImage,
    path: &[SceneAssetNormalizedPoint],
    color: Rgba<u8>,
    width: u32,
    closed: bool,
) -> Result<(), SceneAssetEditError> {
    if path.len() < 2 {
        return Err(SceneAssetEditError::InvalidOperation(
            "stroke path requires at least two points".to_string(),
        ));
    }
    for pair in path.windows(2) {
        draw_normalized_line(image, pair[0], pair[1], color, width)?;
    }
    if closed && path.len() > 2 {
        draw_normalized_line(image, path[path.len() - 1], path[0], color, width)?;
    }
    Ok(())
}

fn draw_rect_outline(
    image: &mut RgbaImage,
    rect: SceneAssetPixelRect,
    color: Rgba<u8>,
    width: u32,
) {
    if rect.w == 0 || rect.h == 0 {
        return;
    }
    let left = rect.x as i32;
    let right = rect.right().saturating_sub(1) as i32;
    let top = rect.y as i32;
    let bottom = rect.bottom().saturating_sub(1) as i32;
    draw_line(image, (left, top), (right, top), color, width);
    draw_line(image, (right, top), (right, bottom), color, width);
    draw_line(image, (right, bottom), (left, bottom), color, width);
    draw_line(image, (left, bottom), (left, top), color, width);
}

fn draw_disk(image: &mut RgbaImage, cx: i32, cy: i32, radius: i32, color: Rgba<u8>) {
    let radius = radius.max(1);
    let r2 = radius * radius;
    for y in (cy - radius)..=(cy + radius) {
        for x in (cx - radius)..=(cx + radius) {
            if (x - cx) * (x - cx) + (y - cy) * (y - cy) <= r2 {
                if let Some(pixel) = pixel_mut_checked(image, x, y) {
                    blend_pixel(pixel, color, 1.0);
                }
            }
        }
    }
}

fn draw_ellipse(
    image: &mut RgbaImage,
    rect: SceneAssetPixelRect,
    stroke: Option<Rgba<u8>>,
    fill: Option<Rgba<u8>>,
    stroke_width: u32,
) {
    let cx = rect.x as f32 + rect.w as f32 / 2.0;
    let cy = rect.y as f32 + rect.h as f32 / 2.0;
    let rx = (rect.w as f32 / 2.0).max(1.0);
    let ry = (rect.h as f32 / 2.0).max(1.0);
    let stroke_band = stroke_width.max(1) as f32 / rx.min(ry).max(1.0);
    for y in rect.y..rect.bottom().min(image.height()) {
        for x in rect.x..rect.right().min(image.width()) {
            let nx = (x as f32 + 0.5 - cx) / rx;
            let ny = (y as f32 + 0.5 - cy) / ry;
            let distance = nx * nx + ny * ny;
            if distance <= 1.0 {
                if let Some(fill_color) = fill {
                    blend_pixel(image.get_pixel_mut(x, y), fill_color, 1.0);
                }
                if let Some(stroke_color) = stroke {
                    if distance >= (1.0 - stroke_band).max(0.0) {
                        blend_pixel(image.get_pixel_mut(x, y), stroke_color, 1.0);
                    }
                }
            }
        }
    }
}

fn composite_scaled(
    image: &mut RgbaImage,
    rect: SceneAssetPixelRect,
    overlay: &RgbaImage,
    opacity: f32,
) {
    if rect.w == 0 || rect.h == 0 {
        return;
    }
    let resized = image::imageops::resize(overlay, rect.w, rect.h, FilterType::Lanczos3);
    for y in 0..resized.height() {
        for x in 0..resized.width() {
            let target_x = rect.x + x;
            let target_y = rect.y + y;
            if target_x < image.width() && target_y < image.height() {
                blend_pixel(
                    image.get_pixel_mut(target_x, target_y),
                    *resized.get_pixel(x, y),
                    opacity,
                );
            }
        }
    }
}

fn translate_region(image: &mut RgbaImage, rect: SceneAssetPixelRect, dx: i32, dy: i32) {
    let copy = crop_region(image, rect);
    erase_region(image, rect, 0);
    paste_region(image, &copy, rect.x as i32 + dx, rect.y as i32 + dy, 1.0);
}

fn scale_region(
    image: &mut RgbaImage,
    rect: SceneAssetPixelRect,
    sx: f32,
    sy: f32,
) -> Result<(), SceneAssetEditError> {
    if sx <= 0.0 || sy <= 0.0 || !sx.is_finite() || !sy.is_finite() {
        return Err(SceneAssetEditError::InvalidOperation(
            "scale_region sx and sy must be finite positive values".to_string(),
        ));
    }
    let copy = crop_region(image, rect);
    let new_w = ((rect.w as f32 * sx).round() as u32).max(1);
    let new_h = ((rect.h as f32 * sy).round() as u32).max(1);
    let resized = image::imageops::resize(&copy, new_w, new_h, FilterType::Lanczos3);
    erase_region(image, rect, 0);
    let x = rect.x as i32 + (rect.w as i32 - new_w as i32) / 2;
    let y = rect.y as i32 + (rect.h as i32 - new_h as i32) / 2;
    paste_region(image, &resized, x, y, 1.0);
    Ok(())
}

fn multiply_alpha(image: &mut RgbaImage, rect: SceneAssetPixelRect, alpha: f32) {
    let alpha = alpha.clamp(0.0, 1.0);
    for y in rect.y..rect.bottom().min(image.height()) {
        for x in rect.x..rect.right().min(image.width()) {
            let pixel = image.get_pixel_mut(x, y);
            pixel[3] = (pixel[3] as f32 * alpha).round().clamp(0.0, 255.0) as u8;
        }
    }
}

fn tint_region(image: &mut RgbaImage, rect: SceneAssetPixelRect, color: Rgba<u8>, amount: f32) {
    let amount = amount.clamp(0.0, 1.0);
    for y in rect.y..rect.bottom().min(image.height()) {
        for x in rect.x..rect.right().min(image.width()) {
            let pixel = image.get_pixel_mut(x, y);
            for channel in 0..3 {
                pixel[channel] =
                    lerp(pixel[channel] as f32, color[channel] as f32, amount).round() as u8;
            }
        }
    }
}

fn crop_region(image: &RgbaImage, rect: SceneAssetPixelRect) -> RgbaImage {
    let mut output = ImageBuffer::from_pixel(rect.w, rect.h, Rgba([0, 0, 0, 0]));
    for y in 0..rect.h {
        for x in 0..rect.w {
            let source_x = rect.x + x;
            let source_y = rect.y + y;
            if source_x < image.width() && source_y < image.height() {
                output.put_pixel(x, y, *image.get_pixel(source_x, source_y));
            }
        }
    }
    output
}

fn paste_region(image: &mut RgbaImage, patch: &RgbaImage, x: i32, y: i32, opacity: f32) {
    for patch_y in 0..patch.height() {
        for patch_x in 0..patch.width() {
            let target_x = x + patch_x as i32;
            let target_y = y + patch_y as i32;
            if let Some(pixel) = pixel_mut_checked(image, target_x, target_y) {
                blend_pixel(pixel, *patch.get_pixel(patch_x, patch_y), opacity);
            }
        }
    }
}

fn paste_layer(
    image: &mut RgbaImage,
    patch: &RgbaImage,
    x: i32,
    y: i32,
    opacity: f32,
    blend: SceneAssetBlendMode,
) {
    for patch_y in 0..patch.height() {
        for patch_x in 0..patch.width() {
            let target_x = x + patch_x as i32;
            let target_y = y + patch_y as i32;
            if let Some(pixel) = pixel_mut_checked(image, target_x, target_y) {
                blend_pixel_mode(pixel, *patch.get_pixel(patch_x, patch_y), opacity, blend);
            }
        }
    }
}

fn blend_pixel(dest: &mut Rgba<u8>, src: Rgba<u8>, opacity: f32) {
    let src_alpha = (src[3] as f32 / 255.0) * opacity.clamp(0.0, 1.0);
    if src_alpha <= 0.0 {
        return;
    }
    let dest_alpha = dest[3] as f32 / 255.0;
    let out_alpha = src_alpha + dest_alpha * (1.0 - src_alpha);
    if out_alpha <= 0.0 {
        *dest = Rgba([0, 0, 0, 0]);
        return;
    }
    for channel in 0..3 {
        let src_c = src[channel] as f32 / 255.0;
        let dest_c = dest[channel] as f32 / 255.0;
        let out = (src_c * src_alpha + dest_c * dest_alpha * (1.0 - src_alpha)) / out_alpha;
        dest[channel] = (out * 255.0).round().clamp(0.0, 255.0) as u8;
    }
    dest[3] = (out_alpha * 255.0).round().clamp(0.0, 255.0) as u8;
}

fn blend_pixel_mode(dest: &mut Rgba<u8>, src: Rgba<u8>, opacity: f32, blend: SceneAssetBlendMode) {
    if blend == SceneAssetBlendMode::Normal {
        blend_pixel(dest, src, opacity);
        return;
    }
    let alpha = (src[3] as f32 / 255.0) * opacity.clamp(0.0, 1.0);
    if alpha <= 0.0 {
        return;
    }
    for channel in 0..3 {
        let s = src[channel] as f32 / 255.0;
        let d = dest[channel] as f32 / 255.0;
        let blended = match blend {
            SceneAssetBlendMode::Normal => s,
            SceneAssetBlendMode::Add => (d + s).clamp(0.0, 1.0),
            SceneAssetBlendMode::Multiply => d * s,
            SceneAssetBlendMode::Screen => 1.0 - (1.0 - d) * (1.0 - s),
        };
        let out = lerp(d, blended, alpha);
        dest[channel] = (out * 255.0).round().clamp(0.0, 255.0) as u8;
    }
    dest[3] = ((dest[3] as f32 / 255.0) + alpha * (1.0 - dest[3] as f32 / 255.0))
        .mul_add(255.0, 0.0)
        .round()
        .clamp(0.0, 255.0) as u8;
}

fn pixel_mut_checked(image: &mut RgbaImage, x: i32, y: i32) -> Option<&mut Rgba<u8>> {
    if x < 0 || y < 0 {
        return None;
    }
    let x = x as u32;
    let y = y as u32;
    if x < image.width() && y < image.height() {
        Some(image.get_pixel_mut(x, y))
    } else {
        None
    }
}

fn normalized_rect_arg(
    rect: Option<SceneAssetNormalizedRect>,
    label: &str,
) -> Result<SceneAssetNormalizedRect, SceneAssetEditError> {
    rect.ok_or_else(|| SceneAssetEditError::InvalidOperation(format!("{label} requires --rect")))
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

fn changed_pixel_count(a: &RgbaImage, b: &RgbaImage) -> usize {
    if a.dimensions() != b.dimensions() {
        return a.width() as usize * a.height() as usize;
    }
    a.pixels()
        .zip(b.pixels())
        .filter(|(a, b)| a.0 != b.0)
        .count()
}

fn color_channels(channel: SceneAssetColorChannel) -> &'static [usize] {
    match channel {
        SceneAssetColorChannel::Rgb => &[0, 1, 2],
        SceneAssetColorChannel::R => &[0],
        SceneAssetColorChannel::G => &[1],
        SceneAssetColorChannel::B => &[2],
        SceneAssetColorChannel::A => &[3],
    }
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

fn state_composite_options(
    manifest: &SceneAssetStateManifest,
    selected_states: &BTreeMap<String, String>,
) -> Result<SceneAssetCompositeOptions, SceneAssetEditError> {
    let mut layers = vec![SceneAssetCompositeLayer {
        path: manifest.base.clone(),
        blend: SceneAssetBlendMode::Normal,
        opacity: 1.0,
        x_offset: 0,
        y_offset: 0,
    }];
    for (part_name, part) in &manifest.parts {
        let state = selected_states
            .get(part_name)
            .map(String::as_str)
            .unwrap_or(&part.default);
        let path = part.states.get(state).ok_or_else(|| {
            SceneAssetEditError::InvalidOperation(format!(
                "unknown state `{state}` for part `{part_name}`"
            ))
        })?;
        layers.push(SceneAssetCompositeLayer {
            path: path.clone(),
            blend: SceneAssetBlendMode::Normal,
            opacity: 1.0,
            x_offset: 0,
            y_offset: 0,
        });
    }
    Ok(SceneAssetCompositeOptions {
        width: None,
        height: None,
        layers,
    })
}

fn render_composite_to_image(
    options: SceneAssetCompositeOptions,
    base_dir: Option<&Path>,
) -> Result<RgbaImage, SceneAssetEditError> {
    if options.layers.is_empty() {
        return Err(SceneAssetEditError::InvalidOperation(
            "composite requires at least one layer".to_string(),
        ));
    }
    let mut loaded = Vec::with_capacity(options.layers.len());
    for layer in &options.layers {
        let path = resolve_recipe_path(&layer.path, base_dir);
        loaded.push((layer, load_rgba_image(&path)?));
    }
    let width = options.width.unwrap_or_else(|| loaded[0].1.width());
    let height = options.height.unwrap_or_else(|| loaded[0].1.height());
    let mut output = ImageBuffer::from_pixel(width, height, Rgba([0, 0, 0, 0]));
    for (layer, image) in loaded {
        paste_layer(
            &mut output,
            &image,
            layer.x_offset,
            layer.y_offset,
            layer.opacity,
            layer.blend,
        );
    }
    Ok(output)
}

fn point_in_rect(rect: SceneAssetPixelRect, point: SceneAssetNormalizedPoint) -> (i32, i32) {
    let x = rect.x as f32 + point.x.clamp(0.0, 1.0) * rect.w.saturating_sub(1) as f32;
    let y = rect.y as f32 + point.y.clamp(0.0, 1.0) * rect.h.saturating_sub(1) as f32;
    (x.round() as i32, y.round() as i32)
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

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn rounded_f32(value: f32) -> f32 {
    (value * 10_000.0).round() / 10_000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use k9::assert_equal;
    use tempfile::tempdir;

    fn write_test_image(path: &Path) {
        let mut image = ImageBuffer::from_pixel(32, 32, Rgba([0u8, 0, 0, 0]));
        for y in 4..28 {
            for x in 8..24 {
                image.put_pixel(x, y, Rgba([220u8, 180, 170, 255]));
            }
        }
        image.save(path).unwrap();
    }

    fn write_subject_on_background(path: &Path) {
        let mut image = ImageBuffer::from_pixel(16, 16, Rgba([245u8, 245, 240, 255]));
        for y in 5..11 {
            for x in 6..10 {
                image.put_pixel(x, y, Rgba([180u8, 40, 70, 255]));
            }
        }
        image.save(path).unwrap();
    }

    fn write_two_matching_islands(path: &Path) {
        let mut image = ImageBuffer::from_pixel(16, 8, Rgba([240u8, 240, 240, 255]));
        for y in 2..6 {
            for x in 2..5 {
                image.put_pixel(x, y, Rgba([30u8, 80, 220, 255]));
            }
            for x in 11..14 {
                image.put_pixel(x, y, Rgba([30u8, 80, 220, 255]));
            }
        }
        image.save(path).unwrap();
    }

    fn test_feature_map() -> SceneAssetFeatureMap {
        let mut regions = BTreeMap::new();
        regions.insert(
            "mouth".to_string(),
            SceneAssetNormalizedRect {
                x: 0.375,
                y: 0.50,
                w: 0.25,
                h: 0.125,
            },
        );
        regions.insert(
            "torso".to_string(),
            SceneAssetNormalizedRect {
                x: 0.25,
                y: 0.25,
                w: 0.50,
                h: 0.50,
            },
        );
        SceneAssetFeatureMap {
            feature_map_version: 1,
            character: "kiki".to_string(),
            base: "kiki-neutral.png".to_string(),
            regions,
            anchors: BTreeMap::new(),
        }
    }

    #[test]
    fn asset_image_inspect_reports_bounds_and_checksum() {
        let dir = tempdir().unwrap();
        let image_path = dir.path().join("kiki.png");
        write_test_image(&image_path);

        let report = inspect_scene_asset_image(&image_path).unwrap();

        assert_equal!(report.width, 32);
        assert_equal!(report.height, 32);
        assert_equal!(
            report.content_bounds,
            Some(SceneAssetPixelRect {
                x: 8,
                y: 4,
                w: 16,
                h: 24
            })
        );
        assert_equal!(report.sha256.len(), 64);
    }

    #[test]
    fn point_report_maps_normalized_points_to_pixels_and_color() {
        let dir = tempdir().unwrap();
        let image_path = dir.path().join("sample.png");
        let mut image = ImageBuffer::from_pixel(4, 4, Rgba([0u8, 0, 0, 255]));
        image.put_pixel(3, 3, Rgba([10u8, 20, 30, 40]));
        image.save(&image_path).unwrap();

        let report =
            report_scene_asset_points(&image_path, &[SceneAssetNormalizedPoint { x: 1.0, y: 1.0 }])
                .unwrap();

        assert_equal!(report.samples.len(), 1);
        assert_equal!(report.samples[0].pixel_x, 3);
        assert_equal!(report.samples[0].pixel_y, 3);
        assert_equal!(report.samples[0].rgba, [10, 20, 30, 40]);
    }

    #[test]
    fn sample_report_summarizes_bounded_region() {
        let dir = tempdir().unwrap();
        let image_path = dir.path().join("sample-region.png");
        let mut image = ImageBuffer::from_pixel(4, 4, Rgba([0u8, 0, 0, 0]));
        for y in 0..4 {
            for x in 0..2 {
                image.put_pixel(x, y, Rgba([10u8, 20, 30, 255]));
            }
        }
        image.save(&image_path).unwrap();

        let report = sample_scene_asset_image(
            &image_path,
            SceneAssetSampleOptions {
                points: vec![SceneAssetNormalizedPoint { x: 0.0, y: 0.0 }],
                within_regions: Vec::new(),
                within_polygons: vec![vec![
                    SceneAssetNormalizedPoint { x: 0.0, y: 0.0 },
                    SceneAssetNormalizedPoint { x: 0.5, y: 0.0 },
                    SceneAssetNormalizedPoint { x: 0.5, y: 1.0 },
                    SceneAssetNormalizedPoint { x: 0.0, y: 1.0 },
                ]],
            },
            None,
        )
        .unwrap();

        let region = report.region.unwrap();
        assert_equal!(report.points[0].rgba, [10, 20, 30, 255]);
        assert_equal!(region.pixel_count, 8);
        assert_equal!(region.median_rgba, [10, 20, 30, 255]);
        assert_equal!(region.mean_rgba, [10.0, 20.0, 30.0, 255.0]);
        assert_equal!(region.alpha_coverage, 1.0);
    }

    #[test]
    fn pipeline_run_chains_preview_paint_and_sample_steps() {
        let dir = tempdir().unwrap();
        let input_root = dir.path().join("Input");
        let transformation_root = dir.path().join("Transformation");
        let output_root = dir.path().join("Output");
        std::fs::create_dir_all(&input_root).unwrap();
        std::fs::create_dir_all(&transformation_root).unwrap();
        std::fs::create_dir_all(&output_root).unwrap();

        let source = input_root.join("source.png");
        let image = ImageBuffer::from_pixel(4, 4, Rgba([0u8, 0, 0, 255]));
        image.save(&source).unwrap();

        let polygon = serde_json::json!(["0.5,0.0;1.0,0.0;1.0,1.0;0.5,1.0"]);
        let pipeline = SceneAssetPipeline {
            asset_pipeline_version: 1,
            name: "test-pipeline".to_string(),
            input: "source.png".to_string(),
            steps: vec![
                SceneAssetPipelineStep {
                    command: "mask-preview".to_string(),
                    output: Some("01-preview.png".to_string()),
                    args: BTreeMap::from([
                        (
                            "selection_mode".to_string(),
                            serde_json::json!("color-range"),
                        ),
                        ("tolerance".to_string(), serde_json::json!(0)),
                    ]),
                },
                SceneAssetPipelineStep {
                    command: "fill-region".to_string(),
                    output: Some("02-filled.png".to_string()),
                    args: BTreeMap::from([
                        ("color".to_string(), serde_json::json!("#ff0000ff")),
                        ("within_polygons".to_string(), polygon),
                    ]),
                },
                SceneAssetPipelineStep {
                    command: "sample".to_string(),
                    output: Some("03-sample.json".to_string()),
                    args: BTreeMap::from([("point".to_string(), serde_json::json!("1.0,0.0"))]),
                },
            ],
        };
        let pipeline_path = dir.path().join("pipeline.json");
        write_scene_asset_json(&pipeline_path, &pipeline, true, false).unwrap();

        let report = run_scene_asset_pipeline(
            &pipeline_path,
            &SceneAssetPipelineRoots {
                input_root,
                transformation_root: transformation_root.clone(),
                output_root,
            },
            SceneAssetPipelineRunOptions {
                force: false,
                dry_run: false,
                pretty: true,
            },
        )
        .unwrap();

        assert_equal!(report.steps.len(), 3);
        assert_equal!(report.steps[0].advanced_source, false);
        assert_equal!(report.steps[1].advanced_source, true);
        assert!(transformation_root.join("01-preview.png").is_file());
        assert!(transformation_root.join("01-preview.report.json").is_file());
        assert!(transformation_root.join("02-filled.png").is_file());
        assert!(transformation_root.join("02-filled.report.json").is_file());
        assert!(transformation_root.join("03-sample.json").is_file());

        let sample =
            load_json::<SceneAssetSampleReport>(&transformation_root.join("03-sample.json"))
                .unwrap();
        assert_equal!(sample.points[0].rgba, [255, 0, 0, 255]);
    }

    #[test]
    fn compare_report_counts_changed_pixels_and_bounds() {
        let dir = tempdir().unwrap();
        let before = dir.path().join("before.png");
        let after = dir.path().join("after.png");
        let image = ImageBuffer::from_pixel(4, 4, Rgba([0u8, 0, 0, 255]));
        image.save(&before).unwrap();
        let mut changed = image.clone();
        changed.put_pixel(1, 1, Rgba([255u8, 0, 0, 255]));
        changed.put_pixel(2, 3, Rgba([0u8, 0, 0, 0]));
        changed.save(&after).unwrap();

        let report = compare_scene_asset_images(&before, &after).unwrap();

        assert!(report.same_dimensions);
        assert_equal!(report.changed_pixels, 2);
        assert_equal!(report.alpha_changed_pixels, 1);
        assert_equal!(report.changed_pixel_ratio, 0.125);
        assert_equal!(
            report.changed_bounds,
            Some(SceneAssetPixelRect {
                x: 1,
                y: 1,
                w: 2,
                h: 3
            })
        );
    }

    #[test]
    fn diff_preview_highlights_color_and_alpha_changes() {
        let dir = tempdir().unwrap();
        let before = dir.path().join("before.png");
        let after = dir.path().join("after.png");
        let raw = dir.path().join("raw.png");
        let alpha = dir.path().join("alpha.png");
        let before_image = ImageBuffer::from_pixel(2, 2, Rgba([0u8, 0, 0, 255]));
        let mut after_image = before_image.clone();
        after_image.put_pixel(1, 0, Rgba([255, 0, 0, 255]));
        after_image.put_pixel(0, 1, Rgba([0, 0, 0, 0]));
        before_image.save(&before).unwrap();
        after_image.save(&after).unwrap();

        write_scene_asset_review_preview(
            &before,
            &after,
            &raw,
            SceneAssetReviewPreviewMode::RawDiff,
            false,
        )
        .unwrap();
        write_scene_asset_review_preview(
            &before,
            &after,
            &alpha,
            SceneAssetReviewPreviewMode::AlphaDiff,
            false,
        )
        .unwrap();

        let raw_image = load_rgba_image(&raw).unwrap();
        assert_equal!(raw_image.get_pixel(0, 0)[3], 0);
        assert_equal!(raw_image.get_pixel(1, 0).0, [255, 48, 96, 255]);
        let alpha_image = load_rgba_image(&alpha).unwrap();
        assert_equal!(alpha_image.get_pixel(0, 1).0, [64, 220, 255, 255]);
        assert_equal!(alpha_image.get_pixel(1, 0).0, [255, 48, 96, 180]);
    }

    #[test]
    fn review_contact_sheet_preserves_dimensions() {
        let dir = tempdir().unwrap();
        let before = dir.path().join("before.png");
        let after = dir.path().join("after.png");
        let output = dir.path().join("contact.png");
        ImageBuffer::from_pixel(2, 3, Rgba([0u8, 0, 0, 255]))
            .save(&before)
            .unwrap();
        ImageBuffer::from_pixel(2, 3, Rgba([255u8, 0, 0, 255]))
            .save(&after)
            .unwrap();

        let report = write_scene_asset_review_preview(
            &before,
            &after,
            &output,
            SceneAssetReviewPreviewMode::ContactSheet,
            false,
        )
        .unwrap();

        assert_equal!(report.report.width, 6);
        assert_equal!(report.report.height, 3);
    }

    #[test]
    fn operation_run_executes_single_step_and_supports_dry_run() {
        let dir = tempdir().unwrap();
        let input_root = dir.path().join("Input");
        let transformation_root = dir.path().join("Transformation");
        let output_root = dir.path().join("Output");
        std::fs::create_dir_all(&input_root).unwrap();
        std::fs::create_dir_all(&transformation_root).unwrap();
        std::fs::create_dir_all(&output_root).unwrap();

        let source = input_root.join("source.png");
        let image = ImageBuffer::from_pixel(4, 4, Rgba([0u8, 0, 0, 255]));
        image.save(&source).unwrap();

        let operation = SceneAssetOperation {
            asset_operation_version: 1,
            id: "fill-right-half".to_string(),
            intent: Some("paint the right half red".to_string()),
            source: "source.png".to_string(),
            output: "01-filled.png".to_string(),
            command: "fill-region".to_string(),
            args: BTreeMap::from([
                ("color".to_string(), serde_json::json!("#ff0000ff")),
                (
                    "within_polygons".to_string(),
                    serde_json::json!(["0.5,0.0;1.0,0.0;1.0,1.0;0.5,1.0"]),
                ),
            ]),
            expectations: SceneAssetOperationExpectations {
                max_changed_pixel_ratio: Some(0.6),
                ..Default::default()
            },
        };
        let operation_path = dir.path().join("operation.json");
        write_scene_asset_json(&operation_path, &operation, true, false).unwrap();

        let roots = SceneAssetPipelineRoots {
            input_root: input_root.clone(),
            transformation_root: transformation_root.clone(),
            output_root: output_root.clone(),
        };
        let report = run_scene_asset_operation(
            &operation_path,
            &roots,
            SceneAssetOperationRunOptions {
                force: false,
                dry_run: false,
                preview: false,
                pretty: true,
            },
        )
        .unwrap();

        assert_equal!(report.status, "ok");
        assert_equal!(report.id, "fill-right-half");
        assert_equal!(report.compare.as_ref().unwrap().changed_pixels, 8);
        assert!(transformation_root.join("01-filled.png").is_file());
        assert!(transformation_root.join("01-filled.report.json").is_file());

        let dry_operation = SceneAssetOperation {
            output: "02-dry-run.png".to_string(),
            ..operation
        };
        let dry_operation_path = dir.path().join("dry-operation.json");
        write_scene_asset_json(&dry_operation_path, &dry_operation, true, false).unwrap();
        let dry_report = run_scene_asset_operation(
            &dry_operation_path,
            &roots,
            SceneAssetOperationRunOptions {
                force: false,
                dry_run: true,
                preview: false,
                pretty: true,
            },
        )
        .unwrap();

        assert_equal!(dry_report.status, "validated");
        assert!(!transformation_root.join("02-dry-run.png").exists());
    }

    #[test]
    fn validate_operation_reports_success_without_writing_output() {
        let dir = tempdir().unwrap();
        let input_root = dir.path().join("Input");
        let transformation_root = dir.path().join("Transformation");
        let output_root = dir.path().join("Output");
        std::fs::create_dir_all(&input_root).unwrap();
        std::fs::create_dir_all(&transformation_root).unwrap();
        std::fs::create_dir_all(&output_root).unwrap();

        let source = input_root.join("source.png");
        let image = ImageBuffer::from_pixel(4, 4, Rgba([0u8, 0, 0, 255]));
        image.save(&source).unwrap();
        let operation = SceneAssetOperation {
            asset_operation_version: 1,
            id: "validate-fill".to_string(),
            intent: None,
            source: "source.png".to_string(),
            output: "01-filled.png".to_string(),
            command: "fill-region".to_string(),
            args: BTreeMap::from([
                ("color".to_string(), serde_json::json!("#ff0000ff")),
                ("whole_image".to_string(), serde_json::json!(true)),
            ]),
            expectations: SceneAssetOperationExpectations::default(),
        };
        let operation_path = dir.path().join("operation.json");
        write_scene_asset_json(&operation_path, &operation, true, false).unwrap();

        let report = validate_scene_asset_operation(
            &operation_path,
            &SceneAssetPipelineRoots {
                input_root,
                transformation_root: transformation_root.clone(),
                output_root,
            },
            false,
        )
        .unwrap();

        assert_equal!(report.operation, "validate_operation");
        assert_equal!(report.id, "validate-fill");
        assert_equal!(report.status, "ok");
        assert_equal!(report.command, "fill-region");
        assert!(!transformation_root.join("01-filled.png").exists());
    }

    #[test]
    fn validate_operation_rejects_unknown_protected_region() {
        let dir = tempdir().unwrap();
        let input_root = dir.path().join("Input");
        let transformation_root = dir.path().join("Transformation");
        let output_root = dir.path().join("Output");
        std::fs::create_dir_all(&input_root).unwrap();
        std::fs::create_dir_all(&transformation_root).unwrap();
        std::fs::create_dir_all(&output_root).unwrap();

        let source = input_root.join("source.png");
        let image = ImageBuffer::from_pixel(4, 4, Rgba([0u8, 0, 0, 255]));
        image.save(&source).unwrap();
        let feature_map = SceneAssetFeatureMap {
            feature_map_version: 1,
            character: "kiki".to_string(),
            base: "source.png".to_string(),
            regions: BTreeMap::from([(
                "face".to_string(),
                SceneAssetNormalizedRect {
                    x: 0.0,
                    y: 0.0,
                    w: 0.5,
                    h: 0.5,
                },
            )]),
            anchors: BTreeMap::new(),
        };
        write_scene_asset_json(&input_root.join("map.json"), &feature_map, true, false).unwrap();
        let operation = SceneAssetOperation {
            asset_operation_version: 1,
            id: "validate-region".to_string(),
            intent: None,
            source: "source.png".to_string(),
            output: "01-filled.png".to_string(),
            command: "fill-region".to_string(),
            args: BTreeMap::from([
                ("protect".to_string(), serde_json::json!("map.json")),
                ("protect_regions".to_string(), serde_json::json!(["eyes"])),
                ("color".to_string(), serde_json::json!("#ff0000ff")),
                ("whole_image".to_string(), serde_json::json!(true)),
            ]),
            expectations: SceneAssetOperationExpectations::default(),
        };
        let operation_path = dir.path().join("operation.json");
        write_scene_asset_json(&operation_path, &operation, true, false).unwrap();

        let err = validate_scene_asset_operation(
            &operation_path,
            &SceneAssetPipelineRoots {
                input_root,
                transformation_root,
                output_root,
            },
            false,
        )
        .unwrap_err();

        assert!(matches!(err, SceneAssetEditError::UnknownRegion(region) if region == "eyes"));
    }

    #[test]
    fn protected_region_assertion_fails_when_region_changes() {
        let dir = tempdir().unwrap();
        let input_root = dir.path().join("Input");
        let transformation_root = dir.path().join("Transformation");
        let output_root = dir.path().join("Output");
        std::fs::create_dir_all(&input_root).unwrap();
        std::fs::create_dir_all(&transformation_root).unwrap();
        std::fs::create_dir_all(&output_root).unwrap();

        let source = input_root.join("source.png");
        ImageBuffer::from_pixel(4, 4, Rgba([0u8, 0, 0, 255]))
            .save(&source)
            .unwrap();
        let feature_map = SceneAssetFeatureMap {
            feature_map_version: 1,
            character: "kiki".to_string(),
            base: "source.png".to_string(),
            regions: BTreeMap::from([(
                "face".to_string(),
                SceneAssetNormalizedRect {
                    x: 0.0,
                    y: 0.0,
                    w: 0.5,
                    h: 1.0,
                },
            )]),
            anchors: BTreeMap::new(),
        };
        write_scene_asset_json(&input_root.join("map.json"), &feature_map, true, false).unwrap();
        let operation = SceneAssetOperation {
            asset_operation_version: 1,
            id: "fill-protected".to_string(),
            intent: None,
            source: "source.png".to_string(),
            output: "filled.png".to_string(),
            command: "fill-region".to_string(),
            args: BTreeMap::from([
                ("protect".to_string(), serde_json::json!("map.json")),
                ("color".to_string(), serde_json::json!("#ff0000ff")),
                ("whole_image".to_string(), serde_json::json!(true)),
            ]),
            expectations: SceneAssetOperationExpectations {
                must_preserve_regions: vec!["face".to_string()],
                max_changed_pixels_in_protected_regions: Some(0),
                ..Default::default()
            },
        };
        let operation_path = dir.path().join("operation.json");
        write_scene_asset_json(&operation_path, &operation, true, false).unwrap();

        let report = run_scene_asset_operation(
            &operation_path,
            &SceneAssetPipelineRoots {
                input_root,
                transformation_root,
                output_root,
            },
            SceneAssetOperationRunOptions {
                force: false,
                dry_run: false,
                preview: false,
                pretty: true,
            },
        )
        .unwrap();

        assert_equal!(report.status, "expectation_failed");
        let protected = report.protected_region_report.unwrap();
        assert_equal!(protected.changed_pixels, 8);
        assert_equal!(protected.changed_regions[0].region, "face");
        assert!(report
            .expectation_failures
            .iter()
            .any(|failure| failure.contains("protected regions changed")));
    }

    #[test]
    fn protected_region_assertion_passes_when_region_is_restored() {
        let dir = tempdir().unwrap();
        let input_root = dir.path().join("Input");
        let transformation_root = dir.path().join("Transformation");
        let output_root = dir.path().join("Output");
        std::fs::create_dir_all(&input_root).unwrap();
        std::fs::create_dir_all(&transformation_root).unwrap();
        std::fs::create_dir_all(&output_root).unwrap();

        let source = input_root.join("source.png");
        ImageBuffer::from_pixel(4, 4, Rgba([0u8, 0, 0, 255]))
            .save(&source)
            .unwrap();
        let feature_map = SceneAssetFeatureMap {
            feature_map_version: 1,
            character: "kiki".to_string(),
            base: "source.png".to_string(),
            regions: BTreeMap::from([(
                "face".to_string(),
                SceneAssetNormalizedRect {
                    x: 0.0,
                    y: 0.0,
                    w: 0.5,
                    h: 1.0,
                },
            )]),
            anchors: BTreeMap::new(),
        };
        write_scene_asset_json(&input_root.join("map.json"), &feature_map, true, false).unwrap();
        let operation = SceneAssetOperation {
            asset_operation_version: 1,
            id: "fill-around-protected".to_string(),
            intent: None,
            source: "source.png".to_string(),
            output: "filled.png".to_string(),
            command: "fill-region".to_string(),
            args: BTreeMap::from([
                ("protect".to_string(), serde_json::json!("map.json")),
                ("protect_regions".to_string(), serde_json::json!(["face"])),
                ("color".to_string(), serde_json::json!("#ff0000ff")),
                ("whole_image".to_string(), serde_json::json!(true)),
            ]),
            expectations: SceneAssetOperationExpectations {
                must_preserve_regions: vec!["face".to_string()],
                max_changed_pixels_in_protected_regions: Some(0),
                ..Default::default()
            },
        };
        let operation_path = dir.path().join("operation.json");
        write_scene_asset_json(&operation_path, &operation, true, false).unwrap();

        let report = run_scene_asset_operation(
            &operation_path,
            &SceneAssetPipelineRoots {
                input_root,
                transformation_root,
                output_root,
            },
            SceneAssetOperationRunOptions {
                force: false,
                dry_run: false,
                preview: false,
                pretty: true,
            },
        )
        .unwrap();

        assert_equal!(report.status, "ok");
        assert_equal!(report.protected_region_report.unwrap().changed_pixels, 0);
        assert!(report.expectation_failures.is_empty());
    }

    #[test]
    fn operation_run_preview_writes_review_artifacts_without_accepting_output() {
        let dir = tempdir().unwrap();
        let input_root = dir.path().join("Input");
        let transformation_root = dir.path().join("Transformation");
        let output_root = dir.path().join("Output");
        std::fs::create_dir_all(&input_root).unwrap();
        std::fs::create_dir_all(&transformation_root).unwrap();
        std::fs::create_dir_all(&output_root).unwrap();

        let source = input_root.join("source.png");
        let image = ImageBuffer::from_pixel(4, 4, Rgba([0u8, 0, 0, 255]));
        image.save(&source).unwrap();
        let operation = SceneAssetOperation {
            asset_operation_version: 1,
            id: "preview fill".to_string(),
            intent: None,
            source: "source.png".to_string(),
            output: "Output/final.png".to_string(),
            command: "fill-region".to_string(),
            args: BTreeMap::from([
                ("color".to_string(), serde_json::json!("#ff0000ff")),
                (
                    "within_polygons".to_string(),
                    serde_json::json!(["0.0,0.0;0.5,0.0;0.5,1.0;0.0,1.0"]),
                ),
            ]),
            expectations: SceneAssetOperationExpectations::default(),
        };
        let operation_path = dir.path().join("preview-operation.json");
        write_scene_asset_json(&operation_path, &operation, true, false).unwrap();

        let report = run_scene_asset_operation(
            &operation_path,
            &SceneAssetPipelineRoots {
                input_root,
                transformation_root: transformation_root.clone(),
                output_root: output_root.clone(),
            },
            SceneAssetOperationRunOptions {
                force: false,
                dry_run: false,
                preview: true,
                pretty: true,
            },
        )
        .unwrap();

        assert_equal!(report.status, "ok");
        assert!(report.preview);
        assert_equal!(
            report.requested_output_path,
            Some(output_root.join("final.png").display().to_string())
        );
        assert!(transformation_root
            .join("preview-fill.preview.png")
            .is_file());
        assert!(transformation_root.join("preview-fill.diff.png").is_file());
        let review_paths = report.review_preview_paths.unwrap();
        assert!(PathBuf::from(review_paths.raw_diff.unwrap()).is_file());
        assert!(PathBuf::from(review_paths.alpha_diff.unwrap()).is_file());
        assert!(PathBuf::from(review_paths.checkerboard.unwrap()).is_file());
        assert!(PathBuf::from(review_paths.dark.unwrap()).is_file());
        assert!(PathBuf::from(review_paths.contact_sheet.unwrap()).is_file());
        assert!(!output_root.join("final.png").exists());
    }

    #[test]
    fn operation_error_report_uses_stable_codes_and_hints() {
        let report =
            scene_asset_operation_error_report(&SceneAssetEditError::UnknownRegion("hair".into()));

        assert_equal!(report.status, "error");
        assert_equal!(report.code, "unknown_region");
        assert!(report.hint.unwrap().contains("map-template"));
    }

    #[test]
    fn accept_output_writes_report_and_refuses_overwrite_without_force() {
        let dir = tempdir().unwrap();
        let input_root = dir.path().join("Input");
        let transformation_root = dir.path().join("Transformation");
        let output_root = dir.path().join("Output");
        std::fs::create_dir_all(&input_root).unwrap();
        std::fs::create_dir_all(&transformation_root).unwrap();
        std::fs::create_dir_all(&output_root).unwrap();

        let reviewed = transformation_root.join("reviewed.png");
        let image = ImageBuffer::from_pixel(2, 2, Rgba([7u8, 8, 9, 255]));
        image.save(&reviewed).unwrap();
        let roots = SceneAssetPipelineRoots {
            input_root,
            transformation_root: transformation_root.clone(),
            output_root: output_root.clone(),
        };

        let report = accept_scene_asset_output(
            Path::new("reviewed.png"),
            Path::new("accepted.png"),
            &roots,
            false,
        )
        .unwrap();

        let accepted = output_root.join("accepted.png");
        assert_equal!(report.operation, "accept_output");
        assert_equal!(report.status, "ok");
        assert_equal!(report.source_path, reviewed.display().to_string());
        assert_equal!(report.output_path, accepted.display().to_string());
        assert!(accepted.is_file());
        assert_equal!(report.image.width, 2);
        assert_equal!(report.image.height, 2);

        let overwrite = accept_scene_asset_output(
            Path::new("reviewed.png"),
            Path::new("accepted.png"),
            &roots,
            false,
        )
        .unwrap_err();
        assert!(matches!(overwrite, SceneAssetEditError::OutputExists(_)));

        accept_scene_asset_output(
            Path::new("Transformation/reviewed.png"),
            Path::new("Output/accepted.png"),
            &roots,
            true,
        )
        .unwrap();
    }

    #[test]
    fn session_run_chains_operation_files_and_transformation_sources() {
        let dir = tempdir().unwrap();
        let input_root = dir.path().join("Input");
        let transformation_root = dir.path().join("Transformation");
        let output_root = dir.path().join("Output");
        std::fs::create_dir_all(&input_root).unwrap();
        std::fs::create_dir_all(&transformation_root).unwrap();
        std::fs::create_dir_all(&output_root).unwrap();

        let source = input_root.join("source.png");
        let image = ImageBuffer::from_pixel(4, 4, Rgba([0u8, 0, 0, 255]));
        image.save(&source).unwrap();
        let fill_operation = SceneAssetOperation {
            asset_operation_version: 1,
            id: "fill".to_string(),
            intent: None,
            source: "source.png".to_string(),
            output: "01-filled.png".to_string(),
            command: "fill-region".to_string(),
            args: BTreeMap::from([
                ("color".to_string(), serde_json::json!("#ff0000ff")),
                ("whole_image".to_string(), serde_json::json!(true)),
            ]),
            expectations: SceneAssetOperationExpectations::default(),
        };
        let alpha_operation = SceneAssetOperation {
            asset_operation_version: 1,
            id: "alpha".to_string(),
            intent: None,
            source: "Transformation/01-filled.png".to_string(),
            output: "02-alpha.png".to_string(),
            command: "alpha-paint".to_string(),
            args: BTreeMap::from([
                ("alpha".to_string(), serde_json::json!(128)),
                ("whole_image".to_string(), serde_json::json!(true)),
            ]),
            expectations: SceneAssetOperationExpectations::default(),
        };
        write_scene_asset_json(&dir.path().join("fill.json"), &fill_operation, true, false)
            .unwrap();
        write_scene_asset_json(
            &dir.path().join("alpha.json"),
            &alpha_operation,
            true,
            false,
        )
        .unwrap();
        let session = SceneAssetEditSession {
            asset_session_version: 1,
            name: "chain".to_string(),
            current_source: Some("source.png".to_string()),
            accepted_outputs: Vec::new(),
            operations: vec!["fill.json".to_string(), "alpha.json".to_string()],
        };
        let session_path = dir.path().join("session.json");
        write_scene_asset_json(&session_path, &session, true, false).unwrap();

        let report = run_scene_asset_edit_session(
            &session_path,
            &SceneAssetPipelineRoots {
                input_root,
                transformation_root: transformation_root.clone(),
                output_root,
            },
            SceneAssetOperationRunOptions {
                force: false,
                dry_run: false,
                preview: false,
                pretty: true,
            },
        )
        .unwrap();

        assert_equal!(report.name, "chain");
        assert_equal!(report.operations.len(), 2);
        assert_equal!(
            report.final_output_path,
            Some(
                transformation_root
                    .join("02-alpha.png")
                    .display()
                    .to_string()
            )
        );
        assert!(transformation_root.join("02-alpha.png").is_file());
    }

    #[test]
    fn grid_preview_draws_reference_lines() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.png");
        let output = dir.path().join("grid.png");
        let image = ImageBuffer::from_pixel(5, 5, Rgba([0u8, 0, 0, 255]));
        image.save(&source).unwrap();

        preview_scene_asset_grid(
            &source,
            &output,
            SceneAssetGridPreviewOptions { step: 0.5 },
            false,
        )
        .unwrap();

        let preview = load_rgba_image(&output).unwrap();
        assert_equal!(*preview.get_pixel(0, 0), Rgba([255u8, 220, 64, 255]));
        assert_equal!(*preview.get_pixel(2, 2), Rgba([255u8, 220, 64, 255]));
        assert_equal!(*preview.get_pixel(4, 2), Rgba([255u8, 64, 64, 220]));
    }

    #[test]
    fn fill_region_paints_only_bounded_polygon() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.png");
        let output = dir.path().join("fill.png");
        let image = ImageBuffer::from_pixel(4, 4, Rgba([0u8, 0, 0, 255]));
        image.save(&source).unwrap();

        let report = fill_scene_asset_region(
            &source,
            &output,
            SceneAssetFillOptions {
                color: [200, 100, 50, 255],
                whole_image: false,
                within_regions: Vec::new(),
                within_polygons: vec![vec![
                    SceneAssetNormalizedPoint { x: 0.0, y: 0.0 },
                    SceneAssetNormalizedPoint { x: 0.5, y: 0.0 },
                    SceneAssetNormalizedPoint { x: 0.5, y: 1.0 },
                    SceneAssetNormalizedPoint { x: 0.0, y: 1.0 },
                ]],
                protect_regions: Vec::new(),
            },
            None,
            false,
        )
        .unwrap();

        let edited = load_rgba_image(&output).unwrap();
        assert_equal!(report.changed_pixels, 8);
        assert_equal!(*edited.get_pixel(0, 0), Rgba([200u8, 100, 50, 255]));
        assert_equal!(*edited.get_pixel(3, 0), Rgba([0u8, 0, 0, 255]));
    }

    #[test]
    fn sample_fill_uses_median_sample_color() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.png");
        let output = dir.path().join("sample-fill.png");
        let mut image = ImageBuffer::from_pixel(4, 4, Rgba([0u8, 0, 0, 255]));
        image.put_pixel(0, 0, Rgba([80u8, 90, 100, 255]));
        image.save(&source).unwrap();

        sample_fill_scene_asset_region(
            &source,
            &output,
            SceneAssetSampleFillOptions {
                sample_point: SceneAssetNormalizedPoint { x: 0.0, y: 0.0 },
                sample_radius: 0,
                within_regions: Vec::new(),
                within_polygons: vec![vec![
                    SceneAssetNormalizedPoint { x: 0.5, y: 0.0 },
                    SceneAssetNormalizedPoint { x: 1.0, y: 0.0 },
                    SceneAssetNormalizedPoint { x: 1.0, y: 1.0 },
                    SceneAssetNormalizedPoint { x: 0.5, y: 1.0 },
                ]],
                protect_regions: Vec::new(),
            },
            None,
            false,
        )
        .unwrap();

        let edited = load_rgba_image(&output).unwrap();
        assert_equal!(*edited.get_pixel(3, 0), Rgba([80u8, 90, 100, 255]));
        assert_equal!(*edited.get_pixel(0, 0), Rgba([80u8, 90, 100, 255]));
        assert_equal!(*edited.get_pixel(1, 0), Rgba([0u8, 0, 0, 255]));
    }

    #[test]
    fn alpha_paint_changes_alpha_without_changing_rgb() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.png");
        let output = dir.path().join("alpha.png");
        let image = ImageBuffer::from_pixel(4, 4, Rgba([20u8, 40, 60, 255]));
        image.save(&source).unwrap();

        alpha_paint_scene_asset_region(
            &source,
            &output,
            SceneAssetAlphaPaintOptions {
                alpha: 80,
                whole_image: false,
                within_regions: Vec::new(),
                within_polygons: vec![vec![
                    SceneAssetNormalizedPoint { x: 0.0, y: 0.0 },
                    SceneAssetNormalizedPoint { x: 0.5, y: 0.0 },
                    SceneAssetNormalizedPoint { x: 0.5, y: 1.0 },
                    SceneAssetNormalizedPoint { x: 0.0, y: 1.0 },
                ]],
                protect_regions: Vec::new(),
            },
            None,
            false,
        )
        .unwrap();

        let edited = load_rgba_image(&output).unwrap();
        assert_equal!(*edited.get_pixel(0, 0), Rgba([20u8, 40, 60, 80]));
        assert_equal!(*edited.get_pixel(3, 0), Rgba([20u8, 40, 60, 255]));
    }

    #[test]
    fn clone_stamp_copies_source_offset_into_bounded_target() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.png");
        let output = dir.path().join("clone.png");
        let mut image = ImageBuffer::from_pixel(4, 4, Rgba([0u8, 0, 0, 255]));
        image.put_pixel(0, 0, Rgba([200u8, 40, 60, 255]));
        image.save(&source).unwrap();

        let report = clone_stamp_scene_asset_region(
            &source,
            &output,
            SceneAssetCloneStampOptions {
                sample_origin: SceneAssetNormalizedPoint { x: 0.0, y: 0.0 },
                target_origin: SceneAssetNormalizedPoint { x: 1.0, y: 1.0 },
                within_regions: Vec::new(),
                within_polygons: vec![vec![
                    SceneAssetNormalizedPoint { x: 0.75, y: 0.75 },
                    SceneAssetNormalizedPoint { x: 1.0, y: 0.75 },
                    SceneAssetNormalizedPoint { x: 1.0, y: 1.0 },
                    SceneAssetNormalizedPoint { x: 0.75, y: 1.0 },
                ]],
                protect_regions: Vec::new(),
            },
            None,
            false,
        )
        .unwrap();

        let edited = load_rgba_image(&output).unwrap();
        assert_equal!(report.changed_pixels, 1);
        assert_equal!(*edited.get_pixel(3, 3), Rgba([200u8, 40, 60, 255]));
        assert_equal!(*edited.get_pixel(0, 0), Rgba([200u8, 40, 60, 255]));
    }

    #[test]
    fn draw_shape_fills_rect_and_stroke_path_draws_outline() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.png");
        let rect_output = dir.path().join("rect.png");
        let stroke_output = dir.path().join("stroke.png");
        let image = ImageBuffer::from_pixel(8, 8, Rgba([0u8, 0, 0, 255]));
        image.save(&source).unwrap();

        let rect_report = draw_scene_asset_shape(
            &source,
            &rect_output,
            SceneAssetDrawShapeOptions {
                shape: SceneAssetDrawShapeKind::Rect,
                color: [20, 200, 40, 255],
                stroke_width: 1,
                fill: true,
                rect: Some(SceneAssetNormalizedRect {
                    x: 0.25,
                    y: 0.25,
                    w: 0.5,
                    h: 0.5,
                }),
                points: Vec::new(),
                protect_regions: Vec::new(),
            },
            None,
            false,
        )
        .unwrap();

        let rect = load_rgba_image(&rect_output).unwrap();
        assert!(rect_report.changed_pixels > 0);
        assert_equal!(*rect.get_pixel(4, 4), Rgba([20u8, 200, 40, 255]));

        stroke_scene_asset_path(
            &source,
            &stroke_output,
            SceneAssetStrokePathOptions {
                path: vec![
                    SceneAssetNormalizedPoint { x: 0.0, y: 0.0 },
                    SceneAssetNormalizedPoint { x: 1.0, y: 0.0 },
                    SceneAssetNormalizedPoint { x: 1.0, y: 1.0 },
                ],
                color: [255, 0, 0, 255],
                width: 1,
                closed: false,
                protect_regions: Vec::new(),
            },
            None,
            false,
        )
        .unwrap();

        let stroke = load_rgba_image(&stroke_output).unwrap();
        assert_equal!(*stroke.get_pixel(0, 0), Rgba([255u8, 0, 0, 255]));
        assert_equal!(*stroke.get_pixel(7, 7), Rgba([255u8, 0, 0, 255]));
    }

    #[test]
    fn crop_pad_and_transform_update_canvas_deterministically() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.png");
        let crop_output = dir.path().join("crop.png");
        let pad_output = dir.path().join("pad.png");
        let transform_output = dir.path().join("transform.png");
        let mut image = ImageBuffer::from_pixel(4, 4, Rgba([0u8, 0, 0, 0]));
        image.put_pixel(1, 1, Rgba([200u8, 40, 60, 255]));
        image.put_pixel(2, 1, Rgba([100u8, 80, 20, 255]));
        image.save(&source).unwrap();

        crop_scene_asset_image(
            &source,
            &crop_output,
            SceneAssetCropOptions {
                rect: None,
                content_bounds: true,
            },
            false,
        )
        .unwrap();
        let cropped = load_rgba_image(&crop_output).unwrap();
        assert_equal!(cropped.width(), 2);
        assert_equal!(cropped.height(), 1);

        pad_scene_asset_image(
            &crop_output,
            &pad_output,
            SceneAssetPadOptions {
                width: 4,
                height: 4,
                anchor: SceneAssetPadAnchor::BottomCenter,
                color: [0, 0, 0, 0],
            },
            false,
        )
        .unwrap();
        let padded = load_rgba_image(&pad_output).unwrap();
        assert_equal!(*padded.get_pixel(1, 3), Rgba([200u8, 40, 60, 255]));

        transform_scene_asset_image(
            &pad_output,
            &transform_output,
            SceneAssetTransformOptions {
                scale: 1.0,
                translate_x: 1,
                translate_y: -1,
                flip_x: true,
                flip_y: false,
                resample: SceneAssetResampleFilter::Nearest,
            },
            false,
        )
        .unwrap();
        let transformed = load_rgba_image(&transform_output).unwrap();
        assert_equal!(*transformed.get_pixel(3, 2), Rgba([200u8, 40, 60, 255]));
    }

    #[test]
    fn tonal_adjustments_can_be_bounded_and_preserve_alpha() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.png");
        let levels_output = dir.path().join("levels.png");
        let hsl_output = dir.path().join("hsl.png");
        let mut image = ImageBuffer::from_pixel(4, 4, Rgba([100u8, 100, 100, 200]));
        image.put_pixel(3, 3, Rgba([200u8, 20, 20, 128]));
        image.save(&source).unwrap();

        levels_scene_asset_image(
            &source,
            &levels_output,
            SceneAssetLevelsOptions {
                channel: SceneAssetColorChannel::Rgb,
                black: 50,
                white: 200,
                gamma: 1.0,
                within_regions: Vec::new(),
                within_polygons: vec![vec![
                    SceneAssetNormalizedPoint { x: 0.0, y: 0.0 },
                    SceneAssetNormalizedPoint { x: 0.5, y: 0.0 },
                    SceneAssetNormalizedPoint { x: 0.5, y: 1.0 },
                    SceneAssetNormalizedPoint { x: 0.0, y: 1.0 },
                ]],
                protect_regions: Vec::new(),
            },
            None,
            false,
        )
        .unwrap();
        let levels = load_rgba_image(&levels_output).unwrap();
        assert_ne!(levels.get_pixel(0, 0)[0], 100);
        assert_equal!(levels.get_pixel(0, 0)[3], 200);
        assert_equal!(*levels.get_pixel(3, 3), Rgba([200u8, 20, 20, 128]));

        hsl_scene_asset_image(
            &source,
            &hsl_output,
            SceneAssetHslOptions {
                hue_degrees: 120.0,
                saturation: 0.0,
                lightness: 0.0,
                within_regions: Vec::new(),
                within_polygons: Vec::new(),
                protect_regions: Vec::new(),
            },
            None,
            false,
        )
        .unwrap();
        let hsl = load_rgba_image(&hsl_output).unwrap();
        assert_ne!(*hsl.get_pixel(3, 3), Rgba([200u8, 20, 20, 128]));
        assert_equal!(hsl.get_pixel(3, 3)[3], 128);
    }

    #[test]
    fn blur_and_unsharp_preserve_dimensions() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.png");
        let blur_output = dir.path().join("blur.png");
        let sharp_output = dir.path().join("sharp.png");
        let mut image = ImageBuffer::from_pixel(5, 5, Rgba([0u8, 0, 0, 255]));
        image.put_pixel(2, 2, Rgba([255u8, 255, 255, 255]));
        image.save(&source).unwrap();

        let blur_report = blur_scene_asset_image(
            &source,
            &blur_output,
            SceneAssetBlurOptions {
                radius: 1.0,
                within_regions: Vec::new(),
                within_polygons: Vec::new(),
                protect_regions: Vec::new(),
            },
            None,
            false,
        )
        .unwrap();
        assert_equal!(blur_report.report.width, 5);
        assert!(blur_report.changed_pixels > 0);

        let sharp_report = unsharp_mask_scene_asset_image(
            &blur_output,
            &sharp_output,
            SceneAssetUnsharpMaskOptions {
                radius: 1.0,
                amount: 1.0,
                threshold: 0,
                within_regions: Vec::new(),
                within_polygons: Vec::new(),
                protect_regions: Vec::new(),
            },
            None,
            false,
        )
        .unwrap();
        assert_equal!(sharp_report.report.height, 5);
        assert!(sharp_report.changed_pixels > 0);
    }

    #[test]
    fn composite_and_state_variants_render_outputs() {
        let dir = tempdir().unwrap();
        let base = dir.path().join("base.png");
        let eye_open = dir.path().join("open.png");
        let eye_closed = dir.path().join("closed.png");
        let composite_output = dir.path().join("composite.png");
        let manifest_path = dir.path().join("manifest.json");
        let render_output = dir.path().join("render.png");
        let frames_path = dir.path().join("frames.json");
        let sheet_output = dir.path().join("sheet.png");
        let index_output = dir.path().join("sheet-index.json");

        ImageBuffer::from_pixel(4, 4, Rgba([10u8, 20, 30, 255]))
            .save(&base)
            .unwrap();
        let mut open = ImageBuffer::from_pixel(4, 4, Rgba([0u8, 0, 0, 0]));
        open.put_pixel(1, 1, Rgba([200u8, 200, 255, 255]));
        open.save(&eye_open).unwrap();
        let mut closed = ImageBuffer::from_pixel(4, 4, Rgba([0u8, 0, 0, 0]));
        closed.put_pixel(1, 1, Rgba([30u8, 30, 80, 255]));
        closed.save(&eye_closed).unwrap();

        composite_scene_asset_layers(
            &composite_output,
            SceneAssetCompositeOptions {
                width: None,
                height: None,
                layers: vec![
                    SceneAssetCompositeLayer {
                        path: base.display().to_string(),
                        blend: SceneAssetBlendMode::Normal,
                        opacity: 1.0,
                        x_offset: 0,
                        y_offset: 0,
                    },
                    SceneAssetCompositeLayer {
                        path: eye_open.display().to_string(),
                        blend: SceneAssetBlendMode::Normal,
                        opacity: 1.0,
                        x_offset: 0,
                        y_offset: 0,
                    },
                ],
            },
            None,
            false,
        )
        .unwrap();
        let composite = load_rgba_image(&composite_output).unwrap();
        assert_equal!(*composite.get_pixel(1, 1), Rgba([200u8, 200, 255, 255]));

        let manifest = create_scene_asset_state_manifest(
            &base,
            &manifest_path,
            SceneAssetStateManifestOptions {
                character: "kiki".to_string(),
                parts: BTreeMap::from([(
                    "eyes".to_string(),
                    vec![
                        eye_open.display().to_string(),
                        eye_closed.display().to_string(),
                    ],
                )]),
            },
            false,
        )
        .unwrap();
        assert_equal!(manifest.parts["eyes"].states.len(), 2);

        render_scene_asset_state(
            &manifest_path,
            &render_output,
            SceneAssetStateRenderOptions {
                states: BTreeMap::from([("eyes".to_string(), "closed".to_string())]),
            },
            false,
        )
        .unwrap();
        let rendered = load_rgba_image(&render_output).unwrap();
        assert_equal!(*rendered.get_pixel(1, 1), Rgba([30u8, 30, 80, 255]));

        write_scene_asset_json(
            &frames_path,
            &vec![
                SceneAssetStateSheetFrame {
                    label: Some("open".to_string()),
                    states: BTreeMap::new(),
                },
                SceneAssetStateSheetFrame {
                    label: Some("closed".to_string()),
                    states: BTreeMap::from([("eyes".to_string(), "closed".to_string())]),
                },
            ],
            true,
            false,
        )
        .unwrap();
        render_scene_asset_state_sheet(
            &manifest_path,
            &frames_path,
            &sheet_output,
            &index_output,
            false,
        )
        .unwrap();
        let sheet = load_rgba_image(&sheet_output).unwrap();
        assert_equal!(sheet.width(), 8);
        assert!(index_output.is_file());
    }

    #[test]
    fn feature_map_validation_rejects_out_of_bounds_regions() {
        let mut feature_map = test_feature_map();
        feature_map.regions.insert(
            "bad".to_string(),
            SceneAssetNormalizedRect {
                x: 0.9,
                y: 0.9,
                w: 0.2,
                h: 0.2,
            },
        );

        assert!(validate_scene_asset_feature_map(&feature_map, 32, 32).is_err());
    }

    #[test]
    fn expression_recipe_edits_region_and_preserves_dimensions() {
        let dir = tempdir().unwrap();
        let image_path = dir.path().join("kiki-neutral.png");
        let output_path = dir.path().join("kiki-surprised.png");
        write_test_image(&image_path);
        let feature_map = test_feature_map();
        let recipe_book = SceneAssetRecipeBook {
            recipe_book_version: 1,
            character: "kiki".to_string(),
            expressions: BTreeMap::from([(
                "surprised".to_string(),
                vec![
                    SceneAssetEditOperation::EraseRegion {
                        region: "mouth".to_string(),
                        soften: 0,
                    },
                    SceneAssetEditOperation::DrawEllipse {
                        region: "mouth".to_string(),
                        stroke: Some("#221111ff".to_string()),
                        fill: Some("#ffbbbbff".to_string()),
                        width: 1,
                    },
                ],
            )]),
            animations: BTreeMap::new(),
        };

        let output = generate_scene_asset_expression(
            &image_path,
            &feature_map,
            &recipe_book,
            "surprised",
            None,
            &output_path,
            false,
        )
        .unwrap();

        assert_equal!(output.report.width, 32);
        assert_equal!(output.report.height, 32);
        assert!(output_path.is_file());
        let edited = load_rgba_image(&output_path).unwrap();
        assert!(edited
            .pixels()
            .any(|pixel| pixel[0] == 255 && pixel[1] == 187));
    }

    #[test]
    fn animation_recipe_generates_named_frames() {
        let dir = tempdir().unwrap();
        let image_path = dir.path().join("kiki-neutral.png");
        let output_dir = dir.path().join("frames");
        write_test_image(&image_path);
        let feature_map = test_feature_map();
        let recipe_book = SceneAssetRecipeBook {
            recipe_book_version: 1,
            character: "kiki".to_string(),
            expressions: BTreeMap::from([
                (
                    "neutral".to_string(),
                    vec![SceneAssetEditOperation::Opacity {
                        region: "torso".to_string(),
                        alpha: 1.0,
                    }],
                ),
                (
                    "breath.1".to_string(),
                    vec![SceneAssetEditOperation::ScaleRegion {
                        region: "torso".to_string(),
                        sx: 1.0,
                        sy: 1.05,
                    }],
                ),
            ]),
            animations: BTreeMap::from([(
                "breath".to_string(),
                SceneAssetAnimationRecipe {
                    fps: 8,
                    frames: vec![
                        SceneAssetAnimationFrame {
                            expression: "neutral".to_string(),
                            output: Some("kiki-breath-0.png".to_string()),
                            duration_ms: Some(180),
                        },
                        SceneAssetAnimationFrame {
                            expression: "breath.1".to_string(),
                            output: Some("kiki-breath-1.png".to_string()),
                            duration_ms: Some(180),
                        },
                    ],
                },
            )]),
        };

        let output = generate_scene_asset_animation(
            &image_path,
            &feature_map,
            &recipe_book,
            "breath",
            None,
            &output_dir,
            "kiki",
            false,
        )
        .unwrap();

        assert_equal!(output.frames.len(), 2);
        assert!(output_dir.join("kiki-breath-0.png").is_file());
        assert!(output_dir.join("kiki-breath-1.png").is_file());
    }

    #[test]
    fn continuity_report_flags_identical_frames_as_warning() {
        let dir = tempdir().unwrap();
        let first = dir.path().join("a.png");
        let second = dir.path().join("b.png");
        write_test_image(&first);
        write_test_image(&second);

        let report = continuity_report_for_scene_asset_frames(&[first, second], 2).unwrap();

        assert_equal!(report.frame_count, 2);
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.contains("pixel-identical")));
    }

    #[test]
    fn export_source_writes_expression_layout() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.png");
        write_test_image(&source);
        let report = export_scene_asset_source_images(
            &source,
            &dir.path().join("source-root"),
            "4cher_set4_vn_sprites",
            "kiki",
            &["neutral".to_string(), "happy".to_string()],
            false,
        )
        .unwrap();

        assert_equal!(report.outputs.len(), 2);
        assert!(dir
            .path()
            .join("source-root/4cher_set4_vn_sprites/kiki-neutral.png")
            .is_file());
    }

    #[test]
    fn remove_background_makes_edge_color_transparent_and_keeps_subject() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.png");
        let output = dir.path().join("transparent.png");
        write_subject_on_background(&source);

        let report = make_scene_asset_background_transparent(
            &source,
            &output,
            8,
            0,
            SceneAssetBackgroundSample::Corners,
            false,
        )
        .unwrap();

        let edited = load_rgba_image(&output).unwrap();
        assert!(report.selected_pixels > 0);
        assert_equal!(edited.get_pixel(0, 0)[3], 0);
        assert_equal!(edited.get_pixel(8, 8)[3], 255);
    }

    #[test]
    fn magic_erase_contiguous_seed_does_not_remove_separate_matching_island() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.png");
        let output = dir.path().join("magic.png");
        write_two_matching_islands(&source);

        let report = magic_erase_scene_asset_image(
            &source,
            &output,
            SceneAssetNormalizedPoint { x: 0.19, y: 0.50 },
            4,
            true,
            0,
            false,
        )
        .unwrap();

        let edited = load_rgba_image(&output).unwrap();
        assert_equal!(report.selected_pixels, 12);
        assert_equal!(edited.get_pixel(3, 3)[3], 0);
        assert_equal!(edited.get_pixel(12, 3)[3], 255);
    }

    #[test]
    fn magic_erase_global_removes_all_matching_pixels() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.png");
        let output = dir.path().join("magic-global.png");
        write_two_matching_islands(&source);

        let report = magic_erase_scene_asset_image(
            &source,
            &output,
            SceneAssetNormalizedPoint { x: 0.19, y: 0.50 },
            4,
            false,
            0,
            false,
        )
        .unwrap();

        let edited = load_rgba_image(&output).unwrap();
        assert_equal!(report.selected_pixels, 24);
        assert_equal!(edited.get_pixel(3, 3)[3], 0);
        assert_equal!(edited.get_pixel(12, 3)[3], 0);
    }

    #[test]
    fn mask_morphology_grows_shrinks_opens_and_closes_selection() {
        let mut pixels = vec![false; 25];
        pixels[mask_index(5, 2, 2)] = true;
        let mask = SceneAssetMask::from_pixels(5, 5, pixels);

        let dilated = mask.dilated(1);
        assert_equal!(dilated.selected_count(), 9);
        assert!(dilated.pixels()[mask_index(5, 1, 1)]);
        assert!(dilated.pixels()[mask_index(5, 3, 3)]);

        let eroded = dilated.eroded(1);
        assert_equal!(eroded.selected_count(), 1);
        assert!(eroded.pixels()[mask_index(5, 2, 2)]);

        let mut noisy = dilated.clone();
        noisy.pixels[mask_index(5, 0, 0)] = true;
        let opened = noisy.opened(1);
        assert!(!opened.pixels()[mask_index(5, 0, 0)]);
        assert!(opened.pixels()[mask_index(5, 2, 2)]);

        let mut holed = vec![true; 25];
        holed[mask_index(5, 2, 2)] = false;
        let closed = SceneAssetMask::from_pixels(5, 5, holed).closed(1);
        assert!(closed.pixels()[mask_index(5, 2, 2)]);
    }

    #[test]
    fn mask_component_cleanup_removes_noise_and_fills_holes() {
        let mut pixels = vec![false; 49];
        for y in 1..6 {
            for x in 1..6 {
                pixels[mask_index(7, x, y)] = true;
            }
        }
        pixels[mask_index(7, 3, 3)] = false;
        pixels[mask_index(7, 0, 0)] = true;
        let mask = SceneAssetMask::from_pixels(7, 7, pixels)
            .without_small_components(2)
            .with_filled_small_holes(1);

        assert!(!mask.pixels()[mask_index(7, 0, 0)]);
        assert!(mask.pixels()[mask_index(7, 3, 3)]);
    }

    #[test]
    fn feathered_mask_creates_partial_alpha_at_selection_edge() {
        let mut image = ImageBuffer::from_pixel(3, 3, Rgba([80u8, 40, 120, 255]));
        let mut mask = vec![false; 9];
        mask[pixel_index(&image, 1, 1)] = true;

        apply_transparency_mask(&mut image, &mask, 1);

        assert_equal!(image.get_pixel(1, 1)[3], 0);
        assert!(image.get_pixel(0, 1)[3] > 0);
        assert!(image.get_pixel(0, 1)[3] < 255);
    }

    #[test]
    fn polished_background_protects_feature_map_regions() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.png");
        let output = dir.path().join("transparent.png");
        let mut image = ImageBuffer::from_pixel(8, 8, Rgba([255u8, 255, 255, 255]));
        for y in 3..5 {
            for x in 3..5 {
                image.put_pixel(x, y, Rgba([250u8, 250, 250, 255]));
            }
        }
        image.save(&source).unwrap();
        let mut regions = BTreeMap::new();
        regions.insert(
            "face".to_string(),
            SceneAssetNormalizedRect {
                x: 0.375,
                y: 0.375,
                w: 0.25,
                h: 0.25,
            },
        );
        let feature_map = SceneAssetFeatureMap {
            feature_map_version: 1,
            character: "kiki".to_string(),
            base: "source.png".to_string(),
            regions,
            anchors: BTreeMap::new(),
        };

        let report = make_scene_asset_background_transparent_polished(
            &source,
            &output,
            SceneAssetMaskPolishOptions {
                tolerance: 10,
                protect_regions: vec!["face".to_string()],
                ..SceneAssetMaskPolishOptions::default()
            },
            Some(&feature_map),
            false,
        )
        .unwrap();

        let edited = load_rgba_image(&output).unwrap();
        assert_equal!(edited.get_pixel(0, 0)[3], 0);
        assert_equal!(edited.get_pixel(3, 3)[3], 255);
        assert_equal!(report.quality.unwrap().protected_regions, 1);
    }

    #[test]
    fn defringe_recolors_light_edge_without_changing_alpha() {
        let mut image = ImageBuffer::from_pixel(5, 5, Rgba([0u8, 0, 0, 0]));
        image.put_pixel(2, 2, Rgba([170u8, 40, 60, 255]));
        image.put_pixel(1, 2, Rgba([245u8, 245, 245, 200]));
        let alpha_before = image.get_pixel(1, 2)[3];

        defringe_scene_asset_edges(&mut image, SceneAssetDefringeMode::White);

        let edited = image.get_pixel(1, 2);
        assert_equal!(edited[3], alpha_before);
        assert!(edited[0] < 245);
        assert!(edited[1] < 245);
        assert!(edited[2] < 245);
    }

    #[test]
    fn color_range_erase_selects_disconnected_white_pockets() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.png");
        let output = dir.path().join("color-range.png");
        let mut image = ImageBuffer::from_pixel(8, 8, Rgba([255u8, 255, 255, 255]));
        for y in 2..6 {
            for x in 2..6 {
                image.put_pixel(x, y, Rgba([230u8, 120, 150, 255]));
            }
        }
        image.put_pixel(4, 4, Rgba([255u8, 255, 255, 255]));
        image.save(&source).unwrap();

        color_range_erase_scene_asset_image(
            &source,
            &output,
            SceneAssetMaskPolishOptions {
                tolerance: 0,
                ..SceneAssetMaskPolishOptions::default()
            },
            None,
            false,
        )
        .unwrap();

        let edited = load_rgba_image(&output).unwrap();
        assert_equal!(edited.get_pixel(0, 0)[3], 0);
        assert_equal!(edited.get_pixel(4, 4)[3], 0);
        assert_equal!(edited.get_pixel(3, 3)[3], 255);
    }

    #[test]
    fn color_range_erase_within_region_does_not_select_outside_pixels() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.png");
        let output = dir.path().join("bounded-color-range.png");
        let image = ImageBuffer::from_pixel(8, 8, Rgba([255u8, 255, 255, 255]));
        image.save(&source).unwrap();
        let mut regions = BTreeMap::new();
        regions.insert(
            "left".to_string(),
            SceneAssetNormalizedRect {
                x: 0.0,
                y: 0.0,
                w: 0.5,
                h: 1.0,
            },
        );
        let feature_map = SceneAssetFeatureMap {
            feature_map_version: 1,
            character: "kiki".to_string(),
            base: "source.png".to_string(),
            regions,
            anchors: BTreeMap::new(),
        };

        let report = color_range_erase_scene_asset_image(
            &source,
            &output,
            SceneAssetMaskPolishOptions {
                tolerance: 0,
                within_regions: vec!["left".to_string()],
                ..SceneAssetMaskPolishOptions::default()
            },
            Some(&feature_map),
            false,
        )
        .unwrap();

        let edited = load_rgba_image(&output).unwrap();
        assert_equal!(report.selected_pixels, 32);
        assert_equal!(edited.get_pixel(1, 1)[3], 0);
        assert_equal!(edited.get_pixel(6, 1)[3], 255);
    }

    #[test]
    fn mask_preview_renders_bounded_selection_without_erasing_source() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.png");
        let output = dir.path().join("preview.png");
        let image = ImageBuffer::from_pixel(8, 8, Rgba([255u8, 255, 255, 255]));
        image.save(&source).unwrap();

        let report = preview_scene_asset_selection_mask(
            &source,
            &output,
            SceneAssetMaskPreviewOptions {
                mode: SceneAssetMaskPreviewMode::ColorRange,
                seeds: Vec::new(),
                threshold: 238,
                neutrality: 28,
                polish: SceneAssetMaskPolishOptions {
                    tolerance: 0,
                    within_polygons: vec![vec![
                        SceneAssetNormalizedPoint { x: 0.0, y: 0.0 },
                        SceneAssetNormalizedPoint { x: 0.5, y: 0.0 },
                        SceneAssetNormalizedPoint { x: 0.5, y: 1.0 },
                        SceneAssetNormalizedPoint { x: 0.0, y: 1.0 },
                    ]],
                    ..SceneAssetMaskPolishOptions::default()
                },
            },
            None,
            false,
        )
        .unwrap();

        let preview = load_rgba_image(&output).unwrap();
        let original = load_rgba_image(&source).unwrap();
        assert_equal!(report.selected_pixels, 32);
        assert!(preview.get_pixel(1, 1)[0] > preview.get_pixel(6, 1)[0]);
        assert_equal!(*original.get_pixel(6, 1), Rgba([255u8, 255, 255, 255]));
    }

    #[test]
    fn mask_export_roundtrips_through_apply_alpha_and_composite() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.png");
        let mask_path = dir.path().join("mask.png");
        let alpha_output = dir.path().join("alpha.png");
        let patch_path = dir.path().join("patch.png");
        let composite_output = dir.path().join("composite.png");
        let mut image = ImageBuffer::from_pixel(4, 4, Rgba([0u8, 0, 0, 255]));
        for y in 0..2 {
            for x in 0..2 {
                image.put_pixel(x, y, Rgba([255, 255, 255, 255]));
            }
        }
        image.save(&source).unwrap();
        ImageBuffer::from_pixel(4, 4, Rgba([255u8, 0, 0, 255]))
            .save(&patch_path)
            .unwrap();

        let mask_report = export_scene_asset_selection_mask(
            &source,
            &mask_path,
            SceneAssetMaskPreviewOptions {
                mode: SceneAssetMaskPreviewMode::MagicAdd,
                seeds: vec![SceneAssetNormalizedPoint { x: 0.125, y: 0.125 }],
                threshold: 238,
                neutrality: 28,
                polish: SceneAssetMaskPolishOptions {
                    tolerance: 0,
                    ..SceneAssetMaskPolishOptions::default()
                },
            },
            None,
            false,
        )
        .unwrap();

        assert_equal!(mask_report.selected_pixels, 4);
        assert_equal!(
            mask_report.selected_bounds,
            Some(SceneAssetPixelRect {
                x: 0,
                y: 0,
                w: 2,
                h: 2
            })
        );

        let alpha_report =
            apply_scene_asset_mask_alpha(&source, &mask_path, &alpha_output, 0, false).unwrap();
        assert_equal!(alpha_report.selected_pixels, 4);
        assert_equal!(alpha_report.changed_pixels, 4);
        let alpha_image = load_rgba_image(&alpha_output).unwrap();
        assert_equal!(alpha_image.get_pixel(0, 0)[3], 0);
        assert_equal!(alpha_image.get_pixel(3, 3)[3], 255);

        let composite_report =
            composite_scene_asset_mask(&source, &patch_path, &mask_path, &composite_output, false)
                .unwrap();
        assert_equal!(composite_report.selected_pixels, 4);
        assert_equal!(composite_report.changed_pixels, 4);
        assert_equal!(
            composite_report.changed_bounds,
            Some(SceneAssetPixelRect {
                x: 0,
                y: 0,
                w: 2,
                h: 2
            })
        );
        let composite = load_rgba_image(&composite_output).unwrap();
        assert_equal!(composite.get_pixel(0, 0).0, [255, 0, 0, 255]);
        assert_equal!(composite.get_pixel(3, 3).0, [0, 0, 0, 255]);
    }

    #[test]
    fn mask_composite_rejects_dimension_mismatch() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.png");
        let patch = dir.path().join("patch.png");
        let mask = dir.path().join("mask.png");
        let output = dir.path().join("output.png");
        ImageBuffer::from_pixel(4, 4, Rgba([0u8, 0, 0, 255]))
            .save(&source)
            .unwrap();
        ImageBuffer::from_pixel(3, 4, Rgba([255u8, 0, 0, 255]))
            .save(&patch)
            .unwrap();
        ImageBuffer::from_pixel(4, 4, Rgba([255u8, 255, 255, 255]))
            .save(&mask)
            .unwrap();

        let err = composite_scene_asset_mask(&source, &patch, &mask, &output, false).unwrap_err();
        assert!(
            matches!(err, SceneAssetEditError::InvalidOperation(message) if message.contains("dimensions differ"))
        );
    }

    #[test]
    fn magic_erase_add_unions_clicked_regions_without_selecting_unclicked_island() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.png");
        let output = dir.path().join("magic-add.png");
        let mut image = ImageBuffer::from_pixel(12, 6, Rgba([60u8, 60, 60, 255]));
        for &(start_x, start_y) in &[(1, 1), (5, 1), (9, 1)] {
            for y in start_y..start_y + 3 {
                for x in start_x..start_x + 2 {
                    image.put_pixel(x, y, Rgba([30u8, 80, 220, 255]));
                }
            }
        }
        image.save(&source).unwrap();

        magic_erase_add_scene_asset_image(
            &source,
            &output,
            &[
                SceneAssetNormalizedPoint {
                    x: 2.0 / 11.0,
                    y: 2.0 / 5.0,
                },
                SceneAssetNormalizedPoint {
                    x: 6.0 / 11.0,
                    y: 2.0 / 5.0,
                },
            ],
            SceneAssetMaskPolishOptions {
                tolerance: 4,
                ..SceneAssetMaskPolishOptions::default()
            },
            None,
            false,
        )
        .unwrap();

        let edited = load_rgba_image(&output).unwrap();
        assert_equal!(edited.get_pixel(2, 2)[3], 0);
        assert_equal!(edited.get_pixel(6, 2)[3], 0);
        assert_equal!(edited.get_pixel(10, 2)[3], 255);
    }

    #[test]
    fn channel_matte_erase_selects_bright_neutral_pockets_not_saturated_hair() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.png");
        let output = dir.path().join("channel-matte.png");
        let mut image = ImageBuffer::from_pixel(6, 4, Rgba([240u8, 150, 170, 255]));
        image.put_pixel(1, 1, Rgba([250u8, 250, 250, 255]));
        image.put_pixel(4, 2, Rgba([245u8, 245, 245, 255]));
        image.save(&source).unwrap();

        channel_matte_erase_scene_asset_image(
            &source,
            &output,
            238,
            16,
            SceneAssetMaskPolishOptions::default(),
            None,
            false,
        )
        .unwrap();

        let edited = load_rgba_image(&output).unwrap();
        assert_equal!(edited.get_pixel(1, 1)[3], 0);
        assert_equal!(edited.get_pixel(4, 2)[3], 0);
        assert_equal!(edited.get_pixel(2, 2)[3], 255);
    }

    #[test]
    fn hair_cleanup_decontaminates_light_edge_and_reports_changed_pixels() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.png");
        let output = dir.path().join("hair-cleanup.png");
        let mut image = ImageBuffer::from_pixel(5, 5, Rgba([0u8, 0, 0, 0]));
        image.put_pixel(2, 2, Rgba([180u8, 60, 80, 255]));
        image.put_pixel(1, 2, Rgba([250u8, 250, 250, 200]));
        image.save(&source).unwrap();

        let report = cleanup_scene_asset_hair_edges(
            &source,
            &output,
            SceneAssetHairCleanupMode::Decontaminate,
            3,
            0.85,
            None,
            None,
            false,
        )
        .unwrap();

        let edited = load_rgba_image(&output).unwrap();
        assert_equal!(report.changed_pixels, 1);
        assert_equal!(edited.get_pixel(1, 2)[3], 200);
        assert!(edited.get_pixel(1, 2)[0] < 250);
    }

    #[test]
    fn recipe_color_range_erase_applies_new_selection_operation() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.png");
        let output = dir.path().join("recipe-output.png");
        let mut image = ImageBuffer::from_pixel(8, 8, Rgba([255u8, 255, 255, 255]));
        for y in 2..6 {
            for x in 2..6 {
                image.put_pixel(x, y, Rgba([50u8, 100, 220, 255]));
            }
        }
        image.put_pixel(4, 4, Rgba([255u8, 255, 255, 255]));
        image.save(&source).unwrap();
        let feature_map = test_feature_map();
        let recipe_book = SceneAssetRecipeBook {
            recipe_book_version: 1,
            character: "kiki".to_string(),
            expressions: BTreeMap::from([(
                "cutout".to_string(),
                vec![SceneAssetEditOperation::ColorRangeErase {
                    tolerance: 0,
                    feather: 0,
                    sample: SceneAssetBackgroundSample::Corners,
                    erode: 0,
                    dilate: 0,
                    open: 0,
                    close: 0,
                    remove_small: 0,
                    fill_holes: 0,
                    defringe: SceneAssetDefringeMode::None,
                    protect_regions: Vec::new(),
                    within_regions: Vec::new(),
                    within_polygons: Vec::new(),
                }],
            )]),
            animations: BTreeMap::new(),
        };

        generate_scene_asset_expression(
            &source,
            &feature_map,
            &recipe_book,
            "cutout",
            None,
            &output,
            false,
        )
        .unwrap();

        let edited = load_rgba_image(&output).unwrap();
        assert_equal!(edited.get_pixel(0, 0)[3], 0);
        assert_equal!(edited.get_pixel(4, 4)[3], 0);
        assert_equal!(edited.get_pixel(3, 3)[3], 255);
    }

    #[test]
    fn restore_from_source_region_copies_base_pixels_into_cutout() {
        let dir = tempdir().unwrap();
        let base = dir.path().join("base.png");
        let cutout = dir.path().join("cutout.png");
        let output = dir.path().join("restored.png");
        let mut base_image = ImageBuffer::from_pixel(8, 8, Rgba([255u8, 255, 255, 255]));
        for y in 2..4 {
            for x in 2..4 {
                base_image.put_pixel(x, y, Rgba([30u8, 80, 220, 255]));
            }
        }
        base_image.save(&base).unwrap();
        ImageBuffer::from_pixel(8, 8, Rgba([0u8, 0, 0, 0]))
            .save(&cutout)
            .unwrap();
        let mut regions = BTreeMap::new();
        regions.insert(
            "detail".to_string(),
            SceneAssetNormalizedRect {
                x: 0.25,
                y: 0.25,
                w: 0.25,
                h: 0.25,
            },
        );
        let feature_map = SceneAssetFeatureMap {
            feature_map_version: 1,
            character: "kiki".to_string(),
            base: "base.png".to_string(),
            regions,
            anchors: BTreeMap::new(),
        };

        let report = restore_scene_asset_from_source(
            &base,
            &cutout,
            &output,
            SceneAssetRestoreOptions {
                regions: vec!["detail".to_string()],
                ..SceneAssetRestoreOptions::default()
            },
            Some(&feature_map),
            false,
        )
        .unwrap();

        let edited = load_rgba_image(&output).unwrap();
        assert_equal!(report.restored_pixels, 4);
        assert_equal!(*edited.get_pixel(2, 2), Rgba([30u8, 80, 220, 255]));
        assert_equal!(edited.get_pixel(0, 0)[3], 0);
    }

    #[test]
    fn restore_from_source_polygon_copies_traced_shape() {
        let dir = tempdir().unwrap();
        let base = dir.path().join("base.png");
        let cutout = dir.path().join("cutout.png");
        let output = dir.path().join("restored.png");
        ImageBuffer::from_pixel(8, 8, Rgba([180u8, 60, 80, 255]))
            .save(&base)
            .unwrap();
        ImageBuffer::from_pixel(8, 8, Rgba([0u8, 0, 0, 0]))
            .save(&cutout)
            .unwrap();

        restore_scene_asset_from_source(
            &base,
            &cutout,
            &output,
            SceneAssetRestoreOptions {
                polygons: vec![vec![
                    SceneAssetNormalizedPoint { x: 0.0, y: 0.0 },
                    SceneAssetNormalizedPoint { x: 0.5, y: 0.0 },
                    SceneAssetNormalizedPoint { x: 0.5, y: 0.5 },
                    SceneAssetNormalizedPoint { x: 0.0, y: 0.5 },
                ]],
                ..SceneAssetRestoreOptions::default()
            },
            None,
            false,
        )
        .unwrap();

        let edited = load_rgba_image(&output).unwrap();
        assert_equal!(edited.get_pixel(1, 1)[3], 255);
        assert_equal!(edited.get_pixel(6, 6)[3], 0);
    }

    #[test]
    fn restore_from_source_non_background_filter_skips_white_pixels() {
        let dir = tempdir().unwrap();
        let base = dir.path().join("base.png");
        let cutout = dir.path().join("cutout.png");
        let output = dir.path().join("restored.png");
        let mut base_image = ImageBuffer::from_pixel(8, 8, Rgba([255u8, 255, 255, 255]));
        base_image.put_pixel(3, 3, Rgba([180u8, 60, 80, 255]));
        base_image.save(&base).unwrap();
        ImageBuffer::from_pixel(8, 8, Rgba([0u8, 0, 0, 0]))
            .save(&cutout)
            .unwrap();

        let report = restore_scene_asset_from_source(
            &base,
            &cutout,
            &output,
            SceneAssetRestoreOptions {
                polygons: vec![vec![
                    SceneAssetNormalizedPoint { x: 0.0, y: 0.0 },
                    SceneAssetNormalizedPoint { x: 1.0, y: 0.0 },
                    SceneAssetNormalizedPoint { x: 1.0, y: 1.0 },
                    SceneAssetNormalizedPoint { x: 0.0, y: 1.0 },
                ]],
                filter: SceneAssetRestoreFilter::NonBackground,
                tolerance: 4,
                ..SceneAssetRestoreOptions::default()
            },
            None,
            false,
        )
        .unwrap();

        let edited = load_rgba_image(&output).unwrap();
        assert_equal!(report.restored_pixels, 1);
        assert_equal!(edited.get_pixel(0, 0)[3], 0);
        assert_equal!(*edited.get_pixel(3, 3), Rgba([180u8, 60, 80, 255]));
    }

    #[test]
    fn recipe_restore_from_source_rehydrates_region() {
        let dir = tempdir().unwrap();
        let base = dir.path().join("base.png");
        let cutout = dir.path().join("cutout.png");
        let output = dir.path().join("recipe-restored.png");
        let mut base_image = ImageBuffer::from_pixel(8, 8, Rgba([255u8, 255, 255, 255]));
        for y in 2..4 {
            for x in 2..4 {
                base_image.put_pixel(x, y, Rgba([40u8, 120, 220, 255]));
            }
        }
        base_image.save(&base).unwrap();
        ImageBuffer::from_pixel(8, 8, Rgba([0u8, 0, 0, 0]))
            .save(&cutout)
            .unwrap();
        let mut regions = BTreeMap::new();
        regions.insert(
            "detail".to_string(),
            SceneAssetNormalizedRect {
                x: 0.25,
                y: 0.25,
                w: 0.25,
                h: 0.25,
            },
        );
        let feature_map = SceneAssetFeatureMap {
            feature_map_version: 1,
            character: "kiki".to_string(),
            base: "cutout.png".to_string(),
            regions,
            anchors: BTreeMap::new(),
        };
        let recipe_book = SceneAssetRecipeBook {
            recipe_book_version: 1,
            character: "kiki".to_string(),
            expressions: BTreeMap::from([(
                "restored".to_string(),
                vec![SceneAssetEditOperation::RestoreFromSource {
                    path: base.display().to_string(),
                    regions: vec!["detail".to_string()],
                    polygons: Vec::new(),
                    filter: SceneAssetRestoreFilter::All,
                    tolerance: 24,
                    sample: SceneAssetBackgroundSample::Corners,
                }],
            )]),
            animations: BTreeMap::new(),
        };

        generate_scene_asset_expression(
            &cutout,
            &feature_map,
            &recipe_book,
            "restored",
            None,
            &output,
            false,
        )
        .unwrap();

        let edited = load_rgba_image(&output).unwrap();
        assert_equal!(*edited.get_pixel(2, 2), Rgba([40u8, 120, 220, 255]));
        assert_equal!(edited.get_pixel(0, 0)[3], 0);
    }
}
