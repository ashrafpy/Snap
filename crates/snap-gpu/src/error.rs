use thiserror::Error;

#[derive(Error, Debug)]
pub enum GpuError {
    #[error("No suitable GPU adapter found")]
    AdapterNotFound,

    #[error("Failed to request GPU device: {0}")]
    DeviceRequestFailed(String),

    #[error("Texture allocation failed: {0}")]
    TextureAllocationFailed(String),

    #[error("Buffer allocation failed: {0}")]
    BufferAllocationFailed(String),

    #[error("Shader compilation error: {0}")]
    ShaderCompilationFailed(String),

    #[error("GPU buffer async mapping failed: {0}")]
    BufferAsyncError(String),

    #[error("Invalid image dimensions: {0}x{1}")]
    InvalidDimensions(u32, u32),

    #[error("Core error: {0}")]
    Core(#[from] snap_core::CoreError),

    #[error("LUT not found: {0}")]
    LutNotFound(String),
}
