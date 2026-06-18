use termwiz::cell::AttributeChange;
use termwiz::surface::{Change, Position};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct SceneTextCell {
    pub(super) col: usize,
    pub(super) row: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct SceneTextSelection {
    anchor: Option<SceneTextCell>,
    focus: Option<SceneTextCell>,
    dragging: bool,
    lines: Vec<String>,
}

impl SceneTextSelection {
    pub(super) fn set_frame_text(&mut self, frame: &str, cols: usize, rows: usize) {
        self.lines = frame
            .replace("\r\n", "\n")
            .lines()
            .take(rows)
            .map(|line| {
                let mut clipped = String::new();
                for ch in line.chars().take(cols) {
                    clipped.push(ch);
                }
                clipped
            })
            .collect();
    }

    pub(super) fn begin(&mut self, cell: SceneTextCell) {
        self.anchor = Some(cell);
        self.focus = Some(cell);
        self.dragging = true;
    }

    pub(super) fn update(&mut self, cell: SceneTextCell) {
        if self.dragging {
            self.focus = Some(cell);
        }
    }

    pub(super) fn finish(&mut self, cell: SceneTextCell) {
        if self.dragging {
            self.focus = Some(cell);
            self.dragging = false;
        }
    }

    pub(super) fn clear(&mut self) {
        self.anchor = None;
        self.focus = None;
        self.dragging = false;
    }

    pub(super) fn is_active(&self) -> bool {
        self.normalized_range().is_some()
    }

    pub(super) fn is_dragging(&self) -> bool {
        self.dragging
    }

    pub(super) fn selected_text(&self) -> Option<String> {
        let (start, end) = self.normalized_range()?;
        let mut out = String::new();
        for row in start.row..=end.row {
            let Some(line) = self.lines.get(row) else {
                continue;
            };
            let line_len = line.chars().count();
            let start_col = if row == start.row { start.col } else { 0 }.min(line_len);
            let end_col = if row == end.row {
                end.col.saturating_add(1)
            } else {
                line_len
            }
            .min(line_len);
            if row > start.row {
                out.push('\n');
            }
            if start_col < end_col {
                out.push_str(&char_slice(line, start_col, end_col));
            }
        }
        if out.is_empty() {
            None
        } else {
            Some(out)
        }
    }

    pub(super) fn render_changes(&self, cols: usize, rows: usize) -> Vec<Change> {
        let Some((start, end)) = self.normalized_range() else {
            return Vec::new();
        };
        let mut changes = Vec::new();
        for row in start.row..=end.row {
            if row >= rows {
                break;
            }
            let Some(line) = self.lines.get(row) else {
                continue;
            };
            let line_len = line.chars().count().min(cols);
            let start_col = if row == start.row { start.col } else { 0 }.min(line_len);
            let end_col = if row == end.row {
                end.col.saturating_add(1)
            } else {
                line_len
            }
            .min(line_len);
            if start_col >= end_col {
                continue;
            }
            let text = char_slice(line, start_col, end_col);
            if text.is_empty() {
                continue;
            }
            changes.push(Change::CursorPosition {
                x: Position::Absolute(start_col),
                y: Position::Absolute(row),
            });
            changes.push(AttributeChange::Reverse(true).into());
            changes.push(Change::Text(text));
            changes.push(AttributeChange::Reverse(false).into());
        }
        changes
    }

    fn normalized_range(&self) -> Option<(SceneTextCell, SceneTextCell)> {
        let anchor = self.anchor?;
        let focus = self.focus?;
        if anchor == focus {
            return None;
        }
        if (anchor.row, anchor.col) <= (focus.row, focus.col) {
            Some((anchor, focus))
        } else {
            Some((focus, anchor))
        }
    }
}

fn char_slice(line: &str, start: usize, end: usize) -> String {
    line.chars()
        .skip(start)
        .take(end.saturating_sub(start))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{SceneTextCell, SceneTextSelection};

    #[test]
    fn scene_text_selection_extracts_multiline_text() {
        let mut selection = SceneTextSelection::default();
        selection.set_frame_text("alpha beta\r\ngamma delta", 80, 24);
        selection.begin(SceneTextCell { col: 6, row: 0 });
        selection.finish(SceneTextCell { col: 4, row: 1 });

        assert_eq!(selection.selected_text().as_deref(), Some("beta\ngamma"));
    }
}
