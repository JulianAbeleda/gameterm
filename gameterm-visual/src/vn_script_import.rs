use crate::{
    SceneAction, SceneActionKind, SceneActionPolicy, VisualCondition, VisualDialogueLine,
    VisualEntity, VisualEntityKind, VisualModeDescriptor, VisualPosition, VisualScene,
    VisualStateEntry, VisualStateValue, VnAssetBindings,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VnScriptDialect {
    Rpy,
}

impl VnScriptDialect {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Rpy => "rpy",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VnScriptImportOptions {
    pub dialect: VnScriptDialect,
    pub source_path: Option<PathBuf>,
    pub source_title: String,
    pub source_version: Option<String>,
    pub asset_root: Option<PathBuf>,
    pub bindings: Option<VnAssetBindings>,
    pub title: String,
}

impl Default for VnScriptImportOptions {
    fn default() -> Self {
        Self {
            dialect: VnScriptDialect::Rpy,
            source_path: None,
            source_title: "VN Script Demo".to_string(),
            source_version: None,
            asset_root: None,
            bindings: None,
            title: "VN Script Demo Import".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VnScriptImportReport {
    pub scene: VisualScene,
    pub attribution: VnScriptAttributionManifest,
    pub warnings: Vec<VnScriptImportWarning>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VnScriptImportWarning {
    pub line: usize,
    pub kind: VnScriptImportWarningKind,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VnScriptImportWarningKind {
    UnsupportedStatement,
    UnsupportedAssignment,
    NonMenuJump,
    UnknownJumpTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VnScriptAttributionManifest {
    pub source: String,
    pub source_title: String,
    pub source_dialect: String,
    pub source_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset_root: Option<String>,
    pub license_url: String,
    pub assets: Vec<VnScriptAssetAttribution>,
    pub notes: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VnScriptAssetAttribution {
    pub source_path: String,
    pub output_path: String,
    pub license: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VnScriptImportError {
    #[error("unsupported VN script dialect `{0}`")]
    UnsupportedDialect(String),
    #[error("generated Scene Mode file is invalid: {0}")]
    InvalidScene(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedChoice {
    label: String,
    source_label: String,
    guard: Option<String>,
    target_label: Option<String>,
    line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedScript {
    dialogue: Vec<VisualDialogueLine>,
    choices: Vec<ParsedChoice>,
    variables: BTreeMap<String, VisualStateValue>,
    warnings: Vec<VnScriptImportWarning>,
    label_targets: HashMap<String, usize>,
}

pub fn import_vn_script_scene(
    source: &str,
    options: VnScriptImportOptions,
) -> Result<VnScriptImportReport, VnScriptImportError> {
    if options.dialect != VnScriptDialect::Rpy {
        return Err(VnScriptImportError::UnsupportedDialect(
            options.dialect.as_str().to_string(),
        ));
    }

    let mut parsed = parse_rpy_subset(source);
    let choices = generated_choices(&parsed.choices, &parsed.label_targets, &mut parsed.warnings);
    let scene = build_scene(
        &options,
        parsed.dialogue,
        choices,
        &parsed.variables,
        &parsed.warnings,
    );
    scene
        .validate()
        .map_err(|err| VnScriptImportError::InvalidScene(err.to_string()))?;
    let attribution = build_attribution(&options, &parsed.warnings);

    Ok(VnScriptImportReport {
        scene,
        attribution,
        warnings: parsed.warnings,
    })
}

fn parse_rpy_subset(source: &str) -> ParsedScript {
    let mut dialogue = Vec::new();
    let mut choices = Vec::new();
    let mut variables = BTreeMap::new();
    let mut warnings = Vec::new();
    let mut label_targets = HashMap::new();
    let mut current_label = "start".to_string();
    let mut pending_choice: Option<ParsedChoice> = None;

    for (line_index, raw_line) in source.lines().enumerate() {
        let line_no = line_index + 1;
        let line = strip_comment(raw_line);
        let stripped = line.trim();
        if stripped.is_empty() {
            continue;
        }

        if let Some(label) = parse_label(stripped) {
            current_label = label.to_string();
            label_targets
                .entry(current_label.clone())
                .or_insert(dialogue.len());
            pending_choice = None;
            continue;
        }

        if let Some((key, raw_value)) = parse_assignment(stripped) {
            if let Some(value) = parse_state_value(raw_value) {
                variables.insert(key.to_string(), value);
            } else {
                warnings.push(warning(
                    line_no,
                    VnScriptImportWarningKind::UnsupportedAssignment,
                    format!("unsupported assignment expression: {stripped}"),
                ));
            }
            continue;
        }

        if is_menu_start(stripped) {
            pending_choice = None;
            continue;
        }

        if let Some((label, guard)) = parse_choice(stripped) {
            pending_choice = Some(ParsedChoice {
                label,
                source_label: current_label.clone(),
                guard,
                target_label: None,
                line: line_no,
            });
            continue;
        }

        if let Some(target_label) = parse_jump(stripped) {
            if let Some(mut choice) = pending_choice.take() {
                choice.target_label = Some(target_label.to_string());
                choices.push(choice);
            } else {
                warnings.push(warning(
                    line_no,
                    VnScriptImportWarningKind::NonMenuJump,
                    format!("non-menu jump is recorded as source flow only: {target_label}"),
                ));
            }
            continue;
        }

        if let Some((speaker, text)) = parse_say(stripped) {
            dialogue.push(VisualDialogueLine {
                speaker,
                text,
                portrait: None,
                metadata: vec![
                    ("source_label".to_string(), current_label.clone()),
                    ("source_line".to_string(), line_no.to_string()),
                ],
            });
            continue;
        }

        if let Some(text) = parse_quoted_line(stripped) {
            dialogue.push(VisualDialogueLine {
                speaker: "Narrator".to_string(),
                text,
                portrait: None,
                metadata: vec![
                    ("source_label".to_string(), current_label.clone()),
                    ("source_line".to_string(), line_no.to_string()),
                ],
            });
            continue;
        }

        if matches!(stripped, "return" | "pass") {
            continue;
        }

        warnings.push(warning(
            line_no,
            VnScriptImportWarningKind::UnsupportedStatement,
            format!("unsupported statement skipped: {stripped}"),
        ));
    }

    ParsedScript {
        dialogue,
        choices,
        variables,
        warnings,
        label_targets,
    }
}

fn generated_choices(
    choices: &[ParsedChoice],
    label_targets: &HashMap<String, usize>,
    warnings: &mut Vec<VnScriptImportWarning>,
) -> Vec<SceneAction> {
    choices
        .iter()
        .map(|choice| {
            let target_label = choice.target_label.as_deref().unwrap_or("start");
            let target = label_targets.get(target_label).copied().unwrap_or_else(|| {
                warnings.push(warning(
                    choice.line,
                    VnScriptImportWarningKind::UnknownJumpTarget,
                    format!("unknown jump target {target_label}; using first dialogue line"),
                ));
                0
            });
            let mut conditions = Vec::new();
            if let Some(guard) = &choice.guard {
                conditions.push(VisualCondition {
                    source: None,
                    variable: guard.clone(),
                    equals: VisualStateValue::Bool(true),
                });
            }
            SceneAction {
                label: choice.label.clone(),
                kind: SceneActionKind::AdvanceDialogue { target },
                policy: Some(SceneActionPolicy {
                    origin: "vn_script_import".to_string(),
                    risk: "state_change".to_string(),
                    scope: "scene".to_string(),
                    requires_confirmation: false,
                    summary: Some(format!(
                        "Continue imported VN script at label {target_label}"
                    )),
                }),
                conditions,
            }
        })
        .collect()
}

fn build_scene(
    options: &VnScriptImportOptions,
    mut dialogue: Vec<VisualDialogueLine>,
    choices: Vec<SceneAction>,
    variables: &BTreeMap<String, VisualStateValue>,
    warnings: &[VnScriptImportWarning],
) -> VisualScene {
    let source_path = options
        .source_path
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_default();
    let mut scene_variables = vec![
        VisualStateEntry {
            key: "source_dialect".to_string(),
            value: VisualStateValue::Text(options.dialect.as_str().to_string()),
        },
        VisualStateEntry {
            key: "source_title".to_string(),
            value: VisualStateValue::Text(options.source_title.clone()),
        },
        VisualStateEntry {
            key: "source_file".to_string(),
            value: VisualStateValue::Text(source_path.clone()),
        },
    ];
    for (key, value) in variables {
        scene_variables.push(VisualStateEntry {
            key: key.clone(),
            value: value.clone(),
        });
    }
    if !warnings.is_empty() {
        scene_variables.push(VisualStateEntry {
            key: "vn_script_import_warnings".to_string(),
            value: VisualStateValue::Number(warnings.len() as i64),
        });
    }

    if dialogue.is_empty() {
        dialogue.push(VisualDialogueLine {
            speaker: "Importer".to_string(),
            text: "No supported VN script dialogue lines were found.".to_string(),
            portrait: None,
            metadata: vec![(
                "source_dialect".to_string(),
                options.dialect.as_str().to_string(),
            )],
        });
    }

    VisualScene {
        title: options.title.clone(),
        background: binding_background(options),
        width: 16,
        height: 9,
        mode: VisualModeDescriptor {
            mode_id: "vn-script-demo".to_string(),
            label: "VN Script Demo".to_string(),
            description: "Imported VN script subset demo".to_string(),
            scene_profile: Some("scene".to_string()),
            allowed_actions: vec!["Inspect".to_string(), "AdvanceDialogue".to_string()],
            default_transition: None,
            lifecycle: Default::default(),
            input_map: Vec::new(),
        },
        layers: Vec::new(),
        variables: scene_variables,
        rpg: Default::default(),
        entities: vec![
            VisualEntity {
                id: "vn-script-source".to_string(),
                kind: VisualEntityKind::Project,
                label: options.source_title.clone(),
                position: VisualPosition { x: 2, y: 2 },
                sprite: "project_core".to_string(),
                visible: true,
                state_flags: Vec::new(),
                metadata: vec![
                    (
                        "source_dialect".to_string(),
                        options.dialect.as_str().to_string(),
                    ),
                    ("source_file".to_string(), source_path),
                ],
            },
            VisualEntity {
                id: "vn-script-narrator".to_string(),
                kind: VisualEntityKind::Agent,
                label: "Narrator".to_string(),
                position: VisualPosition { x: 7, y: 4 },
                sprite: binding_character_sprite(options, "guide", "neutral", "agent_idle"),
                visible: true,
                state_flags: Vec::new(),
                metadata: vec![(
                    "source_dialect".to_string(),
                    options.dialect.as_str().to_string(),
                )],
            },
            VisualEntity {
                id: "vn-script-importer".to_string(),
                kind: VisualEntityKind::Task,
                label: "Import Check".to_string(),
                position: VisualPosition { x: 12, y: 5 },
                sprite: "task_tile".to_string(),
                visible: true,
                state_flags: Vec::new(),
                metadata: vec![("warnings".to_string(), warnings.len().to_string())],
            },
        ],
        dialogue_speaker: dialogue[0].speaker.clone(),
        dialogue: dialogue[0].text.clone(),
        dialogue_lines: dialogue,
        choices,
    }
}

fn binding_background(options: &VnScriptImportOptions) -> String {
    options
        .bindings
        .as_ref()
        .and_then(|bindings| bindings.default_background.clone())
        .unwrap_or_else(|| "workspace-map".to_string())
}

fn binding_character_sprite(
    options: &VnScriptImportOptions,
    character: &str,
    expression: &str,
    fallback: &str,
) -> String {
    options
        .bindings
        .as_ref()
        .and_then(|bindings| bindings.characters.get(character))
        .and_then(|binding| binding.expressions.get(expression))
        .cloned()
        .unwrap_or_else(|| fallback.to_string())
}

fn build_attribution(
    options: &VnScriptImportOptions,
    warnings: &[VnScriptImportWarning],
) -> VnScriptAttributionManifest {
    VnScriptAttributionManifest {
        source: "vn-script-subset".to_string(),
        source_title: options.source_title.clone(),
        source_dialect: options.dialect.as_str().to_string(),
        source_version: options
            .source_version
            .clone()
            .unwrap_or_else(|| "unknown".to_string()),
        source_path: options
            .source_path
            .as_ref()
            .map(|path| path.display().to_string()),
        asset_root: options
            .asset_root
            .as_ref()
            .map(|path| path.display().to_string()),
        license_url: "https://www.renpy.org/doc/html/license.html".to_string(),
        assets: Vec::new(),
        notes: vec![
            "This importer records asset provenance but does not copy assets.".to_string(),
            "The checked-in fixture source is GameTerm-authored Ren'Py-shaped test content."
                .to_string(),
            "When importing upstream visual-novel script material, preserve upstream credits and license files before vendoring assets.".to_string(),
        ],
        warnings: warnings.iter().map(format_warning).collect(),
    }
}

fn strip_comment(line: &str) -> String {
    let mut in_string = false;
    let mut escaped = false;
    let mut out = String::new();
    for ch in line.chars() {
        if escaped {
            out.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' && in_string {
            out.push(ch);
            escaped = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            out.push(ch);
            continue;
        }
        if ch == '#' && !in_string {
            break;
        }
        out.push(ch);
    }
    out.trim_end().to_string()
}

fn parse_label(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("label ")?;
    let label = rest.strip_suffix(':')?.trim();
    is_identifier(label).then_some(label)
}

fn parse_assignment(line: &str) -> Option<(&str, &str)> {
    let rest = line
        .strip_prefix("default ")
        .or_else(|| line.strip_prefix("$ "))?;
    let (key, value) = rest.split_once('=')?;
    let key = key.trim();
    is_identifier(key).then_some((key, value.trim()))
}

fn is_menu_start(line: &str) -> bool {
    line == "menu:"
        || line
            .strip_prefix("menu ")
            .and_then(|rest| rest.strip_suffix(':'))
            .map(str::trim)
            .is_some_and(is_identifier)
}

fn parse_choice(line: &str) -> Option<(String, Option<String>)> {
    let (quoted, rest) = parse_leading_quoted(line)?;
    let rest = rest.trim();
    if let Some(rest) = rest.strip_prefix("if ") {
        let guard = rest.strip_suffix(':')?.trim();
        return is_identifier(guard).then_some((quoted, Some(guard.to_string())));
    }
    (rest == ":").then_some((quoted, None))
}

fn parse_jump(line: &str) -> Option<&str> {
    let target = line.strip_prefix("jump ")?.trim();
    is_identifier(target).then_some(target)
}

fn parse_say(line: &str) -> Option<(String, String)> {
    let (speaker, rest) = line.split_once(' ')?;
    if !is_identifier(speaker) {
        return None;
    }
    let text = parse_quoted_line(rest.trim())?;
    Some((speaker.to_string(), text))
}

fn parse_quoted_line(line: &str) -> Option<String> {
    let (quoted, rest) = parse_leading_quoted(line)?;
    rest.trim().is_empty().then_some(quoted)
}

fn parse_leading_quoted(line: &str) -> Option<(String, &str)> {
    let mut chars = line.char_indices();
    if chars.next()?.1 != '"' {
        return None;
    }
    let mut escaped = false;
    let mut value = String::new();
    for (idx, ch) in chars {
        if escaped {
            value.push(unescape_char(ch));
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == '"' {
            return Some((value, &line[idx + ch.len_utf8()..]));
        }
        value.push(ch);
    }
    None
}

fn parse_state_value(raw: &str) -> Option<VisualStateValue> {
    match raw {
        "True" | "true" => Some(VisualStateValue::Bool(true)),
        "False" | "false" => Some(VisualStateValue::Bool(false)),
        _ => {
            if let Ok(value) = raw.parse::<i64>() {
                Some(VisualStateValue::Number(value))
            } else {
                parse_quoted_line(raw).map(VisualStateValue::Text)
            }
        }
    }
}

fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn unescape_char(ch: char) -> char {
    match ch {
        'n' => '\n',
        'r' => '\r',
        't' => '\t',
        '"' => '"',
        '\\' => '\\',
        other => other,
    }
}

fn warning(
    line: usize,
    kind: VnScriptImportWarningKind,
    detail: impl Into<String>,
) -> VnScriptImportWarning {
    VnScriptImportWarning {
        line,
        kind,
        detail: detail.into(),
    }
}

fn format_warning(warning: &VnScriptImportWarning) -> String {
    format!("line {}: {}", warning.line, warning.detail)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::VnAssetBindingCharacter;

    const SOURCE: &str = r#"
default met_guide = True

label start:
    "A terminal window glows like a tiny stage."
    guide "Scene Mode can read a Ren'Py-shaped script."

    menu:
        "Ask about Scene Mode." if met_guide:
            jump explain
        "End the demo.":
            jump ending

label explain:
    guide "Labels become dialogue targets, and menu items become Scene Mode choices."
    jump ending

label ending:
    "The imported demo is ready."
    return
"#;

    #[test]
    fn vn_script_import_generates_scene_choices_and_warnings() {
        let report = import_vn_script_scene(
            SOURCE,
            VnScriptImportOptions {
                source_path: Some(PathBuf::from("demo.rpy")),
                source_title: "Demo".to_string(),
                source_version: Some("fixture".to_string()),
                title: "Demo Import".to_string(),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(report.scene.title, "Demo Import");
        assert_eq!(report.scene.dialogue_lines.len(), 4);
        assert_eq!(report.scene.choices.len(), 2);
        assert_eq!(
            report.scene.choices[0].policy.as_ref().unwrap().origin,
            "vn_script_import"
        );
        assert_eq!(report.scene.choices[0].conditions[0].variable, "met_guide");
        assert_eq!(
            report.scene.choices[0].kind,
            SceneActionKind::AdvanceDialogue { target: 2 }
        );
        assert!(report.warnings.iter().any(|warning| {
            warning.kind == VnScriptImportWarningKind::NonMenuJump
                && warning.detail.contains("ending")
        }));
        assert_eq!(report.attribution.source_dialect, "rpy");
        assert_eq!(report.attribution.source_version, "fixture");
    }

    #[test]
    fn vn_script_import_rejects_unknown_assignment_values_as_warnings() {
        let report = import_vn_script_scene(
            r#"
default count = call_python()
label start:
    "Hello"
"#,
            VnScriptImportOptions::default(),
        )
        .unwrap();

        assert!(report.warnings.iter().any(|warning| {
            warning.kind == VnScriptImportWarningKind::UnsupportedAssignment
                && warning.detail.contains("call_python")
        }));
        assert!(report.scene.variables.iter().any(|entry| {
            entry.key == "vn_script_import_warnings" && entry.value == VisualStateValue::Number(1)
        }));
    }

    #[test]
    fn vn_script_import_applies_asset_bindings_when_present() {
        let mut guide = VnAssetBindingCharacter::default();
        guide.expressions.insert(
            "neutral".to_string(),
            "vn.character.guide.neutral".to_string(),
        );
        let mut bindings = VnAssetBindings {
            default_background: Some("vn.background.school_classroom".to_string()),
            ..Default::default()
        };
        bindings.characters.insert("guide".to_string(), guide);

        let report = import_vn_script_scene(
            SOURCE,
            VnScriptImportOptions {
                bindings: Some(bindings),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(report.scene.background, "vn.background.school_classroom");
        assert_eq!(
            report
                .scene
                .entities
                .iter()
                .find(|entity| entity.id == "vn-script-narrator")
                .unwrap()
                .sprite,
            "vn.character.guide.neutral"
        );
    }
}
