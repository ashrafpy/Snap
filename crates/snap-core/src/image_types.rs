use half::f16;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

use crate::error::CoreError;

/// Linear Rec.2020 image with 16-bit float channels (RGBA16Float), row-major.
#[derive(Clone, Debug, PartialEq)]
pub struct LinearImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Arc<Vec<[f16; 4]>>,
    pub metadata: ImageMetadata,
}

impl LinearImage {
    /// Create a new `LinearImage`. Validates pixel buffer length matches `width * height`.
    pub fn new(
        width: u32,
        height: u32,
        pixels: impl Into<Arc<Vec<[f16; 4]>>>,
        metadata: ImageMetadata,
    ) -> Result<Self, CoreError> {
        let pixels = pixels.into();
        let expected = (width as usize) * (height as usize);
        if pixels.len() != expected {
            return Err(CoreError::InvalidDimensions {
                width,
                height,
                pixel_count: pixels.len(),
            });
        }
        Ok(Self {
            width,
            height,
            pixels,
            metadata,
        })
    }

    /// Construct a `LinearImage` from f32 RGBA pixels.
    pub fn from_f32_rgba(
        width: u32,
        height: u32,
        f32_rgba: &[[f32; 4]],
        metadata: ImageMetadata,
    ) -> Result<Self, CoreError> {
        let expected = (width as usize) * (height as usize);
        if f32_rgba.len() != expected {
            return Err(CoreError::InvalidDimensions {
                width,
                height,
                pixel_count: f32_rgba.len(),
            });
        }
        let pixels: Vec<[f16; 4]> = f32_rgba
            .iter()
            .map(|&[r, g, b, a]| {
                [
                    f16::from_f32(r),
                    f16::from_f32(g),
                    f16::from_f32(b),
                    f16::from_f32(a),
                ]
            })
            .collect();
        Ok(Self {
            width,
            height,
            pixels: Arc::new(pixels),
            metadata,
        })
    }

    /// Fast box downsampler for viewport proxy rendering when `width.max(height) > max_dim`.
    pub fn downsample_box(&self, max_dim: u32) -> LinearImage {
        if max_dim == 0 || self.width.max(self.height) <= max_dim {
            return self.clone();
        }

        let scale = max_dim as f32 / (self.width.max(self.height) as f32);
        let target_w = ((self.width as f32 * scale).round() as u32).max(1);
        let target_h = ((self.height as f32 * scale).round() as u32).max(1);

        let mut out_pixels = Vec::with_capacity((target_w * target_h) as usize);

        for y_dst in 0..target_h {
            let y_start = (y_dst as f32 * self.height as f32 / target_h as f32).floor() as usize;
            let y_end = (((y_dst + 1) as f32 * self.height as f32 / target_h as f32).ceil()
                as usize)
                .min(self.height as usize)
                .max(y_start + 1);

            for x_dst in 0..target_w {
                let x_start = (x_dst as f32 * self.width as f32 / target_w as f32).floor() as usize;
                let x_end = (((x_dst + 1) as f32 * self.width as f32 / target_w as f32).ceil()
                    as usize)
                    .min(self.width as usize)
                    .max(x_start + 1);

                let mut r_acc = 0.0f32;
                let mut g_acc = 0.0f32;
                let mut b_acc = 0.0f32;
                let mut a_acc = 0.0f32;
                let mut count = 0usize;

                for sy in y_start..y_end {
                    let row_idx = sy * self.width as usize;
                    for sx in x_start..x_end {
                        let px = self.pixels[row_idx + sx];
                        r_acc += px[0].to_f32();
                        g_acc += px[1].to_f32();
                        b_acc += px[2].to_f32();
                        a_acc += px[3].to_f32();
                        count += 1;
                    }
                }

                let inv_c = 1.0 / (count.max(1) as f32);
                out_pixels.push([
                    f16::from_f32(r_acc * inv_c),
                    f16::from_f32(g_acc * inv_c),
                    f16::from_f32(b_acc * inv_c),
                    f16::from_f32(a_acc * inv_c),
                ]);
            }
        }

        LinearImage {
            width: target_w,
            height: target_h,
            pixels: Arc::new(out_pixels),
            metadata: self.metadata.clone(),
        }
    }

    /// Sample pixel with bilinear interpolation using normalized coordinates `u, v` in `0.0..=1.0`.
    pub fn sample_bilinear_f32(&self, u: f32, v: f32) -> [f32; 4] {
        if self.width == 0 || self.height == 0 || self.pixels.is_empty() {
            return [0.0, 0.0, 0.0, 1.0];
        }

        let u_clamped = u.clamp(0.0, 1.0);
        let v_clamped = v.clamp(0.0, 1.0);

        if self.width == 1 && self.height == 1 {
            let p = self.pixels[0];
            return [p[0].to_f32(), p[1].to_f32(), p[2].to_f32(), p[3].to_f32()];
        }

        let fx_coord = u_clamped * ((self.width - 1) as f32);
        let fy_coord = v_clamped * ((self.height - 1) as f32);

        let x0 = (fx_coord.floor() as usize).min(self.width as usize - 1);
        let x1 = (x0 + 1).min(self.width as usize - 1);
        let y0 = (fy_coord.floor() as usize).min(self.height as usize - 1);
        let y1 = (y0 + 1).min(self.height as usize - 1);

        let fx = fx_coord - x0 as f32;
        let fy = fy_coord - y0 as f32;

        let w = self.width as usize;
        let p00 = self.pixels[y0 * w + x0];
        let p10 = self.pixels[y0 * w + x1];
        let p01 = self.pixels[y1 * w + x0];
        let p11 = self.pixels[y1 * w + x1];

        let mut out = [0.0f32; 4];
        for c in 0..4 {
            let c00 = p00[c].to_f32();
            let c10 = p10[c].to_f32();
            let c01 = p01[c].to_f32();
            let c11 = p11[c].to_f32();

            let top = c00 * (1.0 - fx) + c10 * fx;
            let bottom = c01 * (1.0 - fx) + c11 * fx;
            out[c] = top * (1.0 - fy) + bottom * fy;
        }

        out
    }
}

/// Metadata preserved across the editing pipeline.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ImageMetadata {
    pub source_path: Option<PathBuf>,
    pub original_width: u32,
    pub original_height: u32,
    pub is_raw: bool,
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
    pub lens_model: Option<String>,
    pub iso: Option<u32>,
    pub shutter_speed: Option<String>,
    pub aperture: Option<f32>,
    pub focal_length: Option<f32>,
    pub color_space_name: String,
    pub exif_orientation: u16,
}

impl Default for ImageMetadata {
    fn default() -> Self {
        Self {
            source_path: None,
            original_width: 0,
            original_height: 0,
            is_raw: false,
            camera_make: None,
            camera_model: None,
            lens_model: None,
            iso: None,
            shutter_speed: None,
            aperture: None,
            focal_length: None,
            color_space_name: "Linear Rec.2020".to_string(),
            exif_orientation: 1,
        }
    }
}

/// 256-bin RGB and Luminance histogram data.
#[derive(Clone, Debug, PartialEq)]
pub struct HistogramData {
    pub r: [u32; 256],
    pub g: [u32; 256],
    pub b: [u32; 256],
    pub luma: [u32; 256],
    pub max_bin: u32,
}

impl Default for HistogramData {
    fn default() -> Self {
        Self {
            r: [0; 256],
            g: [0; 256],
            b: [0; 256],
            luma: [0; 256],
            max_bin: 0,
        }
    }
}

impl HistogramData {
    /// Construct histogram and compute `max_bin` ignoring extreme clipping bins 0 & 255 if interior bins exist.
    pub fn from_bins(r: [u32; 256], g: [u32; 256], b: [u32; 256], luma: [u32; 256]) -> Self {
        let mut interior_max = 0u32;
        for i in 1..255 {
            interior_max = interior_max.max(r[i]).max(g[i]).max(b[i]).max(luma[i]);
        }

        let max_bin = if interior_max > 0 {
            interior_max
        } else {
            let mut full_max = 0u32;
            for i in 0..256 {
                full_max = full_max.max(r[i]).max(g[i]).max(b[i]).max(luma[i]);
            }
            full_max
        };

        Self {
            r,
            g,
            b,
            luma,
            max_bin,
        }
    }

    /// Compute histogram from a LinearImage using display gamma for binning.
    pub fn from_linear_image(image: &LinearImage) -> Self {
        let mut r_bins = [0u32; 256];
        let mut g_bins = [0u32; 256];
        let mut b_bins = [0u32; 256];
        let mut luma_bins = [0u32; 256];

        for px in image.pixels.iter() {
            let r = px[0].to_f32().max(0.0);
            let g = px[1].to_f32().max(0.0);
            let b = px[2].to_f32().max(0.0);
            let l = crate::color::rec2020_luma([r, g, b]);

            let r_s = crate::color::linear_to_srgb(r);
            let g_s = crate::color::linear_to_srgb(g);
            let b_s = crate::color::linear_to_srgb(b);
            let l_s = crate::color::linear_to_srgb(l);

            let bin_r = (r_s * 255.0).clamp(0.0, 255.0).round() as usize;
            let bin_g = (g_s * 255.0).clamp(0.0, 255.0).round() as usize;
            let bin_b = (b_s * 255.0).clamp(0.0, 255.0).round() as usize;
            let bin_l = (l_s * 255.0).clamp(0.0, 255.0).round() as usize;

            r_bins[bin_r] += 1;
            g_bins[bin_g] += 1;
            b_bins[bin_b] += 1;
            luma_bins[bin_l] += 1;
        }

        Self::from_bins(r_bins, g_bins, b_bins, luma_bins)
    }
}

/// Color space for 3D LUT inputs.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum LutColorSpace {
    #[default]
    Srgb,
    Rec709,
    LinearRec2020,
    AppleLog,
    SonySLog3,
    CanonLog2,
}

impl LutColorSpace {
    pub const ALL: &'static [Self] = &[
        Self::Srgb,
        Self::Rec709,
        Self::LinearRec2020,
        Self::AppleLog,
        Self::SonySLog3,
        Self::CanonLog2,
    ];
}

impl fmt::Display for LutColorSpace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Srgb => write!(f, "sRGB"),
            Self::Rec709 => write!(f, "Rec.709"),
            Self::LinearRec2020 => write!(f, "Linear Rec.2020"),
            Self::AppleLog => write!(f, "Apple Log"),
            Self::SonySLog3 => write!(f, "Sony S-Log3"),
            Self::CanonLog2 => write!(f, "Canon Log 2"),
        }
    }
}

/// 3D Lookup Table for creative color grading.
#[derive(Clone, Debug, PartialEq)]
pub struct Lut3D {
    pub id: String,
    pub name: String,
    pub size: u32,
    pub domain_min: [f32; 3],
    pub domain_max: [f32; 3],
    pub input_space: LutColorSpace,
    pub data: Arc<Vec<[f16; 4]>>,
}

impl Lut3D {
    /// Construct an identity 3D LUT with the given size (voxels per axis).
    pub fn identity(size: u32) -> Self {
        assert!(size >= 2, "LUT size must be at least 2");
        let count = (size * size * size) as usize;
        let mut data = Vec::with_capacity(count);

        let inv_s = 1.0 / ((size - 1) as f32);
        // Standard .cube layout: R changes fastest, then G, then B
        for b in 0..size {
            let b_val = (b as f32) * inv_s;
            for g in 0..size {
                let g_val = (g as f32) * inv_s;
                for r in 0..size {
                    let r_val = (r as f32) * inv_s;
                    data.push([
                        f16::from_f32(r_val),
                        f16::from_f32(g_val),
                        f16::from_f32(b_val),
                        f16::from_f32(1.0),
                    ]);
                }
            }
        }

        Self {
            id: format!("identity_{size}"),
            name: format!("Identity {size}x{size}x{size}"),
            size,
            domain_min: [0.0, 0.0, 0.0],
            domain_max: [1.0, 1.0, 1.0],
            input_space: LutColorSpace::Srgb,
            data: Arc::new(data),
        }
    }

    /// CPU reference trilinear sampler for verification and CPU testing.
    pub fn sample_trilinear(&self, rgb: [f32; 3]) -> [f32; 3] {
        if self.size < 2 || self.data.is_empty() {
            return rgb;
        }

        // Normalize rgb within domain_min..=domain_max
        let norm_r = ((rgb[0] - self.domain_min[0])
            / (self.domain_max[0] - self.domain_min[0]).max(1e-6))
        .clamp(0.0, 1.0);
        let norm_g = ((rgb[1] - self.domain_min[1])
            / (self.domain_max[1] - self.domain_min[1]).max(1e-6))
        .clamp(0.0, 1.0);
        let norm_b = ((rgb[2] - self.domain_min[2])
            / (self.domain_max[2] - self.domain_min[2]).max(1e-6))
        .clamp(0.0, 1.0);

        let s = (self.size - 1) as f32;
        let rx = norm_r * s;
        let gy = norm_g * s;
        let bz = norm_b * s;

        let r0 = (rx.floor() as usize).min(self.size as usize - 1);
        let r1 = (r0 + 1).min(self.size as usize - 1);
        let g0 = (gy.floor() as usize).min(self.size as usize - 1);
        let g1 = (g0 + 1).min(self.size as usize - 1);
        let b0 = (bz.floor() as usize).min(self.size as usize - 1);
        let b1 = (b0 + 1).min(self.size as usize - 1);

        let fr = rx - (r0 as f32);
        let fg = gy - (g0 as f32);
        let fb = bz - (b0 as f32);

        let sz = self.size as usize;
        let idx = |r: usize, g: usize, b: usize| -> usize { r + g * sz + b * sz * sz };

        let v000 = self.data[idx(r0, g0, b0)];
        let v100 = self.data[idx(r1, g0, b0)];
        let v010 = self.data[idx(r0, g1, b0)];
        let v110 = self.data[idx(r1, g1, b0)];
        let v001 = self.data[idx(r0, g0, b1)];
        let v101 = self.data[idx(r1, g0, b1)];
        let v011 = self.data[idx(r0, g1, b1)];
        let v111 = self.data[idx(r1, g1, b1)];

        let mut out = [0.0f32; 3];
        for c in 0..3 {
            let p000 = v000[c].to_f32();
            let p100 = v100[c].to_f32();
            let p010 = v010[c].to_f32();
            let p110 = v110[c].to_f32();
            let p001 = v001[c].to_f32();
            let p101 = v101[c].to_f32();
            let p011 = v011[c].to_f32();
            let p111 = v111[c].to_f32();

            // Interpolate along R (fastest axis)
            let c00 = p000 * (1.0 - fr) + p100 * fr;
            let c10 = p010 * (1.0 - fr) + p110 * fr;
            let c01 = p001 * (1.0 - fr) + p101 * fr;
            let c11 = p011 * (1.0 - fr) + p111 * fr;

            // Interpolate along G
            let c0 = c00 * (1.0 - fg) + c10 * fg;
            let c1 = c01 * (1.0 - fg) + c11 * fg;

            // Interpolate along B
            out[c] = c0 * (1.0 - fb) + c1 * fb;
        }

        out
    }
}
