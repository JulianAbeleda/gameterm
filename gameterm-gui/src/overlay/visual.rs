use anyhow::Context;
use config::keyassignment::SpawnTabDomain;
use gameterm_dynamic::Value;
use gameterm_term::color::ColorAttribute;
use gameterm_term::TerminalSize;
use gameterm_visual::{
    truncate_to_screen, RunCommandTarget, SceneRuntime, VisualActionRequest, VisualInput,
    VisualMode, VisualModeOutcome, VisualResolvedSprite, VisualScene, VisualSceneLoadStatus,
    VisualScenePatch, VisualSceneSource, VisualSpriteManifest, VisualSpriteManifestStatus,
};
use mux::domain::SplitSource;
use mux::tab::{SplitDirection, SplitRequest, SplitSize};
use mux::termwiztermtab::TermWizTerminal;
use mux::Mux;
use portable_pty::CommandBuilder;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, SystemTime};
use termwiz::input::{InputEvent, KeyCode, KeyEvent};
use termwiz::surface::Change;
use termwiz::terminal::Terminal;

pub fn show_visual_scene_overlay(mut term: TermWizTerminal) -> anyhow::Result<()> {
    term.no_grab_mouse_in_raw_mode();
    term.set_raw_mode()?;
    term.render(&[Change::Title("GameTerm Scene".to_string())])?;

    let mut scene_path = default_scene_path();
    let sprite_manifest_path = default_sprite_manifest_path();
    let mut sprite_manifest;
    let mut load_error;
    let mut runtime;
    let mut reload_count = 1;
    let (command_tx, command_rx) = mpsc::channel();
    (runtime, sprite_manifest, load_error) =
        initial_scene_state(&mut term, &scene_path, &sprite_manifest_path, reload_count)?;
    let mut file_watcher = SceneFileWatcher::from_env(&scene_path, &sprite_manifest_path);
    let mut patch_inbox = ScenePatchInbox::from_env();

    loop {
        let mut needs_render = false;
        while let Ok(result) = command_rx.try_recv() {
            if let Some(runtime) = runtime.as_mut() {
                match result {
                    RunCommandResult::Spawned {
                        argv,
                        target,
                        pane_id,
                    } => {
                        runtime.mark_run_command_spawned(&argv, target, pane_id);
                    }
                    RunCommandResult::Failed {
                        argv,
                        target,
                        error,
                    } => {
                        runtime.mark_run_command_failed(&argv, target, error);
                    }
                }
                needs_render = true;
            }
        }
        if needs_render {
            if let Some(runtime) = runtime.as_ref() {
                render_runtime(&mut term, runtime, &sprite_manifest)?;
            }
        }

        let Some(input) = term.poll_input(Some(Duration::from_millis(100)))? else {
            if file_watcher.changed(&scene_path, &sprite_manifest_path) {
                reload_active_scene(
                    &mut term,
                    &scene_path,
                    &sprite_manifest_path,
                    &mut reload_count,
                    &mut runtime,
                    &mut sprite_manifest,
                    &mut load_error,
                )?;
                file_watcher.refresh(&scene_path, &sprite_manifest_path);
            }
            if let Some(path) = patch_inbox.changed_path() {
                if let Some(runtime) = runtime.as_mut() {
                    apply_scene_patch_file(&mut term, runtime, &sprite_manifest, &path)?;
                }
                patch_inbox.refresh();
            }
            continue;
        };
        match input {
            InputEvent::Key(KeyEvent { key, .. }) => {
                let visual_input = visual_input_from_key(key);
                if visual_input == VisualInput::Close {
                    break;
                }
                if visual_input == VisualInput::Reload {
                    reload_active_scene(
                        &mut term,
                        &scene_path,
                        &sprite_manifest_path,
                        &mut reload_count,
                        &mut runtime,
                        &mut sprite_manifest,
                        &mut load_error,
                    )?;
                    file_watcher.refresh(&scene_path, &sprite_manifest_path);
                    continue;
                }
                if let Some(runtime) = runtime.as_mut() {
                    if runtime.handle_input(visual_input) == VisualModeOutcome::Exit {
                        break;
                    }
                    let size = term.get_screen_size()?;
                    dispatch_pending_action(
                        runtime,
                        &mut scene_path,
                        &mut reload_count,
                        RunCommandDispatch {
                            window_id: term
                                .window_id()
                                .context("Scene Mode terminal is not attached to a mux window")?,
                            pane_id: term.pane_id(),
                            terminal_size: TerminalSize {
                                rows: size.rows,
                                cols: size.cols,
                                pixel_width: size.xpixel.saturating_mul(size.cols),
                                pixel_height: size.ypixel.saturating_mul(size.rows),
                                dpi: 0,
                            },
                            command_tx: command_tx.clone(),
                        },
                    )?;
                    file_watcher.refresh(&scene_path, &sprite_manifest_path);
                    patch_inbox.refresh();
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

fn reload_active_scene(
    term: &mut TermWizTerminal,
    scene_path: &PathBuf,
    sprite_manifest_path: &PathBuf,
    reload_count: &mut u64,
    runtime: &mut Option<SceneRuntime>,
    sprite_manifest: &mut VisualSpriteManifestStatus,
    load_error: &mut Option<String>,
) -> anyhow::Result<()> {
    *reload_count = reload_count.saturating_add(1);
    *sprite_manifest = load_sprite_manifest_status(sprite_manifest_path);
    match load_scene(scene_path, *reload_count) {
        Ok((scene, source)) => {
            if let Some(runtime) = runtime.as_mut() {
                runtime.replace_scene_preserving_state(scene, source)?;
                render_runtime(term, runtime, sprite_manifest)?;
            } else {
                let loaded = SceneRuntime::new_with_source(scene, source)?;
                render_runtime(term, &loaded, sprite_manifest)?;
                *runtime = Some(loaded);
            }
            *load_error = None;
        }
        Err(err) => {
            let error = err.to_string();
            if let Some(runtime) = runtime.as_mut() {
                runtime.mark_reload_failed(*reload_count, error);
                render_runtime(term, runtime, sprite_manifest)?;
            } else {
                let source = VisualSceneSource::invalid(
                    scene_path.display().to_string(),
                    *reload_count,
                    error.clone(),
                );
                render_error(term, &source)?;
                *load_error = Some(error);
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct SceneFileWatcher {
    enabled: bool,
    scene_stamp: Option<SystemTime>,
    sprite_stamp: Option<SystemTime>,
    scene_dir_stamp: Option<SystemTime>,
}

impl SceneFileWatcher {
    fn disabled() -> Self {
        Self {
            enabled: false,
            scene_stamp: None,
            sprite_stamp: None,
            scene_dir_stamp: None,
        }
    }

    fn from_env(scene_path: &Path, sprite_path: &Path) -> Self {
        if std::env::var("GAMETERM_SCENE_AUTO_RELOAD").ok().as_deref() != Some("1") {
            return Self::disabled();
        }
        Self::enabled(scene_path, sprite_path)
    }

    fn enabled(scene_path: &Path, sprite_path: &Path) -> Self {
        let mut watcher = Self {
            enabled: true,
            scene_stamp: None,
            sprite_stamp: None,
            scene_dir_stamp: None,
        };
        watcher.refresh(scene_path, sprite_path);
        watcher
    }

    fn refresh(&mut self, scene_path: &Path, sprite_path: &Path) {
        self.scene_stamp = modified_time(scene_path);
        self.sprite_stamp = modified_time(sprite_path);
        self.scene_dir_stamp = scene_path.parent().and_then(modified_time);
    }

    fn changed(&self, scene_path: &Path, sprite_path: &Path) -> bool {
        self.enabled
            && (self.scene_stamp != modified_time(scene_path)
                || self.sprite_stamp != modified_time(sprite_path)
                || self.scene_dir_stamp != scene_path.parent().and_then(modified_time))
    }
}

fn modified_time(path: &Path) -> Option<SystemTime> {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
}

#[derive(Debug, Clone)]
struct ScenePatchInbox {
    path: Option<PathBuf>,
    stamp: Option<SystemTime>,
}

impl ScenePatchInbox {
    fn disabled() -> Self {
        Self {
            path: None,
            stamp: None,
        }
    }

    fn from_env() -> Self {
        let Some(path) = std::env::var_os("GAMETERM_SCENE_PATCH_FILE").map(PathBuf::from) else {
            return Self::disabled();
        };
        Self::watching(path)
    }

    fn watching(path: PathBuf) -> Self {
        let stamp = modified_time(&path);
        Self {
            path: Some(path),
            stamp,
        }
    }

    fn refresh(&mut self) {
        if let Some(path) = &self.path {
            self.stamp = modified_time(path);
        }
    }

    fn changed_path(&self) -> Option<PathBuf> {
        let path = self.path.as_ref()?;
        let stamp = modified_time(path);
        if stamp.is_some() && stamp != self.stamp {
            Some(path.clone())
        } else {
            None
        }
    }
}

fn apply_scene_patch_file(
    term: &mut TermWizTerminal,
    runtime: &mut SceneRuntime,
    sprite_manifest: &VisualSpriteManifestStatus,
    path: &Path,
) -> anyhow::Result<()> {
    match VisualScenePatch::load_from_path(path).and_then(|patch| runtime.apply_scene_patch(patch))
    {
        Ok(()) => {}
        Err(err) => {
            runtime.mark_action_status(format!("Scene patch failed: {}: {err}", path.display()));
        }
    }
    render_runtime(term, runtime, sprite_manifest)
}

enum RunCommandResult {
    Spawned {
        argv: Vec<String>,
        target: RunCommandTarget,
        pane_id: mux::pane::PaneId,
    },
    Failed {
        argv: Vec<String>,
        target: RunCommandTarget,
        error: String,
    },
}

#[derive(Clone)]
struct RunCommandDispatch {
    window_id: mux::window::WindowId,
    pane_id: Option<mux::pane::PaneId>,
    terminal_size: TerminalSize,
    command_tx: mpsc::Sender<RunCommandResult>,
}

fn dispatch_pending_action(
    runtime: &mut SceneRuntime,
    scene_path: &mut PathBuf,
    reload_count: &mut u64,
    command_dispatch: RunCommandDispatch,
) -> anyhow::Result<()> {
    let Some(action) = runtime.take_pending_action() else {
        return Ok(());
    };

    match action {
        VisualActionRequest::OpenFile { path } => {
            gameterm_open_url::open_url(&path.to_string_lossy());
            runtime.mark_open_file_dispatched(&path);
        }
        VisualActionRequest::RunCommand { argv, cwd, target } => {
            dispatch_run_command(runtime, argv, cwd, target, command_dispatch);
        }
        VisualActionRequest::Navigate { target } => {
            *reload_count = reload_count.saturating_add(1);
            let target_path = resolve_scene_target(scene_path, &target);
            match load_scene_required(&target_path, *reload_count) {
                Ok((scene, source)) => {
                    runtime.replace_scene_preserving_state(scene, source)?;
                    *scene_path = target_path;
                }
                Err(err) => {
                    runtime.mark_action_status(format!(
                        "Navigate failed: {}: {err}",
                        target_path.display()
                    ));
                }
            }
        }
    }
    Ok(())
}

fn dispatch_run_command(
    runtime: &mut SceneRuntime,
    argv: Vec<String>,
    cwd: Option<PathBuf>,
    target: RunCommandTarget,
    dispatch: RunCommandDispatch,
) {
    runtime.mark_run_command_spawning(&argv, target);
    promise::spawn::spawn(async move {
        let command_dir = cwd.as_ref().map(|cwd| cwd.display().to_string());
        let mut builder = CommandBuilder::from_argv(argv.iter().map(Into::into).collect());
        if let Some(cwd) = cwd.as_ref() {
            builder.cwd(cwd);
        }

        let result = match spawn_run_command(target, builder, command_dir, &dispatch).await {
            Ok(pane_id) => RunCommandResult::Spawned {
                argv,
                target,
                pane_id,
            },
            Err(err) => RunCommandResult::Failed {
                argv,
                target,
                error: err.to_string(),
            },
        };
        let _ = dispatch.command_tx.send(result);
    })
    .detach();
}

async fn spawn_run_command(
    target: RunCommandTarget,
    builder: CommandBuilder,
    command_dir: Option<String>,
    dispatch: &RunCommandDispatch,
) -> anyhow::Result<mux::pane::PaneId> {
    match target {
        RunCommandTarget::Tab => {
            let (_tab, pane, _window_id) = Mux::get()
                .spawn_tab_or_window(
                    Some(dispatch.window_id),
                    SpawnTabDomain::DefaultDomain,
                    Some(builder),
                    command_dir,
                    dispatch.terminal_size,
                    None,
                    Mux::get().active_workspace(),
                    None,
                )
                .await?;
            Ok(pane.pane_id())
        }
        RunCommandTarget::SplitRight | RunCommandTarget::SplitDown => {
            let pane_id = dispatch
                .pane_id
                .context("Scene Mode terminal is not attached to a mux pane")?;
            let request = SplitRequest {
                direction: match target {
                    RunCommandTarget::SplitRight => SplitDirection::Horizontal,
                    RunCommandTarget::SplitDown => SplitDirection::Vertical,
                    RunCommandTarget::Tab => unreachable!(),
                },
                target_is_second: true,
                top_level: false,
                size: SplitSize::Percent(50),
            };
            let (pane, _size) = Mux::get()
                .split_pane(
                    pane_id,
                    request,
                    SplitSource::Spawn {
                        command: Some(builder),
                        command_dir,
                    },
                    SpawnTabDomain::DefaultDomain,
                )
                .await?;
            Ok(pane.pane_id())
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

fn load_scene_required(
    scene_path: &Path,
    reload_count: u64,
) -> anyhow::Result<(VisualScene, VisualSceneSource)> {
    let scene = VisualScene::load_from_path(scene_path)?;
    Ok((
        scene,
        VisualSceneSource::new(
            scene_path.display().to_string(),
            VisualSceneLoadStatus::Loaded,
            reload_count,
        ),
    ))
}

fn resolve_scene_target(current_scene_path: &Path, target: &str) -> PathBuf {
    let raw_target = PathBuf::from(target);
    if raw_target.is_absolute() {
        return raw_target;
    }

    current_scene_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(raw_target)
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

    fn scene_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("ci")
            .join("fixtures")
            .join("gameterm-scene")
            .join(name)
    }

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

    #[test]
    fn scene_fixture_loaded_source_uses_fixture_path() {
        let path = scene_fixture_path("default.json");
        let (scene, source) = load_scene(&path, 7).unwrap();

        assert_eq!(scene.title, "Scene Harness Default");
        assert_eq!(source.scene_path, path.display().to_string());
        assert_eq!(source.load_status, VisualSceneLoadStatus::Loaded);
        assert_eq!(source.reload_count, 7);
    }

    #[test]
    fn scene_fixture_invalid_load_reports_error() {
        let path = scene_fixture_path("invalid.json");

        assert!(load_scene(&path, 3).is_err());
    }

    #[test]
    fn scene_fixture_sprite_manifest_loads_without_warnings() {
        let path = scene_fixture_path("sprites.json");
        let status = load_sprite_manifest_status(&path);

        assert!(status.warnings.is_empty());
        assert!(status
            .sprites
            .iter()
            .any(|sprite| sprite.id == "project_core"));
    }

    #[test]
    fn scene_fixture_missing_sprite_manifest_reports_warning() {
        let path = scene_fixture_path("sprites-missing.json");
        let status = load_sprite_manifest_status(&path);

        assert_eq!(status.sprites.len(), 2);
        assert_eq!(status.warnings.len(), 1);
        assert!(status.warnings[0].contains("project_core"));
    }

    #[test]
    fn navigate_targets_resolve_against_current_scene_dir() {
        let current = PathBuf::from("/tmp/gameterm/scenes/default.json");

        assert_eq!(
            resolve_scene_target(&current, "memory.json"),
            PathBuf::from("/tmp/gameterm/scenes/memory.json")
        );
        assert_eq!(
            resolve_scene_target(&current, "worlds/debug.json"),
            PathBuf::from("/tmp/gameterm/scenes/worlds/debug.json")
        );
        assert_eq!(
            resolve_scene_target(&current, "/tmp/other.json"),
            PathBuf::from("/tmp/other.json")
        );
    }

    #[test]
    fn navigate_load_requires_scene_file() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("missing.json");

        assert!(load_scene_required(&missing, 3).is_err());
    }

    #[test]
    fn dispatch_navigate_loads_target_scene_and_updates_active_path() {
        let dir = tempfile::tempdir().unwrap();
        let default_path = dir.path().join("default.json");
        let target_path = dir.path().join("memory.json");
        let mut first_scene = VisualScene::demo();
        first_scene.choices = vec![gameterm_visual::SceneAction {
            label: "Open memory".to_string(),
            kind: gameterm_visual::SceneActionKind::Navigate {
                target: "memory.json".to_string(),
            },
        }];
        let mut target_scene = VisualScene::demo();
        target_scene.title = "Memory Scene".to_string();
        std::fs::write(&target_path, serde_json::to_string(&target_scene).unwrap()).unwrap();
        let mut runtime = SceneRuntime::new_with_source(
            first_scene,
            VisualSceneSource::new(
                default_path.display().to_string(),
                VisualSceneLoadStatus::Loaded,
                1,
            ),
        )
        .unwrap();
        let mut active_path = default_path;
        let mut reload_count = 1;
        let (command_tx, _command_rx) = mpsc::channel();

        runtime.activate_choice();
        dispatch_pending_action(
            &mut runtime,
            &mut active_path,
            &mut reload_count,
            RunCommandDispatch {
                window_id: 0,
                pane_id: None,
                terminal_size: TerminalSize::default(),
                command_tx,
            },
        )
        .unwrap();
        let snapshot = runtime.render_snapshot();

        assert_eq!(active_path, target_path);
        assert_eq!(reload_count, 2);
        assert_eq!(snapshot.title, "Memory Scene");
        assert_eq!(
            snapshot.scene_source.scene_path,
            active_path.display().to_string()
        );
    }

    #[test]
    fn scene_file_watcher_detects_scene_and_sprite_changes() {
        let dir = tempfile::tempdir().unwrap();
        let scene = dir.path().join("default.json");
        let sprites = dir.path().join("sprites.json");
        std::fs::write(&scene, BUNDLED_SCENE_JSON).unwrap();
        std::fs::write(&sprites, "{\"sprites\":[]}").unwrap();
        let mut watcher = SceneFileWatcher::enabled(&scene, &sprites);

        assert!(!watcher.changed(&scene, &sprites));

        std::thread::sleep(Duration::from_millis(5));
        std::fs::write(
            &sprites,
            "{\"sprites\":[{\"id\":\"a\",\"path\":\"a.png\"}]}",
        )
        .unwrap();
        assert!(watcher.changed(&scene, &sprites));

        watcher.refresh(&scene, &sprites);
        assert!(!watcher.changed(&scene, &sprites));

        std::thread::sleep(Duration::from_millis(5));
        std::fs::write(
            &scene,
            BUNDLED_SCENE_JSON.replace("GameTerm", "GameTerm Reloaded"),
        )
        .unwrap();
        assert!(watcher.changed(&scene, &sprites));
    }

    #[test]
    fn disabled_scene_file_watcher_never_reports_changes() {
        let dir = tempfile::tempdir().unwrap();
        let scene = dir.path().join("default.json");
        let sprites = dir.path().join("sprites.json");
        std::fs::write(&scene, BUNDLED_SCENE_JSON).unwrap();
        let watcher = SceneFileWatcher::disabled();

        assert!(!watcher.changed(&scene, &sprites));
    }

    #[test]
    fn scene_patch_inbox_detects_file_change_once() {
        let dir = tempfile::tempdir().unwrap();
        let patch = dir.path().join("patch.json");
        let mut inbox = ScenePatchInbox::watching(patch.clone());

        assert_eq!(inbox.changed_path(), None);

        std::fs::write(&patch, "{\"scene_patch_version\":1,\"status\":\"ready\"}").unwrap();
        assert_eq!(inbox.changed_path(), Some(patch.clone()));

        inbox.refresh();
        assert_eq!(inbox.changed_path(), None);

        std::thread::sleep(Duration::from_millis(5));
        std::fs::write(&patch, "{\"scene_patch_version\":1,\"status\":\"updated\"}").unwrap();
        assert_eq!(inbox.changed_path(), Some(patch));
    }

    #[test]
    fn disabled_scene_patch_inbox_never_reports_changes() {
        let dir = tempfile::tempdir().unwrap();
        let patch = dir.path().join("patch.json");
        std::fs::write(&patch, "{\"scene_patch_version\":1,\"status\":\"ready\"}").unwrap();
        let inbox = ScenePatchInbox::disabled();

        assert_eq!(inbox.changed_path(), None);
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
