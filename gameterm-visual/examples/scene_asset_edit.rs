use gameterm_visual::{
    alpha_paint_scene_asset_region, blur_scene_asset_image, brightness_contrast_scene_asset_image,
    channel_matte_erase_scene_asset_image, cleanup_scene_asset_hair_edges,
    clone_stamp_scene_asset_region, color_range_erase_scene_asset_image,
    compare_scene_asset_images, composite_scene_asset_layers,
    continuity_report_for_scene_asset_frames, create_scene_asset_state_manifest,
    crop_scene_asset_image, default_scene_asset_feature_map, draw_scene_asset_shape,
    export_scene_asset_source_images, fill_scene_asset_region, generate_scene_asset_animation,
    generate_scene_asset_expression, hsl_scene_asset_image, inspect_scene_asset_image,
    levels_scene_asset_image, load_scene_asset_feature_map, load_scene_asset_recipe_book,
    magic_erase_add_scene_asset_image, magic_erase_scene_asset_image,
    make_scene_asset_background_transparent, make_scene_asset_background_transparent_polished,
    pad_scene_asset_image, preview_scene_asset_grid, preview_scene_asset_selection_mask,
    render_scene_asset_state, render_scene_asset_state_sheet, report_scene_asset_points,
    restore_scene_asset_from_source, run_scene_asset_operation, run_scene_asset_pipeline,
    sample_fill_scene_asset_region, sample_scene_asset_image, stroke_scene_asset_path,
    transform_scene_asset_image, unsharp_mask_scene_asset_image, validate_scene_asset_feature_map,
    write_scene_asset_json, SceneAssetAlphaPaintOptions, SceneAssetBackgroundSample,
    SceneAssetBlendMode, SceneAssetBlurOptions, SceneAssetBrightnessContrastOptions,
    SceneAssetCloneStampOptions, SceneAssetColorChannel, SceneAssetCompositeLayer,
    SceneAssetCompositeOptions, SceneAssetCropOptions, SceneAssetDefringeMode,
    SceneAssetDrawShapeKind, SceneAssetDrawShapeOptions, SceneAssetFillOptions,
    SceneAssetGridPreviewOptions, SceneAssetHairCleanupMode, SceneAssetHslOptions,
    SceneAssetLevelsOptions, SceneAssetMaskPolishOptions, SceneAssetMaskPreviewMode,
    SceneAssetMaskPreviewOptions, SceneAssetNormalizedPoint, SceneAssetNormalizedRect,
    SceneAssetOperationRunOptions, SceneAssetPadAnchor, SceneAssetPadOptions,
    SceneAssetPipelineRoots, SceneAssetPipelineRunOptions, SceneAssetResampleFilter,
    SceneAssetRestoreFilter, SceneAssetRestoreOptions, SceneAssetSampleFillOptions,
    SceneAssetSampleOptions, SceneAssetStateManifestOptions, SceneAssetStateRenderOptions,
    SceneAssetStrokePathOptions, SceneAssetTransformOptions, SceneAssetUnsharpMaskOptions,
};
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
struct CliArgs {
    command: Option<String>,
    image: Option<PathBuf>,
    base: Option<PathBuf>,
    feature_map: Option<PathBuf>,
    recipe: Option<PathBuf>,
    expression: Option<String>,
    animation: Option<String>,
    output: Option<PathBuf>,
    cutout: Option<PathBuf>,
    output_dir: Option<PathBuf>,
    output_source_root: Option<PathBuf>,
    source: Option<PathBuf>,
    pipeline: Option<PathBuf>,
    operation: Option<PathBuf>,
    before: Option<PathBuf>,
    after: Option<PathBuf>,
    manifest: Option<PathBuf>,
    frames_json: Option<PathBuf>,
    index: Option<PathBuf>,
    input_root: Option<PathBuf>,
    transformation_root: Option<PathBuf>,
    output_root: Option<PathBuf>,
    width: Option<u32>,
    height: Option<u32>,
    source_id: Option<String>,
    protect: Option<PathBuf>,
    protect_regions: Option<String>,
    within_regions: Option<String>,
    restore_regions: Option<String>,
    restore_filter: Option<SceneAssetRestoreFilter>,
    selection_mode: Option<SceneAssetMaskPreviewMode>,
    draw_shape: Option<SceneAssetDrawShapeKind>,
    anchor: Option<SceneAssetPadAnchor>,
    resample: Option<SceneAssetResampleFilter>,
    channel: Option<SceneAssetColorChannel>,
    layers: Vec<SceneAssetCompositeLayer>,
    parts: Vec<(String, Vec<String>)>,
    states: Vec<(String, String)>,
    character: Option<String>,
    expressions: Option<String>,
    tolerance: Option<u8>,
    feather: Option<u32>,
    erode: Option<u32>,
    dilate: Option<u32>,
    open: Option<u32>,
    close: Option<u32>,
    remove_small: Option<usize>,
    fill_holes: Option<usize>,
    defringe: Option<SceneAssetDefringeMode>,
    threshold: Option<u8>,
    neutrality: Option<u8>,
    radius: Option<u32>,
    strength: Option<f32>,
    mode: Option<SceneAssetHairCleanupMode>,
    hair_region: Option<String>,
    seed_x: Option<f32>,
    seed_y: Option<f32>,
    seeds: Vec<SceneAssetNormalizedPoint>,
    points: Vec<SceneAssetNormalizedPoint>,
    sample_point: Option<SceneAssetNormalizedPoint>,
    sample_origin: Option<SceneAssetNormalizedPoint>,
    target_origin: Option<SceneAssetNormalizedPoint>,
    polygons: Vec<Vec<SceneAssetNormalizedPoint>>,
    within_polygons: Vec<Vec<SceneAssetNormalizedPoint>>,
    path: Vec<SceneAssetNormalizedPoint>,
    rect: Option<SceneAssetNormalizedRect>,
    sample: Option<SceneAssetBackgroundSample>,
    color: Option<[u8; 4]>,
    alpha: Option<u8>,
    stroke: Option<u32>,
    sample_radius: Option<u32>,
    step: Option<f32>,
    scale: Option<f32>,
    black: Option<u8>,
    white: Option<u8>,
    gamma: Option<f32>,
    brightness: Option<f32>,
    contrast: Option<f32>,
    hue: Option<f32>,
    saturation: Option<f32>,
    lightness: Option<f32>,
    amount: Option<f32>,
    translate_x: i32,
    translate_y: i32,
    whole_image: bool,
    global: bool,
    fill: bool,
    closed: bool,
    content_bounds: bool,
    flip_x: bool,
    flip_y: bool,
    frames: Vec<PathBuf>,
    pretty: bool,
    force: bool,
    dry_run: bool,
}

fn usage() {
    eprintln!(
        "Usage:
  cargo run -p gameterm-visual --example scene_asset_edit -- inspect IMAGE [--output PATH] [--pretty]
  cargo run -p gameterm-visual --example scene_asset_edit -- pipeline-run --pipeline PIPELINE.json --input-root DIR --transformation-root DIR --output-root DIR [--dry-run] [--force] [--pretty]
  cargo run -p gameterm-visual --example scene_asset_edit -- operation-run --operation OPERATION.json --input-root DIR --transformation-root DIR --output-root DIR [--output REPORT.json] [--dry-run] [--force] [--pretty]
  cargo run -p gameterm-visual --example scene_asset_edit -- compare --before BEFORE.png --after AFTER.png [--output REPORT.json] [--pretty]
  cargo run -p gameterm-visual --example scene_asset_edit -- sample --source IMAGE [--point X,Y ...] [--within-polygon X,Y;X,Y;X,Y] [--within-regions CSV] [--protect FEATURE_MAP] [--output REPORT] [--pretty]
  cargo run -p gameterm-visual --example scene_asset_edit -- point-report --source IMAGE --point X,Y [--point X,Y ...] [--pretty]
  cargo run -p gameterm-visual --example scene_asset_edit -- grid-preview --source IMAGE --output PATH [--step N] [--force]
  cargo run -p gameterm-visual --example scene_asset_edit -- map-template IMAGE --character NAME --output PATH [--base TEXT] [--force]
  cargo run -p gameterm-visual --example scene_asset_edit -- validate-map --image IMAGE --feature-map PATH
  cargo run -p gameterm-visual --example scene_asset_edit -- expression --base IMAGE --feature-map PATH --recipe PATH --expression NAME --output PATH [--force]
  cargo run -p gameterm-visual --example scene_asset_edit -- animation --base IMAGE --feature-map PATH --recipe PATH --animation NAME --output-dir DIR [--character NAME] [--force]
  cargo run -p gameterm-visual --example scene_asset_edit -- fill-region --source IMAGE --output PATH --color '#RRGGBB[AA]' (--within-polygon X,Y;X,Y;X,Y | --within-regions CSV | --whole-image) [--protect FEATURE_MAP] [--protect-regions CSV] [--force]
  cargo run -p gameterm-visual --example scene_asset_edit -- sample-fill --source IMAGE --output PATH --sample-point X,Y (--within-polygon X,Y;X,Y;X,Y | --within-regions CSV) [--sample-radius N] [--protect FEATURE_MAP] [--protect-regions CSV] [--force]
  cargo run -p gameterm-visual --example scene_asset_edit -- alpha-paint --source IMAGE --output PATH --alpha N (--within-polygon X,Y;X,Y;X,Y | --within-regions CSV | --whole-image) [--protect FEATURE_MAP] [--protect-regions CSV] [--force]
  cargo run -p gameterm-visual --example scene_asset_edit -- clone-stamp --source IMAGE --output PATH --sample-origin X,Y --target-origin X,Y (--within-polygon X,Y;X,Y;X,Y | --within-regions CSV) [--protect FEATURE_MAP] [--protect-regions CSV] [--force]
  cargo run -p gameterm-visual --example scene_asset_edit -- draw-shape --source IMAGE --output PATH --shape rect|line|polygon|ellipse --color '#RRGGBB[AA]' [--rect X,Y,W,H] [--point X,Y ...] [--stroke N] [--fill] [--protect FEATURE_MAP] [--protect-regions CSV] [--force]
  cargo run -p gameterm-visual --example scene_asset_edit -- stroke-path --source IMAGE --output PATH --path X,Y;X,Y[;X,Y] --color '#RRGGBB[AA]' [--width N] [--closed] [--protect FEATURE_MAP] [--protect-regions CSV] [--force]
  cargo run -p gameterm-visual --example scene_asset_edit -- crop --source IMAGE --output PATH (--rect X,Y,W,H | --content-bounds) [--force]
  cargo run -p gameterm-visual --example scene_asset_edit -- pad --source IMAGE --output PATH --width N --height N [--anchor center|bottom-center|top-left] [--color '#RRGGBB[AA]'] [--force]
  cargo run -p gameterm-visual --example scene_asset_edit -- transform --source IMAGE --output PATH [--scale N] [--translate X,Y] [--flip-x] [--flip-y] [--resample nearest|lanczos3] [--force]
  cargo run -p gameterm-visual --example scene_asset_edit -- levels --source IMAGE --output PATH [--channel rgb|r|g|b|a] [--black N] [--white N] [--gamma N] [--within-polygon X,Y;X,Y;X,Y] [--protect FEATURE_MAP] [--protect-regions CSV] [--force]
  cargo run -p gameterm-visual --example scene_asset_edit -- brightness-contrast --source IMAGE --output PATH [--brightness N] [--contrast N] [--within-polygon X,Y;X,Y;X,Y] [--protect FEATURE_MAP] [--protect-regions CSV] [--force]
  cargo run -p gameterm-visual --example scene_asset_edit -- hsl --source IMAGE --output PATH [--hue N] [--saturation N] [--lightness N] [--within-polygon X,Y;X,Y;X,Y] [--protect FEATURE_MAP] [--protect-regions CSV] [--force]
  cargo run -p gameterm-visual --example scene_asset_edit -- blur --source IMAGE --output PATH [--radius N] [--within-polygon X,Y;X,Y;X,Y] [--protect FEATURE_MAP] [--protect-regions CSV] [--force]
  cargo run -p gameterm-visual --example scene_asset_edit -- unsharp-mask --source IMAGE --output PATH [--radius N] [--amount N] [--threshold N] [--within-polygon X,Y;X,Y;X,Y] [--protect FEATURE_MAP] [--protect-regions CSV] [--force]
  cargo run -p gameterm-visual --example scene_asset_edit -- composite --output IMAGE --layer PATH,normal|add|multiply|screen,OPACITY,X,Y [--width N --height N] [--force]
  cargo run -p gameterm-visual --example scene_asset_edit -- state-manifest --base IMAGE --output manifest.json --character NAME [--part NAME=state.png,state2.png] [--force]
  cargo run -p gameterm-visual --example scene_asset_edit -- state-render --manifest manifest.json --output IMAGE [--state part=state] [--force]
  cargo run -p gameterm-visual --example scene_asset_edit -- state-sheet --manifest manifest.json --frames frames.json --output SHEET.png --index frame-index.json [--force]
  cargo run -p gameterm-visual --example scene_asset_edit -- remove-background --source IMAGE --output PATH [--tolerance N] [--feather N] [--sample corners|edges] [--force]
  cargo run -p gameterm-visual --example scene_asset_edit -- remove-background-polished --source IMAGE --output PATH [--tolerance N] [--sample corners|edges] [--erode N] [--dilate N] [--open N] [--close N] [--remove-small N] [--fill-holes N] [--feather N] [--defringe none|white] [--protect FEATURE_MAP] [--protect-regions CSV] [--within-regions CSV] [--within-polygon X,Y;X,Y;X,Y] [--force]
  cargo run -p gameterm-visual --example scene_asset_edit -- color-range-erase --source IMAGE --output PATH [--tolerance N] [--sample corners|edges] [--erode N] [--dilate N] [--open N] [--close N] [--remove-small N] [--fill-holes N] [--feather N] [--defringe none|white] [--protect FEATURE_MAP] [--protect-regions CSV] [--within-regions CSV] [--within-polygon X,Y;X,Y;X,Y] [--force]
  cargo run -p gameterm-visual --example scene_asset_edit -- magic-erase --source IMAGE --output PATH --seed-x N --seed-y N [--tolerance N] [--feather N] [--global] [--force]
  cargo run -p gameterm-visual --example scene_asset_edit -- magic-erase-add --source IMAGE --output PATH --seed X,Y [--seed X,Y ...] [--tolerance N] [--erode N] [--dilate N] [--open N] [--close N] [--remove-small N] [--fill-holes N] [--feather N] [--defringe none|white] [--protect FEATURE_MAP] [--protect-regions CSV] [--within-regions CSV] [--within-polygon X,Y;X,Y;X,Y] [--force]
  cargo run -p gameterm-visual --example scene_asset_edit -- channel-matte-erase --source IMAGE --output PATH [--threshold N] [--neutrality N] [--erode N] [--dilate N] [--open N] [--close N] [--remove-small N] [--fill-holes N] [--feather N] [--defringe none|white] [--protect FEATURE_MAP] [--protect-regions CSV] [--within-regions CSV] [--within-polygon X,Y;X,Y;X,Y] [--force]
  cargo run -p gameterm-visual --example scene_asset_edit -- mask-preview --source IMAGE --output PATH --selection-mode background|color-range|magic-add|channel-matte [--seed X,Y ...] [--threshold N] [--neutrality N] [--tolerance N] [--sample corners|edges] [--within-regions CSV] [--within-polygon X,Y;X,Y;X,Y] [--protect FEATURE_MAP] [--protect-regions CSV] [--force]
  cargo run -p gameterm-visual --example scene_asset_edit -- hair-cleanup --source IMAGE --output PATH [--mode decontaminate] [--radius N] [--strength N] [--protect FEATURE_MAP] [--hair-region NAME] [--force]
  cargo run -p gameterm-visual --example scene_asset_edit -- restore-from-source --base IMAGE --cutout IMAGE --output PATH [--feature-map PATH] [--restore-regions CSV] [--polygon X,Y;X,Y;X,Y] [--restore-filter all|non-background] [--tolerance N] [--sample corners|edges] [--force]
  cargo run -p gameterm-visual --example scene_asset_edit -- continuity FRAME... [--pretty]
  cargo run -p gameterm-visual --example scene_asset_edit -- export-source --source IMAGE --output-source-root DIR --source-id ID --character NAME --expressions CSV [--force]

Options:
  --image PATH               Input image for validate-map.
  --base PATH                Base character image.
  --feature-map PATH         Character feature-map JSON.
  --recipe PATH              Expression/animation recipe JSON.
  --expression NAME          Expression to generate.
  --animation NAME           Animation to generate.
  --output PATH              Output JSON or PNG path.
  --cutout PATH              Damaged transparent cutout for restore-from-source.
  --output-dir PATH          Output directory for generated animation frames.
  --output-source-root PATH  Source-root layout used by scene_vn_asset_intake.
  --source PATH              Source image for export-source.
  --pipeline PATH            Pipeline JSON for pipeline-run.
  --operation PATH           Operation JSON for operation-run.
  --before PATH              Before image for compare.
  --after PATH               After image for compare.
  --manifest PATH            State manifest JSON.
  --frames PATH              State-sheet frames JSON.
  --index PATH               State-sheet frame-index JSON output.
  --input-root PATH          Pipeline input root.
  --transformation-root PATH Pipeline intermediate/output root.
  --output-root PATH         Pipeline final output root.
  --width N                  Width for pad; stroke width for stroke-path.
  --height N                 Height for pad.
  --source-id ID             Catalog source directory.
  --protect PATH             Feature-map JSON used to protect foreground regions.
  --protect-regions CSV      Feature region names to subtract from the erase mask.
  --within-regions CSV       Feature region names to bound selection masks.
  --restore-regions CSV      Feature region names to copy from base into cutout.
  --restore-filter all|non-background
                              Filter copied source pixels. Default: all.
  --character NAME           Character id. Default: kiki.
  --expressions CSV          Expression names for export-source.
  --tolerance N              RGB channel tolerance for magic selection. Default: 24.
  --feather N                Pixel feather radius after selection. Default: 0.
  --erode N                  Shrink selected background mask by N pixels.
  --dilate N                 Grow selected background mask by N pixels.
  --open N                   Remove isolated selected mask noise.
  --close N                  Fill small unselected gaps in the selected mask.
  --remove-small N           Drop selected components smaller than N pixels.
  --fill-holes N             Fill unselected holes up to N pixels.
  --defringe none|white      Recolor light edge pixels after alpha masking.
  --threshold N              Channel matte brightness threshold. Default: 238.
  --neutrality N             Channel matte max RGB spread. Default: 28.
  --radius N                 Hair cleanup sample radius. Default: 4.
  --strength N               Hair cleanup decontamination strength, 0..1. Default: 0.85.
  --mode decontaminate       Hair cleanup mode.
  --hair-region NAME         Optional feature-map region for hair cleanup.
  --seed-x N                 Normalized magic-erase seed x, 0..1.
  --seed-y N                 Normalized magic-erase seed y, 0..1.
  --seed X,Y                 Repeated normalized seed for magic-erase-add.
  --point X,Y                Repeated normalized point for point-report.
  --sample-point X,Y         Normalized source color sample point for sample-fill.
  --sample-radius N          Pixel radius for sample-fill median color. Default: 1.
  --polygon X,Y;X,Y;X,Y      Repeated normalized polygon for restore-from-source.
  --within-polygon X,Y;X,Y;X,Y
                              Repeated normalized polygon to bound selection masks.
  --selection-mode MODE      Mask preview mode: background, color-range, magic-add, channel-matte.
  --color '#RRGGBB[AA]'      Fill color for paint operations.
  --alpha N                  Alpha value for paint operations, 0..255.
  --shape NAME               Draw shape: rect, line, polygon, ellipse.
  --rect X,Y,W,H             Normalized rectangle for draw-shape.
  --path X,Y;X,Y[;X,Y]       Normalized path for stroke-path.
  --stroke N                 Stroke width for draw-shape. Default: 1.
  --sample-origin X,Y        Normalized clone-stamp source origin.
  --target-origin X,Y        Normalized clone-stamp target origin.
  --anchor NAME              Pad anchor: center, bottom-center, top-left.
  --scale N                  Transform scale. Default: 1.0.
  --translate X,Y            Transform translation in pixels.
  --resample NAME            Transform resample: nearest, lanczos3.
  --channel NAME             Color channel for levels: rgb, r, g, b, a.
  --black N                  Levels black point. Default: 0.
  --white N                  Levels white point. Default: 255.
  --gamma N                  Levels gamma. Default: 1.0.
  --brightness N             Brightness delta in 8-bit color units.
  --contrast N               Contrast factor delta; 0 leaves contrast unchanged.
  --hue N                    Hue shift in degrees.
  --saturation N             HSL saturation delta, -1..1.
  --lightness N              HSL lightness delta, -1..1.
  --amount N                 Unsharp amount. Default: 1.0.
  --layer SPEC               Composite layer: path,blend,opacity,x,y.
  --part SPEC                State part: name=file.png,file2.png.
  --state SPEC               State override: part=state.
  --step N                   Normalized grid spacing. Default: 0.1.
  --whole-image              Allow a paint operation to affect the whole image.
  --fill                     Fill draw-shape geometry.
  --closed                   Close stroke-path back to the first point.
  --content-bounds           Crop to visible content bounds.
  --flip-x                   Flip transform horizontally.
  --flip-y                   Flip transform vertically.
  --sample corners|edges     Background samples. Default: corners.
  --global                   Select all matching pixels instead of contiguous seed fill.
  --pretty                   Pretty-print JSON.
  --force                    Overwrite existing files.
  --dry-run                  Validate a pipeline without writing outputs.
  -h, --help                 Show this help."
    );
}

fn main() {
    let args = match parse_args() {
        Ok(args) => args,
        Err(message) => {
            eprintln!("{message}");
            usage();
            std::process::exit(2);
        }
    };

    let result = match args.command.as_deref() {
        Some("inspect") => run_inspect(args),
        Some("pipeline-run") => run_pipeline(args),
        Some("operation-run") => run_operation(args),
        Some("compare") => run_compare(args),
        Some("sample") => run_sample(args),
        Some("point-report") => run_point_report(args),
        Some("grid-preview") => run_grid_preview(args),
        Some("map-template") => run_map_template(args),
        Some("validate-map") => run_validate_map(args),
        Some("expression") => run_expression(args),
        Some("animation") => run_animation(args),
        Some("fill-region") => run_fill_region(args),
        Some("sample-fill") => run_sample_fill(args),
        Some("alpha-paint") => run_alpha_paint(args),
        Some("clone-stamp") => run_clone_stamp(args),
        Some("draw-shape") => run_draw_shape(args),
        Some("stroke-path") => run_stroke_path(args),
        Some("crop") => run_crop(args),
        Some("pad") => run_pad(args),
        Some("transform") => run_transform(args),
        Some("levels") => run_levels(args),
        Some("brightness-contrast") => run_brightness_contrast(args),
        Some("hsl") => run_hsl(args),
        Some("blur") => run_blur(args),
        Some("unsharp-mask") => run_unsharp_mask(args),
        Some("composite") => run_composite(args),
        Some("state-manifest") => run_state_manifest(args),
        Some("state-render") => run_state_render(args),
        Some("state-sheet") => run_state_sheet(args),
        Some("remove-background") => run_remove_background(args),
        Some("remove-background-polished") => run_remove_background_polished(args),
        Some("color-range-erase") => run_color_range_erase(args),
        Some("magic-erase") => run_magic_erase(args),
        Some("magic-erase-add") => run_magic_erase_add(args),
        Some("channel-matte-erase") => run_channel_matte_erase(args),
        Some("mask-preview") => run_mask_preview(args),
        Some("hair-cleanup") => run_hair_cleanup(args),
        Some("restore-from-source") => run_restore_from_source(args),
        Some("continuity") => run_continuity(args),
        Some("export-source") => run_export_source(args),
        Some(command) => Err(format!("unknown command: {command}")),
        None => Err("command is required".to_string()),
    };

    if let Err(message) = result {
        eprintln!("{message}");
        std::process::exit(1);
    }
}

fn run_inspect(args: CliArgs) -> Result<(), String> {
    let image = required_path(args.image, "IMAGE")?;
    let report = inspect_scene_asset_image(&image).map_err(|err| err.to_string())?;
    write_json(args.output.as_deref(), &report, args.pretty, args.force)
}

fn run_pipeline(args: CliArgs) -> Result<(), String> {
    let pipeline = required_path(args.pipeline.clone(), "--pipeline")?;
    let report = run_scene_asset_pipeline(
        &pipeline,
        &SceneAssetPipelineRoots {
            input_root: required_path(args.input_root.clone(), "--input-root")?,
            transformation_root: required_path(
                args.transformation_root.clone(),
                "--transformation-root",
            )?,
            output_root: required_path(args.output_root.clone(), "--output-root")?,
        },
        SceneAssetPipelineRunOptions {
            force: args.force,
            dry_run: args.dry_run,
            pretty: args.pretty,
        },
    )
    .map_err(|err| err.to_string())?;
    write_json(args.output.as_deref(), &report, args.pretty, args.force)
}

fn run_operation(args: CliArgs) -> Result<(), String> {
    let operation = required_path(args.operation.clone(), "--operation")?;
    let report = run_scene_asset_operation(
        &operation,
        &SceneAssetPipelineRoots {
            input_root: required_path(args.input_root.clone(), "--input-root")?,
            transformation_root: required_path(
                args.transformation_root.clone(),
                "--transformation-root",
            )?,
            output_root: required_path(args.output_root.clone(), "--output-root")?,
        },
        SceneAssetOperationRunOptions {
            force: args.force,
            dry_run: args.dry_run,
            pretty: args.pretty,
        },
    )
    .map_err(|err| err.to_string())?;
    write_json(args.output.as_deref(), &report, args.pretty, args.force)
}

fn run_compare(args: CliArgs) -> Result<(), String> {
    let before = required_path(args.before.clone(), "--before")?;
    let after = required_path(args.after.clone(), "--after")?;
    let report = compare_scene_asset_images(&before, &after).map_err(|err| err.to_string())?;
    write_json(args.output.as_deref(), &report, args.pretty, args.force)
}

fn run_sample(args: CliArgs) -> Result<(), String> {
    let source = required_path(args.source.clone(), "--source")?;
    let feature_map = load_optional_protect_map(&args)?;
    let report = sample_scene_asset_image(
        &source,
        SceneAssetSampleOptions {
            points: args.points.clone(),
            within_regions: csv_values(args.within_regions.as_deref()),
            within_polygons: args.within_polygons.clone(),
        },
        feature_map.as_ref(),
    )
    .map_err(|err| err.to_string())?;
    write_json(args.output.as_deref(), &report, args.pretty, args.force)
}

fn run_point_report(args: CliArgs) -> Result<(), String> {
    let source = required_path(args.source.clone(), "--source")?;
    let report = report_scene_asset_points(&source, &args.points).map_err(|err| err.to_string())?;
    write_json(args.output.as_deref(), &report, args.pretty, args.force)
}

fn run_grid_preview(args: CliArgs) -> Result<(), String> {
    let source = required_path(args.source.clone(), "--source")?;
    let output = required_path(args.output.clone(), "--output")?;
    let report = preview_scene_asset_grid(
        &source,
        &output,
        SceneAssetGridPreviewOptions {
            step: args.step.unwrap_or(0.1),
        },
        args.force,
    )
    .map_err(|err| err.to_string())?;
    write_json(None, &report, args.pretty, true)
}

fn run_map_template(args: CliArgs) -> Result<(), String> {
    let image = required_path(args.image, "IMAGE")?;
    let character = args.character.unwrap_or_else(|| "kiki".to_string());
    let map = default_scene_asset_feature_map(&image, &character, args.base.map(display_path))
        .map_err(|err| err.to_string())?;
    let output = required_path(args.output, "--output")?;
    write_scene_asset_json(&output, &map, true, args.force).map_err(|err| err.to_string())
}

fn run_validate_map(args: CliArgs) -> Result<(), String> {
    let image = required_path(args.image, "--image")?;
    let feature_map_path = required_path(args.feature_map, "--feature-map")?;
    let feature_map =
        load_scene_asset_feature_map(&feature_map_path).map_err(|err| err.to_string())?;
    let report = inspect_scene_asset_image(&image).map_err(|err| err.to_string())?;
    validate_scene_asset_feature_map(&feature_map, report.width, report.height)
        .map_err(|err| err.to_string())?;
    println!(
        "Feature map OK for {} ({}x{})",
        image.display(),
        report.width,
        report.height
    );
    Ok(())
}

fn run_expression(args: CliArgs) -> Result<(), String> {
    let base = required_path(args.base, "--base")?;
    let feature_map_path = required_path(args.feature_map, "--feature-map")?;
    let recipe_path = required_path(args.recipe, "--recipe")?;
    let expression = args
        .expression
        .ok_or_else(|| "--expression is required".to_string())?;
    let output = required_path(args.output, "--output")?;
    let feature_map =
        load_scene_asset_feature_map(&feature_map_path).map_err(|err| err.to_string())?;
    let recipe_book = load_scene_asset_recipe_book(&recipe_path).map_err(|err| err.to_string())?;
    let report = generate_scene_asset_expression(
        &base,
        &feature_map,
        &recipe_book,
        &expression,
        recipe_path.parent(),
        &output,
        args.force,
    )
    .map_err(|err| err.to_string())?;
    write_json(None, &report, args.pretty, true)
}

fn run_animation(args: CliArgs) -> Result<(), String> {
    let base = required_path(args.base, "--base")?;
    let feature_map_path = required_path(args.feature_map, "--feature-map")?;
    let recipe_path = required_path(args.recipe, "--recipe")?;
    let animation = args
        .animation
        .ok_or_else(|| "--animation is required".to_string())?;
    let output_dir = required_path(args.output_dir, "--output-dir")?;
    let character = args.character.unwrap_or_else(|| "kiki".to_string());
    let feature_map =
        load_scene_asset_feature_map(&feature_map_path).map_err(|err| err.to_string())?;
    let recipe_book = load_scene_asset_recipe_book(&recipe_path).map_err(|err| err.to_string())?;
    let report = generate_scene_asset_animation(
        &base,
        &feature_map,
        &recipe_book,
        &animation,
        recipe_path.parent(),
        &output_dir,
        &character,
        args.force,
    )
    .map_err(|err| err.to_string())?;
    write_json(None, &report, args.pretty, true)
}

fn run_fill_region(args: CliArgs) -> Result<(), String> {
    let source = required_path(args.source.clone(), "--source")?;
    let output = required_path(args.output.clone(), "--output")?;
    let feature_map = load_optional_protect_map(&args)?;
    let report = fill_scene_asset_region(
        &source,
        &output,
        SceneAssetFillOptions {
            color: args
                .color
                .ok_or_else(|| "--color is required".to_string())?,
            whole_image: args.whole_image,
            within_regions: csv_values(args.within_regions.as_deref()),
            within_polygons: args.within_polygons.clone(),
            protect_regions: csv_values(args.protect_regions.as_deref()),
        },
        feature_map.as_ref(),
        args.force,
    )
    .map_err(|err| err.to_string())?;
    write_json(None, &report, args.pretty, true)
}

fn run_sample_fill(args: CliArgs) -> Result<(), String> {
    let source = required_path(args.source.clone(), "--source")?;
    let output = required_path(args.output.clone(), "--output")?;
    let feature_map = load_optional_protect_map(&args)?;
    let report = sample_fill_scene_asset_region(
        &source,
        &output,
        SceneAssetSampleFillOptions {
            sample_point: args
                .sample_point
                .ok_or_else(|| "--sample-point is required".to_string())?,
            sample_radius: args.sample_radius.unwrap_or(1),
            within_regions: csv_values(args.within_regions.as_deref()),
            within_polygons: args.within_polygons.clone(),
            protect_regions: csv_values(args.protect_regions.as_deref()),
        },
        feature_map.as_ref(),
        args.force,
    )
    .map_err(|err| err.to_string())?;
    write_json(None, &report, args.pretty, true)
}

fn run_alpha_paint(args: CliArgs) -> Result<(), String> {
    let source = required_path(args.source.clone(), "--source")?;
    let output = required_path(args.output.clone(), "--output")?;
    let feature_map = load_optional_protect_map(&args)?;
    let report = alpha_paint_scene_asset_region(
        &source,
        &output,
        SceneAssetAlphaPaintOptions {
            alpha: args
                .alpha
                .ok_or_else(|| "--alpha is required".to_string())?,
            whole_image: args.whole_image,
            within_regions: csv_values(args.within_regions.as_deref()),
            within_polygons: args.within_polygons.clone(),
            protect_regions: csv_values(args.protect_regions.as_deref()),
        },
        feature_map.as_ref(),
        args.force,
    )
    .map_err(|err| err.to_string())?;
    write_json(None, &report, args.pretty, true)
}

fn run_clone_stamp(args: CliArgs) -> Result<(), String> {
    let source = required_path(args.source.clone(), "--source")?;
    let output = required_path(args.output.clone(), "--output")?;
    let feature_map = load_optional_protect_map(&args)?;
    let report = clone_stamp_scene_asset_region(
        &source,
        &output,
        SceneAssetCloneStampOptions {
            sample_origin: args
                .sample_origin
                .ok_or_else(|| "--sample-origin is required".to_string())?,
            target_origin: args
                .target_origin
                .ok_or_else(|| "--target-origin is required".to_string())?,
            within_regions: csv_values(args.within_regions.as_deref()),
            within_polygons: args.within_polygons.clone(),
            protect_regions: csv_values(args.protect_regions.as_deref()),
        },
        feature_map.as_ref(),
        args.force,
    )
    .map_err(|err| err.to_string())?;
    write_json(None, &report, args.pretty, true)
}

fn run_draw_shape(args: CliArgs) -> Result<(), String> {
    let source = required_path(args.source.clone(), "--source")?;
    let output = required_path(args.output.clone(), "--output")?;
    let feature_map = load_optional_protect_map(&args)?;
    let report = draw_scene_asset_shape(
        &source,
        &output,
        SceneAssetDrawShapeOptions {
            shape: args
                .draw_shape
                .ok_or_else(|| "--shape is required".to_string())?,
            color: args
                .color
                .ok_or_else(|| "--color is required".to_string())?,
            stroke_width: args.stroke.unwrap_or(1),
            fill: args.fill,
            rect: args.rect,
            points: args.points.clone(),
            protect_regions: csv_values(args.protect_regions.as_deref()),
        },
        feature_map.as_ref(),
        args.force,
    )
    .map_err(|err| err.to_string())?;
    write_json(None, &report, args.pretty, true)
}

fn run_stroke_path(args: CliArgs) -> Result<(), String> {
    let source = required_path(args.source.clone(), "--source")?;
    let output = required_path(args.output.clone(), "--output")?;
    let feature_map = load_optional_protect_map(&args)?;
    let report = stroke_scene_asset_path(
        &source,
        &output,
        SceneAssetStrokePathOptions {
            path: args.path.clone(),
            color: args
                .color
                .ok_or_else(|| "--color is required".to_string())?,
            width: args.stroke.unwrap_or(1),
            closed: args.closed,
            protect_regions: csv_values(args.protect_regions.as_deref()),
        },
        feature_map.as_ref(),
        args.force,
    )
    .map_err(|err| err.to_string())?;
    write_json(None, &report, args.pretty, true)
}

fn run_crop(args: CliArgs) -> Result<(), String> {
    let source = required_path(args.source.clone(), "--source")?;
    let output = required_path(args.output.clone(), "--output")?;
    let report = crop_scene_asset_image(
        &source,
        &output,
        SceneAssetCropOptions {
            rect: args.rect,
            content_bounds: args.content_bounds,
        },
        args.force,
    )
    .map_err(|err| err.to_string())?;
    write_json(None, &report, args.pretty, true)
}

fn run_pad(args: CliArgs) -> Result<(), String> {
    let source = required_path(args.source.clone(), "--source")?;
    let output = required_path(args.output.clone(), "--output")?;
    let report = pad_scene_asset_image(
        &source,
        &output,
        SceneAssetPadOptions {
            width: args
                .width
                .ok_or_else(|| "--width is required".to_string())?,
            height: args
                .height
                .ok_or_else(|| "--height is required".to_string())?,
            anchor: args.anchor.unwrap_or(SceneAssetPadAnchor::Center),
            color: args.color.unwrap_or([0, 0, 0, 0]),
        },
        args.force,
    )
    .map_err(|err| err.to_string())?;
    write_json(None, &report, args.pretty, true)
}

fn run_transform(args: CliArgs) -> Result<(), String> {
    let source = required_path(args.source.clone(), "--source")?;
    let output = required_path(args.output.clone(), "--output")?;
    let report = transform_scene_asset_image(
        &source,
        &output,
        SceneAssetTransformOptions {
            scale: args.scale.unwrap_or(1.0),
            translate_x: args.translate_x,
            translate_y: args.translate_y,
            flip_x: args.flip_x,
            flip_y: args.flip_y,
            resample: args.resample.unwrap_or(SceneAssetResampleFilter::Lanczos3),
        },
        args.force,
    )
    .map_err(|err| err.to_string())?;
    write_json(None, &report, args.pretty, true)
}

fn run_levels(args: CliArgs) -> Result<(), String> {
    let source = required_path(args.source.clone(), "--source")?;
    let output = required_path(args.output.clone(), "--output")?;
    let feature_map = load_optional_protect_map(&args)?;
    let report = levels_scene_asset_image(
        &source,
        &output,
        SceneAssetLevelsOptions {
            channel: args.channel.unwrap_or(SceneAssetColorChannel::Rgb),
            black: args.black.unwrap_or(0),
            white: args.white.unwrap_or(255),
            gamma: args.gamma.unwrap_or(1.0),
            within_regions: csv_values(args.within_regions.as_deref()),
            within_polygons: args.within_polygons.clone(),
            protect_regions: csv_values(args.protect_regions.as_deref()),
        },
        feature_map.as_ref(),
        args.force,
    )
    .map_err(|err| err.to_string())?;
    write_json(None, &report, args.pretty, true)
}

fn run_brightness_contrast(args: CliArgs) -> Result<(), String> {
    let source = required_path(args.source.clone(), "--source")?;
    let output = required_path(args.output.clone(), "--output")?;
    let feature_map = load_optional_protect_map(&args)?;
    let report = brightness_contrast_scene_asset_image(
        &source,
        &output,
        SceneAssetBrightnessContrastOptions {
            brightness: args.brightness.unwrap_or(0.0),
            contrast: args.contrast.unwrap_or(0.0),
            within_regions: csv_values(args.within_regions.as_deref()),
            within_polygons: args.within_polygons.clone(),
            protect_regions: csv_values(args.protect_regions.as_deref()),
        },
        feature_map.as_ref(),
        args.force,
    )
    .map_err(|err| err.to_string())?;
    write_json(None, &report, args.pretty, true)
}

fn run_hsl(args: CliArgs) -> Result<(), String> {
    let source = required_path(args.source.clone(), "--source")?;
    let output = required_path(args.output.clone(), "--output")?;
    let feature_map = load_optional_protect_map(&args)?;
    let report = hsl_scene_asset_image(
        &source,
        &output,
        SceneAssetHslOptions {
            hue_degrees: args.hue.unwrap_or(0.0),
            saturation: args.saturation.unwrap_or(0.0),
            lightness: args.lightness.unwrap_or(0.0),
            within_regions: csv_values(args.within_regions.as_deref()),
            within_polygons: args.within_polygons.clone(),
            protect_regions: csv_values(args.protect_regions.as_deref()),
        },
        feature_map.as_ref(),
        args.force,
    )
    .map_err(|err| err.to_string())?;
    write_json(None, &report, args.pretty, true)
}

fn run_blur(args: CliArgs) -> Result<(), String> {
    let source = required_path(args.source.clone(), "--source")?;
    let output = required_path(args.output.clone(), "--output")?;
    let feature_map = load_optional_protect_map(&args)?;
    let report = blur_scene_asset_image(
        &source,
        &output,
        SceneAssetBlurOptions {
            radius: args.radius.unwrap_or(1) as f32,
            within_regions: csv_values(args.within_regions.as_deref()),
            within_polygons: args.within_polygons.clone(),
            protect_regions: csv_values(args.protect_regions.as_deref()),
        },
        feature_map.as_ref(),
        args.force,
    )
    .map_err(|err| err.to_string())?;
    write_json(None, &report, args.pretty, true)
}

fn run_unsharp_mask(args: CliArgs) -> Result<(), String> {
    let source = required_path(args.source.clone(), "--source")?;
    let output = required_path(args.output.clone(), "--output")?;
    let feature_map = load_optional_protect_map(&args)?;
    let report = unsharp_mask_scene_asset_image(
        &source,
        &output,
        SceneAssetUnsharpMaskOptions {
            radius: args.radius.unwrap_or(1) as f32,
            amount: args.amount.unwrap_or(1.0),
            threshold: args.threshold.unwrap_or(0),
            within_regions: csv_values(args.within_regions.as_deref()),
            within_polygons: args.within_polygons.clone(),
            protect_regions: csv_values(args.protect_regions.as_deref()),
        },
        feature_map.as_ref(),
        args.force,
    )
    .map_err(|err| err.to_string())?;
    write_json(None, &report, args.pretty, true)
}

fn run_composite(args: CliArgs) -> Result<(), String> {
    let output = required_path(args.output.clone(), "--output")?;
    let report = composite_scene_asset_layers(
        &output,
        SceneAssetCompositeOptions {
            width: args.width,
            height: args.height,
            layers: args.layers.clone(),
        },
        None,
        args.force,
    )
    .map_err(|err| err.to_string())?;
    write_json(None, &report, args.pretty, true)
}

fn run_state_manifest(args: CliArgs) -> Result<(), String> {
    let base = required_path(args.base.clone(), "--base")?;
    let output = required_path(args.output.clone(), "--output")?;
    let character = args.character.unwrap_or_else(|| "kiki".to_string());
    let report = create_scene_asset_state_manifest(
        &base,
        &output,
        SceneAssetStateManifestOptions {
            character,
            parts: args.parts.into_iter().collect(),
        },
        args.force,
    )
    .map_err(|err| err.to_string())?;
    write_json(None, &report, args.pretty, true)
}

fn run_state_render(args: CliArgs) -> Result<(), String> {
    let manifest = required_path(args.manifest.clone(), "--manifest")?;
    let output = required_path(args.output.clone(), "--output")?;
    let report = render_scene_asset_state(
        &manifest,
        &output,
        SceneAssetStateRenderOptions {
            states: args.states.into_iter().collect(),
        },
        args.force,
    )
    .map_err(|err| err.to_string())?;
    write_json(None, &report, args.pretty, true)
}

fn run_state_sheet(args: CliArgs) -> Result<(), String> {
    let manifest = required_path(args.manifest.clone(), "--manifest")?;
    let frames = required_path(args.frames_json.clone(), "--frames")?;
    let output = required_path(args.output.clone(), "--output")?;
    let index = required_path(args.index.clone(), "--index")?;
    let report = render_scene_asset_state_sheet(&manifest, &frames, &output, &index, args.force)
        .map_err(|err| err.to_string())?;
    write_json(None, &report, args.pretty, true)
}

fn run_remove_background(args: CliArgs) -> Result<(), String> {
    let source = required_path(args.source, "--source")?;
    let output = required_path(args.output, "--output")?;
    let report = make_scene_asset_background_transparent(
        &source,
        &output,
        args.tolerance.unwrap_or(24),
        args.feather.unwrap_or(0),
        args.sample.unwrap_or(SceneAssetBackgroundSample::Corners),
        args.force,
    )
    .map_err(|err| err.to_string())?;
    write_json(None, &report, args.pretty, true)
}

fn run_remove_background_polished(args: CliArgs) -> Result<(), String> {
    let source = required_path(args.source.clone(), "--source")?;
    let output = required_path(args.output.clone(), "--output")?;
    let feature_map = load_optional_protect_map(&args)?;
    let options = mask_polish_options(&args);
    let report = make_scene_asset_background_transparent_polished(
        &source,
        &output,
        options,
        feature_map.as_ref(),
        args.force,
    )
    .map_err(|err| err.to_string())?;
    write_json(None, &report, args.pretty, true)
}

fn run_color_range_erase(args: CliArgs) -> Result<(), String> {
    let source = required_path(args.source.clone(), "--source")?;
    let output = required_path(args.output.clone(), "--output")?;
    let feature_map = load_optional_protect_map(&args)?;
    let report = color_range_erase_scene_asset_image(
        &source,
        &output,
        mask_polish_options(&args),
        feature_map.as_ref(),
        args.force,
    )
    .map_err(|err| err.to_string())?;
    write_json(None, &report, args.pretty, true)
}

fn run_magic_erase(args: CliArgs) -> Result<(), String> {
    let source = required_path(args.source, "--source")?;
    let output = required_path(args.output, "--output")?;
    let seed = SceneAssetNormalizedPoint {
        x: args
            .seed_x
            .ok_or_else(|| "--seed-x is required".to_string())?,
        y: args
            .seed_y
            .ok_or_else(|| "--seed-y is required".to_string())?,
    };
    let report = magic_erase_scene_asset_image(
        &source,
        &output,
        seed,
        args.tolerance.unwrap_or(24),
        !args.global,
        args.feather.unwrap_or(0),
        args.force,
    )
    .map_err(|err| err.to_string())?;
    write_json(None, &report, args.pretty, true)
}

fn run_magic_erase_add(args: CliArgs) -> Result<(), String> {
    let source = required_path(args.source.clone(), "--source")?;
    let output = required_path(args.output.clone(), "--output")?;
    if args.seeds.is_empty() {
        return Err("--seed is required at least once".to_string());
    }
    let feature_map = load_optional_protect_map(&args)?;
    let report = magic_erase_add_scene_asset_image(
        &source,
        &output,
        &args.seeds,
        mask_polish_options(&args),
        feature_map.as_ref(),
        args.force,
    )
    .map_err(|err| err.to_string())?;
    write_json(None, &report, args.pretty, true)
}

fn run_channel_matte_erase(args: CliArgs) -> Result<(), String> {
    let source = required_path(args.source.clone(), "--source")?;
    let output = required_path(args.output.clone(), "--output")?;
    let feature_map = load_optional_protect_map(&args)?;
    let report = channel_matte_erase_scene_asset_image(
        &source,
        &output,
        args.threshold.unwrap_or(238),
        args.neutrality.unwrap_or(28),
        mask_polish_options(&args),
        feature_map.as_ref(),
        args.force,
    )
    .map_err(|err| err.to_string())?;
    write_json(None, &report, args.pretty, true)
}

fn run_mask_preview(args: CliArgs) -> Result<(), String> {
    let source = required_path(args.source.clone(), "--source")?;
    let output = required_path(args.output.clone(), "--output")?;
    let feature_map = load_optional_protect_map(&args)?;
    let report = preview_scene_asset_selection_mask(
        &source,
        &output,
        SceneAssetMaskPreviewOptions {
            mode: args
                .selection_mode
                .ok_or_else(|| "--selection-mode is required".to_string())?,
            seeds: args.seeds.clone(),
            threshold: args.threshold.unwrap_or(238),
            neutrality: args.neutrality.unwrap_or(28),
            polish: mask_polish_options(&args),
        },
        feature_map.as_ref(),
        args.force,
    )
    .map_err(|err| err.to_string())?;
    write_json(None, &report, args.pretty, true)
}

fn run_hair_cleanup(args: CliArgs) -> Result<(), String> {
    let source = required_path(args.source.clone(), "--source")?;
    let output = required_path(args.output.clone(), "--output")?;
    let feature_map = load_optional_protect_map(&args)?;
    let report = cleanup_scene_asset_hair_edges(
        &source,
        &output,
        args.mode
            .unwrap_or(SceneAssetHairCleanupMode::Decontaminate),
        args.radius.unwrap_or(4),
        args.strength.unwrap_or(0.85),
        feature_map.as_ref(),
        args.hair_region.as_deref(),
        args.force,
    )
    .map_err(|err| err.to_string())?;
    write_json(None, &report, args.pretty, true)
}

fn run_restore_from_source(args: CliArgs) -> Result<(), String> {
    let base = required_path(args.base.clone(), "--base")?;
    let cutout = required_path(args.cutout.clone(), "--cutout")?;
    let output = required_path(args.output.clone(), "--output")?;
    let feature_map = load_optional_protect_map(&args)?;
    let report = restore_scene_asset_from_source(
        &base,
        &cutout,
        &output,
        SceneAssetRestoreOptions {
            regions: csv_values(args.restore_regions.as_deref()),
            polygons: args.polygons.clone(),
            filter: args.restore_filter.unwrap_or(SceneAssetRestoreFilter::All),
            tolerance: args.tolerance.unwrap_or(24),
            sample: args.sample.unwrap_or(SceneAssetBackgroundSample::Corners),
        },
        feature_map.as_ref(),
        args.force,
    )
    .map_err(|err| err.to_string())?;
    write_json(None, &report, args.pretty, true)
}

fn load_optional_protect_map(
    args: &CliArgs,
) -> Result<Option<gameterm_visual::SceneAssetFeatureMap>, String> {
    let protect_map_path = args.protect.as_ref().or(args.feature_map.as_ref());
    protect_map_path
        .map(|path| load_scene_asset_feature_map(path))
        .transpose()
        .map_err(|err| err.to_string())
}

fn mask_polish_options(args: &CliArgs) -> SceneAssetMaskPolishOptions {
    SceneAssetMaskPolishOptions {
        tolerance: args.tolerance.unwrap_or(24),
        feather: args.feather.unwrap_or(0),
        sample: args.sample.unwrap_or(SceneAssetBackgroundSample::Corners),
        erode: args.erode.unwrap_or(0),
        dilate: args.dilate.unwrap_or(0),
        open: args.open.unwrap_or(0),
        close: args.close.unwrap_or(0),
        remove_small: args.remove_small.unwrap_or(0),
        fill_holes: args.fill_holes.unwrap_or(0),
        defringe: args.defringe.unwrap_or(SceneAssetDefringeMode::None),
        protect_regions: csv_values(args.protect_regions.as_deref()),
        within_regions: csv_values(args.within_regions.as_deref()),
        within_polygons: args.within_polygons.clone(),
    }
}

fn run_continuity(args: CliArgs) -> Result<(), String> {
    if args.frames.is_empty() {
        return Err("continuity requires at least one frame path".to_string());
    }
    let report =
        continuity_report_for_scene_asset_frames(&args.frames, 8).map_err(|err| err.to_string())?;
    write_json(args.output.as_deref(), &report, args.pretty, args.force)
}

fn run_export_source(args: CliArgs) -> Result<(), String> {
    let source = required_path(args.source, "--source")?;
    let output_source_root = required_path(args.output_source_root, "--output-source-root")?;
    let source_id = args
        .source_id
        .unwrap_or_else(|| "4cher_set4_vn_sprites".to_string());
    let character = args.character.unwrap_or_else(|| "kiki".to_string());
    let expressions = args
        .expressions
        .ok_or_else(|| "--expressions is required".to_string())?
        .split(',')
        .map(str::trim)
        .filter(|expression| !expression.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let report = export_scene_asset_source_images(
        &source,
        &output_source_root,
        &source_id,
        &character,
        &expressions,
        args.force,
    )
    .map_err(|err| err.to_string())?;
    write_json(args.output.as_deref(), &report, args.pretty, args.force)
}

fn parse_args() -> Result<CliArgs, String> {
    let mut parsed = CliArgs::default();
    let mut args = std::env::args_os().skip(1);
    if let Some(command) = args.next() {
        parsed.command = Some(command.to_string_lossy().to_string());
    }
    while let Some(arg) = args.next() {
        let arg_text = arg.to_string_lossy();
        match arg_text.as_ref() {
            "--image" => parsed.image = Some(next_path(&mut args, "--image")?),
            "--base" => parsed.base = Some(next_path(&mut args, "--base")?),
            "--feature-map" => parsed.feature_map = Some(next_path(&mut args, "--feature-map")?),
            "--recipe" => parsed.recipe = Some(next_path(&mut args, "--recipe")?),
            "--expression" => parsed.expression = Some(next_text(&mut args, "--expression")?),
            "--animation" => parsed.animation = Some(next_text(&mut args, "--animation")?),
            "--output" | "-o" => parsed.output = Some(next_path(&mut args, "--output")?),
            "--cutout" => parsed.cutout = Some(next_path(&mut args, "--cutout")?),
            "--output-dir" => parsed.output_dir = Some(next_path(&mut args, "--output-dir")?),
            "--output-source-root" => {
                parsed.output_source_root = Some(next_path(&mut args, "--output-source-root")?)
            }
            "--source" => parsed.source = Some(next_path(&mut args, "--source")?),
            "--pipeline" => parsed.pipeline = Some(next_path(&mut args, "--pipeline")?),
            "--operation" => parsed.operation = Some(next_path(&mut args, "--operation")?),
            "--before" => parsed.before = Some(next_path(&mut args, "--before")?),
            "--after" => parsed.after = Some(next_path(&mut args, "--after")?),
            "--manifest" => parsed.manifest = Some(next_path(&mut args, "--manifest")?),
            "--frames" => parsed.frames_json = Some(next_path(&mut args, "--frames")?),
            "--index" => parsed.index = Some(next_path(&mut args, "--index")?),
            "--input-root" => parsed.input_root = Some(next_path(&mut args, "--input-root")?),
            "--transformation-root" => {
                parsed.transformation_root = Some(next_path(&mut args, "--transformation-root")?)
            }
            "--output-root" => parsed.output_root = Some(next_path(&mut args, "--output-root")?),
            "--height" => parsed.height = Some(next_parse(&mut args, "--height")?),
            "--source-id" => parsed.source_id = Some(next_text(&mut args, "--source-id")?),
            "--protect" => parsed.protect = Some(next_path(&mut args, "--protect")?),
            "--protect-regions" => {
                parsed.protect_regions = Some(next_text(&mut args, "--protect-regions")?)
            }
            "--within-regions" => {
                parsed.within_regions = Some(next_text(&mut args, "--within-regions")?)
            }
            "--restore-regions" => {
                parsed.restore_regions = Some(next_text(&mut args, "--restore-regions")?)
            }
            "--restore-filter" => {
                parsed.restore_filter = Some(parse_restore_filter(&next_text(
                    &mut args,
                    "--restore-filter",
                )?)?)
            }
            "--character" => parsed.character = Some(next_text(&mut args, "--character")?),
            "--expressions" => parsed.expressions = Some(next_text(&mut args, "--expressions")?),
            "--tolerance" => parsed.tolerance = Some(next_parse(&mut args, "--tolerance")?),
            "--feather" => parsed.feather = Some(next_parse(&mut args, "--feather")?),
            "--erode" => parsed.erode = Some(next_parse(&mut args, "--erode")?),
            "--dilate" => parsed.dilate = Some(next_parse(&mut args, "--dilate")?),
            "--open" => parsed.open = Some(next_parse(&mut args, "--open")?),
            "--close" => parsed.close = Some(next_parse(&mut args, "--close")?),
            "--remove-small" => {
                parsed.remove_small = Some(next_parse(&mut args, "--remove-small")?)
            }
            "--fill-holes" => parsed.fill_holes = Some(next_parse(&mut args, "--fill-holes")?),
            "--defringe" => {
                parsed.defringe = Some(parse_defringe(&next_text(&mut args, "--defringe")?)?)
            }
            "--threshold" => parsed.threshold = Some(next_parse(&mut args, "--threshold")?),
            "--neutrality" => parsed.neutrality = Some(next_parse(&mut args, "--neutrality")?),
            "--radius" => parsed.radius = Some(next_parse(&mut args, "--radius")?),
            "--strength" => parsed.strength = Some(next_parse(&mut args, "--strength")?),
            "--mode" => parsed.mode = Some(parse_hair_mode(&next_text(&mut args, "--mode")?)?),
            "--selection-mode" => {
                parsed.selection_mode = Some(parse_selection_mode(&next_text(
                    &mut args,
                    "--selection-mode",
                )?)?)
            }
            "--shape" => {
                parsed.draw_shape = Some(parse_draw_shape(&next_text(&mut args, "--shape")?)?)
            }
            "--anchor" => parsed.anchor = Some(parse_anchor(&next_text(&mut args, "--anchor")?)?),
            "--resample" => {
                parsed.resample = Some(parse_resample(&next_text(&mut args, "--resample")?)?)
            }
            "--channel" => {
                parsed.channel = Some(parse_channel(&next_text(&mut args, "--channel")?)?)
            }
            "--layer" => parsed
                .layers
                .push(parse_layer(&next_text(&mut args, "--layer")?)?),
            "--part" => parsed
                .parts
                .push(parse_part(&next_text(&mut args, "--part")?)?),
            "--state" => parsed
                .states
                .push(parse_state(&next_text(&mut args, "--state")?)?),
            "--hair-region" => parsed.hair_region = Some(next_text(&mut args, "--hair-region")?),
            "--seed-x" => parsed.seed_x = Some(next_parse(&mut args, "--seed-x")?),
            "--seed-y" => parsed.seed_y = Some(next_parse(&mut args, "--seed-y")?),
            "--seed" => parsed
                .seeds
                .push(parse_seed(&next_text(&mut args, "--seed")?)?),
            "--point" => parsed.points.push(parse_point_value(
                &next_text(&mut args, "--point")?,
                "--point",
            )?),
            "--sample-point" => {
                parsed.sample_point = Some(parse_point_value(
                    &next_text(&mut args, "--sample-point")?,
                    "--sample-point",
                )?)
            }
            "--sample-origin" => {
                parsed.sample_origin = Some(parse_point_value(
                    &next_text(&mut args, "--sample-origin")?,
                    "--sample-origin",
                )?)
            }
            "--target-origin" => {
                parsed.target_origin = Some(parse_point_value(
                    &next_text(&mut args, "--target-origin")?,
                    "--target-origin",
                )?)
            }
            "--polygon" => parsed
                .polygons
                .push(parse_polygon(&next_text(&mut args, "--polygon")?)?),
            "--within-polygon" => parsed
                .within_polygons
                .push(parse_polygon(&next_text(&mut args, "--within-polygon")?)?),
            "--path" => parsed.path = parse_path(&next_text(&mut args, "--path")?)?,
            "--rect" => parsed.rect = Some(parse_rect(&next_text(&mut args, "--rect")?)?),
            "--color" => parsed.color = Some(parse_color(&next_text(&mut args, "--color")?)?),
            "--alpha" => parsed.alpha = Some(next_parse(&mut args, "--alpha")?),
            "--stroke" => parsed.stroke = Some(next_parse(&mut args, "--stroke")?),
            "--width" if parsed.command.as_deref() == Some("stroke-path") => {
                parsed.stroke = Some(next_parse(&mut args, "--width")?)
            }
            "--width" => parsed.width = Some(next_parse(&mut args, "--width")?),
            "--sample-radius" => {
                parsed.sample_radius = Some(next_parse(&mut args, "--sample-radius")?)
            }
            "--step" => parsed.step = Some(next_parse(&mut args, "--step")?),
            "--scale" => parsed.scale = Some(next_parse(&mut args, "--scale")?),
            "--black" => parsed.black = Some(next_parse(&mut args, "--black")?),
            "--white" => parsed.white = Some(next_parse(&mut args, "--white")?),
            "--gamma" => parsed.gamma = Some(next_parse(&mut args, "--gamma")?),
            "--brightness" => parsed.brightness = Some(next_parse(&mut args, "--brightness")?),
            "--contrast" => parsed.contrast = Some(next_parse(&mut args, "--contrast")?),
            "--hue" => parsed.hue = Some(next_parse(&mut args, "--hue")?),
            "--saturation" => parsed.saturation = Some(next_parse(&mut args, "--saturation")?),
            "--lightness" => parsed.lightness = Some(next_parse(&mut args, "--lightness")?),
            "--amount" => parsed.amount = Some(next_parse(&mut args, "--amount")?),
            "--translate" => {
                let (x, y) = parse_i32_pair(&next_text(&mut args, "--translate")?)?;
                parsed.translate_x = x;
                parsed.translate_y = y;
            }
            "--sample" => parsed.sample = Some(parse_sample(&next_text(&mut args, "--sample")?)?),
            "--whole-image" => parsed.whole_image = true,
            "--global" => parsed.global = true,
            "--fill" => parsed.fill = true,
            "--closed" => parsed.closed = true,
            "--content-bounds" => parsed.content_bounds = true,
            "--flip-x" => parsed.flip_x = true,
            "--flip-y" => parsed.flip_y = true,
            "--pretty" => parsed.pretty = true,
            "--force" => parsed.force = true,
            "--dry-run" => parsed.dry_run = true,
            "-h" | "--help" => {
                usage();
                std::process::exit(0);
            }
            _ if arg_text.starts_with('-') => return Err(format!("unknown option: {arg_text}")),
            _ => {
                if matches!(
                    parsed.command.as_deref(),
                    Some("inspect") | Some("map-template")
                ) && parsed.image.is_none()
                {
                    parsed.image = Some(PathBuf::from(arg));
                } else {
                    parsed.frames.push(PathBuf::from(arg));
                }
            }
        }
    }
    Ok(parsed)
}

fn next_path(
    args: &mut impl Iterator<Item = std::ffi::OsString>,
    flag: &str,
) -> Result<PathBuf, String> {
    args.next()
        .map(PathBuf::from)
        .ok_or_else(|| format!("{flag} requires a path"))
}

fn next_text(
    args: &mut impl Iterator<Item = std::ffi::OsString>,
    flag: &str,
) -> Result<String, String> {
    args.next()
        .map(|value| value.to_string_lossy().to_string())
        .ok_or_else(|| format!("{flag} requires a value"))
}

fn next_parse<T>(
    args: &mut impl Iterator<Item = std::ffi::OsString>,
    flag: &str,
) -> Result<T, String>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let value = next_text(args, flag)?;
    value
        .parse()
        .map_err(|err| format!("{flag} value `{value}` is invalid: {err}"))
}

fn parse_sample(value: &str) -> Result<SceneAssetBackgroundSample, String> {
    match value {
        "corners" => Ok(SceneAssetBackgroundSample::Corners),
        "edges" => Ok(SceneAssetBackgroundSample::Edges),
        _ => Err(format!(
            "--sample value `{value}` is invalid; expected corners or edges"
        )),
    }
}

fn parse_defringe(value: &str) -> Result<SceneAssetDefringeMode, String> {
    match value {
        "none" => Ok(SceneAssetDefringeMode::None),
        "white" => Ok(SceneAssetDefringeMode::White),
        _ => Err(format!(
            "--defringe value `{value}` is invalid; expected none or white"
        )),
    }
}

fn parse_hair_mode(value: &str) -> Result<SceneAssetHairCleanupMode, String> {
    match value {
        "decontaminate" => Ok(SceneAssetHairCleanupMode::Decontaminate),
        _ => Err(format!(
            "--mode value `{value}` is invalid; expected decontaminate"
        )),
    }
}

fn parse_restore_filter(value: &str) -> Result<SceneAssetRestoreFilter, String> {
    match value {
        "all" => Ok(SceneAssetRestoreFilter::All),
        "non-background" => Ok(SceneAssetRestoreFilter::NonBackground),
        _ => Err(format!(
            "--restore-filter value `{value}` is invalid; expected all or non-background"
        )),
    }
}

fn parse_selection_mode(value: &str) -> Result<SceneAssetMaskPreviewMode, String> {
    match value {
        "background" => Ok(SceneAssetMaskPreviewMode::Background),
        "color-range" => Ok(SceneAssetMaskPreviewMode::ColorRange),
        "magic-add" => Ok(SceneAssetMaskPreviewMode::MagicAdd),
        "channel-matte" => Ok(SceneAssetMaskPreviewMode::ChannelMatte),
        _ => Err(format!(
            "--selection-mode value `{value}` is invalid; expected background, color-range, magic-add, or channel-matte"
        )),
    }
}

fn parse_draw_shape(value: &str) -> Result<SceneAssetDrawShapeKind, String> {
    match value {
        "rect" => Ok(SceneAssetDrawShapeKind::Rect),
        "line" => Ok(SceneAssetDrawShapeKind::Line),
        "polygon" => Ok(SceneAssetDrawShapeKind::Polygon),
        "ellipse" | "circle" => Ok(SceneAssetDrawShapeKind::Ellipse),
        _ => Err(format!(
            "--shape value `{value}` is invalid; expected rect, line, polygon, or ellipse"
        )),
    }
}

fn parse_anchor(value: &str) -> Result<SceneAssetPadAnchor, String> {
    match value {
        "center" => Ok(SceneAssetPadAnchor::Center),
        "bottom-center" => Ok(SceneAssetPadAnchor::BottomCenter),
        "top-left" => Ok(SceneAssetPadAnchor::TopLeft),
        _ => Err(format!(
            "--anchor value `{value}` is invalid; expected center, bottom-center, or top-left"
        )),
    }
}

fn parse_resample(value: &str) -> Result<SceneAssetResampleFilter, String> {
    match value {
        "nearest" => Ok(SceneAssetResampleFilter::Nearest),
        "lanczos3" => Ok(SceneAssetResampleFilter::Lanczos3),
        _ => Err(format!(
            "--resample value `{value}` is invalid; expected nearest or lanczos3"
        )),
    }
}

fn parse_channel(value: &str) -> Result<SceneAssetColorChannel, String> {
    match value {
        "rgb" => Ok(SceneAssetColorChannel::Rgb),
        "r" => Ok(SceneAssetColorChannel::R),
        "g" => Ok(SceneAssetColorChannel::G),
        "b" => Ok(SceneAssetColorChannel::B),
        "a" => Ok(SceneAssetColorChannel::A),
        _ => Err(format!(
            "--channel value `{value}` is invalid; expected rgb, r, g, b, or a"
        )),
    }
}

fn parse_blend(value: &str) -> Result<SceneAssetBlendMode, String> {
    match value {
        "normal" => Ok(SceneAssetBlendMode::Normal),
        "add" => Ok(SceneAssetBlendMode::Add),
        "multiply" => Ok(SceneAssetBlendMode::Multiply),
        "screen" => Ok(SceneAssetBlendMode::Screen),
        _ => Err(format!(
            "blend value `{value}` is invalid; expected normal, add, multiply, or screen"
        )),
    }
}

fn parse_layer(value: &str) -> Result<SceneAssetCompositeLayer, String> {
    let parts = value.split(',').map(str::trim).collect::<Vec<_>>();
    if parts.len() != 5 {
        return Err(format!(
            "--layer value `{value}` is invalid; expected path,blend,opacity,x,y"
        ));
    }
    Ok(SceneAssetCompositeLayer {
        path: parts[0].to_string(),
        blend: parse_blend(parts[1])?,
        opacity: parts[2]
            .parse()
            .map_err(|err| format!("--layer opacity `{}` is invalid: {err}", parts[2]))?,
        x_offset: parts[3]
            .parse()
            .map_err(|err| format!("--layer x `{}` is invalid: {err}", parts[3]))?,
        y_offset: parts[4]
            .parse()
            .map_err(|err| format!("--layer y `{}` is invalid: {err}", parts[4]))?,
    })
}

fn parse_part(value: &str) -> Result<(String, Vec<String>), String> {
    let (name, files) = value
        .split_once('=')
        .ok_or_else(|| format!("--part value `{value}` is invalid; expected name=file,file2"))?;
    let files = files
        .split(',')
        .map(str::trim)
        .filter(|file| !file.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if name.trim().is_empty() || files.is_empty() {
        return Err("--part requires a name and at least one file".to_string());
    }
    Ok((name.trim().to_string(), files))
}

fn parse_state(value: &str) -> Result<(String, String), String> {
    let (part, state) = value
        .split_once('=')
        .ok_or_else(|| format!("--state value `{value}` is invalid; expected part=state"))?;
    if part.trim().is_empty() || state.trim().is_empty() {
        return Err("--state requires non-empty part and state".to_string());
    }
    Ok((part.trim().to_string(), state.trim().to_string()))
}

fn parse_seed(value: &str) -> Result<SceneAssetNormalizedPoint, String> {
    parse_point_value(value, "--seed")
}

fn parse_point_value(value: &str, label: &str) -> Result<SceneAssetNormalizedPoint, String> {
    let (x, y) = value
        .split_once(',')
        .ok_or_else(|| format!("{label} value `{value}` is invalid; expected X,Y"))?;
    let x = x
        .trim()
        .parse::<f32>()
        .map_err(|err| format!("{label} x value `{x}` is invalid: {err}"))?;
    let y = y
        .trim()
        .parse::<f32>()
        .map_err(|err| format!("{label} y value `{y}` is invalid: {err}"))?;
    Ok(SceneAssetNormalizedPoint { x, y })
}

fn parse_color(value: &str) -> Result<[u8; 4], String> {
    let hex = value.trim().strip_prefix('#').unwrap_or(value.trim());
    if hex.len() != 6 && hex.len() != 8 {
        return Err(format!(
            "--color value `{value}` is invalid; expected #RRGGBB or #RRGGBBAA"
        ));
    }
    let parse_channel = |start: usize| {
        u8::from_str_radix(&hex[start..start + 2], 16)
            .map_err(|err| format!("--color value `{value}` is invalid: {err}"))
    };
    Ok([
        parse_channel(0)?,
        parse_channel(2)?,
        parse_channel(4)?,
        if hex.len() == 8 {
            parse_channel(6)?
        } else {
            255
        },
    ])
}

fn parse_polygon(value: &str) -> Result<Vec<SceneAssetNormalizedPoint>, String> {
    let points = value
        .split(';')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(parse_seed)
        .collect::<Result<Vec<_>, _>>()?;
    if points.len() < 3 {
        return Err("--polygon requires at least three X,Y points".to_string());
    }
    Ok(points)
}

fn parse_path(value: &str) -> Result<Vec<SceneAssetNormalizedPoint>, String> {
    let points = value
        .split(';')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|point| parse_point_value(point, "--path"))
        .collect::<Result<Vec<_>, _>>()?;
    if points.len() < 2 {
        return Err("--path requires at least two X,Y points".to_string());
    }
    Ok(points)
}

fn parse_rect(value: &str) -> Result<SceneAssetNormalizedRect, String> {
    let parts = value.split(',').map(str::trim).collect::<Vec<_>>();
    if parts.len() != 4 {
        return Err(format!(
            "--rect value `{value}` is invalid; expected X,Y,W,H"
        ));
    }
    let parse = |index: usize| {
        parts[index]
            .parse::<f32>()
            .map_err(|err| format!("--rect value `{}` is invalid: {err}", parts[index]))
    };
    Ok(SceneAssetNormalizedRect {
        x: parse(0)?,
        y: parse(1)?,
        w: parse(2)?,
        h: parse(3)?,
    })
}

fn parse_i32_pair(value: &str) -> Result<(i32, i32), String> {
    let (x, y) = value
        .split_once(',')
        .ok_or_else(|| format!("--translate value `{value}` is invalid; expected X,Y"))?;
    let x = x
        .trim()
        .parse::<i32>()
        .map_err(|err| format!("--translate x value `{x}` is invalid: {err}"))?;
    let y = y
        .trim()
        .parse::<i32>()
        .map_err(|err| format!("--translate y value `{y}` is invalid: {err}"))?;
    Ok((x, y))
}

fn csv_values(value: Option<&str>) -> Vec<String> {
    value
        .unwrap_or("")
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn required_path(value: Option<PathBuf>, label: &str) -> Result<PathBuf, String> {
    value.ok_or_else(|| format!("{label} is required"))
}

fn write_json(
    output: Option<&Path>,
    value: &impl Serialize,
    pretty: bool,
    force: bool,
) -> Result<(), String> {
    let json = if pretty {
        serde_json::to_string_pretty(value)
    } else {
        serde_json::to_string(value)
    }
    .map_err(|err| err.to_string())?;
    if let Some(output) = output {
        if output.exists() && !force {
            return Err(format!(
                "refusing to overwrite existing file without --force: {}",
                output.display()
            ));
        }
        if let Some(parent) = output.parent() {
            std::fs::create_dir_all(parent).map_err(|err| err.to_string())?;
        }
        std::fs::write(output, format!("{json}\n")).map_err(|err| err.to_string())?;
    } else {
        println!("{json}");
    }
    Ok(())
}

fn display_path(path: PathBuf) -> String {
    path.display().to_string()
}
