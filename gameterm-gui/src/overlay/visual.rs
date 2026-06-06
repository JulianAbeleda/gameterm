use crate::termwindow::TermWindowNotif;
use anyhow::Context;
use config::keyassignment::SpawnTabDomain;
use gameterm_dynamic::Value;
use gameterm_term::color::ColorAttribute;
use gameterm_term::TerminalSize;
use gameterm_visual::{
    truncate_to_screen, vn_overlay_layout, vn_overlay_layout_with_overrides, RunCommandTarget,
    SceneRuntime, VisualActionRequest, VisualInput, VisualMode, VisualModeOutcome,
    VisualRenderSnapshot, VisualResolvedSprite, VisualScene, VisualSceneDialoguePatch,
    VisualSceneLoadStatus, VisualScenePatch, VisualSceneSource, VisualSpriteManifest,
    VisualSpriteManifestStatus, VisualStoryState, VisualView, VnDialogueScrollMetrics,
    VnOverlayDebugOverrides, VnOverlayRect,
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
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use termwiz::input::{InputEvent, KeyCode, KeyEvent, Modifiers, MouseButtons, MouseEvent};
use termwiz::surface::Change;
use termwiz::terminal::{ScreenSize, Terminal};
use window::{Window, WindowOps};

const VN_OVERLAY_LAYOUT_CONFIG_FILE: &str = "vn-overlay-layout.json";
const KIKI_STAGE_TAG: &str = "kiki";
const KIKI_BASE_SPRITE: &str = "vn.character.kiki.neutral";
const KIKI_BREATH_FRAME_PREFIX: &str = "vn.character.kiki.breath.";
const KIKI_BREATH_FRAME_COUNT: usize = 6;
const KIKI_BREATH_FRAME_MS: u128 = 180;
const KIKI_BLINK_FRAME_PREFIX: &str = "vn.character.kiki.blink.";
const KIKI_BLINK_FRAME_COUNT: usize = 6;
const KIKI_BLINK_FRAME_MS: u128 = 90;
const KIKI_BLINK_INTERVAL_MS: u128 = 4_200;

#[path = "visual_voice_debug.rs"]
mod visual_voice_debug;

use super::visual_compose::ComposeBackendLabel;
use super::visual_compose::{
    compose_running_status, spawn_compose_backend, ComposeBackendRequest, ComposeBackendResult,
};
use super::visual_stt::{
    spawn_stt_backend, SceneSttConfig, SceneSttResult, SceneSttSession, SceneSttState,
};
use super::visual_tts::{
    extract_speakable_segments, SceneTtsConfig, SceneTtsRequest, SceneTtsResult, SceneTtsState,
    SceneTtsWorker, SpeakableSegment, SpeakableSource,
};
use visual_voice_debug::{
    handle_voice_debug_menu_key, is_voice_debug_menu_open_key, SceneVoiceDebugState,
    VoiceDebugMenuEffect,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SceneComposeDebugBackend {
    RealCodex,
    FakeCodex,
}

impl SceneComposeDebugBackend {
    fn toggle(&mut self) -> &'static str {
        *self = match self {
            Self::RealCodex => Self::FakeCodex,
            Self::FakeCodex => Self::RealCodex,
        };
        self.status()
    }

    fn status(self) -> &'static str {
        match self {
            Self::RealCodex => "Compose debug backend: Codex",
            Self::FakeCodex => "Compose debug backend: Fake Codex",
        }
    }

    fn is_fake(self) -> bool {
        matches!(self, Self::FakeCodex)
    }
}

pub(crate) fn show_visual_scene_overlay_with_options(
    term: TermWizTerminal,
    route_pane_id: Option<mux::pane::PaneId>,
    gui_window: Option<Window>,
    launch_options: SceneOverlayLaunchOptions,
) -> anyhow::Result<()> {
    show_visual_scene_overlay_with_source(
        term,
        route_pane_id,
        gui_window,
        VisualSceneOverlaySource::Default,
        launch_options,
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
        SceneOverlayLaunchOptions::default(),
    )
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SceneOverlayLaunchOptions {
    tts_config: Option<SceneTtsConfig>,
    stt_config: Option<SceneSttConfig>,
}

impl SceneOverlayLaunchOptions {
    pub(crate) fn with_voice_config(
        tts_config: SceneTtsConfig,
        stt_config: SceneSttConfig,
    ) -> Self {
        Self {
            tts_config: Some(tts_config),
            stt_config: Some(stt_config),
        }
    }
}

enum VisualSceneOverlaySource {
    Default,
    Generated {
        scene: VisualScene,
        action_base_dir: PathBuf,
        source_label: String,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct SceneDialogueScrollback {
    offset: usize,
    voice_hold_active: bool,
    voice_debug: SceneVoiceDebugState,
}

impl SceneDialogueScrollback {
    fn reset_to_bottom(&mut self) {
        self.offset = 0;
    }

    fn scroll_up(&mut self, max_offset: usize) {
        self.offset = self.offset.saturating_add(1).min(max_offset);
    }

    fn scroll_up_by(&mut self, lines: usize, max_offset: usize) {
        self.offset = self.offset.saturating_add(lines).min(max_offset);
    }

    fn scroll_down(&mut self) {
        self.offset = self.offset.saturating_sub(1);
    }

    fn scroll_down_by(&mut self, lines: usize) {
        self.offset = self.offset.saturating_sub(lines);
    }

    fn clamp(&mut self, max_offset: usize) {
        self.offset = self.offset.min(max_offset);
    }
}

fn show_visual_scene_overlay_with_source(
    mut term: TermWizTerminal,
    route_pane_id: Option<mux::pane::PaneId>,
    gui_window: Option<Window>,
    source: VisualSceneOverlaySource,
    launch_options: SceneOverlayLaunchOptions,
) -> anyhow::Result<()> {
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
    let mut dialogue_scroll = SceneDialogueScrollback::default();
    let mut compose_debug_backend = SceneComposeDebugBackend::RealCodex;
    let (compose_tx, compose_rx) = mpsc::channel();
    let mut compose_backend_running = false;
    let (tts_tx, tts_rx) = mpsc::channel();
    let tts_config = launch_options
        .tts_config
        .unwrap_or_else(SceneTtsConfig::from_env);
    let sync_first_voice_reveal = tts_config.can_play_audio();
    let tts_worker = SceneTtsWorker::new(tts_config, tts_tx.clone());
    let mut tts_state = SceneTtsState::default();
    let mut first_voice_reveal_done = false;
    let mut pending_first_voice_reveal: Option<PendingFirstVoiceReveal> = None;
    let (stt_tx, stt_rx) = mpsc::channel();
    let stt_config = launch_options
        .stt_config
        .unwrap_or_else(SceneSttConfig::from_env);
    let mut stt_state = SceneSttState::default();
    dialogue_scroll.voice_debug = SceneVoiceDebugState::new(&stt_config, &stt_state);
    let mut stt_session: Option<SceneSttSession> = None;
    let mut last_idle_sprite: Option<String> = None;

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
                dialogue_scroll.reset_to_bottom();
                let speakable_segments = compose_result_speakable_segments(&result);
                if should_delay_first_voice_reveal(
                    sync_first_voice_reveal,
                    first_voice_reveal_done,
                    pending_first_voice_reveal.is_some(),
                    tts_state.is_muted(),
                    &speakable_segments,
                ) {
                    for segment in speakable_segments {
                        tts_worker.speak(SceneTtsRequest { segment });
                    }
                    pending_first_voice_reveal = Some(PendingFirstVoiceReveal { result });
                    runtime.mark_action_status("Voice preparing first reply");
                } else {
                    let speakable_segments = apply_compose_backend_result(runtime, result);
                    if !first_voice_reveal_done && !speakable_segments.is_empty() {
                        first_voice_reveal_done = true;
                    }
                    if !tts_state.is_muted() {
                        for segment in speakable_segments {
                            tts_worker.speak(SceneTtsRequest { segment });
                        }
                    }
                }
                needs_render = true;
            }
        }
        while let Ok(result) = tts_rx.try_recv() {
            if let Some(runtime) = runtime.as_mut() {
                if let Some(pending) = pending_first_voice_reveal.take() {
                    apply_compose_backend_result(runtime, pending.result);
                    first_voice_reveal_done = true;
                    dialogue_scroll.reset_to_bottom();
                }
                apply_tts_result(runtime, &mut tts_state, result);
                needs_render = true;
            }
        }
        while let Ok(result) = stt_rx.try_recv() {
            stt_session = None;
            dialogue_scroll.voice_hold_active = false;
            if let Some(runtime) = runtime.as_mut() {
                dialogue_scroll.voice_debug.apply_result(&result);
                if dialogue_scroll.voice_debug.test_mode {
                    let status = stt_state.apply_result(&result);
                    if let Some(transcript) = result.transcript.as_deref() {
                        runtime.mark_action_status(format!("Voice test recognized: {transcript}"));
                    } else if let Some(error) = result.error.as_deref() {
                        runtime.mark_action_status(format!("{status}: {error}"));
                    } else {
                        runtime.mark_action_status(status);
                    }
                    dialogue_scroll
                        .voice_debug
                        .sync_status(stt_state.last_status());
                } else {
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
                    dialogue_scroll
                        .voice_debug
                        .sync_status(stt_state.last_status());
                }
                dialogue_scroll.reset_to_bottom();
                needs_render = true;
            }
        }
        if needs_render {
            if let Some(runtime) = runtime.as_ref() {
                render_runtime_with_compose_and_scroll(
                    &mut term,
                    runtime,
                    &sprite_manifest,
                    &compose_dock,
                    &dialogue_scroll,
                )?;
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
                dialogue_scroll.reset_to_bottom();
                file_watcher.refresh(&scene_path, &sprite_manifest_path);
            }
            if let Some(path) = patch_inbox.changed_path() {
                if let Some(runtime) = runtime.as_mut() {
                    apply_scene_patch_file(&mut term, runtime, &sprite_manifest, &path)?;
                    dialogue_scroll.reset_to_bottom();
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
                    dialogue_scroll.reset_to_bottom();
                }
            }
            if let Some(runtime) = runtime.as_ref() {
                let sprite = current_kiki_idle_sprite(&sprite_manifest);
                if runtime_has_kiki_idle_animation(runtime, &sprite_manifest) {
                    if last_idle_sprite != sprite {
                        render_runtime_with_compose_and_scroll(
                            &mut term,
                            runtime,
                            &sprite_manifest,
                            &compose_dock,
                            &dialogue_scroll,
                        )?;
                        last_idle_sprite = sprite;
                    }
                } else {
                    last_idle_sprite = None;
                }
            }
            continue;
        };
        match input {
            InputEvent::Key(KeyEvent { key, modifiers }) => {
                if let Some(runtime) = runtime.as_mut() {
                    if stt_state.is_running() && matches!(key, KeyCode::Escape) {
                        if let Some(session) = stt_session.take() {
                            session.cancel();
                        }
                        dialogue_scroll.voice_hold_active = false;
                        runtime.mark_action_status(stt_state.mark_canceling());
                        dialogue_scroll
                            .voice_debug
                            .sync_status(stt_state.last_status());
                        render_runtime_with_compose_and_scroll(
                            &mut term,
                            runtime,
                            &sprite_manifest,
                            &compose_dock,
                            &dialogue_scroll,
                        )?;
                        continue;
                    }
                    if dialogue_scroll.voice_debug.menu_open {
                        let voice_debug_effect = handle_voice_debug_menu_key(
                            key,
                            runtime,
                            &mut dialogue_scroll.voice_debug,
                            stt_state.is_running(),
                            compose_backend_running,
                            &mut compose_debug_backend,
                        );
                        if voice_debug_effect.handled {
                            if voice_debug_effect.reset_compose_dialogue {
                                first_voice_reveal_done = false;
                                pending_first_voice_reveal = None;
                                dialogue_scroll.reset_to_bottom();
                            }
                            render_runtime_with_compose_and_scroll(
                                &mut term,
                                runtime,
                                &sprite_manifest,
                                &compose_dock,
                                &dialogue_scroll,
                            )?;
                            continue;
                        }
                    }
                    if runtime.view() == VisualView::TileDebugger
                        && is_voice_debug_menu_open_key(key, modifiers)
                    {
                        let status = dialogue_scroll.voice_debug.open_menu();
                        runtime.mark_action_status(status);
                        render_runtime_with_compose_and_scroll(
                            &mut term,
                            runtime,
                            &sprite_manifest,
                            &compose_dock,
                            &dialogue_scroll,
                        )?;
                        continue;
                    }
                    if is_tts_toggle_key(key, modifiers) {
                        runtime.mark_action_status(tts_state.toggle_muted());
                        render_runtime_with_compose_and_scroll(
                            &mut term,
                            runtime,
                            &sprite_manifest,
                            &compose_dock,
                            &dialogue_scroll,
                        )?;
                        continue;
                    }
                    if is_stt_hold_key(key, modifiers) {
                        if !stt_state.is_running() {
                            runtime.mark_action_status(stt_state.mark_started());
                            if dialogue_scroll.voice_debug.test_mode {
                                runtime.mark_action_status("Voice test listening");
                            }
                            dialogue_scroll
                                .voice_debug
                                .sync_status(stt_state.last_status());
                            stt_session =
                                Some(spawn_stt_backend(stt_config.clone(), stt_tx.clone()));
                            dialogue_scroll.voice_hold_active = true;
                        }
                        render_runtime_with_compose_and_scroll(
                            &mut term,
                            runtime,
                            &sprite_manifest,
                            &compose_dock,
                            &dialogue_scroll,
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
                                render_runtime_with_compose_and_scroll(
                                    &mut term,
                                    runtime,
                                    &sprite_manifest,
                                    &compose_dock,
                                    &dialogue_scroll,
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
                                    render_runtime_with_compose_and_scroll(
                                        &mut term,
                                        runtime,
                                        &sprite_manifest,
                                        &compose_dock,
                                        &dialogue_scroll,
                                    )?;
                                    continue;
                                }
                                compose_dock.mark_submitted(&prompt);
                                let running_status = if compose_debug_backend.is_fake() {
                                    format!("Fake Codex running: {prompt}")
                                } else {
                                    compose_running_status(&prompt)
                                };
                                runtime.mark_compose_running(running_status, &prompt);
                                dialogue_scroll.reset_to_bottom();
                                if compose_debug_backend.is_fake() {
                                    let result = fake_codex_compose_result(prompt);
                                    let speakable_segments =
                                        compose_result_speakable_segments(&result);
                                    if should_delay_first_voice_reveal(
                                        sync_first_voice_reveal,
                                        first_voice_reveal_done,
                                        pending_first_voice_reveal.is_some(),
                                        tts_state.is_muted(),
                                        &speakable_segments,
                                    ) {
                                        for segment in speakable_segments {
                                            tts_worker.speak(SceneTtsRequest { segment });
                                        }
                                        pending_first_voice_reveal =
                                            Some(PendingFirstVoiceReveal { result });
                                        runtime.mark_action_status("Voice preparing first reply");
                                    } else {
                                        let speakable_segments =
                                            apply_compose_backend_result(runtime, result);
                                        if !first_voice_reveal_done
                                            && !speakable_segments.is_empty()
                                        {
                                            first_voice_reveal_done = true;
                                        }
                                        if !tts_state.is_muted() {
                                            for segment in speakable_segments {
                                                tts_worker.speak(SceneTtsRequest { segment });
                                            }
                                        }
                                    }
                                    render_runtime_with_compose_and_scroll(
                                        &mut term,
                                        runtime,
                                        &sprite_manifest,
                                        &compose_dock,
                                        &dialogue_scroll,
                                    )?;
                                    continue;
                                }
                                compose_backend_running = true;
                                spawn_compose_backend(
                                    ComposeBackendRequest {
                                        prompt,
                                        scene_path: Some(scene_path.display().to_string()),
                                        pane_id: Some(pane_id),
                                    },
                                    compose_tx.clone(),
                                );
                                render_runtime_with_compose_and_scroll(
                                    &mut term,
                                    runtime,
                                    &sprite_manifest,
                                    &compose_dock,
                                    &dialogue_scroll,
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
                let in_layout_debug = runtime.as_ref().map_or(false, |runtime| {
                    runtime.view() == VisualView::VnLayoutDebugger
                });
                if !in_layout_debug {
                    if let Some(runtime) = runtime.as_ref() {
                        if handle_dialogue_scroll_key(
                            runtime,
                            &mut dialogue_scroll,
                            visual_input,
                            term.get_screen_size()?,
                        ) {
                            render_runtime_with_compose_and_scroll(
                                &mut term,
                                runtime,
                                &sprite_manifest,
                                &compose_dock,
                                &dialogue_scroll,
                            )?;
                            continue;
                        }
                    }
                }
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
                    dialogue_scroll.reset_to_bottom();
                    file_watcher.refresh(&scene_path, &sprite_manifest_path);
                    continue;
                }
                if let Some(runtime) = runtime.as_mut() {
                    let vn_layout_before = runtime
                        .vn_layout_debug_overrides()
                        .map(persistable_vn_overlay_layout);
                    if runtime.handle_input(visual_input) == VisualModeOutcome::Exit {
                        break;
                    }
                    if visual_input_resets_dialogue_scroll(visual_input) {
                        dialogue_scroll.reset_to_bottom();
                    }
                    persist_vn_overlay_layout_if_changed(vn_layout_before, runtime);
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
                    render_runtime_with_compose_and_scroll(
                        &mut term,
                        runtime,
                        &sprite_manifest,
                        &compose_dock,
                        &dialogue_scroll,
                    )?;
                }
            }
            InputEvent::KeyUp(KeyEvent { key, modifiers: _ }) => {
                if let Some(runtime) = runtime.as_mut() {
                    if is_stt_hold_release_key(key) && stt_state.is_running() {
                        if let Some(session) = stt_session.take() {
                            session.stop();
                        }
                        dialogue_scroll.voice_hold_active = false;
                        runtime.mark_action_status(stt_state.mark_processing());
                        if dialogue_scroll.voice_debug.test_mode {
                            runtime.mark_action_status("Voice test processing");
                        }
                        dialogue_scroll
                            .voice_debug
                            .sync_status(stt_state.last_status());
                        render_runtime_with_compose_and_scroll(
                            &mut term,
                            runtime,
                            &sprite_manifest,
                            &compose_dock,
                            &dialogue_scroll,
                        )?;
                    }
                }
            }
            InputEvent::Mouse(MouseEvent {
                x,
                y,
                mouse_buttons,
                ..
            }) if mouse_buttons.contains(MouseButtons::VERT_WHEEL) => {
                if let Some(runtime) = runtime.as_ref() {
                    let size = term.get_screen_size()?;
                    if handle_dialogue_scroll_wheel(
                        runtime,
                        &mut dialogue_scroll,
                        size.cols,
                        size.rows,
                        x,
                        y,
                        mouse_buttons,
                    ) {
                        render_runtime_with_compose_and_scroll(
                            &mut term,
                            runtime,
                            &sprite_manifest,
                            &compose_dock,
                            &dialogue_scroll,
                        )?;
                        continue;
                    }
                }
            }
            InputEvent::Resized { .. } => {
                if let Some(runtime) = runtime.as_ref() {
                    let size = term.get_screen_size()?;
                    let metrics = runtime.vn_dialogue_scroll_metrics(
                        size.cols,
                        size.rows,
                        dialogue_scroll.offset,
                    );
                    dialogue_scroll.clamp(metrics.max_scroll_offset);
                    render_runtime_with_compose_and_scroll(
                        &mut term,
                        runtime,
                        &sprite_manifest,
                        &compose_dock,
                        &dialogue_scroll,
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
                let mut loaded = SceneRuntime::new_with_source(scene, source)?;
                apply_configured_vn_overlay_layout(&mut loaded);
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
        let mut loaded =
            SceneRuntime::new_with_source_and_action_base_dir(scene, source, action_base_dir)?;
        apply_configured_vn_overlay_layout(&mut loaded);
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
            let mut runtime = SceneRuntime::new_with_source(scene, source)?;
            apply_configured_vn_overlay_layout(&mut runtime);
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
        Ok(mut runtime) => {
            apply_configured_vn_overlay_layout(&mut runtime);
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

fn default_vn_overlay_layout_config_path() -> PathBuf {
    default_scene_dir().join(VN_OVERLAY_LAYOUT_CONFIG_FILE)
}

fn default_scene_dir() -> PathBuf {
    let config_home = config::CONFIG_DIRS
        .first()
        .cloned()
        .unwrap_or_else(|| config::HOME_DIR.join(".config").join("gameterm"));
    config_home.join("scenes")
}

fn apply_configured_vn_overlay_layout(runtime: &mut SceneRuntime) {
    if let Some(overrides) = load_vn_overlay_layout_config() {
        runtime.set_vn_layout_debug_overrides(overrides);
    }
}

fn load_vn_overlay_layout_config() -> Option<VnOverlayDebugOverrides> {
    load_vn_overlay_layout_config_from_path(&default_vn_overlay_layout_config_path())
}

fn load_vn_overlay_layout_config_from_path(path: &Path) -> Option<VnOverlayDebugOverrides> {
    let data = match fs::read_to_string(&path) {
        Ok(data) => data,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return None,
        Err(err) => {
            log::warn!(
                "failed to read VN overlay layout config {}: {err}",
                path.display()
            );
            return None;
        }
    };
    match serde_json::from_str::<VnOverlayDebugOverrides>(&data) {
        Ok(mut overrides) => {
            overrides.editing_buffer = None;
            Some(overrides)
        }
        Err(err) => {
            log::warn!(
                "failed to parse VN overlay layout config {}: {err}",
                path.display()
            );
            None
        }
    }
}

fn persistable_vn_overlay_layout(overrides: &VnOverlayDebugOverrides) -> VnOverlayDebugOverrides {
    let mut overrides = overrides.clone();
    overrides.editing_buffer = None;
    overrides
}

fn save_vn_overlay_layout_config(overrides: &VnOverlayDebugOverrides) -> anyhow::Result<()> {
    save_vn_overlay_layout_config_to_path(&default_vn_overlay_layout_config_path(), overrides)
}

fn save_vn_overlay_layout_config_to_path(
    path: &Path,
    overrides: &VnOverlayDebugOverrides,
) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let overrides = persistable_vn_overlay_layout(overrides);
    fs::write(path, serde_json::to_string_pretty(&overrides)?)?;
    Ok(())
}

fn persist_vn_overlay_layout_if_changed(
    before: Option<VnOverlayDebugOverrides>,
    runtime: &SceneRuntime,
) {
    let after = runtime
        .vn_layout_debug_overrides()
        .map(persistable_vn_overlay_layout);
    if after == before {
        return;
    }
    if let Some(after) = after {
        if let Err(err) = save_vn_overlay_layout_config(&after) {
            log::warn!("failed to save VN overlay layout config: {err}");
        }
    }
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
        KeyCode::PageUp => VisualInput::ScrollDialogueUp,
        KeyCode::PageDown => VisualInput::ScrollDialogueDown,
        KeyCode::Backspace => VisualInput::Backspace,
        KeyCode::Char(c) => VisualInput::Char(c),
        _ => VisualInput::Other,
    }
}

fn visual_input_resets_dialogue_scroll(input: VisualInput) -> bool {
    matches!(input, VisualInput::Activate)
}

fn is_tts_toggle_key(key: KeyCode, modifiers: Modifiers) -> bool {
    matches!(key, KeyCode::Char('m') | KeyCode::Char('M')) && modifiers.contains(Modifiers::ALT)
}

fn is_stt_hold_key(key: KeyCode, modifiers: Modifiers) -> bool {
    matches!(
        key,
        KeyCode::Shift
            | KeyCode::LeftShift
            | KeyCode::RightShift
            | KeyCode::Super
            | KeyCode::LeftWindows
            | KeyCode::RightWindows
    ) && modifiers.contains(Modifiers::SHIFT)
        && modifiers.contains(Modifiers::SUPER)
}

fn is_stt_hold_release_key(key: KeyCode) -> bool {
    matches!(
        key,
        KeyCode::Shift
            | KeyCode::LeftShift
            | KeyCode::RightShift
            | KeyCode::Super
            | KeyCode::LeftWindows
            | KeyCode::RightWindows
    )
}

const COMPOSE_OUTPUT_LIMIT: usize = 1200;

#[derive(Debug, Clone)]
struct PendingFirstVoiceReveal {
    result: ComposeBackendResult,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum StructuredComposeOutcome {
    NoReply,
    WithReply {
        speaker: String,
        dialogue_text: String,
    },
}

fn should_delay_first_voice_reveal(
    sync_first_voice_reveal: bool,
    first_voice_reveal_done: bool,
    reveal_already_pending: bool,
    tts_muted: bool,
    speakable_segments: &[SpeakableSegment],
) -> bool {
    sync_first_voice_reveal
        && !first_voice_reveal_done
        && !reveal_already_pending
        && !tts_muted
        && !speakable_segments.is_empty()
}

fn compose_result_speakable_segments(result: &ComposeBackendResult) -> Vec<SpeakableSegment> {
    let Some((speaker, dialogue_text)) = compose_result_reply_preview(result) else {
        return Vec::new();
    };
    extract_speakable_segments(
        Some(&speaker),
        &dialogue_text,
        SpeakableSource::ComposeReply,
    )
}

fn compose_result_reply_preview(result: &ComposeBackendResult) -> Option<(String, String)> {
    if !result.succeeded() {
        return None;
    }

    if let Some(payload) = parse_structured_compose_payload(&result.stdout) {
        if let Some(value) = payload.patch.clone() {
            if let Ok(patch) = parse_structured_compose_patch(value) {
                if let Some(dialogue) = patch.dialogue {
                    return Some((dialogue.speaker, dialogue.text));
                }
            }
        }

        if payload.dialogue_text_is_present() {
            let speaker = payload
                .speaker
                .as_deref()
                .filter(|speaker| !speaker.trim().is_empty())
                .unwrap_or("Codex")
                .to_string();
            return Some((speaker, payload.text_or_default()));
        }

        return None;
    }

    let reply = sanitize_compose_output(&result.stdout);
    let reply = if reply.is_empty() {
        "The compose backend returned no output.".to_string()
    } else {
        reply
    };
    Some(("Codex".to_string(), reply))
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

fn fake_codex_compose_result(prompt: String) -> ComposeBackendResult {
    let text = if prompt.trim().is_empty() {
        "Fake Codex is ready.".to_string()
    } else {
        format!("Fake Codex received: {}", prompt.trim())
    };
    let stdout = serde_json::json!({
        "speaker": "Fake Codex",
        "text": text,
        "status": "Fake Codex succeeded"
    })
    .to_string();
    ComposeBackendResult {
        prompt,
        stdout,
        stderr: String::new(),
        exit_code: Some(0),
        label: ComposeBackendLabel::Codex,
    }
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
    let diagnostic = result.failure_dialogue(&diagnostic);
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
        status: Some(result.failure_status()),
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
            line.push_str("  type here; enter submits");
        }
        clip_text(&line, cols.max(1))
    }

    fn render_staged_dock_line(&self, cols: usize, rect: VnOverlayRect) -> String {
        let mut line = String::from(" ");
        line.push_str(&self.buffer_with_cursor());
        if self.buffer.is_empty() {
            line.push_str(" type here; enter submits");
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

fn handle_dialogue_scroll_wheel(
    runtime: &SceneRuntime,
    scroll: &mut SceneDialogueScrollback,
    cols: usize,
    rows: usize,
    x: u16,
    y: u16,
    mouse_buttons: MouseButtons,
) -> bool {
    let Some(panel) = runtime.vn_dialogue_panel_rect(cols, rows) else {
        return false;
    };
    let mouse_col = x.saturating_sub(1) as usize;
    let mouse_row = y.saturating_sub(1) as usize;
    if mouse_col < panel.col
        || mouse_col >= panel.right()
        || mouse_row < panel.row
        || mouse_row >= panel.bottom()
    {
        return false;
    }

    let metrics = runtime.vn_dialogue_scroll_metrics(cols, rows, scroll.offset);
    apply_dialogue_scroll_wheel(scroll, metrics, mouse_buttons);
    true
}

fn handle_dialogue_scroll_key(
    runtime: &SceneRuntime,
    scroll: &mut SceneDialogueScrollback,
    input: VisualInput,
    size: ScreenSize,
) -> bool {
    let metrics = runtime.vn_dialogue_scroll_metrics(size.cols, size.rows, scroll.offset);
    apply_dialogue_scroll_key(scroll, metrics, input)
}

fn apply_dialogue_scroll_key(
    scroll: &mut SceneDialogueScrollback,
    metrics: VnDialogueScrollMetrics,
    input: VisualInput,
) -> bool {
    let page_lines = metrics.visible_rows.saturating_sub(1).max(1);
    match input {
        VisualInput::ScrollDialogueUp => {
            scroll.scroll_up_by(page_lines, metrics.max_scroll_offset);
            true
        }
        VisualInput::ScrollDialogueDown => {
            scroll.scroll_down_by(page_lines);
            scroll.clamp(metrics.max_scroll_offset);
            true
        }
        _ => false,
    }
}

fn apply_dialogue_scroll_wheel(
    scroll: &mut SceneDialogueScrollback,
    metrics: VnDialogueScrollMetrics,
    mouse_buttons: MouseButtons,
) {
    if mouse_buttons.contains(MouseButtons::WHEEL_POSITIVE) {
        scroll.scroll_up(metrics.max_scroll_offset);
    } else {
        scroll.scroll_down();
    }
    scroll.clamp(metrics.max_scroll_offset);
}

fn render_runtime_with_compose(
    term: &mut TermWizTerminal,
    runtime: &SceneRuntime,
    sprite_manifest: &VisualSpriteManifestStatus,
    compose_dock: &SceneComposeDock,
) -> anyhow::Result<()> {
    render_runtime_with_compose_and_scroll(
        term,
        runtime,
        sprite_manifest,
        compose_dock,
        &SceneDialogueScrollback::default(),
    )
}

fn render_runtime_with_compose_and_scroll(
    term: &mut TermWizTerminal,
    runtime: &SceneRuntime,
    sprite_manifest: &VisualSpriteManifestStatus,
    compose_dock: &SceneComposeDock,
    dialogue_scroll: &SceneDialogueScrollback,
) -> anyhow::Result<()> {
    let size = term.get_screen_size()?;
    let mut snapshot = runtime.render_snapshot();
    apply_kiki_idle_animation(
        &mut snapshot,
        sprite_manifest,
        current_kiki_idle_sprite(sprite_manifest),
    );
    snapshot.overlay_cols = Some(size.cols);
    snapshot.overlay_rows = Some(size.rows);
    snapshot.vn_dialogue_scroll =
        Some(runtime.vn_dialogue_scroll_metrics(size.cols, size.rows, dialogue_scroll.offset));
    snapshot.vn_voice_hold_active = dialogue_scroll.voice_hold_active;
    term.set_metadata(
        "gameterm_visual_snapshot",
        Value::String(serde_json::to_string(&snapshot)?),
    );
    term.set_metadata(
        "gameterm_visual_sprites",
        Value::String(serde_json::to_string(sprite_manifest)?),
    );
    let mut frame = String::new();
    if snapshot.stage.is_empty() && !sprite_manifest.warnings.is_empty() {
        frame.push_str("Sprites: ");
        frame.push_str(&sprite_manifest.warnings.join("; "));
        frame.push_str("\r\n\r\n");
    }
    frame.push_str(
        &runtime.render_text_frame_with_dialogue_scroll_and_voice_hold(
            size.cols,
            size.rows,
            dialogue_scroll.offset,
            dialogue_scroll.voice_hold_active,
        ),
    );
    if !snapshot.stage.is_empty() {
        let layout = match snapshot.vn_layout_debug.as_ref() {
            Some(overrides) => vn_overlay_layout_with_overrides(
                size.cols,
                size.rows,
                &snapshot.dialogue_speaker,
                "Composer",
                overrides,
            ),
            None => vn_overlay_layout(size.cols, size.rows, &snapshot.dialogue_speaker, "Composer"),
        };
        if let Some(nameplate) = layout.composer_nameplate_text {
            frame = replace_screen_line(
                frame,
                size.cols,
                size.rows,
                nameplate.row.min(size.rows.saturating_sub(1)),
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
    frame = apply_voice_debug_frame(
        frame,
        size.cols,
        size.rows,
        runtime,
        &dialogue_scroll.voice_debug,
    );
    term.render(&[
        Change::ClearScreen(ColorAttribute::Default),
        Change::Text(truncate_to_screen(frame, size.cols, size.rows)),
    ])?;
    term.flush()?;
    Ok(())
}

fn apply_voice_debug_frame(
    mut frame: String,
    cols: usize,
    rows: usize,
    runtime: &SceneRuntime,
    voice_debug: &SceneVoiceDebugState,
) -> String {
    if runtime.view() == VisualView::TileDebugger && !voice_debug.menu_open {
        frame = replace_screen_line(
            frame,
            cols,
            rows,
            1,
            "[tab: layout debug] [v: voice] [arrows/hjkl: select entity] [esc/q: close]",
        );
    }
    let lines = voice_debug.render_lines();
    if lines.is_empty() {
        return frame;
    }
    let max_width = cols.min(96);
    let max_lines = rows.saturating_sub(1).min(lines.len());
    for (idx, line) in lines.iter().take(max_lines).enumerate() {
        frame = replace_screen_line(frame, cols, rows, idx, &clip_text(line, max_width));
    }
    frame
}

fn current_kiki_idle_sprite(sprite_manifest: &VisualSpriteManifestStatus) -> Option<String> {
    let elapsed_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis())
        .unwrap_or(0);
    kiki_idle_sprite_for_elapsed_ms(elapsed_ms, sprite_manifest)
}

fn kiki_idle_sprite_for_elapsed_ms(
    elapsed_ms: u128,
    sprite_manifest: &VisualSpriteManifestStatus,
) -> Option<String> {
    if let Some(frame) = kiki_blink_frame_for_elapsed_ms(elapsed_ms) {
        let sprite = kiki_blink_sprite_id(frame);
        if sprite_manifest_has_id(sprite_manifest, &sprite) {
            return Some(sprite);
        }
    }
    let sprite = kiki_breath_sprite_id(kiki_breath_frame_for_elapsed_ms(elapsed_ms));
    sprite_manifest_has_id(sprite_manifest, &sprite).then_some(sprite)
}

fn kiki_breath_frame_for_elapsed_ms(elapsed_ms: u128) -> usize {
    ((elapsed_ms / KIKI_BREATH_FRAME_MS) % KIKI_BREATH_FRAME_COUNT as u128) as usize
}

fn kiki_blink_frame_for_elapsed_ms(elapsed_ms: u128) -> Option<usize> {
    let blink_elapsed = elapsed_ms % KIKI_BLINK_INTERVAL_MS;
    let frame = (blink_elapsed / KIKI_BLINK_FRAME_MS) as usize;
    (frame < KIKI_BLINK_FRAME_COUNT).then_some(frame)
}

fn runtime_has_kiki_idle_animation(
    runtime: &SceneRuntime,
    sprite_manifest: &VisualSpriteManifestStatus,
) -> bool {
    snapshot_has_kiki_idle_animation(&runtime.render_snapshot(), sprite_manifest)
}

fn snapshot_has_kiki_idle_animation(
    snapshot: &VisualRenderSnapshot,
    sprite_manifest: &VisualSpriteManifestStatus,
) -> bool {
    !kiki_is_speaking(snapshot)
        && snapshot
            .stage
            .iter()
            .any(|displayable| displayable.tag == KIKI_STAGE_TAG)
        && kiki_breath_frames_available(sprite_manifest)
}

fn apply_kiki_idle_animation(
    snapshot: &mut VisualRenderSnapshot,
    sprite_manifest: &VisualSpriteManifestStatus,
    sprite: Option<String>,
) {
    if !snapshot_has_kiki_idle_animation(snapshot, sprite_manifest) {
        return;
    }
    let Some(sprite) = sprite else {
        return;
    };
    if !sprite_manifest_has_id(sprite_manifest, &sprite) {
        return;
    }
    for displayable in &mut snapshot.stage {
        if displayable.tag == KIKI_STAGE_TAG && displayable.sprite == KIKI_BASE_SPRITE {
            displayable.sprite = sprite.clone();
        }
    }
}

fn kiki_is_speaking(snapshot: &VisualRenderSnapshot) -> bool {
    snapshot
        .dialogue_speaker
        .trim()
        .eq_ignore_ascii_case(KIKI_STAGE_TAG)
}

fn kiki_breath_frames_available(sprite_manifest: &VisualSpriteManifestStatus) -> bool {
    (0..KIKI_BREATH_FRAME_COUNT)
        .all(|frame| sprite_manifest_has_id(sprite_manifest, &kiki_breath_sprite_id(frame)))
}

fn sprite_manifest_has_id(sprite_manifest: &VisualSpriteManifestStatus, sprite_id: &str) -> bool {
    sprite_manifest
        .sprites
        .iter()
        .any(|sprite| sprite.id == sprite_id)
}

fn kiki_breath_sprite_id(frame: usize) -> String {
    format!(
        "{}{}",
        KIKI_BREATH_FRAME_PREFIX,
        frame.min(KIKI_BREATH_FRAME_COUNT.saturating_sub(1))
    )
}

fn kiki_blink_sprite_id(frame: usize) -> String {
    format!(
        "{}{}",
        KIKI_BLINK_FRAME_PREFIX,
        frame.min(KIKI_BLINK_FRAME_COUNT.saturating_sub(1))
    )
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
    use gameterm_visual::{
        VisualStage, VisualStageDisplayable, VisualStageLayer, VisualStagePlacement,
        VN_OVERLAY_COMPOSER_NAMEPLATE_TEXT_INSET_ROWS,
        VN_OVERLAY_DIALOGUE_NAMEPLATE_TEXT_INSET_COLS, VN_OVERLAY_NAMEPLATE_OPACITY,
        VN_OVERLAY_PANEL_OPACITY,
    };

    fn scene_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("ci")
            .join("fixtures")
            .join("gameterm-scene")
            .join(name)
    }

    fn kiki_animation_sprite_manifest(
        breath_frame_count: usize,
        blink_frame_count: usize,
    ) -> VisualSpriteManifestStatus {
        VisualSpriteManifestStatus {
            manifest_path: None,
            sprites: (0..breath_frame_count)
                .map(|frame| VisualResolvedSprite {
                    id: kiki_breath_sprite_id(frame),
                    path: format!("/tmp/kiki-breath-{frame}.png"),
                })
                .chain((0..blink_frame_count).map(|frame| VisualResolvedSprite {
                    id: kiki_blink_sprite_id(frame),
                    path: format!("/tmp/kiki-blink-{frame}.png"),
                }))
                .collect(),
            warnings: Vec::new(),
        }
    }

    fn kiki_stage_snapshot(speaker: &str) -> VisualRenderSnapshot {
        let mut scene = VisualScene::demo();
        scene.dialogue_lines.clear();
        scene.dialogue_speaker = speaker.to_string();
        scene.dialogue = "Kiki is waiting.".to_string();
        scene.stage = VisualStage {
            layers: vec![VisualStageLayer {
                layer_id: "characters".to_string(),
                zorder: 10,
                displayables: vec![VisualStageDisplayable {
                    tag: KIKI_STAGE_TAG.to_string(),
                    sprite: KIKI_BASE_SPRITE.to_string(),
                    placement: VisualStagePlacement::Center,
                    zorder: 0,
                    visible: true,
                }],
            }],
        };
        SceneRuntime::new(scene).unwrap().render_snapshot()
    }

    #[test]
    fn kiki_breath_frame_cycles_over_six_frames() {
        assert_eq!(kiki_breath_frame_for_elapsed_ms(0), 0);
        assert_eq!(kiki_breath_frame_for_elapsed_ms(KIKI_BREATH_FRAME_MS), 1);
        assert_eq!(
            kiki_breath_frame_for_elapsed_ms(KIKI_BREATH_FRAME_MS * 5),
            5
        );
        assert_eq!(
            kiki_breath_frame_for_elapsed_ms(KIKI_BREATH_FRAME_MS * 6),
            0
        );
    }

    #[test]
    fn kiki_blink_frame_only_occupies_start_of_interval() {
        assert_eq!(kiki_blink_frame_for_elapsed_ms(0), Some(0));
        assert_eq!(
            kiki_blink_frame_for_elapsed_ms(KIKI_BLINK_FRAME_MS),
            Some(1)
        );
        assert_eq!(
            kiki_blink_frame_for_elapsed_ms(KIKI_BLINK_FRAME_MS * 5),
            Some(5)
        );
        assert_eq!(
            kiki_blink_frame_for_elapsed_ms(KIKI_BLINK_FRAME_MS * 6),
            None
        );
    }

    #[test]
    fn kiki_idle_animation_uses_breath_when_not_speaking() {
        let mut snapshot = kiki_stage_snapshot("Codex");
        let sprites = kiki_animation_sprite_manifest(KIKI_BREATH_FRAME_COUNT, 0);

        apply_kiki_idle_animation(&mut snapshot, &sprites, Some(kiki_breath_sprite_id(3)));

        assert_eq!(snapshot.stage[0].sprite, "vn.character.kiki.breath.3");
    }

    #[test]
    fn kiki_idle_animation_prefers_blink_during_blink_window() {
        let sprites =
            kiki_animation_sprite_manifest(KIKI_BREATH_FRAME_COUNT, KIKI_BLINK_FRAME_COUNT);
        let sprite = kiki_idle_sprite_for_elapsed_ms(KIKI_BLINK_FRAME_MS * 2, &sprites);

        assert_eq!(sprite.as_deref(), Some("vn.character.kiki.blink.2"));
    }

    #[test]
    fn kiki_idle_animation_waits_for_breath_frames() {
        let mut snapshot = kiki_stage_snapshot("Codex");
        let sprites = kiki_animation_sprite_manifest(KIKI_BREATH_FRAME_COUNT - 1, 0);

        apply_kiki_idle_animation(&mut snapshot, &sprites, Some(kiki_breath_sprite_id(3)));

        assert_eq!(snapshot.stage[0].sprite, KIKI_BASE_SPRITE);
    }

    #[test]
    fn kiki_idle_animation_stops_when_kiki_is_speaking() {
        let mut snapshot = kiki_stage_snapshot("Kiki");
        let sprites =
            kiki_animation_sprite_manifest(KIKI_BREATH_FRAME_COUNT, KIKI_BLINK_FRAME_COUNT);

        apply_kiki_idle_animation(&mut snapshot, &sprites, Some(kiki_blink_sprite_id(3)));

        assert_eq!(snapshot.stage[0].sprite, KIKI_BASE_SPRITE);
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
    fn vn_overlay_layout_config_round_trips_without_edit_buffer() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir
            .path()
            .join("scenes")
            .join(VN_OVERLAY_LAYOUT_CONFIG_FILE);
        let mut overrides = VnOverlayDebugOverrides::default();
        overrides.dialogue_text_inset_cols = 9;
        overrides.composer_text_inset_rows = 3;
        overrides.dialogue_panel_opacity = 0.35;
        overrides.composer_nameplate_opacity = 0.72;
        overrides.editing_buffer = Some("partial".to_string());

        save_vn_overlay_layout_config_to_path(&path, &overrides).unwrap();
        let saved = std::fs::read_to_string(&path).unwrap();
        assert!(!saved.contains("editing_buffer"));

        let loaded = load_vn_overlay_layout_config_from_path(&path).unwrap();
        assert_eq!(loaded.dialogue_text_inset_cols, 9);
        assert_eq!(loaded.composer_text_inset_rows, 3);
        assert!((loaded.dialogue_panel_opacity - 0.35).abs() < 0.001);
        assert!((loaded.composer_nameplate_opacity - 0.72).abs() < 0.001);
        assert_eq!(loaded.editing_buffer, None);
    }

    #[test]
    fn vn_overlay_layout_config_loads_previous_schema_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir
            .path()
            .join("scenes")
            .join(VN_OVERLAY_LAYOUT_CONFIG_FILE);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            r#"{
  "dialogue_margin_ratio": 0.18,
  "composer_margin_ratio": 0.04,
  "dialogue_top_ratio": 0.06,
  "dialogue_bottom_ratio": 0.66,
  "composer_height_rows": 7,
  "dialogue_nameplate_height_rows": 3,
  "composer_nameplate_height_rows": 3,
  "dialogue_nameplate_inset_cols": 4,
  "composer_nameplate_inset_cols": 4,
  "dialogue_text_inset_cols": 8,
  "composer_text_inset_cols": 7,
  "dialogue_text_inset_rows": 2,
  "composer_text_inset_rows": 1,
  "selected_param": 12
}"#,
        )
        .unwrap();

        let loaded = load_vn_overlay_layout_config_from_path(&path).unwrap();

        assert_eq!(loaded.dialogue_text_inset_cols, 8);
        assert_eq!(
            loaded.dialogue_nameplate_text_inset_cols,
            VN_OVERLAY_DIALOGUE_NAMEPLATE_TEXT_INSET_COLS
        );
        assert_eq!(
            loaded.composer_nameplate_text_inset_rows,
            VN_OVERLAY_COMPOSER_NAMEPLATE_TEXT_INSET_ROWS
        );
        assert!((loaded.dialogue_panel_opacity - VN_OVERLAY_PANEL_OPACITY).abs() < 0.001);
        assert!((loaded.composer_panel_opacity - VN_OVERLAY_PANEL_OPACITY).abs() < 0.001);
        assert!((loaded.dialogue_nameplate_opacity - VN_OVERLAY_NAMEPLATE_OPACITY).abs() < 0.001);
        assert!((loaded.composer_nameplate_opacity - VN_OVERLAY_NAMEPLATE_OPACITY).abs() < 0.001);
    }

    #[test]
    fn persistable_vn_overlay_layout_ignores_transient_editing() {
        let mut before = VnOverlayDebugOverrides::default();
        let mut after = before.clone();
        after.editing_buffer = Some("0.250".to_string());

        assert_eq!(
            persistable_vn_overlay_layout(&before),
            persistable_vn_overlay_layout(&after)
        );

        before.dialogue_panel_opacity = 0.25;
        assert_ne!(
            persistable_vn_overlay_layout(&before),
            persistable_vn_overlay_layout(&after)
        );
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
    fn scene_dialogue_scrollback_moves_within_bounds() {
        let mut scroll = SceneDialogueScrollback {
            offset: 1,
            ..SceneDialogueScrollback::default()
        };
        let metrics = VnDialogueScrollMetrics {
            total_lines: 20,
            visible_rows: 5,
            scroll_offset: 1,
            max_scroll_offset: 3,
        };

        apply_dialogue_scroll_wheel(
            &mut scroll,
            metrics,
            MouseButtons::VERT_WHEEL | MouseButtons::WHEEL_POSITIVE,
        );
        assert_eq!(scroll.offset, 2);

        apply_dialogue_scroll_wheel(&mut scroll, metrics, MouseButtons::VERT_WHEEL);
        assert_eq!(scroll.offset, 1);

        scroll.offset = 10;
        apply_dialogue_scroll_wheel(&mut scroll, metrics, MouseButtons::VERT_WHEEL);
        assert_eq!(scroll.offset, 3);
    }

    #[test]
    fn scene_dialogue_scrollback_moves_by_page_keys() {
        let metrics = VnDialogueScrollMetrics {
            total_lines: 30,
            visible_rows: 6,
            scroll_offset: 0,
            max_scroll_offset: 12,
        };
        let mut scroll = SceneDialogueScrollback::default();

        assert!(apply_dialogue_scroll_key(
            &mut scroll,
            metrics,
            VisualInput::ScrollDialogueUp,
        ));
        assert_eq!(scroll.offset, 5);

        apply_dialogue_scroll_key(&mut scroll, metrics, VisualInput::ScrollDialogueUp);
        apply_dialogue_scroll_key(&mut scroll, metrics, VisualInput::ScrollDialogueUp);
        assert_eq!(scroll.offset, 12);

        assert!(apply_dialogue_scroll_key(
            &mut scroll,
            metrics,
            VisualInput::ScrollDialogueDown,
        ));
        assert_eq!(scroll.offset, 7);

        apply_dialogue_scroll_key(&mut scroll, metrics, VisualInput::ScrollDialogueDown);
        apply_dialogue_scroll_key(&mut scroll, metrics, VisualInput::ScrollDialogueDown);
        assert_eq!(scroll.offset, 0);
    }

    #[test]
    fn scene_dialogue_scrollback_page_keys_are_mapped() {
        assert_eq!(
            visual_input_from_key(KeyCode::PageUp),
            VisualInput::ScrollDialogueUp
        );
        assert_eq!(
            visual_input_from_key(KeyCode::PageDown),
            VisualInput::ScrollDialogueDown
        );
        assert_eq!(
            visual_input_from_key(KeyCode::UpArrow),
            VisualInput::Previous
        );
        assert_eq!(visual_input_from_key(KeyCode::DownArrow), VisualInput::Next);
    }

    #[test]
    fn activate_resets_dialogue_scroll_but_selection_does_not() {
        assert!(visual_input_resets_dialogue_scroll(VisualInput::Activate));
        assert!(!visual_input_resets_dialogue_scroll(VisualInput::Next));
        assert!(!visual_input_resets_dialogue_scroll(VisualInput::Previous));
        assert!(!visual_input_resets_dialogue_scroll(VisualInput::Left));
        assert!(!visual_input_resets_dialogue_scroll(VisualInput::Right));
        assert!(!visual_input_resets_dialogue_scroll(
            VisualInput::ScrollDialogueUp
        ));
        assert!(!visual_input_resets_dialogue_scroll(
            VisualInput::ScrollDialogueDown
        ));
    }

    #[test]
    fn scene_dialogue_scroll_wheel_is_bounded_to_dialogue_panel() {
        let mut scene = VisualScene::demo();
        scene.stage = VisualStage {
            layers: vec![VisualStageLayer {
                layer_id: "background".to_string(),
                zorder: 0,
                displayables: vec![VisualStageDisplayable {
                    tag: "background".to_string(),
                    sprite: "vn.background.school_classroom".to_string(),
                    placement: VisualStagePlacement::Fullscreen,
                    zorder: 0,
                    visible: true,
                }],
            }],
        };
        let mut runtime = SceneRuntime::new(scene).unwrap();
        for idx in 0..8 {
            runtime.mark_compose_running("Compose running", &format!("prompt {idx}"));
            runtime.mark_compose_succeeded("Codex", &"reply ".repeat(40));
        }
        let panel = runtime.vn_dialogue_panel_rect(100, 30).unwrap();
        let mut scroll = SceneDialogueScrollback::default();

        assert!(!handle_dialogue_scroll_wheel(
            &runtime,
            &mut scroll,
            100,
            30,
            1,
            1,
            MouseButtons::VERT_WHEEL | MouseButtons::WHEEL_POSITIVE,
        ));
        assert_eq!(scroll.offset, 0);

        assert!(handle_dialogue_scroll_wheel(
            &runtime,
            &mut scroll,
            100,
            30,
            panel.col.saturating_add(1) as u16,
            panel.row.saturating_add(1) as u16,
            MouseButtons::VERT_WHEEL | MouseButtons::WHEEL_POSITIVE,
        ));
        assert_eq!(scroll.offset, 1);
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
        assert_eq!(dock.history.last().map(String::as_str), Some("h"));
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
        let nameplate = layout.composer_nameplate_text.unwrap();
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
        assert!(!input.contains("last:"));
        assert!(input.contains("type here; enter submits"));
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
    fn first_voice_reveal_delay_requires_first_unmuted_speakable_audio() {
        let segments = vec![SpeakableSegment {
            speaker: Some("Codex".to_string()),
            text: "Ready.".to_string(),
            source: SpeakableSource::ComposeReply,
        }];

        assert!(should_delay_first_voice_reveal(
            true, false, false, false, &segments
        ));
        assert!(!should_delay_first_voice_reveal(
            false, false, false, false, &segments
        ));
        assert!(!should_delay_first_voice_reveal(
            true, true, false, false, &segments
        ));
        assert!(!should_delay_first_voice_reveal(
            true, false, true, false, &segments
        ));
        assert!(!should_delay_first_voice_reveal(
            true, false, false, true, &segments
        ));
        assert!(!should_delay_first_voice_reveal(
            true,
            false,
            false,
            false,
            &[]
        ));
    }

    #[test]
    fn compose_result_speakable_segments_preview_plain_output() {
        let result = ComposeBackendResult {
            prompt: "status".to_string(),
            stdout: "Here is the answer.\n/Users/julianabeleda/env/gameterm\n".to_string(),
            stderr: String::new(),
            exit_code: Some(0),
            label: ComposeBackendLabel::Codex,
        };

        let segments = compose_result_speakable_segments(&result);

        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].speaker.as_deref(), Some("Codex"));
        assert_eq!(segments[0].text, "Here is the answer.");
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
    fn compose_result_speakable_segments_preview_structured_patch_dialogue() {
        let raw_output = r#"{"status":"Task updated","patch":{"scene_patch_version":1,"dialogue":{"speaker":"Kiki","text":"The stage is ready.","append_history":true}}}"#;
        let result = ComposeBackendResult {
            prompt: "status".to_string(),
            stdout: raw_output.to_string(),
            stderr: String::new(),
            exit_code: Some(0),
            label: ComposeBackendLabel::Codex,
        };

        let segments = compose_result_speakable_segments(&result);

        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].speaker.as_deref(), Some("Kiki"));
        assert_eq!(segments[0].text, "The stage is ready.");
    }

    #[test]
    fn fake_codex_compose_result_renders_fake_speaker() {
        let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
        let segments =
            apply_compose_backend_result(&mut runtime, fake_codex_compose_result("hi".to_string()));

        let snapshot = runtime.render_snapshot();
        assert_eq!(snapshot.dialogue_speaker, "Fake Codex");
        assert_eq!(snapshot.dialogue, "Fake Codex received: hi");
        assert_eq!(snapshot.status, "Fake Codex succeeded");
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].speaker.as_deref(), Some("Fake Codex"));
        assert_eq!(segments[0].text, "Fake Codex received: hi");
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
    fn scene_voice_debug_menu_opens_from_plain_v_only() {
        assert!(is_voice_debug_menu_open_key(
            KeyCode::Char('v'),
            Modifiers::NONE
        ));
        assert!(is_voice_debug_menu_open_key(
            KeyCode::Char('V'),
            Modifiers::NONE
        ));
        assert!(!is_voice_debug_menu_open_key(
            KeyCode::Char('v'),
            Modifiers::ALT
        ));
    }

    #[test]
    fn scene_voice_debug_frame_replaces_bounded_top_lines() {
        let frame = "one\r\ntwo\r\nthree\r\n".to_string();
        let runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
        let config = SceneSttConfig::whisper_default();
        let state = SceneSttState::default();
        let mut debug = SceneVoiceDebugState::new(&config, &state);
        debug.open_menu();
        let rendered = apply_voice_debug_frame(frame, 20, 3, &runtime, &debug);

        let lines = rendered.lines().collect::<Vec<_>>();
        assert_eq!(lines[0], "Scene Voice Debug");
        assert_eq!(lines[1], "[jk: select] [enter:");
    }

    #[test]
    fn scene_voice_debug_test_mode_records_transcript_without_hiding_config() {
        let config = SceneSttConfig::whisper_default();
        let state = SceneSttState::default();
        let mut debug = SceneVoiceDebugState::new(&config, &state);
        debug.open_menu();
        debug.select_next();
        assert_eq!(debug.toggle_selected(), "Voice test mode enabled");
        debug.apply_result(&SceneSttResult {
            status: "Voice transcript ready".to_string(),
            transcript: Some("hello scene".to_string()),
            auto_submit: false,
            error: None,
        });

        let lines = debug.render_lines().join("\n");
        assert!(lines.contains("Mode: test recognition only"));
        assert!(lines.contains("Backend: whisper"));
        assert!(lines.contains("Last transcript: hello scene"));
        assert!(lines.contains("Fake Codex backend"));
    }

    #[test]
    fn scene_voice_debug_menu_toggles_selected_items() {
        let config = SceneSttConfig::whisper_default();
        let state = SceneSttState::default();
        let mut debug = SceneVoiceDebugState::new(&config, &state);
        let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
        let mut compose_debug_backend = SceneComposeDebugBackend::RealCodex;

        assert_eq!(debug.open_menu(), "Voice debug menu opened");
        assert!(debug.menu_open);
        assert!(debug.visible);

        assert_eq!(
            handle_voice_debug_menu_key(
                KeyCode::Enter,
                &mut runtime,
                &mut debug,
                false,
                false,
                &mut compose_debug_backend,
            ),
            VoiceDebugMenuEffect::HANDLED
        );
        assert!(!debug.visible);

        assert_eq!(
            handle_voice_debug_menu_key(
                KeyCode::DownArrow,
                &mut runtime,
                &mut debug,
                false,
                false,
                &mut compose_debug_backend,
            ),
            VoiceDebugMenuEffect::HANDLED
        );
        assert_eq!(
            handle_voice_debug_menu_key(
                KeyCode::Enter,
                &mut runtime,
                &mut debug,
                false,
                false,
                &mut compose_debug_backend,
            ),
            VoiceDebugMenuEffect::HANDLED
        );
        assert!(debug.test_mode);

        assert_eq!(
            handle_voice_debug_menu_key(
                KeyCode::DownArrow,
                &mut runtime,
                &mut debug,
                false,
                false,
                &mut compose_debug_backend,
            ),
            VoiceDebugMenuEffect::HANDLED
        );
        assert_eq!(
            handle_voice_debug_menu_key(
                KeyCode::Enter,
                &mut runtime,
                &mut debug,
                false,
                false,
                &mut compose_debug_backend,
            ),
            VoiceDebugMenuEffect::RESET_COMPOSE_DIALOGUE
        );
        assert!(compose_debug_backend.is_fake());
        assert!(debug.fake_codex_backend);

        assert_eq!(
            handle_voice_debug_menu_key(
                KeyCode::Tab,
                &mut runtime,
                &mut debug,
                false,
                false,
                &mut compose_debug_backend,
            ),
            VoiceDebugMenuEffect::HANDLED
        );
        assert!(!debug.menu_open);
    }

    #[test]
    fn scene_stt_hold_to_talk_uses_shift_command_modifiers() {
        let shift_command = Modifiers::SHIFT | Modifiers::SUPER;
        assert!(is_stt_hold_key(KeyCode::LeftWindows, shift_command));
        assert!(is_stt_hold_key(KeyCode::LeftShift, shift_command));
        assert!(!is_stt_hold_key(KeyCode::LeftWindows, Modifiers::SUPER));
        assert!(!is_stt_hold_key(KeyCode::LeftShift, Modifiers::SHIFT));
        assert!(!is_stt_hold_key(KeyCode::Char(' '), shift_command));
        assert!(is_stt_hold_release_key(KeyCode::LeftWindows));
        assert!(is_stt_hold_release_key(KeyCode::LeftShift));
        assert!(!is_stt_hold_release_key(KeyCode::Char(' ')));
    }

    #[test]
    fn scene_voice_debug_fake_codex_toggle_blocks_while_compose_runs() {
        let config = SceneSttConfig::whisper_default();
        let state = SceneSttState::default();
        let mut debug = SceneVoiceDebugState::new(&config, &state);
        let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
        let mut compose_debug_backend = SceneComposeDebugBackend::RealCodex;
        debug.open_menu();
        debug.select_next();
        debug.select_next();

        assert_eq!(
            handle_voice_debug_menu_key(
                KeyCode::Enter,
                &mut runtime,
                &mut debug,
                false,
                true,
                &mut compose_debug_backend,
            ),
            VoiceDebugMenuEffect::HANDLED
        );
        assert_eq!(compose_debug_backend, SceneComposeDebugBackend::RealCodex);
        assert!(!debug.fake_codex_backend);
        assert_eq!(
            runtime.render_snapshot().status,
            "Compose debug backend toggle unavailable: compose is running"
        );
    }

    #[test]
    fn scene_voice_debug_fake_codex_toggle_clears_dialogue_history() {
        let config = SceneSttConfig::whisper_default();
        let state = SceneSttState::default();
        let mut debug = SceneVoiceDebugState::new(&config, &state);
        let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
        let mut compose_debug_backend = SceneComposeDebugBackend::RealCodex;
        runtime.mark_compose_running("Codex running: hi", "hi");
        runtime.mark_compose_succeeded("Codex", "hello");
        debug.open_menu();
        debug.select_next();
        debug.select_next();

        assert_eq!(
            handle_voice_debug_menu_key(
                KeyCode::Enter,
                &mut runtime,
                &mut debug,
                false,
                false,
                &mut compose_debug_backend,
            ),
            VoiceDebugMenuEffect::RESET_COMPOSE_DIALOGUE
        );
        assert!(runtime.render_snapshot().dialogue_history.is_empty());
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
            timeout: std::time::Duration::from_secs(90),
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
            timeout: std::time::Duration::from_secs(90),
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
            timeout: std::time::Duration::from_secs(90),
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
