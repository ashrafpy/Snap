use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::error::CoreError;
use crate::looks::LookPreset;
use crate::ops::ToolId;

/// Supported export image formats.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ExportFormat {
    Jpeg { quality: u8 },
    Png8,
    Png16,
    Tiff16,
}

impl Default for ExportFormat {
    fn default() -> Self {
        Self::Jpeg { quality: 92 }
    }
}

/// Long-edge downsampling options for export.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ResizeLongEdge {
    #[default]
    Original,
    Px4000,
    Px2000,
    Px1920,
    Px1366,
    Px800,
}

impl ResizeLongEdge {
    pub fn pixels(&self) -> Option<u32> {
        match self {
            Self::Original => None,
            Self::Px4000 => Some(4000),
            Self::Px2000 => Some(2000),
            Self::Px1920 => Some(1920),
            Self::Px1366 => Some(1366),
            Self::Px800 => Some(800),
        }
    }
}

/// Full options configuration for exporting images.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub struct ExportOptions {
    pub format: ExportFormat,
    pub resize: ResizeLongEdge,
    pub preserve_exif: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            format: ExportFormat::default(),
            resize: ResizeLongEdge::default(),
            preserve_exif: true,
        }
    }
}

/// Application settings and user preferences persisted to XDG configuration directory.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AppConfig {
    pub favorites: Vec<ToolId>,
    pub recent_files: Vec<PathBuf>,
    pub dark_theme: bool,
    pub show_sidebar: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            favorites: vec![
                ToolId::TuneImage,
                ToolId::Lut3d,
                ToolId::Curves,
                ToolId::Details,
                ToolId::ColorHsl,
                ToolId::CropRotate,
            ],
            recent_files: Vec::new(),
            dark_theme: true,
            show_sidebar: true,
        }
    }
}

impl AppConfig {
    /// Return the XDG config directory: `~/.config/snap`
    pub fn config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("snap")
    }

    /// Return the XDG local data directory: `~/.local/share/snap`
    pub fn data_dir() -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("~/.local/share"))
            .join("snap")
    }

    /// Return the directory for saved custom looks: `~/.local/share/snap/looks/`
    pub fn looks_dir() -> PathBuf {
        Self::data_dir().join("looks")
    }

    /// Return the directory for user-installed 3D LUTs: `~/.local/share/snap/luts/`
    pub fn luts_dir() -> PathBuf {
        Self::data_dir().join("luts")
    }

    /// Return the full path to `config.json`: `~/.config/snap/config.json`
    pub fn config_file_path() -> PathBuf {
        Self::config_dir().join("config.json")
    }

    /// Load application configuration from disk, falling back to default if not found or invalid.
    pub fn load() -> Self {
        let path = Self::config_file_path();
        if !path.exists() {
            return Self::default();
        }
        match fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Save current configuration to disk as pretty JSON.
    pub fn save(&self) -> Result<(), CoreError> {
        let dir = Self::config_dir();
        fs::create_dir_all(&dir)?;
        let path = Self::config_file_path();
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    /// Load all custom user looks stored in `looks_dir()`.
    pub fn load_custom_looks() -> Result<Vec<LookPreset>, CoreError> {
        let dir = Self::looks_dir();
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut looks = Vec::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(look) = serde_json::from_str::<LookPreset>(&content) {
                        looks.push(look);
                    }
                }
            }
        }
        Ok(looks)
    }

    /// Save a custom look to `looks_dir()/<id>.json`.
    pub fn save_custom_look(look: &LookPreset) -> Result<(), CoreError> {
        let dir = Self::looks_dir();
        fs::create_dir_all(&dir)?;
        let filename = format!("{}.json", look.id);
        let path = dir.join(filename);
        let json = serde_json::to_string_pretty(look)?;
        fs::write(path, json)?;
        Ok(())
    }
}
