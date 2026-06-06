use super::{render_error, render_runtime};
use super::super::visual_stt::SceneSttConfig;
use super::super::visual_tts::SceneTtsConfig;
use anyhow::Context;
use gameterm_visual::{
    SceneRuntime, VisualResolvedSprite, VisualScene, VisualSceneLoadStatus, VisualSceneSource,
    VisualSpriteManifest, VisualSpriteManifestStatus, VnOverlayDebugOverrides,
};
use mux::termwiztermtab::TermWizTerminal;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub(super) const VN_OVERLAY_LAYOUT_CONFIG_FILE: &str = "vn-overlay-layout.json";
pub(super) const BUNDLED_SCENE_JSON: &str =
    include_str!("../../../docs/examples/gameterm-scene-default.json");

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SceneOverlayLaunchOptions {
    pub(super) tts_config: Option<SceneTtsConfig>,
    pub(super) stt_config: Option<SceneSttConfig>,
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

pub(super) enum VisualSceneOverlaySource {
    Default,
    Generated {
        scene: VisualScene,
        action_base_dir: PathBuf,
        source_label: String,
    },
}

pub(super) fn reload_active_scene(
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

pub(super) fn reload_generated_scene(
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
pub(super) struct SceneFileWatcher {
    enabled: bool,
    scene_stamp: Option<SystemTime>,
    sprite_stamp: Option<SystemTime>,
    scene_dir_stamp: Option<SystemTime>,
}

impl SceneFileWatcher {
    pub(super) fn disabled() -> Self {
        Self {
            enabled: false,
            scene_stamp: None,
            sprite_stamp: None,
            scene_dir_stamp: None,
        }
    }

    pub(super) fn from_env(scene_path: &Path, sprite_path: &Path) -> Self {
        if std::env::var("GAMETERM_SCENE_AUTO_RELOAD").ok().as_deref() != Some("1") {
            return Self::disabled();
        }
        Self::enabled(scene_path, sprite_path)
    }

    pub(super) fn enabled(scene_path: &Path, sprite_path: &Path) -> Self {
        let mut watcher = Self {
            enabled: true,
            scene_stamp: None,
            sprite_stamp: None,
            scene_dir_stamp: None,
        };
        watcher.refresh(scene_path, sprite_path);
        watcher
    }

    pub(super) fn refresh(&mut self, scene_path: &Path, sprite_path: &Path) {
        self.scene_stamp = modified_time(scene_path);
        self.sprite_stamp = modified_time(sprite_path);
        self.scene_dir_stamp = scene_path.parent().and_then(modified_time);
    }

    pub(super) fn changed(&self, scene_path: &Path, sprite_path: &Path) -> bool {
        self.enabled
            && (self.scene_stamp != modified_time(scene_path)
                || self.sprite_stamp != modified_time(sprite_path)
                || self.scene_dir_stamp != scene_path.parent().and_then(modified_time))
    }
}

pub(super) fn modified_time(path: &Path) -> Option<SystemTime> {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
}

pub(super) fn initial_scene_state(
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

pub(super) fn initial_generated_scene_state(
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

pub(super) fn load_scene(
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

pub(super) fn load_scene_required(
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

pub(super) fn resolve_scene_target(current_scene_path: &Path, target: &str) -> PathBuf {
    let raw_target = PathBuf::from(target);
    if raw_target.is_absolute() {
        return raw_target;
    }

    current_scene_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(raw_target)
}

pub(super) fn default_scene_path() -> PathBuf {
    default_scene_dir().join("default.json")
}

pub(super) fn default_sprite_manifest_path() -> PathBuf {
    default_scene_dir().join("sprites.json")
}

pub(super) fn default_vn_overlay_layout_config_path() -> PathBuf {
    default_scene_dir().join(VN_OVERLAY_LAYOUT_CONFIG_FILE)
}

fn default_scene_dir() -> PathBuf {
    let config_home = config::CONFIG_DIRS
        .first()
        .cloned()
        .unwrap_or_else(|| config::HOME_DIR.join(".config").join("gameterm"));
    config_home.join("scenes")
}

pub(super) fn apply_configured_vn_overlay_layout(runtime: &mut SceneRuntime) {
    if let Some(overrides) = load_vn_overlay_layout_config() {
        runtime.set_vn_layout_debug_overrides(overrides);
    }
}

fn load_vn_overlay_layout_config() -> Option<VnOverlayDebugOverrides> {
    load_vn_overlay_layout_config_from_path(&default_vn_overlay_layout_config_path())
}

pub(super) fn load_vn_overlay_layout_config_from_path(
    path: &Path,
) -> Option<VnOverlayDebugOverrides> {
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

pub(super) fn persistable_vn_overlay_layout(
    overrides: &VnOverlayDebugOverrides,
) -> VnOverlayDebugOverrides {
    let mut overrides = overrides.clone();
    overrides.editing_buffer = None;
    overrides
}

pub(super) fn save_vn_overlay_layout_config(
    overrides: &VnOverlayDebugOverrides,
) -> anyhow::Result<()> {
    save_vn_overlay_layout_config_to_path(&default_vn_overlay_layout_config_path(), overrides)
}

pub(super) fn save_vn_overlay_layout_config_to_path(
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

pub(super) fn persist_vn_overlay_layout_if_changed(
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

pub(super) fn load_sprite_manifest_status(path: &PathBuf) -> VisualSpriteManifestStatus {
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

pub(super) fn bundled_sprite_asset_path(sprite_id: &str) -> PathBuf {
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

pub(super) fn bundled_scene_sprite_ids() -> anyhow::Result<Vec<String>> {
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
