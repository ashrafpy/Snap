use half::f16;
use snap_core::image_types::{ImageMetadata, LinearImage};

/// Procedural generator for a rich 1600x1066 HDR test scene in Linear Rec.2020 color space.
///
/// Features:
/// - Smooth HDR sunset twilight sky with high-dynamic-range sun disk (up to 4.0 EV highlight)
/// - Multi-layered mountain silhouettes with ambient atmospheric haze
/// - Ground valley with warm golden/green tonal gradients
/// - 12-patch standard color checker swatches (Primary RGB, CMY, skin tones, neutral grays)
/// - Smooth horizontal gradient test ramps for hue & dynamic range inspection
pub fn generate_sample_hdr_scene() -> LinearImage {
    let width = 1600u32;
    let height = 1066u32;
    let mut pixels = Vec::with_capacity((width * height) as usize);

    let sun_cx = 0.50f32;
    let sun_cy = 0.28f32;
    let sun_r = 0.045f32;

    for y in 0..height {
        let v = y as f32 / height as f32; // 0.0 at top, 1.0 at bottom
        for x in 0..width {
            let u = x as f32 / width as f32; // 0.0 at left, 1.0 at right

            // Check if this pixel is inside the bottom test bench area (v >= 0.78)
            if v >= 0.78 {
                let bench_v = (v - 0.78) / 0.22; // 0.0 to 1.0 within bench
                if bench_v < 0.25 {
                    // Smooth continuous gradient ramp (black -> neutral gray -> full white -> super-white)
                    let t = u;
                    let val = t * 1.5; // reaches HDR highlight at right edge
                    pixels.push([
                        f16::from_f32(val),
                        f16::from_f32(val),
                        f16::from_f32(val),
                        f16::from_f32(1.0),
                    ]);
                    continue;
                } else if bench_v < 0.50 {
                    // Spectral Hue sweep across Rec.2020 spectrum
                    let hue = u * 360.0;
                    let rgb = hsv_to_linear_rec2020(hue, 0.85, 0.75);
                    pixels.push([
                        f16::from_f32(rgb[0]),
                        f16::from_f32(rgb[1]),
                        f16::from_f32(rgb[2]),
                        f16::from_f32(1.0),
                    ]);
                    continue;
                } else {
                    // 12 Color Checker Swatches
                    let swatch_idx = (u * 12.0).floor() as usize;
                    let swatch_idx = swatch_idx.min(11);
                    let [r, g, b] = get_color_checker_patch(swatch_idx);
                    pixels.push([
                        f16::from_f32(r),
                        f16::from_f32(g),
                        f16::from_f32(b),
                        f16::from_f32(1.0),
                    ]);
                    continue;
                }
            }

            // Sky, Mountains & Sun Landscape (v < 0.78)
            let sky_t = v / 0.78;

            // Sky gradient: Deep twilight blue/indigo at zenith -> Warm apricot orange -> Golden horizon
            let sky_r = (0.04 + 0.95 * sky_t.powf(2.2)).min(1.2);
            let sky_g = (0.06 + 0.65 * sky_t.powf(1.8)).min(1.0);
            let sky_b = (0.28 + 0.20 * (1.0 - sky_t) + 0.10 * sky_t).min(0.9);

            // Sun disk calculation with HDR core & radial glow
            let dx = (u - sun_cx) * (width as f32 / height as f32);
            let dy = v - sun_cy;
            let dist = (dx * dx + dy * dy).sqrt();

            let mut r = sky_r;
            let mut g = sky_g;
            let mut b = sky_b;

            if dist < sun_r {
                // Sun core (High Dynamic Range specular highlight: ~3.5 EV above 1.0)
                let core_t = 1.0 - (dist / sun_r);
                let sun_intensity = 1.0 + 3.2 * core_t.powi(2);
                r = r.max(sun_intensity * 1.05);
                g = g.max(sun_intensity * 0.95);
                b = b.max(sun_intensity * 0.70);
            } else if dist < sun_r * 4.5 {
                // Sun corona glow
                let glow_t = 1.0 - ((dist - sun_r) / (sun_r * 3.5));
                let glow = glow_t.powi(3) * 0.85;
                r += glow * 1.1;
                g += glow * 0.7;
                b += glow * 0.2;
            }

            // Mountain Ridge 1 (Far background mountains, ~v = 0.38)
            let mtn1_h = 0.36 + 0.08 * (u * 7.5).sin() + 0.04 * (u * 17.0).cos();
            if v > mtn1_h {
                // Silhouetted mountain with atmospheric haze
                let depth = ((v - mtn1_h) / (0.78 - mtn1_h)).clamp(0.0, 1.0);
                let haze = (1.0 - depth * 0.6).clamp(0.2, 1.0);
                r = r * 0.35 * haze + 0.08 * (1.0 - haze);
                g = g * 0.38 * haze + 0.10 * (1.0 - haze);
                b = b * 0.45 * haze + 0.15 * (1.0 - haze);
            }

            // Mountain Ridge 2 (Midground mountains, ~v = 0.50)
            let mtn2_h = 0.48 + 0.06 * (u * 11.0 + 1.2).cos() + 0.03 * (u * 23.0).sin();
            if v > mtn2_h {
                let depth = ((v - mtn2_h) / (0.78 - mtn2_h)).clamp(0.0, 1.0);
                let shade = 0.22 + 0.30 * depth;
                r = 0.07 * shade + 0.03 * (u * 30.0).sin().abs();
                g = 0.14 * shade + 0.04 * (u * 28.0).cos().abs();
                b = 0.09 * shade;
            }

            // Foreground Valley & Rolling hills (v > 0.62)
            let hill_h = 0.62 + 0.04 * (u * 5.0 + 2.0).sin();
            if v > hill_h {
                let ground_t = ((v - hill_h) / (0.78 - hill_h)).clamp(0.0, 1.0);
                let grass_r = 0.08 + 0.12 * ground_t;
                let grass_g = 0.18 + 0.18 * ground_t;
                let grass_b = 0.05 + 0.04 * ground_t;
                r = grass_r;
                g = grass_g;
                b = grass_b;
            }

            pixels.push([
                f16::from_f32(r.clamp(0.0, 10.0)),
                f16::from_f32(g.clamp(0.0, 10.0)),
                f16::from_f32(b.clamp(0.0, 10.0)),
                f16::from_f32(1.0),
            ]);
        }
    }

    let mut metadata = ImageMetadata::default();
    metadata.camera_make = Some("Snap Procedural HDR".to_string());
    metadata.camera_model = Some("Rec.2020 Test Scene".to_string());
    metadata.iso = Some(100);
    metadata.shutter_speed = Some("1/250".to_string());
    metadata.aperture = Some(2.8);
    metadata.focal_length = Some(35.0);
    metadata.original_width = width;
    metadata.original_height = height;

    LinearImage::new(width, height, pixels, metadata).expect("Valid procedural linear image")
}

fn hsv_to_linear_rec2020(h_deg: f32, s: f32, v: f32) -> [f32; 3] {
    let c = v * s;
    let h_prime = (h_deg % 360.0) / 60.0;
    let x = c * (1.0 - (h_prime % 2.0 - 1.0).abs());
    let m = v - c;

    let (r1, g1, b1) = if (0.0..1.0).contains(&h_prime) {
        (c, x, 0.0)
    } else if (1.0..2.0).contains(&h_prime) {
        (x, c, 0.0)
    } else if (2.0..3.0).contains(&h_prime) {
        (0.0, c, x)
    } else if (3.0..4.0).contains(&h_prime) {
        (0.0, x, c)
    } else if (4.0..5.0).contains(&h_prime) {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    let srgb_linear = [r1 + m, g1 + m, b1 + m];
    snap_core::linear_srgb_to_rec2020(srgb_linear)
}

fn get_color_checker_patch(idx: usize) -> [f32; 3] {
    // 12 representative patches in linear Rec.2020:
    // 0: Light Skin, 1: Dark Skin, 2: Blue Sky, 3: Foliage, 4: Blue, 5: Green
    // 6: Red, 7: Yellow, 8: Magenta, 9: Cyan, 10: 50% Gray, 11: 18% Neutral Gray
    match idx {
        0 => [0.45, 0.28, 0.20], // Light Skin
        1 => [0.22, 0.12, 0.08], // Dark Skin
        2 => [0.12, 0.24, 0.48], // Blue Sky
        3 => [0.10, 0.26, 0.08], // Foliage
        4 => [0.05, 0.08, 0.65], // Primary Blue
        5 => [0.06, 0.62, 0.10], // Primary Green
        6 => [0.72, 0.06, 0.05], // Primary Red
        7 => [0.82, 0.72, 0.06], // Primary Yellow
        8 => [0.65, 0.06, 0.55], // Magenta
        9 => [0.06, 0.65, 0.72], // Cyan
        10 => [0.50, 0.50, 0.50], // 50% Gray
        11 => [0.18, 0.18, 0.18], // 18% Neutral Gray
        _ => [0.18, 0.18, 0.18],
    }
}
