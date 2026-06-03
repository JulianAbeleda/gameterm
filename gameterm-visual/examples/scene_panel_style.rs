use serde::Serialize;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Default)]
struct CliArgs {
    png: Option<PathBuf>,
    output: Option<PathBuf>,
    pretty: bool,
    trace_svg: Option<PathBuf>,
}

#[derive(Debug, Serialize)]
struct PanelStyleReport {
    schema: &'static str,
    source: String,
    source_kind: &'static str,
    dimensions: PanelDimensions,
    recommended_renderer: &'static str,
    style: PanelStyle,
    trace_tools: TraceTools,
    notes: Vec<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    trace_svg: Option<TraceSvgReport>,
}

#[derive(Debug, Serialize)]
struct PanelDimensions {
    width: u32,
    height: u32,
}

#[derive(Debug, Serialize)]
struct PanelStyle {
    fill: ColorSample,
    border: ColorSample,
    corner_radius_px: u32,
    border_width_px: f32,
    slice_px: u32,
    transparent_pixel_ratio: f32,
    visible_alpha: AlphaStats,
}

#[derive(Debug, Serialize)]
struct ColorSample {
    rgba8: [u8; 4],
    rgba: [f32; 4],
    alpha: f32,
}

#[derive(Debug, Serialize)]
struct AlphaStats {
    min: f32,
    median: f32,
    max: f32,
}

#[derive(Debug, Serialize)]
struct TraceTools {
    vtracer: Option<String>,
    potrace: Option<String>,
    autotrace: Option<String>,
}

#[derive(Debug, Serialize)]
struct TraceSvgReport {
    tool: String,
    path: String,
}

fn usage() {
    eprintln!(
        "Usage: cargo run -p gameterm-visual --example scene_panel_style -- PNG [OPTIONS]

Options:
  --output PATH       Write style JSON to this path.
  --pretty            Pretty-print JSON.
  --trace-svg PATH    Optionally invoke vtracer, potrace, or autotrace to emit SVG.
  -h, --help          Show this help."
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
    let png = required(args.png, "PNG");

    let mut report = match estimate_style(&png) {
        Ok(report) => report,
        Err(err) => {
            eprintln!("Scene panel style extraction failed: {err}");
            std::process::exit(1);
        }
    };

    if let Some(svg_path) = args.trace_svg {
        match maybe_trace_svg(&png, &svg_path) {
            Ok(tool) => {
                report.trace_svg = Some(TraceSvgReport {
                    tool,
                    path: svg_path.display().to_string(),
                });
            }
            Err(err) => {
                eprintln!("Scene panel SVG trace failed: {err}");
                std::process::exit(1);
            }
        }
    }

    let json = if args.pretty {
        serde_json::to_string_pretty(&report)
    } else {
        serde_json::to_string(&report)
    };
    let json = match json {
        Ok(json) => json,
        Err(err) => {
            eprintln!("failed to serialize panel style report: {err}");
            std::process::exit(1);
        }
    };

    if let Some(output) = args.output {
        if let Some(parent) = output.parent() {
            if let Err(err) = std::fs::create_dir_all(parent) {
                eprintln!("failed to create {}: {err}", parent.display());
                std::process::exit(1);
            }
        }
        if let Err(err) = std::fs::write(&output, format!("{json}\n")) {
            eprintln!("failed to write {}: {err}", output.display());
            std::process::exit(1);
        }
    } else {
        println!("{json}");
    }
}

fn parse_args() -> Result<CliArgs, String> {
    let mut parsed = CliArgs::default();
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        let arg_text = arg.to_string_lossy();
        match arg_text.as_ref() {
            "--output" | "-o" => parsed.output = Some(next_path(&mut args, "--output")?),
            "--pretty" => parsed.pretty = true,
            "--trace-svg" => parsed.trace_svg = Some(next_path(&mut args, "--trace-svg")?),
            "-h" | "--help" => {
                usage();
                std::process::exit(0);
            }
            _ if arg_text.starts_with('-') => return Err(format!("unknown option: {arg_text}")),
            _ => {
                if parsed.png.is_some() {
                    return Err(format!("unexpected positional argument: {arg_text}"));
                }
                parsed.png = Some(PathBuf::from(arg));
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

fn required(value: Option<PathBuf>, label: &str) -> PathBuf {
    match value {
        Some(value) => value,
        None => {
            eprintln!("{label} is required");
            usage();
            std::process::exit(2);
        }
    }
}

fn estimate_style(path: &PathBuf) -> anyhow::Result<PanelStyleReport> {
    let image = image::ImageReader::open(path)?.decode()?.into_rgba8();
    let (width, height) = image.dimensions();
    let pixels: Vec<[u8; 4]> = image.pixels().map(|pixel| pixel.0).collect();
    let visible: Vec<[u8; 4]> = pixels
        .iter()
        .copied()
        .filter(|pixel| pixel[3] > 0)
        .collect();
    let transparent_count = pixels.iter().filter(|pixel| pixel[3] == 0).count();

    let edge_band = (width.min(height) / 16).max(1);
    let mut edge = Vec::new();
    let mut center = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let pixel = pixels[(y * width + x) as usize];
            if pixel[3] == 0 {
                continue;
            }
            if x < edge_band || x >= width - edge_band || y < edge_band || y >= height - edge_band
            {
                edge.push(pixel);
            } else if width / 4 <= x && x < width * 3 / 4 && height / 4 <= y && y < height * 3 / 4
            {
                center.push(pixel);
            }
        }
    }

    let mut visible_alpha: Vec<u8> = visible.iter().map(|pixel| pixel[3]).collect();
    visible_alpha.sort_unstable();
    let alpha_stats = AlphaStats {
        min: normalized(*visible_alpha.first().unwrap_or(&0)),
        median: normalized(visible_alpha.get(visible_alpha.len() / 2).copied().unwrap_or(0)),
        max: normalized(*visible_alpha.last().unwrap_or(&0)),
    };

    Ok(PanelStyleReport {
        schema: "gameterm.scene.panel_style.v1",
        source: path.display().to_string(),
        source_kind: "png",
        dimensions: PanelDimensions { width, height },
        recommended_renderer: "procedural_rounded_panel",
        style: PanelStyle {
            fill: color_sample(median_pixel(if center.is_empty() { &visible } else { &center })),
            border: color_sample(median_pixel(if edge.is_empty() { &visible } else { &edge })),
            corner_radius_px: estimate_corner_radius(width, &pixels),
            border_width_px: 1.5,
            slice_px: (width / 4).max(1),
            transparent_pixel_ratio: rounded(transparent_count as f32 / pixels.len().max(1) as f32),
            visible_alpha: alpha_stats,
        },
        trace_tools: TraceTools {
            vtracer: find_command("vtracer"),
            potrace: find_command("potrace"),
            autotrace: find_command("autotrace"),
        },
        notes: vec![
            "Use this as a compact procedural style, not as a large generated path list.",
            "VTracer/Potrace/AutoTrace can be used for exploratory SVG output, but panels should render through Rust rounded-rect primitives.",
        ],
        trace_svg: None,
    })
}

fn estimate_corner_radius(width: u32, pixels: &[[u8; 4]]) -> u32 {
    let limit = width.min(pixels.len() as u32) / 2;
    for x in 0..limit {
        if pixels[x as usize][3] >= 8 {
            return x;
        }
    }
    0
}

fn median_pixel(pixels: &[[u8; 4]]) -> [u8; 4] {
    if pixels.is_empty() {
        return [0, 0, 0, 0];
    }
    let mut result = [0, 0, 0, 0];
    for channel in 0..4 {
        let mut values: Vec<u8> = pixels.iter().map(|pixel| pixel[channel]).collect();
        values.sort_unstable();
        result[channel] = values[values.len() / 2];
    }
    result
}

fn color_sample(pixel: [u8; 4]) -> ColorSample {
    ColorSample {
        rgba8: pixel,
        rgba: [
            normalized(pixel[0]),
            normalized(pixel[1]),
            normalized(pixel[2]),
            normalized(pixel[3]),
        ],
        alpha: normalized(pixel[3]),
    }
}

fn normalized(value: u8) -> f32 {
    rounded(value as f32 / 255.0)
}

fn rounded(value: f32) -> f32 {
    (value * 10_000.0).round() / 10_000.0
}

fn find_command(command: &str) -> Option<String> {
    let path_var = std::env::var_os("PATH")?;
    for path in std::env::split_paths(&path_var) {
        let candidate = path.join(command);
        if candidate.is_file() {
            return Some(candidate.display().to_string());
        }
    }
    None
}

fn maybe_trace_svg(input_path: &PathBuf, output_path: &PathBuf) -> anyhow::Result<String> {
    if let Some(tool) = find_command("vtracer") {
        run_trace_command(
            Command::new(&tool)
                .arg("--input")
                .arg(input_path)
                .arg("--output")
                .arg(output_path),
            &tool,
        )?;
        return Ok("vtracer".to_string());
    }
    if let Some(tool) = find_command("potrace") {
        run_trace_command(
            Command::new(&tool)
                .arg("--svg")
                .arg("--output")
                .arg(output_path)
                .arg(input_path),
            &tool,
        )?;
        return Ok("potrace".to_string());
    }
    if let Some(tool) = find_command("autotrace") {
        run_trace_command(
            Command::new(&tool)
                .arg(input_path)
                .arg("-output-file")
                .arg(output_path),
            &tool,
        )?;
        return Ok("autotrace".to_string());
    }
    anyhow::bail!("no tracing tool found; install vtracer, potrace, or autotrace");
}

fn run_trace_command(command: &mut Command, tool: &str) -> anyhow::Result<()> {
    let status = command.status()?;
    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("{tool} exited with status {status}");
    }
}
