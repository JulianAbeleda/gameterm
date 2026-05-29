use gameterm_visual::{SceneRuntime, VisualScene, VisualSceneLoadStatus, VisualSceneSource};
use std::path::PathBuf;

fn usage() {
    eprintln!("Usage: cargo run -p gameterm-visual --example scene_validate -- PATH");
}

fn main() {
    let mut args = std::env::args_os().skip(1);
    let Some(path) = args.next() else {
        usage();
        std::process::exit(2);
    };
    if args.next().is_some() {
        usage();
        std::process::exit(2);
    }

    let path = PathBuf::from(path);
    let scene = match VisualScene::load_from_path(&path) {
        Ok(scene) => scene,
        Err(err) => {
            eprintln!("invalid Scene Mode file {}: {err}", path.display());
            std::process::exit(1);
        }
    };
    let runtime = match SceneRuntime::new_with_source(
        scene,
        VisualSceneSource::new(path.display().to_string(), VisualSceneLoadStatus::Loaded, 0),
    ) {
        Ok(runtime) => runtime,
        Err(err) => {
            eprintln!("invalid Scene Mode runtime {}: {err}", path.display());
            std::process::exit(1);
        }
    };
    let report = runtime.debug_report();

    println!("Scene Mode file is valid: {}", report.scene_path);
    println!(
        "title={} size={}x{} entities={} choices={} selected_entity={}",
        report.title,
        report.width,
        report.height,
        report.entity_count,
        report.choice_count,
        report.selected_entity_id.as_deref().unwrap_or("<none>")
    );
}
