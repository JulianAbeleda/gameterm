use super::io::{load_rgba_image, save_rgba_image};
use super::mask::{mask_index, validate_polygon, SceneAssetMask};
use super::model::*;
use super::pixels::{
    blend_pixel, changed_pixel_count, draw_ellipse, draw_normalized_line, draw_normalized_path,
    draw_rect_outline, fill_region, normalized_point_to_pixel, normalized_rect_arg, pixel_len,
};
use super::{inspect_scene_asset_image, rounded_f32, validate_scene_asset_feature_map};
use image::{Rgba, RgbaImage};
use std::path::Path;

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

pub(super) fn paint_bounds_mask(
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
