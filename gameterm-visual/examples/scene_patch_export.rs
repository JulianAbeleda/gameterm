use gameterm_visual::{
    SceneRuntime, VisualScene, VisualSceneLoadStatus, VisualScenePatch, VisualSceneSource,
};
use std::path::PathBuf;

fn usage() {
    eprintln!(
        "Usage: cargo run -p gameterm-visual --example scene_patch_export -- SCENE PATCH OUTPUT"
    );
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
    let Some(output_path) = args.next() else {
        usage();
        std::process::exit(2);
    };
    if args.next().is_some() {
        usage();
        std::process::exit(2);
    }

    let scene_path = PathBuf::from(scene_path);
    let patch_path = PathBuf::from(patch_path);
    let output_path = PathBuf::from(output_path);
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

    let json = match runtime.scene_json_pretty() {
        Ok(json) => json,
        Err(err) => {
            eprintln!("failed to serialize patched Scene Mode file: {err}");
            std::process::exit(1);
        }
    };

    if let Some(parent) = output_path.parent() {
        if let Err(err) = std::fs::create_dir_all(parent) {
            eprintln!("failed to create {}: {err}", parent.display());
            std::process::exit(1);
        }
    }
    if let Err(err) = std::fs::write(&output_path, format!("{json}\n")) {
        eprintln!("failed to write {}: {err}", output_path.display());
        std::process::exit(1);
    }

    println!("Wrote patched Scene Mode file: {}", output_path.display());
}
