use super::SceneRuntime;
use crate::{VisualDialogueLine, VisualScene};

impl SceneRuntime {
    pub fn active_dialogue_line(&self) -> VisualDialogueLine {
        active_dialogue_line(&self.scene, self.dialogue_index)
    }
}

pub(super) fn dialogue_index(scene: &VisualScene, index: usize) -> Option<usize> {
    if scene.dialogue_lines.is_empty() {
        None
    } else {
        Some(index.min(scene.dialogue_lines.len() - 1))
    }
}

fn active_dialogue_line(scene: &VisualScene, index: usize) -> VisualDialogueLine {
    dialogue_index(scene, index)
        .and_then(|index| scene.dialogue_lines.get(index).cloned())
        .unwrap_or_else(|| VisualDialogueLine {
            speaker: scene.dialogue_speaker.clone(),
            text: scene.dialogue.clone(),
            portrait: None,
            metadata: Vec::new(),
        })
}

pub(super) fn initial_dialogue_history(
    scene: &VisualScene,
    index: usize,
) -> Vec<VisualDialogueLine> {
    if scene.dialogue_lines.is_empty() {
        Vec::new()
    } else {
        vec![active_dialogue_line(scene, index)]
    }
}
