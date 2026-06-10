use anyhow::Context;
use gameterm_term::TerminalSize;
use gameterm_visual::{VisualInput, VisualMode, VisualModeOutcome, VisualSceneSource, VisualView};
use mux::termwiztermtab::TermWizTerminal;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;
use termwiz::input::{InputEvent, KeyCode, KeyEvent, MouseButtons, MouseEvent};
use termwiz::surface::Change;
use termwiz::terminal::Terminal;
use window::Window;

use super::super::visual_compose::{
    compose_running_status, spawn_compose_backend, ComposeBackendRequest,
};
use super::super::visual_stt::SceneSttConfig;
use super::super::visual_tts::SceneTtsConfig;
use super::super::visual_voice_hold::set_scene_voice_hold_active;
use super::visual_command_dispatch::{dispatch_pending_action, RunCommandDispatch};
use super::visual_compose_commands::{
    parse_scene_compose_command, scene_compose_help_text, SceneComposeCommand,
};
use super::visual_compose_dock::SceneComposeAction;
use super::visual_compose_result::{apply_compose_backend_result, fake_codex_compose_result};
use super::visual_dialogue_scroll::{handle_dialogue_scroll_key, handle_dialogue_scroll_wheel};
use super::visual_event_drain::{
    drain_command_results, drain_compose_results, drain_mic_test_results, drain_stt_results,
    drain_tts_results,
};
use super::visual_input_keys::{
    is_tts_toggle_key, visual_input_from_key, visual_input_resets_dialogue_scroll,
};
use super::visual_kiki_idle::{current_kiki_idle_sprite, runtime_has_kiki_idle_animation};
use super::visual_overlay_session::VisualOverlaySession;
use super::visual_render::{render_error, render_runtime_with_compose_and_scroll};
use super::visual_scene_debug_input::handle_scene_debug_session_input;
use super::visual_scene_files::{
    default_scene_path, default_sprite_manifest_path, initial_generated_scene_state,
    initial_scene_state, persist_vn_overlay_layout_if_changed, persistable_vn_overlay_layout,
    reload_active_scene, reload_generated_scene, SceneFileWatcher, SceneOverlayLaunchOptions,
    VisualSceneOverlaySource,
};
use super::visual_scene_patches::{
    apply_scene_patch_file, apply_scene_patch_json, ActiveSceneOverlay, ScenePatchInbox,
    ScenePatchNotificationSubscription,
};
use super::visual_voice_hold_flow::reconcile_scene_voice_hold_state;

pub(super) fn show_visual_scene_overlay_with_source(
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
    // Cozy-game entry: open to the boot "press start" screen, which advances to
    // the main menu and then into the scene.
    if let Some(runtime) = runtime.as_mut() {
        runtime.enter_boot();
    }
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
        needs_render |= drain_compose_results(&compose_rx, &mut runtime, &mut session);
        needs_render |= drain_tts_results(&tts_rx, &mut runtime, &mut session);
        needs_render |= drain_stt_results(
            &stt_rx,
            &mut runtime,
            &mut session,
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
        needs_render |=
            reconcile_scene_voice_hold_state(pane_id, &mut runtime, &mut session, &stt_tx);
        needs_render |= session.compose_wait_render_tick();
        if let Some(runtime) = runtime.as_mut() {
            needs_render |= session.advance_dialogue_reveal(runtime);
        }
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
                        set_scene_voice_hold_active(pane_id, false);
                        session.dialogue_scroll.cancel_voice_hold();
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
                    if session.compose_backend_running && matches!(key, KeyCode::Escape) {
                        // Esc during an in-flight compose request cancels the
                        // prompt instead of closing the overlay. The canceled
                        // result still arrives through the compose channel so
                        // completion stays on one path.
                        if let Some(cancel) = session.compose_cancel.take() {
                            cancel.cancel();
                        }
                        runtime.mark_action_status("Compose cancel requested");
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
                        session.sync_tts_debug();
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
                    // Boot, menu, and mode-cycle screens own keyboard input
                    // directly; the compose dock must not intercept it.
                    let in_shell = runtime.view().is_shell();
                    if !runtime.scene().stage.is_empty() && !in_layout_debug && !in_shell {
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
                                // Slash commands are local composer actions:
                                // they never spawn the backend and run even
                                // while a turn is in flight.
                                if let Some(command) = parse_scene_compose_command(&prompt) {
                                    session.compose_dock.mark_submitted(&prompt);
                                    match command {
                                        SceneComposeCommand::Model { name: Some(name) } => {
                                            runtime.mark_action_status(format!(
                                                "Compose model: {name}"
                                            ));
                                            session.compose_model = Some(name);
                                        }
                                        SceneComposeCommand::Model { name: None } => {
                                            runtime.mark_action_status(format!(
                                                "Compose model: {}",
                                                session.active_compose_model_label()
                                            ));
                                        }
                                        SceneComposeCommand::Clear => {
                                            runtime.clear_compose_history();
                                            session.dialogue_scroll.reset_to_bottom();
                                        }
                                        SceneComposeCommand::Help => {
                                            runtime.mark_action_status(scene_compose_help_text());
                                        }
                                        SceneComposeCommand::Unknown(name) => {
                                            runtime.mark_action_status(format!(
                                                "Unknown command /{name}; try /help"
                                            ));
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
                                if session.compose_backend_running {
                                    // Status feedback only, matching the voice
                                    // path: the running turn must not be failed
                                    // and the typed prompt stays in the dock.
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
                                session.interrupt_tts_queue();
                                let backend_prompt = runtime.compose_backend_prompt(&prompt);
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
                                    let speakable_segments = apply_compose_backend_result(
                                        runtime,
                                        result,
                                        !session.tts_state.is_muted(),
                                    );
                                    if !session.tts_state.is_muted() {
                                        session.enqueue_tts_segments(speakable_segments);
                                    } else {
                                        session.sync_tts_debug();
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
                                session.compose_cancel = Some(spawn_compose_backend(
                                    ComposeBackendRequest {
                                        prompt,
                                        backend_prompt,
                                        scene_path: Some(scene_path.display().to_string()),
                                        pane_id: Some(pane_id),
                                        model_override: session.compose_model.clone(),
                                    },
                                    compose_tx.clone(),
                                ));
                                session.compose_dock.begin_backend_wait();
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
                // Shell screens route Close/Reload through handle_input (Esc on
                // the menu backs up a level rather than closing the overlay),
                // so do not let the loop's own Close/Reload shortcuts fire.
                let in_shell =
                    runtime.as_ref().map_or(false, |runtime| runtime.view().is_shell());
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
                if visual_input == VisualInput::Close && !in_layout_debug && !in_shell {
                    break;
                }
                if visual_input == VisualInput::Reload && !in_layout_debug && !in_shell {
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
            InputEvent::KeyUp(KeyEvent { .. }) => {}
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
