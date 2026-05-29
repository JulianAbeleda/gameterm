use gameterm_visual::{
    SceneRuntime, VisualScene, VisualSceneLoadStatus, VisualScenePatch, VisualSceneSource,
};
use std::path::PathBuf;

fn usage() {
    eprintln!("Usage: cargo run -p gameterm-visual --example scene_patch_apply -- SCENE PATCH");
}

fn main() {
    let mut args = std::env::args_os().skip(1);
    let Some(scene_path) = args.next() else {
        usage();
        std::process::exit(2);
    };
    let Some(patch_path) = args.next() else {
        usage();
        std::process::exit(2);
    };
    if args.next().is_some() {
        usage();
        std::process::exit(2);
    }

    let scene_path = PathBuf::from(scene_path);
    let patch_path = PathBuf::from(patch_path);
    let scene = match VisualScene::load_from_path(&scene_path) {
        Ok(scene) => scene,
        Err(err) => {
            eprintln!("invalid Scene Mode file {}: {err}", scene_path.display());
            std::process::exit(1);
        }
    };
    let patch = match VisualScenePatch::load_from_path(&patch_path) {
        Ok(patch) => patch,
        Err(err) => {
            eprintln!("invalid Scene Mode patch {}: {err}", patch_path.display());
            std::process::exit(1);
        }
    };
    let mut runtime = match SceneRuntime::new_with_source(
        scene,
        VisualSceneSource::new(
            scene_path.display().to_string(),
            VisualSceneLoadStatus::Loaded,
            0,
        ),
    ) {
        Ok(runtime) => runtime,
        Err(err) => {
            eprintln!("invalid Scene Mode runtime {}: {err}", scene_path.display());
            std::process::exit(1);
        }
    };

    if let Err(err) = runtime.apply_scene_patch(patch) {
        eprintln!("Scene Mode patch failed: {err}");
        std::process::exit(1);
    }

    let report = runtime.debug_report();
    println!("Scene Mode patch applied: {}", patch_path.display());
    println!("status={}", report.status);
    println!("generation={}", runtime.generation());
    println!(
        "selected_entity={} flags={}",
        report.selected_entity_id.as_deref().unwrap_or("<none>"),
        report.selected_entity_flags.join(",")
    );
    for (key, value) in report.selected_entity_metadata {
        println!("metadata.{key}={value}");
    }
}
