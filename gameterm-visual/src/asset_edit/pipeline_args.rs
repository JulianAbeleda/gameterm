use super::mask::validate_polygon;
use super::pixels::parse_rgba;
use super::roots::{resolve_asset_prefixed_path, resolve_pipeline_input_path};
use super::{
    load_scene_asset_feature_map, SceneAssetBackgroundSample, SceneAssetColorChannel,
    SceneAssetDefringeMode, SceneAssetDrawShapeKind, SceneAssetEditError, SceneAssetFeatureMap,
    SceneAssetMaskPolishOptions, SceneAssetMaskPreviewMode, SceneAssetNormalizedPoint,
    SceneAssetNormalizedRect, SceneAssetPadAnchor, SceneAssetPipelineRoots,
    SceneAssetResampleFilter,
};
use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::path::{Path, PathBuf};

pub(crate) fn pipeline_command_advances_source(command: &str) -> bool {
    !matches!(command, "sample" | "mask-preview" | "mask-export")
}

pub(crate) fn validate_mask_polish_regions(
    feature_map: Option<&SceneAssetFeatureMap>,
    polish: &SceneAssetMaskPolishOptions,
) -> Result<(), SceneAssetEditError> {
    validate_pipeline_region_names(feature_map, &polish.within_regions)?;
    validate_pipeline_region_names(feature_map, &polish.protect_regions)
}

pub(crate) fn validate_adjustment_regions(
    args: &BTreeMap<String, serde_json::Value>,
    feature_map: Option<&SceneAssetFeatureMap>,
) -> Result<(), SceneAssetEditError> {
    let within_regions = pipeline_string_list_arg(args, "within_regions")?;
    pipeline_polygons_arg(args, "within_polygons")?;
    let protect_regions = pipeline_string_list_arg(args, "protect_regions")?;
    validate_pipeline_region_names(feature_map, &within_regions)?;
    validate_pipeline_region_names(feature_map, &protect_regions)
}

pub(crate) fn validate_pipeline_region_names(
    feature_map: Option<&SceneAssetFeatureMap>,
    regions: &[String],
) -> Result<(), SceneAssetEditError> {
    if regions.is_empty() {
        return Ok(());
    }
    let Some(feature_map) = feature_map else {
        return Err(SceneAssetEditError::InvalidOperation(
            "feature map is required when using named regions".to_string(),
        ));
    };
    for region in regions {
        if !feature_map.regions.contains_key(region) {
            return Err(SceneAssetEditError::UnknownRegion(region.clone()));
        }
    }
    Ok(())
}

pub(crate) fn load_pipeline_feature_map(
    roots: &SceneAssetPipelineRoots,
    args: &BTreeMap<String, serde_json::Value>,
) -> Result<Option<SceneAssetFeatureMap>, SceneAssetEditError> {
    let path = pipeline_string_arg(args, "protect")?
        .or(pipeline_string_arg(args, "feature_map")?)
        .map(|path| resolve_pipeline_input_path(roots, &path));
    path.map(|path| load_scene_asset_feature_map(&path))
        .transpose()
}

pub(crate) fn pipeline_arg<'a>(
    args: &'a BTreeMap<String, serde_json::Value>,
    key: &str,
) -> Option<&'a serde_json::Value> {
    args.get(key).or_else(|| {
        let kebab = key.replace('_', "-");
        args.get(&kebab)
    })
}

pub(crate) fn pipeline_string_arg(
    args: &BTreeMap<String, serde_json::Value>,
    key: &str,
) -> Result<Option<String>, SceneAssetEditError> {
    let Some(value) = pipeline_arg(args, key) else {
        return Ok(None);
    };
    value
        .as_str()
        .map(|value| Some(value.to_string()))
        .ok_or_else(|| {
            SceneAssetEditError::InvalidOperation(format!("pipeline arg `{key}` must be a string"))
        })
}

pub(crate) fn pipeline_required_asset_path_arg(
    roots: &SceneAssetPipelineRoots,
    args: &BTreeMap<String, serde_json::Value>,
    key: &str,
) -> Result<PathBuf, SceneAssetEditError> {
    let path = pipeline_string_arg(args, key)?.ok_or_else(|| {
        SceneAssetEditError::InvalidOperation(format!("pipeline arg `{key}` is required"))
    })?;
    Ok(resolve_asset_prefixed_path(
        roots,
        Path::new(&path),
        &roots.transformation_root,
    ))
}

pub(crate) fn pipeline_string_list_arg(
    args: &BTreeMap<String, serde_json::Value>,
    key: &str,
) -> Result<Vec<String>, SceneAssetEditError> {
    let Some(value) = pipeline_arg(args, key) else {
        return Ok(Vec::new());
    };
    match value {
        serde_json::Value::String(text) => Ok(text
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
            .collect()),
        serde_json::Value::Array(values) => values
            .iter()
            .map(|value| {
                value.as_str().map(ToString::to_string).ok_or_else(|| {
                    SceneAssetEditError::InvalidOperation(format!(
                        "pipeline arg `{key}` entries must be strings"
                    ))
                })
            })
            .collect(),
        _ => Err(SceneAssetEditError::InvalidOperation(format!(
            "pipeline arg `{key}` must be a string or string array"
        ))),
    }
}

pub(crate) fn pipeline_u8_arg(
    args: &BTreeMap<String, serde_json::Value>,
    key: &str,
    default: u8,
) -> Result<u8, SceneAssetEditError> {
    let Some(value) = pipeline_arg(args, key) else {
        return Ok(default);
    };
    let number = value.as_u64().ok_or_else(|| {
        SceneAssetEditError::InvalidOperation(format!("pipeline arg `{key}` must be a number"))
    })?;
    u8::try_from(number).map_err(|_| {
        SceneAssetEditError::InvalidOperation(format!("pipeline arg `{key}` must fit in u8"))
    })
}

pub(crate) fn pipeline_u32_arg(
    args: &BTreeMap<String, serde_json::Value>,
    key: &str,
    default: u32,
) -> Result<u32, SceneAssetEditError> {
    let Some(value) = pipeline_arg(args, key) else {
        return Ok(default);
    };
    let number = value.as_u64().ok_or_else(|| {
        SceneAssetEditError::InvalidOperation(format!("pipeline arg `{key}` must be a number"))
    })?;
    u32::try_from(number).map_err(|_| {
        SceneAssetEditError::InvalidOperation(format!("pipeline arg `{key}` must fit in u32"))
    })
}

pub(crate) fn pipeline_u32_required_arg(
    args: &BTreeMap<String, serde_json::Value>,
    key: &str,
) -> Result<u32, SceneAssetEditError> {
    if pipeline_arg(args, key).is_none() {
        return Err(SceneAssetEditError::InvalidOperation(format!(
            "pipeline arg `{key}` is required"
        )));
    }
    pipeline_u32_arg(args, key, 0)
}

pub(crate) fn pipeline_usize_arg(
    args: &BTreeMap<String, serde_json::Value>,
    key: &str,
    default: usize,
) -> Result<usize, SceneAssetEditError> {
    let Some(value) = pipeline_arg(args, key) else {
        return Ok(default);
    };
    let number = value.as_u64().ok_or_else(|| {
        SceneAssetEditError::InvalidOperation(format!("pipeline arg `{key}` must be a number"))
    })?;
    usize::try_from(number).map_err(|_| {
        SceneAssetEditError::InvalidOperation(format!("pipeline arg `{key}` must fit in usize"))
    })
}

pub(crate) fn pipeline_f32_arg(
    args: &BTreeMap<String, serde_json::Value>,
    key: &str,
    default: f32,
) -> Result<f32, SceneAssetEditError> {
    let Some(value) = pipeline_arg(args, key) else {
        return Ok(default);
    };
    let number = value.as_f64().ok_or_else(|| {
        SceneAssetEditError::InvalidOperation(format!("pipeline arg `{key}` must be a number"))
    })?;
    Ok(number as f32)
}

pub(crate) fn pipeline_bool_arg(
    args: &BTreeMap<String, serde_json::Value>,
    key: &str,
    default: bool,
) -> Result<bool, SceneAssetEditError> {
    let Some(value) = pipeline_arg(args, key) else {
        return Ok(default);
    };
    value.as_bool().ok_or_else(|| {
        SceneAssetEditError::InvalidOperation(format!("pipeline arg `{key}` must be a boolean"))
    })
}

pub(crate) fn pipeline_color_arg(
    args: &BTreeMap<String, serde_json::Value>,
    key: &str,
) -> Result<[u8; 4], SceneAssetEditError> {
    let Some(color) = pipeline_string_arg(args, key)? else {
        return Err(SceneAssetEditError::InvalidOperation(format!(
            "pipeline arg `{key}` is required"
        )));
    };
    Ok(parse_rgba(&color)?.0)
}

pub(crate) fn pipeline_optional_color_arg(
    args: &BTreeMap<String, serde_json::Value>,
    key: &str,
    default: [u8; 4],
) -> Result<[u8; 4], SceneAssetEditError> {
    let Some(color) = pipeline_string_arg(args, key)? else {
        return Ok(default);
    };
    Ok(parse_rgba(&color)?.0)
}

pub(crate) fn pipeline_background_sample_arg(
    args: &BTreeMap<String, serde_json::Value>,
    key: &str,
) -> Result<SceneAssetBackgroundSample, SceneAssetEditError> {
    match pipeline_string_arg(args, key)?
        .as_deref()
        .unwrap_or("corners")
    {
        "corners" => Ok(SceneAssetBackgroundSample::Corners),
        "edges" => Ok(SceneAssetBackgroundSample::Edges),
        value => Err(SceneAssetEditError::InvalidOperation(format!(
            "pipeline arg `{key}` value `{value}` is invalid; expected corners or edges"
        ))),
    }
}

pub(crate) fn pipeline_pad_anchor_arg(
    args: &BTreeMap<String, serde_json::Value>,
) -> Result<SceneAssetPadAnchor, SceneAssetEditError> {
    match pipeline_string_arg(args, "anchor")?
        .as_deref()
        .unwrap_or("center")
    {
        "center" => Ok(SceneAssetPadAnchor::Center),
        "bottom-center" | "bottom_center" => Ok(SceneAssetPadAnchor::BottomCenter),
        "top-left" | "top_left" => Ok(SceneAssetPadAnchor::TopLeft),
        value => Err(SceneAssetEditError::InvalidOperation(format!(
            "pipeline arg `anchor` value `{value}` is invalid"
        ))),
    }
}

pub(crate) fn pipeline_resample_arg(
    args: &BTreeMap<String, serde_json::Value>,
) -> Result<SceneAssetResampleFilter, SceneAssetEditError> {
    match pipeline_string_arg(args, "resample")?
        .as_deref()
        .unwrap_or("lanczos3")
    {
        "nearest" => Ok(SceneAssetResampleFilter::Nearest),
        "lanczos3" => Ok(SceneAssetResampleFilter::Lanczos3),
        value => Err(SceneAssetEditError::InvalidOperation(format!(
            "pipeline arg `resample` value `{value}` is invalid"
        ))),
    }
}

pub(crate) fn pipeline_channel_arg(
    args: &BTreeMap<String, serde_json::Value>,
) -> Result<SceneAssetColorChannel, SceneAssetEditError> {
    match pipeline_string_arg(args, "channel")?
        .as_deref()
        .unwrap_or("rgb")
    {
        "rgb" => Ok(SceneAssetColorChannel::Rgb),
        "r" => Ok(SceneAssetColorChannel::R),
        "g" => Ok(SceneAssetColorChannel::G),
        "b" => Ok(SceneAssetColorChannel::B),
        "a" => Ok(SceneAssetColorChannel::A),
        value => Err(SceneAssetEditError::InvalidOperation(format!(
            "pipeline arg `channel` value `{value}` is invalid"
        ))),
    }
}

pub(crate) fn pipeline_translate_arg(
    args: &BTreeMap<String, serde_json::Value>,
) -> Result<(i32, i32), SceneAssetEditError> {
    let Some(value) = pipeline_arg(args, "translate") else {
        return Ok((0, 0));
    };
    match value {
        serde_json::Value::String(text) => parse_i32_pair_text(text, "translate"),
        serde_json::Value::Array(values) if values.len() == 2 => {
            let x = values[0].as_i64().ok_or_else(|| {
                SceneAssetEditError::InvalidOperation(
                    "pipeline translate x must be a number".to_string(),
                )
            })?;
            let y = values[1].as_i64().ok_or_else(|| {
                SceneAssetEditError::InvalidOperation(
                    "pipeline translate y must be a number".to_string(),
                )
            })?;
            Ok((x as i32, y as i32))
        }
        _ => Err(SceneAssetEditError::InvalidOperation(
            "pipeline translate must be `X,Y` or [X,Y]".to_string(),
        )),
    }
}

pub(crate) fn pipeline_draw_shape_arg(
    args: &BTreeMap<String, serde_json::Value>,
) -> Result<SceneAssetDrawShapeKind, SceneAssetEditError> {
    match pipeline_string_arg(args, "shape")?
        .as_deref()
        .unwrap_or("line")
    {
        "rect" => Ok(SceneAssetDrawShapeKind::Rect),
        "line" => Ok(SceneAssetDrawShapeKind::Line),
        "polygon" => Ok(SceneAssetDrawShapeKind::Polygon),
        "ellipse" | "circle" => Ok(SceneAssetDrawShapeKind::Ellipse),
        value => Err(SceneAssetEditError::InvalidOperation(format!(
            "pipeline arg `shape` value `{value}` is invalid"
        ))),
    }
}

pub(crate) fn pipeline_rect_arg(
    args: &BTreeMap<String, serde_json::Value>,
    key: &str,
) -> Result<Option<SceneAssetNormalizedRect>, SceneAssetEditError> {
    let Some(value) = pipeline_arg(args, key) else {
        return Ok(None);
    };
    match value {
        serde_json::Value::String(text) => parse_normalized_rect_text(text, key).map(Some),
        serde_json::Value::Object(object) => {
            let x = object.get("x").and_then(serde_json::Value::as_f64);
            let y = object.get("y").and_then(serde_json::Value::as_f64);
            let w = object.get("w").and_then(serde_json::Value::as_f64);
            let h = object.get("h").and_then(serde_json::Value::as_f64);
            match (x, y, w, h) {
                (Some(x), Some(y), Some(w), Some(h)) => Ok(Some(SceneAssetNormalizedRect {
                    x: x as f32,
                    y: y as f32,
                    w: w as f32,
                    h: h as f32,
                })),
                _ => Err(SceneAssetEditError::InvalidOperation(format!(
                    "pipeline rect `{key}` must include numeric x, y, w, and h"
                ))),
            }
        }
        _ => Err(SceneAssetEditError::InvalidOperation(format!(
            "pipeline rect `{key}` must be `X,Y,W,H` or an object"
        ))),
    }
}

pub(crate) fn pipeline_mask_preview_mode_arg(
    args: &BTreeMap<String, serde_json::Value>,
) -> Result<SceneAssetMaskPreviewMode, SceneAssetEditError> {
    match pipeline_string_arg(args, "selection_mode")?
        .as_deref()
        .unwrap_or("background")
    {
        "background" => Ok(SceneAssetMaskPreviewMode::Background),
        "color-range" | "color_range" => Ok(SceneAssetMaskPreviewMode::ColorRange),
        "magic-add" | "magic_add" => Ok(SceneAssetMaskPreviewMode::MagicAdd),
        "channel-matte" | "channel_matte" => Ok(SceneAssetMaskPreviewMode::ChannelMatte),
        value => Err(SceneAssetEditError::InvalidOperation(format!(
            "pipeline arg `selection_mode` value `{value}` is invalid"
        ))),
    }
}

pub(crate) fn pipeline_mask_polish_options(
    args: &BTreeMap<String, serde_json::Value>,
) -> Result<SceneAssetMaskPolishOptions, SceneAssetEditError> {
    Ok(SceneAssetMaskPolishOptions {
        tolerance: pipeline_u8_arg(args, "tolerance", 24)?,
        feather: pipeline_u32_arg(args, "feather", 0)?,
        sample: pipeline_background_sample_arg(args, "sample")?,
        erode: pipeline_u32_arg(args, "erode", 0)?,
        dilate: pipeline_u32_arg(args, "dilate", 0)?,
        open: pipeline_u32_arg(args, "open", 0)?,
        close: pipeline_u32_arg(args, "close", 0)?,
        remove_small: pipeline_usize_arg(args, "remove_small", 0)?,
        fill_holes: pipeline_usize_arg(args, "fill_holes", 0)?,
        defringe: match pipeline_string_arg(args, "defringe")?
            .as_deref()
            .unwrap_or("none")
        {
            "none" => SceneAssetDefringeMode::None,
            "white" => SceneAssetDefringeMode::White,
            value => {
                return Err(SceneAssetEditError::InvalidOperation(format!(
                    "pipeline arg `defringe` value `{value}` is invalid"
                )))
            }
        },
        protect_regions: pipeline_string_list_arg(args, "protect_regions")?,
        within_regions: pipeline_string_list_arg(args, "within_regions")?,
        within_polygons: pipeline_polygons_arg(args, "within_polygons")?,
    })
}

pub(crate) fn pipeline_points_arg(
    args: &BTreeMap<String, serde_json::Value>,
    plural_key: &str,
    singular_key: &str,
) -> Result<Vec<SceneAssetNormalizedPoint>, SceneAssetEditError> {
    let mut points = Vec::new();
    if let Some(value) = pipeline_arg(args, plural_key) {
        match value {
            serde_json::Value::Array(values) => {
                for value in values {
                    points.push(pipeline_point_value(value, plural_key)?);
                }
            }
            _ => points.push(pipeline_point_value(value, plural_key)?),
        }
    }
    if let Some(value) = pipeline_arg(args, singular_key) {
        points.push(pipeline_point_value(value, singular_key)?);
    }
    Ok(points)
}

pub(crate) fn pipeline_required_point_arg(
    args: &BTreeMap<String, serde_json::Value>,
    key: &str,
) -> Result<SceneAssetNormalizedPoint, SceneAssetEditError> {
    let Some(value) = pipeline_arg(args, key) else {
        return Err(SceneAssetEditError::InvalidOperation(format!(
            "pipeline arg `{key}` is required"
        )));
    };
    pipeline_point_value(value, key)
}

fn pipeline_point_value(
    value: &serde_json::Value,
    key: &str,
) -> Result<SceneAssetNormalizedPoint, SceneAssetEditError> {
    match value {
        serde_json::Value::String(text) => parse_normalized_point_text(text, key),
        serde_json::Value::Object(object) => {
            let x = object.get("x").and_then(serde_json::Value::as_f64);
            let y = object.get("y").and_then(serde_json::Value::as_f64);
            match (x, y) {
                (Some(x), Some(y)) => Ok(SceneAssetNormalizedPoint {
                    x: x as f32,
                    y: y as f32,
                }),
                _ => Err(SceneAssetEditError::InvalidOperation(format!(
                    "pipeline point `{key}` must include numeric x and y"
                ))),
            }
        }
        _ => Err(SceneAssetEditError::InvalidOperation(format!(
            "pipeline point `{key}` must be `X,Y` or {{\"x\":N,\"y\":N}}"
        ))),
    }
}

pub(crate) fn pipeline_polygons_arg(
    args: &BTreeMap<String, serde_json::Value>,
    key: &str,
) -> Result<Vec<Vec<SceneAssetNormalizedPoint>>, SceneAssetEditError> {
    let mut polygons = Vec::new();
    if let Some(value) = pipeline_arg(args, key) {
        match value {
            serde_json::Value::Array(values) => {
                for value in values {
                    polygons.push(pipeline_polygon_value(value, key)?);
                }
            }
            _ => polygons.push(pipeline_polygon_value(value, key)?),
        }
    }
    if let Some(value) = pipeline_arg(args, "within_polygon") {
        polygons.push(pipeline_polygon_value(value, "within_polygon")?);
    }
    Ok(polygons)
}

fn pipeline_polygon_value(
    value: &serde_json::Value,
    key: &str,
) -> Result<Vec<SceneAssetNormalizedPoint>, SceneAssetEditError> {
    match value {
        serde_json::Value::String(text) => parse_normalized_polygon_text(text, key),
        serde_json::Value::Array(values) => values
            .iter()
            .map(|value| pipeline_point_value(value, key))
            .collect(),
        _ => Err(SceneAssetEditError::InvalidOperation(format!(
            "pipeline polygon `{key}` must be a string or point array"
        ))),
    }
}

pub(crate) fn pipeline_path_points_arg(
    args: &BTreeMap<String, serde_json::Value>,
    key: &str,
) -> Result<Vec<SceneAssetNormalizedPoint>, SceneAssetEditError> {
    let Some(value) = pipeline_arg(args, key) else {
        return Err(SceneAssetEditError::InvalidOperation(format!(
            "pipeline arg `{key}` is required"
        )));
    };
    let points = match value {
        serde_json::Value::String(text) => text
            .split(';')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|point| parse_normalized_point_text(point, key))
            .collect::<Result<Vec<_>, _>>()?,
        serde_json::Value::Array(values) => values
            .iter()
            .map(|value| pipeline_point_value(value, key))
            .collect::<Result<Vec<_>, _>>()?,
        _ => {
            return Err(SceneAssetEditError::InvalidOperation(format!(
                "pipeline path `{key}` must be a string or point array"
            )))
        }
    };
    if points.len() < 2 {
        return Err(SceneAssetEditError::InvalidOperation(format!(
            "pipeline path `{key}` requires at least two points"
        )));
    }
    Ok(points)
}

fn parse_normalized_polygon_text(
    value: &str,
    key: &str,
) -> Result<Vec<SceneAssetNormalizedPoint>, SceneAssetEditError> {
    let points = value
        .split(';')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|point| parse_normalized_point_text(point, key))
        .collect::<Result<Vec<_>, _>>()?;
    validate_polygon(&points)?;
    Ok(points)
}

fn parse_normalized_rect_text(
    value: &str,
    key: &str,
) -> Result<SceneAssetNormalizedRect, SceneAssetEditError> {
    let parts = value.split(',').map(str::trim).collect::<Vec<_>>();
    if parts.len() != 4 {
        return Err(SceneAssetEditError::InvalidOperation(format!(
            "pipeline rect `{key}` value `{value}` is invalid; expected X,Y,W,H"
        )));
    }
    let parse = |index: usize| {
        parts[index].parse::<f32>().map_err(|err| {
            SceneAssetEditError::InvalidOperation(format!(
                "pipeline rect `{key}` value `{}` is invalid: {err}",
                parts[index]
            ))
        })
    };
    Ok(SceneAssetNormalizedRect {
        x: parse(0)?,
        y: parse(1)?,
        w: parse(2)?,
        h: parse(3)?,
    })
}

fn parse_i32_pair_text(value: &str, key: &str) -> Result<(i32, i32), SceneAssetEditError> {
    let (x, y) = value.split_once(',').ok_or_else(|| {
        SceneAssetEditError::InvalidOperation(format!(
            "pipeline `{key}` value `{value}` is invalid; expected X,Y"
        ))
    })?;
    let x = x.trim().parse::<i32>().map_err(|err| {
        SceneAssetEditError::InvalidOperation(format!(
            "pipeline `{key}` x value `{x}` is invalid: {err}"
        ))
    })?;
    let y = y.trim().parse::<i32>().map_err(|err| {
        SceneAssetEditError::InvalidOperation(format!(
            "pipeline `{key}` y value `{y}` is invalid: {err}"
        ))
    })?;
    Ok((x, y))
}

fn parse_normalized_point_text(
    value: &str,
    key: &str,
) -> Result<SceneAssetNormalizedPoint, SceneAssetEditError> {
    let (x, y) = value.split_once(',').ok_or_else(|| {
        SceneAssetEditError::InvalidOperation(format!(
            "pipeline point `{key}` value `{value}` is invalid; expected X,Y"
        ))
    })?;
    let x = x.trim().parse::<f32>().map_err(|err| {
        SceneAssetEditError::InvalidOperation(format!(
            "pipeline point `{key}` x value `{x}` is invalid: {err}"
        ))
    })?;
    let y = y.trim().parse::<f32>().map_err(|err| {
        SceneAssetEditError::InvalidOperation(format!(
            "pipeline point `{key}` y value `{y}` is invalid: {err}"
        ))
    })?;
    Ok(SceneAssetNormalizedPoint { x, y })
}
