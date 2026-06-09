use super::io::{load_json, load_rgba_image, save_rgba_image, write_json};
use super::pixels::{paste_layer, paste_region};
use super::roots::resolve_recipe_path;
use super::{
    inspect_scene_asset_image, SceneAssetBlendMode, SceneAssetCompositeLayer,
    SceneAssetCompositeOptions, SceneAssetCompositeReport, SceneAssetEditError,
    SceneAssetStateManifest, SceneAssetStateManifestOptions, SceneAssetStatePart,
    SceneAssetStateRenderOptions, SceneAssetStateSheetFrame, SceneAssetStateSheetIndex,
    SceneAssetStateSheetIndexFrame,
};
use image::{ImageBuffer, Rgba, RgbaImage};
use std::collections::BTreeMap;
use std::path::Path;

pub fn composite_scene_asset_layers(
    output_path: &Path,
    options: SceneAssetCompositeOptions,
    base_dir: Option<&Path>,
    force: bool,
) -> Result<SceneAssetCompositeReport, SceneAssetEditError> {
    if options.layers.is_empty() {
        return Err(SceneAssetEditError::InvalidOperation(
            "composite requires at least one layer".to_string(),
        ));
    }
    let mut loaded = Vec::with_capacity(options.layers.len());
    for layer in &options.layers {
        if !layer.opacity.is_finite() {
            return Err(SceneAssetEditError::InvalidOperation(
                "layer opacity must be finite".to_string(),
            ));
        }
        let path = resolve_recipe_path(&layer.path, base_dir);
        loaded.push((layer, load_rgba_image(&path)?));
    }
    let width = options.width.unwrap_or_else(|| loaded[0].1.width());
    let height = options.height.unwrap_or_else(|| loaded[0].1.height());
    let mut output = ImageBuffer::from_pixel(width, height, Rgba([0, 0, 0, 0]));
    for (layer, image) in loaded {
        paste_layer(
            &mut output,
            &image,
            layer.x_offset,
            layer.y_offset,
            layer.opacity,
            layer.blend,
        );
    }
    save_rgba_image(&output, output_path, force)?;
    Ok(SceneAssetCompositeReport {
        operation: "composite".to_string(),
        output_path: output_path.display().to_string(),
        layer_count: options.layers.len(),
        report: inspect_scene_asset_image(output_path)?,
    })
}

pub fn create_scene_asset_state_manifest(
    base_path: &Path,
    output_path: &Path,
    options: SceneAssetStateManifestOptions,
    force: bool,
) -> Result<SceneAssetStateManifest, SceneAssetEditError> {
    inspect_scene_asset_image(base_path)?;
    let mut parts = BTreeMap::new();
    for (part_name, files) in options.parts {
        if files.is_empty() {
            return Err(SceneAssetEditError::InvalidOperation(format!(
                "state part `{part_name}` requires at least one state file"
            )));
        }
        let mut states = BTreeMap::new();
        for file in files {
            let state_name = Path::new(&file)
                .file_stem()
                .map(|stem| stem.to_string_lossy().to_string())
                .unwrap_or_else(|| file.clone());
            states.insert(state_name, file);
        }
        let default = states.keys().next().cloned().unwrap_or_default();
        parts.insert(part_name, SceneAssetStatePart { default, states });
    }
    let manifest = SceneAssetStateManifest {
        asset_state_version: 1,
        character: options.character,
        base: base_path.display().to_string(),
        parts,
    };
    write_json(output_path, &manifest, true, force)?;
    Ok(manifest)
}

pub fn load_scene_asset_state_manifest(
    path: &Path,
) -> Result<SceneAssetStateManifest, SceneAssetEditError> {
    load_json(path)
}

pub fn render_scene_asset_state(
    manifest_path: &Path,
    output_path: &Path,
    options: SceneAssetStateRenderOptions,
    force: bool,
) -> Result<SceneAssetCompositeReport, SceneAssetEditError> {
    let manifest = load_scene_asset_state_manifest(manifest_path)?;
    let base_dir = manifest_path.parent();
    let composite = state_composite_options(&manifest, &options.states)?;
    composite_scene_asset_layers(output_path, composite, base_dir, force)
}

pub fn render_scene_asset_state_sheet(
    manifest_path: &Path,
    frames_path: &Path,
    output_path: &Path,
    index_path: &Path,
    force: bool,
) -> Result<SceneAssetCompositeReport, SceneAssetEditError> {
    let manifest = load_scene_asset_state_manifest(manifest_path)?;
    let frames: Vec<SceneAssetStateSheetFrame> = load_json(frames_path)?;
    if frames.is_empty() {
        return Err(SceneAssetEditError::InvalidOperation(
            "state-sheet requires at least one frame".to_string(),
        ));
    }
    let base_dir = manifest_path.parent();
    let base_path = resolve_recipe_path(&manifest.base, base_dir);
    let base = load_rgba_image(&base_path)?;
    let mut sheet = ImageBuffer::from_pixel(
        base.width() * frames.len() as u32,
        base.height(),
        Rgba([0, 0, 0, 0]),
    );
    let mut index_frames = Vec::new();
    for (index, frame) in frames.iter().enumerate() {
        let composite = state_composite_options(&manifest, &frame.states)?;
        let rendered = render_composite_to_image(composite, base_dir)?;
        let x = base.width() * index as u32;
        paste_region(&mut sheet, &rendered, x as i32, 0, 1.0);
        index_frames.push(SceneAssetStateSheetIndexFrame {
            index,
            label: frame.label.clone(),
            x,
            y: 0,
            w: base.width(),
            h: base.height(),
            states: frame.states.clone(),
        });
    }
    save_rgba_image(&sheet, output_path, force)?;
    write_json(
        index_path,
        &SceneAssetStateSheetIndex {
            frames: index_frames,
        },
        true,
        force,
    )?;
    Ok(SceneAssetCompositeReport {
        operation: "state_sheet".to_string(),
        output_path: output_path.display().to_string(),
        layer_count: frames.len(),
        report: inspect_scene_asset_image(output_path)?,
    })
}

fn state_composite_options(
    manifest: &SceneAssetStateManifest,
    selected_states: &BTreeMap<String, String>,
) -> Result<SceneAssetCompositeOptions, SceneAssetEditError> {
    let mut layers = vec![SceneAssetCompositeLayer {
        path: manifest.base.clone(),
        blend: SceneAssetBlendMode::Normal,
        opacity: 1.0,
        x_offset: 0,
        y_offset: 0,
    }];
    for (part_name, part) in &manifest.parts {
        let state = selected_states
            .get(part_name)
            .map(String::as_str)
            .unwrap_or(&part.default);
        let path = part.states.get(state).ok_or_else(|| {
            SceneAssetEditError::InvalidOperation(format!(
                "unknown state `{state}` for part `{part_name}`"
            ))
        })?;
        layers.push(SceneAssetCompositeLayer {
            path: path.clone(),
            blend: SceneAssetBlendMode::Normal,
            opacity: 1.0,
            x_offset: 0,
            y_offset: 0,
        });
    }
    Ok(SceneAssetCompositeOptions {
        width: None,
        height: None,
        layers,
    })
}

fn render_composite_to_image(
    options: SceneAssetCompositeOptions,
    base_dir: Option<&Path>,
) -> Result<RgbaImage, SceneAssetEditError> {
    if options.layers.is_empty() {
        return Err(SceneAssetEditError::InvalidOperation(
            "composite requires at least one layer".to_string(),
        ));
    }
    let mut loaded = Vec::with_capacity(options.layers.len());
    for layer in &options.layers {
        let path = resolve_recipe_path(&layer.path, base_dir);
        loaded.push((layer, load_rgba_image(&path)?));
    }
    let width = options.width.unwrap_or_else(|| loaded[0].1.width());
    let height = options.height.unwrap_or_else(|| loaded[0].1.height());
    let mut output = ImageBuffer::from_pixel(width, height, Rgba([0, 0, 0, 0]));
    for (layer, image) in loaded {
        paste_layer(
            &mut output,
            &image,
            layer.x_offset,
            layer.y_offset,
            layer.opacity,
            layer.blend,
        );
    }
    Ok(output)
}
