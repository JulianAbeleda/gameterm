use super::io::load_rgba_image;
use super::{
    validate_scene_asset_feature_map, SceneAssetCompareReport, SceneAssetEditError,
    SceneAssetFeatureMap, SceneAssetOperationErrorReport, SceneAssetOperationExpectations,
    SceneAssetProtectedRegionChange, SceneAssetProtectedRegionReport,
};
use std::path::Path;

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

pub(crate) fn operation_expectation_failures(
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

pub(crate) fn compare_protected_regions(
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
