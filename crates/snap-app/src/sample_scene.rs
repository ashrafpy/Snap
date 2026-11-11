use half::f16;
use rayon::prelude::*;
use snap_core::image_types::{ImageMetadata, LinearImage};

/// Procedural generator for a breathtaking, photographic Golden-Hour Alpine Lake & Dolomite Mountain Sunset.
///
/// Rendered in Linear Rec.2020 color space:
/// - Atmospheric Rayleigh/Mie twilight sky with smooth HDR sun disk (peak ~3.2 Rec.2020) and Mie aureole
/// - Wispy high-altitude cirrus clouds with underside sunset illumination
/// - 4 layered procedural mountain ranges with alpine ridge lighting, snow-dusted gullies, and aerial haze
/// - Calm alpine lake with perspective-foreshortened horizontal ripples, Fresnel teal absorption, and HDR sun-glints
/// - Dark rocky foreground promontories with warm moss/autumn grass accents
pub fn generate_sample_hdr_scene() -> LinearImage {
    let width = 1600u32;
    let height = 1066u32;

    // Precompute 1D mountain profiles & slopes across all columns
    let profiles = MountainProfiles::generate(width as usize);

    let mut pixels = vec![
        [
            f16::from_f32(0.0),
            f16::from_f32(0.0),
            f16::from_f32(0.0),
            f16::from_f32(1.0),
        ];
        (width * height) as usize
    ];

    pixels
        .par_chunks_mut(width as usize)
        .enumerate()
        .for_each(|(y_idx, scanline)| {
            let v = y_idx as f32 / height as f32;
            for (x_idx, px) in scanline.iter_mut().enumerate() {
                let u = x_idx as f32 / width as f32;
                let col = render_scene_pixel(u, v, &profiles);
                *px = [
                    f16::from_f32(col[0].clamp(0.0, 10.0)),
                    f16::from_f32(col[1].clamp(0.0, 10.0)),
                    f16::from_f32(col[2].clamp(0.0, 10.0)),
                    f16::from_f32(1.0),
                ];
            }
        });

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

// =========================================================================
// Scene Rendering Pipeline
// =========================================================================

const HORIZON_Y: f32 = 0.655;
const SUN_X: f32 = 0.38;
const SUN_Y: f32 = 0.43;

#[inline(always)]
fn render_scene_pixel(u: f32, v: f32, profiles: &MountainProfiles) -> [f32; 3] {
    // If above horizon: render landscape directly (sky, mountains)
    if v < HORIZON_Y {
        return render_landscape_sample(u, v, profiles);
    }

    // Mirror Lake Reflection (v >= HORIZON_Y)
    let d_water = (v - HORIZON_Y) / (1.0 - HORIZON_Y);

    // Perspective-foreshortened horizontal ripples: higher frequency near horizon, wider amplitude in foreground
    let freq = 75.0 / (d_water * 0.80 + 0.15);
    let w1 = (u * 38.0 + v * freq).sin();
    let w2 = (u * 82.0 - v * freq * 1.5 + 2.1).sin() * 0.45;
    let w_noise = (smooth_noise2d(u * 22.0, v * 75.0) - 0.5) * 0.5;

    let ripple_dy = (w1 + w2 + w_noise) * (0.0022 + 0.016 * d_water.powf(1.4));
    let ripple_dx = (w1 * 0.6 + w2) * (0.0018 + 0.010 * d_water.powf(1.2));

    let mirror_v = 2.0 * HORIZON_Y - v;
    let refl_u = (u + ripple_dx).clamp(0.001, 0.999);
    let refl_v = (mirror_v + ripple_dy).clamp(0.001, HORIZON_Y);

    let refl_col = render_landscape_sample(refl_u, refl_v, profiles);

    // Fresnel reflectance and deep water absorption
    let cos_theta = (d_water * 0.85 + 0.15).clamp(0.0, 1.0);
    let fresnel = (0.03 + 0.97 * (1.0 - cos_theta).powi(4)).clamp(0.24, 0.86);
    let deep_lake_teal = [0.008, 0.022, 0.032];

    let mut color = [
        deep_lake_teal[0] * (1.0 - fresnel) + refl_col[0] * fresnel * 0.88,
        deep_lake_teal[1] * (1.0 - fresnel) + refl_col[1] * fresnel * 0.88,
        deep_lake_teal[2] * (1.0 - fresnel) + refl_col[2] * fresnel * 0.88,
    ];

    // Sparkling HDR golden sun-glints along the water ripple normals beneath SUN_X
    let sun_dist_x = (u - SUN_X).abs();
    let col_width = 0.035 + 0.085 * d_water;
    let in_sun_column = (-((sun_dist_x / col_width).powi(2))).exp();

    if in_sun_column > 0.01 {
        let micro_glint = smooth_noise2d(u * 140.0, v * 320.0);
        let wave_glint = w1.abs().powi(3);
        if micro_glint > 0.50 {
            let glint_norm = (micro_glint - 0.50) / 0.50;
            let glint_val = glint_norm.powi(2) * wave_glint * in_sun_column * (2.8 - 1.2 * d_water);
            color[0] += glint_val * 1.08;
            color[1] += glint_val * 0.78;
            color[2] += glint_val * 0.35;
        }
    }

    // Layer 4: Foreground Shoreline Promontory silhouettes framing the lake
    if u < 0.26 {
        let shore_l_h = sample_profile(&profiles.shore_l, u);
        if v > shore_l_h {
            let t_shore = ((v - shore_l_h) / 0.15).clamp(0.0, 1.0);
            let shore_n = smooth_noise2d(u * 35.0, v * 35.0);
            let rock_base = [0.014, 0.015, 0.018];
            let moss = [0.028, 0.042, 0.016];
            let grass = [0.055, 0.036, 0.014];
            let foliage = lerp3(moss, grass, (shore_n * 1.5).clamp(0.0, 1.0));
            color = lerp3(rock_base, foliage, (shore_n * 0.80 + t_shore * 0.20).clamp(0.0, 1.0));
        }
    } else if u > 0.74 {
        let shore_r_h = sample_profile(&profiles.shore_r, u);
        if v > shore_r_h {
            let t_shore = ((v - shore_r_h) / 0.15).clamp(0.0, 1.0);
            let shore_n = smooth_noise2d(u * 38.0 + 5.0, v * 38.0);
            let rock_base = [0.013, 0.015, 0.017];
            let moss = [0.026, 0.040, 0.015];
            let grass = [0.052, 0.035, 0.013];
            let foliage = lerp3(moss, grass, (shore_n * 1.4).clamp(0.0, 1.0));
            color = lerp3(rock_base, foliage, (shore_n * 0.75 + t_shore * 0.25).clamp(0.0, 1.0));
        }
    }

    color
}

#[inline(always)]
fn render_landscape_sample(u: f32, v: f32, profiles: &MountainProfiles) -> [f32; 3] {
    // ---------------------------------------------------------------------
    // Layer 3: Near Pine Foothills (y ~ 0.56..0.67)
    // ---------------------------------------------------------------------
    let h3 = sample_profile(&profiles.h3, u);
    if v > h3 {
        let canopy_noise = smooth_noise2d(u * 120.0, v * 120.0);
        return lerp3([0.007, 0.016, 0.010], [0.038, 0.058, 0.022], canopy_noise);
    }

    // ---------------------------------------------------------------------
    // Layer 2: Mid-ground Alpine Ridge (y ~ 0.48..0.66)
    // ---------------------------------------------------------------------
    let h2 = sample_profile(&profiles.h2, u);
    if v > h2 {
        let dh2 = sample_profile(&profiles.dh2, u);
        // Combine macro slope with 2D diagonal alpine rock ridges (eliminates 1D vertical striping)
        let diag_ridge2 = smooth_noise2d(u * 26.0 - (v - h2) * 18.0, v * 34.0) - 0.5;
        let sun_dir = if u > SUN_X { 1.0 } else { -1.0 };
        let rim2 = (dh2 * sun_dir * 4.5 + diag_ridge2 * sun_dir * 0.75 + 0.25).clamp(0.0, 1.0);
        let mut l2_color = lerp3(
            [0.022, 0.028, 0.048],
            [0.195, 0.115, 0.045],
            rim2 * 0.78 + 0.10,
        );

        // Valley mist between Layer 1 and Layer 2
        let mist_factor = (-((v - 0.585) / 0.042).powi(2)).exp() * 0.38;
        let mist_color = [0.420, 0.220, 0.130];
        l2_color = lerp3(l2_color, mist_color, mist_factor);
        return l2_color;
    }

    // ---------------------------------------------------------------------
    // Layer 1: Far Dolomite Peaks (y ~ 0.40..0.66)
    // ---------------------------------------------------------------------
    let h1 = sample_profile(&profiles.h1, u);
    if v > h1 {
        let dh1 = sample_profile(&profiles.dh1, u);
        let diag_ridge1 = 0.65 * (smooth_noise2d(u * 20.0 + (v - h1) * 15.0, v * 26.0) - 0.5)
            + 0.35 * (smooth_noise2d(u * 48.0 - (v - h1) * 24.0, v * 52.0) - 0.5);
        let sun_dir = if u > SUN_X { 1.0 } else { -1.0 };
        let rim_light = (dh1 * sun_dir * 5.0 + diag_ridge1 * sun_dir * 0.85 + 0.28).clamp(0.0, 1.0);

        // Snow-dusted upper gullies
        let snow_h = ((0.48 - v) / 0.09).clamp(0.0, 1.0);
        let gully_noise = smooth_noise2d(u * 45.0, v * 16.0);
        let snow_amt = if gully_noise > 0.46 {
            ((gully_noise - 0.46) / 0.30).clamp(0.0, 1.0) * snow_h
        } else {
            0.0
        };
        let snow_col = lerp3(
            [0.220, 0.200, 0.280],
            [0.680, 0.460, 0.280],
            rim_light * 0.85 + 0.15,
        );

        // Dolomite craggy rock
        let rock_base = [0.040, 0.032, 0.052];
        let rock_sun = [0.380, 0.210, 0.085];
        let rock_col = lerp3(rock_base, rock_sun, rim_light * 0.85 + 0.10);
        let mut l1_color = lerp3(rock_col, snow_col, snow_amt * 0.85);

        // Atmospheric perspective haze increasing toward base
        let depth = ((v - h1) / (HORIZON_Y - h1).max(0.01)).clamp(0.0, 1.0);
        let haze_color = [0.450, 0.220, 0.110];
        let haze_amount = (depth * 0.72).powf(1.35);
        l1_color = lerp3(l1_color, haze_color, haze_amount);
        return l1_color;
    }

    // ---------------------------------------------------------------------
    // Sky Background Gradient (y: 0.0 -> HORIZON_Y)
    // ---------------------------------------------------------------------
    let mut sky = sky_background_color(v);

    // ---------------------------------------------------------------------
    // Sun Disk & Smooth HDR Corona centered at (SUN_X, SUN_Y)
    // ---------------------------------------------------------------------
    let aspect = 1600.0 / 1066.0;
    let dx = (u - SUN_X) * aspect;
    let dy = v - SUN_Y;
    let dist = (dx * dx + dy * dy).sqrt();

    // Exponential Mie scattering aureole
    let aureole_amber = (-dist * 7.5).exp();
    let aureole_scatter = (-dist * 2.4).exp();
    sky[0] += 0.65 * aureole_amber + 0.22 * aureole_scatter;
    sky[1] += 0.32 * aureole_amber + 0.12 * aureole_scatter;
    sky[2] += 0.08 * aureole_amber + 0.09 * aureole_scatter;

    // Glowing HDR solar disk (peak: [3.2, 2.5, 1.4])
    if dist < 0.032 {
        let disk_alpha = if dist < 0.027 {
            1.0
        } else {
            let t = (0.032 - dist) / (0.032 - 0.027);
            t * t * (3.0 - 2.0 * t)
        };
        let core_factor = (1.0 - (dist / 0.027).min(1.0)).powi(2);
        let disk_color = [
            2.4 + 0.8 * core_factor,
            1.9 + 0.6 * core_factor,
            1.0 + 0.4 * core_factor,
        ];
        sky = lerp3(sky, disk_color, disk_alpha);
    }

    // ---------------------------------------------------------------------
    // Wispy High-Altitude Cirrus Clouds
    // ---------------------------------------------------------------------
    if (0.06..0.55).contains(&v) {
        let nx = u * 3.5;
        let ny = v * 14.0;
        let warp = smooth_noise2d(nx + 2.1, ny + 1.4) * 0.35;
        let c_noise = 0.55 * smooth_noise2d(nx + warp, ny)
            + 0.30 * smooth_noise2d((nx + warp) * 2.1 + 4.2, ny * 2.1 + 8.1)
            + 0.15 * smooth_noise2d((nx + warp) * 4.3 + 9.5, ny * 4.2 + 17.3);

        let envelope = if v < 0.22 {
            (v - 0.06) / (0.22 - 0.06)
        } else if v < 0.42 {
            1.0
        } else {
            (0.55 - v) / (0.55 - 0.42)
        };
        let density = ((c_noise - 0.44) / 0.26).clamp(0.0, 1.0) * envelope;

        if density > 0.001 {
            let d_sun = (((u - SUN_X) * 1.5).powi(2) + (v - SUN_Y).powi(2)).sqrt();
            let underlight = (-d_sun * 3.0).exp();
            let cloud_coral = [0.460, 0.220, 0.100];
            let cloud_lavender = [0.090, 0.070, 0.120];
            let cloud_color = lerp3(cloud_lavender, cloud_coral, (underlight * 1.4).clamp(0.0, 1.0));
            sky = lerp3(sky, cloud_color, density * 0.70);
        }
    }

    sky
}

#[inline(always)]
fn sky_background_color(v: f32) -> [f32; 3] {
    let v_clamped = v.clamp(0.0, HORIZON_Y);
    if v_clamped < 0.35 {
        let t = v_clamped / 0.35;
        let t_eased = t * t * (3.0 - 2.0 * t);
        // Zenith slate-cobalt -> Mid-sky dusty lavender/rose
        lerp3([0.018, 0.042, 0.125], [0.110, 0.085, 0.160], t_eased)
    } else {
        let t = (v_clamped - 0.35) / (HORIZON_Y - 0.35);
        let t_eased = t * t * (3.0 - 2.0 * t);
        // Mid-sky dusty lavender/rose -> Horizon warm golden-peach & amber
        lerp3([0.110, 0.085, 0.160], [0.580, 0.240, 0.070], t_eased)
    }
}

// =========================================================================
// Mountain Profiles & Procedural Heightfield Geometry
// =========================================================================

struct MountainProfiles {
    h1: Vec<f32>,
    dh1: Vec<f32>,
    h2: Vec<f32>,
    dh2: Vec<f32>,
    h3: Vec<f32>,
    shore_l: Vec<f32>,
    shore_r: Vec<f32>,
}

impl MountainProfiles {
    fn generate(width: usize) -> Self {
        let mut h1 = Vec::with_capacity(width);
        let mut dh1 = Vec::with_capacity(width);
        let mut h2 = Vec::with_capacity(width);
        let mut dh2 = Vec::with_capacity(width);
        let mut h3 = Vec::with_capacity(width);
        let mut shore_l = Vec::with_capacity(width);
        let mut shore_r = Vec::with_capacity(width);

        let eps = 0.028;

        for x in 0..width {
            let u = x as f32 / width as f32;

            let h1_val = eval_layer1(u);
            let h1_prev = eval_layer1((u - eps).max(0.0));
            let h1_next = eval_layer1((u + eps).min(1.0));
            let slope1 = (h1_next - h1_prev) / (2.0 * eps);

            let h2_val = eval_layer2(u);
            let h2_prev = eval_layer2((u - eps).max(0.0));
            let h2_next = eval_layer2((u + eps).min(1.0));
            let slope2 = (h2_next - h2_prev) / (2.0 * eps);

            let h3_val = eval_layer3(u);
            let sl_val = eval_shore_left(u);
            let sr_val = eval_shore_right(u);

            h1.push(h1_val);
            dh1.push(slope1);
            h2.push(h2_val);
            dh2.push(slope2);
            h3.push(h3_val);
            shore_l.push(sl_val);
            shore_r.push(sr_val);
        }

        Self {
            h1,
            dh1,
            h2,
            dh2,
            h3,
            shore_l,
            shore_r,
        }
    }
}

#[inline(always)]
fn sample_profile(profile: &[f32], u: f32) -> f32 {
    let px = (u * 1599.0).clamp(0.0, 1599.0);
    let i0 = px as usize;
    let i1 = (i0 + 1).min(1599);
    let frac = px - i0 as f32;
    profile[i0] * (1.0 - frac) + profile[i1] * frac
}

fn eval_layer1(u: f32) -> f32 {
    let peak_58 = (-((u - 0.58) / 0.082).powi(2)).exp() * 0.245;
    let tower_63 = (-((u - 0.63) / 0.045).powi(2)).exp() * 0.160;
    let peak_18 = (-((u - 0.18) / 0.095).powi(2)).exp() * 0.210;
    let tower_12 = (-((u - 0.12) / 0.055).powi(2)).exp() * 0.135;
    let gap_38 = (-((u - SUN_X) / 0.090).powi(2)).exp() * 0.080;
    let peak_85 = (-((u - 0.85) / 0.110).powi(2)).exp() * 0.150;

    let crags = 0.135 * mountain_fbm1d(u * 13.0, 103.5);
    HORIZON_Y - (peak_58 + tower_63 + peak_18 + tower_12 + gap_38 + peak_85 + crags)
}

fn eval_layer2(u: f32) -> f32 {
    let ridge_32 = (-((u - 0.32) / 0.13).powi(2)).exp() * 0.135;
    let ridge_72 = (-((u - 0.72) / 0.15).powi(2)).exp() * 0.155;
    let crags = 0.080 * mountain_fbm1d(u * 17.0 + 3.0, 241.7);
    HORIZON_Y - (0.045 + ridge_32 + ridge_72 + crags)
}

fn eval_layer3(u: f32) -> f32 {
    let roll1 = 0.048 * (u * 4.5 + 1.1).sin();
    let roll2 = 0.022 * (u * 9.8 + 2.7).cos();
    let pine_detail = (u * 420.0).sin().abs() * 0.0055 + smooth_noise1d(u * 210.0) * 0.0035;
    0.622 + roll1 + roll2 - pine_detail
}

fn eval_shore_left(u: f32) -> f32 {
    if u > 0.26 {
        return 1.1;
    }
    let t = u / 0.26;
    let base = 0.81 + 0.22 * t.powf(1.7);
    let crag = 0.018 * smooth_noise1d(u * 50.0);
    let pine = if u < 0.16 {
        ((u * 280.0).sin().abs() * 0.012).max(0.0)
    } else {
        0.0
    };
    base - crag - pine
}

fn eval_shore_right(u: f32) -> f32 {
    if u < 0.74 {
        return 1.1;
    }
    let t = (1.0 - u) / 0.26;
    let base = 0.84 + 0.20 * t.powf(1.6);
    let crag = 0.015 * smooth_noise1d(u * 55.0 + 12.0);
    let pine = if u > 0.86 {
        ((u * 320.0).sin().abs() * 0.010).max(0.0)
    } else {
        0.0
    };
    base - crag - pine
}

// =========================================================================
// Noise & Math Utilities
// =========================================================================

#[inline(always)]
fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);
    [
        a[0] + t * (b[0] - a[0]),
        a[1] + t * (b[1] - a[1]),
        a[2] + t * (b[2] - a[2]),
    ]
}

#[inline(always)]
fn hash1d(ix: i32) -> f32 {
    let mut h = (ix as u32).wrapping_mul(0x45d9f3b);
    h ^= h >> 16;
    h = h.wrapping_mul(0x85a0fc29);
    h ^= h >> 16;
    (h & 0xffff) as f32 / 65535.0
}

#[inline(always)]
fn smooth_noise1d(x: f32) -> f32 {
    let ix = x.floor() as i32;
    let fx = x - x.floor();
    let wx = fx * fx * (3.0 - 2.0 * fx);
    let v0 = hash1d(ix);
    let v1 = hash1d(ix + 1);
    v0 + wx * (v1 - v0)
}

fn mountain_fbm1d(x: f32, seed: f32) -> f32 {
    let mut total = 0.0;
    let mut amp = 0.52;
    let mut freq = 1.0;
    let px = x + seed;
    for _ in 0..5 {
        let n = smooth_noise1d(px * freq);
        let ridge = 1.0 - 2.0 * (n - 0.5).abs();
        let smooth = smooth_noise1d(px * freq + 17.3);
        total += amp * (0.70 * ridge + 0.30 * smooth);
        freq *= 2.05;
        amp *= 0.48;
    }
    total
}

#[inline(always)]
fn hash2d(ix: i32, iy: i32) -> f32 {
    let mut h = (ix as u32).wrapping_mul(0x85a0fc29) ^ (iy as u32).wrapping_mul(0x94d049bb);
    h ^= h >> 16;
    h = h.wrapping_mul(0x45d9f3b);
    h ^= h >> 16;
    (h & 0xffff) as f32 / 65535.0
}

#[inline(always)]
fn smooth_noise2d(x: f32, y: f32) -> f32 {
    let ix = x.floor() as i32;
    let iy = y.floor() as i32;
    let fx = x - x.floor();
    let fy = y - y.floor();
    let wx = fx * fx * (3.0 - 2.0 * fx);
    let wy = fy * fy * (3.0 - 2.0 * fy);

    let v00 = hash2d(ix, iy);
    let v10 = hash2d(ix + 1, iy);
    let v01 = hash2d(ix, iy + 1);
    let v11 = hash2d(ix + 1, iy + 1);

    let v0 = v00 + wx * (v10 - v00);
    let v1 = v01 + wx * (v11 - v01);
    v0 + wy * (v1 - v0)
}
