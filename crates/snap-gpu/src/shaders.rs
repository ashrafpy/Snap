pub const COMMON_WGSL: &str = r#"
const SRGB_TO_REC2020: mat3x3<f32> = mat3x3<f32>(
    vec3<f32>(0.627404078626, 0.069097233161, 0.016391438883),
    vec3<f32>(0.329282098415, 0.919541035593, 0.088013307577),
    vec3<f32>(0.043313797307, 0.011361186736, 0.895595253540)
);

const REC2020_TO_SRGB: mat3x3<f32> = mat3x3<f32>(
    vec3<f32>(1.660490995932, -0.124550474515, -0.018150763435),
    vec3<f32>(-0.587641138833, 1.132899897135, -0.100578897042),
    vec3<f32>(-0.072849857099, -0.008349422620, 1.118729660477)
);

fn rec2020_luma(rgb: vec3<f32>) -> f32 {
    return 0.2627 * rgb.r + 0.6780 * rgb.g + 0.0593 * rgb.b;
}

fn linear_to_srgb_ch(c: f32) -> f32 {
    if (c <= 0.0031308) {
        return c * 12.92;
    } else {
        return 1.055 * pow(c, 1.0 / 2.4) - 0.055;
    }
}

fn linear_to_srgb(rgb: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(
        linear_to_srgb_ch(rgb.r),
        linear_to_srgb_ch(rgb.g),
        linear_to_srgb_ch(rgb.b)
    );
}

fn srgb_to_linear_ch(c: f32) -> f32 {
    if (c <= 0.04045) {
        return c / 12.92;
    } else {
        return pow((c + 0.055) / 1.055, 2.4);
    }
}

fn srgb_to_linear(rgb: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(
        srgb_to_linear_ch(rgb.r),
        srgb_to_linear_ch(rgb.g),
        srgb_to_linear_ch(rgb.b)
    );
}

fn signed_cbrt(x: f32) -> f32 {
    if (x < 0.0) {
        return -pow(-x, 1.0 / 3.0);
    } else {
        return pow(x, 1.0 / 3.0);
    }
}

fn linear_srgb_to_oklab(rgb: vec3<f32>) -> vec3<f32> {
    let l = 0.4122214708 * rgb.r + 0.5363325363 * rgb.g + 0.0514459929 * rgb.b;
    let m = 0.2119034982 * rgb.r + 0.6806995451 * rgb.g + 0.1073969566 * rgb.b;
    let s = 0.0883024619 * rgb.r + 0.2817188376 * rgb.g + 0.6299787005 * rgb.b;

    let l_ = signed_cbrt(l);
    let m_ = signed_cbrt(m);
    let s_ = signed_cbrt(s);

    let l_out = 0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_;
    let a_out = 1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_;
    let b_out = 0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_;

    return vec3<f32>(l_out, a_out, b_out);
}

fn oklab_to_linear_srgb(lab: vec3<f32>) -> vec3<f32> {
    let l_ = lab.x + 0.3963377774 * lab.y + 0.2158037573 * lab.z;
    let m_ = lab.x - 0.1055613458 * lab.y - 0.0638541728 * lab.z;
    let s_ = lab.x - 0.0894841775 * lab.y - 1.2914855480 * lab.z;

    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;

    let r = 4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s;
    let g = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s;
    let b = -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s;

    return vec3<f32>(r, g, b);
}

fn linear_rec2020_to_oklab(rgb: vec3<f32>) -> vec3<f32> {
    let srgb = REC2020_TO_SRGB * rgb;
    return linear_srgb_to_oklab(srgb);
}

fn oklab_to_linear_rec2020(lab: vec3<f32>) -> vec3<f32> {
    let srgb = oklab_to_linear_srgb(lab);
    return SRGB_TO_REC2020 * srgb;
}

fn apply_white_balance_rec2020(rgb: vec3<f32>, temp: f32, tint: f32) -> vec3<f32> {
    if (abs(temp) < 1e-4 && abs(tint) < 1e-4) {
        return rgb;
    }
    let dt = temp / 100.0;
    let dT = tint / 100.0;
    let w_r = 0.2627;
    let w_g = 0.6780;
    let w_b = 0.0593;

    let r_est = 1.0 - (w_g * (0.5 * dt + dT) + w_b * dt);
    let g_est = r_est + 0.5 * dt + dT;
    let b_est = r_est + dt;

    let gain_r = 1.0 / max(r_est, 1e-4);
    let gain_g = 1.0 / max(g_est, 1e-4);
    let gain_b = 1.0 / max(b_est, 1e-4);

    return vec3<f32>(rgb.r * gain_r, rgb.g * gain_g, rgb.b * gain_b);
}
"#;

pub const RAW_DEVELOP_WGSL: &str = r#"
struct RawDevelopUniforms {
    exposure_ev: f32,
    highlights: f32,
    shadows: f32,
    contrast: f32,
    temperature: f32,
    tint: f32,
    saturation: f32,
    opacity: f32,
};

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var dst_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var<uniform> u: RawDevelopUniforms;

@compute @workgroup_size(16, 16, 1)
fn cs_raw_develop(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dim = textureDimensions(src_tex);
    if (gid.x >= dim.x || gid.y >= dim.y) {
        return;
    }
    let coord = vec2<i32>(i32(gid.x), i32(gid.y));
    let in_px = textureLoad(src_tex, coord, 0);
    var rgb = in_px.rgb;

    // 1. White balance chromatic adaptation
    rgb = apply_white_balance_rec2020(rgb, u.temperature, u.tint);

    // 2. Exposure EV
    rgb = rgb * exp2(u.exposure_ev);

    // 3. Highlights and shadows smooth recovery
    let luma = rec2020_luma(rgb);
    var luma_scale = 1.0;
    if (u.shadows != 0.0) {
        let s_mask = 1.0 - smoothstep(0.0, 0.35, luma);
        luma_scale += (u.shadows / 100.0) * s_mask * 0.75;
    }
    if (u.highlights != 0.0) {
        let h_mask = smoothstep(0.3, 1.2, luma);
        luma_scale += (u.highlights / 100.0) * h_mask * (luma / (1.0 + luma));
    }
    rgb = rgb * max(luma_scale, 0.0);

    // 4. Contrast around mid-gray 0.18
    if (u.contrast != 0.0) {
        let cur_l = rec2020_luma(rgb);
        if (cur_l > 1e-6) {
            let log_l = log2(cur_l / 0.18);
            let new_log_l = log_l * (1.0 + (u.contrast / 100.0) * 0.4);
            let new_l = 0.18 * exp2(new_log_l);
            rgb = rgb * (new_l / cur_l);
        }
    }

    // 5. Saturation in Oklab
    if (u.saturation != 0.0) {
        let lab = linear_rec2020_to_oklab(rgb);
        let sat_scale = max(0.0, 1.0 + u.saturation / 100.0);
        rgb = oklab_to_linear_rec2020(vec3<f32>(lab.x, lab.y * sat_scale, lab.z * sat_scale));
    }

    let out_rgb = mix(in_px.rgb, rgb, u.opacity);
    textureStore(dst_tex, coord, vec4<f32>(out_rgb, in_px.a));
}
"#;

pub const WHITE_BALANCE_WGSL: &str = r#"
struct WhiteBalanceUniforms {
    temperature: f32,
    tint: f32,
    opacity: f32,
    _pad: f32,
};

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var dst_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var<uniform> u: WhiteBalanceUniforms;

@compute @workgroup_size(16, 16, 1)
fn cs_white_balance(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dim = textureDimensions(src_tex);
    if (gid.x >= dim.x || gid.y >= dim.y) {
        return;
    }
    let coord = vec2<i32>(i32(gid.x), i32(gid.y));
    let in_px = textureLoad(src_tex, coord, 0);

    let rgb = apply_white_balance_rec2020(in_px.rgb, u.temperature, u.tint);
    let out_rgb = mix(in_px.rgb, rgb, u.opacity);

    textureStore(dst_tex, coord, vec4<f32>(out_rgb, in_px.a));
}
"#;

pub const TUNE_IMAGE_WGSL: &str = r#"
struct TuneImageUniforms {
    brightness: f32,
    contrast: f32,
    saturation: f32,
    ambiance: f32,
    highlights: f32,
    shadows: f32,
    warmth: f32,
    opacity: f32,
};

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var dst_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var<uniform> u: TuneImageUniforms;

@compute @workgroup_size(16, 16, 1)
fn cs_tune_image(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dim = textureDimensions(src_tex);
    if (gid.x >= dim.x || gid.y >= dim.y) {
        return;
    }
    let coord = vec2<i32>(i32(gid.x), i32(gid.y));
    let in_px = textureLoad(src_tex, coord, 0);
    var rgb = in_px.rgb;

    // 1. Brightness: smooth exposure lift
    if (u.brightness != 0.0) {
        rgb = rgb * exp2(u.brightness / 100.0 * 1.5);
    }

    // 2. Adjustments in Oklab space
    var lab = linear_rec2020_to_oklab(rgb);
    var L = lab.x;

    // Contrast
    if (u.contrast != 0.0) {
        let c = u.contrast / 100.0;
        L = 0.5 + (L - 0.5) * (1.0 + c * 0.5);
    }

    // Highlights & Shadows
    if (u.shadows != 0.0) {
        L += (u.shadows / 100.0) * 0.25 * (1.0 - smoothstep(0.0, 0.5, L));
    }
    if (u.highlights != 0.0) {
        L += (u.highlights / 100.0) * 0.25 * smoothstep(0.4, 1.0, L);
    }

    // Ambiance (Snapseed fill/balance)
    if (u.ambiance != 0.0) {
        let amb = u.ambiance / 100.0;
        L += amb * 0.15 * sin(3.14159265 * clamp(L, 0.0, 1.0));
        let chroma_mult = 1.0 + amb * 0.1;
        lab.y *= chroma_mult;
        lab.z *= chroma_mult;
    }

    // Warmth (weighted by luminance so pure black/white does not tint)
    if (u.warmth != 0.0) {
        let w = u.warmth / 100.0;
        let w_weight = smoothstep(0.02, 0.2, L) * (1.0 - smoothstep(0.8, 0.98, L));
        lab.y += w * 0.03 * w_weight;
        lab.z += w * 0.10 * w_weight;
    }

    // Saturation
    if (u.saturation != 0.0) {
        let sat = max(0.0, 1.0 + u.saturation / 100.0);
        lab.y *= sat;
        lab.z *= sat;
    }

    lab.x = clamp(L, 0.0, 2.0);
    rgb = oklab_to_linear_rec2020(lab);

    let out_rgb = mix(in_px.rgb, rgb, u.opacity);
    textureStore(dst_tex, coord, vec4<f32>(out_rgb, in_px.a));
}
"#;

pub const COLOR_HSL_WGSL: &str = r#"
struct ColorHslBandGpu {
    hue: f32,
    saturation: f32,
    luminance: f32,
    _pad: f32,
};

struct ColorHslUniforms {
    bands: array<ColorHslBandGpu, 8>,
    opacity: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
};

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var dst_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var<uniform> u: ColorHslUniforms;

fn rgb_to_hsl(c: vec3<f32>) -> vec3<f32> {
    let c_max = max(c.r, max(c.g, c.b));
    let c_min = min(c.r, min(c.g, c.b));
    let delta = c_max - c_min;
    var h: f32 = 0.0;
    var s: f32 = 0.0;
    let l = (c_max + c_min) * 0.5;
    if (delta > 1e-5) {
        if (l < 0.5) {
            s = delta / (c_max + c_min);
        } else {
            s = delta / (2.0 - c_max - c_min);
        }
        if (c.r >= c_max) {
            h = (c.g - c.b) / delta;
            if (h < 0.0) { h += 6.0; }
        } else if (c.g >= c_max) {
            h = 2.0 + (c.b - c.r) / delta;
        } else {
            h = 4.0 + (c.r - c.g) / delta;
        }
        h *= 60.0;
    }
    return vec3<f32>(h, s, l);
}

fn hue_to_rgb(p: f32, q: f32, t_in: f32) -> f32 {
    var t = t_in;
    if (t < 0.0) { t += 1.0; }
    if (t > 1.0) { t -= 1.0; }
    if (t < 1.0 / 6.0) { return p + (q - p) * 6.0 * t; }
    if (t < 1.0 / 2.0) { return q; }
    if (t < 2.0 / 3.0) { return p + (q - p) * (2.0 / 3.0 - t) * 6.0; }
    return p;
}

fn hsl_to_rgb(hsl: vec3<f32>) -> vec3<f32> {
    let h = hsl.x / 360.0;
    let s = hsl.y;
    let l = hsl.z;
    if (s <= 1e-5) {
        return vec3<f32>(l, l, l);
    }
    let q = select(l * (1.0 + s), l + s - l * s, l >= 0.5);
    let p = 2.0 * l - q;
    let r = hue_to_rgb(p, q, h + 1.0 / 3.0);
    let g = hue_to_rgb(p, q, h);
    let b = hue_to_rgb(p, q, h - 1.0 / 3.0);
    return vec3<f32>(r, g, b);
}

fn band_weight(hue: f32, center: f32, width: f32) -> f32 {
    var d = abs(hue - center);
    if (d > 180.0) {
        d = 360.0 - d;
    }
    if (d >= width) {
        return 0.0;
    }
    return 0.5 * (1.0 + cos(3.14159265 * d / width));
}

@compute @workgroup_size(16, 16, 1)
fn cs_color_hsl(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dim = textureDimensions(src_tex);
    if (gid.x >= dim.x || gid.y >= dim.y) {
        return;
    }
    let coord = vec2<i32>(i32(gid.x), i32(gid.y));
    let in_px = textureLoad(src_tex, coord, 0);

    let srgb = max(REC2020_TO_SRGB * in_px.rgb, vec3<f32>(0.0));
    let srgb_gamma = linear_to_srgb(srgb);
    var hsl = rgb_to_hsl(srgb_gamma);

    let centers = array<f32, 8>(0.0, 30.0, 60.0, 120.0, 180.0, 240.0, 280.0, 320.0);
    let widths  = array<f32, 8>(30.0, 25.0, 35.0, 45.0, 45.0, 35.0, 30.0, 30.0);

    var d_h = 0.0;
    var d_s = 0.0;
    var d_l = 0.0;

    for (var i = 0u; i < 8u; i = i + 1u) {
        let w = band_weight(hsl.x, centers[i], widths[i]);
        if (w > 0.0) {
            d_h += w * (u.bands[i].hue / 100.0);
            d_s += w * (u.bands[i].saturation / 100.0);
            d_l += w * (u.bands[i].luminance / 100.0);
        }
    }

    let chroma_weight = smoothstep(0.01, 0.08, hsl.y);
    d_h *= chroma_weight;
    d_s *= chroma_weight;
    d_l *= chroma_weight;

    hsl.x = (hsl.x + d_h * 30.0 + 360.0) % 360.0;
    hsl.y = clamp(hsl.y * (1.0 + d_s), 0.0, 1.0);
    hsl.z = clamp(hsl.z * (1.0 + d_l * 0.3), 0.0, 1.0);

    let out_gamma = hsl_to_rgb(hsl);
    let out_lin_srgb = srgb_to_linear(out_gamma);
    let out_rec2020 = SRGB_TO_REC2020 * out_lin_srgb;

    let final_rgb = mix(in_px.rgb, out_rec2020, u.opacity);
    textureStore(dst_tex, coord, vec4<f32>(final_rgb, in_px.a));
}
"#;

pub const CURVES_WGSL: &str = r#"
struct CurvesUniforms {
    opacity: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
};

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var dst_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var curve_tex: texture_2d<f32>;
@group(0) @binding(3) var curve_sampler: sampler;
@group(0) @binding(4) var<uniform> u: CurvesUniforms;

fn sample_curve(x: f32) -> vec4<f32> {
    let u_coord = clamp((x * 1023.0 + 0.5) / 1024.0, 0.5 / 1024.0, 1023.5 / 1024.0);
    return textureSampleLevel(curve_tex, curve_sampler, vec2<f32>(u_coord, 0.5), 0.0);
}

@compute @workgroup_size(16, 16, 1)
fn cs_curves(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dim = textureDimensions(src_tex);
    if (gid.x >= dim.x || gid.y >= dim.y) {
        return;
    }
    let coord = vec2<i32>(i32(gid.x), i32(gid.y));
    let in_px = textureLoad(src_tex, coord, 0);

    let srgb = max(REC2020_TO_SRGB * in_px.rgb, vec3<f32>(0.0));
    let gamma_rgb = linear_to_srgb(srgb);

    let r_curved = sample_curve(gamma_rgb.r).r;
    let g_curved = sample_curve(gamma_rgb.g).g;
    let b_curved = sample_curve(gamma_rgb.b).b;

    let y_val = 0.2126 * gamma_rgb.r + 0.7152 * gamma_rgb.g + 0.0722 * gamma_rgb.b;
    let y_curved = sample_curve(y_val).a;

    var ratio = 1.0;
    if (y_val > 1e-5) {
        ratio = y_curved / y_val;
    }

    let final_gamma = vec3<f32>(r_curved, g_curved, b_curved) * ratio;
    let final_srgb = srgb_to_linear(final_gamma);
    let final_rec2020 = SRGB_TO_REC2020 * final_srgb;

    let out_rgb = mix(in_px.rgb, final_rec2020, u.opacity);
    textureStore(dst_tex, coord, vec4<f32>(out_rgb, in_px.a));
}
"#;

pub const LUT_3D_WGSL: &str = r#"
struct Lut3dUniforms {
    domain_min: vec3<f32>,
    lut_size: f32,
    domain_max: vec3<f32>,
    strength: f32,
    input_space: u32,
    opacity: f32,
    _pad0: f32,
    _pad1: f32,
};

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var dst_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var lut_tex_3d: texture_3d<f32>;
@group(0) @binding(3) var lut_sampler: sampler;
@group(0) @binding(4) var<uniform> u: Lut3dUniforms;

fn rec709_oetf(c: f32) -> f32 {
    if (c < 0.018) {
        return c * 4.5;
    } else {
        return 1.099 * pow(c, 0.45) - 0.099;
    }
}

fn to_apple_log(x: f32) -> f32 {
    if (x >= 0.000049) {
        return 0.529136 * log2(x + 0.037584) + 0.544154;
    } else {
        return 0.0;
    }
}

fn to_sony_slog3(x: f32) -> f32 {
    if (x >= 0.0) {
        return (420.0 + (log(max((x + 0.01) / 0.19, 1e-5)) / 2.302585) * 261.5) / 1023.0;
    } else {
        return 0.0;
    }
}

fn to_canon_log2(x: f32) -> f32 {
    if (x >= 0.0) {
        return (0.281863077 + (log(max((x + 0.0352) / 0.2152, 1e-5)) / 2.302585) * 0.165969) / 0.828362;
    } else {
        return 0.0;
    }
}

@compute @workgroup_size(16, 16, 1)
fn cs_lut_3d(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dim = textureDimensions(src_tex);
    if (gid.x >= dim.x || gid.y >= dim.y) {
        return;
    }
    let coord = vec2<i32>(i32(gid.x), i32(gid.y));
    let in_px = textureLoad(src_tex, coord, 0);
    let orig_rec2020 = in_px.rgb;

    var c = orig_rec2020;
    if (u.input_space == 0u) { // Srgb
        let srgb_lin = max(REC2020_TO_SRGB * orig_rec2020, vec3<f32>(0.0));
        c = linear_to_srgb(srgb_lin);
    } else if (u.input_space == 1u) { // Rec709
        let srgb_lin = max(REC2020_TO_SRGB * orig_rec2020, vec3<f32>(0.0));
        c = vec3<f32>(rec709_oetf(srgb_lin.r), rec709_oetf(srgb_lin.g), rec709_oetf(srgb_lin.b));
    } else if (u.input_space == 2u) { // LinearRec2020
        c = orig_rec2020;
    } else if (u.input_space == 3u) { // AppleLog
        let srgb_lin = max(REC2020_TO_SRGB * orig_rec2020, vec3<f32>(0.0));
        c = vec3<f32>(to_apple_log(srgb_lin.r), to_apple_log(srgb_lin.g), to_apple_log(srgb_lin.b));
    } else if (u.input_space == 4u) { // SonySLog3
        let srgb_lin = max(REC2020_TO_SRGB * orig_rec2020, vec3<f32>(0.0));
        c = vec3<f32>(to_sony_slog3(srgb_lin.r), to_sony_slog3(srgb_lin.g), to_sony_slog3(srgb_lin.b));
    } else if (u.input_space == 5u) { // CanonLog2
        let srgb_lin = max(REC2020_TO_SRGB * orig_rec2020, vec3<f32>(0.0));
        c = vec3<f32>(to_canon_log2(srgb_lin.r), to_canon_log2(srgb_lin.g), to_canon_log2(srgb_lin.b));
    }

    let t = clamp((c - u.domain_min) / max(u.domain_max - u.domain_min, vec3<f32>(1e-5)), vec3<f32>(0.0), vec3<f32>(1.0));
    let uvw = (t * (u.lut_size - 1.0) + 0.5) / u.lut_size;
    let lut_sample = textureSampleLevel(lut_tex_3d, lut_sampler, uvw, 0.0).rgb;

    var out_rec2020 = orig_rec2020;
    if (u.input_space == 2u) {
        out_rec2020 = lut_sample;
    } else {
        let out_lin_srgb = srgb_to_linear(lut_sample);
        out_rec2020 = SRGB_TO_REC2020 * out_lin_srgb;
    }

    let lut_rgb = mix(orig_rec2020, out_rec2020, u.strength / 100.0);
    let final_rgb = mix(orig_rec2020, lut_rgb, u.opacity);

    textureStore(dst_tex, coord, vec4<f32>(final_rgb, in_px.a));
}
"#;

pub const DETAILS_WGSL: &str = r#"
struct DetailsUniforms {
    structure: f32,
    sharpening: f32,
    opacity: f32,
    pass_idx: u32,
    width: f32,
    height: f32,
    _pad0: f32,
    _pad1: f32,
};

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var dst_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var<uniform> u: DetailsUniforms;

@compute @workgroup_size(16, 16, 1)
fn cs_details(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dim = textureDimensions(src_tex);
    if (gid.x >= dim.x || gid.y >= dim.y) {
        return;
    }
    let coord = vec2<i32>(i32(gid.x), i32(gid.y));
    let center_px = textureLoad(src_tex, coord, 0);
    let center_luma = rec2020_luma(center_px.rgb);

    let sigma_s = 2.0;
    let sigma_r = 0.12;
    let two_sigma_s2 = 2.0 * sigma_s * sigma_s;
    let two_sigma_r2 = 2.0 * sigma_r * sigma_r;

    if (u.pass_idx == 0u) {
        // Horizontal bilateral pass on luminance
        var sum_luma = 0.0;
        var sum_w = 0.0;

        for (var k = -4; k <= 4; k = k + 1) {
            let tap_x = clamp(coord.x + k, 0, i32(dim.x) - 1);
            let tap_px = textureLoad(src_tex, vec2<i32>(tap_x, coord.y), 0);
            let tap_luma = rec2020_luma(tap_px.rgb);

            let d_spatial = f32(k);
            let d_range = tap_luma - center_luma;
            let w = exp(- (d_spatial * d_spatial) / two_sigma_s2 - (d_range * d_range) / two_sigma_r2);

            sum_luma += tap_luma * w;
            sum_w += w;
        }

        let luma_h = sum_luma / max(sum_w, 1e-5);
        textureStore(dst_tex, coord, vec4<f32>(center_px.rgb, luma_h));
    } else {
        // Vertical bilateral pass on filtered luminance
        var sum_luma = 0.0;
        var sum_w = 0.0;

        for (var k = -4; k <= 4; k = k + 1) {
            let tap_y = clamp(coord.y + k, 0, i32(dim.y) - 1);
            let tap_px = textureLoad(src_tex, vec2<i32>(coord.x, tap_y), 0);
            let tap_luma = tap_px.a;

            let d_spatial = f32(k);
            let d_range = tap_luma - center_px.a;
            let w = exp(- (d_spatial * d_spatial) / two_sigma_s2 - (d_range * d_range) / two_sigma_r2);

            sum_luma += tap_luma * w;
            sum_w += w;
        }

        let luma_base = sum_luma / max(sum_w, 1e-5);
        let detail = center_luma - luma_base;
        let luma_struct = center_luma + detail * (u.structure / 100.0);

        // 5-tap unsharp mask acutance
        let px_l = textureLoad(src_tex, vec2<i32>(max(0, coord.x - 1), coord.y), 0);
        let px_r = textureLoad(src_tex, vec2<i32>(min(i32(dim.x) - 1, coord.x + 1), coord.y), 0);
        let px_u = textureLoad(src_tex, vec2<i32>(coord.x, max(0, coord.y - 1)), 0);
        let px_d = textureLoad(src_tex, vec2<i32>(coord.x, min(i32(dim.y) - 1, coord.y + 1)), 0);

        let l_l = rec2020_luma(px_l.rgb);
        let l_r = rec2020_luma(px_r.rgb);
        let l_u = rec2020_luma(px_u.rgb);
        let l_d = rec2020_luma(px_d.rgb);

        var lap = 4.0 * center_luma - (l_l + l_r + l_u + l_d);
        if (abs(lap) < 0.005) {
            lap = 0.0;
        }

        let luma_final = luma_struct + lap * (u.sharpening / 100.0) * 0.4;
        let ratio = max(0.0, luma_final) / max(center_luma, 1e-5);
        let out_rgb = mix(center_px.rgb, center_px.rgb * ratio, u.opacity);

        textureStore(dst_tex, coord, vec4<f32>(out_rgb, 1.0));
    }
}
"#;

pub const BLACK_AND_WHITE_WGSL: &str = r#"
struct BlackAndWhiteUniforms {
    channel_weights: vec3<f32>,
    brightness: f32,
    contrast: f32,
    grain: f32,
    opacity: f32,
    _pad: f32,
};

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var dst_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var<uniform> u: BlackAndWhiteUniforms;

@compute @workgroup_size(16, 16, 1)
fn cs_black_and_white(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dim = textureDimensions(src_tex);
    if (gid.x >= dim.x || gid.y >= dim.y) {
        return;
    }
    let coord = vec2<i32>(i32(gid.x), i32(gid.y));
    let in_px = textureLoad(src_tex, coord, 0);

    let srgb = max(REC2020_TO_SRGB * in_px.rgb, vec3<f32>(0.0));
    var bw = dot(srgb, u.channel_weights);

    // Brightness
    if (u.brightness != 0.0) {
        bw = bw * exp2(u.brightness / 100.0);
    }

    // Contrast
    if (u.contrast != 0.0) {
        let c = u.contrast / 100.0;
        bw = clamp(0.5 + (bw - 0.5) * (1.0 + c * 0.6), 0.0, 1.0);
    }

    // Midtone-weighted organic film grain
    if (u.grain > 0.0) {
        let n = (fract(sin(dot(vec2<f32>(f32(coord.x), f32(coord.y)), vec2<f32>(12.9898, 78.233))) * 43758.5453) - 0.5) * 2.0;
        let midtone_mask = 4.0 * bw * (1.0 - bw);
        bw = clamp(bw + n * (u.grain / 100.0) * 0.08 * midtone_mask, 0.0, 1.0);
    }

    let achromatic_rec2020 = vec3<f32>(bw, bw, bw);
    let out_rgb = mix(in_px.rgb, achromatic_rec2020, u.opacity);

    textureStore(dst_tex, coord, vec4<f32>(out_rgb, in_px.a));
}
"#;

pub const VIGNETTE_WGSL: &str = r#"
struct VignetteUniforms {
    center: vec2<f32>,
    inner_radius: f32,
    outer_radius: f32,
    outer_brightness: f32,
    inner_brightness: f32,
    aspect: f32,
    opacity: f32,
};

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var dst_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var<uniform> u: VignetteUniforms;

@compute @workgroup_size(16, 16, 1)
fn cs_vignette(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dim = textureDimensions(src_tex);
    if (gid.x >= dim.x || gid.y >= dim.y) {
        return;
    }
    let coord = vec2<i32>(i32(gid.x), i32(gid.y));
    let in_px = textureLoad(src_tex, coord, 0);

    let uv = (vec2<f32>(f32(coord.x) + 0.5, f32(coord.y) + 0.5)) / vec2<f32>(f32(dim.x), f32(dim.y));
    var delta = uv - u.center;
    if (u.aspect >= 1.0) {
        delta.x *= u.aspect;
    } else {
        delta.y /= u.aspect;
    }

    let d = length(delta) / 0.70710678;
    let t = smoothstep(u.inner_radius, u.outer_radius, d);
    let b = mix(u.inner_brightness / 100.0, u.outer_brightness / 100.0, t);
    let factor = exp2(b * 1.5);

    let out_rgb = mix(in_px.rgb, in_px.rgb * factor, u.opacity);
    textureStore(dst_tex, coord, vec4<f32>(out_rgb, in_px.a));
}
"#;

pub const HISTOGRAM_WGSL: &str = r#"
@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var<storage, read_write> hist_buf: array<atomic<u32>, 1024>;

var<workgroup> wg_r: array<atomic<u32>, 256>;
var<workgroup> wg_g: array<atomic<u32>, 256>;
var<workgroup> wg_b: array<atomic<u32>, 256>;
var<workgroup> wg_l: array<atomic<u32>, 256>;

@compute @workgroup_size(16, 16, 1)
fn cs_histogram(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(local_invocation_index) lid: u32
) {
    if (lid < 256u) {
        atomicStore(&wg_r[lid], 0u);
        atomicStore(&wg_g[lid], 0u);
        atomicStore(&wg_b[lid], 0u);
        atomicStore(&wg_l[lid], 0u);
    }
    workgroupBarrier();

    let dim = textureDimensions(src_tex);
    if (gid.x < dim.x && gid.y < dim.y) {
        let px = textureLoad(src_tex, vec2<i32>(i32(gid.x), i32(gid.y)), 0).rgb;
        let r = max(px.r, 0.0);
        let g = max(px.g, 0.0);
        let b = max(px.b, 0.0);
        let l = rec2020_luma(vec3<f32>(r, g, b));

        let r_s = linear_to_srgb_ch(r);
        let g_s = linear_to_srgb_ch(g);
        let b_s = linear_to_srgb_ch(b);
        let l_s = linear_to_srgb_ch(l);

        let bin_r = u32(clamp(round(r_s * 255.0), 0.0, 255.0));
        let bin_g = u32(clamp(round(g_s * 255.0), 0.0, 255.0));
        let bin_b = u32(clamp(round(b_s * 255.0), 0.0, 255.0));
        let bin_l = u32(clamp(round(l_s * 255.0), 0.0, 255.0));

        atomicAdd(&wg_r[bin_r], 1u);
        atomicAdd(&wg_g[bin_g], 1u);
        atomicAdd(&wg_b[bin_b], 1u);
        atomicAdd(&wg_l[bin_l], 1u);
    }

    workgroupBarrier();

    if (lid < 256u) {
        let vr = atomicLoad(&wg_r[lid]);
        if (vr > 0u) { atomicAdd(&hist_buf[lid], vr); }
        let vg = atomicLoad(&wg_g[lid]);
        if (vg > 0u) { atomicAdd(&hist_buf[256u + lid], vg); }
        let vb = atomicLoad(&wg_b[lid]);
        if (vb > 0u) { atomicAdd(&hist_buf[512u + lid], vb); }
        let vl = atomicLoad(&wg_l[lid]);
        if (vl > 0u) { atomicAdd(&hist_buf[768u + lid], vl); }
    }
}
"#;

pub const CROP_TRANSFORM_WGSL: &str = r#"
struct CropTransformUniforms {
    crop_rect: vec4<f32>, // x, y, width, height
    src_size: vec2<f32>,
    dst_size: vec2<f32>,
    straighten_rad: f32,
    inscribed_scale: f32,
    quarter_turns: u32,
    flip_flags: u32,
};

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var dst_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var smp: sampler;
@group(0) @binding(3) var<uniform> u: CropTransformUniforms;

@compute @workgroup_size(16, 16, 1)
fn cs_crop_transform(@builtin(global_invocation_id) gid: vec3<u32>) {
    if (f32(gid.x) >= u.dst_size.x || f32(gid.y) >= u.dst_size.y) {
        return;
    }

    // 1. Map output texel to crop sub-rect normalized [0, 1]
    let u_crop = (f32(gid.x) + 0.5) / u.dst_size.x;
    let v_crop = (f32(gid.y) + 0.5) / u.dst_size.y;

    var u1 = u.crop_rect.x + u_crop * u.crop_rect.z;
    var v1 = u.crop_rect.y + v_crop * u.crop_rect.w;

    // 2. Undo flip
    if ((u.flip_flags & 1u) != 0u) {
        u1 = 1.0 - u1;
    }
    if ((u.flip_flags & 2u) != 0u) {
        v1 = 1.0 - v1;
    }

    // 3. Undo quarter turns
    var u2 = u1 - 0.5;
    var v2 = v1 - 0.5;
    let turns = u.quarter_turns % 4u;
    if (turns == 1u) {
        let temp = u2;
        u2 = v2;
        v2 = -temp;
    } else if (turns == 2u) {
        u2 = -u2;
        v2 = -v2;
    } else if (turns == 3u) {
        let temp = u2;
        u2 = -v2;
        v2 = temp;
    }

    // 4. Undo straighten
    let s = u.inscribed_scale;
    let dx = u2 * s;
    let dy = v2 * s;

    let aspect = u.src_size.x / u.src_size.y;
    let dx_phys = dx * aspect;
    let dy_phys = dy;

    let cos_a = cos(u.straighten_rad);
    let sin_a = sin(u.straighten_rad);
    let rot_x = dx_phys * cos_a + dy_phys * sin_a;
    let rot_y = -dx_phys * sin_a + dy_phys * cos_a;

    let u_src = clamp(0.5 + rot_x / aspect, 0.0, 1.0);
    let v_src = clamp(0.5 + rot_y, 0.0, 1.0);

    let sample_px = textureSampleLevel(src_tex, smp, vec2<f32>(u_src, v_src), 0.0);
    textureStore(dst_tex, vec2<i32>(i32(gid.x), i32(gid.y)), sample_px);
}
"#;

pub const VIEWPORT_WGSL: &str = r#"
struct ViewportUniforms {
    viewport_size: vec2<f32>,
    image_size: vec2<f32>,
    crop_rect: vec4<f32>,
    pan: vec2<f32>,
    zoom: f32,
    straighten_rad: f32,
    inscribed_scale: f32,
    quarter_turns: u32,
    flip_flags: u32,
    crop_preview_mode: u32,
    output_is_srgb: u32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
};

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var smp: sampler;
@group(0) @binding(2) var<uniform> u: ViewportUniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) screen_pos: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vid: u32) -> VertexOutput {
    var out: VertexOutput;
    var pos = vec2<f32>(0.0, 0.0);
    if (vid == 0u) {
        pos = vec2<f32>(-1.0, -1.0);
    } else if (vid == 1u) {
        pos = vec2<f32>(3.0, -1.0);
    } else {
        pos = vec2<f32>(-1.0, 3.0);
    }
    out.position = vec4<f32>(pos, 0.0, 1.0);
    out.screen_pos = (pos * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5)) * u.viewport_size;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Canvas background #141416
    let backdrop = vec3<f32>(0.07843137, 0.07843137, 0.08627451);

    // Determine displayed image aspect ratio
    let turns = u.quarter_turns % 4u;
    var base_w = u.image_size.x;
    var base_h = u.image_size.y;
    if (turns == 1u || turns == 3u) {
        base_w = u.image_size.y;
        base_h = u.image_size.x;
    }

    var disp_w = base_w;
    var disp_h = base_h;
    if (u.crop_preview_mode == 0u) {
        disp_w = base_w * u.crop_rect.z;
        disp_h = base_h * u.crop_rect.w;
    }

    let img_aspect = disp_w / max(disp_h, 1e-4);
    let vp_aspect = u.viewport_size.x / max(u.viewport_size.y, 1e-4);

    var fit_w = u.viewport_size.x;
    var fit_h = u.viewport_size.y;
    if (img_aspect > vp_aspect) {
        fit_h = fit_w / img_aspect;
    } else {
        fit_w = fit_h * img_aspect;
    }

    let quad_w = fit_w * u.zoom;
    let quad_h = fit_h * u.zoom;
    let center = u.viewport_size * 0.5 + u.pan;
    let rel_pos = in.screen_pos - center;

    let quad_u = rel_pos.x / quad_w + 0.5;
    let quad_v = rel_pos.y / quad_h + 0.5;

    if (quad_u < 0.0 || quad_u > 1.0 || quad_v < 0.0 || quad_v > 1.0) {
        return vec4<f32>(backdrop, 1.0);
    }

    var u_base = quad_u;
    var v_base = quad_v;
    var in_crop_region = true;

    if (u.crop_preview_mode == 0u) {
        u_base = u.crop_rect.x + quad_u * u.crop_rect.z;
        v_base = u.crop_rect.y + quad_v * u.crop_rect.w;
    } else {
        if (quad_u < u.crop_rect.x || quad_u > (u.crop_rect.x + u.crop_rect.z) ||
            quad_v < u.crop_rect.y || quad_v > (u.crop_rect.y + u.crop_rect.w)) {
            in_crop_region = false;
        }
    }

    // Undo flip
    if ((u.flip_flags & 1u) != 0u) {
        u_base = 1.0 - u_base;
    }
    if ((u.flip_flags & 2u) != 0u) {
        v_base = 1.0 - v_base;
    }

    // Undo quarter turns
    var u2 = u_base - 0.5;
    var v2 = v_base - 0.5;
    if (turns == 1u) {
        let temp = u2;
        u2 = v2;
        v2 = -temp;
    } else if (turns == 2u) {
        u2 = -u2;
        v2 = -v2;
    } else if (turns == 3u) {
        let temp = u2;
        u2 = -v2;
        v2 = temp;
    }

    // Undo straighten
    let s = u.inscribed_scale;
    let dx = u2 * s;
    let dy = v2 * s;

    let aspect = u.image_size.x / u.image_size.y;
    let dx_phys = dx * aspect;
    let dy_phys = dy;

    let cos_a = cos(u.straighten_rad);
    let sin_a = sin(u.straighten_rad);
    let rot_x = dx_phys * cos_a + dy_phys * sin_a;
    let rot_y = -dx_phys * sin_a + dy_phys * cos_a;

    let u_src = clamp(0.5 + rot_x / aspect, 0.0, 1.0);
    let v_src = clamp(0.5 + rot_y, 0.0, 1.0);

    let px = textureSampleLevel(src_tex, smp, vec2<f32>(u_src, v_src), 0.0);
    var rgb_lin_srgb = max(REC2020_TO_SRGB * px.rgb, vec3<f32>(0.0));

    if (u.crop_preview_mode != 0u) {
        if (!in_crop_region) {
            rgb_lin_srgb = rgb_lin_srgb * 0.32;
        } else {
            // Draw crisp anti-aliased border & rule-of-thirds lines
            let border_px = 1.5;
            let crop_px_x = (quad_u - u.crop_rect.x) * quad_w;
            let crop_px_y = (quad_v - u.crop_rect.y) * quad_h;
            let crop_w_px = u.crop_rect.z * quad_w;
            let crop_h_px = u.crop_rect.w * quad_h;

            let dist_left   = abs(crop_px_x);
            let dist_right  = abs(crop_px_x - crop_w_px);
            let dist_top    = abs(crop_px_y);
            let dist_bottom = abs(crop_px_y - crop_h_px);

            let dist_edge = min(min(dist_left, dist_right), min(dist_top, dist_bottom));
            let is_border = dist_edge < border_px;

            let rot1_x = abs(crop_px_x - crop_w_px * 0.3333);
            let rot2_x = abs(crop_px_x - crop_w_px * 0.6666);
            let rot1_y = abs(crop_px_y - crop_h_px * 0.3333);
            let rot2_y = abs(crop_px_y - crop_h_px * 0.6666);

            let dist_rot = min(min(rot1_x, rot2_x), min(rot1_y, rot2_y));
            let is_guide = dist_rot < 1.0;

            if (is_border) {
                rgb_lin_srgb = mix(rgb_lin_srgb, vec3<f32>(1.0, 1.0, 1.0), 0.9);
            } else if (is_guide) {
                rgb_lin_srgb = mix(rgb_lin_srgb, vec3<f32>(1.0, 1.0, 1.0), 0.45);
            }
        }
    }

    if (u.output_is_srgb != 0u) {
        return vec4<f32>(rgb_lin_srgb, 1.0);
    } else {
        return vec4<f32>(linear_to_srgb(rgb_lin_srgb), 1.0);
    }
}
"#;
