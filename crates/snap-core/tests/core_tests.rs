use std::sync::Arc;
use snap_core::*;

#[test]
fn test_rec2020_srgb_roundtrip() {
    let test_colors = [
        [0.0f32, 0.0, 0.0],
        [1.0, 1.0, 1.0],
        [0.5, 0.5, 0.5],
        [0.8, 0.2, 0.1],
        [0.1, 0.9, 0.2],
        [0.05, 0.15, 0.85],
        [0.25, 0.45, 0.65],
    ];

    for &rgb in &test_colors {
        let rec2020 = linear_srgb_to_rec2020(rgb);
        let roundtrip = linear_rec2020_to_srgb(rec2020);

        for c in 0..3 {
            let diff = (roundtrip[c] - rgb[c]).abs();
            assert!(
                diff < 1e-5,
                "sRGB <-> Rec.2020 failed for {:?}: roundtrip is {:?}, diff={diff}",
                rgb,
                roundtrip
            );
        }
    }
}

#[test]
fn test_srgb_transfer_roundtrip() {
    for i in 0..=100 {
        let c = i as f32 / 100.0;
        let lin = srgb_to_linear(c);
        let srgb = linear_to_srgb(lin);
        assert!((srgb - c).abs() < 1e-5, "sRGB transfer failed for {c}: got {srgb}");
    }
}

#[test]
fn test_rec2020_oklab_roundtrip() {
    let test_colors = [
        [0.1f32, 0.2, 0.3],
        [0.5, 0.5, 0.5],
        [0.9, 0.1, 0.05],
        [0.2, 0.8, 0.3],
        [0.1, 0.3, 0.7],
        [0.0, 0.0, 0.0],
        [1.0, 1.0, 1.0],
    ];

    for &rgb in &test_colors {
        let oklab = linear_rec2020_to_oklab(rgb);
        let roundtrip = oklab_to_linear_rec2020(oklab);

        for c in 0..3 {
            let diff = (roundtrip[c] - rgb[c]).abs();
            assert!(
                diff < 1e-4,
                "Rec.2020 <-> Oklab failed for {:?}: roundtrip is {:?}, diff={diff}",
                rgb,
                roundtrip
            );
        }

        // Test Oklab <-> Oklch roundtrip
        let oklch = oklab_to_oklch(oklab);
        let oklab_roundtrip = oklch_to_oklab(oklch);
        for c in 0..3 {
            let diff = (oklab_roundtrip[c] - oklab[c]).abs();
            assert!(
                diff < 1e-4,
                "Oklab <-> Oklch failed for {:?}: roundtrip is {:?}, diff={diff}",
                oklab,
                oklab_roundtrip
            );
        }
    }
}

#[test]
fn test_rec2020_luma() {
    assert!((rec2020_luma([1.0, 0.0, 0.0]) - 0.2627).abs() < 1e-5);
    assert!((rec2020_luma([0.0, 1.0, 0.0]) - 0.6780).abs() < 1e-5);
    assert!((rec2020_luma([0.0, 0.0, 1.0]) - 0.0593).abs() < 1e-5);
    assert!((rec2020_luma([1.0, 1.0, 1.0]) - 1.0000).abs() < 1e-4);
}

#[test]
fn test_fritsch_carlson_monotonicity_and_identity() {
    // 1. Identity curve test
    let identity = CurveChannel::default();
    assert!(identity.is_identity());
    for i in 0..=100 {
        let x = i as f32 / 100.0;
        let y = identity.eval(x);
        assert!(
            (y - x).abs() < 1e-4,
            "Identity curve eval({x}) = {y}, expected {x}"
        );
    }

    // 2. Step input test: points flat at 0, step to 1, flat at 1
    let step_curve = CurveChannel::new(vec![
        CurvePoint::new(0.0, 0.0),
        CurvePoint::new(0.3, 0.0),
        CurvePoint::new(0.7, 1.0),
        CurvePoint::new(1.0, 1.0),
    ]);

    // Flat bottom region must have zero overshoot/undershoot
    for i in 0..=30 {
        let x = i as f32 / 100.0;
        let y = step_curve.eval(x);
        assert_eq!(y, 0.0, "Step curve under flat bottom failed at {x}: {y}");
    }

    // Flat top region must have zero overshoot
    for i in 70..=100 {
        let x = i as f32 / 100.0;
        let y = step_curve.eval(x);
        assert_eq!(y, 1.0, "Step curve under flat top failed at {x}: {y}");
    }

    // Interior region must be strictly non-decreasing (monotonic)
    let mut prev_y = 0.0f32;
    for i in 0..=1000 {
        let x = i as f32 / 1000.0;
        let y = step_curve.eval(x);
        assert!(
            y >= prev_y - 1e-6,
            "Monotonicity violated at x={x}: y={y} < prev={prev_y}"
        );
        assert!(y >= 0.0 && y <= 1.0, "Value out of bounds at x={x}: {y}");
        prev_y = y;
    }
}

#[test]
fn test_curve_presets_and_lut_bake() {
    for &preset in CurvePreset::ALL {
        let params = preset.to_curves_params();
        let lut = params.bake_lut_1024();
        assert_eq!(lut.len(), 1024);
        // Start and end voxels should be bounded
        for entry in &lut {
            for c in 0..4 {
                let v = entry[c].to_f32();
                assert!(v >= 0.0 && v <= 1.0, "Bake LUT value out of bounds: {v}");
            }
        }
    }
}

#[test]
fn test_inscribed_crop_rectangle() {
    let angles = [0.0f32, 15.0, 45.0, -15.0, -45.0];
    let aspect_ratios = [1.0f32, 1.5, 4.0 / 3.0, 16.0 / 9.0, 0.75];

    for &aspect in &aspect_ratios {
        let w = aspect;
        let h = 1.0f32;

        for &angle in &angles {
            let scale = CropRotateParams::inscribed_max_scale(aspect, angle);
            assert!(scale > 0.0 && scale <= 1.0 + 1e-6);

            if angle.abs() < 1e-4 {
                assert!(
                    (scale - 1.0).abs() < 1e-5,
                    "Scale at 0 deg should be 1.0, got {scale}"
                );
            }

            // Test that all 4 corners of the inscribed box (w_in, h_in)
            // rotated by -angle stay within the unrotated boundary [-w/2, w/2] x [-h/2, h/2]
            let w_in = scale * w;
            let h_in = scale * h;

            let rad = angle.to_radians();
            let cos_t = rad.cos();
            let sin_t = rad.sin();

            let half_w = w * 0.5;
            let half_h = h * 0.5;

            let corners = [
                (-w_in * 0.5, -h_in * 0.5),
                (w_in * 0.5, -h_in * 0.5),
                (w_in * 0.5, h_in * 0.5),
                (-w_in * 0.5, h_in * 0.5),
            ];

            for &(cx, cy) in &corners {
                // Rotate by -angle into unrotated frame
                let rx = cx * cos_t + cy * sin_t;
                let ry = -cx * sin_t + cy * cos_t;

                assert!(
                    rx.abs() <= half_w + 1e-4,
                    "Corner X={rx} exceeded half_w={half_w} at aspect={aspect}, angle={angle}, scale={scale}"
                );
                assert!(
                    ry.abs() <= half_h + 1e-4,
                    "Corner Y={ry} exceeded half_h={half_h} at aspect={aspect}, angle={angle}, scale={scale}"
                );
            }
        }
    }
}

#[test]
fn test_white_balance_neutral_eyedropper() {
    // 1. Perfectly neutral gray sample should produce 0 shift
    let neutral = WhiteBalanceParams::from_neutral_rec2020_sample([0.5, 0.5, 0.5]);
    assert!(neutral.temperature.abs() < 1e-4);
    assert!(neutral.tint.abs() < 1e-4);

    // 2. Warm yellow/red sample should produce negative temperature (cool shift)
    let warm_sample = [0.65, 0.50, 0.35];
    let cool_adjust = WhiteBalanceParams::from_neutral_rec2020_sample(warm_sample);
    assert!(
        cool_adjust.temperature < -10.0,
        "Warm sample should produce negative temp, got {}",
        cool_adjust.temperature
    );

    // 3. Cool blue sample should produce positive temperature (warm shift)
    let cool_sample = [0.35, 0.50, 0.65];
    let warm_adjust = WhiteBalanceParams::from_neutral_rec2020_sample(cool_sample);
    assert!(
        warm_adjust.temperature > 10.0,
        "Cool sample should produce positive temp, got {}",
        warm_adjust.temperature
    );

    // 4. Green sample should produce positive tint (magenta shift)
    let green_sample = [0.4, 0.7, 0.4];
    let magenta_adjust = WhiteBalanceParams::from_neutral_rec2020_sample(green_sample);
    assert!(
        magenta_adjust.tint > 10.0,
        "Green sample should produce positive tint, got {}",
        magenta_adjust.tint
    );
}

#[test]
fn test_edit_stack_and_history_manager() {
    let mut stack = EditStack::default();
    assert!(stack.is_empty());

    let op1 = EditOp::TuneImage(TuneParams {
        brightness: 10.0,
        contrast: 15.0,
        ..Default::default()
    });
    let id1 = stack.push_op(op1);
    assert!(!stack.is_empty());
    assert_eq!(stack.nodes.len(), 1);

    let op2 = EditOp::Details(DetailsParams {
        structure: 20.0,
        sharpening: 30.0,
    });
    let id2 = stack.push_op(op2);
    assert_eq!(stack.nodes.len(), 2);

    // Test toggle
    let state = stack.toggle_node(id1);
    assert_eq!(state, Some(false));
    assert_eq!(stack.active_ops_up_to(None).count(), 1);

    // Re-enable
    stack.toggle_node(id1);
    assert_eq!(stack.active_ops_up_to(None).count(), 2);

    // Test JSON serialization roundtrip of EditStack
    let json = serde_json::to_string_pretty(&stack).expect("serialize stack");
    let deserialized: EditStack = serde_json::from_str(&json).expect("deserialize stack");
    assert_eq!(stack, deserialized);

    // Test HistoryManager
    let mut history = HistoryManager::new(stack.clone());
    assert!(!history.can_undo());
    assert!(!history.can_redo());

    let mut stack2 = stack.clone();
    stack2.push_op(EditOp::Vignette(VignetteParams::initial_active()));
    history.commit(stack2.clone());
    assert!(history.can_undo());
    assert!(!history.can_redo());

    let mut stack3 = stack2.clone();
    stack3.remove_node(id2);
    history.commit(stack3.clone());

    // Undo step
    let undo1 = history.undo().expect("undo 1");
    assert_eq!(undo1, &stack2);
    assert!(history.can_redo());

    // Undo step 2
    let undo2 = history.undo().expect("undo 2");
    assert_eq!(undo2, &stack);

    // Redo step
    let redo1 = history.redo().expect("redo 1");
    assert_eq!(redo1, &stack2);

    // Commit new fork
    let mut stack4 = stack2.clone();
    stack4.push_op(EditOp::WhiteBalance(WhiteBalanceParams {
        temperature: 15.0,
        tint: 5.0,
    }));
    history.commit(stack4);
    assert!(!history.can_redo()); // Redo stack should be cleared on commit
}

#[test]
fn test_lut3d_identity_sampling() {
    let lut = Lut3D::identity(17);
    assert_eq!(lut.size, 17);
    assert_eq!(lut.data.len(), 17 * 17 * 17);

    let test_points = [
        [0.0f32, 0.0, 0.0],
        [1.0, 1.0, 1.0],
        [0.5, 0.5, 0.5],
        [0.25, 0.75, 0.125],
        [0.85, 0.35, 0.65],
    ];

    for &rgb in &test_points {
        let sampled = lut.sample_trilinear(rgb);
        for c in 0..3 {
            let diff = (sampled[c] - rgb[c]).abs();
            assert!(
                diff < 0.02,
                "Identity LUT sampling failed at {:?}: got {:?}, diff={diff}",
                rgb,
                sampled
            );
        }
    }
}

#[test]
fn test_linear_image_and_downsampling() {
    let width = 64u32;
    let height = 32u32;
    let mut pixels = Vec::with_capacity((width * height) as usize);

    for y in 0..height {
        for x in 0..width {
            let r = x as f32 / (width - 1) as f32;
            let g = y as f32 / (height - 1) as f32;
            pixels.push([
                half::f16::from_f32(r),
                half::f16::from_f32(g),
                half::f16::from_f32(0.5),
                half::f16::from_f32(1.0),
            ]);
        }
    }

    let img = LinearImage::new(width, height, Arc::new(pixels), ImageMetadata::default())
        .expect("valid image");

    // Bilinear sampling at center
    let center = img.sample_bilinear_f32(0.5, 0.5);
    assert!((center[0] - 0.5).abs() < 0.05);
    assert!((center[1] - 0.5).abs() < 0.05);
    assert!((center[2] - 0.5).abs() < 0.01);
    assert!((center[3] - 1.0).abs() < 0.01);

    // Downsample box
    let downsampled = img.downsample_box(16);
    assert_eq!(downsampled.width.max(downsampled.height), 16);
    assert_eq!(downsampled.width, 16);
    assert_eq!(downsampled.height, 8);
    assert_eq!(downsampled.pixels.len(), 16 * 8);

    // Histogram computation
    let hist = HistogramData::from_linear_image(&img);
    assert!(hist.max_bin > 0);
    assert_eq!(hist.luma.iter().sum::<u32>(), width * height);

    // Auto-tune from histogram
    let auto_tune = TuneParams::auto_from_histogram(&hist);
    assert!(!auto_tune.is_identity());
}

#[test]
fn test_built_in_looks_count_and_integrity() {
    let looks = built_in_looks();
    assert_eq!(looks.len(), 11);

    let expected_names = [
        "Portrait",
        "Smooth",
        "Pop",
        "Accentuate",
        "Faded Glow",
        "Morning",
        "Bright",
        "Fine Art",
        "Push",
        "Structure",
        "Silhouette",
    ];

    for (look, expected_name) in looks.iter().zip(expected_names.iter()) {
        assert_eq!(&look.name, expected_name);
        assert!(look.built_in);
        assert!(!look.stack.is_empty());
    }
}

#[test]
fn test_app_config_defaults_and_paths() {
    let config = AppConfig::default();
    assert_eq!(config.favorites.len(), 6);
    assert!(config.dark_theme);
    assert!(config.show_sidebar);

    let config_dir = AppConfig::config_dir();
    assert!(config_dir.ends_with("snap"));

    let data_dir = AppConfig::data_dir();
    assert!(data_dir.ends_with("snap"));

    let looks_dir = AppConfig::looks_dir();
    assert!(looks_dir.ends_with("looks"));

    let luts_dir = AppConfig::luts_dir();
    assert!(luts_dir.ends_with("luts"));
}

#[test]
fn test_slider_params_hud() {
    let mut op = EditOp::default_for_tool(ToolId::TuneImage);
    let sliders = op.slider_params();
    assert_eq!(sliders.len(), 7); // Brightness, Contrast, Saturation, Ambiance, Highlights, Shadows, Warmth
    assert_eq!(sliders[0].0.name, "Brightness");

    // Set brightness
    op.set_slider_param(0, 42.0);
    if let EditOp::TuneImage(p) = &op {
        assert_eq!(p.brightness, 42.0);
    } else {
        panic!("expected TuneImage op");
    }
}

// ============================================================================
// Phase 2-4 Tests: LayerMask, BrushMap, SmartMaskKind, & 35 Editing Tools
// ============================================================================

#[test]
fn test_layer_mask_paint_and_serde() {
    let mut mask = LayerMask::empty();
    assert!(mask.is_uniform_zero());
    assert!(!mask.is_uniform_full());
    assert_eq!(mask.as_bytes().len(), 65536);

    let hash_empty = mask.content_hash();
    assert_ne!(hash_empty, 0);

    // Paint dab in center
    mask.paint_dab(0.5, 0.5, 0.20, 0.5, 1.0);
    assert!(!mask.is_uniform_zero());
    assert_ne!(mask.content_hash(), hash_empty);

    // Sample center vs outside
    let center_val = mask.sample_bilinear(0.5, 0.5);
    assert!(
        center_val > 0.95,
        "Center of dab should be close to 1.0, got {center_val}"
    );

    let corner_val = mask.sample_bilinear(0.05, 0.05);
    assert_eq!(corner_val, 0.0);

    // Paint stroke
    mask.paint_stroke(0.1, 0.2, 0.9, 0.2, 0.04, 0.8, 1.0);
    let mid_stroke = mask.sample_bilinear(0.5, 0.2);
    assert!(
        mid_stroke > 0.9,
        "Midpoint of stroke should be painted, got {mid_stroke}"
    );

    // Invert mask
    mask.invert();
    let inverted_corner = mask.sample_bilinear(0.05, 0.05);
    assert!(
        inverted_corner > 0.95,
        "Inverted empty region should be near 1.0, got {inverted_corner}"
    );

    // Test Compact JSON serialization
    let full_json = serde_json::to_string(&LayerMask::full()).expect("serialize full mask");
    assert!(full_json.len() < 64, "Uniform mask RLE must be tiny, got {}", full_json.len());

    let json = serde_json::to_string(&mask).expect("serialize mask");
    assert!(
        json.starts_with("\"rle1:"),
        "LayerMask must serialize as compact rle1 string, got: {json}"
    );
    assert!(
        json.len() < 65_536,
        "RLE JSON string must stay compact vs raw JSON array (~260KB), got length {}",
        json.len()
    );

    let roundtrip: LayerMask = serde_json::from_str(&json).expect("deserialize mask");
    assert_eq!(mask, roundtrip);
    assert_eq!(mask.content_hash(), roundtrip.content_hash());

    // Test backward compatibility: EditNode deserialization without "mask" field
    let legacy_json = r#"{
        "id": "11111111-2222-3333-4444-555555555555",
        "enabled": true,
        "opacity": 0.85,
        "op": {
            "TuneImage": {
                "brightness": 12.0,
                "contrast": 5.0,
                "saturation": 0.0,
                "ambiance": 0.0,
                "highlights": 0.0,
                "shadows": 0.0,
                "warmth": 0.0
            }
        }
    }"#;
    let node: EditNode = serde_json::from_str(legacy_json).expect("deserialize legacy node");
    assert_eq!(node.opacity, 0.85);
    assert_eq!(node.mask, None);

    // Test EditNode with mask serializes and deserializes
    let node_with_mask = EditNode::with_mask(
        EditOp::Details(DetailsParams::default()),
        LayerMask::full(),
    );
    let node_json = serde_json::to_string(&node_with_mask).expect("serialize node with mask");
    assert!(node_json.contains("mask"));
    let node_roundtrip: EditNode = serde_json::from_str(&node_json).expect("roundtrip node with mask");
    assert_eq!(node_with_mask, node_roundtrip);
}

#[test]
fn test_brush_map_paint_and_serde() {
    let mut map = BrushMap::new();
    assert!(map.is_empty());
    assert_eq!(map.to_f32_vec().len(), 65536);

    let hash_empty = map.content_hash();
    assert_ne!(hash_empty, 0);

    // Paint stroke with positive step
    map.paint_stroke(0.2, 0.5, 0.8, 0.5, 0.10, 0.4, 50.0, false);
    assert!(!map.is_empty());
    assert_ne!(map.content_hash(), hash_empty);

    let center_val = map.sample_bilinear(0.5, 0.5);
    assert!(
        center_val > 0.40 && center_val <= 1.0,
        "Center should have positive brush value, got {center_val}"
    );

    // Test eraser mode on brush
    map.paint_dab(0.5, 0.5, 0.15, 0.0, 0.0, true);
    let erased_val = map.sample_bilinear(0.5, 0.5);
    assert!(
        erased_val.abs() < 0.1,
        "Erased center should be near 0, got {erased_val}"
    );

    // Test to_f32_vec values are in [-1.0, 1.0]
    let f32_vec = map.to_f32_vec();
    assert_eq!(f32_vec.len(), 65536);
    for &v in &f32_vec {
        assert!(v >= -1.0 && v <= 1.0, "Float out of bounds: {v}");
    }

    // Test compact serde roundtrip
    let json = serde_json::to_string(&map).expect("serialize brush map");
    assert!(json.starts_with("\"brle1:"));
    let roundtrip: BrushMap = serde_json::from_str(&json).expect("deserialize brush map");
    assert_eq!(map, roundtrip);
    assert_eq!(map.content_hash(), roundtrip.content_hash());
}

#[test]
fn test_smart_masks_all_variants() {
    // Construct a synthetic 64x64 test image:
    // Top half: bright cool sky (Rec.2020 blue/cyan, elevated luminance)
    // Bottom half: dark muted ground (low luminance, warm/brownish)
    // Center: warm skin/subject spot
    let width = 64u32;
    let height = 64u32;
    let mut pixels = Vec::with_capacity((width * height) as usize);

    for y in 0..height {
        let v = y as f32 / (height - 1) as f32;
        for x in 0..width {
            let u = x as f32 / (width - 1) as f32;

            let color = if (u - 0.5).hypot(v - 0.5) < 0.15 {
                // Center warm skin/subject
                [0.85f32, 0.55, 0.45, 1.0]
            } else if v < 0.45 {
                // Cool bright sky
                [0.45f32, 0.70, 0.95, 1.0]
            } else {
                // Dark ground
                [0.10f32, 0.12, 0.06, 1.0]
            };

            pixels.push([
                half::f16::from_f32(color[0]),
                half::f16::from_f32(color[1]),
                half::f16::from_f32(color[2]),
                half::f16::from_f32(color[3]),
            ]);
        }
    }

    let img = LinearImage::new(width, height, Arc::new(pixels), ImageMetadata::default())
        .expect("valid image");

    assert_eq!(SmartMaskKind::ALL.len(), 7);

    for &kind in SmartMaskKind::ALL {
        let mask = LayerMask::from_smart_mask(kind, &img);
        assert_eq!(mask.as_bytes().len(), 65536);
        assert_ne!(kind.display_name(), "");

        match kind {
            SmartMaskKind::Sky => {
                let sky_sample = mask.sample_bilinear(0.5, 0.15);
                let ground_sample = mask.sample_bilinear(0.5, 0.85);
                assert!(
                    sky_sample > ground_sample,
                    "Sky sample ({sky_sample}) should exceed ground sample ({ground_sample})"
                );
            }
            SmartMaskKind::Highlights => {
                let bright_sample = mask.sample_bilinear(0.5, 0.15);
                let dark_sample = mask.sample_bilinear(0.1, 0.90);
                assert!(
                    bright_sample > dark_sample,
                    "Highlight mask at bright region ({bright_sample}) should exceed dark region ({dark_sample})"
                );
            }
            SmartMaskKind::Shadows => {
                let dark_sample = mask.sample_bilinear(0.1, 0.90);
                let bright_sample = mask.sample_bilinear(0.5, 0.15);
                assert!(
                    dark_sample > bright_sample,
                    "Shadow mask at dark region ({dark_sample}) should exceed bright region ({bright_sample})"
                );
            }
            SmartMaskKind::SubjectCenter => {
                let center_sample = mask.sample_bilinear(0.5, 0.48);
                let corner_sample = mask.sample_bilinear(0.05, 0.05);
                assert!(
                    center_sample > corner_sample,
                    "SubjectCenter at center ({center_sample}) should exceed corner ({corner_sample})"
                );
            }
            SmartMaskKind::Background => {
                let center_sample = mask.sample_bilinear(0.5, 0.48);
                let corner_sample = mask.sample_bilinear(0.05, 0.95);
                assert!(
                    corner_sample > center_sample,
                    "Background at corner ({corner_sample}) should exceed center ({center_sample})"
                );
            }
            SmartMaskKind::SkinTones => {
                let skin_sample = mask.sample_bilinear(0.5, 0.5);
                let cold_sky_sample = mask.sample_bilinear(0.1, 0.1);
                assert!(
                    skin_sample > cold_sky_sample,
                    "Skin tone mask at center ({skin_sample}) should exceed blue sky ({cold_sky_sample})"
                );
            }
            SmartMaskKind::Midtones => {
                // Should produce smooth valid weights
                let mid_sample = mask.sample_bilinear(0.5, 0.5);
                assert!(mid_sample >= 0.0 && mid_sample <= 1.0);
            }
        }
    }
}

#[test]
fn test_all_35_tools_and_full_stack_serde() {
    assert_eq!(ToolId::ALL.len(), 35);
    assert_eq!(ToolCategory::ALL.len(), 6);

    let mut stack = EditStack::default();

    for &tool in ToolId::ALL {
        let op = EditOp::default_for_tool(tool);
        assert_eq!(op.tool_id(), tool);

        let disp_name = tool.display_name();
        assert!(!disp_name.is_empty());
        assert!(disp_name.is_ascii(), "Display name must be ASCII: {disp_name}");

        let code = tool.short_code();
        assert!(!code.is_empty());
        assert!(code.is_ascii(), "Short code must be ASCII: {code}");

        let category_name = tool.category();
        assert_eq!(category_name, tool.tool_category().display_name());
        assert!(ToolId::in_category(tool.tool_category()).contains(&tool));

        // Slider params testing and roundtrip mutation
        let sliders = op.slider_params();
        for (i, (desc, _cur_val)) in sliders.iter().enumerate() {
            assert!(desc.name.is_ascii(), "Param name must be ASCII: {}", desc.name);
            assert!(desc.unit.is_ascii(), "Param unit must be ASCII: {}", desc.unit);
            assert!(
                desc.min <= desc.max,
                "Slider min ({}) > max ({}) on {}",
                desc.min,
                desc.max,
                desc.name
            );

            // Test setting the parameter
            let mut mutated = op.clone();
            let new_val = (desc.min + desc.max) * 0.5;
            mutated.set_slider_param(i, new_val);
            let updated_sliders = mutated.slider_params();
            assert_eq!(updated_sliders.len(), sliders.len());
        }

        // Push to stack with a mask on some nodes
        let id = if tool == ToolId::Vignette || tool == ToolId::Selective {
            let mut custom_mask = LayerMask::empty();
            custom_mask.paint_dab(0.5, 0.5, 0.25, 0.5, 1.0);
            let node = EditNode::with_mask(op, custom_mask);
            let nid = node.id;
            stack.nodes.push(node);
            nid
        } else {
            stack.push_op(op)
        };

        // Test EditStack helpers
        assert!(stack.node_by_id(id).is_some());
        assert!(stack.node_by_id_mut(id).is_some());
        assert!(stack.set_node_opacity(id, 0.85));
    }

    assert_eq!(stack.nodes.len(), 35);

    // Test setting mask via stack helper
    let first_id = stack.nodes[0].id;
    assert!(stack.set_node_mask(first_id, Some(LayerMask::full())));
    assert!(stack.node_by_id(first_id).unwrap().mask.is_some());

    // Serialize full 35-tool stack to JSON and deserialize back
    let json = serde_json::to_string_pretty(&stack).expect("serialize full stack");
    assert!(json.contains("RawDevelop"));
    assert!(json.contains("TextOverlay"));

    let deserialized: EditStack = serde_json::from_str(&json).expect("deserialize full stack");
    assert_eq!(stack, deserialized);
    assert_eq!(deserialized.nodes.len(), 35);
}

#[test]
fn test_params_default_identity() {
    // Verify that Default::default() is identity for non-converting filter types
    assert!(RawDevelopParams::default().is_identity());
    assert!(TuneParams::default().is_identity());
    assert!(DetailsParams::default().is_identity());
    assert!(CurvesParams::default().is_identity());
    assert!(WhiteBalanceParams::default().is_identity());
    assert!(ColorHslParams::default().is_identity());
    assert!(Lut3dParams::default().is_identity());
    assert!(ColorGradeParams::default().is_identity());
    assert!(DehazeParams::default().is_identity());
    assert!(TonalContrastParams::default().is_identity());
    assert!(PerspectiveParams::default().is_identity());
    assert!(ExpandParams::default().is_identity());
    assert!(SelectiveParams::default().is_identity());
    assert!(BrushParams::default().is_identity());
    assert!(HealingParams::default().is_identity());
    assert!(VignetteParams::default().is_identity());
    assert!(HalationParams::default().is_identity());
    assert!(BloomParams::default().is_identity());
    assert!(GrainyFilmParams::default().is_identity());
    assert!(HdrScapeParams::default().is_identity());
    assert!(GlamourGlowParams::default().is_identity());
    assert!(DramaParams::default().is_identity());
    assert!(VintageParams::default().is_identity());
    assert!(DoubleExposureParams::default().is_identity());
    assert!(RetroluxParams::default().is_identity());
    assert!(GrungeParams::default().is_identity());
    assert!(LensBlurParams::default().is_identity());
    assert!(MotionBlurParams::default().is_identity());
    assert!(PortraitParams::default().is_identity());
    assert!(HeadPoseParams::default().is_identity());
    assert!(FramesParams::default().is_identity());
    assert!(TextOverlayParams::default().is_identity());
    assert!(CropRotateParams::default().is_identity());

    // Creative B&W conversion filters always modify color data
    assert!(!BwParams::default().is_identity());
    assert!(!NoirParams::default().is_identity());
}
