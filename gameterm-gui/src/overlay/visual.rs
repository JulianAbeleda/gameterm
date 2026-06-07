use anyhow::Context;
use gameterm_term::TerminalSize;
use gameterm_visual::{
    SceneRuntime, VisualInput, VisualInteractiveDebugMenu, VisualMode, VisualModeOutcome,
    VisualScene, VisualSceneSource, VisualView,
};
use mux::termwiztermtab::TermWizTerminal;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;
use termwiz::input::{InputEvent, KeyCode, KeyEvent, MouseButtons, MouseEvent};
use termwiz::surface::Change;
use termwiz::terminal::Terminal;
use window::Window;

#[cfg(test)]
use gameterm_visual::{
    vn_overlay_layout, VisualRenderSnapshot, VisualResolvedSprite, VisualSpriteManifestStatus,
    VnDialogueScrollMetrics, VnOverlayRect,
};
#[cfg(test)]
use std::path::Path;
#[cfg(test)]
use termwiz::input::Modifiers;

#[path = "visual_command_dispatch.rs"]
mod visual_command_dispatch;
#[path = "visual_compose_dock.rs"]
mod visual_compose_dock;
#[path = "visual_compose_result.rs"]
mod visual_compose_result;
#[path = "visual_dialogue_scroll.rs"]
mod visual_dialogue_scroll;
#[path = "visual_event_drain.rs"]
mod visual_event_drain;
#[path = "visual_frame.rs"]
mod visual_frame;
#[path = "visual_input_keys.rs"]
mod visual_input_keys;
#[path = "visual_kiki_idle.rs"]
mod visual_kiki_idle;
#[path = "visual_overlay_session.rs"]
mod visual_overlay_session;
#[path = "visual_render.rs"]
mod visual_render;
#[path = "visual_scene_files.rs"]
mod visual_scene_files;
#[path = "visual_scene_patches.rs"]
mod visual_scene_patches;
#[path = "visual_voice_debug.rs"]
mod visual_voice_debug;
#[path = "visual_voice_events.rs"]
mod visual_voice_events;

#[cfg(test)]
use super::visual_compose::ComposeBackendLabel;
#[cfg(test)]
use super::visual_compose::ComposeBackendResult;
use super::visual_compose::{compose_running_status, spawn_compose_backend, ComposeBackendRequest};
use super::visual_stt::{spawn_mic_test, spawn_stt_backend, SceneSttConfig};
#[cfg(test)]
use super::visual_stt::{SceneMicDevice, SceneSttResult, SceneSttState};
use super::visual_tts::{SceneTtsConfig, SceneTtsRequest};
#[cfg(test)]
use super::visual_tts::{SpeakableSegment, SpeakableSource};
#[cfg(test)]
use visual_command_dispatch::write_story_state_file;
use visual_command_dispatch::{dispatch_pending_action, RunCommandDispatch};
use visual_compose_dock::SceneComposeAction;
#[cfg(test)]
use visual_compose_dock::SceneComposeDock;
use visual_compose_result::{
    apply_compose_backend_result, compose_result_speakable_segments, fake_codex_compose_result,
    should_delay_first_voice_reveal, PendingFirstVoiceReveal,
};
#[cfg(test)]
use visual_compose_result::{sanitize_compose_output, COMPOSE_OUTPUT_LIMIT};
#[cfg(test)]
use visual_dialogue_scroll::{
    apply_dialogue_scroll_key, apply_dialogue_scroll_wheel, SceneDialogueScrollback,
};
use visual_dialogue_scroll::{handle_dialogue_scroll_key, handle_dialogue_scroll_wheel};
use visual_event_drain::{
    drain_command_results, drain_compose_results, drain_mic_test_results, drain_stt_results,
    drain_tts_results,
};
#[cfg(test)]
use visual_frame::replace_last_screen_line;
use visual_input_keys::{
    is_stt_hold_key, is_stt_hold_release_key, is_tts_toggle_key, visual_input_from_key,
    visual_input_resets_dialogue_scroll,
};
use visual_kiki_idle::*;
#[cfg(test)]
use visual_overlay_session::SceneComposeDebugBackend;
use visual_overlay_session::VisualOverlaySession;
#[cfg(test)]
use visual_render::apply_voice_debug_frame;
use visual_render::{render_error, render_runtime, render_runtime_with_compose_and_scroll};
pub(crate) use visual_scene_files::SceneOverlayLaunchOptions;
use visual_scene_files::*;
use visual_scene_patches::*;
use visual_voice_debug::VoiceDebugMenuEffect;

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
    let (compose_tx, compose_rx) = mpsc::channel();
    let (tts_tx, tts_rx) = mpsc::channel();
    let tts_config = launch_options
        .tts_config
        .unwrap_or_else(SceneTtsConfig::from_env);
    let (stt_tx, stt_rx) = mpsc::channel();
    let stt_config = launch_options
        .stt_config
        .unwrap_or_else(SceneSttConfig::from_env);
    let (mic_test_tx, mic_test_rx) = mpsc::channel();
    let mut session = VisualOverlaySession::new(tts_config, tts_tx.clone(), stt_config);

    loop {
        let mut needs_render = drain_command_results(&command_rx, &mut runtime);
        needs_render |= drain_compose_results(
            &compose_rx,
            &mut runtime,
            &mut session.compose_backend_running,
            &mut session.dialogue_scroll,
            session.sync_first_voice_reveal,
            &mut session.first_voice_reveal_done,
            &mut session.pending_first_voice_reveal,
            &session.tts_state,
            &session.tts_worker,
        );
        needs_render |= drain_tts_results(
            &tts_rx,
            &mut runtime,
            &mut session.tts_state,
            &mut session.dialogue_scroll,
            &mut session.pending_first_voice_reveal,
            &mut session.first_voice_reveal_done,
        );
        needs_render |= drain_stt_results(
            &stt_rx,
            &mut runtime,
            &mut session.stt_session,
            &mut session.dialogue_scroll,
            &mut session.compose_dock,
            &mut session.stt_state,
            &mut session.compose_backend_running,
            &compose_tx,
            &scene_path,
            pane_id,
        );
        needs_render |= drain_mic_test_results(
            &mic_test_rx,
            &mut runtime,
            &mut session.dialogue_scroll,
            &mut session.mic_test_running,
        );
        if needs_render {
            if let Some(runtime) = runtime.as_ref() {
                render_runtime_with_compose_and_scroll(
                    &mut term,
                    runtime,
                    &sprite_manifest,
                    &session.compose_dock,
                    &session.dialogue_scroll,
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
                session.dialogue_scroll.reset_to_bottom();
                file_watcher.refresh(&scene_path, &sprite_manifest_path);
            }
            if let Some(path) = patch_inbox.changed_path() {
                if let Some(runtime) = runtime.as_mut() {
                    apply_scene_patch_file(&mut term, runtime, &sprite_manifest, &path)?;
                    session.dialogue_scroll.reset_to_bottom();
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
                    session.dialogue_scroll.reset_to_bottom();
                }
            }
            if let Some(runtime) = runtime.as_ref() {
                let sprite = current_kiki_idle_sprite(&sprite_manifest);
                if runtime_has_kiki_idle_animation(runtime, &sprite_manifest) {
                    if session.last_idle_sprite != sprite {
                        render_runtime_with_compose_and_scroll(
                            &mut term,
                            runtime,
                            &sprite_manifest,
                            &session.compose_dock,
                            &session.dialogue_scroll,
                        )?;
                        session.last_idle_sprite = sprite;
                    }
                } else {
                    session.last_idle_sprite = None;
                }
            }
            continue;
        };
        match input {
            InputEvent::Key(KeyEvent { key, modifiers }) => {
                if let Some(runtime) = runtime.as_mut() {
                    if session.stt_state.is_running() && matches!(key, KeyCode::Escape) {
                        if let Some(stt_session) = session.stt_session.take() {
                            stt_session.cancel();
                        }
                        session.dialogue_scroll.voice_hold_active = false;
                        runtime.mark_action_status(session.stt_state.mark_canceling());
                        session
                            .dialogue_scroll
                            .voice_debug
                            .sync_status(session.stt_state.last_status());
                        render_runtime_with_compose_and_scroll(
                            &mut term,
                            runtime,
                            &sprite_manifest,
                            &session.compose_dock,
                            &session.dialogue_scroll,
                        )?;
                        continue;
                    }
                    if runtime.view() != VisualView::VnLayoutDebugger
                        && is_tts_toggle_key(key, modifiers)
                    {
                        runtime.mark_action_status(session.tts_state.toggle_muted());
                        render_runtime_with_compose_and_scroll(
                            &mut term,
                            runtime,
                            &sprite_manifest,
                            &session.compose_dock,
                            &session.dialogue_scroll,
                        )?;
                        continue;
                    }
                    if is_stt_hold_key(key, modifiers) {
                        if !session.stt_state.is_running() {
                            runtime.mark_action_status(session.stt_state.mark_started());
                            if session.dialogue_scroll.voice_debug.test_mode {
                                runtime.mark_action_status("Voice test listening");
                            }
                            session
                                .dialogue_scroll
                                .voice_debug
                                .sync_status(session.stt_state.last_status());
                            session.stt_session = Some(spawn_stt_backend(
                                session.selected_stt_config(),
                                stt_tx.clone(),
                            ));
                            session.dialogue_scroll.voice_hold_active = true;
                        }
                        render_runtime_with_compose_and_scroll(
                            &mut term,
                            runtime,
                            &sprite_manifest,
                            &session.compose_dock,
                            &session.dialogue_scroll,
                        )?;
                        continue;
                    }
                    // While the VN layout debugger menu is open it owns every
                    // key. The compose dock must not intercept text/navigation
                    // input before the debugger can select, adjust, or edit.
                    let in_layout_debug = runtime.view() == VisualView::VnLayoutDebugger;
                    if !runtime.scene().stage.is_empty() && !in_layout_debug {
                        match session.compose_dock.handle_key(key) {
                            SceneComposeAction::Consumed => {
                                render_runtime_with_compose_and_scroll(
                                    &mut term,
                                    runtime,
                                    &sprite_manifest,
                                    &session.compose_dock,
                                    &session.dialogue_scroll,
                                )?;
                                continue;
                            }
                            SceneComposeAction::Submitted(prompt) => {
                                if session.compose_backend_running {
                                    runtime
                                        .mark_compose_failed("Compose backend is already running");
                                    runtime.mark_action_status(
                                        "Compose busy: finish the current reply first",
                                    );
                                    render_runtime_with_compose_and_scroll(
                                        &mut term,
                                        runtime,
                                        &sprite_manifest,
                                        &session.compose_dock,
                                        &session.dialogue_scroll,
                                    )?;
                                    continue;
                                }
                                session.compose_dock.mark_submitted(&prompt);
                                let running_status = if session.compose_debug_backend.is_fake() {
                                    format!("Fake Codex running: {prompt}")
                                } else {
                                    compose_running_status(&prompt)
                                };
                                runtime.mark_compose_running(running_status, &prompt);
                                session.dialogue_scroll.reset_to_bottom();
                                if session.compose_debug_backend.is_fake() {
                                    let result = fake_codex_compose_result(prompt);
                                    let speakable_segments =
                                        compose_result_speakable_segments(&result);
                                    if should_delay_first_voice_reveal(
                                        session.sync_first_voice_reveal,
                                        session.first_voice_reveal_done,
                                        session.pending_first_voice_reveal.is_some(),
                                        session.tts_state.is_muted(),
                                        &speakable_segments,
                                    ) {
                                        for segment in speakable_segments {
                                            session.tts_worker.speak(SceneTtsRequest { segment });
                                        }
                                        session.pending_first_voice_reveal =
                                            Some(PendingFirstVoiceReveal { result });
                                        runtime.mark_action_status("Voice preparing first reply");
                                    } else {
                                        let speakable_segments =
                                            apply_compose_backend_result(runtime, result);
                                        if !session.first_voice_reveal_done
                                            && !speakable_segments.is_empty()
                                        {
                                            session.first_voice_reveal_done = true;
                                        }
                                        if !session.tts_state.is_muted() {
                                            for segment in speakable_segments {
                                                session
                                                    .tts_worker
                                                    .speak(SceneTtsRequest { segment });
                                            }
                                        }
                                    }
                                    render_runtime_with_compose_and_scroll(
                                        &mut term,
                                        runtime,
                                        &sprite_manifest,
                                        &session.compose_dock,
                                        &session.dialogue_scroll,
                                    )?;
                                    continue;
                                }
                                session.compose_backend_running = true;
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
                                    &session.compose_dock,
                                    &session.dialogue_scroll,
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
                            &mut session.dialogue_scroll,
                            visual_input,
                            term.get_screen_size()?,
                        ) {
                            render_runtime_with_compose_and_scroll(
                                &mut term,
                                runtime,
                                &sprite_manifest,
                                &session.compose_dock,
                                &session.dialogue_scroll,
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
                    session.dialogue_scroll.reset_to_bottom();
                    file_watcher.refresh(&scene_path, &sprite_manifest_path);
                    continue;
                }
                if let Some(runtime) = runtime.as_mut() {
                    if in_layout_debug {
                        let debug_effect = handle_scene_debug_session_input(
                            runtime,
                            &mut session,
                            visual_input,
                            &mic_test_tx,
                        );
                        if debug_effect.handled {
                            if debug_effect.reset_compose_dialogue {
                                session.first_voice_reveal_done = false;
                                session.pending_first_voice_reveal = None;
                                session.dialogue_scroll.reset_to_bottom();
                            }
                            render_runtime_with_compose_and_scroll(
                                &mut term,
                                runtime,
                                &sprite_manifest,
                                &session.compose_dock,
                                &session.dialogue_scroll,
                            )?;
                            continue;
                        }
                    }
                    let vn_layout_before = runtime
                        .vn_layout_debug_overrides()
                        .map(persistable_vn_overlay_layout);
                    if runtime.handle_input(visual_input) == VisualModeOutcome::Exit {
                        break;
                    }
                    if visual_input_resets_dialogue_scroll(visual_input) {
                        session.dialogue_scroll.reset_to_bottom();
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
                        &session.compose_dock,
                        &session.dialogue_scroll,
                    )?;
                }
            }
            InputEvent::KeyUp(KeyEvent { key, modifiers: _ }) => {
                if let Some(runtime) = runtime.as_mut() {
                    if is_stt_hold_release_key(key) && session.stt_state.is_running() {
                        if let Some(stt_session) = session.stt_session.take() {
                            stt_session.stop();
                        }
                        session.dialogue_scroll.voice_hold_active = false;
                        runtime.mark_action_status(session.stt_state.mark_processing());
                        if session.dialogue_scroll.voice_debug.test_mode {
                            runtime.mark_action_status("Voice test processing");
                        }
                        session
                            .dialogue_scroll
                            .voice_debug
                            .sync_status(session.stt_state.last_status());
                        render_runtime_with_compose_and_scroll(
                            &mut term,
                            runtime,
                            &sprite_manifest,
                            &session.compose_dock,
                            &session.dialogue_scroll,
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
                        &mut session.dialogue_scroll,
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
                            &session.compose_dock,
                            &session.dialogue_scroll,
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
                        session.dialogue_scroll.offset,
                    );
                    session.dialogue_scroll.clamp(metrics.max_scroll_offset);
                    render_runtime_with_compose_and_scroll(
                        &mut term,
                        runtime,
                        &sprite_manifest,
                        &session.compose_dock,
                        &session.dialogue_scroll,
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

fn handle_scene_debug_session_input(
    runtime: &mut SceneRuntime,
    session: &mut VisualOverlaySession,
    input: VisualInput,
    mic_test_tx: &mpsc::Sender<super::visual_stt::SceneMicTestResult>,
) -> VoiceDebugMenuEffect {
    if runtime.view() != VisualView::VnLayoutDebugger {
        return VoiceDebugMenuEffect::IGNORED;
    }
    let is_action = matches!(
        input,
        VisualInput::Activate | VisualInput::Left | VisualInput::Right
    );
    if !is_action || runtime.interactive_debug_row() == 0 {
        return VoiceDebugMenuEffect::IGNORED;
    }

    match runtime.interactive_debug_menu() {
        VisualInteractiveDebugMenu::Voice => match runtime.interactive_debug_row() {
            1 => {
                runtime
                    .mark_action_status(session.dialogue_scroll.voice_debug.toggle_diagnostics());
                VoiceDebugMenuEffect::HANDLED
            }
            2 => {
                if session.stt_state.is_running() {
                    runtime.mark_action_status(
                        "Voice test mode toggle unavailable: voice is listening",
                    );
                } else {
                    runtime.mark_action_status(
                        session.dialogue_scroll.voice_debug.toggle_voice_test_mode(),
                    );
                }
                VoiceDebugMenuEffect::HANDLED
            }
            3 => {
                runtime.mark_action_status(session.tts_state.toggle_muted());
                VoiceDebugMenuEffect::HANDLED
            }
            4 => {
                let delta = if input == VisualInput::Left { -1 } else { 1 };
                runtime.mark_action_status(session.cycle_selected_mic(delta));
                VoiceDebugMenuEffect::HANDLED
            }
            5 => {
                if session.mic_test_running {
                    runtime.mark_action_status("Mic test already running");
                } else {
                    session.mic_test_running = true;
                    let selected_label = session.selected_mic_label().to_string();
                    session
                        .dialogue_scroll
                        .voice_debug
                        .mark_mic_test_started(&selected_label);
                    runtime.mark_action_status(format!("Mic test listening: {selected_label}"));
                    spawn_mic_test(session.selected_mic_device(), mic_test_tx.clone());
                }
                VoiceDebugMenuEffect::HANDLED
            }
            _ => VoiceDebugMenuEffect::IGNORED,
        },
        VisualInteractiveDebugMenu::Compose => match runtime.interactive_debug_row() {
            1 => {
                if session.compose_backend_running {
                    runtime.mark_action_status(
                        "Compose debug backend toggle unavailable: compose is running",
                    );
                    VoiceDebugMenuEffect::HANDLED
                } else {
                    let status = session.compose_debug_backend.toggle();
                    session.dialogue_scroll.voice_debug.fake_codex_backend =
                        session.compose_debug_backend.is_fake();
                    runtime.clear_compose_history();
                    runtime.mark_action_status(status);
                    VoiceDebugMenuEffect::RESET_COMPOSE_DIALOGUE
                }
            }
            2 => {
                runtime.clear_compose_history();
                runtime.mark_action_status("Compose dialogue history cleared");
                VoiceDebugMenuEffect::RESET_COMPOSE_DIALOGUE
            }
            _ => VoiceDebugMenuEffect::IGNORED,
        },
        _ => VoiceDebugMenuEffect::IGNORED,
    }
}

#[cfg(test)]
mod tests {
    use super::super::visual_compose::{
        codex_compose_argv, codex_output_text, compose_backend_config, run_codex_compose_backend,
        CodexComposeConfig, ComposeBackendConfig,
    };
    use super::visual_voice_debug::SceneVoiceDebugState;
    use super::*;
    use gameterm_visual::{
        VisualSceneLoadStatus, VisualStage, VisualStageDisplayable, VisualStageLayer,
        VisualStagePlacement, VnOverlayDebugOverrides,
        VN_OVERLAY_COMPOSER_NAMEPLATE_TEXT_INSET_ROWS,
        VN_OVERLAY_DIALOGUE_NAMEPLATE_TEXT_INSET_COLS, VN_OVERLAY_NAMEPLATE_OPACITY,
        VN_OVERLAY_PANEL_OPACITY,
    };
    use std::collections::HashSet;

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
    fn scene_voice_debug_frame_only_replaces_debug_view_lines() {
        let frame = "one\r\ntwo\r\nthree\r\nfour\r\nfive\r\nsix\r\nseven\r\n".to_string();
        let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
        let config = SceneSttConfig::whisper_default();
        let state = SceneSttState::default();
        let debug = SceneVoiceDebugState::new(&config, &state);
        let scene_rendered = apply_voice_debug_frame(frame.clone(), 48, 3, &runtime, &debug);
        assert_eq!(scene_rendered, frame);

        runtime.toggle_debugger();
        runtime.handle_input(VisualInput::Right);
        runtime.handle_input(VisualInput::Right);
        let rendered = apply_voice_debug_frame(frame, 48, 7, &runtime, &debug);

        let lines = rendered.lines().collect::<Vec<_>>();
        assert_eq!(lines[5], "  Voice");
        assert!(lines[6].contains("Scene voice diagnostics"));
    }

    #[test]
    fn scene_voice_debug_test_mode_records_transcript_without_hiding_config() {
        let config = SceneSttConfig::whisper_default();
        let state = SceneSttState::default();
        let mut debug = SceneVoiceDebugState::new(&config, &state);
        assert_eq!(debug.toggle_voice_test_mode(), "Voice test mode enabled");
        debug.apply_result(&SceneSttResult {
            status: "Voice transcript ready".to_string(),
            transcript: Some("hello scene".to_string()),
            auto_submit: false,
            error: None,
        });

        let lines = debug.render_voice_lines(2).join("\n");
        assert!(lines.contains("Mode: test recognition only"));
        assert!(lines.contains("Backend: whisper"));
        assert!(lines.contains("Last transcript: hello scene"));
        assert!(lines.contains("TTS mute"));
    }

    #[test]
    fn scene_debug_menu_toggles_voice_items_from_central_menu() {
        let (tts_tx, _tts_rx) = mpsc::channel();
        let (mic_test_tx, _mic_test_rx) = mpsc::channel();
        let mut session = VisualOverlaySession::new(
            SceneTtsConfig::voicevox_default(),
            tts_tx,
            SceneSttConfig::whisper_default(),
        );
        let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
        runtime.toggle_debugger();
        runtime.handle_input(VisualInput::Right);
        runtime.handle_input(VisualInput::Right);
        assert_eq!(
            runtime.interactive_debug_menu(),
            VisualInteractiveDebugMenu::Voice
        );
        runtime.handle_input(VisualInput::Next);

        assert_eq!(
            handle_scene_debug_session_input(
                &mut runtime,
                &mut session,
                VisualInput::Activate,
                &mic_test_tx
            ),
            VoiceDebugMenuEffect::HANDLED
        );
        assert!(session.dialogue_scroll.voice_debug.visible);

        runtime.handle_input(VisualInput::Next);
        assert_eq!(
            handle_scene_debug_session_input(
                &mut runtime,
                &mut session,
                VisualInput::Right,
                &mic_test_tx
            ),
            VoiceDebugMenuEffect::HANDLED
        );
        assert!(session.dialogue_scroll.voice_debug.test_mode);
    }

    #[test]
    fn scene_debug_menu_cycles_microphone_from_voice_section() {
        let (tts_tx, _tts_rx) = mpsc::channel();
        let (mic_test_tx, _mic_test_rx) = mpsc::channel();
        let mut session = VisualOverlaySession::new_with_mic_devices(
            SceneTtsConfig::voicevox_default(),
            tts_tx,
            SceneSttConfig::whisper_default(),
            vec![
                SceneMicDevice {
                    name: "HD Pro Webcam C920".to_string(),
                    is_default: true,
                },
                SceneMicDevice {
                    name: "usethisphone707 Microphone".to_string(),
                    is_default: false,
                },
            ],
        );
        let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
        runtime.toggle_debugger();
        runtime.handle_input(VisualInput::Right);
        runtime.handle_input(VisualInput::Right);
        for _ in 0..4 {
            runtime.handle_input(VisualInput::Next);
        }

        assert_eq!(
            handle_scene_debug_session_input(
                &mut runtime,
                &mut session,
                VisualInput::Right,
                &mic_test_tx
            ),
            VoiceDebugMenuEffect::HANDLED
        );
        assert_eq!(session.selected_mic_label(), "HD Pro Webcam C920");
        assert!(session
            .selected_stt_config()
            .diagnostics_lines()
            .iter()
            .any(|line| line == "Mic: HD Pro Webcam C920"));

        assert_eq!(
            handle_scene_debug_session_input(
                &mut runtime,
                &mut session,
                VisualInput::Right,
                &mic_test_tx
            ),
            VoiceDebugMenuEffect::HANDLED
        );
        assert_eq!(session.selected_mic_label(), "usethisphone707 Microphone");

        let lines = session
            .dialogue_scroll
            .voice_debug
            .render_voice_lines(4)
            .join("\n");
        assert!(lines.contains("Microphone"));
        assert!(lines.contains("usethisphone707 Microphone"));
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
        let (tts_tx, _tts_rx) = mpsc::channel();
        let (mic_test_tx, _mic_test_rx) = mpsc::channel();
        let mut session = VisualOverlaySession::new(
            SceneTtsConfig::voicevox_default(),
            tts_tx,
            SceneSttConfig::whisper_default(),
        );
        let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
        runtime.toggle_debugger();
        for _ in 0..3 {
            runtime.handle_input(VisualInput::Right);
        }
        runtime.handle_input(VisualInput::Next);
        session.compose_backend_running = true;

        assert_eq!(
            handle_scene_debug_session_input(
                &mut runtime,
                &mut session,
                VisualInput::Activate,
                &mic_test_tx
            ),
            VoiceDebugMenuEffect::HANDLED
        );
        assert_eq!(
            session.compose_debug_backend,
            SceneComposeDebugBackend::RealCodex
        );
        assert!(!session.dialogue_scroll.voice_debug.fake_codex_backend);
        assert_eq!(
            runtime.render_snapshot().status,
            "Compose debug backend toggle unavailable: compose is running"
        );
    }

    #[test]
    fn scene_voice_debug_fake_codex_toggle_clears_dialogue_history() {
        let (tts_tx, _tts_rx) = mpsc::channel();
        let (mic_test_tx, _mic_test_rx) = mpsc::channel();
        let mut session = VisualOverlaySession::new(
            SceneTtsConfig::voicevox_default(),
            tts_tx,
            SceneSttConfig::whisper_default(),
        );
        let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
        runtime.mark_compose_running("Codex running: hi", "hi");
        runtime.mark_compose_succeeded("Codex", "hello");
        runtime.toggle_debugger();
        for _ in 0..3 {
            runtime.handle_input(VisualInput::Right);
        }
        runtime.handle_input(VisualInput::Next);

        assert_eq!(
            handle_scene_debug_session_input(
                &mut runtime,
                &mut session,
                VisualInput::Activate,
                &mic_test_tx
            ),
            VoiceDebugMenuEffect::RESET_COMPOSE_DIALOGUE
        );
        assert!(session.compose_debug_backend.is_fake());
        assert!(session.dialogue_scroll.voice_debug.fake_codex_backend);
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
