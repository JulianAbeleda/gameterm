use serde::{Deserialize, Serialize};
use std::collections::HashSet;

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
}

impl VisualScene {
    pub fn from_json(json: &str) -> Result<Self, VisualSceneError> {
        let scene: Self =
            serde_json::from_str(json).map_err(|err| VisualSceneError::Json(err.to_string()))?;
        scene.validate()?;
        Ok(scene)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
        })
    }

    pub fn view(&self) -> VisualView {
        self.view
    }

    pub fn toggle_debugger(&mut self) {
        self.view = match self.view {
            VisualView::Scene => VisualView::TileDebugger,
            VisualView::TileDebugger => VisualView::Scene,
        };
    }

    pub fn select_next_entity(&mut self) {
        if !self.scene.entities.is_empty() {
            self.selected_entity = (self.selected_entity + 1) % self.scene.entities.len();
        }
    }

    pub fn select_prev_entity(&mut self) {
        if !self.scene.entities.is_empty() {
            self.selected_entity = if self.selected_entity == 0 {
                self.scene.entities.len() - 1
            } else {
                self.selected_entity - 1
            };
        }
    }

    pub fn select_next_choice(&mut self) {
        if !self.scene.choices.is_empty() {
            self.selected_choice = (self.selected_choice + 1) % self.scene.choices.len();
        }
    }

    pub fn select_prev_choice(&mut self) {
        if !self.scene.choices.is_empty() {
            self.selected_choice = if self.selected_choice == 0 {
                self.scene.choices.len() - 1
            } else {
                self.selected_choice - 1
            };
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

fn truncate_to_screen(text: String, cols: usize, rows: usize) -> String {
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
}
