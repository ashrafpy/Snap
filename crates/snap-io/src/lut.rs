use half::f16;
use snap_core::config::AppConfig;
use snap_core::image_types::{Lut3D, LutColorSpace};
use std::fs;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;

use crate::error::IoError;

/// Returns true if the file path has a supported 3D LUT extension (`.cube`, `.3dl`, `.png`, `.zip`).
pub fn is_supported_lut_path(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    matches!(ext.as_str(), "cube" | "3dl" | "png" | "zip")
}

/// Parse an Adobe `.cube` 3D LUT file content into `Lut3D`.
///
/// Supports `TITLE`, `LUT_3D_SIZE N` (`2..=129`), `DOMAIN_MIN`, `DOMAIN_MAX`,
/// comments (`#`), and R-fastest data layout (`r + g*N + b*N*N`).
pub fn parse_cube_lut(
    id: impl Into<String>,
    name: impl Into<String>,
    content: &str,
) -> Result<Lut3D, IoError> {
    let mut custom_title = None;
    let mut size_opt = None;
    let mut domain_min = [0.0f32, 0.0, 0.0];
    let mut domain_max = [1.0f32, 1.0, 1.0];
    let mut data = Vec::new();

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let mut parts = line.split_whitespace();
        let first = match parts.next() {
            Some(f) => f,
            None => continue,
        };

        if first.eq_ignore_ascii_case("TITLE") {
            let rest = line["TITLE".len()..].trim();
            let title = rest.trim_matches('"').trim();
            if !title.is_empty() {
                custom_title = Some(title.to_string());
            }
        } else if first.eq_ignore_ascii_case("LUT_3D_SIZE") {
            let size_str = parts
                .next()
                .ok_or_else(|| IoError::LutParse("LUT_3D_SIZE missing size value".into()))?;
            let s: u32 = size_str
                .parse()
                .map_err(|_| IoError::LutParse(format!("Invalid LUT_3D_SIZE: {size_str}")))?;
            if !(2..=129).contains(&s) {
                return Err(IoError::LutParse(format!(
                    "LUT_3D_SIZE {s} outside allowed range 2..=129"
                )));
            }
            size_opt = Some(s);
            data.reserve((s * s * s) as usize);
        } else if first.eq_ignore_ascii_case("DOMAIN_MIN") {
            let r: f32 = parts
                .next()
                .and_then(|v| v.parse().ok())
                .ok_or_else(|| IoError::LutParse("Invalid DOMAIN_MIN R".into()))?;
            let g: f32 = parts
                .next()
                .and_then(|v| v.parse().ok())
                .ok_or_else(|| IoError::LutParse("Invalid DOMAIN_MIN G".into()))?;
            let b: f32 = parts
                .next()
                .and_then(|v| v.parse().ok())
                .ok_or_else(|| IoError::LutParse("Invalid DOMAIN_MIN B".into()))?;
            domain_min = [r, g, b];
        } else if first.eq_ignore_ascii_case("DOMAIN_MAX") {
            let r: f32 = parts
                .next()
                .and_then(|v| v.parse().ok())
                .ok_or_else(|| IoError::LutParse("Invalid DOMAIN_MAX R".into()))?;
            let g: f32 = parts
                .next()
                .and_then(|v| v.parse().ok())
                .ok_or_else(|| IoError::LutParse("Invalid DOMAIN_MAX G".into()))?;
            let b: f32 = parts
                .next()
                .and_then(|v| v.parse().ok())
                .ok_or_else(|| IoError::LutParse("Invalid DOMAIN_MAX B".into()))?;
            domain_max = [r, g, b];
        } else if first.eq_ignore_ascii_case("LUT_1D_SIZE") {
            return Err(IoError::LutParse("1D LUTs are not supported".into()));
        } else if let Ok(r) = first.parse::<f32>() {
            let g: f32 = parts
                .next()
                .and_then(|v| v.parse().ok())
                .ok_or_else(|| IoError::LutParse(format!("Incomplete RGB triple: {line}")))?;
            let b: f32 = parts
                .next()
                .and_then(|v| v.parse().ok())
                .ok_or_else(|| IoError::LutParse(format!("Incomplete RGB triple: {line}")))?;
            data.push([
                f16::from_f32(r),
                f16::from_f32(g),
                f16::from_f32(b),
                f16::from_f32(1.0),
            ]);
        }
    }

    let size = size_opt.ok_or_else(|| IoError::LutParse("Missing LUT_3D_SIZE header".into()))?;
    let expected = (size * size * size) as usize;
    if data.len() != expected {
        return Err(IoError::LutParse(format!(
            "Expected {expected} LUT entries for size {size}, got {}",
            data.len()
        )));
    }

    let final_name = custom_title.unwrap_or_else(|| name.into());

    Ok(Lut3D {
        id: id.into(),
        name: final_name,
        size,
        domain_min,
        domain_max,
        input_space: LutColorSpace::Srgb,
        data: Arc::new(data),
    })
}

/// Parse an Autodesk `.3dl` 3D LUT file content into `Lut3D`.
///
/// Supports optional shaper grid lines, integer/float triples, and maps .3dl B-fastest ordering
/// (`for r { for g { for b } }`) into `Lut3D` R-fastest ordering (`r + g*N + b*N*N`).
pub fn parse_3dl_lut(
    id: impl Into<String>,
    name: impl Into<String>,
    content: &str,
) -> Result<Lut3D, IoError> {
    let mut shaper_size = None;
    let mut triples: Vec<[f32; 3]> = Vec::new();
    let mut max_val = 1.0f32;

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('<') {
            continue;
        }

        let nums: Result<Vec<f32>, _> = line
            .split_whitespace()
            .map(|tok| tok.parse::<f32>())
            .collect();
        let nums = match nums {
            Ok(v) => v,
            Err(_) => continue,
        };

        if nums.len() == 3 {
            let r = nums[0];
            let g = nums[1];
            let b = nums[2];
            max_val = max_val.max(r).max(g).max(b);
            triples.push([r, g, b]);
        } else if nums.len() >= 2 {
            // Optional shaper grid line (e.g. 17 or 33 integers)
            shaper_size = Some(nums.len() as u32);
        }
    }

    let total = triples.len();
    if total == 0 {
        return Err(IoError::LutParse("No RGB data found in .3dl file".into()));
    }

    let size = if let Some(sz) = shaper_size {
        if (sz * sz * sz) as usize == total {
            sz
        } else {
            let c = (total as f64).cbrt().round() as u32;
            if (c * c * c) as usize != total {
                return Err(IoError::LutParse(format!(
                    "Invalid .3dl entry count {total}: not a perfect cube"
                )));
            }
            c
        }
    } else {
        let c = (total as f64).cbrt().round() as u32;
        if (c * c * c) as usize != total {
            return Err(IoError::LutParse(format!(
                "Invalid .3dl entry count {total}: not a perfect cube"
            )));
        }
        c
    };

    if !(2..=129).contains(&size) {
        return Err(IoError::LutParse(format!(
            "3DL LUT size {size} outside allowed range 2..=129"
        )));
    }

    // Determine normalization divisor for integer formats (e.g. 1023, 4095, 255)
    let divisor = if max_val > 1.0 {
        if max_val <= 255.0 {
            255.0
        } else if max_val <= 1023.0 {
            1023.0
        } else if max_val <= 4095.0 {
            4095.0
        } else if max_val <= 65535.0 {
            65535.0
        } else {
            max_val
        }
    } else {
        1.0
    };

    // Reorder from .3dl B-fastest into Lut3D R-fastest:
    // .3dl loops: for r { for g { for b } }
    // Lut3D index: r + g * N + b * N * N
    let expected = (size * size * size) as usize;
    let mut lut_data = vec![
        [
            f16::ZERO,
            f16::ZERO,
            f16::ZERO,
            f16::ONE
        ];
        expected
    ];

    let mut triple_idx = 0;
    for r in 0..size {
        for g in 0..size {
            for b in 0..size {
                let [rv, gv, bv] = triples[triple_idx];
                triple_idx += 1;

                let lut_idx = (r + g * size + b * size * size) as usize;
                lut_data[lut_idx] = [
                    f16::from_f32(rv / divisor),
                    f16::from_f32(gv / divisor),
                    f16::from_f32(bv / divisor),
                    f16::from_f32(1.0),
                ];
            }
        }
    }

    Ok(Lut3D {
        id: id.into(),
        name: name.into(),
        size,
        domain_min: [0.0, 0.0, 0.0],
        domain_max: [1.0, 1.0, 1.0],
        input_space: LutColorSpace::Srgb,
        data: Arc::new(lut_data),
    })
}

/// Parse a HaldCLUT PNG image into `Lut3D`.
///
/// Decodes square PNG where `width == height == level^3` (`level` in `2..=16`).
/// Pixel index `i = y * width + x` maps directly to R-fastest `size^3` with `size = level^2`.
pub fn parse_hald_png_lut(
    id: impl Into<String>,
    name: impl Into<String>,
    bytes: &[u8],
) -> Result<Lut3D, IoError> {
    let img = image::load_from_memory(bytes)?;
    let (w, h) = (img.width(), img.height());
    if w != h {
        return Err(IoError::LutParse(format!(
            "HaldCLUT image must be square, got {w}x{h}"
        )));
    }

    let level = (w as f64).cbrt().round() as u32;
    if !(2..=16).contains(&level) || level * level * level != w {
        return Err(IoError::LutParse(format!(
            "HaldCLUT image dimension {w}x{h} does not correspond to level^3 for level in 2..=16"
        )));
    }

    let size = level * level;
    let total_expected = (size * size * size) as usize;

    let data: Vec<[f16; 4]> = match img {
        image::DynamicImage::ImageRgb16(rgb16) => rgb16
            .pixels()
            .map(|p| {
                [
                    f16::from_f32(p[0] as f32 / 65535.0),
                    f16::from_f32(p[1] as f32 / 65535.0),
                    f16::from_f32(p[2] as f32 / 65535.0),
                    f16::from_f32(1.0),
                ]
            })
            .collect(),
        image::DynamicImage::ImageRgba16(rgba16) => rgba16
            .pixels()
            .map(|p| {
                [
                    f16::from_f32(p[0] as f32 / 65535.0),
                    f16::from_f32(p[1] as f32 / 65535.0),
                    f16::from_f32(p[2] as f32 / 65535.0),
                    f16::from_f32(1.0),
                ]
            })
            .collect(),
        other => {
            let rgb8 = other.to_rgb8();
            rgb8.pixels()
                .map(|p| {
                    [
                        f16::from_f32(p[0] as f32 / 255.0),
                        f16::from_f32(p[1] as f32 / 255.0),
                        f16::from_f32(p[2] as f32 / 255.0),
                        f16::from_f32(1.0),
                    ]
                })
                .collect()
        }
    };

    if data.len() != total_expected {
        return Err(IoError::LutParse(format!(
            "HaldCLUT pixel count {} did not match expected {total_expected}",
            data.len()
        )));
    }

    Ok(Lut3D {
        id: id.into(),
        name: name.into(),
        size,
        domain_min: [0.0, 0.0, 0.0],
        domain_max: [1.0, 1.0, 1.0],
        input_space: LutColorSpace::Srgb,
        data: Arc::new(data),
    })
}

/// Load one or more `Lut3D` tables from a `.cube`, `.3dl`, `.png` (HaldCLUT), or `.zip` bundle.
pub fn load_lut_from_path(path: impl AsRef<Path>) -> Result<Vec<Lut3D>, IoError> {
    let path = path.as_ref();
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("lut")
        .to_string();

    match ext.as_str() {
        "cube" => {
            let content = fs::read_to_string(path)?;
            let lut = parse_cube_lut(&stem, &stem, &content)?;
            Ok(vec![lut])
        }
        "3dl" => {
            let content = fs::read_to_string(path)?;
            let lut = parse_3dl_lut(&stem, &stem, &content)?;
            Ok(vec![lut])
        }
        "png" => {
            let bytes = fs::read(path)?;
            let lut = parse_hald_png_lut(&stem, &stem, &bytes)?;
            Ok(vec![lut])
        }
        "zip" => {
            let file = fs::File::open(path)?;
            let mut archive = zip::ZipArchive::new(file)?;
            let mut luts = Vec::new();

            for i in 0..archive.len() {
                let mut entry = archive.by_index(i)?;
                if !entry.is_file() {
                    continue;
                }
                let entry_name = entry.name().to_string();
                let entry_path = Path::new(&entry_name);
                let entry_ext = entry_path
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_ascii_lowercase();
                let entry_stem = entry_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("lut")
                    .to_string();

                match entry_ext.as_str() {
                    "cube" => {
                        let mut content = String::new();
                        entry.read_to_string(&mut content)?;
                        if let Ok(lut) = parse_cube_lut(&entry_stem, &entry_stem, &content) {
                            luts.push(lut);
                        }
                    }
                    "3dl" => {
                        let mut content = String::new();
                        entry.read_to_string(&mut content)?;
                        if let Ok(lut) = parse_3dl_lut(&entry_stem, &entry_stem, &content) {
                            luts.push(lut);
                        }
                    }
                    "png" => {
                        let mut bytes = Vec::new();
                        entry.read_to_end(&mut bytes)?;
                        if let Ok(lut) = parse_hald_png_lut(&entry_stem, &entry_stem, &bytes) {
                            luts.push(lut);
                        }
                    }
                    _ => {}
                }
            }

            if luts.is_empty() {
                return Err(IoError::LutParse(
                    "No valid LUT files (.cube, .3dl, .png) found in zip archive".into(),
                ));
            }

            Ok(luts)
        }
        _ => Err(IoError::UnsupportedFormat(format!(
            "Unsupported LUT file extension: {ext}"
        ))),
    }
}

/// Validate and install LUT files into a custom target directory.
pub fn install_lut_file_to(
    path: impl AsRef<Path>,
    target_dir: impl AsRef<Path>,
) -> Result<Vec<Lut3D>, IoError> {
    let path = path.as_ref();
    let luts = load_lut_from_path(path)?;
    let luts_dir = target_dir.as_ref();
    fs::create_dir_all(luts_dir)?;

    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    if ext == "zip" {
        let file = fs::File::open(path)?;
        let mut archive = zip::ZipArchive::new(file)?;
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;
            if !entry.is_file() {
                continue;
            }
            let entry_name = entry.name().to_string();
            let entry_path = Path::new(&entry_name);
            let entry_ext = entry_path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();

            if matches!(entry_ext.as_str(), "cube" | "3dl" | "png") {
                if let Some(file_name) = entry_path.file_name() {
                    let dest = luts_dir.join(file_name);
                    let mut out = fs::File::create(&dest)?;
                    std::io::copy(&mut entry, &mut out)?;
                }
            }
        }
    } else if let Some(file_name) = path.file_name() {
        let dest = luts_dir.join(file_name);
        fs::copy(path, dest)?;
    }

    Ok(luts)
}

/// Validate and install LUT files into `AppConfig::luts_dir()`.
pub fn install_lut_file(path: impl AsRef<Path>) -> Result<Vec<Lut3D>, IoError> {
    install_lut_file_to(path, AppConfig::luts_dir())
}

fn s_curve(x: f32, contrast: f32) -> f32 {
    let c = contrast.clamp(0.0, 2.0);
    let mid = 0.5f32;
    if x <= mid {
        0.5 * (2.0 * x).powf(1.0 + c * 0.5)
    } else {
        1.0 - 0.5 * (2.0 * (1.0 - x)).powf(1.0 + c * 0.5)
    }
}

fn generate_lut_33<F>(id: &str, name: &str, transform: F) -> Lut3D
where
    F: Fn(f32, f32, f32) -> [f32; 3],
{
    const SIZE: u32 = 33;
    let inv_s = 1.0 / ((SIZE - 1) as f32);
    let mut data = Vec::with_capacity((SIZE * SIZE * SIZE) as usize);

    // Standard .cube layout: R fastest, G second, B slowest
    for b in 0..SIZE {
        let b_val = (b as f32) * inv_s;
        for g in 0..SIZE {
            let g_val = (g as f32) * inv_s;
            for r in 0..SIZE {
                let r_val = (r as f32) * inv_s;
                let rgb = transform(r_val, g_val, b_val);
                data.push([
                    f16::from_f32(rgb[0].clamp(0.0, 1.0)),
                    f16::from_f32(rgb[1].clamp(0.0, 1.0)),
                    f16::from_f32(rgb[2].clamp(0.0, 1.0)),
                    f16::from_f32(1.0),
                ]);
            }
        }
    }

    Lut3D {
        id: id.to_string(),
        name: name.to_string(),
        size: SIZE,
        domain_min: [0.0, 0.0, 0.0],
        domain_max: [1.0, 1.0, 1.0],
        input_space: LutColorSpace::Srgb,
        data: Arc::new(data),
    }
}

/// Generate 6 tasteful 33x33x33 film/cinematic `Lut3D` tables in pure Rust.
pub fn built_in_luts() -> Vec<Lut3D> {
    vec![
        // 1. Kodak Portra 400 - Warm, natural skin tones, gentle highlight roll-off
        generate_lut_33("kodak_portra_400", "Warm Portra 400", |r, g, b| {
            let luma = 0.2126 * r + 0.7152 * g + 0.0722 * b;
            let tone_r = s_curve(r, 0.25);
            let tone_g = s_curve(g, 0.20);
            let tone_b = s_curve(b, 0.15);
            let out_r = tone_r * 1.04 + 0.01 * (1.0 - luma);
            let out_g = tone_g * 1.01;
            let out_b = tone_b * 0.94 + 0.02 * (1.0 - luma);
            [out_r, out_g, out_b]
        }),
        // 2. Fuji Velvia 50 - Rich saturation, punchy contrast, vibrant greens & blues
        generate_lut_33("fuji_velvia_50", "Vivid Slide 50", |r, g, b| {
            let luma = 0.2126 * r + 0.7152 * g + 0.0722 * b;
            let cr = s_curve(r, 0.65);
            let cg = s_curve(g, 0.65);
            let cb = s_curve(b, 0.65);
            let sat = 1.35;
            let out_r = luma + (cr - luma) * sat;
            let out_g = luma + (cg - luma) * sat;
            let out_b = luma + (cb - luma) * sat;
            [out_r, out_g, out_b]
        }),
        // 3. Fuji Eterna Cinema - Filmic low contrast, muted saturation, soft roll-off
        generate_lut_33("fuji_eterna_cinema", "Cinema Eterna", |r, g, b| {
            let luma = 0.2126 * r + 0.7152 * g + 0.0722 * b;
            let cr = 0.04 + 0.91 * r;
            let cg = 0.04 + 0.91 * g;
            let cb = 0.04 + 0.91 * b;
            let sat = 0.82;
            let mut out_r = luma + (cr - luma) * sat;
            let mut out_g = luma + (cg - luma) * sat;
            let mut out_b = luma + (cb - luma) * sat;
            let shadow_weight = (1.0 - luma).powf(2.0);
            out_r -= 0.015 * shadow_weight;
            out_g += 0.015 * shadow_weight;
            out_b += 0.025 * shadow_weight;
            [out_r, out_g, out_b]
        }),
        // 4. Teal & Orange Cinema - Classic Hollywood blockbuster palette
        generate_lut_33("teal_orange_cinema", "Teal & Orange", |r, g, b| {
            let luma = 0.2126 * r + 0.7152 * g + 0.0722 * b;
            let cr = s_curve(r, 0.40);
            let cg = s_curve(g, 0.35);
            let cb = s_curve(b, 0.35);
            let shadow_t = (1.0 - luma).clamp(0.0, 1.0);
            let high_t = luma.clamp(0.0, 1.0);
            let teal_r = -0.06 * shadow_t;
            let teal_g = 0.04 * shadow_t;
            let teal_b = 0.08 * shadow_t;
            let orange_r = 0.08 * high_t;
            let orange_g = 0.02 * high_t;
            let orange_b = -0.07 * high_t;
            [cr + teal_r + orange_r, cg + teal_g + orange_g, cb + teal_b + orange_b]
        }),
        // 5. Kodak Gold 200 - Golden nostalgia, amber highlights, warm vintage look
        generate_lut_33("kodak_gold_200", "Golden Nostalgia", |r, g, b| {
            let luma = 0.2126 * r + 0.7152 * g + 0.0722 * b;
            let cr = s_curve(r, 0.30);
            let cg = s_curve(g, 0.25);
            let cb = s_curve(b, 0.20);
            let shadow_t = (1.0 - luma).clamp(0.0, 1.0);
            let out_r = cr * 1.06 + 0.02 * shadow_t;
            let out_g = cg * 1.02 + 0.015 * shadow_t;
            let out_b = cb * 0.88 - 0.01 * shadow_t;
            [out_r, out_g, out_b]
        }),
        // 6. Ilford HP5 B&W - Silver halide monochrome with panchromatic response
        generate_lut_33("ilford_hp5_bw", "Silver Halide B&W", |r, g, b| {
            let bw_luma = 0.25 * r + 0.65 * g + 0.10 * b;
            let film_tone = s_curve(bw_luma, 0.55);
            [film_tone, film_tone, film_tone]
        }),
    ]
}

/// Combine built-in LUTs with any user-installed LUTs discovered in `AppConfig::luts_dir()`.
pub fn load_all_luts() -> Vec<Lut3D> {
    let mut all = built_in_luts();
    let dir = AppConfig::luts_dir();
    if dir.is_dir() {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if is_supported_lut_path(&p) {
                    if let Ok(mut luts) = load_lut_from_path(&p) {
                        all.append(&mut luts);
                    }
                }
            }
        }
    }
    all
}
