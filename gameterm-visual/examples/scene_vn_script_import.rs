use gameterm_visual::{
    import_vn_script_scene, VnAssetBindings, VnScriptDialect, VnScriptImportOptions,
};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug)]
struct CliArgs {
    source: PathBuf,
    output: PathBuf,
    attribution: PathBuf,
    source_dialect: VnScriptDialect,
    source_title: String,
    source_version: Option<String>,
    asset_root: Option<PathBuf>,
    bindings: Option<PathBuf>,
    title: String,
}

fn usage() {
    eprintln!(
        "Usage: cargo run -p gameterm-visual --example scene_vn_script_import -- \\
  --source PATH \\
  --output PATH \\
  --attribution PATH \\
  --source-dialect rpy \\
  --source-title TITLE \\
  [--source-version VERSION] \\
  [--asset-root PATH] \\
  [--bindings PATH] \\
  [--title TITLE]"
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

    let source = match std::fs::read_to_string(&args.source) {
        Ok(source) => source,
        Err(err) => {
            eprintln!(
                "VN script source file not found: {}: {err}",
                args.source.display()
            );
            std::process::exit(1);
        }
    };
    let bindings = match args.bindings.as_ref() {
        Some(path) => Some(load_bindings(path)),
        None => None,
    };
    let report = match import_vn_script_scene(
        &source,
        VnScriptImportOptions {
            dialect: args.source_dialect,
            source_path: Some(args.source.clone()),
            source_title: args.source_title,
            source_version: args.source_version,
            asset_root: args.asset_root,
            bindings,
            title: args.title,
        },
    ) {
        Ok(report) => report,
        Err(err) => {
            eprintln!("Scene VN script import failed: {err}");
            std::process::exit(1);
        }
    };

    write_json(&args.output, &report.scene);
    write_json(&args.attribution, &report.attribution);
    for warning in &report.warnings {
        eprintln!("WARN: line {}: {}", warning.line, warning.detail);
    }
    println!(
        "Wrote Scene Mode VN script import: {}",
        args.output.display()
    );
    println!(
        "Wrote Scene Mode VN script attribution: {}",
        args.attribution.display()
    );
}

fn parse_args() -> Result<CliArgs, String> {
    let mut source = None;
    let mut output = None;
    let mut attribution = None;
    let mut source_dialect = None;
    let mut source_title = None;
    let mut source_version = None;
    let mut asset_root = None;
    let mut bindings = None;
    let mut title = "VN Script Demo Import".to_string();

    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        let arg = arg.to_string_lossy();
        match arg.as_ref() {
            "--source" => source = Some(next_path(&mut args, "--source")?),
            "--output" => output = Some(next_path(&mut args, "--output")?),
            "--attribution" => attribution = Some(next_path(&mut args, "--attribution")?),
            "--source-dialect" => {
                source_dialect = Some(parse_dialect(&next_string(&mut args, "--source-dialect")?)?)
            }
            "--source-title" => source_title = Some(next_string(&mut args, "--source-title")?),
            "--source-version" => {
                source_version = Some(next_string(&mut args, "--source-version")?)
            }
            "--asset-root" => asset_root = Some(next_path(&mut args, "--asset-root")?),
            "--bindings" => bindings = Some(next_path(&mut args, "--bindings")?),
            "--title" => title = next_string(&mut args, "--title")?,
            "-h" | "--help" => {
                usage();
                std::process::exit(0);
            }
            _ => return Err(format!("unknown option: {arg}")),
        }
    }

    Ok(CliArgs {
        source: required_path(source, "--source")?,
        output: required_path(output, "--output")?,
        attribution: required_path(attribution, "--attribution")?,
        source_dialect: source_dialect.unwrap_or(VnScriptDialect::Rpy),
        source_title: source_title.unwrap_or_else(|| "VN Script Demo".to_string()),
        source_version,
        asset_root,
        bindings,
        title,
    })
}

fn parse_dialect(value: &str) -> Result<VnScriptDialect, String> {
    match value {
        "rpy" => Ok(VnScriptDialect::Rpy),
        _ => Err(format!(
            "unsupported --source-dialect `{value}`; expected rpy"
        )),
    }
}

fn next_path(
    args: &mut impl Iterator<Item = std::ffi::OsString>,
    flag: &str,
) -> Result<PathBuf, String> {
    args.next()
        .map(PathBuf::from)
        .ok_or_else(|| format!("{flag} requires a path"))
}

fn next_string(
    args: &mut impl Iterator<Item = std::ffi::OsString>,
    flag: &str,
) -> Result<String, String> {
    args.next()
        .map(|value| value.to_string_lossy().to_string())
        .ok_or_else(|| format!("{flag} requires a value"))
}

fn required_path(value: Option<PathBuf>, flag: &str) -> Result<PathBuf, String> {
    value.ok_or_else(|| format!("{flag} is required"))
}

fn load_bindings(path: &PathBuf) -> VnAssetBindings {
    let json = match std::fs::read_to_string(path) {
        Ok(json) => json,
        Err(err) => {
            eprintln!("failed to read bindings {}: {err}", path.display());
            std::process::exit(1);
        }
    };
    match serde_json::from_str(&json) {
        Ok(bindings) => bindings,
        Err(err) => {
            eprintln!("failed to parse bindings {}: {err}", path.display());
            std::process::exit(1);
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
