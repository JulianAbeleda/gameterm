use crate::{VisualSpriteDefinition, VisualSpriteManifest, VisualSpriteManifestError};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VnAssetCatalog {
    pub asset_catalog_version: u64,
    pub purpose: String,
    pub policy: VnAssetCatalogPolicy,
    pub sources: Vec<VnAssetCatalogSource>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VnAssetCatalogPolicy {
    pub default: String,
    #[serde(default)]
    pub preferred_repo_assets: Vec<String>,
    #[serde(default)]
    pub local_only_when: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VnAssetCatalogSource {
    pub id: String,
    pub role: String,
    pub title: String,
    pub author: String,
    pub source_url: String,
    pub download_name: String,
    pub license: String,
    pub license_url: String,
    pub source_disclosure: String,
    pub repo_policy: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attribution: Option<String>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VnAssetIntakeOptions {
    pub catalog_path: PathBuf,
    pub source_root: PathBuf,
    pub output_root: PathBuf,
    pub sprite_manifest_path: Option<PathBuf>,
    pub base_manifest_path: Option<PathBuf>,
    pub force: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VnAssetIntakeReport {
    pub sprite_manifest: VisualSpriteManifest,
    pub attribution: VnAssetAttributionManifest,
    pub bindings: VnAssetBindings,
    pub warnings: Vec<VnAssetIntakeWarning>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VnAssetAttributionManifest {
    pub asset_intake_version: u64,
    pub generated_by: String,
    pub sources: Vec<VnAssetAttributionSource>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VnAssetAttributionSource {
    pub id: String,
    pub title: String,
    pub author: String,
    pub source_url: String,
    pub license: String,
    pub license_url: String,
    pub repo_policy: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attribution: Option<String>,
    #[serde(default)]
    pub used_assets: Vec<VnAssetUsedAsset>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VnAssetUsedAsset {
    pub sprite_id: String,
    pub source_path: String,
    pub output_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct VnAssetBindings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_background: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub characters: BTreeMap<String, VnAssetBindingCharacter>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct VnAssetBindingCharacter {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub expressions: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VnAssetIntakeWarning {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    pub kind: VnAssetIntakeWarningKind,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VnAssetIntakeWarningKind {
    MissingSourceDirectory,
    MissingExpectedAsset,
    BlockedSource,
    CompositionRequired,
    UnsupportedRole,
    UnsupportedImageFormat,
    OutputExists,
    DuplicateSpriteId,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VnAssetIntakeError {
    #[error("asset catalog json error: {0}")]
    CatalogJson(String),
    #[error("asset catalog file error for `{path}`: {message}")]
    CatalogFile { path: String, message: String },
    #[error("sprite manifest error: {0}")]
    SpriteManifest(String),
    #[error("asset file error for `{path}`: {message}")]
    File { path: String, message: String },
}

#[derive(Debug, Clone, Copy)]
struct ExpectedAsset {
    source_file: &'static str,
    output_file: &'static str,
    sprite_id: &'static str,
    character: Option<&'static str>,
    expression: Option<&'static str>,
    is_default_background: bool,
}

pub fn run_vn_asset_intake(
    options: VnAssetIntakeOptions,
) -> Result<VnAssetIntakeReport, VnAssetIntakeError> {
    let catalog = load_catalog(&options.catalog_path)?;
    let mut warnings = Vec::new();
    let mut sprites = load_base_sprites(options.base_manifest_path.as_deref())?;
    let mut sprite_ids = sprites
        .iter()
        .map(|sprite| sprite.id.clone())
        .collect::<HashSet<_>>();
    let mut attribution_sources = Vec::new();
    let mut bindings = VnAssetBindings::default();

    for source in catalog.sources {
        let mut used_assets = Vec::new();
        if !source_is_allowed(&source, &mut warnings) {
            attribution_sources.push(attribution_source(source, used_assets));
            continue;
        }

        let Some(expected_assets) = expected_assets_for_role(&source.role) else {
            warnings.push(warning(
                Some(source.id.clone()),
                VnAssetIntakeWarningKind::UnsupportedRole,
                format!("unsupported VN asset role `{}`", source.role),
            ));
            attribution_sources.push(attribution_source(source, used_assets));
            continue;
        };

        if expected_assets.is_empty() {
            warnings.push(warning(
                Some(source.id.clone()),
                VnAssetIntakeWarningKind::CompositionRequired,
                "source is approved but requires sprite composition before it can produce manifest entries",
            ));
            attribution_sources.push(attribution_source(source, used_assets));
            continue;
        }

        let source_dir = options.source_root.join(&source.id);
        if !source_dir.is_dir() {
            warnings.push(warning(
                Some(source.id.clone()),
                VnAssetIntakeWarningKind::MissingSourceDirectory,
                format!("source directory not found: {}", source_dir.display()),
            ));
            attribution_sources.push(attribution_source(source, used_assets));
            continue;
        }

        for expected in expected_assets {
            let source_path = source_dir.join(expected.source_file);
            if !source_path.is_file() {
                warnings.push(warning(
                    Some(source.id.clone()),
                    VnAssetIntakeWarningKind::MissingExpectedAsset,
                    format!("expected asset not found: {}", source_path.display()),
                ));
                continue;
            }
            if !is_supported_image_path(&source_path) {
                warnings.push(warning(
                    Some(source.id.clone()),
                    VnAssetIntakeWarningKind::UnsupportedImageFormat,
                    format!("unsupported image format: {}", source_path.display()),
                ));
                continue;
            }
            if !sprite_ids.insert(expected.sprite_id.to_string()) {
                warnings.push(warning(
                    Some(source.id.clone()),
                    VnAssetIntakeWarningKind::DuplicateSpriteId,
                    format!("sprite id already exists: {}", expected.sprite_id),
                ));
                continue;
            }

            let output_path = options.output_root.join(expected.output_file);
            copy_asset(&source_path, &output_path, options.force)?;
            let manifest_path =
                manifest_entry_path(&output_path, options.sprite_manifest_path.as_deref());
            sprites.push(VisualSpriteDefinition {
                id: expected.sprite_id.to_string(),
                path: manifest_path.clone(),
            });
            used_assets.push(VnAssetUsedAsset {
                sprite_id: expected.sprite_id.to_string(),
                source_path: source_path.display().to_string(),
                output_path: manifest_path,
            });
            apply_binding(&mut bindings, expected);
        }

        attribution_sources.push(attribution_source(source, used_assets));
    }

    let sprite_manifest = VisualSpriteManifest { sprites };
    sprite_manifest
        .validate()
        .map_err(|err| VnAssetIntakeError::SpriteManifest(err.to_string()))?;
    let warnings_text = warnings
        .iter()
        .map(|warning| match &warning.source_id {
            Some(source_id) => format!("{source_id}: {}", warning.detail),
            None => warning.detail.clone(),
        })
        .collect();
    let attribution = VnAssetAttributionManifest {
        asset_intake_version: 1,
        generated_by: "scene_vn_asset_intake".to_string(),
        sources: attribution_sources,
        warnings: warnings_text,
    };

    Ok(VnAssetIntakeReport {
        sprite_manifest,
        attribution,
        bindings,
        warnings,
    })
}

fn load_catalog(path: &Path) -> Result<VnAssetCatalog, VnAssetIntakeError> {
    let json = std::fs::read_to_string(path).map_err(|err| VnAssetIntakeError::CatalogFile {
        path: path.display().to_string(),
        message: err.to_string(),
    })?;
    serde_json::from_str(&json).map_err(|err| VnAssetIntakeError::CatalogJson(err.to_string()))
}

fn load_base_sprites(
    path: Option<&Path>,
) -> Result<Vec<VisualSpriteDefinition>, VnAssetIntakeError> {
    let Some(path) = path else {
        return Ok(Vec::new());
    };
    let manifest =
        VisualSpriteManifest::load_from_path(path).map_err(|err: VisualSpriteManifestError| {
            VnAssetIntakeError::SpriteManifest(err.to_string())
        })?;
    Ok(manifest
        .resolve_against(path)
        .sprites
        .into_iter()
        .map(|sprite| VisualSpriteDefinition {
            id: sprite.id,
            path: sprite.path,
        })
        .collect())
}

fn source_is_allowed(
    source: &VnAssetCatalogSource,
    warnings: &mut Vec<VnAssetIntakeWarning>,
) -> bool {
    match source.repo_policy.as_str() {
        "allowed_with_provenance" | "allowed_with_attribution" | "local_only" => true,
        "blocked" => {
            warnings.push(warning(
                Some(source.id.clone()),
                VnAssetIntakeWarningKind::BlockedSource,
                "blocked source skipped by catalog policy",
            ));
            false
        }
        _ => {
            warnings.push(warning(
                Some(source.id.clone()),
                VnAssetIntakeWarningKind::BlockedSource,
                format!("unknown repo_policy `{}` skipped", source.repo_policy),
            ));
            false
        }
    }
}

fn expected_assets_for_role(role: &str) -> Option<&'static [ExpectedAsset]> {
    match role {
        "character_sprite" => Some(&[
            ExpectedAsset {
                source_file: "kiki-neutral.png",
                output_file: "characters/kiki-neutral.png",
                sprite_id: "vn.character.kiki.neutral",
                character: Some("kiki"),
                expression: Some("neutral"),
                is_default_background: false,
            },
            ExpectedAsset {
                source_file: "kiki-breath-0.png",
                output_file: "characters/kiki-breath-0.png",
                sprite_id: "vn.character.kiki.breath.0",
                character: Some("kiki"),
                expression: Some("breath.0"),
                is_default_background: false,
            },
            ExpectedAsset {
                source_file: "kiki-breath-1.png",
                output_file: "characters/kiki-breath-1.png",
                sprite_id: "vn.character.kiki.breath.1",
                character: Some("kiki"),
                expression: Some("breath.1"),
                is_default_background: false,
            },
            ExpectedAsset {
                source_file: "kiki-breath-2.png",
                output_file: "characters/kiki-breath-2.png",
                sprite_id: "vn.character.kiki.breath.2",
                character: Some("kiki"),
                expression: Some("breath.2"),
                is_default_background: false,
            },
            ExpectedAsset {
                source_file: "kiki-breath-3.png",
                output_file: "characters/kiki-breath-3.png",
                sprite_id: "vn.character.kiki.breath.3",
                character: Some("kiki"),
                expression: Some("breath.3"),
                is_default_background: false,
            },
            ExpectedAsset {
                source_file: "kiki-breath-4.png",
                output_file: "characters/kiki-breath-4.png",
                sprite_id: "vn.character.kiki.breath.4",
                character: Some("kiki"),
                expression: Some("breath.4"),
                is_default_background: false,
            },
            ExpectedAsset {
                source_file: "kiki-breath-5.png",
                output_file: "characters/kiki-breath-5.png",
                sprite_id: "vn.character.kiki.breath.5",
                character: Some("kiki"),
                expression: Some("breath.5"),
                is_default_background: false,
            },
            ExpectedAsset {
                source_file: "kiki-blink-0.png",
                output_file: "characters/kiki-blink-0.png",
                sprite_id: "vn.character.kiki.blink.0",
                character: Some("kiki"),
                expression: Some("blink.0"),
                is_default_background: false,
            },
            ExpectedAsset {
                source_file: "kiki-blink-1.png",
                output_file: "characters/kiki-blink-1.png",
                sprite_id: "vn.character.kiki.blink.1",
                character: Some("kiki"),
                expression: Some("blink.1"),
                is_default_background: false,
            },
            ExpectedAsset {
                source_file: "kiki-blink-2.png",
                output_file: "characters/kiki-blink-2.png",
                sprite_id: "vn.character.kiki.blink.2",
                character: Some("kiki"),
                expression: Some("blink.2"),
                is_default_background: false,
            },
            ExpectedAsset {
                source_file: "kiki-blink-3.png",
                output_file: "characters/kiki-blink-3.png",
                sprite_id: "vn.character.kiki.blink.3",
                character: Some("kiki"),
                expression: Some("blink.3"),
                is_default_background: false,
            },
            ExpectedAsset {
                source_file: "kiki-blink-4.png",
                output_file: "characters/kiki-blink-4.png",
                sprite_id: "vn.character.kiki.blink.4",
                character: Some("kiki"),
                expression: Some("blink.4"),
                is_default_background: false,
            },
            ExpectedAsset {
                source_file: "kiki-blink-5.png",
                output_file: "characters/kiki-blink-5.png",
                sprite_id: "vn.character.kiki.blink.5",
                character: Some("kiki"),
                expression: Some("blink.5"),
                is_default_background: false,
            },
            ExpectedAsset {
                source_file: "kiki-happy.png",
                output_file: "characters/kiki-happy.png",
                sprite_id: "vn.character.kiki.happy",
                character: Some("kiki"),
                expression: Some("happy"),
                is_default_background: false,
            },
            ExpectedAsset {
                source_file: "kiki-concerned.png",
                output_file: "characters/kiki-concerned.png",
                sprite_id: "vn.character.kiki.concerned",
                character: Some("kiki"),
                expression: Some("concerned"),
                is_default_background: false,
            },
            ExpectedAsset {
                source_file: "kiki-surprised.png",
                output_file: "characters/kiki-surprised.png",
                sprite_id: "vn.character.kiki.surprised",
                character: Some("kiki"),
                expression: Some("surprised"),
                is_default_background: false,
            },
        ]),
        "school_background" => Some(&[
            ExpectedAsset {
                source_file: "school-classroom.png",
                output_file: "backgrounds/school-classroom.png",
                sprite_id: "vn.background.school_classroom",
                character: None,
                expression: None,
                is_default_background: true,
            },
            ExpectedAsset {
                source_file: "school-hallway.png",
                output_file: "backgrounds/school-hallway.png",
                sprite_id: "vn.background.school_hallway",
                character: None,
                expression: None,
                is_default_background: false,
            },
        ]),
        "character_sprite_parts" => Some(&[]),
        _ => None,
    }
}

fn copy_asset(
    source_path: &Path,
    output_path: &Path,
    force: bool,
) -> Result<(), VnAssetIntakeError> {
    if output_path.exists() && !force {
        return Err(VnAssetIntakeError::File {
            path: output_path.display().to_string(),
            message: "output already exists; pass --force to overwrite".to_string(),
        });
    }
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| VnAssetIntakeError::File {
            path: parent.display().to_string(),
            message: err.to_string(),
        })?;
    }
    std::fs::copy(source_path, output_path).map_err(|err| VnAssetIntakeError::File {
        path: output_path.display().to_string(),
        message: err.to_string(),
    })?;
    Ok(())
}

fn manifest_entry_path(output_path: &Path, sprite_manifest_path: Option<&Path>) -> String {
    let Some(sprite_manifest_path) = sprite_manifest_path else {
        return output_path.display().to_string();
    };
    let manifest_dir = sprite_manifest_path
        .parent()
        .unwrap_or_else(|| Path::new(""));
    output_path
        .strip_prefix(manifest_dir)
        .unwrap_or(output_path)
        .display()
        .to_string()
}

fn is_supported_image_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "webp"
            )
        })
        .unwrap_or(false)
}

fn attribution_source(
    source: VnAssetCatalogSource,
    used_assets: Vec<VnAssetUsedAsset>,
) -> VnAssetAttributionSource {
    VnAssetAttributionSource {
        id: source.id,
        title: source.title,
        author: source.author,
        source_url: source.source_url,
        license: source.license,
        license_url: source.license_url,
        repo_policy: source.repo_policy,
        attribution: source.attribution,
        used_assets,
    }
}

fn apply_binding(bindings: &mut VnAssetBindings, expected: &ExpectedAsset) {
    if expected.is_default_background {
        bindings.default_background = Some(expected.sprite_id.to_string());
    }
    if let (Some(character), Some(expression)) = (expected.character, expected.expression) {
        bindings
            .characters
            .entry(character.to_string())
            .or_default()
            .expressions
            .insert(expression.to_string(), expected.sprite_id.to_string());
    }
}

fn warning(
    source_id: Option<String>,
    kind: VnAssetIntakeWarningKind,
    detail: impl Into<String>,
) -> VnAssetIntakeWarning {
    VnAssetIntakeWarning {
        source_id,
        kind,
        detail: detail.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_file(path: &Path, contents: &[u8]) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let mut file = std::fs::File::create(path).unwrap();
        file.write_all(contents).unwrap();
    }

    fn write_catalog(path: &Path) {
        write_file(
            path,
            br#"{
  "asset_catalog_version": 1,
  "purpose": "test",
  "policy": {
    "default": "test",
    "preferred_repo_assets": ["cc0"],
    "local_only_when": []
  },
  "sources": [
    {
      "id": "parts",
      "role": "character_sprite_parts",
      "title": "Parts",
      "author": "GameTerm",
      "source_url": "https://example.test/parts",
      "download_name": "parts.zip",
      "license": "CC0-1.0",
      "license_url": "https://example.test/license",
      "source_disclosure": "Source page disclosure recorded",
      "repo_policy": "allowed_with_provenance"
    },
    {
      "id": "character",
      "role": "character_sprite",
      "title": "Character",
      "author": "GameTerm",
      "source_url": "https://example.test/character",
      "download_name": "character.zip",
      "license": "CC-BY-4.0",
      "license_url": "https://example.test/license",
      "source_disclosure": "Source page disclosure recorded",
      "repo_policy": "allowed_with_attribution",
      "attribution": "GameTerm test"
    },
    {
      "id": "background",
      "role": "school_background",
      "title": "Background",
      "author": "GameTerm",
      "source_url": "https://example.test/background",
      "download_name": "background.zip",
      "license": "free",
      "license_url": "https://example.test/license",
      "source_disclosure": "Source page disclosure recorded",
      "repo_policy": "local_only"
    }
  ]
}"#,
        );
    }

    #[test]
    fn vn_asset_intake_copies_allowed_assets_and_generates_bindings() {
        let tmp = tempfile::tempdir().unwrap();
        let catalog_path = tmp.path().join("catalog.json");
        let source_root = tmp.path().join("source");
        let output_root = tmp.path().join("scene/assets/vn-demo");
        let sprite_manifest_path = tmp.path().join("scene/sprites.json");
        write_catalog(&catalog_path);
        write_file(
            &source_root.join("character/kiki-neutral.png"),
            b"gameterm-test-neutral",
        );
        write_file(
            &source_root.join("character/kiki-happy.png"),
            b"gameterm-test-happy",
        );
        for frame in 0..6 {
            write_file(
                &source_root.join(format!("character/kiki-breath-{frame}.png")),
                format!("gameterm-test-breath-{frame}").as_bytes(),
            );
            write_file(
                &source_root.join(format!("character/kiki-blink-{frame}.png")),
                format!("gameterm-test-blink-{frame}").as_bytes(),
            );
        }

        let report = run_vn_asset_intake(VnAssetIntakeOptions {
            catalog_path,
            source_root,
            output_root: output_root.clone(),
            sprite_manifest_path: Some(sprite_manifest_path),
            base_manifest_path: None,
            force: false,
        })
        .unwrap();

        assert!(output_root.join("characters/kiki-neutral.png").is_file());
        assert!(report.sprite_manifest.sprites.iter().any(|sprite| {
            sprite.id == "vn.character.kiki.neutral"
                && sprite.path == "assets/vn-demo/characters/kiki-neutral.png"
        }));
        assert!(report.sprite_manifest.sprites.iter().any(|sprite| {
            sprite.id == "vn.character.kiki.breath.5"
                && sprite.path == "assets/vn-demo/characters/kiki-breath-5.png"
        }));
        assert!(report.sprite_manifest.sprites.iter().any(|sprite| {
            sprite.id == "vn.character.kiki.blink.5"
                && sprite.path == "assets/vn-demo/characters/kiki-blink-5.png"
        }));
        assert_eq!(
            report
                .bindings
                .characters
                .get("kiki")
                .unwrap()
                .expressions
                .get("happy")
                .map(String::as_str),
            Some("vn.character.kiki.happy")
        );
        assert_eq!(
            report
                .bindings
                .characters
                .get("kiki")
                .unwrap()
                .expressions
                .get("breath.3")
                .map(String::as_str),
            Some("vn.character.kiki.breath.3")
        );
        assert_eq!(
            report
                .bindings
                .characters
                .get("kiki")
                .unwrap()
                .expressions
                .get("blink.2")
                .map(String::as_str),
            Some("vn.character.kiki.blink.2")
        );
        assert!(report.warnings.iter().any(|warning| {
            warning.kind == VnAssetIntakeWarningKind::CompositionRequired
                && warning.source_id.as_deref() == Some("parts")
        }));
        assert!(report
            .attribution
            .sources
            .iter()
            .any(|source| { source.id == "character" && source.used_assets.len() == 14 }));
    }

    #[test]
    fn vn_asset_intake_includes_school_backgrounds_by_default() {
        let tmp = tempfile::tempdir().unwrap();
        let catalog_path = tmp.path().join("catalog.json");
        let source_root = tmp.path().join("source");
        let output_root = tmp.path().join("assets");
        write_catalog(&catalog_path);
        write_file(
            &source_root.join("background/school-classroom.png"),
            b"gameterm-test-classroom",
        );

        let report = run_vn_asset_intake(VnAssetIntakeOptions {
            catalog_path,
            source_root,
            output_root,
            sprite_manifest_path: None,
            base_manifest_path: None,
            force: false,
        })
        .unwrap();

        assert_eq!(
            report.bindings.default_background.as_deref(),
            Some("vn.background.school_classroom")
        );
        assert!(report
            .sprite_manifest
            .sprites
            .iter()
            .any(|sprite| sprite.id == "vn.background.school_classroom"));
    }
}
