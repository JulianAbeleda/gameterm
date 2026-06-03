use crate::{VisualRenderEntity, VisualRenderSnapshot, VisualRenderTile};
use std::ops::Range;

pub fn visible_tiles_for_row(
    snapshot: &VisualRenderSnapshot,
    row: usize,
    columns: Range<usize>,
) -> Vec<&VisualRenderTile> {
    if row >= snapshot.height {
        return Vec::new();
    }

    let columns = clipped_columns(columns, snapshot.width);
    snapshot
        .tiles
        .iter()
        .filter(|tile| tile.position.y == row && columns.contains(&tile.position.x))
        .collect()
}

pub fn intersecting_entities_for_row(
    snapshot: &VisualRenderSnapshot,
    row: usize,
    columns: Range<usize>,
) -> Vec<&VisualRenderEntity> {
    if row >= snapshot.height {
        return Vec::new();
    }

    let columns = clipped_columns(columns, snapshot.width);
    snapshot
        .entities
        .iter()
        .filter(|entity| entity.position.y == row && columns.contains(&entity.position.x))
        .collect()
}

fn clipped_columns(columns: Range<usize>, width: usize) -> Range<usize> {
    columns.start.min(width)..columns.end.min(width)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        VisualModeDescriptor, VisualPosition, VisualRenderEntity, VisualRenderLayer,
        VisualSceneLoadStatus, VisualSceneSource, VisualView,
    };

    fn snapshot_for_filtering() -> VisualRenderSnapshot {
        VisualRenderSnapshot {
            generation: 7,
            view: VisualView::Scene,
            scene_source: VisualSceneSource::new("test", VisualSceneLoadStatus::Loaded, 0),
            active_mode: VisualModeDescriptor::default(),
            active_layers: Vec::new(),
            selected_entity_mode: None,
            variables: Vec::new(),
            rpg: crate::VisualRpgState::default(),
            title: "Test".to_string(),
            background: "bg".to_string(),
            width: 4,
            height: 3,
            selected_entity_id: None,
            selected_choice: 0,
            tiles: vec![
                VisualRenderTile {
                    position: VisualPosition { x: 1, y: 1 },
                    sprite: "visible_tile".to_string(),
                    layer: VisualRenderLayer::Tile,
                },
                VisualRenderTile {
                    position: VisualPosition { x: 3, y: 1 },
                    sprite: "outside_columns".to_string(),
                    layer: VisualRenderLayer::Tile,
                },
                VisualRenderTile {
                    position: VisualPosition { x: 1, y: 2 },
                    sprite: "outside_row".to_string(),
                    layer: VisualRenderLayer::Tile,
                },
            ],
            stage: Vec::new(),
            entities: vec![
                VisualRenderEntity {
                    id: "visible_entity".to_string(),
                    kind: crate::VisualEntityKind::Task,
                    label: "Visible".to_string(),
                    position: VisualPosition { x: 2, y: 1 },
                    sprite: "visible_entity".to_string(),
                    layer: VisualRenderLayer::Entity,
                    selected: false,
                    state_flags: Vec::new(),
                },
                VisualRenderEntity {
                    id: "outside_columns".to_string(),
                    kind: crate::VisualEntityKind::Task,
                    label: "Outside Columns".to_string(),
                    position: VisualPosition { x: 3, y: 1 },
                    sprite: "outside_columns".to_string(),
                    layer: VisualRenderLayer::Entity,
                    selected: false,
                    state_flags: Vec::new(),
                },
                VisualRenderEntity {
                    id: "outside_row".to_string(),
                    kind: crate::VisualEntityKind::Task,
                    label: "Outside Row".to_string(),
                    position: VisualPosition { x: 2, y: 2 },
                    sprite: "outside_row".to_string(),
                    layer: VisualRenderLayer::Entity,
                    selected: false,
                    state_flags: Vec::new(),
                },
            ],
            dialogue_speaker: String::new(),
            dialogue: String::new(),
            dialogue_index: None,
            dialogue_history: Vec::new(),
            status: String::new(),
            choices: Vec::new(),
            overlay_cols: None,
            overlay_rows: None,
            vn_dialogue_scroll: None,
            vn_layout_debug: None,
        }
    }

    #[test]
    fn clipped_columns_clamps_to_width() {
        assert_eq!(clipped_columns(1..99, 4), 1..4);
        assert_eq!(clipped_columns(8..99, 4), 4..4);
    }

    #[test]
    fn visible_tiles_for_row_excludes_offscreen_records() {
        let snapshot = snapshot_for_filtering();
        let tiles = visible_tiles_for_row(&snapshot, 1, 0..3);

        assert_eq!(tiles.len(), 1);
        assert_eq!(tiles[0].sprite, "visible_tile");
    }

    #[test]
    fn intersecting_entities_for_row_excludes_offscreen_records() {
        let snapshot = snapshot_for_filtering();
        let entities = intersecting_entities_for_row(&snapshot, 1, 0..3);

        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].id, "visible_entity");
    }
}
