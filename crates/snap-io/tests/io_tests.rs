use half::f16;
use snap_core::config::{ExportFormat, ExportOptions, ResizeLongEdge};
use snap_core::image_types::{ImageMetadata, LinearImage, LutColorSpace};
use snap_core::ops::{CropRotateParams, EditOp, TuneParams};
use snap_core::stack::EditStack;
use snap_io::{
    built_in_luts, export_linear_image, install_lut_file, is_raw_path, is_supported_image_path,
    is_supported_lut_path, load_image, load_lut_from_path, load_sidecar, parse_3dl_lut,
    parse_cube_lut, parse_hald_png_lut, save_sidecar, sidecar_path_for,
};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(1);

struct TempTestDir {
    path: PathBuf,
}

impl TempTestDir {
    fn new(prefix: &str) -> Self {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!("snap_test_{prefix}_{}_{id}", std::process::id()));
        fs::create_dir_all(&path).expect("failed to create test temp dir");
        Self { path }
    }
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn create_synthetic_test_image(width: u32, height: u32) -> LinearImage {
    let mut pixels = Vec::with_capacity((width * height) as usize);

    for y in 0..height {
        for x in 0..width {
            let u = x as f32 / (width - 1).max(1) as f32;
            let v = y as f32 / (height - 1).max(1) as f32;

            // Generate known colorful sRGB coordinates
            let r_srgb = u;
            let g_srgb = v;
            let b_srgb = (1.0 - u * 0.5 - v * 0.5).clamp(0.0, 1.0);

            // Convert to Linear Rec.2020
            let r_lin = snap_core::srgb_to_linear(r_srgb);
            let g_lin = snap_core::srgb_to_linear(g_srgb);
            let b_lin = snap_core::srgb_to_linear(b_srgb);
            let rec2020 = snap_core::linear_srgb_to_rec2020([r_lin, g_lin, b_lin]);

            pixels.push([
                f16::from_f32(rec2020[0]),
                f16::from_f32(rec2020[1]),
                f16::from_f32(rec2020[2]),
                f16::ONE,
            ]);
        }
    }

    LinearImage::new(
        width,
        height,
        pixels,
        ImageMetadata {
            original_width: width,
            original_height: height,
            ..Default::default()
        },
    )
    .unwrap()
}

#[test]
fn test_extension_checks() {
    assert!(is_supported_image_path(Path::new("test.jpg")));
    assert!(is_supported_image_path(Path::new("test.JPEG")));
    assert!(is_supported_image_path(Path::new("photo.png")));
    assert!(is_supported_image_path(Path::new("photo.tiff")));
    assert!(is_supported_image_path(Path::new("raw.dng")));
    assert!(is_supported_image_path(Path::new("raw.cr3")));
    assert!(is_supported_image_path(Path::new("raw.nef")));
    assert!(is_supported_image_path(Path::new("raw.arw")));
    assert!(!is_supported_image_path(Path::new("document.pdf")));
    assert!(!is_supported_image_path(Path::new("archive.zip")));

    assert!(is_raw_path(Path::new("photo.dng")));
    assert!(is_raw_path(Path::new("photo.CR2")));
    assert!(is_raw_path(Path::new("photo.arw")));
    assert!(is_raw_path(Path::new("photo.raf")));
    assert!(!is_raw_path(Path::new("photo.jpg")));
    assert!(!is_raw_path(Path::new("photo.png")));

    assert!(is_supported_lut_path(Path::new("grade.cube")));
    assert!(is_supported_lut_path(Path::new("grade.3dl")));
    assert!(is_supported_lut_path(Path::new("grade.png")));
    assert!(is_supported_lut_path(Path::new("bundle.zip")));
    assert!(!is_supported_lut_path(Path::new("grade.txt")));
}

#[test]
fn test_export_and_decode_roundtrip_png16() {
    let tmp = TempTestDir::new("png16");
    let out_path = tmp.path.join("test.png");
    let orig = create_synthetic_test_image(64, 48);

    let opts = ExportOptions {
        format: ExportFormat::Png16,
        resize: ResizeLongEdge::Original,
        preserve_exif: false,
    };

    export_linear_image(&orig, &out_path, &opts).expect("failed to export PNG16");
    let loaded = load_image(&out_path).expect("failed to load PNG16");

    assert_eq!(loaded.width, 64);
    assert_eq!(loaded.height, 48);
    assert_eq!(loaded.pixels.len(), orig.pixels.len());

    // 16-bit roundtrip must have high color precision in Linear Rec.2020
    for (idx, (p_orig, p_load)) in orig.pixels.iter().zip(loaded.pixels.iter()).enumerate() {
        for c in 0..3 {
            let diff = (p_orig[c].to_f32() - p_load[c].to_f32()).abs();
            assert!(
                diff < 0.005,
                "Pixel {idx} channel {c} mismatch: orig {}, load {}, diff {}",
                p_orig[c].to_f32(),
                p_load[c].to_f32(),
                diff
            );
        }
    }
}

#[test]
fn test_export_and_decode_roundtrip_tiff16() {
    let tmp = TempTestDir::new("tiff16");
    let out_path = tmp.path.join("test.tif");
    let orig = create_synthetic_test_image(64, 48);

    let opts = ExportOptions {
        format: ExportFormat::Tiff16,
        resize: ResizeLongEdge::Original,
        preserve_exif: false,
    };

    export_linear_image(&orig, &out_path, &opts).expect("failed to export TIFF16");
    let loaded = load_image(&out_path).expect("failed to load TIFF16");

    assert_eq!(loaded.width, 64);
    assert_eq!(loaded.height, 48);

    for (idx, (p_orig, p_load)) in orig.pixels.iter().zip(loaded.pixels.iter()).enumerate() {
        for c in 0..3 {
            let diff = (p_orig[c].to_f32() - p_load[c].to_f32()).abs();
            assert!(
                diff < 0.005,
                "TIFF pixel {idx} channel {c} mismatch: orig {}, load {}, diff {}",
                p_orig[c].to_f32(),
                p_load[c].to_f32(),
                diff
            );
        }
    }
}

#[test]
fn test_export_and_decode_roundtrip_png8() {
    let tmp = TempTestDir::new("png8");
    let out_path = tmp.path.join("test8.png");
    let orig = create_synthetic_test_image(64, 48);

    let opts = ExportOptions {
        format: ExportFormat::Png8,
        resize: ResizeLongEdge::Original,
        preserve_exif: false,
    };

    export_linear_image(&orig, &out_path, &opts).expect("failed to export PNG8");
    let loaded = load_image(&out_path).expect("failed to load PNG8");

    assert_eq!(loaded.width, 64);
    assert_eq!(loaded.height, 48);

    // 8-bit quantization tolerance in Linear Rec.2020 space
    for (idx, (p_orig, p_load)) in orig.pixels.iter().zip(loaded.pixels.iter()).enumerate() {
        for c in 0..3 {
            let diff = (p_orig[c].to_f32() - p_load[c].to_f32()).abs();
            assert!(
                diff < 0.02,
                "PNG8 pixel {idx} channel {c} exceeds tolerance: orig {}, load {}, diff {}",
                p_orig[c].to_f32(),
                p_load[c].to_f32(),
                diff
            );
        }
    }
}

#[test]
fn test_export_and_decode_roundtrip_jpeg() {
    let tmp = TempTestDir::new("jpeg");
    let out_path = tmp.path.join("test.jpg");
    let orig = create_synthetic_test_image(64, 48);

    let opts = ExportOptions {
        format: ExportFormat::Jpeg { quality: 98 },
        resize: ResizeLongEdge::Original,
        preserve_exif: false,
    };

    export_linear_image(&orig, &out_path, &opts).expect("failed to export JPEG");
    let loaded = load_image(&out_path).expect("failed to load JPEG");

    assert_eq!(loaded.width, 64);
    assert_eq!(loaded.height, 48);

    // JPEG compression tolerance in Linear Rec.2020
    for (idx, (p_orig, p_load)) in orig.pixels.iter().zip(loaded.pixels.iter()).enumerate() {
        for c in 0..3 {
            let diff = (p_orig[c].to_f32() - p_load[c].to_f32()).abs();
            assert!(
                diff < 0.06,
                "JPEG pixel {idx} channel {c} exceeds tolerance: orig {}, load {}, diff {}",
                p_orig[c].to_f32(),
                p_load[c].to_f32(),
                diff
            );
        }
    }
}

#[test]
fn test_export_resize_px800() {
    let tmp = TempTestDir::new("resize");
    let out_path = tmp.path.join("resized.png");
    // Original 1600x1200 image
    let orig = create_synthetic_test_image(1600, 1200);

    let opts = ExportOptions {
        format: ExportFormat::Png8,
        resize: ResizeLongEdge::Px800,
        preserve_exif: false,
    };

    export_linear_image(&orig, &out_path, &opts).expect("failed to export with resize");
    let loaded = load_image(&out_path).expect("failed to load resized image");

    assert_eq!(loaded.width, 800);
    assert_eq!(loaded.height, 600);
}

#[test]
fn test_cube_lut_parsing() {
    // 1. Identity 2x2x2 cube
    let cube_identity = "\
# Identity LUT 2x2x2
TITLE \"Identity LUT\"
LUT_3D_SIZE 2
0.0 0.0 0.0
1.0 0.0 0.0
0.0 1.0 0.0
1.0 1.0 0.0
0.0 0.0 1.0
1.0 0.0 1.0
0.0 1.0 1.0
1.0 1.0 1.0
";

    let lut = parse_cube_lut("test_id", "fallback_name", cube_identity).expect("parse cube");
    assert_eq!(lut.name, "Identity LUT");
    assert_eq!(lut.size, 2);
    assert_eq!(lut.domain_min, [0.0, 0.0, 0.0]);
    assert_eq!(lut.domain_max, [1.0, 1.0, 1.0]);

    let sampled = lut.sample_trilinear([0.5, 0.5, 0.5]);
    assert!((sampled[0] - 0.5).abs() < 1e-3);
    assert!((sampled[1] - 0.5).abs() < 1e-3);
    assert!((sampled[2] - 0.5).abs() < 1e-3);

    // 2. Custom domain cube
    let cube_custom = "\
TITLE \"Custom Domain\"
LUT_3D_SIZE 2
DOMAIN_MIN 0.1 0.2 0.3
DOMAIN_MAX 0.9 0.8 0.7
0.1 0.2 0.3
0.9 0.2 0.3
0.1 0.8 0.3
0.9 0.8 0.3
0.1 0.2 0.7
0.9 0.2 0.7
0.1 0.8 0.7
0.9 0.8 0.7
";

    let lut_custom = parse_cube_lut("custom_id", "default", cube_custom).expect("parse custom cube");
    assert_eq!(lut_custom.name, "Custom Domain");
    assert_eq!(lut_custom.domain_min, [0.1, 0.2, 0.3]);
    assert_eq!(lut_custom.domain_max, [0.9, 0.8, 0.7]);
}

#[test]
fn test_3dl_lut_parsing_and_axis_order() {
    // .3dl loops: for r { for g { for b } } (B fastest)
    // 2x2x2 mesh with 10-bit integer coordinates (0..1023)
    let content_3dl = "\
# 3DL test file
<3DMESH>
0 1023
0 0 0
0 0 1023
0 1023 0
0 1023 1023
1023 0 0
1023 0 1023
1023 1023 0
1023 1023 1023
";

    let lut = parse_3dl_lut("test_3dl", "Test 3DL", content_3dl).expect("parse 3dl");
    assert_eq!(lut.size, 2);
    assert_eq!(lut.data.len(), 8);

    // In Lut3D R-fastest layout: index = r + g*2 + b*4
    // (r=0, g=0, b=0) -> idx 0: [0, 0, 0]
    let p000 = lut.data[0];
    assert_eq!([p000[0].to_f32(), p000[1].to_f32(), p000[2].to_f32()], [0.0, 0.0, 0.0]);

    // (r=1, g=0, b=0) -> idx 1: in .3dl this was entry index 4: "1023 0 0" -> [1.0, 0.0, 0.0]
    let p100 = lut.data[1];
    assert_eq!([p100[0].to_f32(), p100[1].to_f32(), p100[2].to_f32()], [1.0, 0.0, 0.0]);

    // (r=0, g=0, b=1) -> idx 4: in .3dl this was entry index 1: "0 0 1023" -> [0.0, 0.0, 1.0]
    let p001 = lut.data[4];
    assert_eq!([p001[0].to_f32(), p001[1].to_f32(), p001[2].to_f32()], [0.0, 0.0, 1.0]);

    // (r=1, g=1, b=1) -> idx 7: in .3dl this was entry index 7: "1023 1023 1023" -> [1.0, 1.0, 1.0]
    let p111 = lut.data[7];
    assert_eq!([p111[0].to_f32(), p111[1].to_f32(), p111[2].to_f32()], [1.0, 1.0, 1.0]);
}

#[test]
fn test_hald_png_lut_parsing() {
    // 64x64 HaldCLUT corresponds to level=4 (4^3 = 64), size = level^2 = 16
    let width = 64u32;
    let height = 64u32;
    let size = 16u32;

    let mut img = image::RgbImage::new(width, height);
    let inv_s = 1.0 / ((size - 1) as f32);

    for y in 0..height {
        for x in 0..width {
            let i = y * width + x;
            let r = i % size;
            let g = (i / size) % size;
            let b = i / (size * size);

            let r_u8 = ((r as f32) * inv_s * 255.0).round() as u8;
            let g_u8 = ((g as f32) * inv_s * 255.0).round() as u8;
            let b_u8 = ((b as f32) * inv_s * 255.0).round() as u8;

            img.put_pixel(x, y, image::Rgb([r_u8, g_u8, b_u8]));
        }
    }

    let mut png_bytes = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut png_bytes);
    image::ImageEncoder::write_image(
        encoder,
        &img,
        width,
        height,
        image::ExtendedColorType::Rgb8,
    )
    .expect("encode hald png");

    let lut = parse_hald_png_lut("hald_16", "Hald 16", &png_bytes).expect("parse hald png");
    assert_eq!(lut.size, 16);
    assert_eq!(lut.data.len(), 16 * 16 * 16);

    let sampled = lut.sample_trilinear([0.5, 0.5, 0.5]);
    assert!((sampled[0] - 0.5).abs() < 0.05);
    assert!((sampled[1] - 0.5).abs() < 0.05);
    assert!((sampled[2] - 0.5).abs() < 0.05);
}

#[test]
fn test_zip_bundle_parsing_and_install() {
    let tmp = TempTestDir::new("zip_test");
    let zip_path = tmp.path.join("luts_bundle.zip");

    // Create a zip with one .cube and one .3dl file
    let file = fs::File::create(&zip_path).expect("create zip");
    let mut zip_writer = zip::ZipWriter::new(file);

    let options = zip::write::SimpleFileOptions::default();
    zip_writer.start_file("cine_warm.cube", options).unwrap();
    zip_writer
        .write_all(
            b"TITLE \"Warm Cinema\"\nLUT_3D_SIZE 2\n0 0 0\n1 0 0\n0 1 0\n1 1 0\n0 0 1\n1 0 1\n0 1 1\n1 1 1\n",
        )
        .unwrap();

    zip_writer.start_file("cine_cool.3dl", options).unwrap();
    zip_writer
        .write_all(
            b"0 0 0\n0 0 1\n0 1 0\n0 1 1\n1 0 0\n1 0 1\n1 1 0\n1 1 1\n",
        )
        .unwrap();

    zip_writer.finish().unwrap();

    let luts = load_lut_from_path(&zip_path).expect("load from zip");
    assert_eq!(luts.len(), 2);
    assert!(luts.iter().any(|l| l.name == "Warm Cinema"));
    assert!(luts.iter().any(|l| l.id == "cine_cool"));

    let target_dir = tmp.path.join("installed_luts");
    let installed = snap_io::install_lut_file_to(&zip_path, &target_dir).expect("install lut file to target");
    assert_eq!(installed.len(), 2);
    assert!(target_dir.join("cine_warm.cube").is_file());
    assert!(target_dir.join("cine_cool.3dl").is_file());

    // Also test install_lut_file with XDG_DATA_HOME overridden to temp
    let xdg_data = tmp.path.join("xdg_data");
    unsafe {
        std::env::set_var("XDG_DATA_HOME", &xdg_data);
    }
    let default_installed = install_lut_file(&zip_path).expect("install lut file");
    assert_eq!(default_installed.len(), 2);
}

#[test]
fn test_built_in_luts_sanity() {
    let luts = built_in_luts();
    assert_eq!(luts.len(), 6);

    let expected_ids = [
        "kodak_portra_400",
        "fuji_velvia_50",
        "fuji_eterna_cinema",
        "teal_orange_cinema",
        "kodak_gold_200",
        "ilford_hp5_bw",
    ];

    for (lut, &expected_id) in luts.iter().zip(expected_ids.iter()) {
        assert_eq!(lut.id, expected_id);
        assert_eq!(lut.size, 33);
        assert_eq!(lut.data.len(), 33 * 33 * 33);
        assert_eq!(lut.input_space, LutColorSpace::Srgb);

        // Sanity check: all voxels are finite and in [0.0, 1.0]
        for px in lut.data.iter() {
            let r = px[0].to_f32();
            let g = px[1].to_f32();
            let b = px[2].to_f32();
            let a = px[3].to_f32();

            assert!(r.is_finite() && (0.0..=1.0).contains(&r));
            assert!(g.is_finite() && (0.0..=1.0).contains(&g));
            assert!(b.is_finite() && (0.0..=1.0).contains(&b));
            assert_eq!(a, 1.0);
        }
    }

    // Verify B&W LUT is purely monochrome (r == g == b for all voxels)
    let hp5 = luts.iter().find(|l| l.id == "ilford_hp5_bw").unwrap();
    for px in hp5.data.iter() {
        let r = px[0].to_f32();
        let g = px[1].to_f32();
        let b = px[2].to_f32();
        assert!((r - g).abs() < 1e-4);
        assert!((g - b).abs() < 1e-4);
    }
}

#[test]
fn test_sidecar_roundtrip() {
    let tmp = TempTestDir::new("sidecar");
    let img_path = tmp.path.join("sample_photo.jpg");
    fs::write(&img_path, b"fake jpeg content").unwrap();

    let expected_sidecar_path = tmp.path.join("sample_photo.jpg.snap");
    assert_eq!(sidecar_path_for(&img_path), expected_sidecar_path);

    // Initial load when no sidecar exists
    let initial = load_sidecar(&img_path).expect("load missing sidecar");
    assert_eq!(initial, None);

    // Create an edit stack
    let mut stack = EditStack::default();
    stack.crop = CropRotateParams {
        quarter_turns_cw: 1,
        flip_h: true,
        ..Default::default()
    };
    stack.push_op(EditOp::TuneImage(TuneParams {
        brightness: 10.0,
        contrast: 15.0,
        highlights: -20.0,
        shadows: 10.0,
        saturation: 5.0,
        warmth: 8.0,
        ambiance: 0.0,
    }));

    let saved_path = save_sidecar(&img_path, &stack).expect("save sidecar");
    assert_eq!(saved_path, expected_sidecar_path);
    assert!(saved_path.is_file());

    let loaded = load_sidecar(&img_path).expect("load saved sidecar");
    assert_eq!(loaded, Some(stack));
}

#[test]
fn test_lut_error_handling() {
    // 1. Missing size header in cube
    let invalid_cube = "TITLE \"Invalid\"\n1.0 1.0 1.0\n";
    assert!(parse_cube_lut("err1", "err1", invalid_cube).is_err());

    // 2. Mismatched entry count in cube
    let cube_bad_count = "LUT_3D_SIZE 2\n0.0 0.0 0.0\n";
    assert!(parse_cube_lut("err2", "err2", cube_bad_count).is_err());

    // 3. Non-cube 3DL data
    let bad_3dl = "0 0 0\n1 1 1\n";
    assert!(parse_3dl_lut("err3", "err3", bad_3dl).is_err());

    // 4. Non-square HaldCLUT
    let mut non_square = image::RgbImage::new(64, 32);
    for p in non_square.pixels_mut() {
        *p = image::Rgb([128, 128, 128]);
    }
    let mut bytes = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut bytes);
    image::ImageEncoder::write_image(
        encoder,
        &non_square,
        64,
        32,
        image::ExtendedColorType::Rgb8,
    )
    .unwrap();
    assert!(parse_hald_png_lut("err4", "err4", &bytes).is_err());
}
