use anyhow::Context;
use gameterm_dynamic::Value;
use gameterm_term::color::ColorAttribute;
use gameterm_visual::{
    truncate_to_screen, SceneRuntime, VisualActionRequest, VisualInput, VisualMode,
    VisualModeOutcome, VisualResolvedSprite, VisualScene, VisualSceneLoadStatus, VisualSceneSource,
    VisualSpriteManifest, VisualSpriteManifestStatus,
};
use mux::termwiztermtab::TermWizTerminal;
use std::collections::HashSet;
use std::path::PathBuf;
use termwiz::input::{InputEvent, KeyCode, KeyEvent};
use termwiz::surface::Change;
use termwiz::terminal::Terminal;

pub fn show_visual_scene_overlay(mut term: TermWizTerminal) -> anyhow::Result<()> {
    term.no_grab_mouse_in_raw_mode();
    term.set_raw_mode()?;
    term.render(&[Change::Title("GameTerm Scene".to_string())])?;

    let scene_path = default_scene_path();
    let sprite_manifest_path = default_sprite_manifest_path();
    let mut sprite_manifest;
    let mut load_error;
    let mut runtime;
    let mut reload_count = 1;
    (runtime, sprite_manifest, load_error) =
        initial_scene_state(&mut term, &scene_path, &sprite_manifest_path, reload_count)?;

    while let Some(input) = term.poll_input(None)? {
        match input {
            InputEvent::Key(KeyEvent { key, .. }) => {
                let visual_input = visual_input_from_key(key);
                if visual_input == VisualInput::Close {
                    break;
                }
                if visual_input == VisualInput::Reload {
                    reload_count = reload_count.saturating_add(1);
                    sprite_manifest = load_sprite_manifest_status(&sprite_manifest_path);
                    match load_scene(&scene_path, reload_count) {
                        Ok((scene, source)) => {
                            if let Some(runtime) = runtime.as_mut() {
                                runtime.replace_scene_preserving_state(scene, source)?;
                                render_runtime(&mut term, runtime, &sprite_manifest)?;
                            } else {
                                let loaded = SceneRuntime::new_with_source(scene, source)?;
                                render_runtime(&mut term, &loaded, &sprite_manifest)?;
                                runtime = Some(loaded);
                            }
                            load_error = None;
                        }
                        Err(err) => {
                            let error = err.to_string();
                            if let Some(runtime) = runtime.as_mut() {
                                runtime.mark_reload_failed(reload_count, error);
                                render_runtime(&mut term, runtime, &sprite_manifest)?;
                            } else {
                                let source = VisualSceneSource::invalid(
                                    scene_path.display().to_string(),
                                    reload_count,
                                    error.clone(),
                                );
                                render_error(&mut term, &source)?;
                                load_error = Some(error);
                            }
                        }
                    }
                    continue;
                }
                if let Some(runtime) = runtime.as_mut() {
                    if runtime.handle_input(visual_input) == VisualModeOutcome::Exit {
                        break;
                    }
                    dispatch_pending_action(runtime);
                    render_runtime(&mut term, runtime, &sprite_manifest)?;
                }
            }
            InputEvent::Resized { .. } => {
                if let Some(runtime) = runtime.as_ref() {
                    render_runtime(&mut term, runtime, &sprite_manifest)?;
                } else {
                    let error = load_error
                        .as_deref()
                        .unwrap_or("scene failed to load for an unknown reason");
                    let source = VisualSceneSource::invalid(
                        scene_path.display().to_string(),
                        reload_count,
                        error.to_string(),
                    );
                    render_error(&mut term, &source)?;
                }
            }
            _ => {}
        }
    }

    Ok(())
}

fn dispatch_pending_action(runtime: &mut SceneRuntime) {
    let Some(action) = runtime.take_pending_action() else {
        return;
    };

    match action {
        VisualActionRequest::OpenFile { path } => {
            gameterm_open_url::open_url(&path.to_string_lossy());
            runtime.mark_open_file_dispatched(&path);
        }
    }
}

fn initial_scene_state(
    term: &mut TermWizTerminal,
    scene_path: &PathBuf,
    sprite_manifest_path: &PathBuf,
    reload_count: u64,
) -> anyhow::Result<(
    Option<SceneRuntime>,
    VisualSpriteManifestStatus,
    Option<String>,
)> {
    let sprite_manifest = load_sprite_manifest_status(sprite_manifest_path);
    match load_scene(scene_path, reload_count) {
        Ok((scene, source)) => {
            let runtime = SceneRuntime::new_with_source(scene, source)?;
            render_runtime(term, &runtime, &sprite_manifest)?;
            Ok((Some(runtime), sprite_manifest, None))
        }
        Err(err) => {
            let error = err.to_string();
            let source = VisualSceneSource::invalid(
                scene_path.display().to_string(),
                reload_count,
                error.clone(),
            );
            render_error(term, &source)?;
            Ok((None, sprite_manifest, Some(error)))
        }
    }
}

const BUNDLED_SCENE_JSON: &str = include_str!("../../../docs/examples/gameterm-scene-default.json");

fn load_scene(
    scene_path: &PathBuf,
    reload_count: u64,
) -> anyhow::Result<(VisualScene, VisualSceneSource)> {
    if scene_path.exists() {
        let scene = VisualScene::load_from_path(scene_path)?;
        Ok((
            scene,
            VisualSceneSource::new(
                scene_path.display().to_string(),
                VisualSceneLoadStatus::Loaded,
                reload_count,
            ),
        ))
    } else {
        let scene = VisualScene::from_json(BUNDLED_SCENE_JSON)
            .context("load bundled Scene Mode default")?;
        Ok((
            scene,
            VisualSceneSource::new(
                "bundled default",
                VisualSceneLoadStatus::Bundled,
                reload_count,
            ),
        ))
    }
}

fn default_scene_path() -> PathBuf {
    default_scene_dir().join("default.json")
}

fn default_sprite_manifest_path() -> PathBuf {
    default_scene_dir().join("sprites.json")
}

fn default_scene_dir() -> PathBuf {
    let config_home = config::CONFIG_DIRS
        .first()
        .cloned()
        .unwrap_or_else(|| config::HOME_DIR.join(".config").join("gameterm"));
    config_home.join("scenes")
}

fn load_sprite_manifest_status(path: &PathBuf) -> VisualSpriteManifestStatus {
    if !path.exists() {
        return bundled_sprite_manifest_status(path);
    }

    match VisualSpriteManifest::load_from_path(path) {
        Ok(manifest) => {
            let mut status = manifest.resolve_against(path);
            for sprite in &status.sprites {
                if let Err(err) = std::fs::metadata(&sprite.path) {
                    status.warnings.push(format!(
                        "sprite `{}` could not read {}: {}",
                        sprite.id, sprite.path, err
                    ));
                }
            }
            status
        }
        Err(err) => VisualSpriteManifestStatus {
            manifest_path: Some(path.display().to_string()),
            sprites: Vec::new(),
            warnings: vec![err.to_string()],
        },
    }
}

fn bundled_sprite_manifest_status(user_path: &PathBuf) -> VisualSpriteManifestStatus {
    let mut warnings = Vec::new();
    let sprite_ids = match bundled_scene_sprite_ids() {
        Ok(ids) => ids,
        Err(err) => {
            warnings.push(format!(
                "bundled sprite ids could not be derived from bundled scene: {err}"
            ));
            Vec::new()
        }
    };
    let sprites = sprite_ids
        .into_iter()
        .map(|id| {
            let sprite_path = bundled_sprite_asset_path(&id);
            if let Err(err) = std::fs::metadata(&sprite_path) {
                warnings.push(format!(
                    "bundled sprite asset `{}` could not read {}: {}",
                    id,
                    sprite_path.display(),
                    err
                ));
            }
            VisualResolvedSprite {
                id,
                path: sprite_path.display().to_string(),
            }
        })
        .collect();

    VisualSpriteManifestStatus {
        manifest_path: Some(format!(
            "bundled defaults because {} was not found",
            user_path.display()
        )),
        sprites,
        warnings,
    }
}

fn bundled_sprite_asset_path(sprite_id: &str) -> PathBuf {
    let file_name = match sprite_id {
        "workspace-map" => "workspace-map.png",
        "project_core" => "project-core.png",
        "task_tile" => "task-tile.png",
        "agent_idle" => "agent-idle.png",
        _ => "terminal.png",
    };
    let asset_dir = if file_name == "terminal.png" {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .map(|root| root.join("assets").join("icon"))
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .map(|root| root.join("assets").join("gameterm-scene"))
    };

    asset_dir.map(|dir| dir.join(file_name)).unwrap_or_else(|| {
        if file_name == "terminal.png" {
            PathBuf::from("assets").join("icon").join(file_name)
        } else {
            PathBuf::from("assets")
                .join("gameterm-scene")
                .join(file_name)
        }
    })
}

fn bundled_scene_sprite_ids() -> anyhow::Result<Vec<String>> {
    let scene = VisualScene::from_json(BUNDLED_SCENE_JSON).context("parse bundled scene")?;
    let mut seen = HashSet::new();
    let mut ids = Vec::new();
    if seen.insert(scene.background.clone()) {
        ids.push(scene.background);
    }
    for entity in scene.entities {
        if seen.insert(entity.sprite.clone()) {
            ids.push(entity.sprite);
        }
    }
    Ok(ids)
}

fn visual_input_from_key(key: KeyCode) -> VisualInput {
    match key {
        KeyCode::Escape | KeyCode::Char('q') | KeyCode::Char('Q') => VisualInput::Close,
        KeyCode::Char('r') | KeyCode::Char('R') => VisualInput::Reload,
        KeyCode::Tab => VisualInput::ToggleDebug,
        KeyCode::Enter => VisualInput::Activate,
        KeyCode::RightArrow | KeyCode::DownArrow | KeyCode::Char('l') | KeyCode::Char('j') => {
            VisualInput::Next
        }
        KeyCode::LeftArrow | KeyCode::UpArrow | KeyCode::Char('h') | KeyCode::Char('k') => {
            VisualInput::Previous
        }
        _ => VisualInput::Other,
    }
}

fn render_runtime(
    term: &mut TermWizTerminal,
    runtime: &SceneRuntime,
    sprite_manifest: &VisualSpriteManifestStatus,
) -> anyhow::Result<()> {
    let size = term.get_screen_size()?;
    term.set_metadata(
        "gameterm_visual_snapshot",
        Value::String(serde_json::to_string(&runtime.render_snapshot())?),
    );
    term.set_metadata(
        "gameterm_visual_sprites",
        Value::String(serde_json::to_string(sprite_manifest)?),
    );
    let mut frame = String::new();
    if !sprite_manifest.warnings.is_empty() {
        frame.push_str("Sprites: ");
        frame.push_str(&sprite_manifest.warnings.join("; "));
        frame.push_str("\r\n\r\n");
    }
    frame.push_str(&runtime.render_text_frame(size.cols, size.rows));
    term.render(&[
        Change::ClearScreen(ColorAttribute::Default),
        Change::Text(truncate_to_screen(frame, size.cols, size.rows)),
    ])?;
    term.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_sprite_ids_are_derived_from_bundled_scene() {
        let scene = VisualScene::from_json(BUNDLED_SCENE_JSON).unwrap();
        let mut seen = HashSet::new();
        let mut expected = Vec::new();
        if seen.insert(scene.background.clone()) {
            expected.push(scene.background);
        }
        expected.extend(scene.entities.into_iter().filter_map(|entity| {
            if seen.insert(entity.sprite.clone()) {
                Some(entity.sprite)
            } else {
                None
            }
        }));
        let actual = bundled_scene_sprite_ids().unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn bundled_sprite_assets_are_id_specific() {
        assert!(bundled_sprite_asset_path("workspace-map").ends_with("workspace-map.png"));
        assert!(bundled_sprite_asset_path("project_core").ends_with("project-core.png"));
        assert!(bundled_sprite_asset_path("task_tile").ends_with("task-tile.png"));
        assert!(bundled_sprite_asset_path("agent_idle").ends_with("agent-idle.png"));
        assert!(bundled_sprite_asset_path("other").ends_with("terminal.png"));
    }

    #[test]
    fn missing_scene_uses_bundled_source_status() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("missing.json");
        let (_scene, source) = load_scene(&path, 4).unwrap();

        assert_eq!(source.scene_path, "bundled default");
        assert_eq!(source.load_status, VisualSceneLoadStatus::Bundled);
        assert_eq!(source.reload_count, 4);
        assert_eq!(source.last_error, None);
    }

    #[test]
    fn scene_file_uses_loaded_source_status() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("scene.json");
        std::fs::write(&path, BUNDLED_SCENE_JSON).unwrap();
        let (_scene, source) = load_scene(&path, 2).unwrap();

        assert_eq!(source.scene_path, path.display().to_string());
        assert_eq!(source.load_status, VisualSceneLoadStatus::Loaded);
        assert_eq!(source.reload_count, 2);
        assert_eq!(source.last_error, None);
    }
}

fn render_error(term: &mut TermWizTerminal, source: &VisualSceneSource) -> anyhow::Result<()> {
    let size = term.get_screen_size()?;
    let frame = format!(
        "GameTerm Scene Mode\r\n\
         Scene file failed to load.\r\n\r\n\
         Path: {}\r\n\
         Load status: {}\r\n\
         Reload counter: {}\r\n\
         Error: {}\r\n\r\n\
         Fix the scene JSON, or remove the file to use the bundled default.\r\n\
         [r: reload] [esc/q: close]\r\n",
        source.scene_path,
        source.load_status.as_str(),
        source.reload_count,
        source
            .last_error
            .as_deref()
            .unwrap_or("scene failed to load for an unknown reason")
    );
    term.render(&[
        Change::ClearScreen(ColorAttribute::Default),
        Change::Text(truncate_to_screen(frame, size.cols, size.rows)),
    ])?;
    term.flush()?;
    Ok(())
}
