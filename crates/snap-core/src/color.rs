use std::f32::consts::PI;

/// Exact D65 ITU-R BT.709 (linear sRGB) to ITU-R BT.2020 3x3 matrix.
pub const SRGB_TO_REC2020: [[f32; 3]; 3] = [
    [0.627404078626, 0.329282098415, 0.043313797307],
    [0.069097233161, 0.919541035593, 0.011361186736],
    [0.016391438883, 0.088013307577, 0.895595253540],
];

/// Exact D65 ITU-R BT.2020 to ITU-R BT.709 (linear sRGB) 3x3 matrix.
pub const REC2020_TO_SRGB: [[f32; 3]; 3] = [
    [1.660490995932, -0.587641138833, -0.072849857099],
    [-0.124550474515, 1.132899897135, -0.008349422620],
    [-0.018150763435, -0.100578897042, 1.118729660477],
];

#[inline]
pub fn mul_mat3_vec3(m: &[[f32; 3]; 3], v: [f32; 3]) -> [f32; 3] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

/// Convert standard sRGB non-linear channel to linear luminance.
#[inline]
pub fn srgb_to_linear(c: f32) -> f32 {
    if c < 0.0 {
        -srgb_to_linear(-c)
    } else if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Convert linear luminance to standard sRGB non-linear channel.
#[inline]
pub fn linear_to_srgb(c: f32) -> f32 {
    if c < 0.0 {
        -linear_to_srgb(-c)
    } else if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

/// Convert Linear sRGB to Linear Rec.2020.
#[inline]
pub fn linear_srgb_to_rec2020(rgb: [f32; 3]) -> [f32; 3] {
    mul_mat3_vec3(&SRGB_TO_REC2020, rgb)
}

/// Convert Linear Rec.2020 to Linear sRGB.
#[inline]
pub fn linear_rec2020_to_srgb(rgb: [f32; 3]) -> [f32; 3] {
    mul_mat3_vec3(&REC2020_TO_SRGB, rgb)
}

/// Convert Linear sRGB to Oklab [L, a, b].
#[inline]
pub fn linear_srgb_to_oklab(rgb: [f32; 3]) -> [f32; 3] {
    let l = 0.4122214708 * rgb[0] + 0.5363325363 * rgb[1] + 0.0514459929 * rgb[2];
    let m = 0.2119034982 * rgb[0] + 0.6806995451 * rgb[1] + 0.1073969566 * rgb[2];
    let s = 0.0883024619 * rgb[0] + 0.2817188376 * rgb[1] + 0.6299787005 * rgb[2];

    let l_ = signed_cbrt(l);
    let m_ = signed_cbrt(m);
    let s_ = signed_cbrt(s);

    let l_out = 0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_;
    let a_out = 1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_;
    let b_out = 0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_;

    [l_out, a_out, b_out]
}

/// Convert Oklab [L, a, b] to Linear sRGB.
#[inline]
pub fn oklab_to_linear_srgb(lab: [f32; 3]) -> [f32; 3] {
    let l_ = lab[0] + 0.3963377774 * lab[1] + 0.2158037573 * lab[2];
    let m_ = lab[0] - 0.1055613458 * lab[1] - 0.0638541728 * lab[2];
    let s_ = lab[0] - 0.0894841775 * lab[1] - 1.2914855480 * lab[2];

    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;

    let r = 4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s;
    let g = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s;
    let b = -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s;

    [r, g, b]
}

/// Convert Linear Rec.2020 to Oklab [L, a, b].
#[inline]
pub fn linear_rec2020_to_oklab(rgb: [f32; 3]) -> [f32; 3] {
    let srgb = linear_rec2020_to_srgb(rgb);
    linear_srgb_to_oklab(srgb)
}

/// Convert Oklab [L, a, b] to Linear Rec.2020.
#[inline]
pub fn oklab_to_linear_rec2020(lab: [f32; 3]) -> [f32; 3] {
    let srgb = oklab_to_linear_srgb(lab);
    linear_srgb_to_rec2020(srgb)
}

/// Convert Oklab [L, a, b] to Oklch [L, C, h_radians].
#[inline]
pub fn oklab_to_oklch(lab: [f32; 3]) -> [f32; 3] {
    let l = lab[0];
    let c = (lab[1] * lab[1] + lab[2] * lab[2]).sqrt();
    let mut h = lab[2].atan2(lab[1]);
    if h < 0.0 {
        h += 2.0 * PI;
    }
    [l, c, h]
}

/// Convert Oklch [L, C, h_radians] to Oklab [L, a, b].
#[inline]
pub fn oklch_to_oklab(lch: [f32; 3]) -> [f32; 3] {
    let l = lch[0];
    let c = lch[1];
    let h = lch[2];
    let a = c * h.cos();
    let b = c * h.sin();
    [l, a, b]
}

/// ITU-R BT.2020 standard luma calculation.
#[inline]
pub fn rec2020_luma(rgb: [f32; 3]) -> f32 {
    0.2627 * rgb[0] + 0.6780 * rgb[1] + 0.0593 * rgb[2]
}

#[inline]
fn signed_cbrt(x: f32) -> f32 {
    if x < 0.0 {
        -(-x).cbrt()
    } else {
        x.cbrt()
    }
}
