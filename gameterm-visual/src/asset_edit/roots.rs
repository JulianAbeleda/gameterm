use super::SceneAssetPipelineRoots;
use std::path::{Path, PathBuf};

pub(crate) fn resolve_pipeline_input_path(roots: &SceneAssetPipelineRoots, path: &str) -> PathBuf {
    let path = PathBuf::from(path);
    if path.is_absolute() {
        path
    } else {
        roots.input_root.join(path)
    }
}

pub(crate) fn resolve_asset_operation_source_path(
    roots: &SceneAssetPipelineRoots,
    path: &str,
) -> PathBuf {
    let path = PathBuf::from(path);
    if path.is_absolute() {
        return path;
    }
    let mut components = path.components();
    if matches!(components.next(), Some(std::path::Component::Normal(prefix)) if prefix == "Transformation")
    {
        return roots.transformation_root.join(components.as_path());
    }
    let mut components = path.components();
    if matches!(components.next(), Some(std::path::Component::Normal(prefix)) if prefix == "Output")
    {
        return roots.output_root.join(components.as_path());
    }
    roots.input_root.join(path)
}

pub(crate) fn resolve_asset_prefixed_path(
    roots: &SceneAssetPipelineRoots,
    path: &Path,
    default_root: &Path,
) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }
    if let Ok(rest) = path.strip_prefix("Input") {
        return roots.input_root.join(rest);
    }
    if let Ok(rest) = path.strip_prefix("Transformation") {
        return roots.transformation_root.join(rest);
    }
    if let Ok(rest) = path.strip_prefix("Output") {
        return roots.output_root.join(rest);
    }
    default_root.join(path)
}

pub(crate) fn resolve_asset_accept_source_path(
    roots: &SceneAssetPipelineRoots,
    path: &Path,
) -> PathBuf {
    resolve_asset_prefixed_path(roots, path, &roots.transformation_root)
}

pub(crate) fn resolve_asset_accept_output_path(
    roots: &SceneAssetPipelineRoots,
    path: &Path,
) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }
    if let Ok(rest) = path.strip_prefix("Output") {
        return roots.output_root.join(rest);
    }
    roots.output_root.join(path)
}

pub(crate) fn resolve_pipeline_output_path(roots: &SceneAssetPipelineRoots, path: &str) -> PathBuf {
    let path = PathBuf::from(path);
    if path.is_absolute() {
        return path;
    }
    let mut components = path.components();
    if matches!(components.next(), Some(std::path::Component::Normal(prefix)) if prefix == "Output")
    {
        return roots.output_root.join(components.as_path());
    }
    roots.transformation_root.join(path)
}

pub(crate) fn pipeline_report_path(output_path: &PathBuf) -> PathBuf {
    let stem = output_path
        .file_stem()
        .map(|stem| stem.to_string_lossy())
        .unwrap_or_else(|| "step".into());
    output_path.with_file_name(format!("{stem}.report.json"))
}

pub(crate) fn resolve_recipe_path(path: &str, base_dir: Option<&Path>) -> PathBuf {
    let path = PathBuf::from(path);
    if path.is_absolute() {
        return path;
    }
    base_dir
        .map(|base_dir| base_dir.join(&path))
        .unwrap_or(path)
}
