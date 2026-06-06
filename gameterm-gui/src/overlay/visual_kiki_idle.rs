use gameterm_visual::{SceneRuntime, VisualRenderSnapshot, VisualSpriteManifestStatus};
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) const KIKI_STAGE_TAG: &str = "kiki";
pub(super) const KIKI_BASE_SPRITE: &str = "vn.character.kiki.neutral";
const KIKI_BREATH_FRAME_PREFIX: &str = "vn.character.kiki.breath.";
pub(super) const KIKI_BREATH_FRAME_COUNT: usize = 6;
pub(super) const KIKI_BREATH_FRAME_MS: u128 = 180;
const KIKI_BLINK_FRAME_PREFIX: &str = "vn.character.kiki.blink.";
pub(super) const KIKI_BLINK_FRAME_COUNT: usize = 6;
pub(super) const KIKI_BLINK_FRAME_MS: u128 = 90;
pub(super) const KIKI_BLINK_INTERVAL_MS: u128 = 4_200;

pub(super) fn current_kiki_idle_sprite(
    sprite_manifest: &VisualSpriteManifestStatus,
) -> Option<String> {
    let elapsed_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis())
        .unwrap_or(0);
    kiki_idle_sprite_for_elapsed_ms(elapsed_ms, sprite_manifest)
}

pub(super) fn kiki_idle_sprite_for_elapsed_ms(
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

pub(super) fn kiki_breath_frame_for_elapsed_ms(elapsed_ms: u128) -> usize {
    ((elapsed_ms / KIKI_BREATH_FRAME_MS) % KIKI_BREATH_FRAME_COUNT as u128) as usize
}

pub(super) fn kiki_blink_frame_for_elapsed_ms(elapsed_ms: u128) -> Option<usize> {
    let blink_elapsed = elapsed_ms % KIKI_BLINK_INTERVAL_MS;
    let frame = (blink_elapsed / KIKI_BLINK_FRAME_MS) as usize;
    (frame < KIKI_BLINK_FRAME_COUNT).then_some(frame)
}

pub(super) fn runtime_has_kiki_idle_animation(
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

pub(super) fn apply_kiki_idle_animation(
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

pub(super) fn kiki_breath_sprite_id(frame: usize) -> String {
    format!(
        "{}{}",
        KIKI_BREATH_FRAME_PREFIX,
        frame.min(KIKI_BREATH_FRAME_COUNT.saturating_sub(1))
    )
}

pub(super) fn kiki_blink_sprite_id(frame: usize) -> String {
    format!(
        "{}{}",
        KIKI_BLINK_FRAME_PREFIX,
        frame.min(KIKI_BLINK_FRAME_COUNT.saturating_sub(1))
    )
}
