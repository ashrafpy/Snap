use snap_core::ops_ext::{TextOverlayParams, TextStyle};

/// Full-frame anti-aliased vector glyph and badge mask buffer.
/// Channel R: foreground glyphs / decorative stroke coverage (0..=255).
/// Channel G: background badge plate / box fill coverage (0..=255).
/// Channels B, A: 0, 255.
pub struct TextRasterBuffer {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub cache_key: u64,
}

/// Compute a deterministic 64-bit FNV-1a hash over all text overlay parameters.
pub fn compute_text_cache_key(params: &TextOverlayParams) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let mut update_byte = |b: u8| {
        hash ^= b as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    };

    for &b in params.text.as_bytes() {
        update_byte(b);
    }
    update_byte(0xff);
    for &b in params.subtext.as_bytes() {
        update_byte(b);
    }
    update_byte(0xff);
    update_byte(params.style as u8);
    for &b in &params.scale.to_le_bytes() {
        update_byte(b);
    }
    update_byte(if params.invert_mask { 1 } else { 0 });
    update_byte(params.color_preset as u8);
    for &b in &params.center[0].to_le_bytes() {
        update_byte(b);
    }
    for &b in &params.center[1].to_le_bytes() {
        update_byte(b);
    }

    hash
}

/// Placed line segment in aspect-corrected canvas coordinates relative to center.
#[derive(Clone, Copy, Debug)]
struct PlacedSegment {
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    radius: f32,
}

/// Background badge plate geometry.
#[derive(Clone, Copy, Debug)]
enum BadgePlate {
    None,
    RoundedBox {
        cx: f32,
        cy: f32,
        half_w: f32,
        half_h: f32,
        radius: f32,
    },
    Circle {
        cx: f32,
        cy: f32,
        radius: f32,
    },
    Diamond {
        cx: f32,
        cy: f32,
        half_w: f32,
        half_h: f32,
    },
}

impl BadgePlate {
    fn distance(&self, x: f32, y: f32) -> f32 {
        match *self {
            BadgePlate::None => 1e6,
            BadgePlate::RoundedBox {
                cx,
                cy,
                half_w,
                half_h,
                radius,
            } => {
                let dx = (x - cx).abs() - half_w + radius;
                let dy = (y - cy).abs() - half_h + radius;
                let ax = dx.max(0.0);
                let ay = dy.max(0.0);
                (ax * ax + ay * ay).sqrt() + dx.max(dy).min(0.0) - radius
            }
            BadgePlate::Circle { cx, cy, radius } => {
                let dx = x - cx;
                let dy = y - cy;
                (dx * dx + dy * dy).sqrt() - radius
            }
            BadgePlate::Diamond {
                cx,
                cy,
                half_w,
                half_h,
            } => {
                let dx = (x - cx).abs();
                let dy = (y - cy).abs();
                let nx = 1.0 / half_w.max(1e-4);
                let ny = 1.0 / half_h.max(1e-4);
                let dist_plane = (dx * nx + dy * ny - 1.0) / (nx * nx + ny * ny).sqrt();
                dist_plane
            }
        }
    }
}

/// Distance from point (px, py) to segment (ax, ay)-(bx, by).
#[inline]
fn dist_to_segment(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let abx = bx - ax;
    let aby = by - ay;
    let apx = px - ax;
    let apy = py - ay;
    let ab_len_sq = abx * abx + aby * aby;
    let t = if ab_len_sq > 1e-8 {
        ((apx * abx + apy * aby) / ab_len_sq).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let cx = ax + t * abx;
    let cy = ay + t * aby;
    let dx = px - cx;
    let dy = py - cy;
    (dx * dx + dy * dy).sqrt()
}

/// Normalized stroke segment definition inside [0.0, 1.0] x [0.0, 1.0].
#[derive(Clone, Copy)]
struct UnitSegment(f32, f32, f32, f32);

/// Returns vector strokes and proportional advance width for any ASCII printable character.
fn get_glyph_strokes(c: char) -> (&'static [UnitSegment], f32) {
    match c {
        // Uppercase alphabet
        'A' => (&[
            UnitSegment(0.12, 0.90, 0.50, 0.10),
            UnitSegment(0.50, 0.10, 0.88, 0.90),
            UnitSegment(0.24, 0.60, 0.76, 0.60),
        ], 0.75),
        'B' => (&[
            UnitSegment(0.15, 0.10, 0.15, 0.90),
            UnitSegment(0.15, 0.10, 0.65, 0.10),
            UnitSegment(0.65, 0.10, 0.82, 0.26),
            UnitSegment(0.82, 0.26, 0.65, 0.48),
            UnitSegment(0.65, 0.48, 0.15, 0.48),
            UnitSegment(0.65, 0.48, 0.85, 0.68),
            UnitSegment(0.85, 0.68, 0.65, 0.90),
            UnitSegment(0.65, 0.90, 0.15, 0.90),
        ], 0.75),
        'C' => (&[
            UnitSegment(0.85, 0.25, 0.65, 0.10),
            UnitSegment(0.65, 0.10, 0.35, 0.10),
            UnitSegment(0.35, 0.10, 0.15, 0.30),
            UnitSegment(0.15, 0.30, 0.15, 0.70),
            UnitSegment(0.15, 0.70, 0.35, 0.90),
            UnitSegment(0.35, 0.90, 0.65, 0.90),
            UnitSegment(0.65, 0.90, 0.85, 0.75),
        ], 0.72),
        'D' => (&[
            UnitSegment(0.15, 0.10, 0.15, 0.90),
            UnitSegment(0.15, 0.10, 0.55, 0.10),
            UnitSegment(0.55, 0.10, 0.85, 0.35),
            UnitSegment(0.85, 0.35, 0.85, 0.65),
            UnitSegment(0.85, 0.65, 0.55, 0.90),
            UnitSegment(0.55, 0.90, 0.15, 0.90),
        ], 0.75),
        'E' => (&[
            UnitSegment(0.15, 0.10, 0.15, 0.90),
            UnitSegment(0.15, 0.10, 0.80, 0.10),
            UnitSegment(0.15, 0.50, 0.65, 0.50),
            UnitSegment(0.15, 0.90, 0.80, 0.90),
        ], 0.70),
        'F' => (&[
            UnitSegment(0.15, 0.10, 0.15, 0.90),
            UnitSegment(0.15, 0.10, 0.80, 0.10),
            UnitSegment(0.15, 0.50, 0.62, 0.50),
        ], 0.68),
        'G' => (&[
            UnitSegment(0.85, 0.25, 0.65, 0.10),
            UnitSegment(0.65, 0.10, 0.35, 0.10),
            UnitSegment(0.35, 0.10, 0.15, 0.30),
            UnitSegment(0.15, 0.30, 0.15, 0.70),
            UnitSegment(0.15, 0.70, 0.35, 0.90),
            UnitSegment(0.35, 0.90, 0.75, 0.90),
            UnitSegment(0.75, 0.90, 0.85, 0.60),
            UnitSegment(0.85, 0.60, 0.50, 0.60),
        ], 0.75),
        'H' => (&[
            UnitSegment(0.15, 0.10, 0.15, 0.90),
            UnitSegment(0.85, 0.10, 0.85, 0.90),
            UnitSegment(0.15, 0.50, 0.85, 0.50),
        ], 0.75),
        'I' => (&[
            UnitSegment(0.50, 0.10, 0.50, 0.90),
            UnitSegment(0.25, 0.10, 0.75, 0.10),
            UnitSegment(0.25, 0.90, 0.75, 0.90),
        ], 0.40),
        'J' => (&[
            UnitSegment(0.75, 0.10, 0.75, 0.72),
            UnitSegment(0.75, 0.72, 0.55, 0.90),
            UnitSegment(0.55, 0.90, 0.30, 0.90),
            UnitSegment(0.30, 0.90, 0.15, 0.75),
        ], 0.60),
        'K' => (&[
            UnitSegment(0.15, 0.10, 0.15, 0.90),
            UnitSegment(0.85, 0.10, 0.15, 0.55),
            UnitSegment(0.38, 0.42, 0.85, 0.90),
        ], 0.75),
        'L' => (&[
            UnitSegment(0.15, 0.10, 0.15, 0.90),
            UnitSegment(0.15, 0.90, 0.80, 0.90),
        ], 0.65),
        'M' => (&[
            UnitSegment(0.12, 0.90, 0.12, 0.10),
            UnitSegment(0.12, 0.10, 0.50, 0.60),
            UnitSegment(0.50, 0.60, 0.88, 0.10),
            UnitSegment(0.88, 0.10, 0.88, 0.90),
        ], 0.85),
        'N' => (&[
            UnitSegment(0.15, 0.90, 0.15, 0.10),
            UnitSegment(0.15, 0.10, 0.85, 0.90),
            UnitSegment(0.85, 0.90, 0.85, 0.10),
        ], 0.75),
        'O' => (&[
            UnitSegment(0.50, 0.10, 0.22, 0.22),
            UnitSegment(0.22, 0.22, 0.14, 0.50),
            UnitSegment(0.14, 0.50, 0.22, 0.78),
            UnitSegment(0.22, 0.78, 0.50, 0.90),
            UnitSegment(0.50, 0.90, 0.78, 0.78),
            UnitSegment(0.78, 0.78, 0.86, 0.50),
            UnitSegment(0.86, 0.50, 0.78, 0.22),
            UnitSegment(0.78, 0.22, 0.50, 0.10),
        ], 0.75),
        'P' => (&[
            UnitSegment(0.15, 0.10, 0.15, 0.90),
            UnitSegment(0.15, 0.10, 0.65, 0.10),
            UnitSegment(0.65, 0.10, 0.85, 0.30),
            UnitSegment(0.85, 0.30, 0.65, 0.52),
            UnitSegment(0.65, 0.52, 0.15, 0.52),
        ], 0.72),
        'Q' => (&[
            UnitSegment(0.50, 0.10, 0.22, 0.22),
            UnitSegment(0.22, 0.22, 0.14, 0.50),
            UnitSegment(0.14, 0.50, 0.22, 0.78),
            UnitSegment(0.22, 0.78, 0.50, 0.90),
            UnitSegment(0.50, 0.90, 0.78, 0.78),
            UnitSegment(0.78, 0.78, 0.86, 0.50),
            UnitSegment(0.86, 0.50, 0.78, 0.22),
            UnitSegment(0.78, 0.22, 0.50, 0.10),
            UnitSegment(0.58, 0.65, 0.88, 0.95),
        ], 0.75),
        'R' => (&[
            UnitSegment(0.15, 0.10, 0.15, 0.90),
            UnitSegment(0.15, 0.10, 0.65, 0.10),
            UnitSegment(0.65, 0.10, 0.85, 0.30),
            UnitSegment(0.85, 0.30, 0.65, 0.52),
            UnitSegment(0.65, 0.52, 0.15, 0.52),
            UnitSegment(0.48, 0.52, 0.85, 0.90),
        ], 0.74),
        'S' => (&[
            UnitSegment(0.82, 0.25, 0.60, 0.10),
            UnitSegment(0.60, 0.10, 0.35, 0.10),
            UnitSegment(0.35, 0.10, 0.18, 0.28),
            UnitSegment(0.18, 0.28, 0.50, 0.50),
            UnitSegment(0.50, 0.50, 0.82, 0.68),
            UnitSegment(0.82, 0.68, 0.65, 0.90),
            UnitSegment(0.65, 0.90, 0.35, 0.90),
            UnitSegment(0.35, 0.90, 0.18, 0.75),
        ], 0.70),
        'T' => (&[
            UnitSegment(0.10, 0.10, 0.90, 0.10),
            UnitSegment(0.50, 0.10, 0.50, 0.90),
        ], 0.72),
        'U' => (&[
            UnitSegment(0.15, 0.10, 0.15, 0.70),
            UnitSegment(0.15, 0.70, 0.35, 0.90),
            UnitSegment(0.35, 0.90, 0.65, 0.90),
            UnitSegment(0.65, 0.90, 0.85, 0.70),
            UnitSegment(0.85, 0.70, 0.85, 0.10),
        ], 0.75),
        'V' => (&[
            UnitSegment(0.15, 0.10, 0.50, 0.90),
            UnitSegment(0.50, 0.90, 0.85, 0.10),
        ], 0.75),
        'W' => (&[
            UnitSegment(0.10, 0.10, 0.28, 0.90),
            UnitSegment(0.28, 0.90, 0.50, 0.45),
            UnitSegment(0.50, 0.45, 0.72, 0.90),
            UnitSegment(0.72, 0.90, 0.90, 0.10),
        ], 0.85),
        'X' => (&[
            UnitSegment(0.15, 0.10, 0.85, 0.90),
            UnitSegment(0.85, 0.10, 0.15, 0.90),
        ], 0.72),
        'Y' => (&[
            UnitSegment(0.15, 0.10, 0.50, 0.50),
            UnitSegment(0.85, 0.10, 0.50, 0.50),
            UnitSegment(0.50, 0.50, 0.50, 0.90),
        ], 0.72),
        'Z' => (&[
            UnitSegment(0.15, 0.10, 0.85, 0.10),
            UnitSegment(0.85, 0.10, 0.15, 0.90),
            UnitSegment(0.15, 0.90, 0.85, 0.90),
        ], 0.70),

        // Lowercase alphabet
        'a' => (&[
            UnitSegment(0.72, 0.40, 0.72, 0.90),
            UnitSegment(0.72, 0.45, 0.38, 0.45),
            UnitSegment(0.38, 0.45, 0.20, 0.65),
            UnitSegment(0.20, 0.65, 0.38, 0.90),
            UnitSegment(0.38, 0.90, 0.72, 0.90),
        ], 0.62),
        'b' => (&[
            UnitSegment(0.20, 0.10, 0.20, 0.90),
            UnitSegment(0.20, 0.50, 0.70, 0.50),
            UnitSegment(0.70, 0.50, 0.80, 0.70),
            UnitSegment(0.80, 0.70, 0.70, 0.90),
            UnitSegment(0.70, 0.90, 0.20, 0.90),
        ], 0.65),
        'c' => (&[
            UnitSegment(0.75, 0.50, 0.40, 0.42),
            UnitSegment(0.40, 0.42, 0.20, 0.65),
            UnitSegment(0.20, 0.65, 0.40, 0.90),
            UnitSegment(0.40, 0.90, 0.75, 0.82),
        ], 0.58),
        'd' => (&[
            UnitSegment(0.75, 0.10, 0.75, 0.90),
            UnitSegment(0.75, 0.50, 0.25, 0.50),
            UnitSegment(0.25, 0.50, 0.18, 0.70),
            UnitSegment(0.18, 0.70, 0.25, 0.90),
            UnitSegment(0.25, 0.90, 0.75, 0.90),
        ], 0.65),
        'e' => (&[
            UnitSegment(0.20, 0.65, 0.75, 0.65),
            UnitSegment(0.75, 0.65, 0.75, 0.50),
            UnitSegment(0.75, 0.50, 0.40, 0.42),
            UnitSegment(0.40, 0.42, 0.20, 0.65),
            UnitSegment(0.20, 0.65, 0.40, 0.90),
            UnitSegment(0.40, 0.90, 0.75, 0.82),
        ], 0.60),
        'f' => (&[
            UnitSegment(0.65, 0.10, 0.45, 0.10),
            UnitSegment(0.45, 0.10, 0.45, 0.90),
            UnitSegment(0.25, 0.42, 0.68, 0.42),
        ], 0.52),
        'g' => (&[
            UnitSegment(0.72, 0.45, 0.38, 0.45),
            UnitSegment(0.38, 0.45, 0.20, 0.65),
            UnitSegment(0.20, 0.65, 0.38, 0.85),
            UnitSegment(0.38, 0.85, 0.72, 0.85),
            UnitSegment(0.72, 0.45, 0.72, 1.10),
            UnitSegment(0.72, 1.10, 0.35, 1.10),
        ], 0.65),
        'h' => (&[
            UnitSegment(0.20, 0.10, 0.20, 0.90),
            UnitSegment(0.20, 0.50, 0.55, 0.45),
            UnitSegment(0.55, 0.45, 0.75, 0.55),
            UnitSegment(0.75, 0.55, 0.75, 0.90),
        ], 0.65),
        'i' => (&[
            UnitSegment(0.50, 0.42, 0.50, 0.90),
            UnitSegment(0.50, 0.20, 0.50, 0.26),
        ], 0.35),
        'j' => (&[
            UnitSegment(0.60, 0.42, 0.60, 1.05),
            UnitSegment(0.60, 1.05, 0.35, 1.10),
            UnitSegment(0.60, 0.20, 0.60, 0.26),
        ], 0.42),
        'k' => (&[
            UnitSegment(0.20, 0.10, 0.20, 0.90),
            UnitSegment(0.75, 0.45, 0.20, 0.68),
            UnitSegment(0.40, 0.58, 0.75, 0.90),
        ], 0.62),
        'l' => (&[
            UnitSegment(0.45, 0.10, 0.45, 0.85),
            UnitSegment(0.45, 0.85, 0.65, 0.90),
        ], 0.38),
        'm' => (&[
            UnitSegment(0.15, 0.42, 0.15, 0.90),
            UnitSegment(0.15, 0.45, 0.45, 0.45),
            UnitSegment(0.45, 0.45, 0.45, 0.90),
            UnitSegment(0.45, 0.45, 0.75, 0.45),
            UnitSegment(0.75, 0.45, 0.75, 0.90),
        ], 0.80),
        'n' => (&[
            UnitSegment(0.20, 0.42, 0.20, 0.90),
            UnitSegment(0.20, 0.45, 0.60, 0.45),
            UnitSegment(0.60, 0.45, 0.75, 0.60),
            UnitSegment(0.75, 0.60, 0.75, 0.90),
        ], 0.62),
        'o' => (&[
            UnitSegment(0.50, 0.45, 0.25, 0.55),
            UnitSegment(0.25, 0.55, 0.25, 0.80),
            UnitSegment(0.25, 0.80, 0.50, 0.90),
            UnitSegment(0.50, 0.90, 0.75, 0.80),
            UnitSegment(0.75, 0.80, 0.75, 0.55),
            UnitSegment(0.75, 0.55, 0.50, 0.45),
        ], 0.62),
        'p' => (&[
            UnitSegment(0.20, 0.42, 0.20, 1.15),
            UnitSegment(0.20, 0.45, 0.68, 0.45),
            UnitSegment(0.68, 0.45, 0.75, 0.65),
            UnitSegment(0.75, 0.65, 0.68, 0.85),
            UnitSegment(0.68, 0.85, 0.20, 0.85),
        ], 0.65),
        'q' => (&[
            UnitSegment(0.75, 0.42, 0.75, 1.15),
            UnitSegment(0.75, 0.45, 0.28, 0.45),
            UnitSegment(0.28, 0.45, 0.20, 0.65),
            UnitSegment(0.20, 0.65, 0.28, 0.85),
            UnitSegment(0.28, 0.85, 0.75, 0.85),
        ], 0.65),
        'r' => (&[
            UnitSegment(0.20, 0.42, 0.20, 0.90),
            UnitSegment(0.20, 0.55, 0.50, 0.45),
            UnitSegment(0.50, 0.45, 0.75, 0.50),
        ], 0.55),
        's' => (&[
            UnitSegment(0.75, 0.50, 0.40, 0.45),
            UnitSegment(0.40, 0.45, 0.22, 0.60),
            UnitSegment(0.22, 0.60, 0.72, 0.72),
            UnitSegment(0.72, 0.72, 0.60, 0.90),
            UnitSegment(0.60, 0.90, 0.22, 0.85),
        ], 0.58),
        't' => (&[
            UnitSegment(0.42, 0.22, 0.42, 0.85),
            UnitSegment(0.42, 0.85, 0.65, 0.90),
            UnitSegment(0.22, 0.42, 0.65, 0.42),
        ], 0.48),
        'u' => (&[
            UnitSegment(0.20, 0.42, 0.20, 0.80),
            UnitSegment(0.20, 0.80, 0.40, 0.90),
            UnitSegment(0.40, 0.90, 0.75, 0.80),
            UnitSegment(0.75, 0.80, 0.75, 0.42),
            UnitSegment(0.75, 0.70, 0.75, 0.90),
        ], 0.62),
        'v' => (&[
            UnitSegment(0.18, 0.42, 0.50, 0.90),
            UnitSegment(0.50, 0.90, 0.82, 0.42),
        ], 0.65),
        'w' => (&[
            UnitSegment(0.15, 0.42, 0.30, 0.90),
            UnitSegment(0.30, 0.90, 0.50, 0.60),
            UnitSegment(0.50, 0.60, 0.70, 0.90),
            UnitSegment(0.70, 0.90, 0.85, 0.42),
        ], 0.80),
        'x' => (&[
            UnitSegment(0.20, 0.42, 0.80, 0.90),
            UnitSegment(0.80, 0.42, 0.20, 0.90),
        ], 0.62),
        'y' => (&[
            UnitSegment(0.20, 0.42, 0.48, 0.82),
            UnitSegment(0.80, 0.42, 0.25, 1.15),
        ], 0.65),
        'z' => (&[
            UnitSegment(0.20, 0.45, 0.78, 0.45),
            UnitSegment(0.78, 0.45, 0.20, 0.90),
            UnitSegment(0.20, 0.90, 0.78, 0.90),
        ], 0.58),

        // Digits
        '0' => (&[
            UnitSegment(0.50, 0.10, 0.22, 0.25),
            UnitSegment(0.22, 0.25, 0.22, 0.75),
            UnitSegment(0.22, 0.75, 0.50, 0.90),
            UnitSegment(0.50, 0.90, 0.78, 0.75),
            UnitSegment(0.78, 0.75, 0.78, 0.25),
            UnitSegment(0.78, 0.25, 0.50, 0.10),
            UnitSegment(0.72, 0.22, 0.28, 0.78),
        ], 0.72),
        '1' => (&[
            UnitSegment(0.30, 0.25, 0.50, 0.10),
            UnitSegment(0.50, 0.10, 0.50, 0.90),
            UnitSegment(0.25, 0.90, 0.75, 0.90),
        ], 0.52),
        '2' => (&[
            UnitSegment(0.20, 0.25, 0.40, 0.10),
            UnitSegment(0.40, 0.10, 0.65, 0.10),
            UnitSegment(0.65, 0.10, 0.80, 0.30),
            UnitSegment(0.80, 0.30, 0.20, 0.90),
            UnitSegment(0.20, 0.90, 0.80, 0.90),
        ], 0.68),
        '3' => (&[
            UnitSegment(0.20, 0.10, 0.78, 0.10),
            UnitSegment(0.78, 0.10, 0.45, 0.45),
            UnitSegment(0.45, 0.45, 0.75, 0.55),
            UnitSegment(0.75, 0.55, 0.75, 0.75),
            UnitSegment(0.75, 0.75, 0.50, 0.90),
            UnitSegment(0.50, 0.90, 0.20, 0.85),
        ], 0.68),
        '4' => (&[
            UnitSegment(0.70, 0.90, 0.70, 0.10),
            UnitSegment(0.70, 0.10, 0.15, 0.65),
            UnitSegment(0.15, 0.65, 0.85, 0.65),
        ], 0.70),
        '5' => (&[
            UnitSegment(0.78, 0.10, 0.22, 0.10),
            UnitSegment(0.22, 0.10, 0.22, 0.45),
            UnitSegment(0.22, 0.45, 0.65, 0.45),
            UnitSegment(0.65, 0.45, 0.80, 0.60),
            UnitSegment(0.80, 0.60, 0.75, 0.80),
            UnitSegment(0.75, 0.80, 0.50, 0.90),
            UnitSegment(0.50, 0.90, 0.20, 0.85),
        ], 0.68),
        '6' => (&[
            UnitSegment(0.75, 0.20, 0.50, 0.10),
            UnitSegment(0.50, 0.10, 0.25, 0.30),
            UnitSegment(0.25, 0.30, 0.20, 0.75),
            UnitSegment(0.20, 0.75, 0.45, 0.90),
            UnitSegment(0.45, 0.90, 0.75, 0.80),
            UnitSegment(0.75, 0.80, 0.75, 0.60),
            UnitSegment(0.75, 0.60, 0.50, 0.50),
            UnitSegment(0.50, 0.50, 0.20, 0.65),
        ], 0.70),
        '7' => (&[
            UnitSegment(0.15, 0.10, 0.85, 0.10),
            UnitSegment(0.85, 0.10, 0.35, 0.90),
        ], 0.68),
        '8' => (&[
            UnitSegment(0.50, 0.10, 0.25, 0.25),
            UnitSegment(0.25, 0.25, 0.50, 0.48),
            UnitSegment(0.50, 0.48, 0.75, 0.25),
            UnitSegment(0.75, 0.25, 0.50, 0.10),
            UnitSegment(0.50, 0.48, 0.20, 0.70),
            UnitSegment(0.20, 0.70, 0.50, 0.90),
            UnitSegment(0.50, 0.90, 0.80, 0.70),
            UnitSegment(0.80, 0.70, 0.50, 0.48),
        ], 0.70),
        '9' => (&[
            UnitSegment(0.25, 0.80, 0.50, 0.90),
            UnitSegment(0.50, 0.90, 0.75, 0.70),
            UnitSegment(0.75, 0.70, 0.80, 0.25),
            UnitSegment(0.80, 0.25, 0.55, 0.10),
            UnitSegment(0.55, 0.10, 0.25, 0.20),
            UnitSegment(0.25, 0.20, 0.25, 0.40),
            UnitSegment(0.25, 0.40, 0.50, 0.50),
            UnitSegment(0.50, 0.50, 0.80, 0.35),
        ], 0.70),

        // Punctuation & symbols
        '.' => (&[UnitSegment(0.50, 0.84, 0.50, 0.90)], 0.35),
        ',' => (&[UnitSegment(0.52, 0.84, 0.40, 0.96)], 0.35),
        '-' => (&[UnitSegment(0.20, 0.50, 0.80, 0.50)], 0.55),
        '_' => (&[UnitSegment(0.05, 0.95, 0.95, 0.95)], 0.60),
        '/' => (&[UnitSegment(0.15, 0.90, 0.85, 0.10)], 0.55),
        ':' => (&[
            UnitSegment(0.50, 0.35, 0.50, 0.40),
            UnitSegment(0.50, 0.75, 0.50, 0.80),
        ], 0.35),
        '!' => (&[
            UnitSegment(0.50, 0.10, 0.50, 0.65),
            UnitSegment(0.50, 0.84, 0.50, 0.90),
        ], 0.35),
        '?' => (&[
            UnitSegment(0.20, 0.25, 0.40, 0.10),
            UnitSegment(0.40, 0.10, 0.65, 0.10),
            UnitSegment(0.65, 0.10, 0.75, 0.30),
            UnitSegment(0.75, 0.30, 0.50, 0.50),
            UnitSegment(0.50, 0.50, 0.50, 0.65),
            UnitSegment(0.50, 0.84, 0.50, 0.90),
        ], 0.62),
        '&' => (&[
            UnitSegment(0.75, 0.90, 0.35, 0.50),
            UnitSegment(0.35, 0.50, 0.50, 0.20),
            UnitSegment(0.50, 0.20, 0.30, 0.10),
            UnitSegment(0.30, 0.10, 0.25, 0.30),
            UnitSegment(0.25, 0.30, 0.65, 0.75),
            UnitSegment(0.65, 0.75, 0.45, 0.90),
            UnitSegment(0.45, 0.90, 0.20, 0.75),
            UnitSegment(0.20, 0.75, 0.45, 0.50),
        ], 0.75),
        '\'' => (&[UnitSegment(0.50, 0.10, 0.45, 0.25)], 0.28),
        '"' => (&[
            UnitSegment(0.35, 0.10, 0.30, 0.25),
            UnitSegment(0.65, 0.10, 0.60, 0.25),
        ], 0.45),
        '(' => (&[
            UnitSegment(0.65, 0.10, 0.35, 0.50),
            UnitSegment(0.35, 0.50, 0.65, 0.90),
        ], 0.40),
        ')' => (&[
            UnitSegment(0.35, 0.10, 0.65, 0.50),
            UnitSegment(0.65, 0.50, 0.35, 0.90),
        ], 0.40),
        '+' => (&[
            UnitSegment(0.15, 0.50, 0.85, 0.50),
            UnitSegment(0.50, 0.15, 0.50, 0.85),
        ], 0.65),
        '#' => (&[
            UnitSegment(0.35, 0.10, 0.35, 0.90),
            UnitSegment(0.65, 0.10, 0.65, 0.90),
            UnitSegment(0.15, 0.35, 0.85, 0.35),
            UnitSegment(0.15, 0.65, 0.85, 0.65),
        ], 0.70),
        ' ' => (&[], 0.45),
        _ => (&[], 0.40),
    }
}

/// Layout a single line of text into segments with alignment and stylistic adjustments.
fn layout_text_line(
    text: &str,
    font_size: f32,
    stroke_r: f32,
    serifs: bool,
    italic: f32,
    tracking: f32,
    center_x: f32,
    base_y: f32,
    out: &mut Vec<PlacedSegment>,
) -> (f32, f32) {
    if text.is_empty() {
        return (0.0, 0.0);
    }

    // Measure total width
    let mut total_w = 0.0;
    for c in text.chars() {
        let (_, adv) = get_glyph_strokes(c);
        total_w += adv * font_size + tracking;
    }
    if total_w > tracking {
        total_w -= tracking;
    }

    let start_x = center_x - total_w * 0.5;
    let mut cursor_x = start_x;

    for c in text.chars() {
        let (strokes, adv) = get_glyph_strokes(c);
        let char_w = adv * font_size;

        for s in strokes {
            let mut x0 = cursor_x + s.0 * char_w;
            let y0 = base_y + s.1 * font_size;
            let mut x1 = cursor_x + s.2 * char_w;
            let y1 = base_y + s.3 * font_size;

            if italic != 0.0 {
                let baseline = base_y + font_size * 0.85;
                x0 += (baseline - y0) * italic;
                x1 += (baseline - y1) * italic;
            }

            out.push(PlacedSegment {
                x0,
                y0,
                x1,
                y1,
                radius: stroke_r,
            });

            // If editorial serifs are enabled, add terminal cross-ticks for vertical stems
            if serifs && (s.0 - s.2).abs() < 0.05 && (s.1 - s.3).abs() > 0.4 {
                let tick_len = font_size * 0.12;
                out.push(PlacedSegment {
                    x0: x0 - tick_len,
                    y0,
                    x1: x0 + tick_len,
                    y1: y0,
                    radius: stroke_r * 0.85,
                });
                out.push(PlacedSegment {
                    x0: x1 - tick_len,
                    y0: y1,
                    x1: x1 + tick_len,
                    y1,
                    radius: stroke_r * 0.85,
                });
            }
        }

        cursor_x += char_w + tracking;
    }

    (total_w, font_size)
}

/// Rasterizes anti-aliased vector text, glyphs, and badges for TextOverlayParams.
pub fn rasterize_text_overlay(params: &TextOverlayParams, canvas_aspect: f32) -> TextRasterBuffer {
    const BUFFER_SIZE: u32 = 768;
    let cache_key = compute_text_cache_key(params);

    let mut rgba = vec![0u8; (BUFFER_SIZE * BUFFER_SIZE * 4) as usize];
    // Pre-initialize alpha channel to 255
    for i in 0..(BUFFER_SIZE * BUFFER_SIZE) as usize {
        rgba[i * 4 + 3] = 255;
    }

    if params.text.is_empty() {
        return TextRasterBuffer {
            width: BUFFER_SIZE,
            height: BUFFER_SIZE,
            rgba,
            cache_key,
        };
    }

    let mut segments = Vec::with_capacity(256);
    let mut badge = BadgePlate::None;

    let base_scale = params.scale.clamp(0.1, 4.0);
    let main_font_size = 0.075 * base_scale;
    let sub_font_size = 0.038 * base_scale;

    match params.style {
        TextStyle::ModernBold => {
            let stroke_r = main_font_size * 0.11;
            let tracking = main_font_size * 0.25;
            let (w_main, _) = layout_text_line(
                &params.text,
                main_font_size,
                stroke_r,
                false,
                0.0,
                tracking,
                0.0,
                -main_font_size * 0.55,
                &mut segments,
            );

            if !params.subtext.is_empty() {
                let sub_stroke = sub_font_size * 0.08;
                let sub_track = sub_font_size * 0.40;
                layout_text_line(
                    &params.subtext,
                    sub_font_size,
                    sub_stroke,
                    false,
                    0.0,
                    sub_track,
                    0.0,
                    main_font_size * 0.65,
                    &mut segments,
                );
            }

            if params.invert_mask {
                let bw = (w_main * 0.65).max(0.20 * base_scale);
                let bh = main_font_size * 1.35;
                badge = BadgePlate::RoundedBox {
                    cx: 0.0,
                    cy: 0.0,
                    half_w: bw,
                    half_h: bh,
                    radius: 0.02 * base_scale,
                };
            }
        }
        TextStyle::EditorialSerif => {
            let stroke_r = main_font_size * 0.055;
            let tracking = main_font_size * 0.18;
            let (w_main, _) = layout_text_line(
                &params.text,
                main_font_size,
                stroke_r,
                true,
                0.0,
                tracking,
                0.0,
                -main_font_size * 0.60,
                &mut segments,
            );

            let divider_y = main_font_size * 0.48;
            let half_line = (w_main * 0.55).max(0.15 * base_scale);
            segments.push(PlacedSegment {
                x0: -half_line,
                y0: divider_y,
                x1: half_line,
                y1: divider_y,
                radius: 0.0016 * base_scale,
            });

            if !params.subtext.is_empty() {
                let sub_stroke = sub_font_size * 0.06;
                let sub_track = sub_font_size * 0.35;
                layout_text_line(
                    &params.subtext,
                    sub_font_size,
                    sub_stroke,
                    true,
                    0.0,
                    sub_track,
                    0.0,
                    divider_y + 0.015 * base_scale,
                    &mut segments,
                );
            }

            if params.invert_mask {
                badge = BadgePlate::RoundedBox {
                    cx: 0.0,
                    cy: 0.0,
                    half_w: half_line + 0.04 * base_scale,
                    half_h: main_font_size * 1.4,
                    radius: 0.01 * base_scale,
                };
            }
        }
        TextStyle::MinimalBox => {
            let stroke_r = main_font_size * 0.08;
            let tracking = main_font_size * 0.15;
            let (w_main, _) = layout_text_line(
                &params.text,
                main_font_size,
                stroke_r,
                false,
                0.0,
                tracking,
                0.0,
                -main_font_size * 0.50,
                &mut segments,
            );

            if !params.subtext.is_empty() {
                let sub_stroke = sub_font_size * 0.07;
                let sub_track = sub_font_size * 0.25;
                layout_text_line(
                    &params.subtext,
                    sub_font_size,
                    sub_stroke,
                    false,
                    0.0,
                    sub_track,
                    0.0,
                    main_font_size * 0.60,
                    &mut segments,
                );
            }

            let box_hw = (w_main * 0.58).max(0.18 * base_scale) + 0.04 * base_scale;
            let box_hh = main_font_size * 1.25 + 0.02 * base_scale;
            let box_r = 0.018 * base_scale;

            badge = BadgePlate::RoundedBox {
                cx: 0.0,
                cy: 0.0,
                half_w: box_hw,
                half_h: box_hh,
                radius: box_r,
            };

            // Outer crisp border stroke in R channel
            let b_thick = 0.0035 * base_scale;
            let p0 = (-box_hw + box_r, -box_hh);
            let p1 = (box_hw - box_r, -box_hh);
            let p2 = (box_hw, -box_hh + box_r);
            let p3 = (box_hw, box_hh - box_r);
            let p4 = (box_hw - box_r, box_hh);
            let p5 = (-box_hw + box_r, box_hh);
            let p6 = (-box_hw, box_hh - box_r);
            let p7 = (-box_hw, -box_hh + box_r);

            segments.push(PlacedSegment { x0: p0.0, y0: p0.1, x1: p1.0, y1: p1.1, radius: b_thick });
            segments.push(PlacedSegment { x0: p2.0, y0: p2.1, x1: p3.0, y1: p3.1, radius: b_thick });
            segments.push(PlacedSegment { x0: p4.0, y0: p4.1, x1: p5.0, y1: p5.1, radius: b_thick });
            segments.push(PlacedSegment { x0: p6.0, y0: p6.1, x1: p7.0, y1: p7.1, radius: b_thick });
        }
        TextStyle::StampFrame => {
            let stroke_r = main_font_size * 0.085;
            let tracking = main_font_size * 0.20;
            let (w_main, _) = layout_text_line(
                &params.text,
                main_font_size,
                stroke_r,
                false,
                0.0,
                tracking,
                0.0,
                -main_font_size * 0.45,
                &mut segments,
            );

            if !params.subtext.is_empty() {
                let sub_stroke = sub_font_size * 0.08;
                let sub_track = sub_font_size * 0.28;
                layout_text_line(
                    &params.subtext,
                    sub_font_size,
                    sub_stroke,
                    false,
                    0.0,
                    sub_track,
                    0.0,
                    main_font_size * 0.65,
                    &mut segments,
                );
            }

            let stamp_r = (w_main * 0.62).max(main_font_size * 1.85);
            badge = BadgePlate::Circle {
                cx: 0.0,
                cy: 0.0,
                radius: stamp_r,
            };

            // Double circle border in R channel
            let outer_r = stamp_r;
            let inner_r = stamp_r * 0.88;
            let num_pts = 48;
            for i in 0..num_pts {
                let a0 = (i as f32 / num_pts as f32) * std::f32::consts::TAU;
                let a1 = ((i + 1) as f32 / num_pts as f32) * std::f32::consts::TAU;

                segments.push(PlacedSegment {
                    x0: a0.cos() * outer_r,
                    y0: a0.sin() * outer_r,
                    x1: a1.cos() * outer_r,
                    y1: a1.sin() * outer_r,
                    radius: 0.0035 * base_scale,
                });
                segments.push(PlacedSegment {
                    x0: a0.cos() * inner_r,
                    y0: a0.sin() * inner_r,
                    x1: a1.cos() * inner_r,
                    y1: a1.sin() * inner_r,
                    radius: 0.0020 * base_scale,
                });
            }

            // Small decorative dots on left and right
            let dot_x = stamp_r * 0.72;
            segments.push(PlacedSegment {
                x0: -dot_x,
                y0: 0.0,
                x1: -dot_x,
                y1: 0.0,
                radius: 0.006 * base_scale,
            });
            segments.push(PlacedSegment {
                x0: dot_x,
                y0: 0.0,
                x1: dot_x,
                y1: 0.0,
                radius: 0.006 * base_scale,
            });
        }
        TextStyle::CinemaSub => {
            let stroke_r = main_font_size * 0.08;
            let tracking = main_font_size * 0.12;
            let (w_main, _) = layout_text_line(
                &params.text,
                main_font_size,
                stroke_r,
                false,
                0.0,
                tracking,
                0.0,
                -main_font_size * 0.40,
                &mut segments,
            );

            if !params.subtext.is_empty() {
                let sub_stroke = sub_font_size * 0.07;
                let sub_track = sub_font_size * 0.20;
                layout_text_line(
                    &params.subtext,
                    sub_font_size,
                    sub_stroke,
                    false,
                    0.0,
                    sub_track,
                    0.0,
                    main_font_size * 0.70,
                    &mut segments,
                );
            }

            // Lower-third / center dark bar
            let bar_hw = (w_main * 0.60).max(0.35 * canvas_aspect * base_scale) + 0.05 * base_scale;
            let bar_hh = main_font_size * 1.35;
            badge = BadgePlate::RoundedBox {
                cx: 0.0,
                cy: 0.0,
                half_w: bar_hw,
                half_h: bar_hh,
                radius: 0.008 * base_scale,
            };
        }
        TextStyle::RetroBadge => {
            let stroke_r = main_font_size * 0.09;
            let tracking = main_font_size * 0.22;
            let (w_main, _) = layout_text_line(
                &params.text,
                main_font_size,
                stroke_r,
                false,
                0.0,
                tracking,
                0.0,
                -main_font_size * 0.48,
                &mut segments,
            );

            if !params.subtext.is_empty() {
                let sub_stroke = sub_font_size * 0.075;
                let sub_track = sub_font_size * 0.35;
                layout_text_line(
                    &params.subtext,
                    sub_font_size,
                    sub_stroke,
                    false,
                    0.0,
                    sub_track,
                    0.0,
                    main_font_size * 0.70,
                    &mut segments,
                );
            }

            let crest_hw = (w_main * 0.65).max(0.22 * base_scale);
            let crest_hh = main_font_size * 1.55;
            badge = BadgePlate::Diamond {
                cx: 0.0,
                cy: 0.0,
                half_w: crest_hw,
                half_h: crest_hh,
            };

            // Diamond border lines in R channel
            let b_thick = 0.003 * base_scale;
            segments.push(PlacedSegment { x0: 0.0, y0: -crest_hh, x1: crest_hw, y1: 0.0, radius: b_thick });
            segments.push(PlacedSegment { x0: crest_hw, y0: 0.0, x1: 0.0, y1: crest_hh, radius: b_thick });
            segments.push(PlacedSegment { x0: 0.0, y0: crest_hh, x1: -crest_hw, y1: 0.0, radius: b_thick });
            segments.push(PlacedSegment { x0: -crest_hw, y0: 0.0, x1: 0.0, y1: -crest_hh, radius: b_thick });

            // Horizontal divider rules
            let div_w = crest_hw * 0.70;
            let y_rule_top = -main_font_size * 0.65;
            let y_rule_bot = main_font_size * 0.50;
            segments.push(PlacedSegment { x0: -div_w, y0: y_rule_top, x1: div_w, y1: y_rule_top, radius: 0.0018 * base_scale });
            segments.push(PlacedSegment { x0: -div_w, y0: y_rule_bot, x1: div_w, y1: y_rule_bot, radius: 0.0018 * base_scale });
        }
        TextStyle::VerticalTag => {
            let stroke_r = main_font_size * 0.085;
            let line_step = main_font_size * 0.95;
            let chars: Vec<char> = params.text.chars().collect();
            let n = chars.len() as f32;
            let start_y = -(n - 1.0) * line_step * 0.5 - main_font_size * 0.5;

            for (i, &c) in chars.iter().enumerate() {
                let char_y = start_y + i as f32 * line_step;
                let mut buf = String::with_capacity(1);
                buf.push(c);
                layout_text_line(
                    &buf,
                    main_font_size,
                    stroke_r,
                    false,
                    0.0,
                    0.0,
                    0.02 * base_scale,
                    char_y,
                    &mut segments,
                );
            }

            // Vertical accent bar on the left
            let bar_x = -main_font_size * 0.55;
            let bar_y0 = start_y;
            let bar_y1 = start_y + n * line_step;
            segments.push(PlacedSegment {
                x0: bar_x,
                y0: bar_y0,
                x1: bar_x,
                y1: bar_y1,
                radius: 0.004 * base_scale,
            });

            let tag_hw = main_font_size * 0.85;
            let tag_hh = (n * line_step * 0.55).max(0.15 * base_scale);
            badge = BadgePlate::RoundedBox {
                cx: 0.0,
                cy: 0.0,
                half_w: tag_hw,
                half_h: tag_hh,
                radius: 0.015 * base_scale,
            };
        }
        TextStyle::Signature => {
            let stroke_r = main_font_size * 0.052;
            let tracking = main_font_size * 0.08;
            let italic = 0.28;
            let (w_main, _) = layout_text_line(
                &params.text,
                main_font_size,
                stroke_r,
                false,
                italic,
                tracking,
                0.0,
                -main_font_size * 0.55,
                &mut segments,
            );

            // Expressive underline flourish / swirl
            let swirl_y = main_font_size * 0.52;
            let sx0 = -w_main * 0.55;
            let sx1 = w_main * 0.58;
            let sx2 = sx1 + 0.05 * base_scale;
            let sy2 = swirl_y - 0.035 * base_scale;

            segments.push(PlacedSegment { x0: sx0, y0: swirl_y, x1: sx1, y1: swirl_y, radius: stroke_r * 0.9 });
            segments.push(PlacedSegment { x0: sx1, y0: swirl_y, x1: sx2, y1: sy2, radius: stroke_r * 0.7 });

            if !params.subtext.is_empty() {
                let sub_stroke = sub_font_size * 0.055;
                let sub_track = sub_font_size * 0.25;
                layout_text_line(
                    &params.subtext,
                    sub_font_size,
                    sub_stroke,
                    false,
                    italic * 0.5,
                    sub_track,
                    0.0,
                    swirl_y + 0.02 * base_scale,
                    &mut segments,
                );
            }

            if params.invert_mask {
                badge = BadgePlate::RoundedBox {
                    cx: 0.0,
                    cy: 0.0,
                    half_w: (w_main * 0.65).max(0.18 * base_scale),
                    half_h: main_font_size * 1.35,
                    radius: 0.02 * base_scale,
                };
            }
        }
    }

    // Determine bounding box in normalized aspect-corrected canvas coordinates
    let mut min_x: f32 = 1e6;
    let mut max_x: f32 = -1e6;
    let mut min_y: f32 = 1e6;
    let mut max_y: f32 = -1e6;

    for s in &segments {
        let r = s.radius + 0.004;
        min_x = min_x.min(s.x0 - r).min(s.x1 - r);
        max_x = max_x.max(s.x0 + r).max(s.x1 + r);
        min_y = min_y.min(s.y0 - r).min(s.y1 - r);
        max_y = max_y.max(s.y0 + r).max(s.y1 + r);
    }

    match badge {
        BadgePlate::None => {}
        BadgePlate::RoundedBox { cx, cy, half_w, half_h, .. } => {
            min_x = min_x.min(cx - half_w - 0.01);
            max_x = max_x.max(cx + half_w + 0.01);
            min_y = min_y.min(cy - half_h - 0.01);
            max_y = max_y.max(cy + half_h + 0.01);
        }
        BadgePlate::Circle { cx, cy, radius } => {
            min_x = min_x.min(cx - radius - 0.01);
            max_x = max_x.max(cx + radius + 0.01);
            min_y = min_y.min(cy - radius - 0.01);
            max_y = max_y.max(cy + radius + 0.01);
        }
        BadgePlate::Diamond { cx, cy, half_w, half_h } => {
            min_x = min_x.min(cx - half_w - 0.01);
            max_x = max_x.max(cx + half_w + 0.01);
            min_y = min_y.min(cy - half_h - 0.01);
            max_y = max_y.max(cy + half_h + 0.01);
        }
    }

    if min_x > max_x || min_y > max_y {
        return TextRasterBuffer {
            width: BUFFER_SIZE,
            height: BUFFER_SIZE,
            rgba,
            cache_key,
        };
    }

    // Convert aspect-corrected bounds relative to center into UV pixel ranges
    let inv_aspect = 1.0 / canvas_aspect.max(1e-4);
    let min_u = min_x * inv_aspect + params.center[0];
    let max_u = max_x * inv_aspect + params.center[0];
    let min_v = min_y + params.center[1];
    let max_v = max_y + params.center[1];

    let px_start = ((min_u * BUFFER_SIZE as f32).floor() as i32).clamp(0, BUFFER_SIZE as i32 - 1) as usize;
    let px_end = ((max_u * BUFFER_SIZE as f32).ceil() as i32).clamp(0, BUFFER_SIZE as i32) as usize;
    let py_start = ((min_v * BUFFER_SIZE as f32).floor() as i32).clamp(0, BUFFER_SIZE as i32 - 1) as usize;
    let py_end = ((max_v * BUFFER_SIZE as f32).ceil() as i32).clamp(0, BUFFER_SIZE as i32) as usize;

    let feather = (1.0 + canvas_aspect) / (2.0 * BUFFER_SIZE as f32) * 1.25;

    // Rasterize over the active pixel area
    for py in py_start..py_end {
        let v = (py as f32 + 0.5) / BUFFER_SIZE as f32;
        let y = v - params.center[1];
        let row_offset = py * (BUFFER_SIZE as usize) * 4;

        for px in px_start..px_end {
            let u = (px as f32 + 0.5) / BUFFER_SIZE as f32;
            let x = (u - params.center[0]) * canvas_aspect;

            // Compute minimum distance to foreground vector segments
            let mut min_fg_dist: f32 = 1e6;
            for s in &segments {
                let d = dist_to_segment(x, y, s.x0, s.y0, s.x1, s.y1) - s.radius;
                if d < min_fg_dist {
                    min_fg_dist = d;
                }
            }

            let fg_cov = (0.5 - min_fg_dist / feather).clamp(0.0, 1.0);

            // Compute distance to background badge plate
            let bg_dist = badge.distance(x, y);
            let bg_cov = (0.5 - bg_dist / feather).clamp(0.0, 1.0);

            let idx = row_offset + px * 4;
            rgba[idx] = (fg_cov * 255.0).round() as u8;
            rgba[idx + 1] = (bg_cov * 255.0).round() as u8;
            rgba[idx + 2] = 0;
            rgba[idx + 3] = 255;
        }
    }

    TextRasterBuffer {
        width: BUFFER_SIZE,
        height: BUFFER_SIZE,
        rgba,
        cache_key,
    }
}
