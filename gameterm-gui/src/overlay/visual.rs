use gameterm_visual::VisualScene;
use mux::termwiztermtab::TermWizTerminal;
use std::path::PathBuf;
use window::Window;

#[path = "visual_command_dispatch.rs"]
mod visual_command_dispatch;
#[path = "visual_compose_commands.rs"]
mod visual_compose_commands;
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
#[path = "visual_loop.rs"]
mod visual_loop;
#[path = "visual_overlay_session.rs"]
mod visual_overlay_session;
#[path = "visual_render.rs"]
mod visual_render;
#[path = "visual_scene_debug_input.rs"]
mod visual_scene_debug_input;
#[path = "visual_scene_files.rs"]
mod visual_scene_files;
#[path = "visual_scene_patches.rs"]
mod visual_scene_patches;
#[path = "visual_text_selection.rs"]
mod visual_text_selection;
#[path = "visual_voice_debug.rs"]
mod visual_voice_debug;
#[path = "visual_voice_events.rs"]
mod visual_voice_events;
#[path = "visual_voice_hold_flow.rs"]
mod visual_voice_hold_flow;

use visual_render::{render_error, render_runtime};
pub(crate) use visual_scene_files::SceneOverlayLaunchOptions;
use visual_scene_files::VisualSceneOverlaySource;

pub(crate) fn show_visual_scene_overlay_with_options(
    term: TermWizTerminal,
    route_pane_id: Option<mux::pane::PaneId>,
    gui_window: Option<Window>,
    launch_options: SceneOverlayLaunchOptions,
) -> anyhow::Result<()> {
    visual_loop::show_visual_scene_overlay_with_source(
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
    visual_loop::show_visual_scene_overlay_with_source(
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

#[cfg(test)]
#[path = "visual_tests.rs"]
mod tests;
