use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::PngEncoder;
use image::codecs::tiff::TiffEncoder;
use image::{ExtendedColorType, ImageEncoder};
use rayon::prelude::*;
use snap_core::config::{ExportFormat, ExportOptions};
use snap_core::image_types::LinearImage;
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::Path;

use crate::error::IoError;

/// Export a `LinearImage` (in Linear Rec.2020 color space) to disk according to `ExportOptions`.
pub fn export_linear_image(
    img: &LinearImage,
    path: impl AsRef<Path>,
    options: &ExportOptions,
) -> Result<(), IoError> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    // Optional long-edge downsampling preserving aspect ratio
    let resampled;
    let target_img = if let Some(max_dim) = options.resize.pixels() {
        if img.width.max(img.height) > max_dim {
            resampled = img.downsample_box(max_dim);
            &resampled
        } else {
            img
        }
    } else {
        img
    };

    let width = target_img.width;
    let height = target_img.height;

    match options.format {
        ExportFormat::Jpeg { quality } => {
            // Convert Linear Rec.2020 -> linear sRGB -> non-linear sRGB (0..=255) in parallel
            let rgb8: Vec<u8> = target_img
                .pixels
                .par_iter()
                .flat_map_iter(|px| {
                    let rec2020 = [px[0].to_f32(), px[1].to_f32(), px[2].to_f32()];
                    let srgb_lin = snap_core::linear_rec2020_to_srgb(rec2020);
                    let r = (snap_core::linear_to_srgb(srgb_lin[0]).clamp(0.0, 1.0) * 255.0).round()
                        as u8;
                    let g = (snap_core::linear_to_srgb(srgb_lin[1]).clamp(0.0, 1.0) * 255.0).round()
                        as u8;
                    let b = (snap_core::linear_to_srgb(srgb_lin[2]).clamp(0.0, 1.0) * 255.0).round()
                        as u8;
                    [r, g, b]
                })
                .collect();

            let file = File::create(path)?;
            let mut writer = BufWriter::new(file);
            let mut encoder = JpegEncoder::new_with_quality(&mut writer, quality);
            encoder.encode(&rgb8, width, height, ExtendedColorType::Rgb8)?;
        }
        ExportFormat::Png8 => {
            let rgba8: Vec<u8> = target_img
                .pixels
                .par_iter()
                .flat_map_iter(|px| {
                    let rec2020 = [px[0].to_f32(), px[1].to_f32(), px[2].to_f32()];
                    let srgb_lin = snap_core::linear_rec2020_to_srgb(rec2020);
                    let r = (snap_core::linear_to_srgb(srgb_lin[0]).clamp(0.0, 1.0) * 255.0).round()
                        as u8;
                    let g = (snap_core::linear_to_srgb(srgb_lin[1]).clamp(0.0, 1.0) * 255.0).round()
                        as u8;
                    let b = (snap_core::linear_to_srgb(srgb_lin[2]).clamp(0.0, 1.0) * 255.0).round()
                        as u8;
                    let a = (px[3].to_f32().clamp(0.0, 1.0) * 255.0).round() as u8;
                    [r, g, b, a]
                })
                .collect();

            let file = File::create(path)?;
            let mut writer = BufWriter::new(file);
            let encoder = PngEncoder::new(&mut writer);
            encoder.write_image(&rgba8, width, height, ExtendedColorType::Rgba8)?;
        }
        ExportFormat::Png16 => {
            let rgba16: Vec<u16> = target_img
                .pixels
                .par_iter()
                .flat_map_iter(|px| {
                    let rec2020 = [px[0].to_f32(), px[1].to_f32(), px[2].to_f32()];
                    let srgb_lin = snap_core::linear_rec2020_to_srgb(rec2020);
                    let r = (snap_core::linear_to_srgb(srgb_lin[0]).clamp(0.0, 1.0) * 65535.0)
                        .round() as u16;
                    let g = (snap_core::linear_to_srgb(srgb_lin[1]).clamp(0.0, 1.0) * 65535.0)
                        .round() as u16;
                    let b = (snap_core::linear_to_srgb(srgb_lin[2]).clamp(0.0, 1.0) * 65535.0)
                        .round() as u16;
                    let a = (px[3].to_f32().clamp(0.0, 1.0) * 65535.0).round() as u16;
                    [r, g, b, a]
                })
                .collect();

            let bytes: &[u8] = bytemuck::cast_slice(&rgba16);
            let file = File::create(path)?;
            let mut writer = BufWriter::new(file);
            let encoder = PngEncoder::new(&mut writer);
            encoder.write_image(bytes, width, height, ExtendedColorType::Rgba16)?;
        }
        ExportFormat::Tiff16 => {
            let rgba16: Vec<u16> = target_img
                .pixels
                .par_iter()
                .flat_map_iter(|px| {
                    let rec2020 = [px[0].to_f32(), px[1].to_f32(), px[2].to_f32()];
                    let srgb_lin = snap_core::linear_rec2020_to_srgb(rec2020);
                    let r = (snap_core::linear_to_srgb(srgb_lin[0]).clamp(0.0, 1.0) * 65535.0)
                        .round() as u16;
                    let g = (snap_core::linear_to_srgb(srgb_lin[1]).clamp(0.0, 1.0) * 65535.0)
                        .round() as u16;
                    let b = (snap_core::linear_to_srgb(srgb_lin[2]).clamp(0.0, 1.0) * 65535.0)
                        .round() as u16;
                    let a = (px[3].to_f32().clamp(0.0, 1.0) * 65535.0).round() as u16;
                    [r, g, b, a]
                })
                .collect();

            let bytes: &[u8] = bytemuck::cast_slice(&rgba16);
            let file = File::create(path)?;
            let encoder = TiffEncoder::new(file);
            encoder.write_image(bytes, width, height, ExtendedColorType::Rgba16)?;
        }
    }

    Ok(())
}
