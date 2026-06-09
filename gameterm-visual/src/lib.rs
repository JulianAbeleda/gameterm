mod asset_edit;
mod compose_state;
mod conditions;
pub mod render;
mod runtime_status;
mod scene_model;
mod scene_runtime;
mod schema;
mod validation;
mod vn_asset_intake;
mod vn_layout;
mod vn_text;
mod workspace_scene;

pub use asset_edit::{
    accept_scene_asset_output, alpha_paint_scene_asset_region, apply_scene_asset_mask_alpha,
    blur_scene_asset_image, brightness_contrast_scene_asset_image,
    channel_matte_erase_scene_asset_image, cleanup_scene_asset_hair_edges,
    clone_stamp_scene_asset_region, color_range_erase_scene_asset_image,
    compare_scene_asset_images, composite_scene_asset_layers, composite_scene_asset_mask,
    continuity_report_for_scene_asset_frames, create_scene_asset_state_manifest,
    crop_scene_asset_image, default_scene_asset_feature_map, draw_scene_asset_shape,
    export_scene_asset_selection_mask, export_scene_asset_source_images, fill_scene_asset_region,
    generate_scene_asset_animation, generate_scene_asset_expression, hsl_scene_asset_image,
    inspect_scene_asset_image, levels_scene_asset_image, load_scene_asset_feature_map,
    load_scene_asset_recipe_book, load_scene_asset_state_manifest,
    magic_erase_add_scene_asset_image, magic_erase_scene_asset_image,
    make_scene_asset_background_transparent, make_scene_asset_background_transparent_polished,
    pad_scene_asset_image, preview_scene_asset_grid, preview_scene_asset_selection_mask,
    render_scene_asset_state, render_scene_asset_state_sheet, report_scene_asset_points,
    restore_scene_asset_from_source, run_scene_asset_edit_session, run_scene_asset_operation,
    run_scene_asset_pipeline, sample_fill_scene_asset_region, sample_scene_asset_image,
    scene_asset_operation_error_report, stroke_scene_asset_path, transform_scene_asset_image,
    unsharp_mask_scene_asset_image, validate_scene_asset_feature_map,
    validate_scene_asset_operation, write_scene_asset_json, write_scene_asset_review_preview,
    SceneAssetAcceptOutputReport, SceneAssetAlphaPaintOptions, SceneAssetAnimationFrame,
    SceneAssetAnimationOutput, SceneAssetAnimationRecipe, SceneAssetBackgroundSample,
    SceneAssetBlendMode, SceneAssetBlurOptions, SceneAssetBrightnessContrastOptions,
    SceneAssetCloneStampOptions, SceneAssetColorChannel, SceneAssetCompareReport,
    SceneAssetCompositeLayer, SceneAssetCompositeOptions, SceneAssetCompositeReport,
    SceneAssetContinuityCheck, SceneAssetContinuityReport, SceneAssetContinuityStatus,
    SceneAssetCropOptions, SceneAssetCutoutQualityReport, SceneAssetDefringeMode,
    SceneAssetDimensions, SceneAssetDrawShapeKind, SceneAssetDrawShapeOptions, SceneAssetEditError,
    SceneAssetEditOperation, SceneAssetEditSession, SceneAssetEditSessionRunReport,
    SceneAssetExportReport, SceneAssetExpressionOutput, SceneAssetFeatureMap,
    SceneAssetFillOptions, SceneAssetGridPreviewOptions, SceneAssetGridPreviewReport,
    SceneAssetHairCleanupMode, SceneAssetHairCleanupReport, SceneAssetHslOptions,
    SceneAssetImageReport, SceneAssetLevelsOptions, SceneAssetMaskApplyReport,
    SceneAssetMaskCompositeReport, SceneAssetMaskExportReport, SceneAssetMaskPolishOptions,
    SceneAssetMaskPreviewMode, SceneAssetMaskPreviewOptions, SceneAssetMaskPreviewReport,
    SceneAssetNormalizedPoint, SceneAssetNormalizedRect, SceneAssetOperation,
    SceneAssetOperationErrorReport, SceneAssetOperationExpectations, SceneAssetOperationRunOptions,
    SceneAssetOperationRunReport, SceneAssetOperationValidationReport, SceneAssetPadAnchor,
    SceneAssetPadOptions, SceneAssetPaintReport, SceneAssetPipeline, SceneAssetPipelineRoots,
    SceneAssetPipelineRunOptions, SceneAssetPipelineRunReport, SceneAssetPipelineStep,
    SceneAssetPipelineStepReport, SceneAssetPixelRect, SceneAssetPointReport,
    SceneAssetPointSample, SceneAssetProtectedRegionChange, SceneAssetProtectedRegionReport,
    SceneAssetRecipeBook, SceneAssetRegionSample, SceneAssetResampleFilter,
    SceneAssetRestoreFilter, SceneAssetRestoreOptions, SceneAssetRestoreReport,
    SceneAssetReviewPreviewMode, SceneAssetReviewPreviewPaths, SceneAssetReviewPreviewReport,
    SceneAssetSampleFillOptions, SceneAssetSampleOptions, SceneAssetSampleReport,
    SceneAssetSelectionReport, SceneAssetStateManifest, SceneAssetStateManifestOptions,
    SceneAssetStatePart, SceneAssetStateRenderOptions, SceneAssetStateSheetFrame,
    SceneAssetStateSheetIndex, SceneAssetStateSheetIndexFrame, SceneAssetStrokePathOptions,
    SceneAssetTransformOptions, SceneAssetUnsharpMaskOptions,
};
#[cfg(test)]
pub(crate) use compose_state::VisualComposePhase;
pub(crate) use compose_state::{VisualComposeMessage, VisualComposeRole};
pub use render::{intersecting_entities_for_row, visible_tiles_for_row};
pub(crate) use scene_model::is_empty_rpg_state;
pub use scene_model::{
    RunCommandTarget, SceneAction, SceneActionKind, SceneActionPolicy, VisualActionRequest,
    VisualCommandFilter, VisualCommandOption, VisualCondition, VisualDialogueLine, VisualEntity,
    VisualEntityKind, VisualInput, VisualInteractiveDebugMenu, VisualInventoryItem, VisualMode,
    VisualModeOutcome, VisualPosition, VisualProcessPhase, VisualProcessState, VisualQuest,
    VisualRelationship, VisualRenderEntity, VisualRenderLayer, VisualRenderSnapshot,
    VisualRenderStageDisplayable, VisualRenderTile, VisualResolvedSprite, VisualRpgState,
    VisualScene, VisualSceneError, VisualSceneLoadStatus, VisualSceneSource,
    VisualSpriteDefinition, VisualSpriteManifest, VisualSpriteManifestError,
    VisualSpriteManifestStatus, VisualStage, VisualStageDisplayable, VisualStageLayer,
    VisualStagePlacement, VisualStat, VisualStateEntry, VisualStateOperation, VisualStateValue,
    VisualView,
};
pub use scene_runtime::{
    SceneRuntime, VisualSceneDebugReport, VisualSceneDialoguePatch, VisualSceneEntityPatch,
    VisualScenePatch, VisualScenePatchError, VisualStoryState, VisualStoryStateError,
};
#[cfg(test)]
pub(crate) use schema::default_scene_mode;
pub use schema::{
    VisualInputBinding, VisualLayerState, VisualLayerTransition, VisualLayerTransitionReport,
    VisualModeDescriptor, VisualModeLifecycle, VisualRuntimeEvent,
};
pub(crate) use validation::{
    relationship_key, validate_dialogue_lines, validate_layers, validate_rpg_state,
    validate_state_entries, validate_state_operations, VisualDialogueLineError,
    VisualStateEntryError,
};
pub use vn_asset_intake::{
    run_vn_asset_intake, VnAssetAttributionManifest, VnAssetBindingCharacter, VnAssetBindings,
    VnAssetCatalog, VnAssetCatalogPolicy, VnAssetCatalogSource, VnAssetIntakeError,
    VnAssetIntakeOptions, VnAssetIntakeReport, VnAssetIntakeWarning, VnAssetIntakeWarningKind,
    VnAssetRepoPolicy, VnAssetUsedAsset,
};
pub use vn_layout::*;
pub use vn_text::{
    dialogue_text_blocks, truncate_to_screen, VisualDialogueTextBlock, VisualDialogueTextBlockKind,
};
pub use workspace_scene::{
    generate_workspace_context_error_scene, generate_workspace_scene, ScenePaneContext,
    SceneWorkspaceContext, WorkspaceSceneReport,
};
