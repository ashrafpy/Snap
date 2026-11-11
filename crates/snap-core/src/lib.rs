pub mod color;
pub mod config;
pub mod curves;
pub mod error;
pub mod image_types;
pub mod looks;
pub mod ops;
pub mod stack;

// Clean top-level re-exports
pub use color::{
    linear_rec2020_to_oklab, linear_rec2020_to_srgb, linear_srgb_to_oklab,
    linear_srgb_to_rec2020, linear_to_srgb, oklab_to_linear_rec2020, oklab_to_linear_srgb,
    oklab_to_oklch, oklch_to_oklab, rec2020_luma, srgb_to_linear, REC2020_TO_SRGB,
    SRGB_TO_REC2020,
};
pub use config::{AppConfig, ExportFormat, ExportOptions, ResizeLongEdge};
pub use curves::{CurveChannel, CurveChannelId, CurvePoint, CurvePreset, CurvesParams};
pub use error::CoreError;
pub use image_types::{HistogramData, ImageMetadata, LinearImage, Lut3D, LutColorSpace};
pub use looks::{built_in_looks, LookId, LookPreset};
pub use ops::{
    AspectRatio, BwColorFilter, BwParams, BwPreset, ColorHslParams, CropRect, CropRotateParams,
    DetailsParams, EditOp, HslBand, HslChannelShift, Lut3dParams, ParamDescriptor,
    RawDevelopParams, ToolId, TuneParams, VignetteParams, WhiteBalanceParams, WbPreset,
};
pub use stack::{EditNode, EditStack, HistoryManager, ToolSessionHistory, DEFAULT_MAX_HISTORY};

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
