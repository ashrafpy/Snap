use half::f16;
use serde::{Deserialize, Serialize};

/// A single control point on a 2D tone curve in normalized [0.0, 1.0] coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CurvePoint {
    pub x: f32,
    pub y: f32,
}

impl CurvePoint {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            x: x.clamp(0.0, 1.0),
            y: y.clamp(0.0, 1.0),
        }
    }
}

/// A single color/tone channel curve composed of sorted control points.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CurveChannel {
    pub points: Vec<CurvePoint>,
}

impl Default for CurveChannel {
    fn default() -> Self {
        Self {
            points: vec![CurvePoint::new(0.0, 0.0), CurvePoint::new(1.0, 1.0)],
        }
    }
}

impl CurveChannel {
    /// Create a curve channel with given control points, sorted by X and deduplicated.
    pub fn new(mut points: Vec<CurvePoint>) -> Self {
        if points.len() < 2 {
            return Self::default();
        }
        points.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal));
        // Deduplicate close x values within 1e-4
        let mut deduped: Vec<CurvePoint> = Vec::with_capacity(points.len());
        for p in points {
            if let Some(last) = deduped.last_mut() {
                if (last.x - p.x).abs() <= 1e-4 {
                    *last = p;
                    continue;
                }
            }
            deduped.push(p);
        }
        if deduped.len() < 2 {
            return Self::default();
        }
        Self { points: deduped }
    }

    /// Check if this curve is an identity mapping within 1e-4 tolerance.
    pub fn is_identity(&self) -> bool {
        // Sample at 17 points across [0, 1]
        for i in 0..=16 {
            let t = i as f32 / 16.0;
            if (self.eval(t) - t).abs() > 1e-4 {
                return false;
            }
        }
        true
    }

    /// Add a new point or update an existing point if within 1e-4 X distance.
    /// Returns the index of the point.
    pub fn add_or_move_point(&mut self, x: f32, y: f32) -> usize {
        let x = x.clamp(0.0, 1.0);
        let y = y.clamp(0.0, 1.0);

        for (idx, p) in self.points.iter_mut().enumerate() {
            if (p.x - x).abs() <= 1e-4 {
                p.x = x;
                p.y = y;
                return idx;
            }
        }

        self.points.push(CurvePoint::new(x, y));
        self.points
            .sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal));
        self.points
            .iter()
            .position(|p| (p.x - x).abs() <= 1e-4)
            .unwrap_or(0)
    }

    /// Remove a point at `index`. Endpoints (0 and last) or curves with <= 2 points cannot be removed.
    pub fn remove_point(&mut self, index: usize) -> bool {
        if self.points.len() <= 2 || index == 0 || index >= self.points.len() - 1 {
            return false;
        }
        self.points.remove(index);
        true
    }

    /// Evaluate the curve at `x` using a Fritsch–Carlson monotonic cubic Hermite spline.
    /// Guaranteed to be strictly monotonic without overshoot on monotonic inputs.
    pub fn eval(&self, x: f32) -> f32 {
        let x = x.clamp(0.0, 1.0);
        let n = self.points.len();
        if n == 0 {
            return x;
        }
        if n == 1 {
            return self.points[0].y;
        }
        if x <= self.points[0].x {
            return self.points[0].y;
        }
        if x >= self.points[n - 1].x {
            return self.points[n - 1].y;
        }

        // 1. Calculate secant slopes delta_k
        let mut deltas = Vec::with_capacity(n - 1);
        for k in 0..(n - 1) {
            let dx = self.points[k + 1].x - self.points[k].x;
            let dy = self.points[k + 1].y - self.points[k].y;
            if dx.abs() < 1e-7 {
                deltas.push(0.0);
            } else {
                deltas.push(dy / dx);
            }
        }

        // 2. Initial tangents m_k
        let mut m = vec![0.0f32; n];
        if n == 2 {
            m[0] = deltas[0];
            m[1] = deltas[0];
        } else {
            m[0] = deltas[0];
            m[n - 1] = deltas[n - 2];
            for k in 1..(n - 1) {
                if deltas[k - 1] * deltas[k] <= 0.0 {
                    m[k] = 0.0;
                } else {
                    m[k] = (deltas[k - 1] + deltas[k]) * 0.5;
                }
            }
        }

        // 3. Fritsch-Carlson monotonicity condition
        for k in 0..(n - 1) {
            let delta = deltas[k];
            if delta.abs() < 1e-7 {
                m[k] = 0.0;
                m[k + 1] = 0.0;
            } else {
                let alpha = m[k] / delta;
                let beta = m[k + 1] / delta;
                if alpha < 0.0 {
                    m[k] = 0.0;
                }
                if beta < 0.0 {
                    m[k + 1] = 0.0;
                }
                let hyp = alpha * alpha + beta * beta;
                if hyp > 9.0 {
                    let tau = 3.0 / hyp.sqrt();
                    m[k] = tau * alpha * delta;
                    m[k + 1] = tau * beta * delta;
                }
            }
        }

        // 4. Binary search interval [x_k, x_{k+1}]
        let mut low = 0;
        let mut high = n - 2;
        let mut k = 0;
        while low <= high {
            let mid = (low + high) / 2;
            if self.points[mid].x <= x {
                k = mid;
                low = mid + 1;
            } else {
                if mid == 0 {
                    break;
                }
                high = mid - 1;
            }
        }

        let h = self.points[k + 1].x - self.points[k].x;
        if h.abs() < 1e-7 {
            return self.points[k].y;
        }

        let t = (x - self.points[k].x) / h;
        let t2 = t * t;
        let t3 = t2 * t;

        // Hermite basis functions
        let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
        let h10 = t3 - 2.0 * t2 + t;
        let h01 = -2.0 * t3 + 3.0 * t2;
        let h11 = t3 - t2;

        let y = h00 * self.points[k].y
            + h10 * h * m[k]
            + h01 * self.points[k + 1].y
            + h11 * h * m[k + 1];

        y.clamp(0.0, 1.0)
    }
}

/// Identifiers for editable curve channels.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum CurveChannelId {
    Rgb,
    Red,
    Green,
    Blue,
    Luminance,
}

/// The 10 standard Curves presets.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum CurvePreset {
    Neutral,
    HardContrast,
    SoftContrast,
    Brighten,
    Darken,
    HighKey,
    LowKey,
    Fade,
    Punch,
    CrossProcess,
}

impl CurvePreset {
    pub const ALL: &'static [Self] = &[
        Self::Neutral,
        Self::HardContrast,
        Self::SoftContrast,
        Self::Brighten,
        Self::Darken,
        Self::HighKey,
        Self::LowKey,
        Self::Fade,
        Self::Punch,
        Self::CrossProcess,
    ];

    pub fn to_curves_params(&self) -> CurvesParams {
        let mut params = CurvesParams::default();
        match self {
            Self::Neutral => {}
            Self::HardContrast => {
                params.rgb = CurveChannel::new(vec![
                    CurvePoint::new(0.0, 0.0),
                    CurvePoint::new(0.22, 0.12),
                    CurvePoint::new(0.78, 0.88),
                    CurvePoint::new(1.0, 1.0),
                ]);
            }
            Self::SoftContrast => {
                params.rgb = CurveChannel::new(vec![
                    CurvePoint::new(0.0, 0.0),
                    CurvePoint::new(0.28, 0.22),
                    CurvePoint::new(0.72, 0.78),
                    CurvePoint::new(1.0, 1.0),
                ]);
            }
            Self::Brighten => {
                params.rgb = CurveChannel::new(vec![
                    CurvePoint::new(0.0, 0.0),
                    CurvePoint::new(0.45, 0.58),
                    CurvePoint::new(1.0, 1.0),
                ]);
            }
            Self::Darken => {
                params.rgb = CurveChannel::new(vec![
                    CurvePoint::new(0.0, 0.0),
                    CurvePoint::new(0.55, 0.42),
                    CurvePoint::new(1.0, 1.0),
                ]);
            }
            Self::HighKey => {
                params.rgb = CurveChannel::new(vec![
                    CurvePoint::new(0.0, 0.20),
                    CurvePoint::new(0.45, 0.68),
                    CurvePoint::new(1.0, 1.0),
                ]);
            }
            Self::LowKey => {
                params.rgb = CurveChannel::new(vec![
                    CurvePoint::new(0.0, 0.0),
                    CurvePoint::new(0.55, 0.32),
                    CurvePoint::new(1.0, 0.80),
                ]);
            }
            Self::Fade => {
                params.rgb = CurveChannel::new(vec![
                    CurvePoint::new(0.0, 0.15),
                    CurvePoint::new(1.0, 0.90),
                ]);
            }
            Self::Punch => {
                params.rgb = CurveChannel::new(vec![
                    CurvePoint::new(0.0, 0.0),
                    CurvePoint::new(0.18, 0.08),
                    CurvePoint::new(0.82, 0.92),
                    CurvePoint::new(1.0, 1.0),
                ]);
            }
            Self::CrossProcess => {
                params.rgb = CurveChannel::new(vec![
                    CurvePoint::new(0.0, 0.0),
                    CurvePoint::new(0.25, 0.20),
                    CurvePoint::new(0.75, 0.80),
                    CurvePoint::new(1.0, 1.0),
                ]);
                params.red = CurveChannel::new(vec![
                    CurvePoint::new(0.0, 0.05),
                    CurvePoint::new(0.45, 0.50),
                    CurvePoint::new(1.0, 0.92),
                ]);
                params.green = CurveChannel::new(vec![
                    CurvePoint::new(0.0, 0.0),
                    CurvePoint::new(0.50, 0.52),
                    CurvePoint::new(1.0, 1.0),
                ]);
                params.blue = CurveChannel::new(vec![
                    CurvePoint::new(0.0, 0.16),
                    CurvePoint::new(0.50, 0.44),
                    CurvePoint::new(1.0, 0.82),
                ]);
            }
        }
        params
    }
}

/// Curve parameters encompassing master RGB, individual channel curves, and luminance curve.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct CurvesParams {
    pub rgb: CurveChannel,
    pub red: CurveChannel,
    pub green: CurveChannel,
    pub blue: CurveChannel,
    pub luma: CurveChannel,
}

impl CurvesParams {
    /// Check if all curve channels are identity curves.
    pub fn is_identity(&self) -> bool {
        self.rgb.is_identity()
            && self.red.is_identity()
            && self.green.is_identity()
            && self.blue.is_identity()
            && self.luma.is_identity()
    }

    /// Bake the curves into a 1024-entry 1D LUT table of RGBA f16 vectors:
    /// `r = rgb(red(t))`, `g = rgb(green(t))`, `b = rgb(blue(t))`, `l = luma(t)`.
    pub fn bake_lut_1024(&self) -> Vec<[f16; 4]> {
        let mut table = Vec::with_capacity(1024);
        for i in 0..1024 {
            let t = i as f32 / 1023.0;
            let r = self.rgb.eval(self.red.eval(t));
            let g = self.rgb.eval(self.green.eval(t));
            let b = self.rgb.eval(self.blue.eval(t));
            let l = self.luma.eval(t);
            table.push([
                f16::from_f32(r),
                f16::from_f32(g),
                f16::from_f32(b),
                f16::from_f32(l),
            ]);
        }
        table
    }
}
