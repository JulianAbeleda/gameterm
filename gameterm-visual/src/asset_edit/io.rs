use super::SceneAssetEditError;
use image::{DynamicImage, RgbaImage};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;

pub(crate) fn load_json<T: for<'de> Deserialize<'de>>(
    path: &Path,
) -> Result<T, SceneAssetEditError> {
    let json = std::fs::read_to_string(path).map_err(|err| SceneAssetEditError::JsonFile {
        path: path.display().to_string(),
        message: err.to_string(),
    })?;
    serde_json::from_str(&json).map_err(|err| SceneAssetEditError::JsonParse {
        path: path.display().to_string(),
        message: err.to_string(),
    })
}

pub(crate) fn read_file(path: &Path, kind: &str) -> Result<Vec<u8>, SceneAssetEditError> {
    std::fs::read(path).map_err(|err| SceneAssetEditError::ImageFile {
        path: path.display().to_string(),
        message: format!("{kind}: {err}"),
    })
}

pub(crate) fn write_file(
    path: &Path,
    bytes: &[u8],
    force: bool,
) -> Result<(), SceneAssetEditError> {
    if path.exists() && !force {
        return Err(SceneAssetEditError::OutputExists(
            path.display().to_string(),
        ));
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| SceneAssetEditError::ImageFile {
            path: parent.display().to_string(),
            message: err.to_string(),
        })?;
    }
    std::fs::write(path, bytes).map_err(|err| SceneAssetEditError::ImageFile {
        path: path.display().to_string(),
        message: err.to_string(),
    })
}

pub(crate) fn write_json(
    path: &Path,
    value: &impl Serialize,
    pretty: bool,
    force: bool,
) -> Result<(), SceneAssetEditError> {
    if path.exists() && !force {
        return Err(SceneAssetEditError::OutputExists(
            path.display().to_string(),
        ));
    }
    let json = if pretty {
        serde_json::to_string_pretty(value)
    } else {
        serde_json::to_string(value)
    }
    .map_err(|err| SceneAssetEditError::JsonFile {
        path: path.display().to_string(),
        message: err.to_string(),
    })?;
    write_file(path, format!("{json}\n").as_bytes(), force)
}

pub(crate) fn load_rgba_image(path: &Path) -> Result<RgbaImage, SceneAssetEditError> {
    image::ImageReader::open(path)
        .map_err(|err| SceneAssetEditError::ImageFile {
            path: path.display().to_string(),
            message: err.to_string(),
        })?
        .decode()
        .map(DynamicImage::into_rgba8)
        .map_err(|err| SceneAssetEditError::ImageFile {
            path: path.display().to_string(),
            message: err.to_string(),
        })
}

pub(crate) fn save_rgba_image(
    image: &RgbaImage,
    path: &Path,
    force: bool,
) -> Result<(), SceneAssetEditError> {
    if path.exists() && !force {
        return Err(SceneAssetEditError::OutputExists(
            path.display().to_string(),
        ));
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| SceneAssetEditError::ImageFile {
            path: parent.display().to_string(),
            message: err.to_string(),
        })?;
    }
    image
        .save(path)
        .map_err(|err| SceneAssetEditError::ImageFile {
            path: path.display().to_string(),
            message: err.to_string(),
        })
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
