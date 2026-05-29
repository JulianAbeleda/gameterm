use anyhow::Context;
use gameterm_dynamic::Value;
use gameterm_term::color::ColorAttribute;
use gameterm_visual::{
    truncate_to_screen, SceneRuntime, VisualInput, VisualMode, VisualModeOutcome,
    VisualResolvedSprite, VisualScene, VisualSpriteManifest, VisualSpriteManifestStatus,
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
    (runtime, sprite_manifest, load_error) =
        reload_scene_state(&mut term, &scene_path, &sprite_manifest_path)?;

    while let Some(input) = term.poll_input(None)? {
        match input {
            InputEvent::Key(KeyEvent { key, .. }) => {
                let visual_input = visual_input_from_key(key);
                if visual_input == VisualInput::Close {
                    break;
                }
                if visual_input == VisualInput::Reload {
                    (runtime, sprite_manifest, load_error) =
                        reload_scene_state(&mut term, &scene_path, &sprite_manifest_path)?;
                    continue;
                }
                if let Some(runtime) = runtime.as_mut() {
                    if runtime.handle_input(visual_input) == VisualModeOutcome::Exit {
                        break;
                    }
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
                    render_error(&mut term, &scene_path, error)?;
                }
            }
            _ => {}
        }
    }

    Ok(())
}

fn reload_scene_state(
    term: &mut TermWizTerminal,
    scene_path: &PathBuf,
    sprite_manifest_path: &PathBuf,
) -> anyhow::Result<(
    Option<SceneRuntime>,
    VisualSpriteManifestStatus,
    Option<String>,
)> {
    let sprite_manifest = load_sprite_manifest_status(sprite_manifest_path);
    match load_scene_runtime(scene_path) {
        Ok(runtime) => {
            render_runtime(term, &runtime, &sprite_manifest)?;
            Ok((Some(runtime), sprite_manifest, None))
        }
        Err(err) => {
            let error = err.to_string();
            render_error(term, scene_path, &error)?;
            Ok((None, sprite_manifest, Some(error)))
        }
    }
}

const BUNDLED_SCENE_JSON: &str = include_str!("../../../docs/examples/gameterm-scene-default.json");

fn load_scene_runtime(scene_path: &PathBuf) -> anyhow::Result<SceneRuntime> {
    let scene = if scene_path.exists() {
        VisualScene::load_from_path(scene_path)?
    } else {
        VisualScene::from_json(BUNDLED_SCENE_JSON).context("load bundled Scene Mode default")?
    };
    Ok(SceneRuntime::new(scene)?)
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
    let sprite_path = bundled_sprite_asset_path();
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
    if let Err(err) = std::fs::metadata(&sprite_path) {
        warnings.push(format!(
            "bundled sprite asset could not read {}: {}",
            sprite_path.display(),
            err
        ));
    }

    VisualSpriteManifestStatus {
        manifest_path: Some(format!(
            "bundled defaults because {} was not found",
            user_path.display()
        )),
        sprites: sprite_ids
            .into_iter()
            .map(|id| VisualResolvedSprite {
                id,
                path: sprite_path.display().to_string(),
            })
            .collect(),
        warnings,
    }
}

fn bundled_sprite_asset_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|root| root.join("assets").join("icon").join("terminal.png"))
        .unwrap_or_else(|| PathBuf::from("assets/icon/terminal.png"))
}

fn bundled_scene_sprite_ids() -> anyhow::Result<Vec<String>> {
    let scene = VisualScene::from_json(BUNDLED_SCENE_JSON).context("parse bundled scene")?;
    let mut seen = HashSet::new();
    let mut ids = Vec::new();
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
        let expected: Vec<_> = scene
            .entities
            .into_iter()
            .filter_map(|entity| {
                if seen.insert(entity.sprite.clone()) {
                    Some(entity.sprite)
                } else {
                    None
                }
            })
            .collect();
        let actual = bundled_scene_sprite_ids().unwrap();

        assert_eq!(actual, expected);
    }
}

fn render_error(
    term: &mut TermWizTerminal,
    scene_path: &PathBuf,
    error: &str,
) -> anyhow::Result<()> {
    let size = term.get_screen_size()?;
    let frame = format!(
        "GameTerm Scene Mode\r\n\
         Scene file failed to load.\r\n\r\n\
         Path: {}\r\n\
         Error: {}\r\n\r\n\
         Fix the scene JSON, or remove the file to use the bundled default.\r\n\
         [r: reload] [esc/q: close]\r\n",
        scene_path.display(),
        error
    );
    term.render(&[
        Change::ClearScreen(ColorAttribute::Default),
        Change::Text(truncate_to_screen(frame, size.cols, size.rows)),
    ])?;
    term.flush()?;
    Ok(())
}
