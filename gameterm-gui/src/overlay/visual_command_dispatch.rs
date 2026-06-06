use super::visual_scene_files::{load_scene_required, resolve_scene_target};
use crate::termwindow::TermWindowNotif;
use anyhow::Context;
use config::keyassignment::SpawnTabDomain;
use gameterm_term::TerminalSize;
use gameterm_visual::{RunCommandTarget, SceneRuntime, VisualActionRequest, VisualStoryState};
use mux::domain::SplitSource;
use mux::tab::{SplitDirection, SplitRequest, SplitSize};
use mux::Mux;
use portable_pty::CommandBuilder;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use window::{Window, WindowOps};

pub(super) enum RunCommandResult {
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
pub(super) struct RunCommandDispatch {
    pub(super) window_id: mux::window::WindowId,
    pub(super) pane_id: Option<mux::pane::PaneId>,
    pub(super) terminal_size: TerminalSize,
    pub(super) gui_window: Option<Window>,
    pub(super) command_tx: mpsc::Sender<RunCommandResult>,
}

pub(super) fn dispatch_pending_action(
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

pub(super) fn write_story_state_file(path: &Path, json: &str) -> anyhow::Result<()> {
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
