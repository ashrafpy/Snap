use thiserror::Error;

#[derive(Error, Debug)]
pub enum IoError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Image error: {0}")]
    Image(#[from] image::ImageError),

    #[error("Core error: {0}")]
    Core(#[from] snap_core::CoreError),

    #[error("JPEG decode error: {0}")]
    Jpeg(String),

    #[error("RAW decode error: {0}")]
    Raw(String),

    #[error("CMS error: {0}")]
    Cms(#[from] moxcms::CmsError),

    #[error("EXIF error: {0}")]
    Exif(#[from] exif::Error),

    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("LUT parse error: {0}")]
    LutParse(String),

    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("Invalid image data: {0}")]
    InvalidImageData(String),

    #[error("Export error: {0}")]
    Export(String),
}

impl From<rawler::RawlerError> for IoError {
    fn from(err: rawler::RawlerError) -> Self {
        IoError::Raw(err.to_string())
    }
}

impl From<zune_jpeg::errors::DecodeErrors> for IoError {
    fn from(err: zune_jpeg::errors::DecodeErrors) -> Self {
        IoError::Jpeg(format!("{err:?}"))
    }
}
