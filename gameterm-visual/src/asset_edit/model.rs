use image::imageops::FilterType;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

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
    pub(crate) fn right(self) -> u32 {
        self.x.saturating_add(self.w)
    }

    pub(crate) fn bottom(self) -> u32 {
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
    pub(crate) fn to_pixels(self, width: u32, height: u32) -> SceneAssetPixelRect {
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
pub struct SceneAssetPipeline {
    pub asset_pipeline_version: u64,
    pub name: String,
    pub input: String,
    pub steps: Vec<SceneAssetPipelineStep>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetPipelineStep {
    pub command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub args: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneAssetPipelineRoots {
    pub input_root: PathBuf,
    pub transformation_root: PathBuf,
    pub output_root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneAssetPipelineRunOptions {
    pub force: bool,
    pub dry_run: bool,
    pub pretty: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetPipelineRunReport {
    pub operation: String,
    pub name: String,
    pub input: String,
    pub final_source: String,
    pub dry_run: bool,
    pub steps: Vec<SceneAssetPipelineStepReport>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetPipelineStepReport {
    pub index: usize,
    pub command: String,
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report_path: Option<String>,
    pub advanced_source: bool,
    pub dry_run: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetOperation {
    pub asset_operation_version: u64,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intent: Option<String>,
    pub source: String,
    pub output: String,
    pub command: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub args: BTreeMap<String, serde_json::Value>,
    #[serde(
        default,
        skip_serializing_if = "SceneAssetOperationExpectations::is_empty"
    )]
    pub expectations: SceneAssetOperationExpectations,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetOperationExpectations {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_changed_pixel_ratio: Option<f32>,
    #[serde(default)]
    pub must_preserve_alpha_outside_region: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub must_preserve_regions: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_changed_pixels_in_protected_regions: Option<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub review_points: Vec<String>,
}

impl SceneAssetOperationExpectations {
    pub(crate) fn is_empty(&self) -> bool {
        self.max_changed_pixel_ratio.is_none()
            && !self.must_preserve_alpha_outside_region
            && self.must_preserve_regions.is_empty()
            && self.max_changed_pixels_in_protected_regions.is_none()
            && self.review_points.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneAssetOperationRunOptions {
    pub force: bool,
    pub dry_run: bool,
    pub preview: bool,
    pub pretty: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetOperationRunReport {
    pub operation: String,
    pub id: String,
    pub command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intent: Option<String>,
    pub source: String,
    pub output_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report_path: Option<String>,
    pub dry_run: bool,
    pub preview: bool,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_output_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diff_preview_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review_preview_paths: Option<SceneAssetReviewPreviewPaths>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before: Option<SceneAssetImageReport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<SceneAssetImageReport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compare: Option<SceneAssetCompareReport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protected_region_report: Option<SceneAssetProtectedRegionReport>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expectation_failures: Vec<String>,
    pub step: SceneAssetPipelineStepReport,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneAssetProtectedRegionChange {
    pub region: String,
    pub changed_pixels: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneAssetProtectedRegionReport {
    pub checked_regions: Vec<String>,
    pub changed_pixels: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub changed_regions: Vec<SceneAssetProtectedRegionChange>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetEditSession {
    pub asset_session_version: u64,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_source: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub accepted_outputs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub operations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetEditSessionRunReport {
    pub operation: String,
    pub name: String,
    pub dry_run: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_source: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub accepted_outputs: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub final_output_path: Option<String>,
    pub operations: Vec<SceneAssetOperationRunReport>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetAcceptOutputReport {
    pub operation: String,
    pub source_path: String,
    pub output_path: String,
    pub status: String,
    pub image: SceneAssetImageReport,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetOperationValidationReport {
    pub operation: String,
    pub id: String,
    pub status: String,
    pub source_path: String,
    pub requested_output_path: String,
    pub command: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneAssetOperationErrorReport {
    pub operation: String,
    pub status: String,
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
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
    MagicErase {
        seed: SceneAssetNormalizedPoint,
        #[serde(default = "default_magic_tolerance")]
        tolerance: u8,
        #[serde(default = "default_true")]
        contiguous: bool,
        #[serde(default)]
        feather: u32,
    },
    RemoveBackground {
        #[serde(default = "default_magic_tolerance")]
        tolerance: u8,
        #[serde(default)]
        feather: u32,
        #[serde(default = "default_background_sample")]
        sample: SceneAssetBackgroundSample,
    },
    RemoveBackgroundPolished {
        #[serde(default = "default_magic_tolerance")]
        tolerance: u8,
        #[serde(default)]
        feather: u32,
        #[serde(default = "default_background_sample")]
        sample: SceneAssetBackgroundSample,
        #[serde(default)]
        erode: u32,
        #[serde(default)]
        dilate: u32,
        #[serde(default)]
        open: u32,
        #[serde(default)]
        close: u32,
        #[serde(default)]
        remove_small: usize,
        #[serde(default)]
        fill_holes: usize,
        #[serde(default = "default_defringe_mode")]
        defringe: SceneAssetDefringeMode,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        protect_regions: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        within_regions: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        within_polygons: Vec<Vec<SceneAssetNormalizedPoint>>,
    },
    ColorRangeErase {
        #[serde(default = "default_magic_tolerance")]
        tolerance: u8,
        #[serde(default)]
        feather: u32,
        #[serde(default = "default_background_sample")]
        sample: SceneAssetBackgroundSample,
        #[serde(default)]
        erode: u32,
        #[serde(default)]
        dilate: u32,
        #[serde(default)]
        open: u32,
        #[serde(default)]
        close: u32,
        #[serde(default)]
        remove_small: usize,
        #[serde(default)]
        fill_holes: usize,
        #[serde(default = "default_defringe_mode")]
        defringe: SceneAssetDefringeMode,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        protect_regions: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        within_regions: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        within_polygons: Vec<Vec<SceneAssetNormalizedPoint>>,
    },
    MagicEraseAdd {
        seeds: Vec<SceneAssetNormalizedPoint>,
        #[serde(default = "default_magic_tolerance")]
        tolerance: u8,
        #[serde(default)]
        feather: u32,
        #[serde(default)]
        erode: u32,
        #[serde(default)]
        dilate: u32,
        #[serde(default)]
        open: u32,
        #[serde(default)]
        close: u32,
        #[serde(default)]
        remove_small: usize,
        #[serde(default)]
        fill_holes: usize,
        #[serde(default = "default_defringe_mode")]
        defringe: SceneAssetDefringeMode,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        protect_regions: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        within_regions: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        within_polygons: Vec<Vec<SceneAssetNormalizedPoint>>,
    },
    ChannelMatteErase {
        #[serde(default = "default_channel_matte_threshold")]
        threshold: u8,
        #[serde(default = "default_channel_matte_neutrality")]
        neutrality: u8,
        #[serde(default)]
        feather: u32,
        #[serde(default)]
        erode: u32,
        #[serde(default)]
        dilate: u32,
        #[serde(default)]
        open: u32,
        #[serde(default)]
        close: u32,
        #[serde(default)]
        remove_small: usize,
        #[serde(default)]
        fill_holes: usize,
        #[serde(default = "default_defringe_mode")]
        defringe: SceneAssetDefringeMode,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        protect_regions: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        within_regions: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        within_polygons: Vec<Vec<SceneAssetNormalizedPoint>>,
    },
    HairCleanup {
        #[serde(default = "default_hair_cleanup_mode")]
        mode: SceneAssetHairCleanupMode,
        #[serde(default = "default_hair_cleanup_radius")]
        radius: u32,
        #[serde(default = "default_hair_cleanup_strength")]
        strength: f32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        region: Option<String>,
    },
    RestoreFromSource {
        path: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        regions: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        polygons: Vec<Vec<SceneAssetNormalizedPoint>>,
        #[serde(default = "default_restore_filter")]
        filter: SceneAssetRestoreFilter,
        #[serde(default = "default_magic_tolerance")]
        tolerance: u8,
        #[serde(default = "default_background_sample")]
        sample: SceneAssetBackgroundSample,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneAssetBackgroundSample {
    Corners,
    Edges,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneAssetDefringeMode {
    None,
    White,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneAssetHairCleanupMode {
    Decontaminate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneAssetRestoreFilter {
    All,
    NonBackground,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneAssetMaskPreviewMode {
    Background,
    ColorRange,
    MagicAdd,
    ChannelMatte,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetMaskPolishOptions {
    pub tolerance: u8,
    pub feather: u32,
    pub sample: SceneAssetBackgroundSample,
    pub erode: u32,
    pub dilate: u32,
    pub open: u32,
    pub close: u32,
    pub remove_small: usize,
    pub fill_holes: usize,
    pub defringe: SceneAssetDefringeMode,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub protect_regions: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub within_regions: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub within_polygons: Vec<Vec<SceneAssetNormalizedPoint>>,
}

impl Default for SceneAssetMaskPolishOptions {
    fn default() -> Self {
        Self {
            tolerance: default_magic_tolerance(),
            feather: 0,
            sample: default_background_sample(),
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
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetExportReport {
    pub source: String,
    pub source_id: String,
    pub character: String,
    pub outputs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetSelectionReport {
    pub operation: String,
    pub source: String,
    pub output_path: String,
    pub selected_pixels: usize,
    pub total_pixels: usize,
    pub tolerance: u8,
    pub feather: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality: Option<SceneAssetCutoutQualityReport>,
    pub report: SceneAssetImageReport,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetCutoutQualityReport {
    pub protected_regions: usize,
    pub transparent_pixels: usize,
    pub partial_alpha_pixels: usize,
    pub light_edge_pixels: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetHairCleanupReport {
    pub operation: String,
    pub source: String,
    pub output_path: String,
    pub changed_pixels: usize,
    pub radius: u32,
    pub strength: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    pub report: SceneAssetImageReport,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetRestoreOptions {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub regions: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub polygons: Vec<Vec<SceneAssetNormalizedPoint>>,
    pub filter: SceneAssetRestoreFilter,
    pub tolerance: u8,
    pub sample: SceneAssetBackgroundSample,
}

impl Default for SceneAssetRestoreOptions {
    fn default() -> Self {
        Self {
            regions: Vec::new(),
            polygons: Vec::new(),
            filter: SceneAssetRestoreFilter::All,
            tolerance: default_magic_tolerance(),
            sample: default_background_sample(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetRestoreReport {
    pub operation: String,
    pub base: String,
    pub cutout: String,
    pub output_path: String,
    pub restored_pixels: usize,
    pub filter: SceneAssetRestoreFilter,
    pub report: SceneAssetImageReport,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetMaskPreviewOptions {
    pub mode: SceneAssetMaskPreviewMode,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub seeds: Vec<SceneAssetNormalizedPoint>,
    #[serde(default = "default_channel_matte_threshold")]
    pub threshold: u8,
    #[serde(default = "default_channel_matte_neutrality")]
    pub neutrality: u8,
    pub polish: SceneAssetMaskPolishOptions,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetMaskPreviewReport {
    pub operation: String,
    pub source: String,
    pub output_path: String,
    pub mode: SceneAssetMaskPreviewMode,
    pub selected_pixels: usize,
    pub total_pixels: usize,
    pub report: SceneAssetImageReport,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetMaskExportReport {
    pub operation: String,
    pub source: String,
    pub output_path: String,
    pub mode: SceneAssetMaskPreviewMode,
    pub selected_pixels: usize,
    pub total_pixels: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_bounds: Option<SceneAssetPixelRect>,
    pub report: SceneAssetImageReport,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetMaskApplyReport {
    pub operation: String,
    pub source: String,
    pub mask: String,
    pub output_path: String,
    pub selected_pixels: usize,
    pub changed_pixels: usize,
    pub alpha: u8,
    pub report: SceneAssetImageReport,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetMaskCompositeReport {
    pub operation: String,
    pub source: String,
    pub patch: String,
    pub mask: String,
    pub output_path: String,
    pub selected_pixels: usize,
    pub changed_pixels: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub changed_bounds: Option<SceneAssetPixelRect>,
    pub report: SceneAssetImageReport,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetPointSample {
    pub point: SceneAssetNormalizedPoint,
    pub pixel_x: u32,
    pub pixel_y: u32,
    pub rgba: [u8; 4],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetPointReport {
    pub operation: String,
    pub source: String,
    pub samples: Vec<SceneAssetPointSample>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetRegionSample {
    pub pixel_count: usize,
    pub mean_rgba: [f32; 4],
    pub median_rgba: [u8; 4],
    pub alpha_coverage: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetSampleOptions {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub points: Vec<SceneAssetNormalizedPoint>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub within_regions: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub within_polygons: Vec<Vec<SceneAssetNormalizedPoint>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetSampleReport {
    pub operation: String,
    pub source: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub points: Vec<SceneAssetPointSample>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<SceneAssetRegionSample>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetGridPreviewOptions {
    pub step: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetGridPreviewReport {
    pub operation: String,
    pub source: String,
    pub output_path: String,
    pub step: f32,
    pub report: SceneAssetImageReport,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetPaintReport {
    pub operation: String,
    pub source: String,
    pub output_path: String,
    pub changed_pixels: usize,
    pub report: SceneAssetImageReport,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetCompareReport {
    pub operation: String,
    pub before_path: String,
    pub after_path: String,
    pub same_dimensions: bool,
    pub changed_pixels: usize,
    pub changed_pixel_ratio: f32,
    pub alpha_changed_pixels: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub changed_bounds: Option<SceneAssetPixelRect>,
    pub before: SceneAssetImageReport,
    pub after: SceneAssetImageReport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneAssetReviewPreviewMode {
    RawDiff,
    OverlayDiff,
    AlphaDiff,
    Checkerboard,
    Dark,
    ContactSheet,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetReviewPreviewReport {
    pub operation: String,
    pub before_path: String,
    pub after_path: String,
    pub output_path: String,
    pub mode: SceneAssetReviewPreviewMode,
    pub report: SceneAssetImageReport,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneAssetReviewPreviewPaths {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_diff: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overlay_diff: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alpha_diff: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checkerboard: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dark: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact_sheet: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetFillOptions {
    pub color: [u8; 4],
    #[serde(default)]
    pub whole_image: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub within_regions: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub within_polygons: Vec<Vec<SceneAssetNormalizedPoint>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub protect_regions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetSampleFillOptions {
    pub sample_point: SceneAssetNormalizedPoint,
    pub sample_radius: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub within_regions: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub within_polygons: Vec<Vec<SceneAssetNormalizedPoint>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub protect_regions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetAlphaPaintOptions {
    pub alpha: u8,
    #[serde(default)]
    pub whole_image: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub within_regions: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub within_polygons: Vec<Vec<SceneAssetNormalizedPoint>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub protect_regions: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneAssetDrawShapeKind {
    Rect,
    Line,
    Polygon,
    Ellipse,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetCloneStampOptions {
    pub sample_origin: SceneAssetNormalizedPoint,
    pub target_origin: SceneAssetNormalizedPoint,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub within_regions: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub within_polygons: Vec<Vec<SceneAssetNormalizedPoint>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub protect_regions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetDrawShapeOptions {
    pub shape: SceneAssetDrawShapeKind,
    pub color: [u8; 4],
    #[serde(default = "default_stroke_width")]
    pub stroke_width: u32,
    #[serde(default)]
    pub fill: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rect: Option<SceneAssetNormalizedRect>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub points: Vec<SceneAssetNormalizedPoint>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub protect_regions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetStrokePathOptions {
    pub path: Vec<SceneAssetNormalizedPoint>,
    pub color: [u8; 4],
    #[serde(default = "default_stroke_width")]
    pub width: u32,
    #[serde(default)]
    pub closed: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub protect_regions: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneAssetPadAnchor {
    Center,
    BottomCenter,
    TopLeft,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneAssetResampleFilter {
    Nearest,
    Lanczos3,
}

impl SceneAssetResampleFilter {
    pub(crate) fn filter_type(self) -> FilterType {
        match self {
            SceneAssetResampleFilter::Nearest => FilterType::Nearest,
            SceneAssetResampleFilter::Lanczos3 => FilterType::Lanczos3,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetCropOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rect: Option<SceneAssetNormalizedRect>,
    #[serde(default)]
    pub content_bounds: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetPadOptions {
    pub width: u32,
    pub height: u32,
    pub anchor: SceneAssetPadAnchor,
    pub color: [u8; 4],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetTransformOptions {
    #[serde(default = "default_scale")]
    pub scale: f32,
    #[serde(default)]
    pub translate_x: i32,
    #[serde(default)]
    pub translate_y: i32,
    #[serde(default)]
    pub flip_x: bool,
    #[serde(default)]
    pub flip_y: bool,
    pub resample: SceneAssetResampleFilter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneAssetColorChannel {
    Rgb,
    R,
    G,
    B,
    A,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetLevelsOptions {
    pub channel: SceneAssetColorChannel,
    pub black: u8,
    pub white: u8,
    pub gamma: f32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub within_regions: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub within_polygons: Vec<Vec<SceneAssetNormalizedPoint>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub protect_regions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetBrightnessContrastOptions {
    pub brightness: f32,
    pub contrast: f32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub within_regions: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub within_polygons: Vec<Vec<SceneAssetNormalizedPoint>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub protect_regions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetHslOptions {
    pub hue_degrees: f32,
    pub saturation: f32,
    pub lightness: f32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub within_regions: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub within_polygons: Vec<Vec<SceneAssetNormalizedPoint>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub protect_regions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetBlurOptions {
    pub radius: f32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub within_regions: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub within_polygons: Vec<Vec<SceneAssetNormalizedPoint>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub protect_regions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetUnsharpMaskOptions {
    pub radius: f32,
    pub amount: f32,
    pub threshold: u8,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub within_regions: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub within_polygons: Vec<Vec<SceneAssetNormalizedPoint>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub protect_regions: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneAssetBlendMode {
    Normal,
    Add,
    Multiply,
    Screen,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetCompositeLayer {
    pub path: String,
    pub blend: SceneAssetBlendMode,
    pub opacity: f32,
    pub x_offset: i32,
    pub y_offset: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetCompositeOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    pub layers: Vec<SceneAssetCompositeLayer>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetCompositeReport {
    pub operation: String,
    pub output_path: String,
    pub layer_count: usize,
    pub report: SceneAssetImageReport,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetStateManifest {
    pub asset_state_version: u64,
    pub character: String,
    pub base: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub parts: BTreeMap<String, SceneAssetStatePart>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetStatePart {
    pub default: String,
    pub states: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetStateManifestOptions {
    pub character: String,
    pub parts: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetStateRenderOptions {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub states: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetStateSheetFrame {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub states: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetStateSheetIndex {
    pub frames: Vec<SceneAssetStateSheetIndexFrame>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetStateSheetIndexFrame {
    pub index: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub states: BTreeMap<String, String>,
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

fn default_magic_tolerance() -> u8 {
    24
}

fn default_background_sample() -> SceneAssetBackgroundSample {
    SceneAssetBackgroundSample::Corners
}

fn default_defringe_mode() -> SceneAssetDefringeMode {
    SceneAssetDefringeMode::None
}

fn default_hair_cleanup_mode() -> SceneAssetHairCleanupMode {
    SceneAssetHairCleanupMode::Decontaminate
}

fn default_hair_cleanup_radius() -> u32 {
    4
}

fn default_hair_cleanup_strength() -> f32 {
    0.85
}

fn default_restore_filter() -> SceneAssetRestoreFilter {
    SceneAssetRestoreFilter::All
}

fn default_channel_matte_threshold() -> u8 {
    238
}

fn default_channel_matte_neutrality() -> u8 {
    28
}

fn default_true() -> bool {
    true
}
