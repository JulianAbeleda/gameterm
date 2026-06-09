use gameterm_visual::{
    alpha_paint_scene_asset_region, channel_matte_erase_scene_asset_image,
    cleanup_scene_asset_hair_edges, color_range_erase_scene_asset_image,
    continuity_report_for_scene_asset_frames, default_scene_asset_feature_map,
    export_scene_asset_source_images, fill_scene_asset_region, generate_scene_asset_animation,
    generate_scene_asset_expression, inspect_scene_asset_image, load_scene_asset_feature_map,
    load_scene_asset_recipe_book, magic_erase_add_scene_asset_image, magic_erase_scene_asset_image,
    make_scene_asset_background_transparent, make_scene_asset_background_transparent_polished,
    preview_scene_asset_grid, preview_scene_asset_selection_mask, report_scene_asset_points,
    restore_scene_asset_from_source, sample_fill_scene_asset_region, sample_scene_asset_image,
    validate_scene_asset_feature_map, write_scene_asset_json, SceneAssetAlphaPaintOptions,
    SceneAssetBackgroundSample, SceneAssetDefringeMode, SceneAssetFillOptions,
    SceneAssetGridPreviewOptions, SceneAssetHairCleanupMode, SceneAssetMaskPolishOptions,
    SceneAssetMaskPreviewMode, SceneAssetMaskPreviewOptions, SceneAssetNormalizedPoint,
    SceneAssetRestoreFilter, SceneAssetRestoreOptions, SceneAssetSampleFillOptions,
    SceneAssetSampleOptions,
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
    source_id: Option<String>,
    protect: Option<PathBuf>,
    protect_regions: Option<String>,
    within_regions: Option<String>,
    restore_regions: Option<String>,
    restore_filter: Option<SceneAssetRestoreFilter>,
    selection_mode: Option<SceneAssetMaskPreviewMode>,
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
    polygons: Vec<Vec<SceneAssetNormalizedPoint>>,
    within_polygons: Vec<Vec<SceneAssetNormalizedPoint>>,
    sample: Option<SceneAssetBackgroundSample>,
    color: Option<[u8; 4]>,
    alpha: Option<u8>,
    sample_radius: Option<u32>,
    step: Option<f32>,
    whole_image: bool,
    global: bool,
    frames: Vec<PathBuf>,
    pretty: bool,
    force: bool,
}

fn usage() {
    eprintln!(
        "Usage:
  cargo run -p gameterm-visual --example scene_asset_edit -- inspect IMAGE [--output PATH] [--pretty]
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
  --step N                   Normalized grid spacing. Default: 0.1.
  --whole-image              Allow a paint operation to affect the whole image.
  --sample corners|edges     Background samples. Default: corners.
  --global                   Select all matching pixels instead of contiguous seed fill.
  --pretty                   Pretty-print JSON.
  --force                    Overwrite existing files.
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
            "--polygon" => parsed
                .polygons
                .push(parse_polygon(&next_text(&mut args, "--polygon")?)?),
            "--within-polygon" => parsed
                .within_polygons
                .push(parse_polygon(&next_text(&mut args, "--within-polygon")?)?),
            "--color" => parsed.color = Some(parse_color(&next_text(&mut args, "--color")?)?),
            "--alpha" => parsed.alpha = Some(next_parse(&mut args, "--alpha")?),
            "--sample-radius" => {
                parsed.sample_radius = Some(next_parse(&mut args, "--sample-radius")?)
            }
            "--step" => parsed.step = Some(next_parse(&mut args, "--step")?),
            "--sample" => parsed.sample = Some(parse_sample(&next_text(&mut args, "--sample")?)?),
            "--whole-image" => parsed.whole_image = true,
            "--global" => parsed.global = true,
            "--pretty" => parsed.pretty = true,
            "--force" => parsed.force = true,
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
