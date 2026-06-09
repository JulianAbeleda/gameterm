use super::dialogue::dialogue_index;
use super::SceneRuntime;
use crate::{
    VisualEntity, VisualPosition, VisualRenderEntity, VisualRenderLayer, VisualRenderSnapshot,
    VisualRenderStageDisplayable, VisualRenderTile,
};

impl SceneRuntime {
    pub fn render_snapshot(&self) -> VisualRenderSnapshot {
        let selected_entity_id = self.selected_entity().map(|entity| entity.id.clone());
        let selected_entity_mode = self.selected_entity().and_then(entity_mode);
        let active_dialogue = self.active_dialogue_line();
        VisualRenderSnapshot {
            generation: self.generation,
            view: self.view,
            scene_source: self.scene_source.clone(),
            active_mode: self.scene.mode.clone(),
            active_layers: self.scene.layers.clone(),
            selected_entity_mode,
            variables: self.scene.variables.clone(),
            rpg: self.scene.rpg.clone(),
            title: self.scene.title.clone(),
            background: self.scene.background.clone(),
            width: self.scene.width,
            height: self.scene.height,
            selected_entity_id,
            selected_choice: self.selected_choice,
            tiles: self.render_tiles(),
            stage: self.render_stage(),
            entities: self.render_entities(),
            dialogue_speaker: active_dialogue.speaker,
            dialogue: active_dialogue.text,
            dialogue_index: dialogue_index(&self.scene, self.dialogue_index),
            dialogue_history: self.dialogue_history.clone(),
            status: self.status.clone(),
            overlay_cols: None,
            overlay_rows: None,
            vn_dialogue_scroll: None,
            vn_layout_debug: self.vn_layout_debug.clone(),
            interactive_debug_menu: self.interactive_debug_menu,
            vn_voice_hold_active: false,
            choices: self
                .scene
                .choices
                .iter()
                .map(|choice| choice.label.clone())
                .collect(),
        }
    }

    fn render_tiles(&self) -> Vec<VisualRenderTile> {
        if !self.scene.stage.is_empty() {
            return Vec::new();
        }
        let mut tiles = Vec::with_capacity(self.scene.width * self.scene.height);
        for y in 0..self.scene.height {
            for x in 0..self.scene.width {
                tiles.push(VisualRenderTile {
                    position: VisualPosition { x, y },
                    sprite: self.scene.background.clone(),
                    layer: VisualRenderLayer::Tile,
                });
            }
        }
        tiles
    }

    fn render_stage(&self) -> Vec<VisualRenderStageDisplayable> {
        let mut displayables = Vec::new();
        for layer in &self.scene.stage.layers {
            for displayable in &layer.displayables {
                if displayable.visible {
                    displayables.push(VisualRenderStageDisplayable {
                        layer_id: layer.layer_id.clone(),
                        tag: displayable.tag.clone(),
                        sprite: displayable.sprite.clone(),
                        placement: displayable.placement,
                        layer_zorder: layer.zorder,
                        zorder: displayable.zorder,
                    });
                }
            }
        }
        displayables.sort_by(|left, right| {
            left.layer_zorder
                .cmp(&right.layer_zorder)
                .then(left.zorder.cmp(&right.zorder))
                .then(left.layer_id.cmp(&right.layer_id))
                .then(left.tag.cmp(&right.tag))
        });
        displayables
    }

    fn render_entities(&self) -> Vec<VisualRenderEntity> {
        self.scene
            .entities
            .iter()
            .enumerate()
            .filter(|(_, entity)| entity.visible)
            .map(|(idx, entity)| VisualRenderEntity {
                id: entity.id.clone(),
                kind: entity.kind.clone(),
                label: entity.label.clone(),
                position: entity.position,
                sprite: entity.sprite.clone(),
                layer: VisualRenderLayer::Entity,
                selected: idx == self.selected_entity,
                state_flags: entity.state_flags.clone(),
            })
            .collect()
    }
}

pub(super) fn entity_mode(entity: &VisualEntity) -> Option<String> {
    entity
        .metadata
        .iter()
        .find(|(key, value)| key == "mode" && !value.trim().is_empty())
        .map(|(_, value)| value.clone())
}
