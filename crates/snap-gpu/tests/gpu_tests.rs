use std::sync::Arc;
use half::f16;
use snap_core::curves::CurvesParams;
use snap_core::image_types::{ImageMetadata, LinearImage, Lut3D, LutColorSpace};
use snap_core::mask::LayerMask;
use snap_core::ops::{
    ColorHslParams, CropRotateParams, DetailsParams, EditOp, Lut3dParams, TuneParams,
    WhiteBalanceParams,
};
use snap_core::ops_ext::*;
use snap_core::stack::{EditNode, EditStack};
use snap_gpu::{HeadlessGpu, ViewportParams};

fn create_synthetic_colorchecker(w: u32, h: u32) -> LinearImage {
    let mut pixels = Vec::with_capacity((w * h) as usize);
    let colors = [
        [0.18, 0.18, 0.18, 1.0], // Mid gray
        [0.70, 0.20, 0.15, 1.0], // Reddish
        [0.15, 0.65, 0.20, 1.0], // Greenish
        [0.10, 0.25, 0.75, 1.0], // Blueish
        [0.85, 0.75, 0.20, 1.0], // Yellowish
        [0.60, 0.15, 0.60, 1.0], // Purple
        [0.05, 0.05, 0.05, 1.0], // Dark
        [0.90, 0.90, 0.90, 1.0], // Bright
    ];

    for _y in 0..h {
        for x in 0..w {
            let col_idx = (((x * 8) / w) as usize) % colors.len();
            let c = colors[col_idx];
            pixels.push([
                f16::from_f32(c[0]),
                f16::from_f32(c[1]),
                f16::from_f32(c[2]),
                f16::from_f32(c[3]),
            ]);
        }
    }

    LinearImage::new(w, h, pixels, ImageMetadata::default()).unwrap()
}

#[test]
fn test_adapter_initialization() {
    let gpu = HeadlessGpu::new().expect("Headless GPU initialization should succeed on NVIDIA GeForce RTX 2060 SUPER");
    let info = gpu.adapter.get_info();
    println!("GPU Adapter Name: {}", info.name);
    println!("GPU Backend: {:?}", info.backend);
    println!("GPU Device Type: {:?}", info.device_type);
    assert_eq!(info.backend, wgpu::Backend::Vulkan);
}

#[test]
fn test_identity_neutrality() {
    let mut gpu = HeadlessGpu::new().expect("GPU init");
    let src = create_synthetic_colorchecker(64, 48);

    // 1. Empty EditStack
    let empty_stack = EditStack::default();
    let out_empty = gpu
        .export_full_res(&src, &empty_stack, &[])
        .expect("Export empty stack");

    assert_eq!(out_empty.width, src.width);
    assert_eq!(out_empty.height, src.height);

    let mut max_delta_empty = 0.0f32;
    for i in 0..src.pixels.len() {
        for c in 0..4 {
            let delta = (out_empty.pixels[i][c].to_f32() - src.pixels[i][c].to_f32()).abs();
            if delta > max_delta_empty {
                max_delta_empty = delta;
            }
        }
    }
    println!("Max channel delta (empty stack): {:.6}", max_delta_empty);
    assert!(
        max_delta_empty < 2e-3,
        "Empty stack delta exceeded 2e-3: {}",
        max_delta_empty
    );

    // 2. Default-valued ops stack
    let mut default_stack = EditStack::default();
    default_stack.push_op(EditOp::TuneImage(TuneParams::default()));
    default_stack.push_op(EditOp::Curves(CurvesParams::default()));
    default_stack.push_op(EditOp::ColorHsl(ColorHslParams::default()));

    let id_lut = Lut3D::identity(17);
    default_stack.push_op(EditOp::Lut3d(Lut3dParams {
        lut_id: id_lut.id.clone(),
        lut_name: id_lut.name.clone(),
        strength: 100.0,
        input_space: LutColorSpace::LinearRec2020,
    }));
    default_stack.push_op(EditOp::Details(DetailsParams::default()));

    let out_default = gpu
        .export_full_res(&src, &default_stack, &[id_lut])
        .expect("Export default stack");

    let mut max_delta_default = 0.0f32;
    for i in 0..src.pixels.len() {
        for c in 0..3 {
            let delta = (out_default.pixels[i][c].to_f32() - src.pixels[i][c].to_f32()).abs();
            if delta > max_delta_default {
                max_delta_default = delta;
            }
        }
    }
    println!("Max channel delta (default ops): {:.6}", max_delta_default);
    assert!(
        max_delta_default < 2e-3,
        "Default ops delta exceeded 2e-3: {}",
        max_delta_default
    );
}

#[test]
fn test_3d_lut_parity() {
    let mut gpu = HeadlessGpu::new().expect("GPU init");

    // Create non-trivial 17x17x17 LUT
    let size = 17;
    let base_lut = Lut3D::identity(size);
    let mut mod_data = Vec::with_capacity(base_lut.data.len());
    for px in base_lut.data.iter() {
        let r = px[0].to_f32();
        let g = px[1].to_f32();
        let b = px[2].to_f32();
        let r_out = r.powf(0.85);
        let g_out = g.powf(1.10);
        let b_out = (b * 0.90 + 0.05).min(1.0);
        mod_data.push([
            f16::from_f32(r_out),
            f16::from_f32(g_out),
            f16::from_f32(b_out),
            f16::from_f32(1.0),
        ]);
    }
    let custom_lut = Lut3D {
        id: "custom_17".into(),
        name: "Custom 17x17x17".into(),
        size,
        domain_min: [0.0, 0.0, 0.0],
        domain_max: [1.0, 1.0, 1.0],
        input_space: LutColorSpace::LinearRec2020,
        data: Arc::new(mod_data),
    };

    // Create 8x8 test image with diverse colors
    let w = 8u32;
    let h = 8u32;
    let mut test_pixels = Vec::with_capacity((w * h) as usize);
    for y in 0..h {
        for x in 0..w {
            let r = (x as f32 + 0.5) / (w as f32);
            let g = (y as f32 + 0.5) / (h as f32);
            let b = ((x + y) as f32 + 0.5) / ((w + h) as f32);
            test_pixels.push([
                f16::from_f32(r),
                f16::from_f32(g),
                f16::from_f32(b),
                f16::from_f32(1.0),
            ]);
        }
    }
    let src_img = LinearImage::new(w, h, test_pixels, ImageMetadata::default()).unwrap();

    // 1. LinearRec2020 LUT space parity
    let mut stack_rec2020 = EditStack::default();
    stack_rec2020.push_op(EditOp::Lut3d(Lut3dParams {
        lut_id: custom_lut.id.clone(),
        lut_name: custom_lut.name.clone(),
        strength: 100.0,
        input_space: LutColorSpace::LinearRec2020,
    }));

    let gpu_out = gpu
        .export_full_res(&src_img, &stack_rec2020, &[custom_lut.clone()])
        .expect("Export LUT Rec2020");

    let mut max_err_rec2020 = 0.0f32;
    for i in 0..src_img.pixels.len() {
        let in_rgb = [
            src_img.pixels[i][0].to_f32(),
            src_img.pixels[i][1].to_f32(),
            src_img.pixels[i][2].to_f32(),
        ];
        let cpu_expected = custom_lut.sample_trilinear(in_rgb);
        let gpu_rgb = [
            gpu_out.pixels[i][0].to_f32(),
            gpu_out.pixels[i][1].to_f32(),
            gpu_out.pixels[i][2].to_f32(),
        ];
        for c in 0..3 {
            let err = (gpu_rgb[c] - cpu_expected[c]).abs();
            if err > max_err_rec2020 {
                max_err_rec2020 = err;
            }
        }
    }
    println!("3D LUT Parity Max Error (LinearRec2020): {:.6}", max_err_rec2020);
    assert!(
        max_err_rec2020 < 5e-3,
        "LUT LinearRec2020 error exceeded 5e-3: {}",
        max_err_rec2020
    );

    // 2. Srgb LUT space parity
    let mut srgb_lut = custom_lut.clone();
    srgb_lut.id = "custom_17_srgb".into();
    srgb_lut.input_space = LutColorSpace::Srgb;

    let mut stack_srgb = EditStack::default();
    stack_srgb.push_op(EditOp::Lut3d(Lut3dParams {
        lut_id: srgb_lut.id.clone(),
        lut_name: srgb_lut.name.clone(),
        strength: 100.0,
        input_space: LutColorSpace::Srgb,
    }));

    let gpu_out_srgb = gpu
        .export_full_res(&src_img, &stack_srgb, &[srgb_lut.clone()])
        .expect("Export LUT Srgb");

    let mut max_err_srgb = 0.0f32;
    for i in 0..src_img.pixels.len() {
        let in_rgb = [
            src_img.pixels[i][0].to_f32(),
            src_img.pixels[i][1].to_f32(),
            src_img.pixels[i][2].to_f32(),
        ];
        // CPU pipeline for Srgb space LUT:
        let srgb_lin = snap_core::color::linear_rec2020_to_srgb(in_rgb);
        let srgb_gamma = [
            snap_core::color::linear_to_srgb(srgb_lin[0].max(0.0)),
            snap_core::color::linear_to_srgb(srgb_lin[1].max(0.0)),
            snap_core::color::linear_to_srgb(srgb_lin[2].max(0.0)),
        ];
        let lut_srgb_gamma = srgb_lut.sample_trilinear(srgb_gamma);
        let lut_srgb_lin = [
            snap_core::color::srgb_to_linear(lut_srgb_gamma[0]),
            snap_core::color::srgb_to_linear(lut_srgb_gamma[1]),
            snap_core::color::srgb_to_linear(lut_srgb_gamma[2]),
        ];
        let cpu_expected = snap_core::color::linear_srgb_to_rec2020(lut_srgb_lin);

        let gpu_rgb = [
            gpu_out_srgb.pixels[i][0].to_f32(),
            gpu_out_srgb.pixels[i][1].to_f32(),
            gpu_out_srgb.pixels[i][2].to_f32(),
        ];
        for c in 0..3 {
            let err = (gpu_rgb[c] - cpu_expected[c]).abs();
            if err > max_err_srgb {
                max_err_srgb = err;
            }
        }
    }
    println!("3D LUT Parity Max Error (Srgb): {:.6}", max_err_srgb);
    assert!(
        max_err_srgb < 5e-3,
        "LUT Srgb error exceeded 5e-3: {}",
        max_err_srgb
    );
}

#[test]
fn test_white_balance_eyedropper_neutralization() {
    let mut gpu = HeadlessGpu::new().expect("GPU init");

    // Feed a warm cast pixel
    let warm_cast = [0.45f32, 0.35f32, 0.20f32];
    let wb_params = WhiteBalanceParams::from_neutral_rec2020_sample(warm_cast);
    println!("Calculated WB params: temp={:.2}, tint={:.2}", wb_params.temperature, wb_params.tint);

    // Create 16x16 image with the warm cast
    let w = 16u32;
    let h = 16u32;
    let pixels: Vec<[f16; 4]> = (0..(w * h))
        .map(|_| [
            f16::from_f32(warm_cast[0]),
            f16::from_f32(warm_cast[1]),
            f16::from_f32(warm_cast[2]),
            f16::from_f32(1.0),
        ])
        .collect();
    let src = LinearImage::new(w, h, pixels, ImageMetadata::default()).unwrap();

    let mut stack = EditStack::default();
    stack.push_op(EditOp::WhiteBalance(wb_params));

    let out = gpu.export_full_res(&src, &stack, &[]).expect("Export WB neutralization");

    let p0 = [
        out.pixels[0][0].to_f32(),
        out.pixels[0][1].to_f32(),
        out.pixels[0][2].to_f32(),
    ];
    println!("Neutralized pixel output: R={:.4}, G={:.4}, B={:.4}", p0[0], p0[1], p0[2]);

    let diff_rg = (p0[0] - p0[1]).abs();
    let diff_gb = (p0[1] - p0[2]).abs();
    let diff_rb = (p0[0] - p0[2]).abs();

    assert!(diff_rg < 1e-3, "R-G mismatch: {}", diff_rg);
    assert!(diff_gb < 1e-3, "G-B mismatch: {}", diff_gb);
    assert!(diff_rb < 1e-3, "R-B mismatch: {}", diff_rb);
}

#[test]
fn test_crop_rotate_straighten() {
    let mut gpu = HeadlessGpu::new().expect("GPU init");

    // 1. 90-degree rotate dimension swap
    let w = 200u32;
    let h = 100u32;
    let pixels = vec![[f16::from_f32(0.5); 4]; (w * h) as usize];
    let src = LinearImage::new(w, h, pixels, ImageMetadata::default()).unwrap();

    let mut stack_90 = EditStack::default();
    stack_90.crop = CropRotateParams {
        quarter_turns_cw: 1,
        ..Default::default()
    };

    let out_90 = gpu.export_full_res(&src, &stack_90, &[]).expect("Export 90 deg");
    println!("90 CW output dimensions: {}x{}", out_90.width, out_90.height);
    assert_eq!(out_90.width, 100);
    assert_eq!(out_90.height, 200);

    // 2. 15-degree straighten on solid white: zero black border pixels at 4 corners
    let sw = 120u32;
    let sh = 80u32;
    let white_pixels = vec![[f16::from_f32(1.0); 4]; (sw * sh) as usize];
    let src_white = LinearImage::new(sw, sh, white_pixels, ImageMetadata::default()).unwrap();

    let mut stack_straighten = EditStack::default();
    stack_straighten.crop = CropRotateParams {
        straighten_deg: 15.0,
        ..Default::default()
    };

    let out_straight = gpu
        .export_full_res(&src_white, &stack_straighten, &[])
        .expect("Export 15 deg straighten");

    let ow = out_straight.width as usize;
    let oh = out_straight.height as usize;

    let corners = [
        out_straight.pixels[0],                       // Top-left
        out_straight.pixels[ow - 1],                  // Top-right
        out_straight.pixels[(oh - 1) * ow],           // Bottom-left
        out_straight.pixels[(oh - 1) * ow + (ow - 1)], // Bottom-right
    ];

    for (idx, corner) in corners.iter().enumerate() {
        let r = corner[0].to_f32();
        let g = corner[1].to_f32();
        let b = corner[2].to_f32();
        println!("Corner {} RGB: ({:.3}, {:.3}, {:.3})", idx, r, g, b);
        assert!(r > 0.95, "Corner {} R was black or dimmed: {}", idx, r);
        assert!(g > 0.95, "Corner {} G was black or dimmed: {}", idx, g);
        assert!(b > 0.95, "Corner {} B was black or dimmed: {}", idx, b);
    }
}

#[test]
fn test_gpu_histogram_accuracy() {
    let mut gpu = HeadlessGpu::new().expect("GPU init");
    let w = 128u32;
    let h = 96u32;
    let expected_total = w * h;

    let src = create_synthetic_colorchecker(w, h);
    gpu.engine.upload_source_image(&gpu.device, &gpu.queue, 1, &src);

    let hist = gpu
        .engine
        .compute_histogram_sync(&gpu.device, &gpu.queue)
        .expect("Histogram compute");

    let sum_r: u32 = hist.r.iter().sum();
    let sum_g: u32 = hist.g.iter().sum();
    let sum_b: u32 = hist.b.iter().sum();
    let sum_l: u32 = hist.luma.iter().sum();

    println!(
        "Histogram bin sums: R={}, G={}, B={}, Luma={} (Expected: {})",
        sum_r, sum_g, sum_b, sum_l, expected_total
    );

    assert_eq!(sum_r, expected_total);
    assert_eq!(sum_g, expected_total);
    assert_eq!(sum_b, expected_total);
    assert_eq!(sum_l, expected_total);
}

#[test]
fn test_srgb_vs_nonsrgb_viewport_output() {
    let mut gpu = HeadlessGpu::new().expect("GPU init");
    let w = 64u32;
    let h = 64u32;

    let src = create_synthetic_colorchecker(w, h);
    gpu.engine.upload_source_image(&gpu.device, &gpu.queue, 1, &src);

    let create_target = |format: wgpu::TextureFormat, label: &str| {
        let tex = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
        (tex, view)
    };

    let (tex_srgb, view_srgb) = create_target(wgpu::TextureFormat::Rgba8UnormSrgb, "tex_srgb");
    let (tex_nonsrgb, view_nonsrgb) = create_target(wgpu::TextureFormat::Rgba8Unorm, "tex_nonsrgb");

    let params = ViewportParams {
        zoom: 1.0,
        pan: [0.0, 0.0],
        crop: CropRotateParams::default(),
        crop_preview_mode: false,
    };

    gpu.engine.render_viewport_standalone(
        &gpu.device,
        &gpu.queue,
        &view_srgb,
        wgpu::TextureFormat::Rgba8UnormSrgb,
        [0, 0, w, h],
        &params,
    );

    gpu.engine.render_viewport_standalone(
        &gpu.device,
        &gpu.queue,
        &view_nonsrgb,
        wgpu::TextureFormat::Rgba8Unorm,
        [0, 0, w, h],
        &params,
    );

    let read_u8_tex = |tex: &wgpu::Texture| -> Vec<u8> {
        let unpadded = w * 4;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded = ((unpadded + align - 1) / align) * align;
        let staging = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("u8_staging"),
            size: (padded * h) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut encoder = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &staging,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: Some(h),
                },
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        gpu.queue.submit(Some(encoder.finish()));

        let slice = staging.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |res| { let _ = tx.send(res); });
        let _ = gpu.device.poll(wgpu::PollType::wait_indefinitely());
        rx.recv().unwrap().unwrap();

        let mapped = slice.get_mapped_range();
        let mut bytes = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            let start = (y * padded) as usize;
            let end = start + (unpadded as usize);
            bytes.extend_from_slice(&mapped[start..end]);
        }
        drop(mapped);
        staging.unmap();
        bytes
    };

    let bytes_srgb = read_u8_tex(&tex_srgb);
    let bytes_nonsrgb = read_u8_tex(&tex_nonsrgb);

    assert_eq!(bytes_srgb.len(), bytes_nonsrgb.len());
    let mut max_byte_diff = 0i32;
    for i in 0..bytes_srgb.len() {
        let diff = (bytes_srgb[i] as i32 - bytes_nonsrgb[i] as i32).abs();
        if diff > max_byte_diff {
            max_byte_diff = diff;
        }
    }

    println!("Max byte diff between sRGB and non-sRGB output: {}", max_byte_diff);
    assert!(
        max_byte_diff <= 1,
        "Byte discrepancy between sRGB and non-sRGB exceeded ±1: {}",
        max_byte_diff
    );
}

#[test]
fn test_screen_pos_to_image_uv() {
    let clip = [0, 0, 800, 600];
    let params = ViewportParams {
        zoom: 1.0,
        pan: [0.0, 0.0],
        crop: CropRotateParams::default(),
        crop_preview_mode: false,
    };

    // Center click
    let center_uv = snap_gpu::GpuPipelineEngine::screen_pos_to_image_uv([400.0, 300.0], clip, 400, 300, &params);
    assert!(center_uv.is_some());
    let uv = center_uv.unwrap();
    assert!((uv[0] - 0.5).abs() < 1e-3);
    assert!((uv[1] - 0.5).abs() < 1e-3);

    // Outside click returns None for narrow aspect: 400x800 image in 800x400 viewport
    let narrow_uv = snap_gpu::GpuPipelineEngine::screen_pos_to_image_uv([10.0, 200.0], [0, 0, 800, 400], 400, 800, &params);
    assert!(narrow_uv.is_none(), "Far left click outside narrow image should be None");
}

#[test]
fn test_details_pass1_pass2_isolation() {
    let mut gpu = HeadlessGpu::new().expect("GPU init");
    let w = 32u32;
    let h = 32u32;
    let pixels = vec![[f16::from_f32(0.5); 4]; (w * h) as usize];
    let src = LinearImage::new(w, h, pixels, ImageMetadata::default()).unwrap();

    let mut stack = EditStack::default();
    stack.push_op(EditOp::Details(DetailsParams {
        structure: 40.0,
        sharpening: 30.0,
    }));

    let out = gpu.export_full_res(&src, &stack, &[]).expect("Export Details");
    assert_eq!(out.width, w);
    assert_eq!(out.height, h);

    // If Pass 1 uniform buffer was overwritten by Pass 2 before queue.submit,
    // pass_idx was 1 during Pass 1, reading alpha=1.0 as luma_base and crushing mid-gray to 0.0 (pitch black).
    for i in 0..out.pixels.len() {
        let r = out.pixels[i][0].to_f32();
        let g = out.pixels[i][1].to_f32();
        let b = out.pixels[i][2].to_f32();
        assert!(r > 0.45 && r < 0.55, "Details crushed pixel {} to black or invalid: R={}", i, r);
        assert!(g > 0.45 && g < 0.55, "Details crushed pixel {} to black or invalid: G={}", i, g);
        assert!(b > 0.45 && b < 0.55, "Details crushed pixel {} to black or invalid: B={}", i, b);
    }
}

#[test]
fn test_multi_layer_same_op_isolation() {
    let mut gpu = HeadlessGpu::new().expect("GPU init");
    let w = 16u32;
    let h = 16u32;
    let pixels = vec![[f16::from_f32(0.2); 4]; (w * h) as usize];
    let src = LinearImage::new(w, h, pixels, ImageMetadata::default()).unwrap();

    let mut stack = EditStack::default();
    // Two sequential TuneImage layers with different params
    stack.push_op(EditOp::TuneImage(TuneParams {
        brightness: 20.0,
        ..Default::default()
    }));
    stack.push_op(EditOp::TuneImage(TuneParams {
        brightness: 15.0,
        ..Default::default()
    }));

    let out = gpu.export_full_res(&src, &stack, &[]).expect("Export Multi-layer TuneImage");
    let p0_r = out.pixels[0][0].to_f32();
    // 0.2 with two brightness boosts must be noticeably brighter than single boost
    println!("Multi-layer TuneImage output R: {:.4}", p0_r);
    assert!(p0_r > 0.25, "Multi-layer TuneImage failed to accumulate brightness: {}", p0_r);
}

#[test]
fn test_hdr_curves_no_solarization() {
    let mut gpu = HeadlessGpu::new().expect("GPU init");
    let w = 8u32;
    let h = 8u32;
    // Bright HDR highlight pixel (e.g. sun core at 3.0)
    let pixels = vec![[f16::from_f32(3.0); 4]; (w * h) as usize];
    let src = LinearImage::new(w, h, pixels, ImageMetadata::default()).unwrap();

    // Default identity curve
    let mut stack = EditStack::default();
    let mut curve_params = CurvesParams::default();
    // Add a tiny dummy node or ensure it evaluates non-identity so process_stack runs it
    // Or set a control point: (0.5, 0.5) which preserves identity
    curve_params.rgb.points = vec![
        snap_core::curves::CurvePoint::new(0.0, 0.0),
        snap_core::curves::CurvePoint::new(0.5, 0.52),
        snap_core::curves::CurvePoint::new(1.0, 1.0),
    ];
    stack.push_op(EditOp::Curves(curve_params));

    let out = gpu.export_full_res(&src, &stack, &[]).expect("Export HDR Curves");
    let p0_r = out.pixels[0][0].to_f32();
    let p0_g = out.pixels[0][1].to_f32();
    let p0_b = out.pixels[0][2].to_f32();

    println!("HDR Curves output: R={:.4}, G={:.4}, B={:.4}", p0_r, p0_g, p0_b);
    // Before fix: clamped curve gave ratio = 1.0 / 1.6 = 0.625, turning 3.0 into a dark hole ~0.35!
    // With fix: slope continuity preserves HDR headroom >= 2.9
    assert!(p0_r > 2.8, "HDR highlight solarized or clipped by Curves: R={}", p0_r);
    assert!(p0_g > 2.8, "HDR highlight solarized or clipped by Curves: G={}", p0_g);
    assert!(p0_b > 2.8, "HDR highlight solarized or clipped by Curves: B={}", p0_b);
}

#[test]
fn test_hdr_color_hsl_no_negative_saturation() {
    let mut gpu = HeadlessGpu::new().expect("GPU init");
    let w = 8u32;
    let h = 8u32;
    // HDR pixel with c_max + c_min > 2.0
    let pixels = vec![[
        f16::from_f32(2.5),
        f16::from_f32(1.2),
        f16::from_f32(0.9),
        f16::from_f32(1.0),
    ]; (w * h) as usize];
    let src = LinearImage::new(w, h, pixels, ImageMetadata::default()).unwrap();

    let mut stack = EditStack::default();
    let mut hsl_params = ColorHslParams::default();
    // Adjust hue of red band slightly so it's not identity
    hsl_params.bands[0].hue = 5.0;
    stack.push_op(EditOp::ColorHsl(hsl_params));

    let out = gpu.export_full_res(&src, &stack, &[]).expect("Export HDR ColorHsl");
    let p0_r = out.pixels[0][0].to_f32();
    let p0_g = out.pixels[0][1].to_f32();
    let p0_b = out.pixels[0][2].to_f32();

    println!("HDR ColorHsl output: R={:.4}, G={:.4}, B={:.4}", p0_r, p0_g, p0_b);
    // Before fix: c_max + c_min > 2.0 produced negative saturation and crushed / distorted colors
    assert!(p0_r > 2.0, "HDR red channel distorted: {}", p0_r);
    assert!(p0_g > 0.8, "HDR green channel distorted: {}", p0_g);
    assert!(p0_b > 0.6, "HDR blue channel distorted: {}", p0_b);
}

fn assert_finite_and_transformed(out: &LinearImage, src: &LinearImage, op_name: &str) {
    assert_eq!(out.width, src.width);
    assert_eq!(out.height, src.height);
    let mut max_delta = 0.0f32;
    for i in 0..out.pixels.len() {
        for c in 0..4 {
            let val = out.pixels[i][c].to_f32();
            assert!(!val.is_nan(), "{}: pixel {} channel {} is NaN", op_name, i, c);
            assert!(val.is_finite(), "{}: pixel {} channel {} is infinite", op_name, i, c);
            if c < 3 {
                let delta = (val - src.pixels[i][c].to_f32()).abs();
                if delta > max_delta {
                    max_delta = delta;
                }
            } else {
                // Alpha must remain 1.0
                assert!((val - 1.0).abs() < 1e-2, "{}: pixel {} alpha is {} (expected 1.0)", op_name, i, val);
            }
        }
    }
    assert!(
        max_delta > 0.005,
        "{}: max delta {:.6} is too small (op produced no visible transformation)",
        op_name,
        max_delta
    );
}

#[test]
fn test_all_25_phase2_4_tools() {
    let mut gpu = HeadlessGpu::new().expect("GPU init");
    let src = create_synthetic_colorchecker(64, 48);

    // Upload secondary image for DoubleExposure
    let sec_pixels = vec![[f16::from_f32(0.1), f16::from_f32(0.8), f16::from_f32(0.2), f16::from_f32(1.0)]; 64 * 48];
    let sec_img = LinearImage::new(64, 48, sec_pixels, ImageMetadata::default()).unwrap();
    gpu.upload_secondary_image("sec_test", &sec_img);

    let mut brush_map = snap_core::mask::BrushMap::new();
    brush_map.paint_dab(0.5, 0.5, 0.25, 0.5, 0.8, false);

    let tools: Vec<(&str, EditOp)> = vec![
        ("ColorGrade", EditOp::ColorGrade({
            let mut p = ColorGradeParams::default();
            p.shadows.saturation = 40.0;
            p.shadows.hue_deg = 220.0;
            p.balance = 10.0;
            p
        })),
        ("Dehaze", EditOp::Dehaze({
            let mut p = DehazeParams::default();
            p.strength = 50.0;
            p
        })),
        ("TonalContrast", EditOp::TonalContrast({
            let mut p = TonalContrastParams::default();
            p.high_tones = 30.0;
            p.mid_tones = 20.0;
            p
        })),
        ("Perspective", EditOp::Perspective({
            let mut p = PerspectiveParams::default();
            p.vertical = 25.0;
            p.horizontal = 15.0;
            p
        })),
        ("Expand", EditOp::Expand({
            let mut p = ExpandParams::default();
            p.top = 0.15;
            p.bottom = 0.15;
            p.fill_mode = ExpandFillMode::White;
            p
        })),
        ("Selective", EditOp::Selective({
            let mut p = SelectiveParams::default();
            let mut pin = SelectivePin::default();
            pin.brightness = 40.0;
            pin.radius = 0.5;
            p.pins = vec![pin];
            p
        })),
        ("Brush", EditOp::Brush({
            let mut p = BrushParams::default();
            p.channel = BrushChannel::DodgeBurn;
            p.map = brush_map;
            p
        })),
        ("Healing", EditOp::Healing({
            let mut p = HealingParams::default();
            p.strokes.push(HealStroke {
                points: vec![[0.5, 0.5]],
                radius: 0.15,
                source_offset: Some([0.1, 0.1]),
            });
            p
        })),
        ("Halation", EditOp::Halation({
            let mut p = HalationParams::default();
            p.threshold = 0.1;
            p.strength = 80.0;
            p.spread = 40.0;
            p
        })),
        ("Bloom", EditOp::Bloom({
            let mut p = BloomParams::default();
            p.strength = 60.0;
            p.size = 50.0;
            p.highlight_range = 30.0;
            p
        })),
        ("GrainyFilm", EditOp::GrainyFilm({
            let mut p = GrainyFilmParams::default();
            p.stock = FilmStock::Velvia50;
            p.grain = 40.0;
            p.style_strength = 60.0;
            p
        })),
        ("Noir", EditOp::Noir({
            let mut p = NoirParams::default();
            p.preset = NoirPreset::Film;
            p.wash = 30.0;
            p
        })),
        ("HdrScape", EditOp::HdrScape({
            let mut p = HdrScapeParams::default();
            p.filter_strength = 50.0;
            p.brightness = 15.0;
            p
        })),
        ("GlamourGlow", EditOp::GlamourGlow({
            let mut p = GlamourGlowParams::default();
            p.glow = 60.0;
            p.saturation = 20.0;
            p
        })),
        ("Drama", EditOp::Drama({
            let mut p = DramaParams::default();
            p.preset = DramaPreset::Drama1;
            p.filter_strength = 60.0;
            p
        })),
        ("Vintage", EditOp::Vintage({
            let mut p = VintageParams::default();
            p.style = 3;
            p.style_strength = 60.0;
            p
        })),
        ("LensBlur", EditOp::LensBlur({
            let mut p = LensBlurParams::default();
            p.blur_strength = 60.0;
            p.bokeh_boost = 50.0;
            p
        })),
        ("MotionBlur", EditOp::MotionBlur({
            let mut p = MotionBlurParams::default();
            p.strength = 40.0;
            p.angle_deg = 45.0;
            p
        })),
        ("DoubleExposure", EditOp::DoubleExposure({
            let mut p = DoubleExposureParams::default();
            p.secondary_path = Some("sec_test".into());
            p.blend_mode = DoubleExposureBlend::Blend;
            p.opacity = 60.0;
            p
        })),
        ("Retrolux", EditOp::Retrolux({
            let mut p = RetroluxParams::default();
            p.style = 2;
            p.style_strength = 60.0;
            p.light_leaks = 40.0;
            p
        })),
        ("Grunge", EditOp::Grunge({
            let mut p = GrungeParams::default();
            p.style = 1;
            p.texture_strength = 50.0;
            p
        })),
        ("Portrait", EditOp::Portrait({
            let mut p = PortraitParams::default();
            p.skin_smoothing = 50.0;
            p.eye_clarity = 40.0;
            p
        })),
        ("HeadPose", EditOp::HeadPose({
            let mut p = HeadPoseParams::default();
            p.yaw_x = 30.0;
            p.pitch_y = 20.0;
            p.smile = 30.0;
            p
        })),
        ("Frames", EditOp::Frames({
            let mut p = FramesParams::default();
            p.width = 15.0;
            p
        })),
        ("TextOverlayNormal", EditOp::TextOverlay({
            let mut p = TextOverlayParams::default();
            p.text = "SNAP".to_string();
            p.opacity = 100.0;
            p.scale = 0.5;
            p.invert_mask = false;
            p
        })),
        ("TextOverlayKnockout", EditOp::TextOverlay({
            let mut p = TextOverlayParams::default();
            p.text = "SNAP".to_string();
            p.opacity = 100.0;
            p.scale = 0.5;
            p.invert_mask = true;
            p
        })),
    ];

    for (name, op) in tools {
        let mut stack = EditStack::default();
        stack.push_op(op);
        let out = gpu
            .export_full_res(&src, &stack, &[])
            .unwrap_or_else(|e| panic!("Export failed for {}: {:?}", name, e));

        assert_finite_and_transformed(&out, &src, name);
    }
}

#[test]
fn test_2pass_separable_isolation() {
    let mut gpu = HeadlessGpu::new().expect("GPU init");
    let src = create_synthetic_colorchecker(32, 32);

    let ops: Vec<(&str, EditOp)> = vec![
        ("Halation", EditOp::Halation(HalationParams { threshold: 0.1, strength: 80.0, spread: 40.0 })),
        ("Bloom", EditOp::Bloom(BloomParams { strength: 60.0, size: 50.0, highlight_range: 30.0 })),
        ("TonalContrast", EditOp::TonalContrast(TonalContrastParams { high_tones: 30.0, mid_tones: 20.0, low_tones: 10.0, protect_shadows: 10.0, protect_highlights: 10.0 })),
        ("HdrScape", EditOp::HdrScape(HdrScapeParams { filter_strength: 50.0, brightness: 15.0, ..Default::default() })),
        ("GlamourGlow", EditOp::GlamourGlow(GlamourGlowParams { glow: 60.0, saturation: 20.0, ..Default::default() })),
    ];

    for (name, op) in ops {
        let mut stack = EditStack::default();
        stack.push_op(op);
        let out = gpu.export_full_res(&src, &stack, &[]).unwrap_or_else(|e| panic!("Export failed for {}: {:?}", name, e));
        for i in 0..out.pixels.len() {
            let a = out.pixels[i][3].to_f32();
            assert!((a - 1.0).abs() < 1e-2, "{}: pixel {} alpha is {} (expected 1.0)", name, i, a);
            for c in 0..3 {
                let v = out.pixels[i][c].to_f32();
                assert!(!v.is_nan() && v.is_finite(), "{}: pixel {} channel {} is not finite: {}", name, i, c, v);
            }
        }
    }
}

#[test]
fn test_layer_mask_blending_and_half_split() {
    let mut gpu = HeadlessGpu::new().expect("GPU init");
    let w = 64u32;
    let h = 48u32;
    let src = create_synthetic_colorchecker(w, h);

    // Create half-split mask: left half (x < 128 in 256x256 coordinate) is 0, right half (x >= 128) is 255
    let mut mask_bytes = vec![0u8; 256 * 256];
    for y in 0..256 {
        for x in 128..256 {
            mask_bytes[y * 256 + x] = 255;
        }
    }
    let half_mask = LayerMask::from_bytes(mask_bytes).unwrap();

    // 1. Test half-split on 1-pass op (TuneImage brightness boost)
    {
        let mut stack = EditStack::default();
        let mut node = EditNode::new(EditOp::TuneImage(TuneParams {
            brightness: 50.0,
            ..Default::default()
        }));
        node.mask = Some(half_mask.clone());
        stack.nodes.push(node);

        let out = gpu.export_full_res(&src, &stack, &[]).expect("Export 1-pass masked op");
        for y in 0..h {
            for x in 0..w {
                let idx = (y * w + x) as usize;
                let src_px = src.pixels[idx];
                let out_px = out.pixels[idx];
                // Left half (x < 24) should be untouched
                if x < 24 {
                    for c in 0..3 {
                        let delta = (out_px[c].to_f32() - src_px[c].to_f32()).abs();
                        assert!(delta < 2e-3, "Left half pixel ({}, {}) modified: delta={}", x, y, delta);
                    }
                }
                // Right half (x >= 40) should be brightened
                if x >= 40 {
                    let out_luma = out_px[0].to_f32() + out_px[1].to_f32() + out_px[2].to_f32();
                    let src_luma = src_px[0].to_f32() + src_px[1].to_f32() + src_px[2].to_f32();
                    assert!(out_luma > src_luma + 0.05, "Right half pixel ({}, {}) not brightened", x, y);
                }
            }
        }
    }

    // 2. Test half-split on 2-pass separable op (Halation)
    {
        let mut stack = EditStack::default();
        let mut node = EditNode::new(EditOp::Halation(HalationParams {
            threshold: 0.1,
            strength: 90.0,
            spread: 50.0,
        }));
        node.mask = Some(half_mask.clone());
        stack.nodes.push(node);

        let out = gpu.export_full_res(&src, &stack, &[]).expect("Export 2-pass masked Halation");
        for y in 0..h {
            for x in 0..w {
                let idx = (y * w + x) as usize;
                let src_px = src.pixels[idx];
                let out_px = out.pixels[idx];
                if x < 24 {
                    for c in 0..3 {
                        let delta = (out_px[c].to_f32() - src_px[c].to_f32()).abs();
                        assert!(delta < 2e-3, "Halation left half pixel ({}, {}) modified: delta={}", x, y, delta);
                    }
                }
            }
        }
    }

    // 3. Test 0% uniform mask leaves entire image untouched
    {
        let mut stack = EditStack::default();
        let mut node = EditNode::new(EditOp::TuneImage(TuneParams {
            brightness: 50.0,
            ..Default::default()
        }));
        node.mask = Some(LayerMask::empty());
        stack.nodes.push(node);

        let out = gpu.export_full_res(&src, &stack, &[]).expect("Export empty mask");
        for i in 0..out.pixels.len() {
            for c in 0..3 {
                let delta = (out.pixels[i][c].to_f32() - src.pixels[i][c].to_f32()).abs();
                assert!(delta < 2e-3, "0% mask pixel {} modified: delta={}", i, delta);
            }
        }
    }

    // 4. Test 50% uniform mask blends between pre and post
    {
        let half_bytes = vec![128u8; 256 * 256];
        let mid_mask = LayerMask::from_bytes(half_bytes).unwrap();

        let mut stack_full = EditStack::default();
        let mut node_full = EditNode::new(EditOp::TuneImage(TuneParams {
            brightness: 50.0,
            ..Default::default()
        }));
        node_full.mask = Some(LayerMask::full());
        stack_full.nodes.push(node_full);
        let out_full = gpu.export_full_res(&src, &stack_full, &[]).expect("Export full mask");

        let mut stack_mid = EditStack::default();
        let mut node_mid = EditNode::new(EditOp::TuneImage(TuneParams {
            brightness: 50.0,
            ..Default::default()
        }));
        node_mid.mask = Some(mid_mask);
        stack_mid.nodes.push(node_mid);
        let out_mid = gpu.export_full_res(&src, &stack_mid, &[]).expect("Export 50% mask");

        let p_src = src.pixels[0][0].to_f32();
        let p_full = out_full.pixels[0][0].to_f32();
        let p_mid = out_mid.pixels[0][0].to_f32();
        assert!(p_mid > p_src + 0.01 && p_mid < p_full - 0.01, "50% mask value {} not between src {} and full {}", p_mid, p_src, p_full);
    }
}

#[test]
fn test_hdr_highlight_safety_phase2_4() {
    let mut gpu = HeadlessGpu::new().expect("GPU init");
    let w = 16u32;
    let h = 16u32;
    // Bright HDR highlight pixel (e.g. sun at 3.2)
    let pixels = vec![[f16::from_f32(3.2); 4]; (w * h) as usize];
    let src = LinearImage::new(w, h, pixels, ImageMetadata::default()).unwrap();

    let ops: Vec<(&str, EditOp)> = vec![
        ("ColorGrade", EditOp::ColorGrade(ColorGradeParams { balance: 10.0, ..Default::default() })),
        ("Halation", EditOp::Halation(HalationParams { threshold: 0.5, strength: 50.0, spread: 20.0 })),
        ("Bloom", EditOp::Bloom(BloomParams { strength: 40.0, size: 30.0, highlight_range: 20.0 })),
        ("GrainyFilm", EditOp::GrainyFilm(GrainyFilmParams { stock: FilmStock::K100, grain: 20.0, style_strength: 30.0 })),
        ("HdrScape", EditOp::HdrScape(HdrScapeParams { filter_strength: 30.0, brightness: 10.0, ..Default::default() })),
        ("LensBlur", EditOp::LensBlur(LensBlurParams { blur_strength: 30.0, bokeh_boost: 20.0, ..Default::default() })),
    ];

    for (name, op) in ops {
        let mut stack = EditStack::default();
        stack.push_op(op);
        let out = gpu.export_full_res(&src, &stack, &[]).unwrap_or_else(|e| panic!("Export HDR failed for {}: {:?}", name, e));
        for i in 0..out.pixels.len() {
            for c in 0..3 {
                let v = out.pixels[i][c].to_f32();
                assert!(!v.is_nan() && v.is_finite(), "{}: pixel {} channel {} is not finite: {}", name, i, c, v);
                assert!(v > 1.0, "{}: pixel {} channel {} solarized or crushed to <= 1.0: {}", name, i, c, v);
            }
        }
    }
}

#[test]
fn test_batch_multi_image_export_regression() {
    let mut gpu = HeadlessGpu::new().expect("GPU init");
    let w = 32u32;
    let h = 32u32;

    // Image 1: Red
    let red_pixels = vec![[f16::from_f32(1.0), f16::from_f32(0.0), f16::from_f32(0.0), f16::from_f32(1.0)]; (w * h) as usize];
    let img1 = LinearImage::new(w, h, red_pixels, ImageMetadata::default()).unwrap();

    // Image 2: Blue
    let blue_pixels = vec![[f16::from_f32(0.0), f16::from_f32(0.0), f16::from_f32(1.0), f16::from_f32(1.0)]; (w * h) as usize];
    let img2 = LinearImage::new(w, h, blue_pixels, ImageMetadata::default()).unwrap();

    let mut stack = EditStack::default();
    stack.push_op(EditOp::TuneImage(TuneParams {
        brightness: 10.0,
        ..Default::default()
    }));

    // Sequential exports on the SAME HeadlessGpu instance
    let out1 = gpu.export_full_res(&img1, &stack, &[]).expect("Export img1");
    let out2 = gpu.export_full_res(&img2, &stack, &[]).expect("Export img2");

    let p1_r = out1.pixels[0][0].to_f32();
    let p1_b = out1.pixels[0][2].to_f32();
    assert!(p1_r > 0.8, "Image 1 red channel is crushed: {}", p1_r);
    assert!(p1_b < 0.2, "Image 1 blue channel contaminated: {}", p1_b);

    let p2_r = out2.pixels[0][0].to_f32();
    let p2_b = out2.pixels[0][2].to_f32();
    assert!(p2_b > 0.8, "Image 2 blue channel is crushed (stale cache from Image 1?): {}", p2_b);
    assert!(p2_r < 0.2, "Image 2 red channel contaminated (stale cache from Image 1?): {}", p2_r);
}
