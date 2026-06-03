use crate::termwindow::TermWindowNotif;
use anyhow::Context;
use config::keyassignment::SpawnTabDomain;
use gameterm_dynamic::Value;
use gameterm_term::color::ColorAttribute;
use gameterm_term::TerminalSize;
use gameterm_visual::{
    truncate_to_screen, vn_overlay_layout, vn_overlay_layout_with_overrides, RunCommandTarget, SceneRuntime, VisualActionRequest, VisualView,
    VisualInput, VisualMode, VisualModeOutcome, VisualResolvedSprite, VisualScene,
    VisualSceneDialoguePatch, VisualSceneLoadStatus, VisualScenePatch, VisualSceneSource,
    VisualSpriteManifest, VisualSpriteManifestStatus, VisualStoryState, VnOverlayRect,
};
use mux::domain::SplitSource;
use mux::tab::{SplitDirection, SplitRequest, SplitSize};
use mux::termwiztermtab::TermWizTerminal;
use mux::{Mux, MuxNotification};
use portable_pty::CommandBuilder;
use serde::Deserialize;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use termwiz::input::{InputEvent, KeyCode, KeyEvent, Modifiers};
use termwiz::surface::Change;
use termwiz::terminal::Terminal;
use window::{Window, WindowOps};

#[cfg(test)]
use super::visual_compose::ComposeBackendLabel;
use super::visual_compose::{
    compose_running_status, spawn_compose_backend, ComposeBackendRequest, ComposeBackendResult,
};
use super::visual_stt::{spawn_stt_backend, SceneSttCancel, SceneSttResult, SceneSttState};
use super::visual_tts::{
    extract_speakable_segments, spawn_tts_backend, SceneTtsRequest, SceneTtsResult, SceneTtsState,
    SpeakableSegment, SpeakableSource,
};

pub fn show_visual_scene_overlay(
    term: TermWizTerminal,
    route_pane_id: Option<mux::pane::PaneId>,
    gui_window: Option<Window>,
) -> anyhow::Result<()> {
    show_visual_scene_overlay_with_source(
        term,
        route_pane_id,
        gui_window,
        VisualSceneOverlaySource::Default,
    )
}

pub fn show_generated_visual_scene_overlay(
    term: TermWizTerminal,
    route_pane_id: Option<mux::pane::PaneId>,
    gui_window: Option<Window>,
    scene: VisualScene,
    action_base_dir: PathBuf,
    source_label: impl Into<String>,
) -> anyhow::Result<()> {
    show_visual_scene_overlay_with_source(
        term,
        route_pane_id,
        gui_window,
        VisualSceneOverlaySource::Generated {
            scene,
            action_base_dir,
            source_label: source_label.into(),
        },
    )
}

enum VisualSceneOverlaySource {
    Default,
    Generated {
        scene: VisualScene,
        action_base_dir: PathBuf,
        source_label: String,
    },
}

fn show_visual_scene_overlay_with_source(
    mut term: TermWizTerminal,
    route_pane_id: Option<mux::pane::PaneId>,
    gui_window: Option<Window>,
    source: VisualSceneOverlaySource,
) -> anyhow::Result<()> {
    term.no_grab_mouse_in_raw_mode();
    term.set_raw_mode()?;
    term.render(&[Change::Title("GameTerm Scene".to_string())])?;
    let pane_id = term
        .pane_id()
        .context("Scene Mode terminal is not attached to a mux pane")?;
    let _active_scene_overlay = ActiveSceneOverlay::new(pane_id);

    let mut scene_path = default_scene_path();
    let sprite_manifest_path = default_sprite_manifest_path();
    let mut sprite_manifest;
    let mut load_error;
    let mut runtime;
    let mut reload_count = 1;
    let (command_tx, command_rx) = mpsc::channel();
    let generated_scene = match source {
        VisualSceneOverlaySource::Default => {
            (runtime, sprite_manifest, load_error) =
                initial_scene_state(&mut term, &scene_path, &sprite_manifest_path, reload_count)?;
            None
        }
        VisualSceneOverlaySource::Generated {
            scene,
            action_base_dir,
            source_label,
        } => {
            (runtime, sprite_manifest, load_error) = initial_generated_scene_state(
                &mut term,
                scene.clone(),
                source_label.clone(),
                &sprite_manifest_path,
                action_base_dir.clone(),
                reload_count,
            )?;
            scene_path = PathBuf::from(source_label.as_str());
            Some((scene, source_label, action_base_dir))
        }
    };
    let mut file_watcher = if generated_scene.is_some() {
        SceneFileWatcher::disabled()
    } else {
        SceneFileWatcher::from_env(&scene_path, &sprite_manifest_path)
    };
    let mut patch_inbox = ScenePatchInbox::from_env();
    let (scene_patch_tx, scene_patch_rx) = mpsc::channel();
    let _scene_patch_subscription =
        ScenePatchNotificationSubscription::new(pane_id, route_pane_id, scene_patch_tx);
    let mut compose_dock = SceneComposeDock::default();
    let (compose_tx, compose_rx) = mpsc::channel();
    let mut compose_backend_running = false;
    let (tts_tx, tts_rx) = mpsc::channel();
    let mut tts_state = SceneTtsState::default();
    let (stt_tx, stt_rx) = mpsc::channel();
    let mut stt_state = SceneSttState::default();
    let mut stt_cancel: Option<SceneSttCancel> = None;

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
        while let Ok(result) = compose_rx.try_recv() {
            compose_backend_running = false;
            if let Some(runtime) = runtime.as_mut() {
                let speakable_segments = apply_compose_backend_result(runtime, result);
                if !tts_state.is_muted() {
                    for segment in speakable_segments {
                        spawn_tts_backend(SceneTtsRequest { segment }, tts_tx.clone());
                    }
                }
                needs_render = true;
            }
        }
        while let Ok(result) = tts_rx.try_recv() {
            if let Some(runtime) = runtime.as_mut() {
                apply_tts_result(runtime, &mut tts_state, result);
                needs_render = true;
            }
        }
        while let Ok(result) = stt_rx.try_recv() {
            stt_cancel = None;
            if let Some(runtime) = runtime.as_mut() {
                apply_stt_result(
                    runtime,
                    &mut compose_dock,
                    &mut stt_state,
                    result,
                    &mut compose_backend_running,
                    &compose_tx,
                    &scene_path,
                    pane_id,
                );
                needs_render = true;
            }
        }
        if needs_render {
            if let Some(runtime) = runtime.as_ref() {
                render_runtime_with_compose(&mut term, runtime, &sprite_manifest, &compose_dock)?;
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
            while let Ok(scene_patch) = scene_patch_rx.try_recv() {
                if let Some(runtime) = runtime.as_mut() {
                    apply_scene_patch_json(
                        &mut term,
                        runtime,
                        &sprite_manifest,
                        &scene_patch.patch_json,
                        scene_patch.source_pane_id,
                    )?;
                }
            }
            continue;
        };
        match input {
            InputEvent::Key(KeyEvent { key, modifiers }) => {
                if let Some(runtime) = runtime.as_mut() {
                    if is_tts_toggle_key(key, modifiers) {
                        runtime.mark_action_status(tts_state.toggle_muted());
                        render_runtime_with_compose(
                            &mut term,
                            runtime,
                            &sprite_manifest,
                            &compose_dock,
                        )?;
                        continue;
                    }
                    if is_stt_toggle_key(key, modifiers) {
                        if stt_state.is_running() {
                            if let Some(cancel) = stt_cancel.take() {
                                cancel.cancel();
                            }
                            runtime.mark_action_status(stt_state.mark_canceling());
                        } else {
                            runtime.mark_action_status(stt_state.mark_started());
                            stt_cancel = Some(spawn_stt_backend(stt_tx.clone()));
                        }
                        render_runtime_with_compose(
                            &mut term,
                            runtime,
                            &sprite_manifest,
                            &compose_dock,
                        )?;
                        continue;
                    }
                    // While the VN layout debugger menu is open it owns every
                    // key. The compose dock must not intercept text/navigation
                    // input before the debugger can select, adjust, or edit.
                    let in_layout_debug = runtime.view() == VisualView::VnLayoutDebugger;
                    if !runtime.scene().stage.is_empty() && !in_layout_debug {
                        match compose_dock.handle_key(key) {
                            SceneComposeAction::Consumed => {
                                render_runtime_with_compose(
                                    &mut term,
                                    runtime,
                                    &sprite_manifest,
                                    &compose_dock,
                                )?;
                                continue;
                            }
                            SceneComposeAction::Submitted(prompt) => {
                                if compose_backend_running {
                                    runtime
                                        .mark_compose_failed("Compose backend is already running");
                                    runtime.mark_action_status(
                                        "Compose busy: finish the current reply first",
                                    );
                                    render_runtime_with_compose(
                                        &mut term,
                                        runtime,
                                        &sprite_manifest,
                                        &compose_dock,
                                    )?;
                                    continue;
                                }
                                compose_dock.mark_submitted(&prompt);
                                runtime
                                    .mark_compose_running(compose_running_status(&prompt), &prompt);
                                compose_backend_running = true;
                                spawn_compose_backend(
                                    ComposeBackendRequest {
                                        prompt,
                                        scene_path: Some(scene_path.display().to_string()),
                                        pane_id: Some(pane_id),
                                    },
                                    compose_tx.clone(),
                                );
                                render_runtime_with_compose(
                                    &mut term,
                                    runtime,
                                    &sprite_manifest,
                                    &compose_dock,
                                )?;
                                continue;
                            }
                            SceneComposeAction::Fallthrough => {}
                        }
                    }
                }
                let visual_input = visual_input_from_key(key);
                // While the VN layout debugger menu is open it owns every key so
                // that r resets values and esc cancels an edit instead of
                // reloading or closing the whole overlay.
                let in_layout_debug = runtime
                    .as_ref()
                    .map_or(false, |runtime| runtime.view() == VisualView::VnLayoutDebugger);
                if visual_input == VisualInput::Close && !in_layout_debug {
                    break;
                }
                if visual_input == VisualInput::Reload && !in_layout_debug {
                    if let Some((scene, source_label, action_base_dir)) = &generated_scene {
                        reload_generated_scene(
                            &mut term,
                            scene.clone(),
                            source_label,
                            &sprite_manifest_path,
                            action_base_dir,
                            &mut reload_count,
                            &mut runtime,
                            &mut sprite_manifest,
                            &mut load_error,
                        )?;
                    } else {
                        reload_active_scene(
                            &mut term,
                            &scene_path,
                            &sprite_manifest_path,
                            &mut reload_count,
                            &mut runtime,
                            &mut sprite_manifest,
                            &mut load_error,
                        )?;
                    }
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
                            pane_id: route_pane_id.or_else(|| term.pane_id()),
                            terminal_size: TerminalSize {
                                rows: size.rows,
                                cols: size.cols,
                                pixel_width: size.xpixel.saturating_mul(size.cols),
                                pixel_height: size.ypixel.saturating_mul(size.rows),
                                dpi: 0,
                            },
                            gui_window: gui_window.clone(),
                            command_tx: command_tx.clone(),
                        },
                    )?;
                    file_watcher.refresh(&scene_path, &sprite_manifest_path);
                    patch_inbox.refresh();
                    render_runtime_with_compose(
                        &mut term,
                        runtime,
                        &sprite_manifest,
                        &compose_dock,
                    )?;
                }
            }
            InputEvent::Resized { .. } => {
                if let Some(runtime) = runtime.as_ref() {
                    render_runtime_with_compose(
                        &mut term,
                        runtime,
                        &sprite_manifest,
                        &compose_dock,
                    )?;
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

struct ScenePatchNotificationSubscription {
    dead: Arc<AtomicBool>,
}

struct ScenePatchNotification {
    patch_json: String,
    source_pane_id: Option<mux::pane::PaneId>,
}

impl ScenePatchNotificationSubscription {
    fn new(
        pane_id: mux::pane::PaneId,
        route_pane_id: Option<mux::pane::PaneId>,
        scene_patch_tx: mpsc::Sender<ScenePatchNotification>,
    ) -> Self {
        let dead = Arc::new(AtomicBool::new(false));
        let subscription_dead = Arc::clone(&dead);
        Mux::get().subscribe(move |notification| {
            if subscription_dead.load(Ordering::Relaxed) {
                return false;
            }
            if let MuxNotification::GameTermScenePatch {
                patch_json,
                target_pane_id,
                source_pane_id,
            } = notification
            {
                if !scene_patch_target_matches(
                    target_pane_id,
                    Mux::get().active_gameterm_scene_pane(),
                    pane_id,
                    route_pane_id,
                ) {
                    return true;
                }
                let _ = scene_patch_tx.send(ScenePatchNotification {
                    patch_json,
                    source_pane_id,
                });
            }
            true
        });
        Self { dead }
    }
}

fn scene_patch_target_matches(
    target_pane_id: Option<mux::pane::PaneId>,
    active_pane_id: Option<mux::pane::PaneId>,
    overlay_pane_id: mux::pane::PaneId,
    route_pane_id: Option<mux::pane::PaneId>,
) -> bool {
    let target_pane_id = target_pane_id.or(active_pane_id);
    target_pane_id == Some(overlay_pane_id) || target_pane_id == route_pane_id
}

struct ActiveSceneOverlay {
    pane_id: mux::pane::PaneId,
}

impl ActiveSceneOverlay {
    fn new(pane_id: mux::pane::PaneId) -> Self {
        Mux::get().set_active_gameterm_scene_pane(pane_id);
        Self { pane_id }
    }
}

impl Drop for ActiveSceneOverlay {
    fn drop(&mut self) {
        Mux::get().clear_active_gameterm_scene_pane(self.pane_id);
    }
}

impl Drop for ScenePatchNotificationSubscription {
    fn drop(&mut self) {
        self.dead.store(true, Ordering::Relaxed);
    }
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

fn reload_generated_scene(
    term: &mut TermWizTerminal,
    scene: VisualScene,
    source_label: &str,
    sprite_manifest_path: &PathBuf,
    action_base_dir: &Path,
    reload_count: &mut u64,
    runtime: &mut Option<SceneRuntime>,
    sprite_manifest: &mut VisualSpriteManifestStatus,
    load_error: &mut Option<String>,
) -> anyhow::Result<()> {
    *reload_count = reload_count.saturating_add(1);
    *sprite_manifest = load_sprite_manifest_status(sprite_manifest_path);
    let source = VisualSceneSource::new(
        source_label.to_string(),
        VisualSceneLoadStatus::Loaded,
        *reload_count,
    );
    if let Some(runtime) = runtime.as_mut() {
        runtime.replace_scene_preserving_state(scene, source)?;
        render_runtime(term, runtime, sprite_manifest)?;
    } else {
        let loaded =
            SceneRuntime::new_with_source_and_action_base_dir(scene, source, action_base_dir)?;
        render_runtime(term, &loaded, sprite_manifest)?;
        *runtime = Some(loaded);
    }
    *load_error = None;
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
    match VisualScenePatch::load_from_path(path).and_then(|patch| {
        runtime.apply_scene_patch_with_source(patch, Some("file".to_string()), None)
    }) {
        Ok(()) => {}
        Err(err) => {
            runtime.mark_scene_patch_failed(format!("file {}", path.display()), None, err);
        }
    }
    render_runtime(term, runtime, sprite_manifest)
}

fn apply_scene_patch_json(
    term: &mut TermWizTerminal,
    runtime: &mut SceneRuntime,
    sprite_manifest: &VisualSpriteManifestStatus,
    patch_json: &str,
    source_pane_id: Option<mux::pane::PaneId>,
) -> anyhow::Result<()> {
    match VisualScenePatch::from_json(patch_json).and_then(|patch| {
        runtime.apply_scene_patch_with_source(patch, Some("mux".to_string()), source_pane_id)
    }) {
        Ok(()) => {}
        Err(err) => {
            runtime.mark_scene_patch_failed("mux", source_pane_id, err);
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
    gui_window: Option<Window>,
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
        VisualActionRequest::ExportStoryState { path } => match runtime.story_state_json_pretty() {
            Ok(json) => match write_story_state_file(&path, &json) {
                Ok(()) => runtime.mark_story_state_exported(&path),
                Err(err) => runtime.mark_story_state_failed("export", &path, err),
            },
            Err(err) => runtime.mark_story_state_failed("export", &path, err),
        },
        VisualActionRequest::ImportStoryState { path } => {
            match VisualStoryState::load_from_path(&path) {
                Ok(state) => match runtime.import_story_state(state) {
                    Ok(()) => runtime.mark_story_state_imported(&path),
                    Err(err) => runtime.mark_story_state_failed("import", &path, err),
                },
                Err(err) => runtime.mark_story_state_failed("import", &path, err),
            }
        }
    }
    Ok(())
}

fn write_story_state_file(path: &Path, json: &str) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, format!("{json}\n"))?;
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
    let Some(gui_window) = dispatch.gui_window.clone() else {
        let _ = dispatch.command_tx.send(RunCommandResult::Failed {
            argv,
            target,
            error: "Scene Mode RunCommand dispatch is not attached to a GUI window".to_string(),
        });
        return;
    };

    gui_window.notify(TermWindowNotif::Apply(Box::new(move |_term_window| {
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
    })));
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

fn initial_generated_scene_state(
    term: &mut TermWizTerminal,
    scene: VisualScene,
    source_label: String,
    sprite_manifest_path: &PathBuf,
    action_base_dir: PathBuf,
    reload_count: u64,
) -> anyhow::Result<(
    Option<SceneRuntime>,
    VisualSpriteManifestStatus,
    Option<String>,
)> {
    let sprite_manifest = load_sprite_manifest_status(sprite_manifest_path);
    let source = VisualSceneSource::new(source_label, VisualSceneLoadStatus::Loaded, reload_count);
    match SceneRuntime::new_with_source_and_action_base_dir(scene, source.clone(), action_base_dir)
    {
        Ok(runtime) => {
            render_runtime(term, &runtime, &sprite_manifest)?;
            Ok((Some(runtime), sprite_manifest, None))
        }
        Err(err) => {
            let error = err.to_string();
            let source = VisualSceneSource::invalid(source.scene_path, reload_count, error.clone());
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
        KeyCode::DownArrow | KeyCode::Char('j') | KeyCode::Char('J') => VisualInput::Next,
        KeyCode::UpArrow | KeyCode::Char('k') | KeyCode::Char('K') => VisualInput::Previous,
        KeyCode::RightArrow | KeyCode::Char('l') | KeyCode::Char('L') => VisualInput::Right,
        KeyCode::LeftArrow | KeyCode::Char('h') | KeyCode::Char('H') => VisualInput::Left,
        KeyCode::Backspace => VisualInput::Backspace,
        KeyCode::Char(c) => VisualInput::Char(c),
        _ => VisualInput::Other,
    }
}

fn is_tts_toggle_key(key: KeyCode, modifiers: Modifiers) -> bool {
    matches!(key, KeyCode::Char('m') | KeyCode::Char('M')) && modifiers.contains(Modifiers::ALT)
}

fn is_stt_toggle_key(key: KeyCode, modifiers: Modifiers) -> bool {
    matches!(key, KeyCode::Char('v') | KeyCode::Char('V')) && modifiers.contains(Modifiers::ALT)
}

const COMPOSE_OUTPUT_LIMIT: usize = 1200;

#[derive(Debug, Clone, PartialEq, Eq)]
enum StructuredComposeOutcome {
    NoReply,
    WithReply {
        speaker: String,
        dialogue_text: String,
    },
}

fn apply_compose_backend_result(
    runtime: &mut SceneRuntime,
    result: ComposeBackendResult,
) -> Vec<SpeakableSegment> {
    if result.succeeded() {
        match apply_structured_compose_backend_result(runtime, &result) {
            Some(StructuredComposeOutcome::WithReply {
                speaker,
                dialogue_text,
            }) => {
                runtime.mark_compose_succeeded(&speaker, &dialogue_text);
                return extract_speakable_segments(
                    Some(&speaker),
                    &dialogue_text,
                    SpeakableSource::ComposeReply,
                );
            }
            Some(StructuredComposeOutcome::NoReply) => {
                runtime.mark_compose_succeeded("Scene", "");
                return Vec::new();
            }
            None => {}
        }

        let reply = sanitize_compose_output(&result.stdout);
        let reply = if reply.is_empty() {
            "The compose backend returned no output.".to_string()
        } else {
            reply
        };
        let patch = VisualScenePatch {
            scene_patch_version: VisualScenePatch::VERSION,
            updates: vec![],
            variables: vec![],
            selected_entity_id: None,
            process_state: None,
            dialogue: Some(VisualSceneDialoguePatch {
                speaker: "Codex".to_string(),
                text: reply.clone(),
                append_history: true,
            }),
            status: Some(result.label.succeeded_status().to_string()),
        };
        if let Err(err) = runtime.apply_scene_patch(patch) {
            runtime.mark_action_status(format!("Compose reply failed: {err}"));
        } else {
            runtime.mark_compose_succeeded("Codex", &reply);
            return extract_speakable_segments(
                Some("Codex"),
                &reply,
                SpeakableSource::ComposeReply,
            );
        }
        return Vec::new();
    }

    apply_compose_backend_failure_result(runtime, &result);
    Vec::new()
}

fn apply_tts_result(
    runtime: &mut SceneRuntime,
    tts_state: &mut SceneTtsState,
    result: SceneTtsResult,
) {
    let status = tts_state.apply_result(&result);
    if result.succeeded() {
        runtime.mark_action_status(status);
    } else if let Some(error) = result.error {
        runtime.mark_action_status(format!("{status}: {error}"));
    } else {
        runtime.mark_action_status(status);
    }
}

fn apply_stt_result(
    runtime: &mut SceneRuntime,
    compose_dock: &mut SceneComposeDock,
    stt_state: &mut SceneSttState,
    result: SceneSttResult,
    compose_backend_running: &mut bool,
    compose_tx: &mpsc::Sender<ComposeBackendResult>,
    scene_path: &Path,
    pane_id: mux::pane::PaneId,
) {
    let status = stt_state.apply_result(&result);
    let succeeded = result.succeeded();
    if succeeded {
        let Some(transcript) = result.transcript else {
            return;
        };
        compose_dock.insert_transcript(&transcript);
        runtime.mark_action_status(status);
        if result.auto_submit {
            let prompt = compose_dock.buffer.trim().to_string();
            if prompt.is_empty() {
                return;
            }
            if *compose_backend_running {
                runtime.mark_action_status("Voice transcript ready: compose busy");
                return;
            }
            compose_dock.mark_submitted(&prompt);
            runtime.mark_compose_running(compose_running_status(&prompt), &prompt);
            *compose_backend_running = true;
            spawn_compose_backend(
                ComposeBackendRequest {
                    prompt,
                    scene_path: Some(scene_path.display().to_string()),
                    pane_id: Some(pane_id),
                },
                compose_tx.clone(),
            );
        }
    } else if let Some(error) = result.error {
        runtime.mark_action_status(format!("{status}: {error}"));
    } else {
        runtime.mark_action_status(status);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct StructuredComposePayload {
    #[serde(default)]
    speaker: Option<String>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    append_history: Option<bool>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    patch: Option<serde_json::Value>,
}

impl StructuredComposePayload {
    fn meaningful(&self) -> bool {
        self.speaker.is_some()
            || self
                .text
                .as_ref()
                .is_some_and(|text| !text.trim().is_empty())
            || self.append_history.is_some()
            || self.status.is_some()
            || self.patch.is_some()
    }

    fn text_or_default(&self) -> String {
        sanitize_compose_output(self.text.as_deref().unwrap_or(""))
    }

    fn dialogue_text_is_present(&self) -> bool {
        self.text
            .as_ref()
            .is_some_and(|text| !text.trim().is_empty())
    }
}

fn parse_structured_payload(raw: &str) -> Option<StructuredComposePayload> {
    serde_json::from_str::<StructuredComposePayload>(raw)
        .ok()
        .filter(|payload| payload.meaningful())
}

fn parse_structured_compose_payload(raw: &str) -> Option<StructuredComposePayload> {
    parse_structured_payload(raw.trim()).or_else(|| {
        raw.lines()
            .rev()
            .find_map(|line| parse_structured_payload(line.trim()))
    })
}

fn parse_structured_compose_patch(value: serde_json::Value) -> Result<VisualScenePatch, String> {
    let patch: VisualScenePatch = serde_json::from_value(value)
        .map_err(|err| format!("invalid compose patch json: {err}"))?;
    patch
        .validate()
        .map_err(|err| format!("compose patch invalid: {err}"))?;
    Ok(patch)
}

fn apply_structured_compose_backend_result(
    runtime: &mut SceneRuntime,
    result: &ComposeBackendResult,
) -> Option<StructuredComposeOutcome> {
    let mut payload = match parse_structured_compose_payload(&result.stdout) {
        Some(payload) => payload,
        None => return None,
    };

    let mut patch = match payload.patch.take() {
        Some(value) => match parse_structured_compose_patch(value) {
            Ok(patch) => patch,
            Err(err) => {
                runtime.mark_action_status(format!("Compose patch parse failed: {err}"));
                return None;
            }
        },
        None => VisualScenePatch {
            scene_patch_version: VisualScenePatch::VERSION,
            updates: vec![],
            variables: vec![],
            selected_entity_id: None,
            process_state: None,
            dialogue: None,
            status: None,
        },
    };

    let mut applied_dialogue = patch.dialogue.clone();

    if patch.dialogue.is_none() && payload.dialogue_text_is_present() {
        let speaker = payload
            .speaker
            .take()
            .filter(|speaker| !speaker.trim().is_empty())
            .unwrap_or_else(|| "Codex".to_string());

        let dialogue = VisualSceneDialoguePatch {
            speaker,
            text: payload.text_or_default(),
            append_history: payload.append_history.unwrap_or(true),
        };
        applied_dialogue = Some(dialogue.clone());
        patch.dialogue = Some(dialogue);
    }

    if patch.status.is_none() {
        patch.status = Some(
            payload
                .status
                .unwrap_or_else(|| result.label.succeeded_status().to_string()),
        );
    }

    if let Err(err) = runtime.apply_scene_patch(patch) {
        runtime.mark_action_status(format!("Compose patch failed: {err}"));
        return None;
    }

    if applied_dialogue.is_some() {
        return applied_dialogue
            .take()
            .map(|dialogue| StructuredComposeOutcome::WithReply {
                speaker: dialogue.speaker,
                dialogue_text: dialogue.text,
            });
    }

    Some(StructuredComposeOutcome::NoReply)
}

fn apply_compose_backend_failure_result(runtime: &mut SceneRuntime, result: &ComposeBackendResult) {
    let diagnostic = sanitize_compose_output(&result.stderr);
    let diagnostic = if diagnostic.is_empty() {
        format!("Compose backend failed for: {}", result.prompt)
    } else {
        diagnostic
    };
    let marked_diagnostic = diagnostic.clone();
    let patch = VisualScenePatch {
        scene_patch_version: VisualScenePatch::VERSION,
        updates: vec![],
        variables: vec![],
        selected_entity_id: None,
        process_state: None,
        dialogue: Some(VisualSceneDialoguePatch {
            speaker: "Scene".to_string(),
            text: diagnostic,
            append_history: true,
        }),
        status: Some(result.label.failed_status().to_string()),
    };
    if let Err(err) = runtime.apply_scene_patch(patch) {
        runtime.mark_action_status(format!("Compose reply failed: {err}"));
    } else {
        runtime.mark_compose_failed(&marked_diagnostic);
    }
}

fn sanitize_compose_output(output: &str) -> String {
    let mut sanitized = String::new();
    let mut blank_pending = false;
    for ch in output.chars() {
        if sanitized.chars().count() >= COMPOSE_OUTPUT_LIMIT {
            break;
        }
        match ch {
            '\n' | '\r' => {
                if !blank_pending {
                    sanitized.push('\n');
                    blank_pending = true;
                }
            }
            '\t' => {
                sanitized.push(' ');
                blank_pending = false;
            }
            ch if ch.is_control() => {}
            ch => {
                sanitized.push(ch);
                blank_pending = false;
            }
        }
    }
    sanitized.trim().to_string()
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct SceneComposeDock {
    buffer: String,
    cursor: usize,
    last_submitted: Option<String>,
    history: Vec<String>,
    history_index: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SceneComposeAction {
    Consumed,
    Submitted(String),
    Fallthrough,
}

impl SceneComposeDock {
    fn handle_key(&mut self, key: KeyCode) -> SceneComposeAction {
        match key {
            KeyCode::Backspace => {
                self.remove_before_cursor();
                SceneComposeAction::Consumed
            }
            KeyCode::Delete => {
                if self.buffer.is_empty() {
                    SceneComposeAction::Fallthrough
                } else {
                    self.clear_buffer();
                    SceneComposeAction::Consumed
                }
            }
            KeyCode::LeftArrow => {
                self.cursor = self.cursor.saturating_sub(1);
                SceneComposeAction::Consumed
            }
            KeyCode::RightArrow => {
                self.cursor = (self.cursor + 1).min(self.buffer_char_len());
                SceneComposeAction::Consumed
            }
            KeyCode::Home => {
                self.cursor = 0;
                SceneComposeAction::Consumed
            }
            KeyCode::End => {
                self.cursor = self.buffer_char_len();
                SceneComposeAction::Consumed
            }
            KeyCode::UpArrow => {
                self.recall_previous_history();
                SceneComposeAction::Consumed
            }
            KeyCode::DownArrow => {
                self.recall_next_history();
                SceneComposeAction::Consumed
            }
            KeyCode::Enter => {
                let submitted = self.buffer.trim().to_string();
                if submitted.is_empty() {
                    SceneComposeAction::Fallthrough
                } else {
                    SceneComposeAction::Submitted(submitted)
                }
            }
            KeyCode::Char(ch) if is_compose_char(ch) => {
                self.insert_char(ch);
                SceneComposeAction::Consumed
            }
            _ => SceneComposeAction::Fallthrough,
        }
    }

    fn mark_submitted(&mut self, prompt: &str) {
        self.last_submitted = Some(prompt.to_string());
        self.history.push(prompt.to_string());
        if self.history.len() > 20 {
            self.history.remove(0);
        }
        self.clear_buffer();
    }

    fn insert_transcript(&mut self, transcript: &str) {
        let transcript = transcript.trim();
        if transcript.is_empty() {
            return;
        }
        if !self.buffer.is_empty()
            && self.cursor == self.buffer_char_len()
            && !self.buffer.chars().last().is_some_and(char::is_whitespace)
        {
            self.insert_char(' ');
        }
        for ch in transcript.chars().filter(|ch| is_compose_char(*ch)) {
            self.insert_char(ch);
        }
    }

    fn render_line(&self, cols: usize) -> String {
        let mut line = String::from(" Compose: ");
        line.push_str(&self.buffer_with_cursor());
        if self.buffer.is_empty() {
            if let Some(last_submitted) = &self.last_submitted {
                line.push_str("  last: ");
                line.push_str(last_submitted);
            } else {
                line.push_str("  type here; enter submits");
            }
        }
        clip_text(&line, cols.max(1))
    }

    fn render_staged_dock_line(&self, cols: usize, rect: VnOverlayRect) -> String {
        let mut line = String::from(" ");
        line.push_str(&self.buffer_with_cursor());
        if self.buffer.is_empty() {
            if let Some(last_submitted) = &self.last_submitted {
                line.push_str(" last: ");
                line.push_str(last_submitted);
            } else {
                line.push_str(" type here; enter submits");
            }
        }
        let content_width = rect.width.min(cols.saturating_sub(rect.col)).max(1);
        let indent = " ".repeat(rect.col.min(cols.saturating_sub(1)));
        format!(
            "{indent}{:<content_width$}",
            clip_text(&line, content_width)
        )
    }

    fn render_staged_nameplate_line(&self, cols: usize, rect: VnOverlayRect) -> String {
        let content_width = rect.width.min(cols.saturating_sub(rect.col)).max(1);
        let indent = " ".repeat(rect.col.min(cols.saturating_sub(1)));
        let label = format!("{:<content_width$}", clip_text("Composer", content_width));
        format!(
            "{indent}{:<content_width$}",
            clip_text(&label, content_width)
        )
    }

    fn insert_char(&mut self, ch: char) {
        let byte_idx = self.cursor_byte_idx();
        self.buffer.insert(byte_idx, ch);
        self.cursor += 1;
        self.history_index = None;
    }

    fn remove_before_cursor(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let start = self.cursor_to_byte_idx(self.cursor - 1);
        let end = self.cursor_byte_idx();
        self.buffer.replace_range(start..end, "");
        self.cursor -= 1;
        self.history_index = None;
    }

    fn clear_buffer(&mut self) {
        self.buffer.clear();
        self.cursor = 0;
        self.history_index = None;
    }

    fn recall_previous_history(&mut self) {
        if self.history.is_empty() {
            return;
        }
        let next_index = match self.history_index {
            Some(index) => index.saturating_sub(1),
            None => self.history.len() - 1,
        };
        self.set_history_index(next_index);
    }

    fn recall_next_history(&mut self) {
        let Some(index) = self.history_index else {
            return;
        };
        if index + 1 >= self.history.len() {
            self.clear_buffer();
        } else {
            self.set_history_index(index + 1);
        }
    }

    fn set_history_index(&mut self, index: usize) {
        self.history_index = Some(index);
        self.buffer = self.history[index].clone();
        self.cursor = self.buffer_char_len();
    }

    fn buffer_with_cursor(&self) -> String {
        let mut out = String::new();
        for (idx, ch) in self.buffer.chars().enumerate() {
            if idx == self.cursor {
                out.push('_');
            }
            out.push(ch);
        }
        if self.cursor >= self.buffer_char_len() {
            out.push('_');
        }
        out
    }

    fn cursor_byte_idx(&self) -> usize {
        self.cursor_to_byte_idx(self.cursor)
    }

    fn cursor_to_byte_idx(&self, cursor: usize) -> usize {
        self.buffer
            .char_indices()
            .nth(cursor)
            .map(|(idx, _)| idx)
            .unwrap_or(self.buffer.len())
    }

    fn buffer_char_len(&self) -> usize {
        self.buffer.chars().count()
    }
}

fn is_compose_char(ch: char) -> bool {
    !ch.is_control()
}

fn render_runtime(
    term: &mut TermWizTerminal,
    runtime: &SceneRuntime,
    sprite_manifest: &VisualSpriteManifestStatus,
) -> anyhow::Result<()> {
    render_runtime_with_compose(term, runtime, sprite_manifest, &SceneComposeDock::default())
}

fn render_runtime_with_compose(
    term: &mut TermWizTerminal,
    runtime: &SceneRuntime,
    sprite_manifest: &VisualSpriteManifestStatus,
    compose_dock: &SceneComposeDock,
) -> anyhow::Result<()> {
    let size = term.get_screen_size()?;
    let mut snapshot = runtime.render_snapshot();
    snapshot.overlay_cols = Some(size.cols);
    snapshot.overlay_rows = Some(size.rows);
    term.set_metadata(
        "gameterm_visual_snapshot",
        Value::String(serde_json::to_string(&snapshot)?),
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
    if !snapshot.stage.is_empty() {
        let layout = match snapshot.vn_layout_debug.as_ref() {
            Some(overrides) => vn_overlay_layout_with_overrides(size.cols, size.rows, &snapshot.dialogue_speaker, "Composer", overrides),
            None => vn_overlay_layout(size.cols, size.rows, &snapshot.dialogue_speaker, "Composer"),
        };
        if let Some(nameplate) = layout.composer_nameplate {
            frame = replace_screen_line(
                frame,
                size.cols,
                size.rows,
                nameplate.row.saturating_add(nameplate.height.saturating_sub(1)).min(size.rows.saturating_sub(1)),
                &compose_dock.render_staged_nameplate_line(size.cols, nameplate),
            );
        }
        if let (Some(panel), Some(text_row)) = (layout.composer_panel, layout.composer_text_row) {
            let input_rect = VnOverlayRect {
                col: panel.col.saturating_add(layout.composer_text_inset_cols),
                row: text_row,
                width: panel
                    .width
                    .saturating_sub(layout.composer_text_inset_cols * 2),
                height: 1,
            };
            frame = replace_screen_line(
                frame,
                size.cols,
                size.rows,
                text_row,
                &compose_dock.render_staged_dock_line(size.cols, input_rect),
            );
        }
    } else {
        frame = replace_last_screen_line(
            frame,
            size.cols,
            size.rows,
            &compose_dock.render_line(size.cols),
        );
    }
    term.render(&[
        Change::ClearScreen(ColorAttribute::Default),
        Change::Text(truncate_to_screen(frame, size.cols, size.rows)),
    ])?;
    term.flush()?;
    Ok(())
}

fn replace_last_screen_line(frame: String, cols: usize, rows: usize, replacement: &str) -> String {
    let rows = rows.max(1);
    replace_screen_line(frame, cols, rows, rows - 1, replacement)
}

fn replace_screen_line(
    frame: String,
    cols: usize,
    rows: usize,
    target_row: usize,
    replacement: &str,
) -> String {
    let rows = rows.max(1);
    let mut lines = frame.lines().map(str::to_string).collect::<Vec<_>>();
    while lines.len() < rows {
        lines.push(String::new());
    }
    lines.truncate(rows);
    lines[target_row.min(rows - 1)] = clip_text(replacement, cols.max(1));
    let mut out = lines.join("\r\n");
    out.push_str("\r\n");
    out
}

fn clip_text(text: &str, max_chars: usize) -> String {
    text.chars().take(max_chars).collect()
}

#[cfg(test)]
mod tests {
    use super::super::visual_compose::{
        codex_compose_argv, codex_output_text, compose_backend_config, run_codex_compose_backend,
        CodexComposeConfig, ComposeBackendConfig,
    };
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
            policy: None,
            conditions: vec![],
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
                gui_window: None,
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
    fn dispatch_export_story_state_writes_state_file() {
        let dir = tempfile::tempdir().unwrap();
        let state_path = dir.path().join("story/state.json");
        let mut scene = VisualScene::demo();
        scene.choices = vec![gameterm_visual::SceneAction {
            label: "Export story".to_string(),
            kind: gameterm_visual::SceneActionKind::ExportStoryState {
                path: "story/state.json".to_string(),
            },
            policy: None,
            conditions: vec![],
        }];
        let mut runtime = SceneRuntime::new_with_source_and_action_base_dir(
            scene,
            VisualSceneSource::new(
                dir.path().join("default.json").display().to_string(),
                VisualSceneLoadStatus::Loaded,
                1,
            ),
            dir.path(),
        )
        .unwrap();
        let mut active_path = dir.path().join("default.json");
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
                gui_window: None,
                command_tx,
            },
        )
        .unwrap();

        assert!(state_path.is_file());
        assert_eq!(
            runtime.debug_report().last_story_state_action.as_deref(),
            Some("export")
        );
        assert!(runtime
            .render_snapshot()
            .status
            .starts_with("Story state exported: "));
    }

    #[test]
    fn dispatch_import_story_state_restores_state_file() {
        let dir = tempfile::tempdir().unwrap();
        let state_path = dir.path().join("story/state.json");
        let state = gameterm_visual::VisualStoryState {
            story_state_version: gameterm_visual::VisualStoryState::VERSION,
            variables: vec![gameterm_visual::VisualStateEntry {
                key: "loaded_from_gui".to_string(),
                value: gameterm_visual::VisualStateValue::Bool(true),
            }],
            rpg: gameterm_visual::VisualRpgState::default(),
            dialogue_index: None,
            dialogue_history: vec![],
        };
        write_story_state_file(&state_path, &serde_json::to_string_pretty(&state).unwrap())
            .unwrap();
        let mut scene = VisualScene::demo();
        scene.choices = vec![gameterm_visual::SceneAction {
            label: "Import story".to_string(),
            kind: gameterm_visual::SceneActionKind::ImportStoryState {
                path: "story/state.json".to_string(),
            },
            policy: None,
            conditions: vec![],
        }];
        let mut runtime = SceneRuntime::new_with_source_and_action_base_dir(
            scene,
            VisualSceneSource::new(
                dir.path().join("default.json").display().to_string(),
                VisualSceneLoadStatus::Loaded,
                1,
            ),
            dir.path(),
        )
        .unwrap();
        let mut active_path = dir.path().join("default.json");
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
                gui_window: None,
                command_tx,
            },
        )
        .unwrap();

        let report = runtime.debug_report();
        assert_eq!(report.last_story_state_action.as_deref(), Some("import"));
        assert!(report
            .variables
            .contains(&gameterm_visual::VisualStateEntry {
                key: "loaded_from_gui".to_string(),
                value: gameterm_visual::VisualStateValue::Bool(true),
            }));
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

    #[test]
    fn scene_patch_target_matches_overlay_and_route_panes() {
        assert!(scene_patch_target_matches(Some(7), None, 7, Some(3)));
        assert!(scene_patch_target_matches(Some(3), None, 7, Some(3)));
        assert!(scene_patch_target_matches(None, Some(7), 7, Some(3)));
        assert!(!scene_patch_target_matches(Some(4), None, 7, Some(3)));
        assert!(!scene_patch_target_matches(None, Some(4), 7, Some(3)));
    }

    #[test]
    fn scene_compose_dock_edits_and_submits_buffer() {
        let mut dock = SceneComposeDock::default();

        assert_eq!(
            dock.handle_key(KeyCode::Char('h')),
            SceneComposeAction::Consumed
        );
        assert_eq!(
            dock.handle_key(KeyCode::Char('i')),
            SceneComposeAction::Consumed
        );
        assert_eq!(dock.buffer, "hi");
        assert_eq!(dock.cursor, 2);
        assert_eq!(
            dock.handle_key(KeyCode::Backspace),
            SceneComposeAction::Consumed
        );
        assert_eq!(dock.buffer, "h");
        assert_eq!(
            dock.handle_key(KeyCode::Enter),
            SceneComposeAction::Submitted("h".to_string())
        );
        dock.mark_submitted("h");

        assert!(dock.buffer.is_empty());
        assert_eq!(dock.last_submitted.as_deref(), Some("h"));
    }

    #[test]
    fn scene_compose_dock_empty_enter_falls_through() {
        let mut dock = SceneComposeDock::default();

        assert_eq!(
            dock.handle_key(KeyCode::Enter),
            SceneComposeAction::Fallthrough
        );
    }

    #[test]
    fn scene_compose_dock_moves_cursor_and_recalls_history() {
        let mut dock = SceneComposeDock::default();
        dock.handle_key(KeyCode::Char('a'));
        dock.handle_key(KeyCode::Char('c'));
        dock.handle_key(KeyCode::LeftArrow);
        dock.handle_key(KeyCode::Char('b'));

        assert_eq!(dock.buffer, "abc");
        assert_eq!(dock.cursor, 2);

        dock.mark_submitted("abc");
        dock.handle_key(KeyCode::Char('x'));
        dock.handle_key(KeyCode::UpArrow);

        assert_eq!(dock.buffer, "abc");
        assert_eq!(dock.cursor, 3);
    }

    #[test]
    fn scene_compose_dock_inserts_voice_transcript_as_editable_text() {
        let mut dock = SceneComposeDock::default();
        dock.insert_transcript("open the roadmap");
        assert_eq!(dock.buffer, "open the roadmap");
        assert_eq!(dock.cursor, "open the roadmap".chars().count());

        dock.insert_transcript("and summarize it");
        assert_eq!(dock.buffer, "open the roadmap and summarize it");
    }

    #[test]
    fn scene_compose_dock_staged_nameplate_and_input_are_separate() {
        let mut dock = SceneComposeDock::default();
        dock.mark_submitted("say hi");
        let layout = vn_overlay_layout(80, 24, "Codex", "Composer");
        let nameplate = layout.composer_nameplate.unwrap();
        let panel = layout.composer_panel.unwrap();
        let input_rect = VnOverlayRect {
            col: panel.col.saturating_add(layout.composer_text_inset_cols),
            row: layout.composer_text_row.unwrap(),
            width: panel
                .width
                .saturating_sub(layout.composer_text_inset_cols * 2),
            height: 1,
        };

        let nameplate = dock.render_staged_nameplate_line(80, nameplate);
        let input = dock.render_staged_dock_line(80, input_rect);

        assert!(nameplate.contains("Composer"));
        assert!(!nameplate.contains("last:"));
        assert!(input.contains("last: say hi"));
        assert!(!input.contains("Compose:"));
        assert!(!input.contains("Composer"));
    }

    #[test]
    fn compose_backend_success_updates_dialogue() {
        let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
        let segments = apply_compose_backend_result(
            &mut runtime,
            ComposeBackendResult {
                prompt: "status".to_string(),
                stdout: "Workspace is ready.\n".to_string(),
                stderr: String::new(),
                exit_code: Some(0),
                label: ComposeBackendLabel::Compose,
            },
        );

        let snapshot = runtime.render_snapshot();
        assert_eq!(snapshot.dialogue_speaker, "Codex");
        assert_eq!(snapshot.dialogue, "Workspace is ready.");
        assert_eq!(snapshot.status, "Compose succeeded");
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].text, "Workspace is ready.");
        assert_eq!(segments[0].speaker.as_deref(), Some("Codex"));
    }

    #[test]
    fn compose_backend_structured_output_applies_patch_and_reply() {
        let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
        let raw_output = r#"{"speaker":"Guide","text":"The task now tracks progress","append_history":true,"status":"Task updated","patch":{"scene_patch_version":1,"updates":[{"entity_id":"task-render","label":"Task Render"}]}}"#;
        apply_compose_backend_result(
            &mut runtime,
            ComposeBackendResult {
                prompt: "status".to_string(),
                stdout: raw_output.to_string(),
                stderr: String::new(),
                exit_code: Some(0),
                label: ComposeBackendLabel::Codex,
            },
        );

        let snapshot = runtime.render_snapshot();
        let task = snapshot
            .entities
            .iter()
            .find(|entity| entity.id == "task-render")
            .expect("task-render entity exists");

        assert_eq!(task.label, "Task Render");
        assert_eq!(snapshot.dialogue_speaker, "Guide");
        assert_eq!(snapshot.dialogue, "The task now tracks progress");
        assert_eq!(snapshot.status, "Task updated");
    }

    #[test]
    fn compose_backend_structured_output_without_reply_uses_status() {
        let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
        let raw_output = r#"{"status":"Scene status patch only","patch":{"scene_patch_version":1,"status":"Scene status patch only"}}"#;
        apply_compose_backend_result(
            &mut runtime,
            ComposeBackendResult {
                prompt: "status".to_string(),
                stdout: raw_output.to_string(),
                stderr: String::new(),
                exit_code: Some(0),
                label: ComposeBackendLabel::Codex,
            },
        );

        let snapshot = runtime.render_snapshot();
        assert_eq!(snapshot.status, "Scene status patch only");
    }

    #[test]
    fn compose_backend_failure_updates_error_dialogue() {
        let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
        let segments = apply_compose_backend_result(
            &mut runtime,
            ComposeBackendResult {
                prompt: "status".to_string(),
                stdout: String::new(),
                stderr: "backend unavailable".to_string(),
                exit_code: Some(2),
                label: ComposeBackendLabel::Compose,
            },
        );

        let snapshot = runtime.render_snapshot();
        assert_eq!(snapshot.dialogue_speaker, "Scene");
        assert_eq!(snapshot.dialogue, "backend unavailable");
        assert_eq!(snapshot.status, "Compose failed");
        assert!(segments.is_empty());
    }

    #[test]
    fn scene_tts_toggle_uses_alt_m_without_consuming_plain_m() {
        assert!(is_tts_toggle_key(KeyCode::Char('m'), Modifiers::ALT));
        assert!(is_tts_toggle_key(KeyCode::Char('M'), Modifiers::ALT));
        assert!(!is_tts_toggle_key(KeyCode::Char('m'), Modifiers::NONE));
    }

    #[test]
    fn scene_stt_toggle_uses_alt_v_without_consuming_plain_v() {
        assert!(is_stt_toggle_key(KeyCode::Char('v'), Modifiers::ALT));
        assert!(is_stt_toggle_key(KeyCode::Char('V'), Modifiers::ALT));
        assert!(!is_stt_toggle_key(KeyCode::Char('v'), Modifiers::NONE));
    }

    #[test]
    fn compose_backend_output_is_sanitized_and_clipped() {
        let raw = format!("ok\u{1b}\n\n\t{}", "x".repeat(COMPOSE_OUTPUT_LIMIT + 20));
        let sanitized = sanitize_compose_output(&raw);

        assert!(!sanitized.contains('\u{1b}'));
        assert!(!sanitized.contains("\n\n"));
        assert!(sanitized.len() <= COMPOSE_OUTPUT_LIMIT + 3);
    }

    #[test]
    fn compose_backend_config_selects_codex_explicitly() {
        let codex_config = CodexComposeConfig {
            program: "codex".to_string(),
            workspace: PathBuf::from("/workspace"),
            sandbox: "read-only".to_string(),
            approval: "on-request".to_string(),
            json: true,
        };

        assert_eq!(
            compose_backend_config(None, None, codex_config.clone()),
            ComposeBackendConfig::BuiltIn
        );
        assert_eq!(
            compose_backend_config(None, Some("helper --flag"), codex_config.clone()),
            ComposeBackendConfig::Command("helper --flag".to_string())
        );
        assert_eq!(
            compose_backend_config(Some("codex"), None, codex_config.clone()),
            ComposeBackendConfig::Codex(codex_config.clone())
        );
        assert_eq!(
            compose_backend_config(None, Some("codex"), codex_config.clone()),
            ComposeBackendConfig::Codex(codex_config)
        );
    }

    #[test]
    fn codex_compose_argv_uses_structured_arguments() {
        let config = CodexComposeConfig {
            program: "codex".to_string(),
            workspace: PathBuf::from("/workspace with spaces"),
            sandbox: "read-only".to_string(),
            approval: "on-request".to_string(),
            json: true,
        };
        let argv = codex_compose_argv(
            &config,
            Path::new("/tmp/last-message.txt"),
            "inspect roadmap && do not shell split",
        );

        assert_eq!(argv[0], "codex");
        assert_eq!(argv[1], "exec");
        assert!(argv.contains(&"--json".to_string()));
        assert_eq!(
            argv,
            vec![
                "codex",
                "exec",
                "--output-last-message",
                "/tmp/last-message.txt",
                "-C",
                "/workspace with spaces",
                "-s",
                "read-only",
                "-c",
                "approval_policy=\"on-request\"",
                "--json",
                "inspect roadmap && do not shell split"
            ]
        );
    }

    #[test]
    fn codex_output_prefers_last_message_file() {
        let dir = tempfile::tempdir().unwrap();
        let output_file = dir.path().join("last.txt");
        std::fs::write(&output_file, "final Codex reply\n").unwrap();

        assert_eq!(
            codex_output_text(&output_file, b"{\"event\":\"stdout fallback\"}\n"),
            "final Codex reply\n"
        );
    }

    #[test]
    fn codex_backend_fake_command_updates_dialogue_status() {
        let dir = tempfile::tempdir().unwrap();
        let fake_codex = dir.path().join("fake-codex.sh");
        std::fs::write(
            &fake_codex,
            "#!/usr/bin/env sh\nwhile [ \"$1\" != \"\" ]; do\n  if [ \"$1\" = \"--output-last-message\" ]; then\n    shift\n    printf 'Codex says: %s\\n' \"$GAMETERM_SCENE_COMPOSE_PROMPT\" > \"$1\"\n  fi\n  shift || exit 0\ndone\nprintf '{\"event\":\"done\"}\\n'\n",
        )
        .unwrap();
        let mut permissions = std::fs::metadata(&fake_codex).unwrap().permissions();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            permissions.set_mode(0o755);
            std::fs::set_permissions(&fake_codex, permissions).unwrap();
        }

        let request = ComposeBackendRequest {
            prompt: "look at roadmap".to_string(),
            scene_path: Some("scene.json".to_string()),
            pane_id: Some(7),
        };
        let config = CodexComposeConfig {
            program: fake_codex.display().to_string(),
            workspace: dir.path().to_path_buf(),
            sandbox: "read-only".to_string(),
            approval: "on-request".to_string(),
            json: true,
        };
        let result = run_codex_compose_backend(request, config);

        assert_eq!(result.label, ComposeBackendLabel::Codex);
        assert_eq!(result.exit_code, Some(0));
        assert_eq!(
            sanitize_compose_output(&result.stdout),
            "Codex says: look at roadmap"
        );

        let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
        apply_compose_backend_result(&mut runtime, result);
        let snapshot = runtime.render_snapshot();

        assert_eq!(snapshot.dialogue_speaker, "Codex");
        assert_eq!(snapshot.dialogue, "Codex says: look at roadmap");
        assert_eq!(snapshot.status, "Codex succeeded");
    }

    #[test]
    fn replace_last_screen_line_preserves_frame_height() {
        let frame = replace_last_screen_line("one\r\ntwo\r\n".to_string(), 8, 4, "Compose: hello");

        let lines = frame.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), 4);
        assert_eq!(lines[0], "one");
        assert_eq!(lines[3], "Compose:");
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
