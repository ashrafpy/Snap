//! Phase 2 WGSL Compute Shaders & Uniform Layouts for Snap GPU Engine.
//!
//! Provides uniform definitions, helper constructors from `snap_core::ops_ext`,
//! and WGSL compute shader sources for all Phase 2 creative and retouch tools:
//!
//! 1. Color Grading (3-Way + Global Color Wheels with Oklab tinting)
//! 2. Dehaze (Dark-channel atmospheric veil recovery)
//! 3. Tonal Contrast (2-pass multi-scale bilateral luminance contrast)
//! 4. Halation (2-pass film emulsion red-orange scatter)
//! 5. Bloom (2-pass Pro-Mist soft highlight diffusion)
//! 6. Grainy Film (8 distinct film stock profiles with organic dual-octave grain)
//! 7. Noir (Dramatic silver-gelatin monochrome with 4 presets)
//! 8. Selective (Nik U-Point perceptual color selectivity with up to 16 pins)
//! 9. Brush (4-channel local retouching with manual bilinear sampling)
//! 10. Mask Blend (Layer mask blending with pre/post texture interpolation)

use bytemuck::{Pod, Zeroable};
use snap_core::ops_ext::{
    BloomParams, BrushChannel, BrushParams, ColorGradeParams, ColorWheelValue, DehazeParams,
    FilmStock, GrainyFilmParams, HalationParams, NoirParams, NoirPreset, SelectiveParams,
    TonalContrastParams,
};

// ============================================================================
// 1. Color Grading
// ============================================================================

/// Single color wheel value formatted for GPU uniform alignment (16 bytes).
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct GpuColorWheel {
    pub hue_deg: f32,
    pub saturation: f32,
    pub luminance: f32,
    pub _pad: f32,
}

impl GpuColorWheel {
    pub fn from_value(val: &ColorWheelValue) -> Self {
        Self {
            hue_deg: val.hue_deg,
            saturation: val.saturation,
            luminance: val.luminance,
            _pad: 0.0,
        }
    }
}

/// Uniforms for Color Grade compute shader (80 bytes = 5 x 16-byte blocks).
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct ColorGradeUniforms {
    pub shadows: GpuColorWheel,
    pub midtones: GpuColorWheel,
    pub highlights: GpuColorWheel,
    pub global: GpuColorWheel,
    pub balance: f32,
    pub blending: f32,
    pub opacity: f32,
    pub _pad: f32,
}

impl ColorGradeUniforms {
    pub fn from_params(params: &ColorGradeParams, opacity: f32) -> Self {
        Self {
            shadows: GpuColorWheel::from_value(&params.shadows),
            midtones: GpuColorWheel::from_value(&params.midtones),
            highlights: GpuColorWheel::from_value(&params.highlights),
            global: GpuColorWheel::from_value(&params.global),
            balance: params.balance,
            blending: params.blending,
            opacity,
            _pad: 0.0,
        }
    }
}

pub const COLOR_GRADE_WGSL: &str = r#"
struct ColorWheelGpu {
    hue_deg: f32,
    saturation: f32,
    luminance: f32,
    _pad: f32,
};

struct ColorGradeUniforms {
    shadows: ColorWheelGpu,
    midtones: ColorWheelGpu,
    highlights: ColorWheelGpu,
    global: ColorWheelGpu,
    balance: f32,
    blending: f32,
    opacity: f32,
    _pad: f32,
};

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var dst_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var<uniform> u: ColorGradeUniforms;

fn color_wheel_tint(wheel: ColorWheelGpu) -> vec2<f32> {
    if (wheel.saturation <= 0.0) {
        return vec2<f32>(0.0, 0.0);
    }
    let rad = wheel.hue_deg * (3.141592653589793 / 180.0);
    let chroma = (wheel.saturation / 100.0) * 0.22;
    return vec2<f32>(chroma * cos(rad), chroma * sin(rad));
}

@compute @workgroup_size(16, 16, 1)
fn cs_color_grade(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dim = textureDimensions(src_tex);
    if (gid.x >= dim.x || gid.y >= dim.y) {
        return;
    }
    let coord = vec2<i32>(i32(gid.x), i32(gid.y));
    let in_px = textureLoad(src_tex, coord, 0);
    let rgb = in_px.rgb;

    // Preserving HDR dynamic range
    let hdr_scale = max(1.0, max(rgb.r, max(rgb.g, rgb.b)));
    let norm_rgb = rgb / hdr_scale;

    let luma = rec2020_luma(norm_rgb);
    let l_norm = luma / (1.0 + luma);

    // Crossover thresholds adjusted by balance (-100..+100)
    let bal_shift = (u.balance / 100.0) * 0.25;
    let crossover_low = clamp(0.33 + bal_shift, 0.05, 0.65);
    let crossover_high = clamp(0.66 + bal_shift, 0.35, 0.95);

    // Blending controls zone overlap smoothness (0..100)
    let blend_w = mix(0.08, 0.45, u.blending / 100.0);

    let w_shadows = 1.0 - smoothstep(crossover_low - blend_w, crossover_low + blend_w, l_norm);
    let w_highlights = smoothstep(crossover_high - blend_w, crossover_high + blend_w, l_norm);
    let w_midtones = max(0.0, 1.0 - w_shadows - w_highlights);

    let total_w = max(w_shadows + w_midtones + w_highlights, 1e-4);
    let ws = w_shadows / total_w;
    let wm = w_midtones / total_w;
    let wh = w_highlights / total_w;

    // Chroma tinting in Oklab space
    var lab = linear_rec2020_to_oklab(norm_rgb);

    let tint_s = color_wheel_tint(u.shadows);
    let tint_m = color_wheel_tint(u.midtones);
    let tint_h = color_wheel_tint(u.highlights);
    let tint_g = color_wheel_tint(u.global);

    let tint_ab = ws * tint_s + wm * tint_m + wh * tint_h + tint_g;
    lab.y += tint_ab.x;
    lab.z += tint_ab.y;

    // Zone luminance lift, gamma, gain, and global exposure
    var l_val = lab.x;
    if (u.shadows.luminance != 0.0) {
        l_val += (u.shadows.luminance / 100.0) * 0.15 * ws;
    }
    if (u.midtones.luminance != 0.0) {
        l_val *= (1.0 + (u.midtones.luminance / 100.0) * 0.30 * wm);
    }
    if (u.highlights.luminance != 0.0) {
        l_val *= (1.0 + (u.highlights.luminance / 100.0) * 0.30 * wh);
    }
    if (u.global.luminance != 0.0) {
        l_val *= (1.0 + (u.global.luminance / 100.0) * 0.30);
    }
    lab.x = max(l_val, 0.0);

    let graded_norm = oklab_to_linear_rec2020(lab);
    let graded_rgb = graded_norm * hdr_scale;

    let out_rgb = mix(in_px.rgb, graded_rgb, u.opacity);
    textureStore(dst_tex, coord, vec4<f32>(out_rgb, in_px.a));
}
"#;

// ============================================================================
// 2. Dehaze
// ============================================================================

/// Uniforms for Dehaze compute shader (16 bytes).
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct DehazeUniforms {
    pub strength: f32,
    pub warmth_offset: f32,
    pub opacity: f32,
    pub _pad: f32,
}

impl DehazeUniforms {
    pub fn from_params(params: &DehazeParams, opacity: f32) -> Self {
        Self {
            strength: params.strength,
            warmth_offset: params.warmth_offset,
            opacity,
            _pad: 0.0,
        }
    }
}

pub const DEHAZE_WGSL: &str = r#"
struct DehazeUniforms {
    strength: f32,
    warmth_offset: f32,
    opacity: f32,
    _pad: f32,
};

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var dst_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var<uniform> u: DehazeUniforms;

@compute @workgroup_size(16, 16, 1)
fn cs_dehaze(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dim = textureDimensions(src_tex);
    if (gid.x >= dim.x || gid.y >= dim.y) {
        return;
    }
    let coord = vec2<i32>(i32(gid.x), i32(gid.y));
    let in_px = textureLoad(src_tex, coord, 0);
    let rgb = in_px.rgb;

    // 5x5 sparse neighborhood (step = 3 pixels) to estimate dark channel
    var min_dark = 10000.0;
    for (var dy = -2; dy <= 2; dy = dy + 1) {
        for (var dx = -2; dx <= 2; dx = dx + 1) {
            let sample_coord = clamp(
                coord + vec2<i32>(dx * 3, dy * 3),
                vec2<i32>(0, 0),
                vec2<i32>(i32(dim.x) - 1, i32(dim.y) - 1)
            );
            let s_px = textureLoad(src_tex, sample_coord, 0).rgb;
            let ch_min = min(s_px.r, min(s_px.g, s_px.b));
            min_dark = min(min_dark, ch_min);
        }
    }
    let dark_norm = clamp(min_dark, 0.0, 1.0);

    // Atmospheric airlight estimation in linear Rec.2020
    let airlight = vec3<f32>(0.85, 0.90, 0.95);

    var dehazed_rgb = rgb;
    if (u.strength >= 0.0) {
        // Recover scene radiance: (rgb - airlight * (1.0 - t)) / t
        let k = (u.strength / 100.0) * 0.85;
        let t = clamp(1.0 - k * dark_norm, 0.22, 1.0);

        // HDR safety: separate [0, 1] base and hdr excess
        let hdr_excess = max(rgb - vec3<f32>(1.0), vec3<f32>(0.0));
        let base_rgb = clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0));
        let recovered_base = max(vec3<f32>(0.0), (base_rgb - airlight * (1.0 - t)) / t);
        dehazed_rgb = recovered_base + hdr_excess;
    } else {
        // Negative strength: add atmospheric veil/haze
        let haze_amt = (-u.strength / 100.0) * 0.6;
        let luma = rec2020_luma(rgb);
        let depth_factor = 1.0 - clamp(luma * 0.5, 0.0, 0.8);
        dehazed_rgb = mix(rgb, airlight, haze_amt * depth_factor);
    }

    // Warmth offset via white balance adaptation
    if (abs(u.warmth_offset) > 1e-4) {
        dehazed_rgb = apply_white_balance_rec2020(dehazed_rgb, u.warmth_offset, 0.0);
    }

    let out_rgb = mix(in_px.rgb, dehazed_rgb, u.opacity);
    textureStore(dst_tex, coord, vec4<f32>(out_rgb, in_px.a));
}
"#;

// ============================================================================
// 3. Tonal Contrast
// ============================================================================

/// Uniforms for Tonal Contrast 2-pass compute shader (32 bytes).
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct TonalContrastUniforms {
    pub high_tones: f32,
    pub mid_tones: f32,
    pub low_tones: f32,
    pub protect_shadows: f32,
    pub protect_highlights: f32,
    pub opacity: f32,
    pub pass_idx: u32,
    pub _pad: u32,
}

impl TonalContrastUniforms {
    pub fn from_params(params: &TonalContrastParams, opacity: f32, pass_idx: u32) -> Self {
        Self {
            high_tones: params.high_tones,
            mid_tones: params.mid_tones,
            low_tones: params.low_tones,
            protect_shadows: params.protect_shadows,
            protect_highlights: params.protect_highlights,
            opacity,
            pass_idx,
            _pad: 0,
        }
    }
}

pub const TONAL_CONTRAST_WGSL: &str = r#"
struct TonalContrastUniforms {
    high_tones: f32,
    mid_tones: f32,
    low_tones: f32,
    protect_shadows: f32,
    protect_highlights: f32,
    opacity: f32,
    pass_idx: u32,
    _pad: u32,
};

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var dst_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var<uniform> u: TonalContrastUniforms;

@compute @workgroup_size(16, 16, 1)
fn cs_tonal_contrast(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dim = textureDimensions(src_tex);
    if (gid.x >= dim.x || gid.y >= dim.y) {
        return;
    }
    let coord = vec2<i32>(i32(gid.x), i32(gid.y));
    let center_px = textureLoad(src_tex, coord, 0);
    let center_luma = rec2020_luma(center_px.rgb);

    let sigma_s = 3.5;
    let sigma_r = 0.20;
    let two_sigma_s2 = 2.0 * sigma_s * sigma_s;
    let two_sigma_r2 = 2.0 * sigma_r * sigma_r;

    if (u.pass_idx == 0u) {
        // Horizontal multi-scale Gaussian/bilateral luminance filter across -8..=8
        var sum_luma = 0.0;
        var sum_w = 0.0;

        for (var k = -8; k <= 8; k = k + 1) {
            let tap_x = clamp(coord.x + k, 0, i32(dim.x) - 1);
            let tap_px = textureLoad(src_tex, vec2<i32>(tap_x, coord.y), 0);
            let tap_luma = rec2020_luma(tap_px.rgb);

            let d_spatial = f32(k);
            let d_range = tap_luma - center_luma;
            let w = exp(- (d_spatial * d_spatial) / two_sigma_s2 - (d_range * d_range) / two_sigma_r2);

            sum_luma += tap_luma * w;
            sum_w += w;
        }

        let blur_h = sum_luma / max(sum_w, 1e-5);
        // Store center rgb untouched, horizontal blur into .a
        textureStore(dst_tex, coord, vec4<f32>(center_px.rgb, blur_h));
    } else {
        // Vertical filter on .a channel
        var sum_luma = 0.0;
        var sum_w = 0.0;
        let center_luma_h = center_px.a;

        for (var k = -8; k <= 8; k = k + 1) {
            let tap_y = clamp(coord.y + k, 0, i32(dim.y) - 1);
            let tap_px = textureLoad(src_tex, vec2<i32>(coord.x, tap_y), 0);
            let tap_luma = tap_px.a;

            let d_spatial = f32(k);
            let d_range = tap_luma - center_luma_h;
            let w = exp(- (d_spatial * d_spatial) / two_sigma_s2 - (d_range * d_range) / two_sigma_r2);

            sum_luma += tap_luma * w;
            sum_w += w;
        }

        let luma_base = sum_luma / max(sum_w, 1e-5);
        let detail = center_luma - luma_base;

        // Partition into tonal bands based on normalized luminance
        let l_norm = center_luma / (1.0 + center_luma);

        // Low tones (< 0.25)
        let w_low = 1.0 - smoothstep(0.10, 0.25, l_norm);
        // Mid tones (0.15..0.75)
        let w_mid = smoothstep(0.10, 0.25, l_norm) * (1.0 - smoothstep(0.55, 0.75, l_norm));
        // High tones (> 0.55)
        let w_high = smoothstep(0.55, 0.75, l_norm);

        let boost = (u.low_tones / 100.0) * w_low
                  + (u.mid_tones / 100.0) * w_mid
                  + (u.high_tones / 100.0) * w_high;

        // Protection masks
        let shadow_protect = mix(1.0, smoothstep(0.01, 0.15, l_norm), u.protect_shadows / 100.0);
        let highlight_protect = mix(1.0, 1.0 - smoothstep(0.70, 0.95, l_norm), u.protect_highlights / 100.0);
        let atten = shadow_protect * highlight_protect;

        let luma_final = center_luma + detail * boost * atten;
        let ratio = max(0.0, luma_final) / max(center_luma, 1e-5);
        let out_rgb = mix(center_px.rgb, center_px.rgb * ratio, u.opacity);

        textureStore(dst_tex, coord, vec4<f32>(out_rgb, 1.0));
    }
}
"#;

// ============================================================================
// 4. Halation
// ============================================================================

/// Uniforms for Halation 2-pass compute shader (32 bytes).
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct HalationUniforms {
    pub threshold: f32,
    pub strength: f32,
    pub spread: f32,
    pub opacity: f32,
    pub pass_idx: u32,
    pub _pad0: u32,
    pub _pad1: u32,
    pub _pad2: u32,
}

impl HalationUniforms {
    pub fn from_params(params: &HalationParams, opacity: f32, pass_idx: u32) -> Self {
        Self {
            threshold: params.threshold,
            strength: params.strength,
            spread: params.spread,
            opacity,
            pass_idx,
            _pad0: 0,
            _pad1: 0,
            _pad2: 0,
        }
    }
}

pub const HALATION_WGSL: &str = r#"
struct HalationUniforms {
    threshold: f32,
    strength: f32,
    spread: f32,
    opacity: f32,
    pass_idx: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
};

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var dst_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var<uniform> u: HalationUniforms;

@compute @workgroup_size(16, 16, 1)
fn cs_halation(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dim = textureDimensions(src_tex);
    if (gid.x >= dim.x || gid.y >= dim.y) {
        return;
    }
    let coord = vec2<i32>(i32(gid.x), i32(gid.y));
    let center_px = textureLoad(src_tex, coord, 0);

    let step_val = max(1, i32(round(1.0 + (u.spread / 100.0) * 2.0)));
    let sigma = max(1.0, (u.spread / 100.0) * 8.0 + 2.0);
    let two_sigma2 = 2.0 * sigma * sigma;
    let thresh_val = (u.threshold / 100.0) * 0.8 + 0.1;

    if (u.pass_idx == 0u) {
        // Horizontal pass: extract highlight energy and blur into .a
        var sum_energy = 0.0;
        var sum_w = 0.0;

        for (var k = -14; k <= 14; k = k + 1) {
            let tap_x = clamp(coord.x + k * step_val, 0, i32(dim.x) - 1);
            let tap_px = textureLoad(src_tex, vec2<i32>(tap_x, coord.y), 0);
            let tap_luma = rec2020_luma(tap_px.rgb);

            let excess = max(0.0, tap_luma - thresh_val);
            let energy = excess + max(0.0, tap_luma - 1.0) * 0.5;

            let dist = f32(k * step_val);
            let w = exp(- (dist * dist) / two_sigma2);

            sum_energy += energy * w;
            sum_w += w;
        }

        let blur_h = sum_energy / max(sum_w, 1e-5);
        textureStore(dst_tex, coord, vec4<f32>(center_px.rgb, blur_h));
    } else {
        // Vertical pass on .a channel -> halo
        var sum_halo = 0.0;
        var sum_w = 0.0;

        for (var k = -14; k <= 14; k = k + 1) {
            let tap_y = clamp(coord.y + k * step_val, 0, i32(dim.y) - 1);
            let tap_px = textureLoad(src_tex, vec2<i32>(coord.x, tap_y), 0);
            let tap_energy = tap_px.a;

            let dist = f32(k * step_val);
            let w = exp(- (dist * dist) / two_sigma2);

            sum_halo += tap_energy * w;
            sum_w += w;
        }

        let halo = sum_halo / max(sum_w, 1e-5);

        // Authentic film red-orange emulsion scatter
        let scatter_tint = SRGB_TO_REC2020 * vec3<f32>(1.0, 0.20, 0.03);
        let halation_rgb = scatter_tint * (halo * (u.strength / 100.0) * 1.5);

        let blended = center_px.rgb + halation_rgb;
        let out_rgb = mix(center_px.rgb, blended, u.opacity);

        textureStore(dst_tex, coord, vec4<f32>(out_rgb, 1.0));
    }
}
"#;

// ============================================================================
// 5. Bloom
// ============================================================================

/// Uniforms for Bloom 2-pass compute shader (32 bytes).
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct BloomUniforms {
    pub strength: f32,
    pub size: f32,
    pub highlight_range: f32,
    pub opacity: f32,
    pub pass_idx: u32,
    pub _pad0: u32,
    pub _pad1: u32,
    pub _pad2: u32,
}

impl BloomUniforms {
    pub fn from_params(params: &BloomParams, opacity: f32, pass_idx: u32) -> Self {
        Self {
            strength: params.strength,
            size: params.size,
            highlight_range: params.highlight_range,
            opacity,
            pass_idx,
            _pad0: 0,
            _pad1: 0,
            _pad2: 0,
        }
    }
}

pub const BLOOM_WGSL: &str = r#"
struct BloomUniforms {
    strength: f32,
    size: f32,
    highlight_range: f32,
    opacity: f32,
    pass_idx: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
};

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var dst_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var<uniform> u: BloomUniforms;

@compute @workgroup_size(16, 16, 1)
fn cs_bloom(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dim = textureDimensions(src_tex);
    if (gid.x >= dim.x || gid.y >= dim.y) {
        return;
    }
    let coord = vec2<i32>(i32(gid.x), i32(gid.y));
    let center_px = textureLoad(src_tex, coord, 0);

    let step_val = max(1, i32(round(1.0 + (u.size / 100.0) * 3.0)));
    let sigma = max(1.0, (u.size / 100.0) * 10.0 + 2.0);
    let two_sigma2 = 2.0 * sigma * sigma;

    let thresh_min = mix(0.08, 0.42, 1.0 - (u.highlight_range / 100.0));
    let thresh_max = thresh_min + 0.18;

    if (u.pass_idx == 0u) {
        // Horizontal pass: extract soft Pro-Mist highlight luminance into .a
        var sum_lum = 0.0;
        var sum_w = 0.0;

        for (var k = -12; k <= 12; k = k + 1) {
            let tap_x = clamp(coord.x + k * step_val, 0, i32(dim.x) - 1);
            let tap_px = textureLoad(src_tex, vec2<i32>(tap_x, coord.y), 0);
            let tap_luma = rec2020_luma(tap_px.rgb);

            let norm_l = tap_luma / (1.0 + tap_luma);
            let h_weight = smoothstep(thresh_min, thresh_max, norm_l);
            let hdr_excess = max(0.0, tap_luma - 1.0);
            let glow_lum = tap_luma * h_weight + hdr_excess;

            let dist = f32(k * step_val);
            let w = exp(- (dist * dist) / two_sigma2);

            sum_lum += glow_lum * w;
            sum_w += w;
        }

        let blur_h = sum_lum / max(sum_w, 1e-5);
        textureStore(dst_tex, coord, vec4<f32>(center_px.rgb, blur_h));
    } else {
        // Vertical pass on .a channel -> glow
        var sum_glow = 0.0;
        var sum_w = 0.0;

        for (var k = -12; k <= 12; k = k + 1) {
            let tap_y = clamp(coord.y + k * step_val, 0, i32(dim.y) - 1);
            let tap_px = textureLoad(src_tex, vec2<i32>(coord.x, tap_y), 0);
            let tap_lum = tap_px.a;

            let dist = f32(k * step_val);
            let w = exp(- (dist * dist) / two_sigma2);

            sum_glow += tap_lum * w;
            sum_w += w;
        }

        let glow = sum_glow / max(sum_w, 1e-5);

        // Soft warm diffusion tint
        let glow_tint = SRGB_TO_REC2020 * vec3<f32>(1.0, 0.94, 0.86);
        let glow_rgb = glow_tint * (glow * (u.strength / 100.0) * 0.85);

        let blended = center_px.rgb + glow_rgb;
        let out_rgb = mix(center_px.rgb, blended, u.opacity);

        textureStore(dst_tex, coord, vec4<f32>(out_rgb, 1.0));
    }
}
"#;

// ============================================================================
// 6. Grainy Film
// ============================================================================

/// Uniforms for Grainy Film compute shader (16 bytes).
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct GrainyFilmUniforms {
    pub stock: u32,
    pub grain: f32,
    pub style_strength: f32,
    pub opacity: f32,
}

impl GrainyFilmUniforms {
    pub fn from_params(params: &GrainyFilmParams, opacity: f32) -> Self {
        let stock = match params.stock {
            FilmStock::K100 => 0u32,
            FilmStock::K400 => 1u32,
            FilmStock::F200 => 2u32,
            FilmStock::Velvia50 => 3u32,
            FilmStock::Cinestill800T => 4u32,
            FilmStock::IlfordHP5 => 5u32,
            FilmStock::CrossProcess => 6u32,
            FilmStock::Polaroid => 7u32,
        };
        Self {
            stock,
            grain: params.grain,
            style_strength: params.style_strength,
            opacity,
        }
    }
}

pub const GRAINY_FILM_WGSL: &str = r#"
struct GrainyFilmUniforms {
    stock: u32,
    grain: f32,
    style_strength: f32,
    opacity: f32,
};

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var dst_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var<uniform> u: GrainyFilmUniforms;

fn film_hash2(p: vec2<f32>) -> f32 {
    let p3 = fract(vec3<f32>(p.xyx) * 0.1031);
    let p4 = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p4.x + p4.y) * p4.z) * 2.0 - 1.0;
}

@compute @workgroup_size(16, 16, 1)
fn cs_grainy_film(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dim = textureDimensions(src_tex);
    if (gid.x >= dim.x || gid.y >= dim.y) {
        return;
    }
    let coord = vec2<i32>(i32(gid.x), i32(gid.y));
    let in_px = textureLoad(src_tex, coord, 0);
    let rgb = in_px.rgb;

    // Preserve HDR dynamic range
    let hdr_scale = max(1.0, max(rgb.r, max(rgb.g, rgb.b)));
    let norm_rgb = rgb / hdr_scale;
    var lab = linear_rec2020_to_oklab(norm_rgb);

    let strength = u.style_strength / 100.0;

    // Characteristic film toe/shoulder S-curve
    let orig_L = lab.x;
    let curved_L = smoothstep(-0.04, 1.04, orig_L);
    lab.x = mix(orig_L, curved_L, strength * 0.45);

    // Stock-specific color profiles (0..7)
    if (u.stock == 0u) {
        // Kodak Ektar 100: Rich warm red/yellow, punchy saturation
        lab.y += 0.015 * strength;
        lab.z += 0.025 * strength;
        lab.y *= (1.0 + 0.18 * strength);
        lab.z *= (1.0 + 0.18 * strength);
    } else if (u.stock == 1u) {
        // Kodak Portra 400: Flattering soft skin, lifted warm shadows
        let shadow_mask = 1.0 - smoothstep(0.0, 0.40, lab.x);
        lab.x += shadow_mask * 0.035 * strength;
        lab.y += 0.010 * strength;
        lab.z += 0.016 * strength;
        lab.y *= (1.0 - 0.05 * strength);
        lab.z *= (1.0 - 0.05 * strength);
    } else if (u.stock == 2u) {
        // Fuji Superia 200: Emerald green tint, cool cyan shadows
        lab.y -= 0.012 * strength;
        let shadow_mask = 1.0 - smoothstep(0.0, 0.45, lab.x);
        lab.z -= (0.010 + 0.020 * shadow_mask) * strength;
    } else if (u.stock == 3u) {
        // Fuji Velvia 50: Hyper-vivid landscape slide film, saturated chroma
        lab.y *= (1.0 + 0.42 * strength);
        lab.z *= (1.0 + 0.42 * strength);
        lab.x = 0.5 + (lab.x - 0.5) * (1.0 + 0.22 * strength);
    } else if (u.stock == 4u) {
        // CineStill 800T: Cool tungsten blue, warm glowing highlight tint
        lab.z -= 0.038 * strength;
        let h_mask = smoothstep(0.50, 0.95, lab.x);
        lab.y += 0.025 * h_mask * strength;
        lab.z += 0.025 * h_mask * strength;
    } else if (u.stock == 5u) {
        // Ilford HP5: Silver monochrome, high contrast, subtle selenium split
        lab.y = 0.0;
        lab.z = 0.0;
        lab.x = 0.5 + (lab.x - 0.5) * (1.0 + 0.28 * strength);
        let s_mask = 1.0 - smoothstep(0.0, 0.40, lab.x);
        lab.y += 0.005 * s_mask * strength;
        lab.z -= 0.005 * s_mask * strength;
    } else if (u.stock == 6u) {
        // Cross Process: E-6 in C-41, yellow-magenta shadows, cyan highlights
        let h_mask = smoothstep(0.40, 0.90, lab.x);
        let s_mask = 1.0 - smoothstep(0.10, 0.60, lab.x);
        lab.y += (0.045 * s_mask - 0.030 * h_mask) * strength;
        lab.z += (0.055 * s_mask - 0.045 * h_mask) * strength;
        lab.x = 0.5 + (lab.x - 0.5) * (1.0 + 0.32 * strength);
    } else if (u.stock == 7u) {
        // Polaroid: Milky faded blacks, desaturated, warm instant print
        lab.x = lab.x * 0.92 + 0.055 * strength;
        lab.y -= 0.007 * strength;
        lab.z += 0.032 * strength;
        lab.y *= (1.0 - 0.12 * strength);
        lab.z *= (1.0 - 0.12 * strength);
    }

    lab.x = max(lab.x, 0.0);
    var film_rgb = oklab_to_linear_rec2020(lab) * hdr_scale;

    // Organic midtone-weighted dual-octave spatial hash film grain
    if (u.grain > 0.0) {
        let coord_f = vec2<f32>(f32(coord.x), f32(coord.y));
        let n1 = film_hash2(coord_f);
        let n2 = film_hash2(coord_f * 0.5 + vec2<f32>(123.4, 567.8));
        let noise_val = n1 * 0.7 + n2 * 0.3;

        let luma_norm = clamp(rec2020_luma(film_rgb) / max(hdr_scale, 1.0), 0.0, 1.0);
        let grain_w = 4.0 * luma_norm * (1.0 - luma_norm);
        let grain_scale = (u.grain / 100.0) * 0.075 * grain_w;

        film_rgb = max(vec3<f32>(0.0), film_rgb + vec3<f32>(noise_val * grain_scale));
    }

    let out_rgb = mix(in_px.rgb, film_rgb, u.opacity);
    textureStore(dst_tex, coord, vec4<f32>(out_rgb, in_px.a));
}
"#;

// ============================================================================
// 7. Noir
// ============================================================================

/// Uniforms for Noir compute shader (32 bytes).
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct NoirUniforms {
    pub preset: u32,
    pub brightness: f32,
    pub contrast: f32,
    pub wash: f32,
    pub grain: f32,
    pub opacity: f32,
    pub _pad0: u32,
    pub _pad1: u32,
}

impl NoirUniforms {
    pub fn from_params(params: &NoirParams, opacity: f32) -> Self {
        let preset = match params.preset {
            NoirPreset::Dark => 0u32,
            NoirPreset::Light => 1u32,
            NoirPreset::Film => 2u32,
            NoirPreset::Harsh => 3u32,
        };
        Self {
            preset,
            brightness: params.brightness,
            contrast: params.contrast,
            wash: params.wash,
            grain: params.grain,
            opacity,
            _pad0: 0,
            _pad1: 0,
        }
    }
}

pub const NOIR_WGSL: &str = r#"
struct NoirUniforms {
    preset: u32,
    brightness: f32,
    contrast: f32,
    wash: f32,
    grain: f32,
    opacity: f32,
    _pad0: u32,
    _pad1: u32,
};

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var dst_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var<uniform> u: NoirUniforms;

@compute @workgroup_size(16, 16, 1)
fn cs_noir(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dim = textureDimensions(src_tex);
    if (gid.x >= dim.x || gid.y >= dim.y) {
        return;
    }
    let coord = vec2<i32>(i32(gid.x), i32(gid.y));
    let in_px = textureLoad(src_tex, coord, 0);

    let srgb = max(REC2020_TO_SRGB * in_px.rgb, vec3<f32>(0.0));

    // Preset-specific spectral sensitivity weights
    var weights = vec3<f32>(0.40, 0.45, 0.15);
    var base_contrast = 0.0;
    var base_bright = 0.0;

    if (u.preset == 0u) {
        // Dark: heavy red panchromatic sensitivity, deep shadows
        weights = vec3<f32>(0.65, 0.30, 0.05);
        base_contrast = 0.20;
    } else if (u.preset == 1u) {
        // Light: luminous skin, high-key midtones
        weights = vec3<f32>(0.35, 0.55, 0.10);
        base_bright = 0.15;
    } else if (u.preset == 2u) {
        // Film: Tri-X balanced silver halide
        weights = vec3<f32>(0.40, 0.45, 0.15);
        base_contrast = 0.10;
    } else if (u.preset == 3u) {
        // Harsh: stark chiaroscuro noir, high contrast
        weights = vec3<f32>(0.70, 0.25, 0.05);
        base_contrast = 0.55;
    }

    var bw = dot(srgb, weights);

    // Brightness
    let eff_bright = (u.brightness / 100.0) + base_bright;
    bw = bw * exp2(eff_bright);

    // S-curve contrast
    let eff_contrast = (u.contrast / 100.0) * 0.65 + base_contrast;
    bw = clamp(0.5 + (bw - 0.5) * (1.0 + eff_contrast), 0.0, 1.5);

    // Matte black floor wash
    if (u.wash > 0.0) {
        let wash_lift = (u.wash / 100.0) * 0.12;
        bw = mix(bw, max(bw, wash_lift), 1.0 - smoothstep(0.0, 0.35, bw));
    }

    var out_mono = vec3<f32>(bw, bw, bw);

    // Subtle selenium/cool silver split-tint for Film preset
    if (u.preset == 2u) {
        let cool_tint = SRGB_TO_REC2020 * vec3<f32>(0.96, 0.98, 1.02);
        let warm_tint = SRGB_TO_REC2020 * vec3<f32>(1.02, 0.99, 0.96);
        let split = mix(warm_tint, cool_tint, smoothstep(0.20, 0.80, bw));
        out_mono = out_mono * split;
    }

    // Midtone-weighted organic film grain
    if (u.grain > 0.0) {
        let n = (fract(sin(dot(vec2<f32>(f32(coord.x), f32(coord.y)), vec2<f32>(12.9898, 78.233))) * 43758.5453) - 0.5) * 2.0;
        let mid_mask = 4.0 * clamp(bw, 0.0, 1.0) * (1.0 - clamp(bw, 0.0, 1.0));
        out_mono = max(vec3<f32>(0.0), out_mono + vec3<f32>(n * (u.grain / 100.0) * 0.08 * mid_mask));
    }

    // Preserve original HDR highlight headroom above 1.0
    let orig_excess = max(in_px.rgb - vec3<f32>(1.0), vec3<f32>(0.0));
    out_mono = out_mono + orig_excess;

    let out_rgb = mix(in_px.rgb, out_mono, u.opacity);
    textureStore(dst_tex, coord, vec4<f32>(out_rgb, in_px.a));
}
"#;

// ============================================================================
// 8. Selective (Nik U-Point)
// ============================================================================

/// Single selective control point (32 bytes = 8 x 4-byte floats).
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct GpuSelectivePin {
    pub x: f32,
    pub y: f32,
    pub radius: f32,
    pub brightness: f32,
    pub contrast: f32,
    pub saturation: f32,
    pub structure: f32,
    pub _pad: f32,
}

impl Default for GpuSelectivePin {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            radius: 0.0,
            brightness: 0.0,
            contrast: 0.0,
            saturation: 0.0,
            structure: 0.0,
            _pad: 0.0,
        }
    }
}

/// Uniforms for Selective compute shader (528 bytes = 16 * 32 + 16 bytes).
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct SelectiveUniforms {
    pub pins: [GpuSelectivePin; 16],
    pub pin_count: u32,
    pub opacity: f32,
    pub _pad: [u32; 2],
}

impl SelectiveUniforms {
    pub fn from_params(params: &SelectiveParams, opacity: f32) -> Self {
        let mut pins = [GpuSelectivePin::default(); 16];
        let count = params.pins.len().min(16);
        for i in 0..count {
            let p = &params.pins[i];
            pins[i] = GpuSelectivePin {
                x: p.x,
                y: p.y,
                radius: p.radius,
                brightness: p.brightness,
                contrast: p.contrast,
                saturation: p.saturation,
                structure: p.structure,
                _pad: 0.0,
            };
        }
        Self {
            pins,
            pin_count: count as u32,
            opacity,
            _pad: [0, 0],
        }
    }
}

pub const SELECTIVE_WGSL: &str = r#"
struct SelectivePinGpu {
    x: f32,
    y: f32,
    radius: f32,
    brightness: f32,
    contrast: f32,
    saturation: f32,
    structure: f32,
    _pad: f32,
};

struct SelectiveUniforms {
    pins: array<SelectivePinGpu, 16>,
    pin_count: u32,
    opacity: f32,
    _pad0: u32,
    _pad1: u32,
};

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var dst_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var<uniform> u: SelectiveUniforms;

@compute @workgroup_size(16, 16, 1)
fn cs_selective(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dim = textureDimensions(src_tex);
    if (gid.x >= dim.x || gid.y >= dim.y) {
        return;
    }
    let coord = vec2<i32>(i32(gid.x), i32(gid.y));
    let in_px = textureLoad(src_tex, coord, 0);

    if (u.pin_count == 0u) {
        textureStore(dst_tex, coord, in_px);
        return;
    }

    let uv = (vec2<f32>(f32(coord.x) + 0.5, f32(coord.y) + 0.5)) / vec2<f32>(f32(dim.x), f32(dim.y));
    let aspect = f32(dim.x) / f32(dim.y);

    var cur_lab = linear_rec2020_to_oklab(in_px.rgb);

    // 5-tap Laplacian structure estimate
    let px_l = textureLoad(src_tex, vec2<i32>(max(0, coord.x - 1), coord.y), 0).rgb;
    let px_r = textureLoad(src_tex, vec2<i32>(min(i32(dim.x) - 1, coord.x + 1), coord.y), 0).rgb;
    let px_u = textureLoad(src_tex, vec2<i32>(coord.x, max(0, coord.y - 1)), 0).rgb;
    let px_d = textureLoad(src_tex, vec2<i32>(coord.x, min(i32(dim.y) - 1, coord.y + 1)), 0).rgb;
    let lap = 4.0 * cur_lab.x - (
        linear_rec2020_to_oklab(px_l).x +
        linear_rec2020_to_oklab(px_r).x +
        linear_rec2020_to_oklab(px_u).x +
        linear_rec2020_to_oklab(px_d).x
    );

    let count = min(u.pin_count, 16u);
    for (var i = 0u; i < count; i = i + 1u) {
        let pin = u.pins[i];

        // Aspect-corrected radial distance
        var delta_pos = uv - vec2<f32>(pin.x, pin.y);
        if (aspect >= 1.0) {
            delta_pos.x *= aspect;
        } else {
            delta_pos.y /= aspect;
        }
        let dist = length(delta_pos);
        let r = max(pin.radius, 1e-4);
        if (dist >= r) {
            continue;
        }
        let radial_w = 1.0 - smoothstep(0.0, r, dist);

        // Anchor pixel in Oklab space
        let anchor_coord = vec2<i32>(
            clamp(i32(round(pin.x * f32(dim.x))), 0, i32(dim.x) - 1),
            clamp(i32(round(pin.y * f32(dim.y))), 0, i32(dim.y) - 1)
        );
        let anchor_px = textureLoad(src_tex, anchor_coord, 0).rgb;
        let anchor_lab = linear_rec2020_to_oklab(anchor_px);

        // Perceptual color distance Delta E in Oklab
        let delta_lab = cur_lab - anchor_lab;
        let delta_e = length(delta_lab);
        let color_w = exp(- (delta_e * delta_e) / (0.18 * 0.18));
        let w = radial_w * color_w;

        // Apply local adjustments weighted by U-Point selectivity w
        if (pin.brightness != 0.0) {
            cur_lab.x += (pin.brightness / 100.0) * 0.30 * w;
        }
        if (pin.contrast != 0.0) {
            let c = (pin.contrast / 100.0) * 0.40 * w;
            cur_lab.x = 0.5 + (cur_lab.x - 0.5) * (1.0 + c);
        }
        if (pin.saturation != 0.0) {
            let sat_scale = max(0.0, 1.0 + (pin.saturation / 100.0) * w);
            cur_lab.y *= sat_scale;
            cur_lab.z *= sat_scale;
        }
        if (pin.structure != 0.0) {
            cur_lab.x += lap * (pin.structure / 100.0) * 0.50 * w;
        }
    }

    cur_lab.x = max(cur_lab.x, 0.0);
    let adj_rgb = oklab_to_linear_rec2020(cur_lab);

    // Preserve HDR dynamic range
    let hdr_excess = max(in_px.rgb - vec3<f32>(1.0), vec3<f32>(0.0));
    let final_rgb = mix(in_px.rgb, adj_rgb + hdr_excess, u.opacity);

    textureStore(dst_tex, coord, vec4<f32>(final_rgb, in_px.a));
}
"#;

// ============================================================================
// 9. Brush
// ============================================================================

/// Uniforms for Brush compute shader (16 bytes).
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct BrushUniforms {
    pub channel: u32,
    pub opacity: f32,
    pub _pad0: u32,
    pub _pad1: u32,
}

impl BrushUniforms {
    pub fn from_params(params: &BrushParams, opacity: f32) -> Self {
        let channel = match params.channel {
            BrushChannel::DodgeBurn => 0u32,
            BrushChannel::Exposure => 1u32,
            BrushChannel::Temperature => 2u32,
            BrushChannel::Saturation => 3u32,
        };
        Self {
            channel,
            opacity,
            _pad0: 0,
            _pad1: 0,
        }
    }
}

pub const BRUSH_WGSL: &str = r#"
struct BrushUniforms {
    channel: u32,
    opacity: f32,
    _pad0: u32,
    _pad1: u32,
};

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var dst_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var brush_tex: texture_2d<f32>;
@group(0) @binding(3) var<uniform> u: BrushUniforms;

fn sample_brush_bilinear(uv: vec2<f32>) -> f32 {
    let u_clamped = clamp(uv.x, 0.0, 1.0);
    let v_clamped = clamp(uv.y, 0.0, 1.0);

    let fx_coord = u_clamped * 255.0;
    let fy_coord = v_clamped * 255.0;

    let x0 = i32(floor(fx_coord));
    let y0 = i32(floor(fy_coord));
    let x1 = min(x0 + 1, 255);
    let y1 = min(y0 + 1, 255);

    let fx = fx_coord - f32(x0);
    let fy = fy_coord - f32(y0);

    let p00 = textureLoad(brush_tex, vec2<i32>(x0, y0), 0).r;
    let p10 = textureLoad(brush_tex, vec2<i32>(x1, y0), 0).r;
    let p01 = textureLoad(brush_tex, vec2<i32>(x0, y1), 0).r;
    let p11 = textureLoad(brush_tex, vec2<i32>(x1, y1), 0).r;

    let top = mix(p00, p10, fx);
    let bottom = mix(p01, p11, fx);
    return clamp(mix(top, bottom, fy), -1.0, 1.0);
}

@compute @workgroup_size(16, 16, 1)
fn cs_brush(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dim = textureDimensions(src_tex);
    if (gid.x >= dim.x || gid.y >= dim.y) {
        return;
    }
    let coord = vec2<i32>(i32(gid.x), i32(gid.y));
    let in_px = textureLoad(src_tex, coord, 0);

    let uv = (vec2<f32>(f32(coord.x) + 0.5, f32(coord.y) + 0.5)) / vec2<f32>(f32(dim.x), f32(dim.y));
    let amt = sample_brush_bilinear(uv);

    if (abs(amt) < 1e-5) {
        textureStore(dst_tex, coord, in_px);
        return;
    }

    var mod_rgb = in_px.rgb;

    if (u.channel == 0u) {
        // Dodge & Burn: smooth EV curve preserving HDR
        let factor = exp2(amt * 1.5);
        mod_rgb = in_px.rgb * factor;
    } else if (u.channel == 1u) {
        // Exposure EV
        let factor = exp2(amt * 1.5);
        mod_rgb = in_px.rgb * factor;
    } else if (u.channel == 2u) {
        // Temperature (white balance)
        mod_rgb = apply_white_balance_rec2020(in_px.rgb, amt * 65.0, 0.0);
    } else if (u.channel == 3u) {
        // Saturation in Oklab space
        var lab = linear_rec2020_to_oklab(in_px.rgb);
        let sat_scale = max(0.0, 1.0 + amt * 1.0);
        lab.y *= sat_scale;
        lab.z *= sat_scale;
        mod_rgb = oklab_to_linear_rec2020(lab);
    }

    let out_rgb = mix(in_px.rgb, mod_rgb, u.opacity);
    textureStore(dst_tex, coord, vec4<f32>(out_rgb, in_px.a));
}
"#;

// ============================================================================
// 10. Mask Blend
// ============================================================================

/// Uniforms for Mask Blend compute shader (16 bytes).
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct MaskBlendUniforms {
    pub opacity: f32,
    pub has_mask: u32,
    pub _pad0: u32,
    pub _pad1: u32,
}

impl MaskBlendUniforms {
    pub fn new(opacity: f32, has_mask: bool) -> Self {
        Self {
            opacity,
            has_mask: if has_mask { 1u32 } else { 0u32 },
            _pad0: 0,
            _pad1: 0,
        }
    }

    pub fn from_params(opacity: f32, has_mask: bool) -> Self {
        Self::new(opacity, has_mask)
    }
}

pub const MASK_BLEND_WGSL: &str = r#"
struct MaskBlendUniforms {
    opacity: f32,
    has_mask: u32,
    _pad0: u32,
    _pad1: u32,
};

@group(0) @binding(0) var pre_tex: texture_2d<f32>;
@group(0) @binding(1) var post_tex: texture_2d<f32>;
@group(0) @binding(2) var dst_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(3) var mask_tex: texture_2d<f32>;
@group(0) @binding(4) var mask_sampler: sampler;
@group(0) @binding(5) var<uniform> u: MaskBlendUniforms;

@compute @workgroup_size(16, 16, 1)
fn cs_mask_blend(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dim = textureDimensions(pre_tex);
    if (gid.x >= dim.x || gid.y >= dim.y) {
        return;
    }
    let coord = vec2<i32>(i32(gid.x), i32(gid.y));
    let pre_px = textureLoad(pre_tex, coord, 0);
    let post_px = textureLoad(post_tex, coord, 0);

    let uv = (vec2<f32>(gid.xy) + vec2<f32>(0.5)) / vec2<f32>(dim);
    let m = select(1.0, textureSampleLevel(mask_tex, mask_sampler, uv, 0.0).r, u.has_mask != 0u);

    let blend_w = clamp(u.opacity * m, 0.0, 1.0);
    let out_rgb = mix(pre_px.rgb, post_px.rgb, blend_w);

    textureStore(dst_tex, coord, vec4<f32>(out_rgb, 1.0));
}
"#;

// ============================================================================
// Compile-time Uniform Struct Layout & 16-byte Alignment Verification
// ============================================================================

const _: () = {
    // 1. Color Grade
    assert!(std::mem::size_of::<GpuColorWheel>() == 16);
    assert!(std::mem::size_of::<GpuColorWheel>() % 16 == 0);
    assert!(std::mem::size_of::<ColorGradeUniforms>() == 80);
    assert!(std::mem::size_of::<ColorGradeUniforms>() % 16 == 0);

    // 2. Dehaze
    assert!(std::mem::size_of::<DehazeUniforms>() == 16);
    assert!(std::mem::size_of::<DehazeUniforms>() % 16 == 0);

    // 3. Tonal Contrast
    assert!(std::mem::size_of::<TonalContrastUniforms>() == 32);
    assert!(std::mem::size_of::<TonalContrastUniforms>() % 16 == 0);

    // 4. Halation
    assert!(std::mem::size_of::<HalationUniforms>() == 32);
    assert!(std::mem::size_of::<HalationUniforms>() % 16 == 0);

    // 5. Bloom
    assert!(std::mem::size_of::<BloomUniforms>() == 32);
    assert!(std::mem::size_of::<BloomUniforms>() % 16 == 0);

    // 6. Grainy Film
    assert!(std::mem::size_of::<GrainyFilmUniforms>() == 16);
    assert!(std::mem::size_of::<GrainyFilmUniforms>() % 16 == 0);

    // 7. Noir
    assert!(std::mem::size_of::<NoirUniforms>() == 32);
    assert!(std::mem::size_of::<NoirUniforms>() % 16 == 0);

    // 8. Selective (Nik U-Point)
    assert!(std::mem::size_of::<GpuSelectivePin>() == 32);
    assert!(std::mem::size_of::<GpuSelectivePin>() % 16 == 0);
    assert!(std::mem::size_of::<SelectiveUniforms>() == 528);
    assert!(std::mem::size_of::<SelectiveUniforms>() % 16 == 0);

    // 9. Brush
    assert!(std::mem::size_of::<BrushUniforms>() == 16);
    assert!(std::mem::size_of::<BrushUniforms>() % 16 == 0);

    // 10. Mask Blend
    assert!(std::mem::size_of::<MaskBlendUniforms>() == 16);
    assert!(std::mem::size_of::<MaskBlendUniforms>() % 16 == 0);
};
