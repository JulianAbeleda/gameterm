use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::ops::Range;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualPosition {
    pub x: usize,
    pub y: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VisualEntityKind {
    Agent,
    Memory,
    Principle,
    Project,
    Task,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualEntity {
    pub id: String,
    pub kind: VisualEntityKind,
    pub label: String,
    pub position: VisualPosition,
    pub sprite: String,
    #[serde(default)]
    pub state_flags: Vec<String>,
    #[serde(default)]
    pub metadata: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SceneActionKind {
    Inspect,
    OpenFile { path: String },
    RunCommand { command: String },
    Navigate { target: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneAction {
    pub label: String,
    pub kind: SceneActionKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualScene {
    pub title: String,
    pub background: String,
    pub width: usize,
    pub height: usize,
    pub entities: Vec<VisualEntity>,
    pub dialogue_speaker: String,
    pub dialogue: String,
    pub choices: Vec<SceneAction>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VisualRenderLayer {
    Background,
    Tile,
    Entity,
    Selection,
    Dialogue,
    Debug,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualRenderTile {
    pub position: VisualPosition,
    pub sprite: String,
    pub layer: VisualRenderLayer,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualRenderEntity {
    pub id: String,
    pub kind: VisualEntityKind,
    pub label: String,
    pub position: VisualPosition,
    pub sprite: String,
    pub layer: VisualRenderLayer,
    pub selected: bool,
    pub state_flags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualRenderSnapshot {
    pub generation: u64,
    pub view: VisualView,
    pub title: String,
    pub background: String,
    pub width: usize,
    pub height: usize,
    pub selected_entity_id: Option<String>,
    pub selected_choice: usize,
    pub tiles: Vec<VisualRenderTile>,
    pub entities: Vec<VisualRenderEntity>,
    pub dialogue_speaker: String,
    pub dialogue: String,
    pub status: String,
    pub choices: Vec<String>,
}

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

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VisualSceneError {
    #[error("scene dimensions must be non-zero")]
    EmptyScene,
    #[error("duplicate entity id `{0}`")]
    DuplicateEntityId(String),
    #[error("entity `{id}` is outside scene bounds at {x},{y}")]
    EntityOutOfBounds { id: String, x: usize, y: usize },
    #[error("scene json error: {0}")]
    Json(String),
    #[error("scene file error for `{path}`: {message}")]
    File { path: String, message: String },
}

impl VisualScene {
    pub fn from_json(json: &str) -> Result<Self, VisualSceneError> {
        let scene: Self =
            serde_json::from_str(json).map_err(|err| VisualSceneError::Json(err.to_string()))?;
        scene.validate()?;
        Ok(scene)
    }

    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self, VisualSceneError> {
        let path = path.as_ref();
        let json = std::fs::read_to_string(path).map_err(|err| VisualSceneError::File {
            path: path.display().to_string(),
            message: err.to_string(),
        })?;
        Self::from_json(&json)
    }

    pub fn validate(&self) -> Result<(), VisualSceneError> {
        if self.width == 0 || self.height == 0 {
            return Err(VisualSceneError::EmptyScene);
        }

        let mut ids = HashSet::new();
        for entity in &self.entities {
            if !ids.insert(entity.id.as_str()) {
                return Err(VisualSceneError::DuplicateEntityId(entity.id.clone()));
            }
            if entity.position.x >= self.width || entity.position.y >= self.height {
                return Err(VisualSceneError::EntityOutOfBounds {
                    id: entity.id.clone(),
                    x: entity.position.x,
                    y: entity.position.y,
                });
            }
        }

        Ok(())
    }

    pub fn demo() -> Self {
        Self {
            title: "GameTerm Scene Mode".to_string(),
            background: "workspace-map".to_string(),
            width: 18,
            height: 9,
            entities: vec![
                VisualEntity {
                    id: "project-gameterm".to_string(),
                    kind: VisualEntityKind::Project,
                    label: "GameTerm".to_string(),
                    position: VisualPosition { x: 3, y: 2 },
                    sprite: "project_core".to_string(),
                    state_flags: vec!["active".to_string()],
                    metadata: vec![
                        ("repo".to_string(), "JulianAbeleda/gameterm".to_string()),
                        ("mode".to_string(), "hard-fork".to_string()),
                    ],
                },
                VisualEntity {
                    id: "task-render".to_string(),
                    kind: VisualEntityKind::Task,
                    label: "Render Scene".to_string(),
                    position: VisualPosition { x: 9, y: 4 },
                    sprite: "task_tile".to_string(),
                    state_flags: vec!["running".to_string()],
                    metadata: vec![
                        ("reference".to_string(), "Ren'Py scene flow".to_string()),
                        ("reference".to_string(), "mGBA PPU/debug split".to_string()),
                    ],
                },
                VisualEntity {
                    id: "agent-audit".to_string(),
                    kind: VisualEntityKind::Agent,
                    label: "Audit Agent".to_string(),
                    position: VisualPosition { x: 14, y: 2 },
                    sprite: "agent_idle".to_string(),
                    state_flags: vec!["watching".to_string()],
                    metadata: vec![("role".to_string(), "review scene state".to_string())],
                },
            ],
            dialogue_speaker: "GameTerm".to_string(),
            dialogue: "Scene Mode renders project state as symbolic entities while preserving terminal control.".to_string(),
            choices: vec![
                SceneAction {
                    label: "Inspect selected entity".to_string(),
                    kind: SceneActionKind::Inspect,
                },
                SceneAction {
                    label: "Open MIGRATION.md".to_string(),
                    kind: SceneActionKind::OpenFile {
                        path: "MIGRATION.md".to_string(),
                    },
                },
                SceneAction {
                    label: "Run cargo check -p gameterm-visual".to_string(),
                    kind: SceneActionKind::RunCommand {
                        command: "cargo check -p gameterm-visual".to_string(),
                    },
                },
            ],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VisualView {
    Scene,
    TileDebugger,
}

#[derive(Debug, Clone)]
pub struct SceneRuntime {
    scene: VisualScene,
    selected_entity: usize,
    selected_choice: usize,
    view: VisualView,
    status: String,
    generation: u64,
}

impl SceneRuntime {
    pub fn new(scene: VisualScene) -> Result<Self, VisualSceneError> {
        scene.validate()?;
        Ok(Self {
            scene,
            selected_entity: 0,
            selected_choice: 0,
            view: VisualView::Scene,
            status: "Ready".to_string(),
            generation: 0,
        })
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn view(&self) -> VisualView {
        self.view
    }

    pub fn toggle_debugger(&mut self) {
        self.view = match self.view {
            VisualView::Scene => VisualView::TileDebugger,
            VisualView::TileDebugger => VisualView::Scene,
        };
        self.bump_generation();
    }

    pub fn select_next_entity(&mut self) {
        if self.scene.entities.len() > 1 {
            self.selected_entity = (self.selected_entity + 1) % self.scene.entities.len();
            self.bump_generation();
        }
    }

    pub fn select_prev_entity(&mut self) {
        if self.scene.entities.len() > 1 {
            self.selected_entity = if self.selected_entity == 0 {
                self.scene.entities.len() - 1
            } else {
                self.selected_entity - 1
            };
            self.bump_generation();
        }
    }

    pub fn select_next_choice(&mut self) {
        if self.scene.choices.len() > 1 {
            self.selected_choice = (self.selected_choice + 1) % self.scene.choices.len();
            self.bump_generation();
        }
    }

    pub fn select_prev_choice(&mut self) {
        if self.scene.choices.len() > 1 {
            self.selected_choice = if self.selected_choice == 0 {
                self.scene.choices.len() - 1
            } else {
                self.selected_choice - 1
            };
            self.bump_generation();
        }
    }

    pub fn activate_choice(&mut self) {
        if let Some(choice) = self.scene.choices.get(self.selected_choice) {
            self.status = match &choice.kind {
                SceneActionKind::Inspect => self
                    .selected_entity()
                    .map(|entity| format!("Inspecting {} ({})", entity.label, entity.id))
                    .unwrap_or_else(|| "No entity selected".to_string()),
                SceneActionKind::OpenFile { path } => {
                    format!("Action placeholder: open file `{path}`")
                }
                SceneActionKind::RunCommand { command } => {
                    format!("Action placeholder: run `{command}`")
                }
                SceneActionKind::Navigate { target } => {
                    format!("Action placeholder: navigate to `{target}`")
                }
            };
            self.bump_generation();
        }
    }

    pub fn selected_entity(&self) -> Option<&VisualEntity> {
        self.scene.entities.get(self.selected_entity)
    }

    pub fn render_text_frame(&self, cols: usize, rows: usize) -> String {
        match self.view {
            VisualView::Scene => self.render_scene(cols, rows),
            VisualView::TileDebugger => self.render_debugger(cols, rows),
        }
    }

    pub fn render_snapshot(&self) -> VisualRenderSnapshot {
        let selected_entity_id = self.selected_entity().map(|entity| entity.id.clone());
        VisualRenderSnapshot {
            generation: self.generation,
            view: self.view,
            title: self.scene.title.clone(),
            background: self.scene.background.clone(),
            width: self.scene.width,
            height: self.scene.height,
            selected_entity_id,
            selected_choice: self.selected_choice,
            tiles: self.render_tiles(),
            entities: self.render_entities(),
            dialogue_speaker: self.scene.dialogue_speaker.clone(),
            dialogue: self.scene.dialogue.clone(),
            status: self.status.clone(),
            choices: self
                .scene
                .choices
                .iter()
                .map(|choice| choice.label.clone())
                .collect(),
        }
    }

    fn render_tiles(&self) -> Vec<VisualRenderTile> {
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

    fn render_entities(&self) -> Vec<VisualRenderEntity> {
        self.scene
            .entities
            .iter()
            .enumerate()
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

    fn bump_generation(&mut self) {
        self.generation = self.generation.saturating_add(1);
    }

    fn render_scene(&self, cols: usize, rows: usize) -> String {
        let mut out = String::new();
        out.push_str(&format!("{}\r\n", self.scene.title));
        out.push_str("Scene Mode  [arrows/hjkl: select] [enter: action] [tab: debugger] [esc/q: close]\r\n\r\n");

        let mut grid = vec![vec!['.'; self.scene.width]; self.scene.height];
        for (idx, entity) in self.scene.entities.iter().enumerate() {
            let glyph = match entity.kind {
                VisualEntityKind::Agent => 'A',
                VisualEntityKind::Memory => 'M',
                VisualEntityKind::Principle => 'P',
                VisualEntityKind::Project => 'R',
                VisualEntityKind::Task => 'T',
            };
            grid[entity.position.y][entity.position.x] = if idx == self.selected_entity {
                '@'
            } else {
                glyph
            };
        }

        let available_grid_rows = rows.saturating_sub(13).min(self.scene.height);
        for row in grid.into_iter().take(available_grid_rows) {
            out.push_str("  ");
            for ch in row {
                out.push(ch);
                out.push(' ');
            }
            out.push_str("\r\n");
        }

        out.push_str("\r\n");
        if let Some(entity) = self.selected_entity() {
            out.push_str(&format!(
                "Selected: {} [{:?}] sprite={} flags={}\r\n",
                entity.label,
                entity.kind,
                entity.sprite,
                entity.state_flags.join(", ")
            ));
        }
        out.push_str(&format!(
            "{}: {}\r\n\r\n",
            self.scene.dialogue_speaker, self.scene.dialogue
        ));

        out.push_str("Choices:\r\n");
        for (idx, choice) in self.scene.choices.iter().enumerate() {
            let marker = if idx == self.selected_choice {
                ">"
            } else {
                " "
            };
            out.push_str(&format!("{marker} {}\r\n", choice.label));
        }
        out.push_str(&format!("\r\nStatus: {}\r\n", self.status));
        truncate_to_screen(out, cols, rows)
    }

    fn render_debugger(&self, cols: usize, rows: usize) -> String {
        let mut out = String::new();
        out.push_str("GameTerm Tile Debugger\r\n");
        out.push_str("[tab: scene] [arrows/hjkl: select entity] [esc/q: close]\r\n\r\n");
        out.push_str(&format!(
            "scene={} background={} size={}x{}\r\n\r\n",
            self.scene.title, self.scene.background, self.scene.width, self.scene.height
        ));
        out.push_str("Layer order:\r\n");
        out.push_str("  0 background\r\n  1 tile grid\r\n  2 entity sprites\r\n  3 selection/relations\r\n  4 dialogue\r\n  5 debug overlay\r\n\r\n");
        out.push_str("Entities:\r\n");
        for (idx, entity) in self.scene.entities.iter().enumerate() {
            let marker = if idx == self.selected_entity {
                ">"
            } else {
                " "
            };
            out.push_str(&format!(
                "{marker} id={} kind={:?} pos={},{} sprite={} flags={}\r\n",
                entity.id,
                entity.kind,
                entity.position.x,
                entity.position.y,
                entity.sprite,
                entity.state_flags.join(", ")
            ));
        }
        if let Some(entity) = self.selected_entity() {
            out.push_str("\r\nSelected metadata:\r\n");
            for (key, value) in &entity.metadata {
                out.push_str(&format!("  {key}: {value}\r\n"));
            }
        }
        truncate_to_screen(out, cols, rows)
    }
}

pub fn truncate_to_screen(text: String, cols: usize, rows: usize) -> String {
    let max_cols = cols.max(1);
    text.lines()
        .take(rows.max(1))
        .map(|line| {
            let mut clipped = line.chars().take(max_cols).collect::<String>();
            clipped.push_str("\r\n");
            clipped
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot_for_filtering() -> VisualRenderSnapshot {
        VisualRenderSnapshot {
            generation: 7,
            view: VisualView::Scene,
            title: "Filter Fixture".to_string(),
            background: "floor".to_string(),
            width: 4,
            height: 3,
            selected_entity_id: None,
            selected_choice: 0,
            tiles: vec![
                VisualRenderTile {
                    position: VisualPosition { x: 0, y: 1 },
                    sprite: "left".to_string(),
                    layer: VisualRenderLayer::Tile,
                },
                VisualRenderTile {
                    position: VisualPosition { x: 1, y: 1 },
                    sprite: "middle".to_string(),
                    layer: VisualRenderLayer::Tile,
                },
                VisualRenderTile {
                    position: VisualPosition { x: 3, y: 1 },
                    sprite: "right".to_string(),
                    layer: VisualRenderLayer::Tile,
                },
                VisualRenderTile {
                    position: VisualPosition { x: 1, y: 2 },
                    sprite: "other-row".to_string(),
                    layer: VisualRenderLayer::Tile,
                },
            ],
            entities: vec![
                VisualRenderEntity {
                    id: "row-one-left".to_string(),
                    kind: VisualEntityKind::Task,
                    label: "Row One Left".to_string(),
                    position: VisualPosition { x: 0, y: 1 },
                    sprite: "task".to_string(),
                    layer: VisualRenderLayer::Entity,
                    selected: false,
                    state_flags: Vec::new(),
                },
                VisualRenderEntity {
                    id: "row-one-right".to_string(),
                    kind: VisualEntityKind::Agent,
                    label: "Row One Right".to_string(),
                    position: VisualPosition { x: 3, y: 1 },
                    sprite: "agent".to_string(),
                    layer: VisualRenderLayer::Entity,
                    selected: false,
                    state_flags: Vec::new(),
                },
                VisualRenderEntity {
                    id: "row-two".to_string(),
                    kind: VisualEntityKind::Memory,
                    label: "Row Two".to_string(),
                    position: VisualPosition { x: 1, y: 2 },
                    sprite: "memory".to_string(),
                    layer: VisualRenderLayer::Entity,
                    selected: false,
                    state_flags: Vec::new(),
                },
            ],
            dialogue_speaker: String::new(),
            dialogue: String::new(),
            status: String::new(),
            choices: Vec::new(),
        }
    }

    #[test]
    fn demo_scene_validates() {
        k9::assert_ok!(VisualScene::demo().validate());
    }

    #[test]
    fn duplicate_entity_ids_are_rejected() {
        let mut scene = VisualScene::demo();
        scene.entities[1].id = scene.entities[0].id.clone();
        assert!(matches!(
            scene.validate(),
            Err(VisualSceneError::DuplicateEntityId(_))
        ));
    }

    #[test]
    fn runtime_toggles_debugger() {
        let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
        assert_eq!(runtime.view(), VisualView::Scene);
        runtime.toggle_debugger();
        assert_eq!(runtime.view(), VisualView::TileDebugger);
    }

    #[test]
    fn scene_frame_contains_selected_entity() {
        let runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
        let frame = runtime.render_text_frame(80, 24);
        assert!(frame.contains("Selected: GameTerm"));
    }

    #[test]
    fn truncate_to_screen_clips_rows_and_columns() {
        let frame = truncate_to_screen("abcdef\n123456\nxyz".to_string(), 3, 2);
        assert_eq!(frame, "abc\r\n123\r\n");
    }

    #[test]
    fn valid_scene_json_loads_from_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("scene.json");
        std::fs::write(
            &path,
            r#"{
                "title": "Loaded Scene",
                "background": "test",
                "width": 2,
                "height": 2,
                "entities": [{
                    "id": "task-one",
                    "kind": "Task",
                    "label": "Task One",
                    "position": { "x": 1, "y": 1 },
                    "sprite": "task"
                }],
                "dialogue_speaker": "Loader",
                "dialogue": "Loaded from disk.",
                "choices": [{
                    "label": "Open docs",
                    "kind": { "OpenFile": { "path": "docs/gameterm-scene-mode.md" } }
                }]
            }"#,
        )
        .unwrap();

        let scene = VisualScene::load_from_path(path).unwrap();
        assert_eq!(scene.title, "Loaded Scene");
        assert_eq!(scene.entities[0].id, "task-one");
        assert!(matches!(
            scene.choices[0].kind,
            SceneActionKind::OpenFile { .. }
        ));
    }

    #[test]
    fn malformed_json_returns_scene_json_error() {
        assert!(matches!(
            VisualScene::from_json("{"),
            Err(VisualSceneError::Json(_))
        ));
    }

    #[test]
    fn out_of_bounds_entity_is_rejected() {
        let mut scene = VisualScene::demo();
        scene.entities[0].position.x = scene.width;
        assert!(matches!(
            scene.validate(),
            Err(VisualSceneError::EntityOutOfBounds { .. })
        ));
    }

    #[test]
    fn snapshot_includes_all_demo_entities() {
        let runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
        let snapshot = runtime.render_snapshot();
        assert_eq!(snapshot.entities.len(), 3);
        assert_eq!(snapshot.entities[0].id, "project-gameterm");
        assert_eq!(snapshot.tiles.len(), snapshot.width * snapshot.height);
    }

    #[test]
    fn snapshot_marks_selected_entity() {
        let runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
        let snapshot = runtime.render_snapshot();
        assert_eq!(
            snapshot.selected_entity_id.as_deref(),
            Some("project-gameterm")
        );
        assert_eq!(
            snapshot
                .entities
                .iter()
                .filter(|entity| entity.selected)
                .count(),
            1
        );
        assert!(snapshot.entities[0].selected);
    }

    #[test]
    fn selection_changes_increment_generation() {
        let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
        let initial_generation = runtime.generation();
        runtime.select_next_entity();
        assert!(runtime.generation() > initial_generation);
        assert_eq!(
            runtime.render_snapshot().selected_entity_id.as_deref(),
            Some("task-render")
        );
    }

    #[test]
    fn snapshot_generation_is_stable_without_state_changes() {
        let runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
        let first = runtime.render_snapshot();
        let second = runtime.render_snapshot();
        assert_eq!(first.generation, second.generation);
    }

    #[test]
    fn activating_choice_updates_snapshot_status() {
        let mut runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
        let initial = runtime.render_snapshot();

        runtime.activate_choice();
        let activated = runtime.render_snapshot();

        assert!(activated.generation > initial.generation);
        assert_ne!(activated.status, initial.status);
        assert_eq!(activated.status, "Inspecting GameTerm (project-gameterm)");
    }

    #[test]
    fn empty_entities_render_without_selection() {
        let mut scene = VisualScene::demo();
        scene.entities.clear();

        let runtime = SceneRuntime::new(scene).unwrap();
        let snapshot = runtime.render_snapshot();

        assert_eq!(snapshot.selected_entity_id, None);
        assert!(snapshot.entities.is_empty());
        assert_eq!(snapshot.tiles.len(), snapshot.width * snapshot.height);
    }

    #[test]
    fn empty_choices_do_not_change_generation_on_activate() {
        let mut scene = VisualScene::demo();
        scene.choices.clear();

        let mut runtime = SceneRuntime::new(scene).unwrap();
        let initial = runtime.render_snapshot();

        runtime.activate_choice();
        let activated = runtime.render_snapshot();

        assert_eq!(activated.generation, initial.generation);
        assert_eq!(activated.status, initial.status);
        assert!(activated.choices.is_empty());
    }

    #[test]
    fn snapshot_layer_ordering_is_deterministic() {
        let runtime = SceneRuntime::new(VisualScene::demo()).unwrap();
        let first = runtime.render_snapshot();
        let second = runtime.render_snapshot();
        assert_eq!(first.tiles, second.tiles);
        assert_eq!(first.entities, second.entities);
        assert!(first
            .tiles
            .iter()
            .all(|tile| tile.layer == VisualRenderLayer::Tile));
        assert!(first
            .entities
            .iter()
            .all(|entity| entity.layer == VisualRenderLayer::Entity));
    }

    #[test]
    fn visible_tiles_for_row_matches_only_requested_row() {
        let snapshot = snapshot_for_filtering();
        let tiles = visible_tiles_for_row(&snapshot, 1, 0..snapshot.width);

        assert_eq!(tiles.len(), 3);
        assert_eq!(tiles[0].sprite, "left");
        assert_eq!(tiles[1].sprite, "middle");
        assert_eq!(tiles[2].sprite, "right");
        assert!(tiles.iter().all(|tile| tile.position.y == 1));
    }

    #[test]
    fn visible_tiles_for_row_clips_to_viewport_columns() {
        let snapshot = snapshot_for_filtering();
        let tiles = visible_tiles_for_row(&snapshot, 1, 1..99);

        assert_eq!(tiles.len(), 2);
        assert_eq!(tiles[0].position.x, 1);
        assert_eq!(tiles[1].position.x, 3);
    }

    #[test]
    fn intersecting_entities_for_row_matches_row_and_columns() {
        let snapshot = snapshot_for_filtering();
        let entities = intersecting_entities_for_row(&snapshot, 1, 1..4);

        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].id, "row-one-right");
        assert_eq!(entities[0].position, VisualPosition { x: 3, y: 1 });
    }

    #[test]
    fn row_filter_helpers_return_empty_for_empty_data() {
        let mut snapshot = snapshot_for_filtering();
        snapshot.tiles.clear();
        snapshot.entities.clear();

        assert!(visible_tiles_for_row(&snapshot, 1, 0..snapshot.width).is_empty());
        assert!(intersecting_entities_for_row(&snapshot, 1, 0..snapshot.width).is_empty());
        assert!(visible_tiles_for_row(&snapshot, snapshot.height, 0..snapshot.width).is_empty());
        assert!(
            intersecting_entities_for_row(&snapshot, snapshot.height, 0..snapshot.width).is_empty()
        );
    }
}
