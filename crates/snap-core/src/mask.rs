use serde::{Deserialize, Serialize};

use crate::color::{linear_rec2020_to_oklab, oklab_to_oklch};
use crate::image_types::LinearImage;

pub const MASK_DIM: usize = 256;
pub const MASK_LEN: usize = MASK_DIM * MASK_DIM; // 65,536

// ----------------------------------------------------------------------------
// Helpers for RLE serialization
// ----------------------------------------------------------------------------

fn hex_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn encode_rle_bytes(prefix: &str, data: &[u8]) -> String {
    let mut out_bytes = Vec::new();
    if data.is_empty() {
        return prefix.to_string();
    }
    let mut i = 0;
    while i < data.len() {
        let val = data[i];
        let mut count = 1usize;
        while i + count < data.len() && data[i + count] == val && count < 65535 {
            count += 1;
        }
        out_bytes.push((count >> 8) as u8);
        out_bytes.push((count & 0xff) as u8);
        out_bytes.push(val);
        i += count;
    }

    let mut s = String::with_capacity(prefix.len() + out_bytes.len() * 2);
    s.push_str(prefix);
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for &b in &out_bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0x0f) as usize] as char);
    }
    s
}

fn decode_rle_bytes(prefix: &str, s: &str, expected_len: usize) -> Result<Vec<u8>, String> {
    let hex_str = s
        .strip_prefix(prefix)
        .ok_or_else(|| format!("missing {prefix} prefix"))?;
    if hex_str.len() % 6 != 0 {
        return Err(format!("invalid {prefix} hex length (not multiple of 6)"));
    }
    let mut out = Vec::with_capacity(expected_len);
    let bytes = hex_str.as_bytes();
    for chunk in bytes.chunks_exact(6) {
        let h0 = hex_nibble(chunk[0]).ok_or("invalid hex char")?;
        let h1 = hex_nibble(chunk[1]).ok_or("invalid hex char")?;
        let h2 = hex_nibble(chunk[2]).ok_or("invalid hex char")?;
        let h3 = hex_nibble(chunk[3]).ok_or("invalid hex char")?;
        let h4 = hex_nibble(chunk[4]).ok_or("invalid hex char")?;
        let h5 = hex_nibble(chunk[5]).ok_or("invalid hex char")?;

        let count = (((h0 as usize) << 12)
            | ((h1 as usize) << 8)
            | ((h2 as usize) << 4)
            | (h3 as usize))
            .max(1);
        let val = (h4 << 4) | h5;
        for _ in 0..count {
            out.push(val);
        }
    }
    if out.len() != expected_len {
        return Err(format!("expected {expected_len} bytes, got {}", out.len()));
    }
    Ok(out)
}

fn fnv1a_hash(bytes: &[u8]) -> u64 {
    const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let mut hash = FNV_OFFSET_BASIS;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

#[inline]
fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

// ----------------------------------------------------------------------------
// LayerMask
// ----------------------------------------------------------------------------

/// A 256x256 8-bit opacity mask (0 = 0% masked out, 255 = 100% applied).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayerMask {
    data: Vec<u8>,
}

impl Default for LayerMask {
    fn default() -> Self {
        Self::full()
    }
}

impl LayerMask {
    /// Construct a full mask (all pixels = 255).
    pub fn full() -> Self {
        Self {
            data: vec![255; MASK_LEN],
        }
    }

    /// Construct an empty mask (all pixels = 0).
    pub fn empty() -> Self {
        Self {
            data: vec![0; MASK_LEN],
        }
    }

    /// Construct from raw 65,536 bytes.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, &'static str> {
        if bytes.len() != MASK_LEN {
            return Err("LayerMask requires exactly 65,536 bytes");
        }
        Ok(Self { data: bytes })
    }

    /// Fill all cells with the specified value.
    pub fn fill(&mut self, value: u8) {
        self.data.fill(value);
    }

    /// Clear mask to zero (all 0, reveal nothing).
    pub fn clear(&mut self) {
        self.fill(0);
    }

    /// Reset mask to full (all 255, reveal all).
    pub fn reset_full(&mut self) {
        self.fill(255);
    }

    /// Invert mask values (`v = 255 - v`).
    pub fn invert(&mut self) {
        for v in &mut self.data {
            *v = 255 - *v;
        }
    }

    /// Check if every cell is 255.
    pub fn is_uniform_full(&self) -> bool {
        self.data.iter().all(|&v| v == 255)
    }

    /// Check if every cell is 0.
    pub fn is_uniform_zero(&self) -> bool {
        self.data.iter().all(|&v| v == 0)
    }

    /// Read raw 65,536 bytes slice.
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Deterministic 64-bit FNV-1a content hash over the 65,536 bytes for GPU texture caching.
    pub fn content_hash(&self) -> u64 {
        fnv1a_hash(&self.data)
    }

    /// Sample mask with bilinear interpolation in normalized coordinates `[0.0, 1.0]`.
    pub fn sample_bilinear(&self, u: f32, v: f32) -> f32 {
        let u_clamped = u.clamp(0.0, 1.0);
        let v_clamped = v.clamp(0.0, 1.0);

        let fx_coord = u_clamped * ((MASK_DIM - 1) as f32);
        let fy_coord = v_clamped * ((MASK_DIM - 1) as f32);

        let x0 = (fx_coord.floor() as usize).min(MASK_DIM - 1);
        let x1 = (x0 + 1).min(MASK_DIM - 1);
        let y0 = (fy_coord.floor() as usize).min(MASK_DIM - 1);
        let y1 = (y0 + 1).min(MASK_DIM - 1);

        let fx = fx_coord - x0 as f32;
        let fy = fy_coord - y0 as f32;

        let p00 = self.data[y0 * MASK_DIM + x0] as f32 / 255.0;
        let p10 = self.data[y0 * MASK_DIM + x1] as f32 / 255.0;
        let p01 = self.data[y1 * MASK_DIM + x0] as f32 / 255.0;
        let p11 = self.data[y1 * MASK_DIM + x1] as f32 / 255.0;

        let top = p00 * (1.0 - fx) + p10 * fx;
        let bottom = p01 * (1.0 - fx) + p11 * fx;
        (top * (1.0 - fy) + bottom * fy).clamp(0.0, 1.0)
    }

    /// Paint a circular feathered dab at normalized `(nx, ny)`.
    pub fn paint_dab(
        &mut self,
        nx: f32,
        ny: f32,
        radius_norm: f32,
        hardness: f32,
        target_opacity: f32,
    ) {
        let cx = nx.clamp(0.0, 1.0) * ((MASK_DIM - 1) as f32);
        let cy = ny.clamp(0.0, 1.0) * ((MASK_DIM - 1) as f32);
        let r_px = (radius_norm * (MASK_DIM as f32)).max(1.0);

        let min_x = ((cx - r_px).floor() as isize).clamp(0, (MASK_DIM - 1) as isize) as usize;
        let max_x = ((cx + r_px).ceil() as isize).clamp(0, (MASK_DIM - 1) as isize) as usize;
        let min_y = ((cy - r_px).floor() as isize).clamp(0, (MASK_DIM - 1) as isize) as usize;
        let max_y = ((cy + r_px).ceil() as isize).clamp(0, (MASK_DIM - 1) as isize) as usize;

        let h = hardness.clamp(0.0, 1.0);
        let inner_ratio = h.clamp(0.0, 0.999);
        let target_val = (target_opacity.clamp(0.0, 1.0) * 255.0).round();

        for y in min_y..=max_y {
            let dy = (y as f32) - cy;
            let dy2 = dy * dy;
            let row_offset = y * MASK_DIM;
            for x in min_x..=max_x {
                let dx = (x as f32) - cx;
                let d = (dx * dx + dy2).sqrt();
                let dist_norm = d / r_px;
                if dist_norm >= 1.0 {
                    continue;
                }

                let weight = if dist_norm <= inner_ratio {
                    1.0
                } else {
                    let t = (dist_norm - inner_ratio) / (1.0 - inner_ratio);
                    1.0 - (t * t * (3.0 - 2.0 * t))
                };

                let idx = row_offset + x;
                let cur = self.data[idx] as f32;
                let new_val = cur + (target_val - cur) * weight;
                self.data[idx] = new_val.round().clamp(0.0, 255.0) as u8;
            }
        }
    }

    /// Paint a continuous stroke from `(x0, y0)` to `(x1, y1)`.
    pub fn paint_stroke(
        &mut self,
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
        radius_norm: f32,
        hardness: f32,
        target_opacity: f32,
    ) {
        let dx = x1 - x0;
        let dy = y1 - y0;
        let dist = (dx * dx + dy * dy).sqrt();
        let step = (radius_norm * 0.25).max(1.0 / (MASK_DIM as f32));
        if dist <= 1e-6 {
            self.paint_dab(x0, y0, radius_norm, hardness, target_opacity);
        } else {
            let num_steps = (dist / step).ceil() as usize;
            for i in 0..=num_steps {
                let t = i as f32 / num_steps as f32;
                let x = x0 + dx * t;
                let y = y0 + dy * t;
                self.paint_dab(x, y, radius_norm, hardness, target_opacity);
            }
        }
    }

    /// Generate a smart mask automatically from a LinearImage using color & spatial salience.
    pub fn from_smart_mask(kind: SmartMaskKind, image: &LinearImage) -> Self {
        let inv_dim = 1.0 / ((MASK_DIM - 1) as f32);

        // 1. Bilinear sample image across 256x256 grid and convert to Oklab & Oklch
        let mut lab_grid = Vec::with_capacity(MASK_LEN);
        let mut lch_grid = Vec::with_capacity(MASK_LEN);

        for y in 0..MASK_DIM {
            let v = y as f32 * inv_dim;
            for x in 0..MASK_DIM {
                let u = x as f32 * inv_dim;
                let rgba = image.sample_bilinear_f32(u, v);
                let rgb = [rgba[0].max(0.0), rgba[1].max(0.0), rgba[2].max(0.0)];
                let lab = linear_rec2020_to_oklab(rgb);
                let lch = oklab_to_oklch(lab);
                lab_grid.push(lab);
                lch_grid.push(lch);
            }
        }

        // 2. Average border Oklab color for background/subject contrast salience
        let mut border_sum = [0.0f32; 3];
        let mut border_count = 0usize;
        for x in 0..MASK_DIM {
            let top_lab = lab_grid[x];
            let bot_lab = lab_grid[(MASK_DIM - 1) * MASK_DIM + x];
            border_sum[0] += top_lab[0] + bot_lab[0];
            border_sum[1] += top_lab[1] + bot_lab[1];
            border_sum[2] += top_lab[2] + bot_lab[2];
            border_count += 2;
        }
        for y in 1..(MASK_DIM - 1) {
            let left_lab = lab_grid[y * MASK_DIM];
            let right_lab = lab_grid[y * MASK_DIM + (MASK_DIM - 1)];
            border_sum[0] += left_lab[0] + right_lab[0];
            border_sum[1] += left_lab[1] + right_lab[1];
            border_sum[2] += left_lab[2] + right_lab[2];
            border_count += 2;
        }
        let inv_bc = 1.0 / (border_count as f32);
        let border_avg = [
            border_sum[0] * inv_bc,
            border_sum[1] * inv_bc,
            border_sum[2] * inv_bc,
        ];

        // 3. Compute raw float weights per cell
        let mut raw_weights = Vec::with_capacity(MASK_LEN);
        for y in 0..MASK_DIM {
            let v = y as f32 * inv_dim;
            for x in 0..MASK_DIM {
                let u = x as f32 * inv_dim;
                let idx = y * MASK_DIM + x;
                let [l, c, h] = lch_grid[idx];
                let lab = lab_grid[idx];

                let w = match kind {
                    SmartMaskKind::Sky => {
                        let spatial = 1.0 - smoothstep(0.25, 0.72, v);
                        let lum = smoothstep(0.35, 0.55, l);
                        let hue_cool = smoothstep(3.0, 3.6, h) * (1.0 - smoothstep(5.0, 5.5, h));
                        let neutral_sky = 1.0 - smoothstep(0.01, 0.05, c);
                        let color_w = (hue_cool * smoothstep(0.015, 0.05, c) + neutral_sky).clamp(0.0, 1.0);
                        (spatial * lum * color_w).clamp(0.0, 1.0)
                    }
                    SmartMaskKind::SubjectCenter => {
                        let dx = (u - 0.5) / 0.35;
                        let dy = (v - 0.48) / 0.40;
                        let spatial = (-0.5 * (dx * dx + dy * dy)).exp();
                        let d_l = lab[0] - border_avg[0];
                        let d_a = lab[1] - border_avg[1];
                        let d_b = lab[2] - border_avg[2];
                        let delta_e = (d_l * d_l + d_a * d_a + d_b * d_b).sqrt();
                        let salience = smoothstep(0.05, 0.30, delta_e);
                        (spatial * (0.35 + 0.65 * salience)).clamp(0.0, 1.0)
                    }
                    SmartMaskKind::Background => {
                        let dx = (u - 0.5) / 0.35;
                        let dy = (v - 0.48) / 0.40;
                        let spatial = (-0.5 * (dx * dx + dy * dy)).exp();
                        let d_l = lab[0] - border_avg[0];
                        let d_a = lab[1] - border_avg[1];
                        let d_b = lab[2] - border_avg[2];
                        let delta_e = (d_l * d_l + d_a * d_a + d_b * d_b).sqrt();
                        let salience = smoothstep(0.05, 0.30, delta_e);
                        let subject_w = (spatial * (0.35 + 0.65 * salience)).clamp(0.0, 1.0);
                        (1.0 - subject_w).clamp(0.0, 1.0)
                    }
                    SmartMaskKind::Highlights => smoothstep(0.55, 0.88, l),
                    SmartMaskKind::Midtones => {
                        let diff = (l - 0.52) / 0.22;
                        (-diff * diff).exp()
                    }
                    SmartMaskKind::Shadows => 1.0 - smoothstep(0.18, 0.52, l),
                    SmartMaskKind::SkinTones => {
                        let hue_w = smoothstep(0.25, 0.45, h) * (1.0 - smoothstep(1.25, 1.45, h));
                        let c_w = smoothstep(0.015, 0.035, c) * (1.0 - smoothstep(0.16, 0.22, c));
                        let l_w = smoothstep(0.20, 0.30, l) * (1.0 - smoothstep(0.88, 0.95, l));
                        (hue_w * c_w * l_w).clamp(0.0, 1.0)
                    }
                };
                raw_weights.push(w);
            }
        }

        // 4. Two-pass separable box blur (radius 3) to feather transitions smoothly
        const RADIUS: isize = 3;
        const KERNEL_SIZE: f32 = 7.0;

        let mut temp_weights = vec![0.0f32; MASK_LEN];
        for y in 0..MASK_DIM {
            let row_start = y * MASK_DIM;
            for x in 0..MASK_DIM {
                let mut sum = 0.0f32;
                for dx in -RADIUS..=RADIUS {
                    let sx = (x as isize + dx).clamp(0, (MASK_DIM - 1) as isize) as usize;
                    sum += raw_weights[row_start + sx];
                }
                temp_weights[row_start + x] = sum / KERNEL_SIZE;
            }
        }

        let mut blurred_bytes = vec![0u8; MASK_LEN];
        for y in 0..MASK_DIM {
            let row_start = y * MASK_DIM;
            for x in 0..MASK_DIM {
                let mut sum = 0.0f32;
                for dy in -RADIUS..=RADIUS {
                    let sy = (y as isize + dy).clamp(0, (MASK_DIM - 1) as isize) as usize;
                    sum += temp_weights[sy * MASK_DIM + x];
                }
                let avg = sum / KERNEL_SIZE;
                blurred_bytes[row_start + x] = (avg * 255.0).round().clamp(0.0, 255.0) as u8;
            }
        }

        LayerMask { data: blurred_bytes }
    }
}

impl Serialize for LayerMask {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let s = encode_rle_bytes("rle1:", &self.data);
        serializer.serialize_str(&s)
    }
}

impl<'de> Deserialize<'de> for LayerMask {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct LayerMaskVisitor;

        impl<'de> serde::de::Visitor<'de> for LayerMaskVisitor {
            type Value = LayerMask;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a compact rle1 string or a 65536-byte array")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                decode_rle_bytes("rle1:", v, MASK_LEN)
                    .map(|data| LayerMask { data })
                    .map_err(E::custom)
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut data = Vec::with_capacity(MASK_LEN);
                while let Some(byte) = seq.next_element::<u8>()? {
                    data.push(byte);
                }
                if data.len() != MASK_LEN {
                    return Err(serde::de::Error::invalid_length(data.len(), &self));
                }
                Ok(LayerMask { data })
            }
        }

        deserializer.deserialize_any(LayerMaskVisitor)
    }
}

// ----------------------------------------------------------------------------
// SmartMaskKind
// ----------------------------------------------------------------------------

/// Categorical kinds for automated semantic and luminance smart masks.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum SmartMaskKind {
    Sky,
    SubjectCenter,
    Background,
    Highlights,
    Midtones,
    Shadows,
    SkinTones,
}

impl SmartMaskKind {
    pub const ALL: &'static [SmartMaskKind] = &[
        SmartMaskKind::Sky,
        SmartMaskKind::SubjectCenter,
        SmartMaskKind::Background,
        SmartMaskKind::Highlights,
        SmartMaskKind::Midtones,
        SmartMaskKind::Shadows,
        SmartMaskKind::SkinTones,
    ];

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Sky => "Sky",
            Self::SubjectCenter => "Subject",
            Self::Background => "Background",
            Self::Highlights => "Highlights",
            Self::Midtones => "Midtones",
            Self::Shadows => "Shadows",
            Self::SkinTones => "Skin",
        }
    }
}

// ----------------------------------------------------------------------------
// BrushMap
// ----------------------------------------------------------------------------

/// Signed 256x256 brush modulation grid stored as i8 in `[-100, 100]` representing `[-1.0, 1.0]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrushMap {
    data: Vec<i8>,
}

impl Default for BrushMap {
    fn default() -> Self {
        Self::new()
    }
}

impl BrushMap {
    /// Construct a new empty (all zero) brush map.
    pub fn new() -> Self {
        Self {
            data: vec![0; MASK_LEN],
        }
    }

    /// Construct from raw 65,536 signed bytes.
    pub fn from_i8_vec(data: Vec<i8>) -> Result<Self, &'static str> {
        if data.len() != MASK_LEN {
            return Err("BrushMap requires exactly 65,536 values");
        }
        Ok(Self { data })
    }

    /// Clear all cells to zero.
    pub fn clear(&mut self) {
        self.data.fill(0);
    }

    /// Check if every cell is 0.
    pub fn is_empty(&self) -> bool {
        self.data.iter().all(|&v| v == 0)
    }

    /// Raw slice of signed i8 cells.
    pub fn as_slice(&self) -> &[i8] {
        &self.data
    }

    /// Deterministic 64-bit FNV-1a content hash over the 65,536 bytes for GPU texture caching.
    pub fn content_hash(&self) -> u64 {
        // Safe reinterpret cast: i8 as u8 for FNV-1a hashing
        const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
        const FNV_PRIME: u64 = 0x100000001b3;

        let mut hash = FNV_OFFSET_BASIS;
        for &b in &self.data {
            hash ^= (b as u8) as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        hash
    }

    /// Convert signed values into 65,536 normalized floats in `[-1.0, 1.0]` for GPU upload.
    pub fn to_f32_vec(&self) -> Vec<f32> {
        self.data.iter().map(|&v| (v as f32) / 100.0).collect()
    }

    /// Sample brush map with bilinear interpolation in normalized coordinates `[0.0, 1.0]`.
    pub fn sample_bilinear(&self, u: f32, v: f32) -> f32 {
        let u_clamped = u.clamp(0.0, 1.0);
        let v_clamped = v.clamp(0.0, 1.0);

        let fx_coord = u_clamped * ((MASK_DIM - 1) as f32);
        let fy_coord = v_clamped * ((MASK_DIM - 1) as f32);

        let x0 = (fx_coord.floor() as usize).min(MASK_DIM - 1);
        let x1 = (x0 + 1).min(MASK_DIM - 1);
        let y0 = (fy_coord.floor() as usize).min(MASK_DIM - 1);
        let y1 = (y0 + 1).min(MASK_DIM - 1);

        let fx = fx_coord - x0 as f32;
        let fy = fy_coord - y0 as f32;

        let p00 = (self.data[y0 * MASK_DIM + x0] as f32) / 100.0;
        let p10 = (self.data[y0 * MASK_DIM + x1] as f32) / 100.0;
        let p01 = (self.data[y1 * MASK_DIM + x0] as f32) / 100.0;
        let p11 = (self.data[y1 * MASK_DIM + x1] as f32) / 100.0;

        let top = p00 * (1.0 - fx) + p10 * fx;
        let bottom = p01 * (1.0 - fx) + p11 * fx;
        (top * (1.0 - fy) + bottom * fy).clamp(-1.0, 1.0)
    }

    /// Paint a circular feathered dab at normalized `(nx, ny)`.
    pub fn paint_dab(
        &mut self,
        nx: f32,
        ny: f32,
        radius_norm: f32,
        feather: f32,
        step_delta: f32,
        eraser: bool,
    ) {
        let cx = nx.clamp(0.0, 1.0) * ((MASK_DIM - 1) as f32);
        let cy = ny.clamp(0.0, 1.0) * ((MASK_DIM - 1) as f32);
        let r_px = (radius_norm * (MASK_DIM as f32)).max(1.0);

        let min_x = ((cx - r_px).floor() as isize).clamp(0, (MASK_DIM - 1) as isize) as usize;
        let max_x = ((cx + r_px).ceil() as isize).clamp(0, (MASK_DIM - 1) as isize) as usize;
        let min_y = ((cy - r_px).floor() as isize).clamp(0, (MASK_DIM - 1) as isize) as usize;
        let max_y = ((cy + r_px).ceil() as isize).clamp(0, (MASK_DIM - 1) as isize) as usize;

        let f = feather.clamp(0.0, 1.0);
        let inner_ratio = (1.0 - f).clamp(0.0, 0.999);
        let delta = if step_delta.abs() <= 1.0 && step_delta != 0.0 {
            step_delta * 100.0
        } else {
            step_delta
        };

        for y in min_y..=max_y {
            let dy = (y as f32) - cy;
            let dy2 = dy * dy;
            let row_offset = y * MASK_DIM;
            for x in min_x..=max_x {
                let dx = (x as f32) - cx;
                let d = (dx * dx + dy2).sqrt();
                let dist_norm = d / r_px;
                if dist_norm >= 1.0 {
                    continue;
                }

                let falloff = if dist_norm <= inner_ratio {
                    1.0
                } else {
                    let t = (dist_norm - inner_ratio) / (1.0 - inner_ratio);
                    1.0 - (t * t * (3.0 - 2.0 * t))
                };

                let idx = row_offset + x;
                let cur = self.data[idx] as f32;
                let new_val = if eraser {
                    cur * (1.0 - falloff)
                } else {
                    cur + delta * falloff
                };
                self.data[idx] = new_val.round().clamp(-100.0, 100.0) as i8;
            }
        }
    }

    /// Paint a continuous stroke from `(x0, y0)` to `(x1, y1)`.
    pub fn paint_stroke(
        &mut self,
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
        radius_norm: f32,
        feather: f32,
        step_delta: f32,
        eraser: bool,
    ) {
        let dx = x1 - x0;
        let dy = y1 - y0;
        let dist = (dx * dx + dy * dy).sqrt();
        let step = (radius_norm * 0.25).max(1.0 / (MASK_DIM as f32));
        if dist <= 1e-6 {
            self.paint_dab(x0, y0, radius_norm, feather, step_delta, eraser);
        } else {
            let num_steps = (dist / step).ceil() as usize;
            for i in 0..=num_steps {
                let t = i as f32 / num_steps as f32;
                let x = x0 + dx * t;
                let y = y0 + dy * t;
                self.paint_dab(x, y, radius_norm, feather, step_delta, eraser);
            }
        }
    }
}

impl Serialize for BrushMap {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let u8_slice: &[u8] = bytemuck::cast_slice(&self.data);
        let s = encode_rle_bytes("brle1:", u8_slice);
        serializer.serialize_str(&s)
    }
}

impl<'de> Deserialize<'de> for BrushMap {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct BrushMapVisitor;

        impl<'de> serde::de::Visitor<'de> for BrushMapVisitor {
            type Value = BrushMap;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a compact brle1 string or a 65536-element i8 array")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let prefix = if v.starts_with("brle1:") {
                    "brle1:"
                } else if v.starts_with("rle1:") {
                    "rle1:"
                } else {
                    return Err(E::custom("invalid prefix, expected brle1: or rle1:"));
                };

                decode_rle_bytes(prefix, v, MASK_LEN)
                    .map(|u8_vec| {
                        let i8_vec: Vec<i8> = u8_vec.into_iter().map(|b| b as i8).collect();
                        BrushMap { data: i8_vec }
                    })
                    .map_err(E::custom)
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut data = Vec::with_capacity(MASK_LEN);
                while let Some(val) = seq.next_element::<i8>()? {
                    data.push(val);
                }
                if data.len() != MASK_LEN {
                    return Err(serde::de::Error::invalid_length(data.len(), &self));
                }
                Ok(BrushMap { data })
            }
        }

        deserializer.deserialize_any(BrushMapVisitor)
    }
}
