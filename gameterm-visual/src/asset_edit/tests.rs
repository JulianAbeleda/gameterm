use super::pixels::pixel_index;
use super::selection_ops::apply_transparency_mask;
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
        load_json::<SceneAssetSampleReport>(&transformation_root.join("03-sample.json")).unwrap();
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
    write_scene_asset_json(&dir.path().join("fill.json"), &fill_operation, true, false).unwrap();
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
