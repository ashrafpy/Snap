use serde::{Deserialize, Serialize};
use snap_core::stack::EditStack;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::IoError;

/// Sidecar structure storing non-destructive editing history and current stack.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SnapSidecar {
    pub app_version: String,
    pub stack: EditStack,
}

/// Compute the sidecar path for a given image path: e.g. `foo/bar.jpg` -> `foo/bar.jpg.snap`.
pub fn sidecar_path_for(image_path: impl AsRef<Path>) -> PathBuf {
    let path = image_path.as_ref();
    let mut os_str = path.as_os_str().to_os_string();
    os_str.push(".snap");
    PathBuf::from(os_str)
}

/// Save an edit stack to a sidecar file next to the original image.
pub fn save_sidecar(image_path: impl AsRef<Path>, stack: &EditStack) -> Result<PathBuf, IoError> {
    let sidecar_path = sidecar_path_for(image_path);
    let sidecar = SnapSidecar {
        app_version: snap_core::version().to_string(),
        stack: stack.clone(),
    };
    let json = serde_json::to_string_pretty(&sidecar)?;
    if let Some(parent) = sidecar_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::write(&sidecar_path, json)?;
    Ok(sidecar_path)
}

/// Load an edit stack from an existing sidecar file if present.
pub fn load_sidecar(image_path: impl AsRef<Path>) -> Result<Option<EditStack>, IoError> {
    let sidecar_path = sidecar_path_for(image_path);
    if !sidecar_path.is_file() {
        return Ok(None);
    }
    let content = fs::read_to_string(&sidecar_path)?;
    let sidecar: SnapSidecar = serde_json::from_str(&content)?;
    Ok(Some(sidecar.stack))
}
