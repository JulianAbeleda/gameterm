use serde::{Deserialize, Serialize};

use crate::VisualCondition;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualModeDescriptor {
    pub mode_id: String,
    pub label: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub scene_profile: Option<String>,
    #[serde(default)]
    pub allowed_actions: Vec<String>,
    #[serde(default)]
    pub default_transition: Option<String>,
    #[serde(default, skip_serializing_if = "VisualModeLifecycle::is_empty")]
    pub lifecycle: VisualModeLifecycle,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub input_map: Vec<VisualInputBinding>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualModeLifecycle {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enter_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub update_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_status: Option<String>,
}

impl VisualModeLifecycle {
    pub fn is_empty(&self) -> bool {
        self.enter_status.is_none() && self.update_status.is_none() && self.exit_status.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualInputBinding {
    pub input: String,
    pub action: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<VisualCondition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualLayerState {
    pub layer_id: String,
    pub state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub input_map: Vec<VisualInputBinding>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub transitions: Vec<VisualLayerTransition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualLayerTransition {
    pub input: String,
    pub target_state: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<VisualCondition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualLayerTransitionReport {
    pub layer_id: String,
    pub input: String,
    pub from_state: String,
    pub target_state: String,
    pub result: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualRuntimeEvent {
    pub kind: String,
    pub detail: String,
}

impl Default for VisualModeDescriptor {
    fn default() -> Self {
        default_scene_mode()
    }
}

pub(crate) fn default_scene_mode() -> VisualModeDescriptor {
    VisualModeDescriptor {
        mode_id: "workspace".to_string(),
        label: "Workspace".to_string(),
        description: "Default Scene Mode workspace context".to_string(),
        scene_profile: Some("scene".to_string()),
        allowed_actions: vec![
            "Inspect".to_string(),
            "OpenFile".to_string(),
            "RunCommand".to_string(),
            "Navigate".to_string(),
        ],
        default_transition: None,
        lifecycle: VisualModeLifecycle::default(),
        input_map: Vec::new(),
    }
}

pub(crate) fn is_default_scene_mode(mode: &VisualModeDescriptor) -> bool {
    mode == &default_scene_mode()
}
