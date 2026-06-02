use gameterm_visual::{run_vn_asset_intake, VnAssetIntakeOptions};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Default)]
struct CliArgs {
    catalog: Option<PathBuf>,
    source_root: Option<PathBuf>,
    output_root: Option<PathBuf>,
    sprite_manifest: Option<PathBuf>,
    attribution: Option<PathBuf>,
    bindings: Option<PathBuf>,
    base_manifest: Option<PathBuf>,
    force: bool,
}

fn usage() {
    eprintln!(
        "Usage: cargo run -p gameterm-visual --example scene_vn_asset_intake -- \\
  --catalog PATH \\
  --source-root PATH \\
  --output-root PATH \\
  --sprite-manifest PATH \\
  --attribution PATH \\
  --bindings PATH \\
  [--base-manifest PATH] \\
  [--force]"
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

    let catalog = required(args.catalog, "--catalog");
    let source_root = required(args.source_root, "--source-root");
    let output_root = required(args.output_root, "--output-root");
    let sprite_manifest = required(args.sprite_manifest, "--sprite-manifest");
    let attribution = required(args.attribution, "--attribution");
    let bindings = required(args.bindings, "--bindings");

    let report = match run_vn_asset_intake(VnAssetIntakeOptions {
        catalog_path: catalog,
        source_root,
        output_root,
        sprite_manifest_path: Some(sprite_manifest.clone()),
        base_manifest_path: args.base_manifest,
        force: args.force,
    }) {
        Ok(report) => report,
        Err(err) => {
            eprintln!("Scene VN asset intake failed: {err}");
            std::process::exit(1);
        }
    };

    write_json(&sprite_manifest, &report.sprite_manifest);
    write_json(&attribution, &report.attribution);
    write_json(&bindings, &report.bindings);

    for warning in &report.warnings {
        match &warning.source_id {
            Some(source_id) => eprintln!("WARN: {source_id}: {}", warning.detail),
            None => eprintln!("WARN: {}", warning.detail),
        }
    }
    println!(
        "Wrote Scene VN asset manifest: {}",
        sprite_manifest.display()
    );
    println!(
        "Wrote Scene VN asset attribution: {}",
        attribution.display()
    );
    println!("Wrote Scene VN asset bindings: {}", bindings.display());
}

fn parse_args() -> Result<CliArgs, String> {
    let mut parsed = CliArgs::default();
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        let arg = arg.to_string_lossy();
        match arg.as_ref() {
            "--catalog" => parsed.catalog = Some(next_path(&mut args, "--catalog")?),
            "--source-root" => parsed.source_root = Some(next_path(&mut args, "--source-root")?),
            "--output-root" => parsed.output_root = Some(next_path(&mut args, "--output-root")?),
            "--sprite-manifest" => {
                parsed.sprite_manifest = Some(next_path(&mut args, "--sprite-manifest")?)
            }
            "--attribution" => parsed.attribution = Some(next_path(&mut args, "--attribution")?),
            "--bindings" => parsed.bindings = Some(next_path(&mut args, "--bindings")?),
            "--base-manifest" => {
                parsed.base_manifest = Some(next_path(&mut args, "--base-manifest")?)
            }
            "--force" => parsed.force = true,
            "-h" | "--help" => {
                usage();
                std::process::exit(0);
            }
            _ => return Err(format!("unknown option: {arg}")),
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

fn required(value: Option<PathBuf>, flag: &str) -> PathBuf {
    match value {
        Some(value) => value,
        None => {
            eprintln!("{flag} is required");
            usage();
            std::process::exit(2);
        }
    }
}

fn write_json(path: &PathBuf, value: &impl Serialize) {
    if let Some(parent) = path.parent() {
        if let Err(err) = std::fs::create_dir_all(parent) {
            eprintln!("failed to create {}: {err}", parent.display());
            std::process::exit(1);
        }
    }
    let json = match serde_json::to_string_pretty(value) {
        Ok(json) => json,
        Err(err) => {
            eprintln!("failed to serialize {}: {err}", path.display());
            std::process::exit(1);
        }
    };
    if let Err(err) = std::fs::write(path, format!("{json}\n")) {
        eprintln!("failed to write {}: {err}", path.display());
        std::process::exit(1);
    }
}
