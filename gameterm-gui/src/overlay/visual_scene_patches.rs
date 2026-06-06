use super::render_runtime;
use super::visual_scene_files::modified_time;
use gameterm_visual::{SceneRuntime, VisualScenePatch, VisualSpriteManifestStatus};
use mux::termwiztermtab::TermWizTerminal;
use mux::{Mux, MuxNotification};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::SystemTime;

pub(super) struct ScenePatchNotificationSubscription {
    dead: Arc<AtomicBool>,
}

pub(super) struct ScenePatchNotification {
    pub(super) patch_json: String,
    pub(super) source_pane_id: Option<mux::pane::PaneId>,
}

impl ScenePatchNotificationSubscription {
    pub(super) fn new(
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

pub(super) fn scene_patch_target_matches(
    target_pane_id: Option<mux::pane::PaneId>,
    active_pane_id: Option<mux::pane::PaneId>,
    overlay_pane_id: mux::pane::PaneId,
    route_pane_id: Option<mux::pane::PaneId>,
) -> bool {
    let target_pane_id = target_pane_id.or(active_pane_id);
    target_pane_id == Some(overlay_pane_id) || target_pane_id == route_pane_id
}

pub(super) struct ActiveSceneOverlay {
    pane_id: mux::pane::PaneId,
}

impl ActiveSceneOverlay {
    pub(super) fn new(pane_id: mux::pane::PaneId) -> Self {
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

#[derive(Debug, Clone)]
pub(super) struct ScenePatchInbox {
    path: Option<PathBuf>,
    stamp: Option<SystemTime>,
}

impl ScenePatchInbox {
    pub(super) fn disabled() -> Self {
        Self {
            path: None,
            stamp: None,
        }
    }

    pub(super) fn from_env() -> Self {
        let Some(path) = std::env::var_os("GAMETERM_SCENE_PATCH_FILE").map(PathBuf::from) else {
            return Self::disabled();
        };
        Self::watching(path)
    }

    pub(super) fn watching(path: PathBuf) -> Self {
        let stamp = modified_time(&path);
        Self {
            path: Some(path),
            stamp,
        }
    }

    pub(super) fn refresh(&mut self) {
        if let Some(path) = &self.path {
            self.stamp = modified_time(path);
        }
    }

    pub(super) fn changed_path(&self) -> Option<PathBuf> {
        let path = self.path.as_ref()?;
        let stamp = modified_time(path);
        if stamp.is_some() && stamp != self.stamp {
            Some(path.clone())
        } else {
            None
        }
    }
}

pub(super) fn apply_scene_patch_file(
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

pub(super) fn apply_scene_patch_json(
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
