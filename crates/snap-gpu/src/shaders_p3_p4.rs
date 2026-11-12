use snap_core::ops_ext::{
    DoubleExposureBlend, DoubleExposureParams, DramaParams, DramaPreset, ExpandFillMode,
    ExpandParams, FrameStyle, FramesParams, GlamourGlowParams, GrungeParams, HdrScapeParams,
    HdrScapePreset, HeadPoseParams, HealingParams, LensBlurParams, LensBlurShape, MotionBlurMode,
    MotionBlurParams, PerspectiveFillMode, PerspectiveParams, PortraitParams, PortraitSkinTone,
    RetroluxParams, TextColorPreset, TextOverlayParams, VintageParams,
};

fn srgb_to_linear_ch(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn srgb_to_rec2020_lin(r: f32, g: f32, b: f32) -> [f32; 3] {
    let lr = srgb_to_linear_ch(r);
    let lg = srgb_to_linear_ch(g);
    let lb = srgb_to_linear_ch(b);
    [
        0.6274041 * lr + 0.3292821 * lg + 0.0433138 * lb,
        0.0690972 * lr + 0.9195410 * lg + 0.0113612 * lb,
        0.0163914 * lr + 0.0880133 * lg + 0.8955953 * lb,
    ]
}

fn invert_mat3(m: &[[f32; 3]; 3]) -> [[f32; 3]; 3] {
    let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
    if det.abs() < 1e-8 {
        return [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    }
    let inv_det = 1.0 / det;
    [
        [
            (m[1][1] * m[2][2] - m[1][2] * m[2][1]) * inv_det,
            (m[0][2] * m[2][1] - m[0][1] * m[2][2]) * inv_det,
            (m[0][1] * m[1][2] - m[0][2] * m[1][1]) * inv_det,
        ],
        [
            (m[1][2] * m[2][0] - m[1][0] * m[2][2]) * inv_det,
            (m[0][0] * m[2][2] - m[0][2] * m[2][0]) * inv_det,
            (m[0][2] * m[1][0] - m[0][0] * m[1][2]) * inv_det,
        ],
        [
            (m[1][0] * m[2][1] - m[1][1] * m[2][0]) * inv_det,
            (m[0][1] * m[2][0] - m[0][0] * m[2][1]) * inv_det,
            (m[0][0] * m[1][1] - m[0][1] * m[1][0]) * inv_det,
        ],
    ]
}

// ============================================================================
// 1. Healing
// ============================================================================

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuHealSpot {
    pub p0: [f32; 2],
    pub p1: [f32; 2],
    pub radius: f32,
    pub off_x: f32,
    pub off_y: f32,
    pub has_off: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct HealingUniforms {
    pub spots: [GpuHealSpot; 32],
    pub count: u32,
    pub opacity: f32,
    pub _pad0: f32,
    pub _pad1: f32,
}

impl HealingUniforms {
    pub fn from_params(params: &HealingParams, _width: u32, _height: u32, opacity: f32) -> Self {
        let mut spots = [GpuHealSpot {
            p0: [0.0, 0.0],
            p1: [0.0, 0.0],
            radius: 0.0,
            off_x: 0.0,
            off_y: 0.0,
            has_off: 0.0,
        }; 32];
        let mut count = 0u32;

        for stroke in &params.strokes {
            if count >= 32 {
                break;
            }
            let r = stroke.radius;
            let (off_x, off_y, has_off) = match stroke.source_offset {
                Some(off) => (off[0], off[1], 1.0),
                None => (0.0, 0.0, 0.0),
            };

            if stroke.points.is_empty() {
                continue;
            } else if stroke.points.len() == 1 {
                spots[count as usize] = GpuHealSpot {
                    p0: stroke.points[0],
                    p1: stroke.points[0],
                    radius: r,
                    off_x,
                    off_y,
                    has_off,
                };
                count += 1;
            } else {
                for i in 0..stroke.points.len() - 1 {
                    if count >= 32 {
                        break;
                    }
                    spots[count as usize] = GpuHealSpot {
                        p0: stroke.points[i],
                        p1: stroke.points[i + 1],
                        radius: r,
                        off_x,
                        off_y,
                        has_off,
                    };
                    count += 1;
                }
            }
        }

        Self {
            spots,
            count,
            opacity: opacity.clamp(0.0, 1.0),
            _pad0: 0.0,
            _pad1: 0.0,
        }
    }
}

pub const HEALING_WGSL: &str = r#"
struct GpuHealSpot {
    p0: vec2<f32>,
    p1: vec2<f32>,
    radius: f32,
    off_x: f32,
    off_y: f32,
    has_off: f32,
};

struct HealingUniforms {
    spots: array<GpuHealSpot, 32>,
    count: u32,
    opacity: f32,
    _pad0: f32,
    _pad1: f32,
};

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var dst_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var<uniform> u: HealingUniforms;

fn sample_src_bilinear(coord_f: vec2<f32>, dim: vec2<i32>) -> vec3<f32> {
    let base = floor(coord_f);
    let f = coord_f - base;
    let x0 = clamp(i32(base.x), 0, dim.x - 1);
    let x1 = clamp(i32(base.x) + 1, 0, dim.x - 1);
    let y0 = clamp(i32(base.y), 0, dim.y - 1);
    let y1 = clamp(i32(base.y) + 1, 0, dim.y - 1);

    let p00 = textureLoad(src_tex, vec2<i32>(x0, y0), 0).rgb;
    let p10 = textureLoad(src_tex, vec2<i32>(x1, y0), 0).rgb;
    let p01 = textureLoad(src_tex, vec2<i32>(x0, y1), 0).rgb;
    let p11 = textureLoad(src_tex, vec2<i32>(x1, y1), 0).rgb;

    let top = mix(p00, p10, f.x);
    let bot = mix(p01, p11, f.x);
    return mix(top, bot, f.y);
}

fn dist_to_segment(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>) -> f32 {
    let pa = p - a;
    let ba = b - a;
    let ba_len_sq = dot(ba, ba);
    var t = 0.0;
    if (ba_len_sq > 1e-8) {
        t = clamp(dot(pa, ba) / ba_len_sq, 0.0, 1.0);
    }
    return length(pa - ba * t);
}

@compute @workgroup_size(16, 16, 1)
fn cs_healing(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dim = textureDimensions(src_tex);
    if (gid.x >= dim.x || gid.y >= dim.y) {
        return;
    }
    let coord = vec2<i32>(i32(gid.x), i32(gid.y));
    let in_px = textureLoad(src_tex, coord, 0);

    if (u.count == 0u || u.opacity <= 0.0) {
        textureStore(dst_tex, coord, in_px);
        return;
    }

    let aspect = f32(dim.x) / f32(dim.y);
    let uv = (vec2<f32>(coord) + 0.5) / vec2<f32>(dim);
    let p_asp = vec2<f32>(uv.x * aspect, uv.y);

    var best_weight = 0.0;
    var healed_rgb = in_px.rgb;

    for (var i = 0u; i < 32u && i < u.count; i = i + 1u) {
        let spot = u.spots[i];
        let p0_asp = vec2<f32>(spot.p0.x * aspect, spot.p0.y);
        let p1_asp = vec2<f32>(spot.p1.x * aspect, spot.p1.y);
        let d = dist_to_segment(p_asp, p0_asp, p1_asp);

        if (d < spot.radius) {
            let feather_start = 0.65 * spot.radius;
            let w = 1.0 - smoothstep(feather_start, spot.radius, d);

            var sample_rgb = in_px.rgb;
            if (spot.has_off > 0.5) {
                let off_coord = (uv + vec2<f32>(spot.off_x, spot.off_y)) * vec2<f32>(dim);
                sample_rgb = sample_src_bilinear(off_coord, vec2<i32>(dim));
            } else {
                let ring_r = spot.radius * 1.35;
                var ring_sum = vec3<f32>(0.0);
                var ring_w = 0.0;
                let angles = array<f32, 6>(0.0, 1.04719755, 2.0943951, 3.14159265, 4.1887902, 5.23598776);
                for (var k = 0u; k < 6u; k = k + 1u) {
                    let dir = vec2<f32>(cos(angles[k]) / aspect, sin(angles[k]));
                    let ring_uv = uv + dir * ring_r;
                    let tap_coord = ring_uv * vec2<f32>(dim);
                    let tap_px = sample_src_bilinear(tap_coord, vec2<i32>(dim));
                    let tap_luma = rec2020_luma(tap_px);
                    let in_luma = rec2020_luma(in_px.rgb);
                    let kw = exp(-abs(tap_luma - in_luma) * 4.0);
                    ring_sum += tap_px * kw;
                    ring_w += kw;
                }
                sample_rgb = ring_sum / max(ring_w, 1e-4);
            }

            if (w > best_weight) {
                best_weight = w;
                healed_rgb = sample_rgb;
            }
        }
    }

    let out_rgb = mix(in_px.rgb, healed_rgb, best_weight * u.opacity);
    textureStore(dst_tex, coord, vec4<f32>(out_rgb, in_px.a));
}
"#;

// ============================================================================
// 2. Perspective
// ============================================================================

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PerspectiveUniforms {
    pub inv_homography: [f32; 12],
    pub edge_fill: u32,
    pub opacity: f32,
    pub _pad0: f32,
    pub _pad1: f32,
}

impl PerspectiveUniforms {
    pub fn from_params(params: &PerspectiveParams, width: u32, height: u32, opacity: f32) -> Self {
        let edge_fill = match params.edge_fill {
            PerspectiveFillMode::Black => 0,
            PerspectiveFillMode::White => 1,
            PerspectiveFillMode::SmartMirror => 2,
        };

        let aspect = width as f32 / (height as f32).max(1.0);
        let theta = params.rotate_deg.to_radians();
        let cos_t = theta.cos();
        let sin_t = theta.sin();
        let scale = params.scale.max(0.01);
        let kx = (params.horizontal / 100.0) * 0.75;
        let ky = (params.vertical / 100.0) * 0.75;

        let mul3 = |m1: [[f32; 3]; 3], m2: [[f32; 3]; 3]| -> [[f32; 3]; 3] {
            let mut r = [[0.0; 3]; 3];
            for i in 0..3 {
                for j in 0..3 {
                    r[i][j] = m1[i][0] * m2[0][j] + m1[i][1] * m2[1][j] + m1[i][2] * m2[2][j];
                }
            }
            r
        };

        let t_center = [[1.0, 0.0, -0.5], [0.0, 1.0, -0.5], [0.0, 0.0, 1.0]];
        let m_asp = [[aspect, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let m_rot_scale = [
            [scale * cos_t, -scale * sin_t, 0.0],
            [scale * sin_t, scale * cos_t, 0.0],
            [0.0, 0.0, 1.0],
        ];
        let m_persp = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [kx, ky, 1.0]];
        let m_asp_inv = [[1.0 / aspect, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let t_uncenter = [[1.0, 0.0, 0.5], [0.0, 1.0, 0.5], [0.0, 0.0, 1.0]];

        let mut m_fwd = t_uncenter;
        m_fwd = mul3(m_fwd, m_asp_inv);
        m_fwd = mul3(m_fwd, m_persp);
        m_fwd = mul3(m_fwd, m_rot_scale);
        m_fwd = mul3(m_fwd, m_asp);
        m_fwd = mul3(m_fwd, t_center);

        let inv_m = invert_mat3(&m_fwd);

        let inv_homography = [
            inv_m[0][0], inv_m[1][0], inv_m[2][0], 0.0,
            inv_m[0][1], inv_m[1][1], inv_m[2][1], 0.0,
            inv_m[0][2], inv_m[1][2], inv_m[2][2], 0.0,
        ];

        Self {
            inv_homography,
            edge_fill,
            opacity: opacity.clamp(0.0, 1.0),
            _pad0: 0.0,
            _pad1: 0.0,
        }
    }
}

pub const PERSPECTIVE_WGSL: &str = r#"
struct PerspectiveUniforms {
    inv_h0: vec4<f32>,
    inv_h1: vec4<f32>,
    inv_h2: vec4<f32>,
    edge_fill: u32,
    opacity: f32,
    _pad0: f32,
    _pad1: f32,
};

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var dst_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var<uniform> u: PerspectiveUniforms;

fn sample_src_bilinear(coord_f: vec2<f32>, dim: vec2<i32>) -> vec3<f32> {
    let base = floor(coord_f);
    let f = coord_f - base;
    let x0 = clamp(i32(base.x), 0, dim.x - 1);
    let x1 = clamp(i32(base.x) + 1, 0, dim.x - 1);
    let y0 = clamp(i32(base.y), 0, dim.y - 1);
    let y1 = clamp(i32(base.y) + 1, 0, dim.y - 1);

    let p00 = textureLoad(src_tex, vec2<i32>(x0, y0), 0).rgb;
    let p10 = textureLoad(src_tex, vec2<i32>(x1, y0), 0).rgb;
    let p01 = textureLoad(src_tex, vec2<i32>(x0, y1), 0).rgb;
    let p11 = textureLoad(src_tex, vec2<i32>(x1, y1), 0).rgb;

    let top = mix(p00, p10, f.x);
    let bot = mix(p01, p11, f.x);
    return mix(top, bot, f.y);
}

@compute @workgroup_size(16, 16, 1)
fn cs_perspective(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dim = textureDimensions(src_tex);
    if (gid.x >= dim.x || gid.y >= dim.y) {
        return;
    }
    let coord = vec2<i32>(i32(gid.x), i32(gid.y));
    let in_px = textureLoad(src_tex, coord, 0);

    let uv = (vec2<f32>(coord) + 0.5) / vec2<f32>(dim);
    let h = mat3x3<f32>(u.inv_h0.xyz, u.inv_h1.xyz, u.inv_h2.xyz);
    let p_src = h * vec3<f32>(uv.x, uv.y, 1.0);
    let src_uv = p_src.xy / max(abs(p_src.z), 1e-6);

    var sample_rgb = vec3<f32>(0.0);
    if (src_uv.x >= 0.0 && src_uv.x <= 1.0 && src_uv.y >= 0.0 && src_uv.y <= 1.0) {
        sample_rgb = sample_src_bilinear(src_uv * vec2<f32>(dim), vec2<i32>(dim));
    } else {
        if (u.edge_fill == 0u) {
            sample_rgb = vec3<f32>(0.0);
        } else if (u.edge_fill == 1u) {
            sample_rgb = vec3<f32>(1.0);
        } else {
            let mx = 1.0 - abs(abs(src_uv.x) % 2.0 - 1.0);
            let my = 1.0 - abs(abs(src_uv.y) % 2.0 - 1.0);
            let mirr_col = sample_src_bilinear(vec2<f32>(mx, my) * vec2<f32>(dim), vec2<i32>(dim));
            let edge_dist = max(max(-src_uv.x, src_uv.x - 1.0), max(-src_uv.y, src_uv.y - 1.0));
            let feather = 1.0 - smoothstep(0.0, 0.15, edge_dist) * 0.25;
            sample_rgb = mirr_col * feather;
        }
    }

    let final_rgb = mix(in_px.rgb, sample_rgb, u.opacity);
    textureStore(dst_tex, coord, vec4<f32>(final_rgb, in_px.a));
}
"#;

// ============================================================================
// 3. Expand
// ============================================================================

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ExpandUniforms {
    pub margins: [f32; 4],
    pub fill_mode: u32,
    pub opacity: f32,
    pub _pad0: f32,
    pub _pad1: f32,
}

impl ExpandUniforms {
    pub fn from_params(params: &ExpandParams, _width: u32, _height: u32, opacity: f32) -> Self {
        let fill_mode = match params.fill_mode {
            ExpandFillMode::Black => 0,
            ExpandFillMode::White => 1,
            ExpandFillMode::SmartEdge => 2,
        };
        Self {
            margins: [
                params.top.clamp(0.0, 0.45),
                params.bottom.clamp(0.0, 0.45),
                params.left.clamp(0.0, 0.45),
                params.right.clamp(0.0, 0.45),
            ],
            fill_mode,
            opacity: opacity.clamp(0.0, 1.0),
            _pad0: 0.0,
            _pad1: 0.0,
        }
    }
}

pub const EXPAND_WGSL: &str = r#"
struct ExpandUniforms {
    margins: vec4<f32>,
    fill_mode: u32,
    opacity: f32,
    _pad0: f32,
    _pad1: f32,
};

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var dst_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var<uniform> u: ExpandUniforms;

fn sample_src_bilinear(coord_f: vec2<f32>, dim: vec2<i32>) -> vec3<f32> {
    let base = floor(coord_f);
    let f = coord_f - base;
    let x0 = clamp(i32(base.x), 0, dim.x - 1);
    let x1 = clamp(i32(base.x) + 1, 0, dim.x - 1);
    let y0 = clamp(i32(base.y), 0, dim.y - 1);
    let y1 = clamp(i32(base.y) + 1, 0, dim.y - 1);

    let p00 = textureLoad(src_tex, vec2<i32>(x0, y0), 0).rgb;
    let p10 = textureLoad(src_tex, vec2<i32>(x1, y0), 0).rgb;
    let p01 = textureLoad(src_tex, vec2<i32>(x0, y1), 0).rgb;
    let p11 = textureLoad(src_tex, vec2<i32>(x1, y1), 0).rgb;

    let top = mix(p00, p10, f.x);
    let bot = mix(p01, p11, f.x);
    return mix(top, bot, f.y);
}

@compute @workgroup_size(16, 16, 1)
fn cs_expand(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dim = textureDimensions(src_tex);
    if (gid.x >= dim.x || gid.y >= dim.y) {
        return;
    }
    let coord = vec2<i32>(i32(gid.x), i32(gid.y));
    let in_px = textureLoad(src_tex, coord, 0);

    let uv = (vec2<f32>(coord) + 0.5) / vec2<f32>(dim);

    let inner_w = max(1.0 - u.margins.z - u.margins.w, 0.01);
    let inner_h = max(1.0 - u.margins.x - u.margins.y, 0.01);
    let u_orig = (uv.x - u.margins.z) / inner_w;
    let v_orig = (uv.y - u.margins.x) / inner_h;

    var col = vec3<f32>(0.0);
    if (u_orig >= 0.0 && u_orig <= 1.0 && v_orig >= 0.0 && v_orig <= 1.0) {
        col = sample_src_bilinear(vec2<f32>(u_orig, v_orig) * vec2<f32>(dim), vec2<i32>(dim));
    } else {
        if (u.fill_mode == 0u) {
            col = vec3<f32>(0.0);
        } else if (u.fill_mode == 1u) {
            col = vec3<f32>(1.0);
        } else {
            let clamped_u = clamp(u_orig, 0.0, 1.0);
            let clamped_v = clamp(v_orig, 0.0, 1.0);
            let mirr_u = 1.0 - abs(abs(u_orig) % 2.0 - 1.0);
            let mirr_v = 1.0 - abs(abs(v_orig) % 2.0 - 1.0);
            let s_mirr = sample_src_bilinear(vec2<f32>(mirr_u, mirr_v) * vec2<f32>(dim), vec2<i32>(dim));
            let s_edge = sample_src_bilinear(vec2<f32>(clamped_u, clamped_v) * vec2<f32>(dim), vec2<i32>(dim));
            let d_out = max(max(-u_orig, u_orig - 1.0), max(-v_orig, v_orig - 1.0));
            let blend_k = smoothstep(0.0, 0.08, d_out);
            col = mix(s_edge, s_mirr, blend_k);
        }
    }

    let final_rgb = mix(in_px.rgb, col, u.opacity);
    textureStore(dst_tex, coord, vec4<f32>(final_rgb, in_px.a));
}
"#;

// ============================================================================
// 4. Lens Blur
// ============================================================================

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LensBlurUniforms {
    pub center: [f32; 2],
    pub inner_radius: f32,
    pub transition: f32,
    pub angle_rad: f32,
    pub blur_strength: f32,
    pub bokeh_boost: f32,
    pub vignette_strength: f32,
    pub shape: u32,
    pub opacity: f32,
    pub aspect: f32,
    pub _pad0: f32,
}

impl LensBlurUniforms {
    pub fn from_params(params: &LensBlurParams, width: u32, height: u32, opacity: f32) -> Self {
        let shape = match params.shape {
            LensBlurShape::Elliptical => 0,
            LensBlurShape::Linear => 1,
        };
        let aspect = width as f32 / (height as f32).max(1.0);
        Self {
            center: params.center,
            inner_radius: params.inner_radius.max(0.01),
            transition: params.transition.max(0.01),
            angle_rad: params.angle_deg.to_radians(),
            blur_strength: params.blur_strength.clamp(0.0, 100.0),
            bokeh_boost: params.bokeh_boost.clamp(0.0, 100.0),
            vignette_strength: params.vignette_strength.clamp(0.0, 100.0),
            shape,
            opacity: opacity.clamp(0.0, 1.0),
            aspect,
            _pad0: 0.0,
        }
    }
}

pub const LENS_BLUR_WGSL: &str = r#"
struct LensBlurUniforms {
    center: vec2<f32>,
    inner_radius: f32,
    transition: f32,
    angle_rad: f32,
    blur_strength: f32,
    bokeh_boost: f32,
    vignette_strength: f32,
    shape: u32,
    opacity: f32,
    aspect: f32,
    _pad0: f32,
};

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var dst_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var<uniform> u: LensBlurUniforms;

fn sample_src_bilinear(coord_f: vec2<f32>, dim: vec2<i32>) -> vec3<f32> {
    let base = floor(coord_f);
    let f = coord_f - base;
    let x0 = clamp(i32(base.x), 0, dim.x - 1);
    let x1 = clamp(i32(base.x) + 1, 0, dim.x - 1);
    let y0 = clamp(i32(base.y), 0, dim.y - 1);
    let y1 = clamp(i32(base.y) + 1, 0, dim.y - 1);

    let p00 = textureLoad(src_tex, vec2<i32>(x0, y0), 0).rgb;
    let p10 = textureLoad(src_tex, vec2<i32>(x1, y0), 0).rgb;
    let p01 = textureLoad(src_tex, vec2<i32>(x0, y1), 0).rgb;
    let p11 = textureLoad(src_tex, vec2<i32>(x1, y1), 0).rgb;

    let top = mix(p00, p10, f.x);
    let bot = mix(p01, p11, f.x);
    return mix(top, bot, f.y);
}

const GOLDEN_ANGLE: f32 = 2.39996323;

@compute @workgroup_size(16, 16, 1)
fn cs_lens_blur(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dim = textureDimensions(src_tex);
    if (gid.x >= dim.x || gid.y >= dim.y) {
        return;
    }
    let coord = vec2<i32>(i32(gid.x), i32(gid.y));
    let in_px = textureLoad(src_tex, coord, 0);

    let uv = (vec2<f32>(coord) + 0.5) / vec2<f32>(dim);
    let d_uv = uv - u.center;

    var blur_factor = 0.0;
    if (u.shape == 0u) {
        let cos_a = cos(-u.angle_rad);
        let sin_a = sin(-u.angle_rad);
        let rot_x = (d_uv.x * cos_a - d_uv.y * sin_a) * u.aspect;
        let rot_y = (d_uv.x * sin_a + d_uv.y * cos_a);
        let dist = length(vec2<f32>(rot_x, rot_y));
        blur_factor = smoothstep(u.inner_radius, u.inner_radius + max(u.transition, 1e-4), dist);
    } else {
        let perp = vec2<f32>(-sin(u.angle_rad), cos(u.angle_rad));
        let dist = abs(dot(d_uv * vec2<f32>(u.aspect, 1.0), perp));
        blur_factor = smoothstep(u.inner_radius, u.inner_radius + max(u.transition, 1e-4), dist);
    }

    var blurred_rgb = in_px.rgb;
    let max_radius = (u.blur_strength / 100.0) * 28.0 * blur_factor;

    if (max_radius > 0.5) {
        var sum_rgb = vec3<f32>(0.0);
        var sum_w = 0.0;
        let bokeh_k = u.bokeh_boost / 10.0;

        for (var i = 0u; i < 32u; i = i + 1u) {
            let theta = f32(i) * GOLDEN_ANGLE;
            let r = sqrt((f32(i) + 0.5) / 32.0) * max_radius;
            let tap_coord = vec2<f32>(coord) + vec2<f32>(cos(theta), sin(theta)) * r;
            let tap_px = sample_src_bilinear(tap_coord, vec2<i32>(dim));
            let tap_luma = rec2020_luma(tap_px);
            let w = 1.0 + bokeh_k * pow(max(tap_luma - 0.55, 0.0), 2.5);
            sum_rgb += tap_px * w;
            sum_w += w;
        }
        blurred_rgb = sum_rgb / max(sum_w, 1e-4);
    }

    let vig_dist = length((uv - vec2<f32>(0.5, 0.5)) * vec2<f32>(u.aspect, 1.0));
    let vig_mult = 1.0 - (u.vignette_strength / 100.0) * smoothstep(0.35, 1.1, vig_dist) * 0.75;
    let final_blur = blurred_rgb * vig_mult;

    let out_rgb = mix(in_px.rgb, final_blur, u.opacity);
    textureStore(dst_tex, coord, vec4<f32>(out_rgb, in_px.a));
}
"#;

// ============================================================================
// 5. Motion Blur
// ============================================================================

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MotionBlurUniforms {
    pub center: [f32; 2],
    pub angle_rad: f32,
    pub strength: f32,
    pub mode: u32,
    pub opacity: f32,
    pub aspect: f32,
    pub _pad0: f32,
}

impl MotionBlurUniforms {
    pub fn from_params(params: &MotionBlurParams, width: u32, height: u32, opacity: f32) -> Self {
        let mode = match params.mode {
            MotionBlurMode::FullScene => 0,
            MotionBlurMode::RadialPan => 1,
        };
        let aspect = width as f32 / (height as f32).max(1.0);
        Self {
            center: params.center,
            angle_rad: params.angle_deg.to_radians(),
            strength: params.strength.clamp(0.0, 100.0),
            mode,
            opacity: opacity.clamp(0.0, 1.0),
            aspect,
            _pad0: 0.0,
        }
    }
}

pub const MOTION_BLUR_WGSL: &str = r#"
struct MotionBlurUniforms {
    center: vec2<f32>,
    angle_rad: f32,
    strength: f32,
    mode: u32,
    opacity: f32,
    aspect: f32,
    _pad0: f32,
};

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var dst_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var<uniform> u: MotionBlurUniforms;

fn sample_src_bilinear(coord_f: vec2<f32>, dim: vec2<i32>) -> vec3<f32> {
    let base = floor(coord_f);
    let f = coord_f - base;
    let x0 = clamp(i32(base.x), 0, dim.x - 1);
    let x1 = clamp(i32(base.x) + 1, 0, dim.x - 1);
    let y0 = clamp(i32(base.y), 0, dim.y - 1);
    let y1 = clamp(i32(base.y) + 1, 0, dim.y - 1);

    let p00 = textureLoad(src_tex, vec2<i32>(x0, y0), 0).rgb;
    let p10 = textureLoad(src_tex, vec2<i32>(x1, y0), 0).rgb;
    let p01 = textureLoad(src_tex, vec2<i32>(x0, y1), 0).rgb;
    let p11 = textureLoad(src_tex, vec2<i32>(x1, y1), 0).rgb;

    let top = mix(p00, p10, f.x);
    let bot = mix(p01, p11, f.x);
    return mix(top, bot, f.y);
}

@compute @workgroup_size(16, 16, 1)
fn cs_motion_blur(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dim = textureDimensions(src_tex);
    if (gid.x >= dim.x || gid.y >= dim.y) {
        return;
    }
    let coord = vec2<i32>(i32(gid.x), i32(gid.y));
    let in_px = textureLoad(src_tex, coord, 0);

    if (u.strength <= 0.0 || u.opacity <= 0.0) {
        textureStore(dst_tex, coord, in_px);
        return;
    }

    let uv = (vec2<f32>(coord) + 0.5) / vec2<f32>(dim);
    var streak_vec = vec2<f32>(0.0);

    if (u.mode == 0u) {
        let dir = vec2<f32>(cos(u.angle_rad), sin(u.angle_rad));
        let streak_len = (u.strength / 100.0) * 45.0;
        streak_vec = dir * streak_len;
    } else {
        let d = (uv - u.center) * vec2<f32>(u.aspect, 1.0);
        let dist = length(d);
        if (dist > 1e-4) {
            let rad_dir = d / dist;
            let tan_dir = vec2<f32>(-rad_dir.y, rad_dir.x);
            let dir = normalize(mix(rad_dir, tan_dir, 0.25));
            let streak_len = (u.strength / 100.0) * dist * 60.0;
            streak_vec = dir * streak_len;
        }
    }

    var sum_rgb = vec3<f32>(0.0);
    var sum_w = 0.0;

    for (var k = 0u; k < 24u; k = k + 1u) {
        let t = (f32(k) - 11.5) / 11.5;
        let tap_coord = vec2<f32>(coord) + streak_vec * (t * 0.5);
        let tap_px = sample_src_bilinear(tap_coord, vec2<i32>(dim));
        let w = 0.5 * (1.0 + cos(3.14159265 * t));
        sum_rgb += tap_px * w;
        sum_w += w;
    }

    let blurred_rgb = sum_rgb / max(sum_w, 1e-4);
    let final_rgb = mix(in_px.rgb, blurred_rgb, u.opacity);
    textureStore(dst_tex, coord, vec4<f32>(final_rgb, in_px.a));
}
"#;

// ============================================================================
// 6. HDR Scape (2-Pass Separable)
// ============================================================================

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct HdrScapeUniforms {
    pub filter_strength: f32,
    pub brightness: f32,
    pub saturation: f32,
    pub opacity: f32,
    pub preset: u32,
    pub pass_idx: u32,
    pub _pad0: f32,
    pub _pad1: f32,
}

impl HdrScapeUniforms {
    pub fn from_params(params: &HdrScapeParams, pass_idx: u32, opacity: f32) -> Self {
        let preset = match params.preset {
            HdrScapePreset::Nature => 0,
            HdrScapePreset::People => 1,
            HdrScapePreset::Fine => 2,
            HdrScapePreset::Strong => 3,
        };
        Self {
            filter_strength: params.filter_strength.clamp(0.0, 100.0),
            brightness: params.brightness.clamp(-100.0, 100.0),
            saturation: params.saturation.clamp(-100.0, 100.0),
            opacity: opacity.clamp(0.0, 1.0),
            preset,
            pass_idx,
            _pad0: 0.0,
            _pad1: 0.0,
        }
    }
}

pub const HDR_SCAPE_WGSL: &str = r#"
struct HdrScapeUniforms {
    filter_strength: f32,
    brightness: f32,
    saturation: f32,
    opacity: f32,
    preset: u32,
    pass_idx: u32,
    _pad0: f32,
    _pad1: f32,
};

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var dst_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var<uniform> u: HdrScapeUniforms;

@compute @workgroup_size(16, 16, 1)
fn cs_hdr_scape(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dim = textureDimensions(src_tex);
    if (gid.x >= dim.x || gid.y >= dim.y) {
        return;
    }
    let coord = vec2<i32>(i32(gid.x), i32(gid.y));
    let center_px = textureLoad(src_tex, coord, 0);

    var sigma_s = 3.5;
    var sigma_r = 0.30;
    if (u.preset == 0u) { // Nature
        sigma_s = 4.5;
        sigma_r = 0.35;
    } else if (u.preset == 1u) { // People
        sigma_s = 3.0;
        sigma_r = 0.18;
    } else if (u.preset == 2u) { // Fine
        sigma_s = 2.0;
        sigma_r = 0.14;
    } else { // Strong
        sigma_s = 6.0;
        sigma_r = 0.45;
    }

    let two_sigma_s2 = 2.0 * sigma_s * sigma_s;
    let two_sigma_r2 = 2.0 * sigma_r * sigma_r;

    if (u.pass_idx == 0u) {
        let center_log_luma = log2(max(rec2020_luma(center_px.rgb), 1e-4) + 0.05);
        var sum_log = 0.0;
        var sum_w = 0.0;

        for (var k = -5; k <= 5; k = k + 1) {
            let tap_x = clamp(coord.x + k, 0, i32(dim.x) - 1);
            let tap_px = textureLoad(src_tex, vec2<i32>(tap_x, coord.y), 0);
            let tap_log_luma = log2(max(rec2020_luma(tap_px.rgb), 1e-4) + 0.05);

            let d_spatial = f32(k);
            let d_range = tap_log_luma - center_log_luma;
            let w = exp(- (d_spatial * d_spatial) / two_sigma_s2 - (d_range * d_range) / two_sigma_r2);

            sum_log += tap_log_luma * w;
            sum_w += w;
        }

        let luma_h = sum_log / max(sum_w, 1e-5);
        textureStore(dst_tex, coord, vec4<f32>(center_px.rgb, luma_h));
    } else {
        let center_log_luma = log2(max(rec2020_luma(center_px.rgb), 1e-4) + 0.05);
        var sum_log = 0.0;
        var sum_w = 0.0;

        for (var k = -5; k <= 5; k = k + 1) {
            let tap_y = clamp(coord.y + k, 0, i32(dim.y) - 1);
            let tap_px = textureLoad(src_tex, vec2<i32>(coord.x, tap_y), 0);
            let tap_log_luma = tap_px.a;

            let d_spatial = f32(k);
            let d_range = tap_log_luma - center_px.a;
            let w = exp(- (d_spatial * d_spatial) / two_sigma_s2 - (d_range * d_range) / two_sigma_r2);

            sum_log += tap_log_luma * w;
            sum_w += w;
        }

        let base_log_luma = sum_log / max(sum_w, 1e-5);
        let detail = center_log_luma - base_log_luma;

        let str = u.filter_strength / 100.0;
        let comp_base = base_log_luma * (1.0 - str * 0.28);
        let boosted_detail = detail * (1.0 + str * 1.55);
        let new_log_luma = comp_base + boosted_detail;

        let orig_luma = max(rec2020_luma(center_px.rgb), 1e-4);
        let new_luma = max(exp2(new_log_luma) - 0.05, 1e-4) + (u.brightness / 100.0) * 0.20;

        let scale_ratio = new_luma / orig_luma;
        var hdr_rgb = center_px.rgb * scale_ratio;

        let sat_mult = 1.0 + u.saturation / 100.0;
        let luma_val = rec2020_luma(hdr_rgb);
        hdr_rgb = mix(vec3<f32>(luma_val), hdr_rgb, sat_mult);

        let final_rgb = mix(center_px.rgb, hdr_rgb, u.opacity);
        textureStore(dst_tex, coord, vec4<f32>(final_rgb, 1.0));
    }
}
"#;

// ============================================================================
// 7. Glamour Glow (2-Pass Separable)
// ============================================================================

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GlamourGlowUniforms {
    pub glow: f32,
    pub saturation: f32,
    pub warmth: f32,
    pub opacity: f32,
    pub style: u32,
    pub pass_idx: u32,
    pub _pad0: f32,
    pub _pad1: f32,
}

impl GlamourGlowUniforms {
    pub fn from_params(params: &GlamourGlowParams, pass_idx: u32, opacity: f32) -> Self {
        Self {
            glow: params.glow.clamp(0.0, 100.0),
            saturation: params.saturation.clamp(-100.0, 100.0),
            warmth: params.warmth.clamp(-100.0, 100.0),
            opacity: opacity.clamp(0.0, 1.0),
            style: params.style.clamp(1, 5) as u32,
            pass_idx,
            _pad0: 0.0,
            _pad1: 0.0,
        }
    }
}

pub const GLAMOUR_GLOW_WGSL: &str = r#"
struct GlamourGlowUniforms {
    glow: f32,
    saturation: f32,
    warmth: f32,
    opacity: f32,
    style: u32,
    pass_idx: u32,
    _pad0: f32,
    _pad1: f32,
};

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var dst_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var<uniform> u: GlamourGlowUniforms;

@compute @workgroup_size(16, 16, 1)
fn cs_glamour_glow(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dim = textureDimensions(src_tex);
    if (gid.x >= dim.x || gid.y >= dim.y) {
        return;
    }
    let coord = vec2<i32>(i32(gid.x), i32(gid.y));
    let center_px = textureLoad(src_tex, coord, 0);

    let sigma = 4.0;
    let two_sigma2 = 2.0 * sigma * sigma;

    if (u.pass_idx == 0u) {
        var sum_val = 0.0;
        var sum_w = 0.0;

        for (var k = -6; k <= 6; k = k + 1) {
            let tap_x = clamp(coord.x + k * 2, 0, i32(dim.x) - 1);
            let tap_px = textureLoad(src_tex, vec2<i32>(tap_x, coord.y), 0);
            let tap_luma = rec2020_luma(tap_px.rgb);

            let d = f32(k);
            let w = exp(- (d * d) / two_sigma2);
            sum_val += tap_luma * w;
            sum_w += w;
        }

        let glow_h = sum_val / max(sum_w, 1e-5);
        textureStore(dst_tex, coord, vec4<f32>(center_px.rgb, glow_h));
    } else {
        var sum_val = 0.0;
        var sum_w = 0.0;

        for (var k = -6; k <= 6; k = k + 1) {
            let tap_y = clamp(coord.y + k * 2, 0, i32(dim.y) - 1);
            let tap_px = textureLoad(src_tex, vec2<i32>(coord.x, tap_y), 0);
            let tap_glow = tap_px.a;

            let d = f32(k);
            let w = exp(- (d * d) / two_sigma2);
            sum_val += tap_glow * w;
            sum_w += w;
        }

        let glow_base = sum_val / max(sum_w, 1e-5);
        let glow_k = u.glow / 100.0;

        // Soft-light diffused halo preserving HDR > 1.0 headroom
        var soft_light = center_px.rgb;
        let b = clamp(glow_base / (1.0 + glow_base) * 1.6, 0.0, 1.0);
        for (var c = 0; c < 3; c = c + 1) {
            let a_raw = center_px.rgb[c];
            let a = clamp(a_raw, 0.0, 1.0);
            let hdr_excess = max(a_raw - 1.0, 0.0);
            let sl = (1.0 - 2.0 * b) * (a * a) + 2.0 * b * a + hdr_excess * (1.0 + glow_k * 0.25);
            soft_light[c] = mix(a_raw, sl, glow_k * 0.75);
        }

        // Warmth / romantic tinting
        let warmed = apply_white_balance_rec2020(soft_light, u.warmth * 0.35, 0.0);

        // Saturation refinement
        let luma = rec2020_luma(warmed);
        let sat_mult = 1.0 + (u.saturation / 100.0) * 0.5;
        let tinted = mix(vec3<f32>(luma), warmed, sat_mult);

        let final_rgb = mix(center_px.rgb, tinted, u.opacity);
        textureStore(dst_tex, coord, vec4<f32>(final_rgb, 1.0));
    }
}
"#;

// ============================================================================
// 8. Drama
// ============================================================================

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DramaUniforms {
    pub filter_strength: f32,
    pub saturation: f32,
    pub opacity: f32,
    pub preset: u32,
}

impl DramaUniforms {
    pub fn from_params(params: &DramaParams, opacity: f32) -> Self {
        let preset = match params.preset {
            DramaPreset::Drama1 => 0,
            DramaPreset::Drama2 => 1,
            DramaPreset::Bright1 => 2,
            DramaPreset::Bright2 => 3,
            DramaPreset::Dark1 => 4,
            DramaPreset::Dark2 => 5,
        };
        Self {
            filter_strength: params.filter_strength.clamp(0.0, 100.0),
            saturation: params.saturation.clamp(-100.0, 100.0),
            opacity: opacity.clamp(0.0, 1.0),
            preset,
        }
    }
}

pub const DRAMA_WGSL: &str = r#"
struct DramaUniforms {
    filter_strength: f32,
    saturation: f32,
    opacity: f32,
    preset: u32,
};

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var dst_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var<uniform> u: DramaUniforms;

@compute @workgroup_size(16, 16, 1)
fn cs_drama(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dim = textureDimensions(src_tex);
    if (gid.x >= dim.x || gid.y >= dim.y) {
        return;
    }
    let coord = vec2<i32>(i32(gid.x), i32(gid.y));
    let in_px = textureLoad(src_tex, coord, 0);

    let offset = 4;
    let l_px = textureLoad(src_tex, vec2<i32>(max(0, coord.x - offset), coord.y), 0);
    let r_px = textureLoad(src_tex, vec2<i32>(min(i32(dim.x) - 1, coord.x + offset), coord.y), 0);
    let u_px = textureLoad(src_tex, vec2<i32>(coord.x, max(0, coord.y - offset)), 0);
    let d_px = textureLoad(src_tex, vec2<i32>(coord.x, min(i32(dim.y) - 1, coord.y + offset)), 0);

    let c_luma = rec2020_luma(in_px.rgb);
    let avg_luma = (c_luma + rec2020_luma(l_px.rgb) + rec2020_luma(r_px.rgb) + rec2020_luma(u_px.rgb) + rec2020_luma(d_px.rgb)) * 0.2;
    let micro_contrast = c_luma - avg_luma;

    let str = u.filter_strength / 100.0;
    var contrast_k = 1.35;
    var shadow_bias = 0.0;
    var highlight_bias = 0.0;

    if (u.preset == 0u) { // Drama1
        contrast_k = 1.30;
    } else if (u.preset == 1u) { // Drama2
        contrast_k = 1.55;
        shadow_bias = -0.05;
    } else if (u.preset == 2u) { // Bright1
        contrast_k = 1.25;
        highlight_bias = 0.10;
    } else if (u.preset == 3u) { // Bright2
        contrast_k = 1.45;
        highlight_bias = 0.15;
    } else if (u.preset == 4u) { // Dark1
        contrast_k = 1.40;
        shadow_bias = -0.12;
    } else { // Dark2
        contrast_k = 1.65;
        shadow_bias = -0.18;
    }

    let boosted_luma = c_luma + micro_contrast * (str * contrast_k) + shadow_bias * str + highlight_bias * str;
    let rolloff_luma = boosted_luma / (1.0 + max(boosted_luma - 1.0, 0.0) * 0.4);

    let luma_ratio = max(rolloff_luma, 1e-4) / max(c_luma, 1e-4);
    var dramatic_rgb = in_px.rgb * luma_ratio;

    let sat_mult = 1.0 + (u.saturation / 100.0) * 0.8;
    let dramatic_luma = rec2020_luma(dramatic_rgb);
    dramatic_rgb = mix(vec3<f32>(dramatic_luma), dramatic_rgb, sat_mult);

    let final_rgb = mix(in_px.rgb, dramatic_rgb, u.opacity);
    textureStore(dst_tex, coord, vec4<f32>(final_rgb, in_px.a));
}
"#;

// ============================================================================
// 9. Vintage
// ============================================================================

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VintageUniforms {
    pub brightness: f32,
    pub saturation: f32,
    pub style_strength: f32,
    pub vignette_strength: f32,
    pub center_size: f32,
    pub style: u32,
    pub opacity: f32,
    pub _pad0: f32,
}

impl VintageUniforms {
    pub fn from_params(params: &VintageParams, opacity: f32) -> Self {
        Self {
            brightness: params.brightness.clamp(-100.0, 100.0),
            saturation: params.saturation.clamp(-100.0, 100.0),
            style_strength: params.style_strength.clamp(0.0, 100.0),
            vignette_strength: params.vignette_strength.clamp(0.0, 100.0),
            center_size: params.center_size.clamp(0.0, 100.0),
            style: (params.style as u32).min(11),
            opacity: opacity.clamp(0.0, 1.0),
            _pad0: 0.0,
        }
    }
}

pub const VINTAGE_WGSL: &str = r#"
struct VintageUniforms {
    brightness: f32,
    saturation: f32,
    style_strength: f32,
    vignette_strength: f32,
    center_size: f32,
    style: u32,
    opacity: f32,
    _pad0: f32,
};

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var dst_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var<uniform> u: VintageUniforms;

@compute @workgroup_size(16, 16, 1)
fn cs_vintage(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dim = textureDimensions(src_tex);
    if (gid.x >= dim.x || gid.y >= dim.y) {
        return;
    }
    let coord = vec2<i32>(i32(gid.x), i32(gid.y));
    let in_px = textureLoad(src_tex, coord, 0);

    let uv = (vec2<f32>(coord) + 0.5) / vec2<f32>(dim);
    let luma = rec2020_luma(in_px.rgb);

    let shadow_tints = array<vec3<f32>, 12>(
        vec3<f32>(1.12, 1.02, 0.88), vec3<f32>(1.08, 1.05, 0.82), vec3<f32>(0.92, 1.06, 1.12),
        vec3<f32>(1.10, 0.95, 1.08), vec3<f32>(1.15, 1.02, 0.90), vec3<f32>(0.85, 1.08, 1.15),
        vec3<f32>(1.00, 1.00, 1.02), vec3<f32>(1.18, 1.10, 0.92), vec3<f32>(1.05, 1.02, 1.08),
        vec3<f32>(1.16, 1.00, 0.85), vec3<f32>(0.90, 1.12, 1.00), vec3<f32>(1.14, 1.06, 0.92)
    );

    let hl_tints = array<vec3<f32>, 12>(
        vec3<f32>(0.92, 0.98, 1.08), vec3<f32>(0.95, 0.92, 1.05), vec3<f32>(1.12, 0.95, 0.90),
        vec3<f32>(0.92, 1.05, 0.95), vec3<f32>(0.88, 0.95, 1.12), vec3<f32>(1.12, 1.02, 0.95),
        vec3<f32>(0.98, 0.98, 0.96), vec3<f32>(0.90, 0.95, 1.08), vec3<f32>(0.95, 1.05, 1.05),
        vec3<f32>(0.88, 0.96, 1.15), vec3<f32>(1.10, 1.04, 0.92), vec3<f32>(0.92, 0.98, 1.10)
    );

    let str = u.style_strength / 100.0;
    let s_idx = u.style % 12u;
    let s_tint = shadow_tints[s_idx];
    let h_tint = hl_tints[s_idx];

    let shadow_w = 1.0 - smoothstep(0.0, 0.55, luma);
    let hl_w = smoothstep(0.45, 1.0, luma);

    var styled_rgb = in_px.rgb * mix(vec3<f32>(1.0), s_tint, shadow_w * str);
    styled_rgb = styled_rgb * mix(vec3<f32>(1.0), h_tint, hl_w * str);

    // Brightness + Saturation
    styled_rgb += vec3<f32>((u.brightness / 100.0) * 0.20);
    let sat_mult = 1.0 + (u.saturation / 100.0) * 0.7;
    let cur_luma = rec2020_luma(styled_rgb);
    styled_rgb = mix(vec3<f32>(cur_luma), styled_rgb, sat_mult);

    // Center vignette
    let center_r = (u.center_size / 100.0) * 0.45 + 0.15;
    let aspect = f32(dim.x) / f32(dim.y);
    let d = length((uv - vec2<f32>(0.5, 0.5)) * vec2<f32>(aspect, 1.0));
    let vig_factor = 1.0 - (u.vignette_strength / 100.0) * smoothstep(center_r, center_r + 0.45, d) * 0.70;
    styled_rgb *= vig_factor;

    let final_rgb = mix(in_px.rgb, styled_rgb, u.opacity);
    textureStore(dst_tex, coord, vec4<f32>(final_rgb, in_px.a));
}
"#;

// ============================================================================
// 10. Retrolux
// ============================================================================

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RetroluxUniforms {
    pub brightness: f32,
    pub saturation: f32,
    pub contrast: f32,
    pub style_strength: f32,
    pub scratches: f32,
    pub light_leaks: f32,
    pub seed: u32,
    pub style: u32,
    pub opacity: f32,
    pub _pad0: f32,
    pub _pad1: f32,
    pub _pad2: f32,
}

impl RetroluxUniforms {
    pub fn from_params(params: &RetroluxParams, opacity: f32) -> Self {
        Self {
            brightness: params.brightness.clamp(-100.0, 100.0),
            saturation: params.saturation.clamp(-100.0, 100.0),
            contrast: params.contrast.clamp(-100.0, 100.0),
            style_strength: params.style_strength.clamp(0.0, 100.0),
            scratches: params.scratches.clamp(0.0, 100.0),
            light_leaks: params.light_leaks.clamp(0.0, 100.0),
            seed: params.seed,
            style: (params.style as u32).min(12),
            opacity: opacity.clamp(0.0, 1.0),
            _pad0: 0.0,
            _pad1: 0.0,
            _pad2: 0.0,
        }
    }
}

pub const RETROLUX_WGSL: &str = r#"
struct RetroluxUniforms {
    brightness: f32,
    saturation: f32,
    contrast: f32,
    style_strength: f32,
    scratches: f32,
    light_leaks: f32,
    seed: u32,
    style: u32,
    opacity: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
};

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var dst_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var<uniform> u: RetroluxUniforms;

fn hash11(p: f32) -> f32 {
    let p3 = fract(p * 0.1031);
    let p_rot = p3 * (p3 + 33.33);
    return fract((p3 + p_rot) * p_rot);
}

fn hash21(p: vec2<f32>) -> f32 {
    let p3 = fract(vec3<f32>(p.xyx) * 0.1031);
    let dot_val = dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * dot_val);
}

@compute @workgroup_size(16, 16, 1)
fn cs_retrolux(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dim = textureDimensions(src_tex);
    if (gid.x >= dim.x || gid.y >= dim.y) {
        return;
    }
    let coord = vec2<i32>(i32(gid.x), i32(gid.y));
    let in_px = textureLoad(src_tex, coord, 0);

    let uv = (vec2<f32>(coord) + 0.5) / vec2<f32>(dim);
    let str = u.style_strength / 100.0;
    let s_seed = f32(u.seed % 1000u);

    // Vintage emulsion tone curves
    var col = in_px.rgb;
    let luma = rec2020_luma(col);
    let fade = 0.04 * str;
    col = (col - vec3<f32>(0.5)) * (1.0 + (u.contrast / 100.0) * 0.45) + vec3<f32>(0.5) + vec3<f32>((u.brightness / 100.0) * 0.2);
    col = max(col, vec3<f32>(fade));

    let sat_mult = 1.0 + (u.saturation / 100.0) * 0.6;
    col = mix(vec3<f32>(rec2020_luma(col)), col, sat_mult);

    // Procedural scratches
    if (u.scratches > 0.0) {
        let scratch_str = u.scratches / 100.0;
        let x_cell = floor(uv.x * 220.0);
        let h_cell = hash11(x_cell + s_seed * 17.13);
        if (h_cell > 0.94) {
            let x_in_cell = fract(uv.x * 220.0);
            let d_line = abs(x_in_cell - 0.5);
            let scratch_alpha = smoothstep(0.12, 0.0, d_line) * scratch_str;
            let break_noise = hash21(vec2<f32>(floor(uv.y * 30.0), x_cell));
            if (break_noise > 0.25) {
                col += vec3<f32>(0.25 * scratch_alpha);
            }
        }
    }

    // Procedural warm/magenta light leaks
    if (u.light_leaks > 0.0) {
        let leak_str = u.light_leaks / 100.0;
        let corner_idx = u.seed % 4u;
        var leak_center = vec2<f32>(0.0, 0.0);
        if (corner_idx == 1u) {
            leak_center = vec2<f32>(1.0, 0.0);
        } else if (corner_idx == 2u) {
            leak_center = vec2<f32>(0.0, 1.0);
        } else if (corner_idx == 3u) {
            leak_center = vec2<f32>(1.0, 1.0);
        }

        let leak_d = length(uv - leak_center);
        let leak_mask = smoothstep(0.85, 0.0, leak_d) * leak_str;
        let leak_color = vec3<f32>(1.35, 0.45, 0.15) * 0.8;
        col += leak_color * leak_mask;
    }

    let final_rgb = mix(in_px.rgb, col, u.opacity);
    textureStore(dst_tex, coord, vec4<f32>(final_rgb, in_px.a));
}
"#;

// ============================================================================
// 11. Grunge
// ============================================================================

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GrungeUniforms {
    pub brightness: f32,
    pub contrast: f32,
    pub saturation: f32,
    pub texture_strength: f32,
    pub style: u32,
    pub seed: u32,
    pub opacity: f32,
    pub _pad0: f32,
}

impl GrungeUniforms {
    pub fn from_params(params: &GrungeParams, opacity: f32) -> Self {
        Self {
            brightness: params.brightness.clamp(-100.0, 100.0),
            contrast: params.contrast.clamp(-100.0, 100.0),
            saturation: params.saturation.clamp(-100.0, 100.0),
            texture_strength: params.texture_strength.clamp(0.0, 100.0),
            style: (params.style as u32).min(4),
            seed: params.seed,
            opacity: opacity.clamp(0.0, 1.0),
            _pad0: 0.0,
        }
    }
}

pub const GRUNGE_WGSL: &str = r#"
struct GrungeUniforms {
    brightness: f32,
    contrast: f32,
    saturation: f32,
    texture_strength: f32,
    style: u32,
    seed: u32,
    opacity: f32,
    _pad0: f32,
};

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var dst_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var<uniform> u: GrungeUniforms;

fn hash22(p: vec2<f32>) -> vec2<f32> {
    let p3 = fract(vec3<f32>(p.xyx) * vec3<f32>(0.1031, 0.1030, 0.0973));
    let d = dot(p3, p3.yzx + 33.33);
    return fract((p3.xx + p3.yz) * d);
}

fn value_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u_val = f * f * (3.0 - 2.0 * f);

    let a = hash22(i).x;
    let b = hash22(i + vec2<f32>(1.0, 0.0)).x;
    let c = hash22(i + vec2<f32>(0.0, 1.0)).x;
    let d = hash22(i + vec2<f32>(1.0, 1.0)).x;

    return mix(mix(a, b, u_val.x), mix(c, d, u_val.x), u_val.y);
}

fn fbm_grunge(p: vec2<f32>) -> f32 {
    var v = 0.0;
    var a = 0.5;
    var shift = vec2<f32>(100.0, 100.0);
    var pos = p;
    for (var i = 0; i < 4; i = i + 1) {
        v += a * value_noise(pos);
        pos = pos * 2.15 + shift;
        a *= 0.48;
    }
    return v;
}

@compute @workgroup_size(16, 16, 1)
fn cs_grunge(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dim = textureDimensions(src_tex);
    if (gid.x >= dim.x || gid.y >= dim.y) {
        return;
    }
    let coord = vec2<i32>(i32(gid.x), i32(gid.y));
    let in_px = textureLoad(src_tex, coord, 0);

    let uv = (vec2<f32>(coord) + 0.5) / vec2<f32>(dim);
    let s_seed = f32(u.seed % 1000u);

    let n = fbm_grunge(uv * 18.0 + vec2<f32>(s_seed * 3.71, s_seed * 7.13));
    let tex_str = u.texture_strength / 100.0;

    // 5 grunge style palettes
    var style_tint = vec3<f32>(0.95, 0.92, 0.88);
    if (u.style == 1u) {
        style_tint = vec3<f32>(1.12, 0.95, 0.82); // rust / sepia
    } else if (u.style == 2u) {
        style_tint = vec3<f32>(0.85, 0.88, 0.92); // charcoal street
    } else if (u.style == 3u) {
        style_tint = vec3<f32>(1.05, 1.10, 0.85); // mold vintage
    } else if (u.style == 4u) {
        style_tint = vec3<f32>(1.15, 1.15, 1.15); // bleached canvas
    }

    // Multiply / soft-light blend of grunge texture
    let grunge_factor = mix(1.0, n * 1.5, tex_str * 0.7);
    var col = in_px.rgb * grunge_factor * style_tint;

    col = (col - vec3<f32>(0.5)) * (1.0 + (u.contrast / 100.0) * 0.5) + vec3<f32>(0.5) + vec3<f32>((u.brightness / 100.0) * 0.2);
    let sat_mult = 1.0 + (u.saturation / 100.0) * 0.6;
    let cur_luma = rec2020_luma(col);
    col = mix(vec3<f32>(cur_luma), col, sat_mult);

    let final_rgb = mix(in_px.rgb, col, u.opacity);
    textureStore(dst_tex, coord, vec4<f32>(final_rgb, in_px.a));
}
"#;

// ============================================================================
// 12. Portrait
// ============================================================================

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PortraitUniforms {
    pub face_center: [f32; 2],
    pub face_radius: f32,
    pub face_spotlight: f32,
    pub skin_smoothing: f32,
    pub eye_clarity: f32,
    pub skin_tone: u32,
    pub opacity: f32,
    pub aspect: f32,
    pub _pad0: f32,
    pub _pad1: f32,
    pub _pad2: f32,
}

impl PortraitUniforms {
    pub fn from_params(params: &PortraitParams, width: u32, height: u32, opacity: f32) -> Self {
        let skin_tone = match params.skin_tone {
            PortraitSkinTone::None => 0,
            PortraitSkinTone::Pale => 1,
            PortraitSkinTone::Fair => 2,
            PortraitSkinTone::Medium => 3,
            PortraitSkinTone::Tan => 4,
            PortraitSkinTone::Deep => 5,
        };
        let aspect = width as f32 / (height as f32).max(1.0);
        Self {
            face_center: params.face_center,
            face_radius: params.face_radius.clamp(0.05, 0.8),
            face_spotlight: params.face_spotlight.clamp(0.0, 100.0),
            skin_smoothing: params.skin_smoothing.clamp(0.0, 100.0),
            eye_clarity: params.eye_clarity.clamp(0.0, 100.0),
            skin_tone,
            opacity: opacity.clamp(0.0, 1.0),
            aspect,
            _pad0: 0.0,
            _pad1: 0.0,
            _pad2: 0.0,
        }
    }
}

pub const PORTRAIT_WGSL: &str = r#"
struct PortraitUniforms {
    face_center: vec2<f32>,
    face_radius: f32,
    face_spotlight: f32,
    skin_smoothing: f32,
    eye_clarity: f32,
    skin_tone: u32,
    opacity: f32,
    aspect: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
};

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var dst_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var<uniform> u: PortraitUniforms;

@compute @workgroup_size(16, 16, 1)
fn cs_portrait(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dim = textureDimensions(src_tex);
    if (gid.x >= dim.x || gid.y >= dim.y) {
        return;
    }
    let coord = vec2<i32>(i32(gid.x), i32(gid.y));
    let in_px = textureLoad(src_tex, coord, 0);

    let uv = (vec2<f32>(coord) + 0.5) / vec2<f32>(dim);
    let d_face = length((uv - u.face_center) * vec2<f32>(u.aspect, 1.25)) / u.face_radius;
    let face_w = 1.0 - smoothstep(0.4, 1.2, d_face);

    // Oklab skin-hue selectivity
    let lab = linear_rec2020_to_oklab(in_px.rgb);
    let chroma = sqrt(lab.y * lab.y + lab.z * lab.z);
    let hue = atan2(lab.z, lab.y);
    let is_skin_hue = smoothstep(0.40, 0.70, hue) * (1.0 - smoothstep(1.20, 1.50, hue));
    let is_skin_chroma = smoothstep(0.03, 0.07, chroma) * (1.0 - smoothstep(0.22, 0.32, chroma));
    let skin_mask = face_w * is_skin_hue * is_skin_chroma;

    // 9-tap bilateral filter for skin smoothing
    var smoothed_rgb = in_px.rgb;
    if (u.skin_smoothing > 0.0 && skin_mask > 0.02) {
        var sum_px = vec3<f32>(0.0);
        var sum_w = 0.0;
        let c_luma = rec2020_luma(in_px.rgb);

        for (var dy = -1; dy <= 1; dy = dy + 1) {
            for (var dx = -1; dx <= 1; dx = dx + 1) {
                let tap_c = vec2<i32>(clamp(coord.x + dx * 2, 0, i32(dim.x) - 1), clamp(coord.y + dy * 2, 0, i32(dim.y) - 1));
                let tap_px = textureLoad(src_tex, tap_c, 0);
                let tap_luma = rec2020_luma(tap_px.rgb);
                let w = exp(-abs(tap_luma - c_luma) * 8.0);
                sum_px += tap_px.rgb * w;
                sum_w += w;
            }
        }
        let bilateral_px = sum_px / max(sum_w, 1e-4);
        let smooth_k = (u.skin_smoothing / 100.0) * skin_mask;
        smoothed_rgb = mix(in_px.rgb, bilateral_px, smooth_k);
    }

    // Eye clarity micro-contrast in upper face
    if (u.eye_clarity > 0.0 && uv.y < u.face_center.y + 0.10 * u.face_radius && face_w > 0.2) {
        let eye_w = face_w * (u.eye_clarity / 100.0) * (1.0 - skin_mask);
        let px_l = textureLoad(src_tex, vec2<i32>(max(0, coord.x - 2), coord.y), 0);
        let px_r = textureLoad(src_tex, vec2<i32>(min(i32(dim.x) - 1, coord.x + 2), coord.y), 0);
        let lap = in_px.rgb * 2.0 - (px_l.rgb + px_r.rgb);
        smoothed_rgb += lap * (eye_w * 0.65);
    }

    // Skin tone refinement
    if (u.skin_tone > 0u && skin_mask > 0.05) {
        var tone_lab = linear_rec2020_to_oklab(smoothed_rgb);
        if (u.skin_tone == 1u) { // Pale
            tone_lab.x += 0.03 * skin_mask;
        } else if (u.skin_tone == 2u) { // Fair
            tone_lab.y += 0.015 * skin_mask;
            tone_lab.z += 0.010 * skin_mask;
        } else if (u.skin_tone == 3u) { // Medium
            tone_lab.z += 0.025 * skin_mask;
        } else if (u.skin_tone == 4u) { // Tan
            tone_lab.x -= 0.02 * skin_mask;
            tone_lab.y += 0.02 * skin_mask;
            tone_lab.z += 0.03 * skin_mask;
        } else if (u.skin_tone == 5u) { // Deep
            tone_lab.x -= 0.05 * skin_mask;
            tone_lab.y += 0.01 * skin_mask;
            tone_lab.z += 0.02 * skin_mask;
        }
        smoothed_rgb = oklab_to_linear_rec2020(tone_lab);
    }

    // Face spotlight
    if (u.face_spotlight > 0.0) {
        let spot_k = (u.face_spotlight / 100.0);
        let inner_boost = 1.0 + spot_k * face_w * 0.25;
        let outer_darken = 1.0 - spot_k * (1.0 - face_w) * 0.20;
        smoothed_rgb *= (inner_boost * outer_darken);
    }

    let final_rgb = mix(in_px.rgb, smoothed_rgb, u.opacity);
    textureStore(dst_tex, coord, vec4<f32>(final_rgb, in_px.a));
}
"#;

// ============================================================================
// 13. Head Pose
// ============================================================================

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct HeadPoseUniforms {
    pub face_center: [f32; 2],
    pub yaw_x: f32,
    pub pitch_y: f32,
    pub smile: f32,
    pub focal_length: f32,
    pub opacity: f32,
    pub aspect: f32,
}

impl HeadPoseUniforms {
    pub fn from_params(params: &HeadPoseParams, width: u32, height: u32, opacity: f32) -> Self {
        let aspect = width as f32 / (height as f32).max(1.0);
        Self {
            face_center: params.face_center,
            yaw_x: params.yaw_x.clamp(-100.0, 100.0),
            pitch_y: params.pitch_y.clamp(-100.0, 100.0),
            smile: params.smile.clamp(-100.0, 100.0),
            focal_length: params.focal_length.clamp(-100.0, 100.0),
            opacity: opacity.clamp(0.0, 1.0),
            aspect,
        }
    }
}

pub const HEAD_POSE_WGSL: &str = r#"
struct HeadPoseUniforms {
    face_center: vec2<f32>,
    yaw_x: f32,
    pitch_y: f32,
    smile: f32,
    focal_length: f32,
    opacity: f32,
    aspect: f32,
};

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var dst_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var<uniform> u: HeadPoseUniforms;

fn sample_src_bilinear(coord_f: vec2<f32>, dim: vec2<i32>) -> vec3<f32> {
    let base = floor(coord_f);
    let f = coord_f - base;
    let x0 = clamp(i32(base.x), 0, dim.x - 1);
    let x1 = clamp(i32(base.x) + 1, 0, dim.x - 1);
    let y0 = clamp(i32(base.y), 0, dim.y - 1);
    let y1 = clamp(i32(base.y) + 1, 0, dim.y - 1);

    let p00 = textureLoad(src_tex, vec2<i32>(x0, y0), 0).rgb;
    let p10 = textureLoad(src_tex, vec2<i32>(x1, y0), 0).rgb;
    let p01 = textureLoad(src_tex, vec2<i32>(x0, y1), 0).rgb;
    let p11 = textureLoad(src_tex, vec2<i32>(x1, y1), 0).rgb;

    let top = mix(p00, p10, f.x);
    let bot = mix(p01, p11, f.x);
    return mix(top, bot, f.y);
}

@compute @workgroup_size(16, 16, 1)
fn cs_head_pose(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dim = textureDimensions(src_tex);
    if (gid.x >= dim.x || gid.y >= dim.y) {
        return;
    }
    let coord = vec2<i32>(i32(gid.x), i32(gid.y));
    let in_px = textureLoad(src_tex, coord, 0);

    let uv = (vec2<f32>(coord) + 0.5) / vec2<f32>(dim);
    let d_center = (uv - u.face_center) * vec2<f32>(u.aspect, 1.0);
    let dist = length(d_center);

    let r_max = 0.35;
    var w = smoothstep(r_max, 0.0, dist);
    w = w * w;

    // Yaw perspective displacement
    let delta_yaw = vec2<f32>((u.yaw_x / 100.0) * 0.045 * w * (1.0 - dist / r_max) / u.aspect, 0.0);

    // Pitch perspective displacement
    let delta_pitch = vec2<f32>(0.0, (u.pitch_y / 100.0) * 0.045 * w * (1.0 - dist / r_max));

    // Smile upward curvature below face center
    var delta_smile = vec2<f32>(0.0, 0.0);
    if (d_center.y > 0.04 && d_center.y < 0.22 && abs(d_center.x) < 0.16) {
        let mouth_y = smoothstep(0.04, 0.11, d_center.y) * smoothstep(0.22, 0.11, d_center.y);
        let mouth_x = smoothstep(0.16, 0.0, abs(d_center.x));
        let smile_curve = -(d_center.x * d_center.x) * 18.0 + 0.02;
        delta_smile = vec2<f32>(0.0, (u.smile / 100.0) * smile_curve * mouth_y * mouth_x);
    }

    // Focal length barrel / pincushion
    let delta_focal = (uv - u.face_center) * (u.focal_length / 100.0) * 0.07 * w;

    let total_disp = (delta_yaw + delta_pitch + delta_smile + delta_focal) * u.opacity;
    let src_uv = clamp(uv - total_disp, vec2<f32>(0.0), vec2<f32>(1.0));
    let warped_rgb = sample_src_bilinear(src_uv * vec2<f32>(dim), vec2<i32>(dim));

    let final_rgb = mix(in_px.rgb, warped_rgb, u.opacity);
    textureStore(dst_tex, coord, vec4<f32>(final_rgb, in_px.a));
}
"#;

// ============================================================================
// 14. Frames
// ============================================================================

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct FramesUniforms {
    pub width: f32,
    pub aspect_pad: f32,
    pub style: u32,
    pub inner_line: u32,
    pub opacity: f32,
    pub aspect: f32,
    pub _pad0: f32,
    pub _pad1: f32,
}

impl FramesUniforms {
    pub fn from_params(params: &FramesParams, width: u32, height: u32, opacity: f32) -> Self {
        let style = match params.style {
            FrameStyle::CleanWhite => 0,
            FrameStyle::MatteBlack => 1,
            FrameStyle::Polaroid => 2,
            FrameStyle::Film35mm => 3,
            FrameStyle::InstantDate => 4,
            FrameStyle::GalleryLine => 5,
            FrameStyle::RoughEdge => 6,
            FrameStyle::Letterbox => 7,
        };
        let aspect = width as f32 / (height as f32).max(1.0);
        Self {
            width: params.width.clamp(0.0, 100.0),
            aspect_pad: params.aspect_pad.clamp(0.0, 100.0),
            style,
            inner_line: if params.inner_line { 1 } else { 0 },
            opacity: opacity.clamp(0.0, 1.0),
            aspect,
            _pad0: 0.0,
            _pad1: 0.0,
        }
    }
}

pub const FRAMES_WGSL: &str = r#"
struct FramesUniforms {
    width: f32,
    aspect_pad: f32,
    style: u32,
    inner_line: u32,
    opacity: f32,
    aspect: f32,
    _pad0: f32,
    _pad1: f32,
};

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var dst_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var<uniform> u: FramesUniforms;

@compute @workgroup_size(16, 16, 1)
fn cs_frames(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dim = textureDimensions(src_tex);
    if (gid.x >= dim.x || gid.y >= dim.y) {
        return;
    }
    let coord = vec2<i32>(i32(gid.x), i32(gid.y));
    let in_px = textureLoad(src_tex, coord, 0);

    let uv = (vec2<f32>(coord) + 0.5) / vec2<f32>(dim);
    let border_base = (u.width / 100.0) * 0.12 + 0.015;

    var border_t = border_base;
    var border_b = border_base;
    var border_l = border_base;
    var border_r = border_base;

    var frame_color = vec3<f32>(1.0);

    if (u.style == 1u) { // MatteBlack
        frame_color = vec3<f32>(0.02);
    } else if (u.style == 2u) { // Polaroid
        frame_color = vec3<f32>(0.96, 0.95, 0.93);
        border_b = border_base * 2.8;
    } else if (u.style == 3u) { // Film35mm
        frame_color = vec3<f32>(0.02);
        border_t = border_base * 1.6;
        border_b = border_base * 1.6;
    } else if (u.style == 4u) { // InstantDate
        frame_color = vec3<f32>(0.08, 0.07, 0.06);
        border_b = border_base * 1.8;
    } else if (u.style == 5u) { // GalleryLine
        frame_color = vec3<f32>(0.98, 0.97, 0.95);
        border_t = border_base * 1.5;
        border_b = border_base * 1.5;
        border_l = border_base * 1.5;
        border_r = border_base * 1.5;
    } else if (u.style == 7u) { // Letterbox (2.39:1)
        frame_color = vec3<f32>(0.0);
        let target_h = 1.0 / (2.39 * u.aspect);
        if (target_h < 1.0) {
            let bar_h = (1.0 - target_h) * 0.5;
            border_t = bar_h;
            border_b = bar_h;
            border_l = 0.0;
            border_r = 0.0;
        }
    }

    let is_frame = (uv.y < border_t) || (uv.y > 1.0 - border_b) || (uv.x < border_l) || (uv.x > 1.0 - border_r);

    var out_rgb = in_px.rgb;
    if (is_frame) {
        out_rgb = frame_color;

        // 35mm film sprocket holes
        if (u.style == 3u) {
            let in_top_bar = uv.y < border_t;
            let in_bot_bar = uv.y > 1.0 - border_b;
            if (in_top_bar || in_bot_bar) {
                let sprocket_step = 0.08;
                let sx = abs(fract(uv.x / sprocket_step) - 0.5) * sprocket_step;
                let sy = select(abs(uv.y - border_t * 0.5), abs((1.0 - uv.y) - border_b * 0.5), in_bot_bar);
                if (sx < 0.022 && sy < 0.015) {
                    out_rgb = vec3<f32>(0.92); // sprocket hole
                }
            }
        }

        // Instant date amber corner stamp
        if (u.style == 4u && uv.y > 1.0 - border_b * 0.75 && uv.x > 0.70 && uv.x < 0.94) {
            out_rgb = vec3<f32>(0.95, 0.55, 0.15); // amber date ticks
        }
    } else {
        // Inner hairline border
        if (u.inner_line != 0u || u.style == 5u) {
            let line_margin = 0.015;
            let dt = abs(uv.y - border_t - line_margin);
            let db = abs(uv.y - (1.0 - border_b - line_margin));
            let dl = abs(uv.x - border_l - line_margin);
            let dr = abs(uv.x - (1.0 - border_r - line_margin));
            let is_on_line = min(min(dt, db), min(dl, dr)) < 0.0015;
            if (is_on_line) {
                out_rgb = vec3<f32>(0.25);
            }
        }
    }

    let final_rgb = mix(in_px.rgb, out_rgb, u.opacity);
    textureStore(dst_tex, coord, vec4<f32>(final_rgb, in_px.a));
}
"#;

// ============================================================================
// 15. Double Exposure (5-Binding curves_bgl)
// ============================================================================

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DoubleExposureUniforms {
    pub blend_mode: u32,
    pub blend_opacity: f32,
    pub node_opacity: f32,
    pub has_secondary: u32,
}

impl DoubleExposureUniforms {
    pub fn from_params(params: &DoubleExposureParams, has_secondary: bool, node_opacity: f32) -> Self {
        let blend_mode = match params.blend_mode {
            DoubleExposureBlend::Default => 0,
            DoubleExposureBlend::Lighten => 1,
            DoubleExposureBlend::Darken => 2,
            DoubleExposureBlend::Add => 3,
            DoubleExposureBlend::Subtract => 4,
            DoubleExposureBlend::Blend => 5,
        };
        Self {
            blend_mode,
            blend_opacity: (params.opacity / 100.0).clamp(0.0, 1.0),
            node_opacity: node_opacity.clamp(0.0, 1.0),
            has_secondary: if has_secondary { 1 } else { 0 },
        }
    }
}

pub const DOUBLE_EXPOSURE_WGSL: &str = r#"
struct DoubleExposureUniforms {
    blend_mode: u32,
    blend_opacity: f32,
    node_opacity: f32,
    has_secondary: u32,
};

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var dst_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var sec_tex: texture_2d<f32>;
@group(0) @binding(3) var sec_sampler: sampler;
@group(0) @binding(4) var<uniform> u: DoubleExposureUniforms;

@compute @workgroup_size(16, 16, 1)
fn cs_double_exposure(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dim = textureDimensions(src_tex);
    if (gid.x >= dim.x || gid.y >= dim.y) {
        return;
    }
    let coord = vec2<i32>(i32(gid.x), i32(gid.y));
    let in_px = textureLoad(src_tex, coord, 0);

    if (u.has_secondary == 0u || u.blend_opacity <= 0.0 || u.node_opacity <= 0.0) {
        textureStore(dst_tex, coord, in_px);
        return;
    }

    let uv = (vec2<f32>(coord) + 0.5) / vec2<f32>(dim);

    // Aspect-preserving cover UV mapping for secondary texture
    let src_dims = vec2<f32>(dim);
    let sec_dims = vec2<f32>(textureDimensions(sec_tex));
    let src_aspect = src_dims.x / src_dims.y;
    let sec_aspect = sec_dims.x / sec_dims.y;

    var cover_uv = uv;
    if (src_aspect > sec_aspect) {
        let sc = sec_aspect / src_aspect;
        cover_uv = vec2<f32>(uv.x, (uv.y - 0.5) * sc + 0.5);
    } else {
        let sc = src_aspect / sec_aspect;
        cover_uv = vec2<f32>((uv.x - 0.5) * sc + 0.5, uv.y);
    }

    let sec_px = textureSampleLevel(sec_tex, sec_sampler, clamp(cover_uv, vec2<f32>(0.0), vec2<f32>(1.0)), 0.0);
    let a = in_px.rgb;
    let b = sec_px.rgb;

    var blended = a;
    if (u.blend_mode == 0u) { // Default (Balanced photographic screen)
        blended = a + b - a * b * 0.75;
    } else if (u.blend_mode == 1u) { // Lighten
        blended = max(a, b);
    } else if (u.blend_mode == 2u) { // Darken
        blended = min(a, b);
    } else if (u.blend_mode == 3u) { // Add
        blended = a + b;
    } else if (u.blend_mode == 4u) { // Subtract
        blended = max(a - b, vec3<f32>(0.0));
    } else { // Blend
        blended = mix(a, b, 0.5);
    }

    let mixed = mix(a, blended, u.blend_opacity);
    let final_rgb = mix(a, mixed, u.node_opacity);
    textureStore(dst_tex, coord, vec4<f32>(final_rgb, in_px.a));
}
"#;

// ============================================================================
// 16. Text Overlay (5-Binding curves_bgl)
// ============================================================================

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TextOverlayUniforms {
    pub fg_rgb: [f32; 4],
    pub bg_rgb: [f32; 4],
    pub text_opacity: f32,
    pub node_opacity: f32,
    pub invert_mask: u32,
    pub _pad: u32,
}

impl TextOverlayUniforms {
    pub fn from_params(params: &TextOverlayParams, node_opacity: f32) -> Self {
        let fg_rec2020 = match params.color_preset {
            TextColorPreset::White => [1.0, 1.0, 1.0],
            TextColorPreset::Black => [0.002, 0.002, 0.002],
            TextColorPreset::Cream => srgb_to_rec2020_lin(0.96, 0.93, 0.84),
            TextColorPreset::Gold => srgb_to_rec2020_lin(0.96, 0.77, 0.24),
            TextColorPreset::Coral => srgb_to_rec2020_lin(0.95, 0.45, 0.38),
            TextColorPreset::Cyan => srgb_to_rec2020_lin(0.22, 0.82, 0.90),
            TextColorPreset::Sage => srgb_to_rec2020_lin(0.62, 0.78, 0.65),
            TextColorPreset::Slate => srgb_to_rec2020_lin(0.45, 0.52, 0.60),
        };

        let bg_rec2020 = match params.color_preset {
            TextColorPreset::Black => [0.90, 0.90, 0.90, 0.85],
            _ => [0.02, 0.02, 0.02, 0.70],
        };

        Self {
            fg_rgb: [fg_rec2020[0], fg_rec2020[1], fg_rec2020[2], 1.0],
            bg_rgb: bg_rec2020,
            text_opacity: (params.opacity / 100.0).clamp(0.0, 1.0),
            node_opacity: node_opacity.clamp(0.0, 1.0),
            invert_mask: if params.invert_mask { 1 } else { 0 },
            _pad: 0,
        }
    }
}

pub const TEXT_OVERLAY_WGSL: &str = r#"
struct TextOverlayUniforms {
    fg_rgb: vec4<f32>,
    bg_rgb: vec4<f32>,
    text_opacity: f32,
    node_opacity: f32,
    invert_mask: u32,
    _pad: u32,
};

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var dst_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var glyph_tex: texture_2d<f32>;
@group(0) @binding(3) var glyph_sampler: sampler;
@group(0) @binding(4) var<uniform> u: TextOverlayUniforms;

@compute @workgroup_size(16, 16, 1)
fn cs_text_overlay(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dim = textureDimensions(src_tex);
    if (gid.x >= dim.x || gid.y >= dim.y) {
        return;
    }
    let coord = vec2<i32>(i32(gid.x), i32(gid.y));
    let in_px = textureLoad(src_tex, coord, 0);

    let uv = (vec2<f32>(coord) + 0.5) / vec2<f32>(dim);
    let glyph_data = textureSampleLevel(glyph_tex, glyph_sampler, uv, 0.0);

    let fg_alpha = glyph_data.r * u.text_opacity;
    let bg_alpha = glyph_data.g * u.bg_rgb.a * u.text_opacity;

    var out_rgb = in_px.rgb;

    if (u.invert_mask == 0u) {
        // Normal mode: composite background badge plate then foreground text glyphs
        let after_bg = mix(in_px.rgb, u.bg_rgb.rgb, bg_alpha);
        out_rgb = mix(after_bg, u.fg_rgb.rgb, fg_alpha);
    } else {
        // Knockout mode: photo shines through text glyphs while background plate/canvas is tinted/dimmed
        let dimmed_photo = in_px.rgb * 0.22 + u.bg_rgb.rgb * 0.18;
        let plate_rgb = mix(dimmed_photo, u.bg_rgb.rgb, bg_alpha);
        out_rgb = mix(plate_rgb, in_px.rgb, fg_alpha);
    }

    let final_rgb = mix(in_px.rgb, out_rgb, u.node_opacity);
    textureStore(dst_tex, coord, vec4<f32>(final_rgb, in_px.a));
}
"#;
