use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("Invalid image dimensions: {width}x{height} does not match buffer length {pixel_count}")]
    InvalidDimensions {
        width: u32,
        height: u32,
        pixel_count: usize,
    },
    #[error("Invalid LUT size {size}: expected {expected} voxels, got {actual}")]
    InvalidLutSize {
        size: u32,
        expected: usize,
        actual: usize,
    },
    #[error("Curve point out of bounds: ({x}, {y})")]
    InvalidCurvePoint { x: f32, y: f32 },
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Core operation error: {0}")]
    Other(String),
}
