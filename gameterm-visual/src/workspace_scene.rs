use std::fs;
use std::path::{Path, PathBuf};

use crate::{
    RunCommandTarget, SceneAction, SceneActionKind, VisualEntity, VisualEntityKind,
    VisualInputBinding, VisualModeDescriptor, VisualModeLifecycle, VisualPosition, VisualRpgState,
    VisualScene, VisualStateEntry, VisualStateValue,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenePaneContext {
    pub pane_id: usize,
    pub mux_window_id: usize,
    pub cwd: Option<PathBuf>,
    pub foreground_process_name: Option<String>,
    pub progress: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneWorkspaceContext {
    pub cwd: PathBuf,
    pub pane: Option<ScenePaneContext>,
    pub max_files: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceSceneReport {
    pub root: PathBuf,
    pub discovered_file_count: usize,
    pub used_pane_cwd: bool,
}

pub fn generate_workspace_scene(
    context: SceneWorkspaceContext,
) -> (VisualScene, WorkspaceSceneReport) {
    let workspace_root = discover_workspace_root(&context.cwd);
    let files = discover_files(&workspace_root, context.max_files);
    let used_pane_cwd = context
        .pane
        .as_ref()
        .and_then(|pane| pane.cwd.as_ref())
        .is_some();
    let active_cwd = context
        .pane
        .as_ref()
        .and_then(|pane| pane.cwd.as_ref())
        .unwrap_or(&context.cwd);
    let file_count = files.len();
    let mut entities = vec![
        VisualEntity {
            id: "workspace-root".to_string(),
            kind: VisualEntityKind::Project,
            label: workspace_root
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("Workspace")
                .to_string(),
            position: VisualPosition { x: 2, y: 2 },
            sprite: "project_core".to_string(),
            visible: true,
            state_flags: vec!["active".to_string()],
            metadata: vec![
                ("path".to_string(), workspace_root.display().to_string()),
                (
                    "discovery".to_string(),
                    "rust-workspace-generator".to_string(),
                ),
            ],
        },
        VisualEntity {
            id: "active-pane".to_string(),
            kind: VisualEntityKind::Agent,
            label: active_pane_label(context.pane.as_ref()),
            position: VisualPosition { x: 8, y: 3 },
            sprite: "agent_idle".to_string(),
            visible: true,
            state_flags: vec!["observing".to_string()],
            metadata: pane_metadata(context.pane.as_ref(), active_cwd),
        },
        VisualEntity {
            id: "workspace-files".to_string(),
            kind: VisualEntityKind::Memory,
            label: format!("{} files", file_count),
            position: VisualPosition { x: 14, y: 2 },
            sprite: "memory_note".to_string(),
            visible: true,
            state_flags: vec!["discovered".to_string()],
            metadata: files
                .iter()
                .take(8)
                .map(|path| ("file".to_string(), path.display().to_string()))
                .collect(),
        },
    ];

    for (index, path) in files.iter().take(4).enumerate() {
        entities.push(VisualEntity {
            id: format!("workspace-file-{index}"),
            kind: VisualEntityKind::Task,
            label: path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("file")
                .to_string(),
            position: VisualPosition {
                x: 4 + index * 3,
                y: 6,
            },
            sprite: "task_tile".to_string(),
            visible: true,
            state_flags: vec!["file".to_string()],
            metadata: vec![("path".to_string(), path.display().to_string())],
        });
    }

    let scene = VisualScene {
        title: "Active Pane Workspace".to_string(),
        background: "workspace-map".to_string(),
        width: 18,
        height: 9,
        mode: VisualModeDescriptor {
            mode_id: "active_pane_workspace".to_string(),
            label: "Active Pane Workspace".to_string(),
            description: "Generated workspace scene from the active pane context".to_string(),
            scene_profile: Some("workspace".to_string()),
            allowed_actions: vec![
                "Inspect".to_string(),
                "OpenFile".to_string(),
                "RunCommand".to_string(),
            ],
            default_transition: None,
            lifecycle: VisualModeLifecycle {
                enter_status: Some("Generated active pane workspace".to_string()),
                update_status: Some("Active pane workspace refreshed".to_string()),
                exit_status: Some("Closed active pane workspace".to_string()),
            },
            input_map: vec![VisualInputBinding {
                input: "reload".to_string(),
                action: "run_update_hooks".to_string(),
                conditions: Vec::new(),
            }],
        },
        layers: Vec::new(),
        variables: vec![
            text_var("workspace_mode", "active_pane"),
            text_var("workspace_root", workspace_root.display().to_string()),
            text_var("active_cwd", active_cwd.display().to_string()),
            text_var("discovery_source", "rust-workspace-generator"),
            VisualStateEntry {
                key: "discovered_file_count".to_string(),
                value: VisualStateValue::Number(file_count as i64),
            },
            text_var("pane_context", pane_context_value(context.pane.as_ref())),
        ],
        rpg: VisualRpgState::default(),
        entities,
        dialogue_speaker: "GameTerm".to_string(),
        dialogue: format!(
            "Scene Mode generated this workspace from {}.",
            active_cwd.display()
        ),
        dialogue_lines: Vec::new(),
        choices: workspace_choices(&workspace_root, files.first()),
    };

    (
        scene,
        WorkspaceSceneReport {
            root: workspace_root,
            discovered_file_count: file_count,
            used_pane_cwd,
        },
    )
}

fn workspace_choices(root: &Path, first_file: Option<&PathBuf>) -> Vec<SceneAction> {
    let mut choices = vec![
        SceneAction {
            label: "Inspect workspace".to_string(),
            kind: SceneActionKind::Inspect,
            policy: None,
            conditions: Vec::new(),
        },
        SceneAction {
            label: "Run git status".to_string(),
            kind: SceneActionKind::RunCommand {
                argv: vec![
                    "git".to_string(),
                    "status".to_string(),
                    "--short".to_string(),
                ],
                cwd: Some(root.display().to_string()),
                target: RunCommandTarget::Tab,
            },
            policy: None,
            conditions: Vec::new(),
        },
    ];

    if let Some(path) = first_file {
        choices.insert(
            1,
            SceneAction {
                label: format!("Open {}", path.display()),
                kind: SceneActionKind::OpenFile {
                    path: path.display().to_string(),
                },
                policy: None,
                conditions: Vec::new(),
            },
        );
    }

    choices
}

fn text_var(key: impl Into<String>, value: impl Into<String>) -> VisualStateEntry {
    VisualStateEntry {
        key: key.into(),
        value: VisualStateValue::Text(value.into()),
    }
}

fn active_pane_label(pane: Option<&ScenePaneContext>) -> String {
    pane.and_then(|pane| pane.foreground_process_name.as_ref())
        .filter(|name| !name.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| "Active Pane".to_string())
}

fn pane_context_value(pane: Option<&ScenePaneContext>) -> String {
    match pane {
        Some(pane) => format!("pane:{} window:{}", pane.pane_id, pane.mux_window_id),
        None => "none".to_string(),
    }
}

fn pane_metadata(pane: Option<&ScenePaneContext>, active_cwd: &Path) -> Vec<(String, String)> {
    let mut metadata = vec![("cwd".to_string(), active_cwd.display().to_string())];
    if let Some(pane) = pane {
        metadata.push(("pane_id".to_string(), pane.pane_id.to_string()));
        metadata.push(("mux_window_id".to_string(), pane.mux_window_id.to_string()));
        if let Some(process) = pane.foreground_process_name.as_ref() {
            metadata.push(("process".to_string(), process.clone()));
        }
        if let Some(progress) = pane.progress.as_ref() {
            metadata.push(("progress".to_string(), progress.clone()));
        }
    }
    metadata
}

fn discover_workspace_root(cwd: &Path) -> PathBuf {
    let mut current = if cwd.is_dir() {
        cwd.to_path_buf()
    } else {
        cwd.parent().unwrap_or(cwd).to_path_buf()
    };

    loop {
        if current.join(".git").exists()
            || current.join("Cargo.toml").exists()
            || current.join("package.json").exists()
        {
            return current;
        }
        if !current.pop() {
            return cwd.to_path_buf();
        }
    }
}

fn discover_files(root: &Path, max_files: usize) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_files(root, root, max_files.max(1), &mut files);
    files
}

fn collect_files(root: &Path, dir: &Path, max_files: usize, files: &mut Vec<PathBuf>) {
    if files.len() >= max_files {
        return;
    }

    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut entries = entries.filter_map(Result::ok).collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        if files.len() >= max_files {
            return;
        }
        let path = entry.path();
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if matches!(name, ".git" | "target" | "node_modules" | ".DS_Store") {
            continue;
        }
        if path.is_dir() {
            collect_files(root, &path, max_files, files);
        } else if path.is_file() {
            files.push(path.strip_prefix(root).unwrap_or(&path).to_path_buf());
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn generated_workspace_scene_validates_and_uses_pane_context() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"fixture\"\n",
        )
        .unwrap();
        fs::write(dir.path().join("README.md"), "fixture").unwrap();

        let (scene, report) = generate_workspace_scene(SceneWorkspaceContext {
            cwd: dir.path().to_path_buf(),
            pane: Some(ScenePaneContext {
                pane_id: 7,
                mux_window_id: 3,
                cwd: Some(dir.path().to_path_buf()),
                foreground_process_name: Some("zsh".to_string()),
                progress: Some("0.42".to_string()),
            }),
            max_files: 8,
        });

        scene.validate().unwrap();
        assert_eq!(report.root, dir.path());
        assert!(report.used_pane_cwd);
        assert!(report.discovered_file_count >= 1);
        assert!(scene
            .variables
            .iter()
            .any(|entry| entry.key == "pane_context"
                && entry.value == VisualStateValue::Text("pane:7 window:3".to_string())));
        assert!(scene
            .entities
            .iter()
            .any(|entity| entity.id == "active-pane" && entity.label == "zsh"));
    }

    #[test]
    fn generated_workspace_scene_falls_back_without_pane_context() {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src").join("main.rs"), "fn main() {}\n").unwrap();

        let (scene, report) = generate_workspace_scene(SceneWorkspaceContext {
            cwd: dir.path().join("src"),
            pane: None,
            max_files: 1,
        });

        scene.validate().unwrap();
        assert!(!report.used_pane_cwd);
        assert_eq!(report.discovered_file_count, 1);
        assert!(scene
            .variables
            .iter()
            .any(|entry| entry.key == "pane_context"
                && entry.value == VisualStateValue::Text("none".to_string())));
    }
}
