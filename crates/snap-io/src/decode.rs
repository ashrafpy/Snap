use half::f16;
use moxcms::{ColorProfile, Layout, TransformOptions};
use rayon::prelude::*;
use snap_core::image_types::{ImageMetadata, LinearImage};
use std::fs;
use std::path::Path;
use zune_jpeg::zune_core::bytestream::ZCursor;
use zune_jpeg::JpegDecoder;

use crate::error::IoError;

/// List of recognized RAW image file extensions (lowercase).
pub const RAW_EXTENSIONS: &[&str] = &[
    "dng", "cr2", "cr3", "nef", "nrw", "arw", "srf", "sr2", "raf", "orf", "rw2", "pef", "erf",
    "3fr", "iiq",
];

/// List of standard raster image extensions supported.
pub const RASTER_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "tiff", "tif", "webp", "bmp"];

/// Returns true if the path corresponds to a supported RAW digital camera image format.
pub fn is_raw_path(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    RAW_EXTENSIONS.contains(&ext.as_str())
}

/// Returns true if the path has a supported image extension (standard raster or digital camera RAW).
pub fn is_supported_image_path(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    RASTER_EXTENSIONS.contains(&ext.as_str()) || RAW_EXTENSIONS.contains(&ext.as_str())
}

/// Extract EXIF tags from byte buffer into `ImageMetadata`.
fn extract_exif_metadata(bytes: &[u8], metadata: &mut ImageMetadata) {
    let mut cursor = std::io::Cursor::new(bytes);
    if let Ok(exif) = exif::Reader::new().read_from_container(&mut cursor) {
        if let Some(f) = exif.get_field(exif::Tag::Make, exif::In::PRIMARY) {
            let s = f.display_value().to_string().trim().to_string();
            if !s.is_empty() {
                metadata.camera_make = Some(s);
            }
        }
        if let Some(f) = exif.get_field(exif::Tag::Model, exif::In::PRIMARY) {
            let s = f.display_value().to_string().trim().to_string();
            if !s.is_empty() {
                metadata.camera_model = Some(s);
            }
        }
        if let Some(f) = exif.get_field(exif::Tag::LensModel, exif::In::PRIMARY) {
            let s = f.display_value().to_string().trim().to_string();
            if !s.is_empty() {
                metadata.lens_model = Some(s);
            }
        }
        if let Some(f) = exif.get_field(exif::Tag::PhotographicSensitivity, exif::In::PRIMARY) {
            metadata.iso = f.value.get_uint(0);
        }
        if let Some(f) = exif.get_field(exif::Tag::ExposureTime, exif::In::PRIMARY) {
            let s = f.display_value().to_string().trim().to_string();
            if !s.is_empty() {
                metadata.shutter_speed = Some(s);
            }
        }
        if let Some(f) = exif.get_field(exif::Tag::FNumber, exif::In::PRIMARY) {
            if let exif::Value::Rational(ref v) = f.value {
                if !v.is_empty() {
                    metadata.aperture = Some(v[0].to_f32());
                }
            }
        }
        if let Some(f) = exif.get_field(exif::Tag::FocalLength, exif::In::PRIMARY) {
            if let exif::Value::Rational(ref v) = f.value {
                if !v.is_empty() {
                    metadata.focal_length = Some(v[0].to_f32());
                }
            }
        }
        if let Some(f) = exif.get_field(exif::Tag::Orientation, exif::In::PRIMARY) {
            if let Some(orient) = f.value.get_uint(0) {
                if (1..=8).contains(&orient) {
                    metadata.exif_orientation = orient as u16;
                }
            }
        }
    }
}

/// Convert an `image::DynamicImage` into a `LinearImage` (in Linear Rec.2020),
/// applying EXIF orientation and preserving full 16-bit precision if available.
fn dynamic_image_to_linear(
    mut dyn_img: image::DynamicImage,
    mut metadata: ImageMetadata,
) -> Result<LinearImage, IoError> {
    if metadata.exif_orientation > 1 && metadata.exif_orientation <= 8 {
        if let Some(orient) =
            image::metadata::Orientation::from_exif(metadata.exif_orientation as u8)
        {
            dyn_img.apply_orientation(orient);
            metadata.exif_orientation = 1;
        }
    }

    let w = dyn_img.width();
    let h = dyn_img.height();
    metadata.original_width = w;
    metadata.original_height = h;

    let srgb_table: [f32; 256] =
        std::array::from_fn(|i| snap_core::srgb_to_linear(i as f32 / 255.0));

    let pixels: Vec<[f16; 4]> = match dyn_img {
        image::DynamicImage::ImageRgb16(img) => img
            .as_raw()
            .par_chunks_exact(3)
            .map(|rgb| {
                let r = snap_core::srgb_to_linear(rgb[0] as f32 / 65535.0);
                let g = snap_core::srgb_to_linear(rgb[1] as f32 / 65535.0);
                let b = snap_core::srgb_to_linear(rgb[2] as f32 / 65535.0);
                let rec2020 = snap_core::linear_srgb_to_rec2020([r, g, b]);
                [
                    f16::from_f32(rec2020[0]),
                    f16::from_f32(rec2020[1]),
                    f16::from_f32(rec2020[2]),
                    f16::ONE,
                ]
            })
            .collect(),
        image::DynamicImage::ImageRgba16(img) => img
            .as_raw()
            .par_chunks_exact(4)
            .map(|rgba| {
                let r = snap_core::srgb_to_linear(rgba[0] as f32 / 65535.0);
                let g = snap_core::srgb_to_linear(rgba[1] as f32 / 65535.0);
                let b = snap_core::srgb_to_linear(rgba[2] as f32 / 65535.0);
                let a = rgba[3] as f32 / 65535.0;
                let rec2020 = snap_core::linear_srgb_to_rec2020([r, g, b]);
                [
                    f16::from_f32(rec2020[0]),
                    f16::from_f32(rec2020[1]),
                    f16::from_f32(rec2020[2]),
                    f16::from_f32(a),
                ]
            })
            .collect(),
        image::DynamicImage::ImageLuma16(img) => img
            .as_raw()
            .par_iter()
            .map(|&l| {
                let lin = snap_core::srgb_to_linear(l as f32 / 65535.0);
                let rec2020 = snap_core::linear_srgb_to_rec2020([lin, lin, lin]);
                [
                    f16::from_f32(rec2020[0]),
                    f16::from_f32(rec2020[1]),
                    f16::from_f32(rec2020[2]),
                    f16::ONE,
                ]
            })
            .collect(),
        image::DynamicImage::ImageLumaA16(img) => img
            .as_raw()
            .par_chunks_exact(2)
            .map(|la| {
                let lin = snap_core::srgb_to_linear(la[0] as f32 / 65535.0);
                let a = la[1] as f32 / 65535.0;
                let rec2020 = snap_core::linear_srgb_to_rec2020([lin, lin, lin]);
                [
                    f16::from_f32(rec2020[0]),
                    f16::from_f32(rec2020[1]),
                    f16::from_f32(rec2020[2]),
                    f16::from_f32(a),
                ]
            })
            .collect(),
        image::DynamicImage::ImageRgb8(img) => img
            .as_raw()
            .par_chunks_exact(3)
            .map(|rgb| {
                let r = srgb_table[rgb[0] as usize];
                let g = srgb_table[rgb[1] as usize];
                let b = srgb_table[rgb[2] as usize];
                let rec2020 = snap_core::linear_srgb_to_rec2020([r, g, b]);
                [
                    f16::from_f32(rec2020[0]),
                    f16::from_f32(rec2020[1]),
                    f16::from_f32(rec2020[2]),
                    f16::ONE,
                ]
            })
            .collect(),
        image::DynamicImage::ImageRgba8(img) => img
            .as_raw()
            .par_chunks_exact(4)
            .map(|rgba| {
                let r = srgb_table[rgba[0] as usize];
                let g = srgb_table[rgba[1] as usize];
                let b = srgb_table[rgba[2] as usize];
                let a = rgba[3] as f32 / 255.0;
                let rec2020 = snap_core::linear_srgb_to_rec2020([r, g, b]);
                [
                    f16::from_f32(rec2020[0]),
                    f16::from_f32(rec2020[1]),
                    f16::from_f32(rec2020[2]),
                    f16::from_f32(a),
                ]
            })
            .collect(),
        other => {
            let rgb8 = other.to_rgb8();
            rgb8.as_raw()
                .par_chunks_exact(3)
                .map(|rgb| {
                    let r = srgb_table[rgb[0] as usize];
                    let g = srgb_table[rgb[1] as usize];
                    let b = srgb_table[rgb[2] as usize];
                    let rec2020 = snap_core::linear_srgb_to_rec2020([r, g, b]);
                    [
                        f16::from_f32(rec2020[0]),
                        f16::from_f32(rec2020[1]),
                        f16::from_f32(rec2020[2]),
                        f16::ONE,
                    ]
                })
                .collect()
        }
    };

    LinearImage::new(w, h, pixels, metadata).map_err(IoError::Core)
}

/// Load an image file from disk and convert it into a `LinearImage` in Linear Rec.2020.
///
/// Handles digital camera RAW files (via `rawler`), JPEG files (fast decode via `zune-jpeg`
/// with ICC transform to sRGB via `moxcms` and EXIF orientation normalization), and
/// PNG / TIFF / WebP / BMP files (preserving full 16-bit precision when available).
pub fn load_image(path: impl AsRef<Path>) -> Result<LinearImage, IoError> {
    let path = path.as_ref();
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let mut metadata = ImageMetadata {
        source_path: Some(path.to_path_buf()),
        ..Default::default()
    };

    if is_raw_path(path) {
        let raw_img = rawler::decode_file(path)?;
        metadata.is_raw = true;

        if !raw_img.clean_make.is_empty() {
            metadata.camera_make = Some(raw_img.clean_make.clone());
        } else if !raw_img.make.is_empty() {
            metadata.camera_make = Some(raw_img.make.clone());
        }

        if !raw_img.clean_model.is_empty() {
            metadata.camera_model = Some(raw_img.clean_model.clone());
        } else if !raw_img.model.is_empty() {
            metadata.camera_model = Some(raw_img.model.clone());
        }

        if let Ok(bytes) = fs::read(path) {
            extract_exif_metadata(&bytes, &mut metadata);
        }

        let intermediate =
            rawler::imgop::develop::RawDevelop::default().develop_intermediate(&raw_img)?;
        let dyn_img = intermediate.to_dynamic_image().ok_or_else(|| {
            IoError::Raw("RawDevelop did not produce a valid image buffer".into())
        })?;

        return dynamic_image_to_linear(dyn_img, metadata);
    }

    let bytes = fs::read(path)?;
    extract_exif_metadata(&bytes, &mut metadata);

    let is_jpeg = ext == "jpg"
        || ext == "jpeg"
        || (bytes.len() >= 3 && bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF);

    if is_jpeg {
        // Fast decode path using zune-jpeg
        let decode_result = (|| -> Result<image::DynamicImage, IoError> {
            let mut decoder = JpegDecoder::new(ZCursor::new(&bytes));
            decoder.decode_headers()?;
            let info = decoder
                .info()
                .ok_or_else(|| IoError::Jpeg("Failed to decode JPEG info".into()))?;
            let width = info.width as u32;
            let height = info.height as u32;
            let icc = decoder.icc_profile();
            let decoded = decoder.decode()?;

            let mut rgb = if info.components == 1 {
                let mut buf = Vec::with_capacity(decoded.len() * 3);
                for &g in &decoded {
                    buf.push(g);
                    buf.push(g);
                    buf.push(g);
                }
                buf
            } else if info.components == 3 {
                decoded
            } else {
                return Err(IoError::Jpeg("Unsupported component count in JPEG".into()));
            };

            // Apply embedded ICC profile -> sRGB if present
            if let Some(icc_data) = icc {
                if let Ok(src_profile) = ColorProfile::new_from_slice(&icc_data) {
                    let dst_profile = ColorProfile::new_srgb();
                    if let Ok(transform) = src_profile.create_transform_8bit(
                        Layout::Rgb,
                        &dst_profile,
                        Layout::Rgb,
                        TransformOptions::default(),
                    ) {
                        let mut dst = vec![0u8; rgb.len()];
                        if transform.transform(&rgb, &mut dst).is_ok() {
                            rgb = dst;
                        }
                    }
                }
            }

            let rgb_img = image::RgbImage::from_raw(width, height, rgb).ok_or_else(|| {
                IoError::InvalidImageData("Failed to construct RGB image buffer".into())
            })?;
            Ok(image::DynamicImage::ImageRgb8(rgb_img))
        })();

        let dyn_img = match decode_result {
            Ok(img) => img,
            Err(_) => image::load_from_memory(&bytes)?,
        };

        return dynamic_image_to_linear(dyn_img, metadata);
    }

    // Standard raster decoding (PNG / TIFF / WebP / BMP)
    let dyn_img = image::load_from_memory(&bytes)?;
    dynamic_image_to_linear(dyn_img, metadata)
}
