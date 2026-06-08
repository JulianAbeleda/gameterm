use image::imageops::FilterType;
use image::{DynamicImage, ImageBuffer, Rgba, RgbaImage};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetImageReport {
    pub asset_document_version: u64,
    pub source: String,
    pub width: u32,
    pub height: u32,
    pub color_type: String,
    pub has_alpha: bool,
    pub transparent_pixel_ratio: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_bounds: Option<SceneAssetPixelRect>,
    pub sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneAssetPixelRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl SceneAssetPixelRect {
    fn right(self) -> u32 {
        self.x.saturating_add(self.w)
    }

    fn bottom(self) -> u32 {
        self.y.saturating_add(self.h)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetNormalizedRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl SceneAssetNormalizedRect {
    fn to_pixels(self, width: u32, height: u32) -> SceneAssetPixelRect {
        let x = scaled_floor(self.x, width);
        let y = scaled_floor(self.y, height);
        let right = scaled_ceil(self.x + self.w, width).clamp(x.saturating_add(1), width);
        let bottom = scaled_ceil(self.y + self.h, height).clamp(y.saturating_add(1), height);
        SceneAssetPixelRect {
            x,
            y,
            w: right.saturating_sub(x),
            h: bottom.saturating_sub(y),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetNormalizedPoint {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetFeatureMap {
    pub feature_map_version: u64,
    pub character: String,
    pub base: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub regions: BTreeMap<String, SceneAssetNormalizedRect>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub anchors: BTreeMap<String, SceneAssetNormalizedPoint>,
}

impl SceneAssetFeatureMap {
    pub fn pixel_region(
        &self,
        region: &str,
        width: u32,
        height: u32,
    ) -> Result<SceneAssetPixelRect, SceneAssetEditError> {
        let rect = self
            .regions
            .get(region)
            .ok_or_else(|| SceneAssetEditError::UnknownRegion(region.to_string()))?;
        Ok(rect.to_pixels(width, height))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetRecipeBook {
    pub recipe_book_version: u64,
    pub character: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub expressions: BTreeMap<String, Vec<SceneAssetEditOperation>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub animations: BTreeMap<String, SceneAssetAnimationRecipe>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetAnimationRecipe {
    pub fps: u32,
    pub frames: Vec<SceneAssetAnimationFrame>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetAnimationFrame {
    pub expression: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum SceneAssetEditOperation {
    EraseRegion {
        region: String,
        #[serde(default)]
        soften: u32,
    },
    FillRegion {
        region: String,
        color: String,
    },
    DrawLine {
        region: String,
        from: SceneAssetNormalizedPoint,
        to: SceneAssetNormalizedPoint,
        color: String,
        #[serde(default = "default_stroke_width")]
        width: u32,
    },
    DrawPolyline {
        region: String,
        points: Vec<SceneAssetNormalizedPoint>,
        color: String,
        #[serde(default = "default_stroke_width")]
        width: u32,
    },
    DrawEllipse {
        region: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        stroke: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fill: Option<String>,
        #[serde(default = "default_stroke_width")]
        width: u32,
    },
    CompositePng {
        region: String,
        path: String,
        #[serde(default = "default_opacity")]
        opacity: f32,
    },
    TranslateRegion {
        region: String,
        #[serde(default)]
        dx: i32,
        #[serde(default)]
        dy: i32,
    },
    ScaleRegion {
        region: String,
        #[serde(default = "default_scale")]
        sx: f32,
        #[serde(default = "default_scale")]
        sy: f32,
    },
    Opacity {
        region: String,
        alpha: f32,
    },
    ColorTint {
        region: String,
        color: String,
        #[serde(default = "default_tint_amount")]
        amount: f32,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetExpressionOutput {
    pub expression: String,
    pub output_path: String,
    pub report: SceneAssetImageReport,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetAnimationOutput {
    pub animation: String,
    pub frames: Vec<SceneAssetExpressionOutput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetContinuityReport {
    pub frame_count: usize,
    pub dimensions: Option<SceneAssetDimensions>,
    pub content_bounds: Vec<Option<SceneAssetPixelRect>>,
    pub checks: Vec<SceneAssetContinuityCheck>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneAssetDimensions {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetContinuityCheck {
    pub name: String,
    pub status: SceneAssetContinuityStatus,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneAssetContinuityStatus {
    Pass,
    Warning,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetExportReport {
    pub source: String,
    pub source_id: String,
    pub character: String,
    pub outputs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SceneAssetEditError {
    #[error("image file error for `{path}`: {message}")]
    ImageFile { path: String, message: String },
    #[error("json file error for `{path}`: {message}")]
    JsonFile { path: String, message: String },
    #[error("json parse error for `{path}`: {message}")]
    JsonParse { path: String, message: String },
    #[error("unknown feature region `{0}`")]
    UnknownRegion(String),
    #[error("invalid feature map: {0}")]
    InvalidFeatureMap(String),
    #[error("invalid color `{0}`; expected #rrggbb or #rrggbbaa")]
    InvalidColor(String),
    #[error("unknown expression `{0}`")]
    UnknownExpression(String),
    #[error("unknown animation `{0}`")]
    UnknownAnimation(String),
    #[error("invalid edit operation: {0}")]
    InvalidOperation(String),
    #[error("refusing to overwrite existing file without --force: {0}")]
    OutputExists(String),
}

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
    if path.exists() && !force {
        return Err(SceneAssetEditError::OutputExists(
            path.display().to_string(),
        ));
    }
    let json = if pretty {
        serde_json::to_string_pretty(value)
    } else {
        serde_json::to_string(value)
    }
    .map_err(|err| SceneAssetEditError::JsonFile {
        path: path.display().to_string(),
        message: err.to_string(),
    })?;
    write_file(path, format!("{json}\n").as_bytes(), force)
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

fn load_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, SceneAssetEditError> {
    let json = std::fs::read_to_string(path).map_err(|err| SceneAssetEditError::JsonFile {
        path: path.display().to_string(),
        message: err.to_string(),
    })?;
    serde_json::from_str(&json).map_err(|err| SceneAssetEditError::JsonParse {
        path: path.display().to_string(),
        message: err.to_string(),
    })
}

fn read_file(path: &Path, kind: &str) -> Result<Vec<u8>, SceneAssetEditError> {
    std::fs::read(path).map_err(|err| SceneAssetEditError::ImageFile {
        path: path.display().to_string(),
        message: format!("{kind}: {err}"),
    })
}

fn write_file(path: &Path, bytes: &[u8], force: bool) -> Result<(), SceneAssetEditError> {
    if path.exists() && !force {
        return Err(SceneAssetEditError::OutputExists(
            path.display().to_string(),
        ));
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| SceneAssetEditError::ImageFile {
            path: parent.display().to_string(),
            message: err.to_string(),
        })?;
    }
    std::fs::write(path, bytes).map_err(|err| SceneAssetEditError::ImageFile {
        path: path.display().to_string(),
        message: err.to_string(),
    })
}

fn load_rgba_image(path: &Path) -> Result<RgbaImage, SceneAssetEditError> {
    image::ImageReader::open(path)
        .map_err(|err| SceneAssetEditError::ImageFile {
            path: path.display().to_string(),
            message: err.to_string(),
        })?
        .decode()
        .map(DynamicImage::into_rgba8)
        .map_err(|err| SceneAssetEditError::ImageFile {
            path: path.display().to_string(),
            message: err.to_string(),
        })
}

fn save_rgba_image(image: &RgbaImage, path: &Path, force: bool) -> Result<(), SceneAssetEditError> {
    if path.exists() && !force {
        return Err(SceneAssetEditError::OutputExists(
            path.display().to_string(),
        ));
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| SceneAssetEditError::ImageFile {
            path: parent.display().to_string(),
            message: err.to_string(),
        })?;
    }
    image
        .save(path)
        .map_err(|err| SceneAssetEditError::ImageFile {
            path: path.display().to_string(),
            message: err.to_string(),
        })
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

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
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

fn point_in_rect(rect: SceneAssetPixelRect, point: SceneAssetNormalizedPoint) -> (i32, i32) {
    let x = rect.x as f32 + point.x.clamp(0.0, 1.0) * rect.w.saturating_sub(1) as f32;
    let y = rect.y as f32 + point.y.clamp(0.0, 1.0) * rect.h.saturating_sub(1) as f32;
    (x.round() as i32, y.round() as i32)
}

fn resolve_recipe_path(path: &str, base_dir: Option<&Path>) -> PathBuf {
    let path = PathBuf::from(path);
    if path.is_absolute() {
        return path;
    }
    base_dir
        .map(|base_dir| base_dir.join(&path))
        .unwrap_or(path)
}

fn scaled_floor(value: f32, limit: u32) -> u32 {
    (value.clamp(0.0, 1.0) * limit as f32)
        .floor()
        .clamp(0.0, limit as f32) as u32
}

fn scaled_ceil(value: f32, limit: u32) -> u32 {
    (value.clamp(0.0, 1.0) * limit as f32)
        .ceil()
        .clamp(0.0, limit as f32) as u32
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

fn default_stroke_width() -> u32 {
    1
}

fn default_opacity() -> f32 {
    1.0
}

fn default_scale() -> f32 {
    1.0
}

fn default_tint_amount() -> f32 {
    0.5
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
}
