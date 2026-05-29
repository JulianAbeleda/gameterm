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

    #[test]
    fn clipped_columns_clamps_to_width() {
        assert_eq!(clipped_columns(1..99, 4), 1..4);
        assert_eq!(clipped_columns(8..99, 4), 4..4);
    }
}
