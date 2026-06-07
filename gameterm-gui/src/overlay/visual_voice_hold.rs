use mux::pane::PaneId;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

static SCENE_VOICE_HOLD: LazyLock<Mutex<HashMap<PaneId, bool>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub(crate) fn set_scene_voice_hold_active(pane_id: PaneId, active: bool) {
    SCENE_VOICE_HOLD
        .lock()
        .expect("Scene voice hold state mutex poisoned")
        .insert(pane_id, active);
}

pub(crate) fn scene_voice_hold_active(pane_id: PaneId) -> bool {
    SCENE_VOICE_HOLD
        .lock()
        .expect("Scene voice hold state mutex poisoned")
        .get(&pane_id)
        .copied()
        .unwrap_or(false)
}

pub(crate) fn clear_scene_voice_hold(pane_id: PaneId) {
    SCENE_VOICE_HOLD
        .lock()
        .expect("Scene voice hold state mutex poisoned")
        .remove(&pane_id);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_voice_hold_tracks_pane_state() {
        let pane_id = 9001;

        clear_scene_voice_hold(pane_id);
        assert!(!scene_voice_hold_active(pane_id));

        set_scene_voice_hold_active(pane_id, true);
        assert!(scene_voice_hold_active(pane_id));

        set_scene_voice_hold_active(pane_id, false);
        assert!(!scene_voice_hold_active(pane_id));

        set_scene_voice_hold_active(pane_id, true);
        clear_scene_voice_hold(pane_id);
        assert!(!scene_voice_hold_active(pane_id));
    }
}
