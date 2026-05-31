use gameterm_visual::{
    SceneRuntime, VisualScene, VisualSceneLoadStatus, VisualSceneSource, VisualStoryState,
};
use std::path::PathBuf;

fn usage() {
    eprintln!(
        "Usage:\n  scene_story_state export SCENE OUTPUT\n  scene_story_state import SCENE STATE OUTPUT\n  scene_story_state validate STATE\n  scene_story_state inspect STATE"
    );
}

fn load_runtime(scene_path: PathBuf) -> SceneRuntime {
    let scene = match VisualScene::load_from_path(&scene_path) {
        Ok(scene) => scene,
        Err(err) => {
            eprintln!("invalid Scene Mode file {}: {err}", scene_path.display());
            std::process::exit(1);
        }
    };
    match SceneRuntime::new_with_source(
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
    }
}

fn load_story_state(path: PathBuf) -> VisualStoryState {
    match VisualStoryState::load_from_path(&path) {
        Ok(state) => state,
        Err(err) => {
            eprintln!("invalid Scene Mode story state {}: {err}", path.display());
            std::process::exit(1);
        }
    }
}

fn write_output(path: PathBuf, json: String) {
    if let Some(parent) = path.parent() {
        if let Err(err) = std::fs::create_dir_all(parent) {
            eprintln!("failed to create {}: {err}", parent.display());
            std::process::exit(1);
        }
    }
    if let Err(err) = std::fs::write(&path, format!("{json}\n")) {
        eprintln!("failed to write {}: {err}", path.display());
        std::process::exit(1);
    }
    println!("Wrote Scene Mode story state: {}", path.display());
}

fn main() {
    let mut args = std::env::args_os().skip(1);
    let Some(command) = args.next() else {
        usage();
        std::process::exit(2);
    };
    match command.to_string_lossy().as_ref() {
        "export" => {
            let Some(scene_path) = args.next() else {
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
            let runtime = load_runtime(PathBuf::from(scene_path));
            let json = runtime.story_state_json_pretty().unwrap_or_else(|err| {
                eprintln!("failed to serialize Scene Mode story state: {err}");
                std::process::exit(1);
            });
            write_output(PathBuf::from(output_path), json);
        }
        "import" => {
            let Some(scene_path) = args.next() else {
                usage();
                std::process::exit(2);
            };
            let Some(state_path) = args.next() else {
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
            let mut runtime = load_runtime(PathBuf::from(scene_path));
            let state = load_story_state(PathBuf::from(state_path));
            if let Err(err) = runtime.import_story_state(state) {
                eprintln!("failed to import Scene Mode story state: {err}");
                std::process::exit(1);
            }
            let json = runtime.story_state_json_pretty().unwrap_or_else(|err| {
                eprintln!("failed to serialize imported Scene Mode story state: {err}");
                std::process::exit(1);
            });
            write_output(PathBuf::from(output_path), json);
        }
        "validate" => {
            let Some(state_path) = args.next() else {
                usage();
                std::process::exit(2);
            };
            if args.next().is_some() {
                usage();
                std::process::exit(2);
            }
            load_story_state(PathBuf::from(&state_path));
            println!(
                "Scene Mode story state is valid: {}",
                PathBuf::from(state_path).display()
            );
        }
        "inspect" => {
            let Some(state_path) = args.next() else {
                usage();
                std::process::exit(2);
            };
            if args.next().is_some() {
                usage();
                std::process::exit(2);
            }
            let state = load_story_state(PathBuf::from(&state_path));
            println!("story_state_version={}", state.story_state_version);
            println!("variables={}", state.variables.len());
            let dialogue_index = state
                .dialogue_index
                .map(|index| index.to_string())
                .unwrap_or_else(|| "none".to_string());
            println!("dialogue_index={dialogue_index}");
            println!("dialogue_history={}", state.dialogue_history.len());
            println!("inventory={}", state.rpg.inventory.len());
            println!("stats={}", state.rpg.stats.len());
            println!("quests={}", state.rpg.quests.len());
            println!("relationships={}", state.rpg.relationships.len());
        }
        _ => {
            usage();
            std::process::exit(2);
        }
    }
}
