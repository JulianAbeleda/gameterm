use super::io::{load_rgba_image, save_rgba_image};
use super::roots::resolve_pipeline_output_path;
use super::{
    inspect_scene_asset_image, SceneAssetEditError, SceneAssetPipelineRoots,
    SceneAssetReviewPreviewMode, SceneAssetReviewPreviewPaths, SceneAssetReviewPreviewReport,
};
use image::{ImageBuffer, Rgba, RgbaImage};
use std::path::Path;

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

pub(crate) fn write_scene_asset_operation_review_previews(
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

pub(crate) fn operation_preview_output(id: &str) -> String {
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
