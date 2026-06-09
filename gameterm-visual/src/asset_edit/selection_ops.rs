use super::io::{load_rgba_image, save_rgba_image};
use super::mask::{mask_index, SceneAssetMask};
use super::model::*;
use super::pixels::{lerp, normalized_point_to_pixel, pixel_index, pixel_len};
use super::{inspect_scene_asset_image, rounded_f32, validate_scene_asset_feature_map};
use image::{ImageBuffer, Rgba, RgbaImage};
use std::collections::VecDeque;
use std::path::Path;

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

pub(super) fn background_magic_mask(
    image: &RgbaImage,
    tolerance: u8,
    sample: SceneAssetBackgroundSample,
) -> Vec<bool> {
    let sample_colors = background_sample_colors(image, sample);
    let seeds = edge_seed_points_matching_samples(image, &sample_colors, tolerance);
    contiguous_magic_mask_with_samples(image, &seeds, &sample_colors, tolerance)
}

pub(super) fn polished_background_mask(
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

pub(super) fn restore_pixels_from_source_image(
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

pub(super) fn apply_mask_polish(
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

pub(super) fn color_range_mask(
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

pub(super) fn multi_seed_contiguous_mask(
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

pub(super) fn channel_matte_mask(image: &RgbaImage, threshold: u8, neutrality: u8) -> Vec<bool> {
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

pub(super) fn contiguous_magic_mask(
    image: &RgbaImage,
    seeds: &[(u32, u32)],
    tolerance: u8,
) -> Vec<bool> {
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

pub(super) fn global_magic_mask(
    image: &RgbaImage,
    sample_color: Rgba<u8>,
    tolerance: u8,
) -> Vec<bool> {
    image
        .pixels()
        .map(|pixel| pixel_matches(*pixel, sample_color, tolerance))
        .collect()
}

pub(super) fn apply_transparency_mask(image: &mut RgbaImage, mask: &[bool], feather: u32) {
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

pub(super) fn defringe_scene_asset_edges(image: &mut RgbaImage, mode: SceneAssetDefringeMode) {
    if mode == SceneAssetDefringeMode::None {
        return;
    }
    decontaminate_light_edges(image, 4, 0.75, None);
}

pub(super) fn decontaminate_light_edges(
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
