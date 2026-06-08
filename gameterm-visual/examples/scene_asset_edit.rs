use gameterm_visual::{
    continuity_report_for_scene_asset_frames, default_scene_asset_feature_map,
    export_scene_asset_source_images, generate_scene_asset_animation,
    generate_scene_asset_expression, inspect_scene_asset_image, load_scene_asset_feature_map,
    load_scene_asset_recipe_book, magic_erase_scene_asset_image,
    make_scene_asset_background_transparent, validate_scene_asset_feature_map,
    write_scene_asset_json, SceneAssetBackgroundSample, SceneAssetNormalizedPoint,
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
    output_dir: Option<PathBuf>,
    output_source_root: Option<PathBuf>,
    source: Option<PathBuf>,
    source_id: Option<String>,
    character: Option<String>,
    expressions: Option<String>,
    tolerance: Option<u8>,
    feather: Option<u32>,
    seed_x: Option<f32>,
    seed_y: Option<f32>,
    sample: Option<SceneAssetBackgroundSample>,
    global: bool,
    frames: Vec<PathBuf>,
    pretty: bool,
    force: bool,
}

fn usage() {
    eprintln!(
        "Usage:
  cargo run -p gameterm-visual --example scene_asset_edit -- inspect IMAGE [--output PATH] [--pretty]
  cargo run -p gameterm-visual --example scene_asset_edit -- map-template IMAGE --character NAME --output PATH [--base TEXT] [--force]
  cargo run -p gameterm-visual --example scene_asset_edit -- validate-map --image IMAGE --feature-map PATH
  cargo run -p gameterm-visual --example scene_asset_edit -- expression --base IMAGE --feature-map PATH --recipe PATH --expression NAME --output PATH [--force]
  cargo run -p gameterm-visual --example scene_asset_edit -- animation --base IMAGE --feature-map PATH --recipe PATH --animation NAME --output-dir DIR [--character NAME] [--force]
  cargo run -p gameterm-visual --example scene_asset_edit -- remove-background --source IMAGE --output PATH [--tolerance N] [--feather N] [--sample corners|edges] [--force]
  cargo run -p gameterm-visual --example scene_asset_edit -- magic-erase --source IMAGE --output PATH --seed-x N --seed-y N [--tolerance N] [--feather N] [--global] [--force]
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
  --output-dir PATH          Output directory for generated animation frames.
  --output-source-root PATH  Source-root layout used by scene_vn_asset_intake.
  --source PATH              Source image for export-source.
  --source-id ID             Catalog source directory.
  --character NAME           Character id. Default: kiki.
  --expressions CSV          Expression names for export-source.
  --tolerance N              RGB channel tolerance for magic selection. Default: 24.
  --feather N                Pixel feather radius after selection. Default: 0.
  --seed-x N                 Normalized magic-erase seed x, 0..1.
  --seed-y N                 Normalized magic-erase seed y, 0..1.
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
        Some("map-template") => run_map_template(args),
        Some("validate-map") => run_validate_map(args),
        Some("expression") => run_expression(args),
        Some("animation") => run_animation(args),
        Some("remove-background") => run_remove_background(args),
        Some("magic-erase") => run_magic_erase(args),
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
            "--output-dir" => parsed.output_dir = Some(next_path(&mut args, "--output-dir")?),
            "--output-source-root" => {
                parsed.output_source_root = Some(next_path(&mut args, "--output-source-root")?)
            }
            "--source" => parsed.source = Some(next_path(&mut args, "--source")?),
            "--source-id" => parsed.source_id = Some(next_text(&mut args, "--source-id")?),
            "--character" => parsed.character = Some(next_text(&mut args, "--character")?),
            "--expressions" => parsed.expressions = Some(next_text(&mut args, "--expressions")?),
            "--tolerance" => parsed.tolerance = Some(next_parse(&mut args, "--tolerance")?),
            "--feather" => parsed.feather = Some(next_parse(&mut args, "--feather")?),
            "--seed-x" => parsed.seed_x = Some(next_parse(&mut args, "--seed-x")?),
            "--seed-y" => parsed.seed_y = Some(next_parse(&mut args, "--seed-y")?),
            "--sample" => parsed.sample = Some(parse_sample(&next_text(&mut args, "--sample")?)?),
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
