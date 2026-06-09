use super::io::{load_rgba_image, save_rgba_image};
use super::mask::{mask_index, SceneAssetMask};
use super::model::*;
use super::paint_ops::paint_bounds_mask;
use super::pixels::{
    changed_pixel_count, color_channels, crop_region, normalized_rect_arg, paste_region,
};
use super::{content_bounds, inspect_scene_asset_image};
use image::{ImageBuffer, Rgba, RgbaImage};
use std::path::Path;

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
