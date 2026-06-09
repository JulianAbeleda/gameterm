use super::io::{load_json, load_rgba_image, read_file, write_file};
use super::model::*;
use super::operation_support::{compare_protected_regions, operation_expectation_failures};
use super::paint_ops::{
    alpha_paint_scene_asset_region, clone_stamp_scene_asset_region, draw_scene_asset_shape,
    fill_scene_asset_region, sample_fill_scene_asset_region, sample_scene_asset_image,
    stroke_scene_asset_path,
};
use super::pipeline_args::*;
use super::pixels::pixel_len;
use super::review::{operation_preview_output, write_scene_asset_operation_review_previews};
use super::roots::{
    pipeline_report_path, resolve_asset_accept_output_path, resolve_asset_accept_source_path,
    resolve_asset_operation_source_path, resolve_pipeline_input_path, resolve_pipeline_output_path,
    resolve_recipe_path,
};
use super::selection_ops::{
    apply_scene_asset_mask_alpha, cleanup_scene_asset_hair_edges,
    color_range_erase_scene_asset_image, composite_scene_asset_mask,
    export_scene_asset_selection_mask, magic_erase_add_scene_asset_image,
    make_scene_asset_background_transparent, make_scene_asset_background_transparent_polished,
    preview_scene_asset_selection_mask,
};
use super::transform_ops::{
    blur_scene_asset_image, brightness_contrast_scene_asset_image, crop_scene_asset_image,
    hsl_scene_asset_image, levels_scene_asset_image, pad_scene_asset_image,
    transform_scene_asset_image, unsharp_mask_scene_asset_image,
};
use super::{inspect_scene_asset_image, rounded_f32, write_scene_asset_json};
use std::path::{Path, PathBuf};

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
