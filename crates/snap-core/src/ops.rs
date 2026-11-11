use serde::{Deserialize, Serialize};

use crate::curves::CurvesParams;
use crate::image_types::{HistogramData, LutColorSpace};

/// Identifiers for all supported Snap editing tools.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ToolId {
    RawDevelop,
    TuneImage,
    Details,
    Curves,
    WhiteBalance,
    ColorHsl,
    Lut3d,
    BlackAndWhite,
    Vignette,
    CropRotate,
}

impl ToolId {
    pub const ALL: &'static [ToolId] = &[
        ToolId::RawDevelop,
        ToolId::TuneImage,
        ToolId::Details,
        ToolId::Curves,
        ToolId::WhiteBalance,
        ToolId::ColorHsl,
        ToolId::Lut3d,
        ToolId::BlackAndWhite,
        ToolId::Vignette,
        ToolId::CropRotate,
    ];

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::RawDevelop => "RAW Develop",
            Self::TuneImage => "Tune Image",
            Self::Details => "Details",
            Self::Curves => "Curves",
            Self::WhiteBalance => "White Balance",
            Self::ColorHsl => "Color / HSL",
            Self::Lut3d => "3D LUT",
            Self::BlackAndWhite => "Black & White",
            Self::Vignette => "Vignette",
            Self::CropRotate => "Crop & Rotate",
        }
    }

    pub fn short_code(&self) -> &'static str {
        match self {
            Self::RawDevelop => "raw",
            Self::TuneImage => "tune",
            Self::Details => "det",
            Self::Curves => "crv",
            Self::WhiteBalance => "wb",
            Self::ColorHsl => "hsl",
            Self::Lut3d => "lut",
            Self::BlackAndWhite => "bw",
            Self::Vignette => "vig",
            Self::CropRotate => "crop",
        }
    }

    pub fn category(&self) -> &'static str {
        match self {
            Self::RawDevelop => "Develop",
            Self::TuneImage => "Tone",
            Self::Curves => "Tone",
            Self::Details => "Detail",
            Self::WhiteBalance => "Color",
            Self::ColorHsl => "Color",
            Self::Lut3d => "Creative",
            Self::BlackAndWhite => "Creative",
            Self::Vignette => "Creative",
            Self::CropRotate => "Geometry",
        }
    }
}

/// Metadata descriptor for a tool slider parameter (Snapseed gesture HUD).
#[derive(Clone, Debug, PartialEq)]
pub struct ParamDescriptor {
    pub name: &'static str,
    pub key: &'static str,
    pub min: f32,
    pub max: f32,
    pub default: f32,
    pub step: f32,
    pub unit: &'static str,
}

/// White balance presets.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum WbPreset {
    #[default]
    AsShot,
    Daylight,
    Cloudy,
    Shade,
    Tungsten,
    Fluorescent,
    Flash,
    Custom,
}

/// RAW development parameters.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct RawDevelopParams {
    pub exposure_ev: f32,
    pub highlights: f32,
    pub shadows: f32,
    pub contrast: f32,
    pub structure: f32,
    pub temperature: f32,
    pub tint: f32,
    pub saturation: f32,
    pub wb_preset: WbPreset,
}

impl Default for RawDevelopParams {
    fn default() -> Self {
        Self {
            exposure_ev: 0.0,
            highlights: 0.0,
            shadows: 0.0,
            contrast: 0.0,
            structure: 0.0,
            temperature: 0.0,
            tint: 0.0,
            saturation: 0.0,
            wb_preset: WbPreset::AsShot,
        }
    }
}

impl RawDevelopParams {
    pub fn is_identity(&self) -> bool {
        self.exposure_ev == 0.0
            && self.highlights == 0.0
            && self.shadows == 0.0
            && self.contrast == 0.0
            && self.structure == 0.0
            && self.temperature == 0.0
            && self.tint == 0.0
            && self.saturation == 0.0
            && self.wb_preset == WbPreset::AsShot
    }
}

/// White balance adjustment parameters.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
pub struct WhiteBalanceParams {
    pub temperature: f32, // -100..=100
    pub tint: f32,        // -100..=100
}

impl WhiteBalanceParams {
    pub fn is_identity(&self) -> bool {
        self.temperature == 0.0 && self.tint == 0.0
    }

    /// Calculate temperature and tint adjustments required to neutralize a chromatic cast
    /// sampled from a neutral gray target pixel in Linear Rec.2020.
    pub fn from_neutral_rec2020_sample(rgb: [f32; 3]) -> Self {
        let luma = crate::color::rec2020_luma(rgb);
        if luma <= 1e-4 {
            return Self::default();
        }

        let r = rgb[0] / luma;
        let g = rgb[1] / luma;
        let b = rgb[2] / luma;

        // Temperature: warm (r > b) -> shift cool (negative); cool (b > r) -> shift warm (positive)
        let temp_diff = b - r;
        let temperature = (temp_diff * 100.0).clamp(-100.0, 100.0);

        // Tint: Green vs Magenta axis.
        // Green cast (g > mid(r, b)) requires magenta shift (positive tint).
        // Magenta cast requires green shift (negative tint).
        let rb_mid = (r + b) * 0.5;
        let tint_diff = g - rb_mid;
        let tint = (tint_diff * 100.0).clamp(-100.0, 100.0);

        Self { temperature, tint }
    }
}

/// Tune Image parameters (Snapseed core tone & color adjustments).
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
pub struct TuneParams {
    pub brightness: f32, // -100..=100
    pub contrast: f32,   // -100..=100
    pub saturation: f32, // -100..=100
    pub ambiance: f32,   // -100..=100
    pub highlights: f32, // -100..=100
    pub shadows: f32,    // -100..=100
    pub warmth: f32,     // -100..=100
}

impl TuneParams {
    pub fn is_identity(&self) -> bool {
        self.brightness == 0.0
            && self.contrast == 0.0
            && self.saturation == 0.0
            && self.ambiance == 0.0
            && self.highlights == 0.0
            && self.shadows == 0.0
            && self.warmth == 0.0
    }

    /// Snapseed Magic Wand: Intelligent auto-enhancement from image histogram analysis.
    pub fn auto_from_histogram(hist: &HistogramData) -> Self {
        let total_pixels: u64 = hist.luma.iter().map(|&c| c as u64).sum();
        if total_pixels == 0 {
            return Self::default();
        }

        let p_target = |percentile: f32| -> usize {
            let count_target = (total_pixels as f32 * percentile) as u64;
            let mut accum = 0u64;
            for (i, &count) in hist.luma.iter().enumerate() {
                accum += count as u64;
                if accum >= count_target {
                    return i;
                }
            }
            255
        };

        let p5 = p_target(0.05) as f32;
        let p50 = p_target(0.50) as f32;
        let p95 = p_target(0.95) as f32;

        // Brightness correction toward balanced midpoint (128)
        let brightness = ((128.0 - p50) / 128.0 * 35.0).clamp(-40.0, 40.0);

        // Contrast boost if dynamic range is narrow
        let dr = p95 - p5;
        let contrast = if dr < 170.0 {
            ((170.0 - dr) / 170.0 * 25.0).clamp(0.0, 30.0)
        } else {
            0.0
        };

        let ambiance = 15.0;
        let shadows = if p5 < 25.0 { 20.0 } else { 5.0 };
        let highlights = if p95 > 230.0 { -20.0 } else { -5.0 };
        let saturation = 10.0;
        let warmth = 0.0;

        Self {
            brightness,
            contrast,
            saturation,
            ambiance,
            highlights,
            shadows,
            warmth,
        }
    }
}

/// Structure & sharpening parameters.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
pub struct DetailsParams {
    pub structure: f32,  // -100..=100
    pub sharpening: f32, // 0..=100
}

impl DetailsParams {
    pub fn is_identity(&self) -> bool {
        self.structure == 0.0 && self.sharpening == 0.0
    }
}

/// 8 Hue bands for selective color grading.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum HslBand {
    Red,
    Orange,
    Yellow,
    Green,
    Cyan,
    Blue,
    Purple,
    Magenta,
}

impl HslBand {
    pub const ALL: [HslBand; 8] = [
        HslBand::Red,
        HslBand::Orange,
        HslBand::Yellow,
        HslBand::Green,
        HslBand::Cyan,
        HslBand::Blue,
        HslBand::Purple,
        HslBand::Magenta,
    ];

    pub fn center_hue_deg(&self) -> f32 {
        match self {
            Self::Red => 0.0,
            Self::Orange => 30.0,
            Self::Yellow => 60.0,
            Self::Green => 120.0,
            Self::Cyan => 180.0,
            Self::Blue => 240.0,
            Self::Purple => 280.0,
            Self::Magenta => 320.0,
        }
    }
}

/// Selective shift for a single HSL band.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug, Default)]
pub struct HslChannelShift {
    pub hue: f32,        // -100..=100
    pub saturation: f32, // -100..=100
    pub luminance: f32,  // -100..=100
}

impl HslChannelShift {
    pub fn is_identity(&self) -> bool {
        self.hue == 0.0 && self.saturation == 0.0 && self.luminance == 0.0
    }
}

/// 8-band HSL color mixer parameters.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
pub struct ColorHslParams {
    pub bands: [HslChannelShift; 8],
}

impl ColorHslParams {
    pub fn is_identity(&self) -> bool {
        self.bands.iter().all(|b| b.is_identity())
    }
}

/// 3D LUT adjustment parameters.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Lut3dParams {
    pub lut_id: String,
    pub lut_name: String,
    pub strength: f32, // 0..=100
    pub input_space: LutColorSpace,
}

impl Default for Lut3dParams {
    fn default() -> Self {
        Self {
            lut_id: String::new(),
            lut_name: String::new(),
            strength: 100.0,
            input_space: LutColorSpace::Srgb,
        }
    }
}

impl Lut3dParams {
    pub fn is_identity(&self) -> bool {
        self.lut_id.is_empty() || self.strength <= 0.0
    }
}

/// Color filters for Black & White conversion.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum BwColorFilter {
    #[default]
    Neutral,
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
}

impl BwColorFilter {
    pub fn channel_weights(&self) -> [f32; 3] {
        match self {
            Self::Neutral => [0.2627, 0.6780, 0.0593],
            Self::Red => [0.75, 0.20, 0.05],
            Self::Orange => [0.55, 0.40, 0.05],
            Self::Yellow => [0.40, 0.55, 0.05],
            Self::Green => [0.15, 0.75, 0.10],
            Self::Blue => [0.10, 0.20, 0.70],
        }
    }
}

/// Black & White conversion presets.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum BwPreset {
    #[default]
    Neutral,
    Contrast,
    Bright,
    Dark,
    Film,
    DarkSky,
}

impl BwPreset {
    pub fn to_params(&self) -> BwParams {
        match self {
            Self::Neutral => BwParams {
                preset: *self,
                color_filter: BwColorFilter::Neutral,
                brightness: 0.0,
                contrast: 0.0,
                grain: 0.0,
            },
            Self::Contrast => BwParams {
                preset: *self,
                color_filter: BwColorFilter::Neutral,
                brightness: 0.0,
                contrast: 25.0,
                grain: 0.0,
            },
            Self::Bright => BwParams {
                preset: *self,
                color_filter: BwColorFilter::Neutral,
                brightness: 25.0,
                contrast: 10.0,
                grain: 0.0,
            },
            Self::Dark => BwParams {
                preset: *self,
                color_filter: BwColorFilter::Neutral,
                brightness: -20.0,
                contrast: 20.0,
                grain: 0.0,
            },
            Self::Film => BwParams {
                preset: *self,
                color_filter: BwColorFilter::Yellow,
                brightness: 5.0,
                contrast: 15.0,
                grain: 30.0,
            },
            Self::DarkSky => BwParams {
                preset: *self,
                color_filter: BwColorFilter::Red,
                brightness: -10.0,
                contrast: 35.0,
                grain: 10.0,
            },
        }
    }
}

/// Black & White tool parameters.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
pub struct BwParams {
    pub preset: BwPreset,
    pub color_filter: BwColorFilter,
    pub brightness: f32, // -100..=100
    pub contrast: f32,   // -100..=100
    pub grain: f32,      // 0..=100
}

impl BwParams {
    pub fn is_identity(&self) -> bool {
        false // B&W conversion always modifies color images
    }
}

/// Vignette tool parameters.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct VignetteParams {
    pub center: [f32; 2],
    pub inner_radius: f32,
    pub outer_radius: f32,
    pub outer_brightness: f32, // -100..=100
    pub inner_brightness: f32, // -100..=100
}

impl Default for VignetteParams {
    fn default() -> Self {
        Self {
            center: [0.5, 0.5],
            inner_radius: 0.25,
            outer_radius: 0.85,
            outer_brightness: 0.0,
            inner_brightness: 0.0,
        }
    }
}

impl VignetteParams {
    /// Default parameters when the Vignette tool is actively opened.
    pub fn initial_active() -> Self {
        Self {
            center: [0.5, 0.5],
            inner_radius: 0.25,
            outer_radius: 0.85,
            outer_brightness: -50.0,
            inner_brightness: 0.0,
        }
    }

    pub fn is_identity(&self) -> bool {
        self.outer_brightness == 0.0 && self.inner_brightness == 0.0
    }
}

/// Aspect ratio presets for cropping.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum AspectRatio {
    #[default]
    Free,
    Original,
    Square,
    Din,
    R3x2,
    R4x3,
    R5x4,
    R7x5,
    R16x9,
}

impl AspectRatio {
    pub fn ratio(&self, orig_w: u32, orig_h: u32, landscape: bool) -> Option<f32> {
        let (num, den) = match self {
            Self::Free => return None,
            Self::Original => (orig_w as f32, orig_h.max(1) as f32),
            Self::Square => (1.0, 1.0),
            Self::Din => (1.41421356, 1.0),
            Self::R3x2 => (3.0, 2.0),
            Self::R4x3 => (4.0, 3.0),
            Self::R5x4 => (5.0, 4.0),
            Self::R7x5 => (7.0, 5.0),
            Self::R16x9 => (16.0, 9.0),
        };
        let r = num / den;
        if landscape {
            Some(r.max(1.0 / r))
        } else {
            Some(r.min(1.0 / r))
        }
    }
}

/// Normalized crop rectangle `[0.0, 1.0]`.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
pub struct CropRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Default for CropRect {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
        }
    }
}

impl CropRect {
    pub fn is_identity(&self) -> bool {
        self.x.abs() < 1e-4
            && self.y.abs() < 1e-4
            && (self.width - 1.0).abs() < 1e-4
            && (self.height - 1.0).abs() < 1e-4
    }
}

/// Crop and rotation parameters.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct CropRotateParams {
    pub rect: CropRect,
    pub aspect: AspectRatio,
    pub aspect_landscape: bool,
    pub straighten_deg: f32, // -45..=45
    pub quarter_turns_cw: u8,
    pub flip_h: bool,
    pub flip_v: bool,
}

impl Default for CropRotateParams {
    fn default() -> Self {
        Self {
            rect: CropRect::default(),
            aspect: AspectRatio::Free,
            aspect_landscape: true,
            straighten_deg: 0.0,
            quarter_turns_cw: 0,
            flip_h: false,
            flip_v: false,
        }
    }
}

impl CropRotateParams {
    pub fn is_identity(&self) -> bool {
        self.rect.is_identity()
            && self.straighten_deg.abs() < 1e-4
            && self.quarter_turns_cw % 4 == 0
            && !self.flip_h
            && !self.flip_v
    }

    /// Exact analytical formula for the maximum axis-aligned rectangle of the given aspect ratio
    /// that fits inside a rectangle of dimensions `(aspect_w_over_h, 1.0)` rotated by `angle_deg`.
    /// Guaranteed so straighten never reveals black corners.
    pub fn inscribed_max_scale(aspect_w_over_h: f32, angle_deg: f32) -> f32 {
        let theta = angle_deg.abs().to_radians();
        let cos_t = theta.cos();
        let sin_t = theta.sin();
        let r = aspect_w_over_h.max(1e-4);
        let max_r = r.max(1.0 / r);
        let denom = cos_t + max_r * sin_t;
        if denom <= 1e-6 {
            1.0
        } else {
            (1.0 / denom).min(1.0)
        }
    }

    /// Constrain `self.rect` to stay within [0, 1] bounds and maintain `self.aspect` if set.
    pub fn constrain_rect(&mut self, orig_w: u32, orig_h: u32) {
        if let Some(desired_aspect) = self.aspect.ratio(orig_w, orig_h, self.aspect_landscape) {
            let img_aspect = (orig_w as f32) / (orig_h.max(1) as f32);
            let target_norm_aspect = desired_aspect / img_aspect;

            let cur_aspect = self.rect.width / self.rect.height.max(1e-6);
            if cur_aspect > target_norm_aspect {
                self.rect.width = (self.rect.height * target_norm_aspect).min(1.0);
            } else {
                self.rect.height = (self.rect.width / target_norm_aspect).min(1.0);
            }
        }

        self.rect.width = self.rect.width.clamp(0.01, 1.0);
        self.rect.height = self.rect.height.clamp(0.01, 1.0);

        if self.rect.x + self.rect.width > 1.0 {
            self.rect.x = (1.0 - self.rect.width).max(0.0);
        }
        if self.rect.y + self.rect.height > 1.0 {
            self.rect.y = (1.0 - self.rect.height).max(0.0);
        }
        self.rect.x = self.rect.x.clamp(0.0, 1.0);
        self.rect.y = self.rect.y.clamp(0.0, 1.0);
    }
}

/// Unified enum for any editing operation in the Snap processing pipeline.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum EditOp {
    RawDevelop(RawDevelopParams),
    TuneImage(TuneParams),
    Details(DetailsParams),
    Curves(CurvesParams),
    WhiteBalance(WhiteBalanceParams),
    ColorHsl(ColorHslParams),
    Lut3d(Lut3dParams),
    BlackAndWhite(BwParams),
    Vignette(VignetteParams),
    CropRotate(CropRotateParams),
}

impl EditOp {
    pub fn tool_id(&self) -> ToolId {
        match self {
            Self::RawDevelop(_) => ToolId::RawDevelop,
            Self::TuneImage(_) => ToolId::TuneImage,
            Self::Details(_) => ToolId::Details,
            Self::Curves(_) => ToolId::Curves,
            Self::WhiteBalance(_) => ToolId::WhiteBalance,
            Self::ColorHsl(_) => ToolId::ColorHsl,
            Self::Lut3d(_) => ToolId::Lut3d,
            Self::BlackAndWhite(_) => ToolId::BlackAndWhite,
            Self::Vignette(_) => ToolId::Vignette,
            Self::CropRotate(_) => ToolId::CropRotate,
        }
    }

    pub fn default_for_tool(tool: ToolId) -> Self {
        match tool {
            ToolId::RawDevelop => Self::RawDevelop(RawDevelopParams::default()),
            ToolId::TuneImage => Self::TuneImage(TuneParams::default()),
            ToolId::Details => Self::Details(DetailsParams::default()),
            ToolId::Curves => Self::Curves(CurvesParams::default()),
            ToolId::WhiteBalance => Self::WhiteBalance(WhiteBalanceParams::default()),
            ToolId::ColorHsl => Self::ColorHsl(ColorHslParams::default()),
            ToolId::Lut3d => Self::Lut3d(Lut3dParams::default()),
            ToolId::BlackAndWhite => Self::BlackAndWhite(BwParams::default()),
            ToolId::Vignette => Self::Vignette(VignetteParams::initial_active()),
            ToolId::CropRotate => Self::CropRotate(CropRotateParams::default()),
        }
    }

    pub fn is_identity(&self) -> bool {
        match self {
            Self::RawDevelop(p) => p.is_identity(),
            Self::TuneImage(p) => p.is_identity(),
            Self::Details(p) => p.is_identity(),
            Self::Curves(p) => p.is_identity(),
            Self::WhiteBalance(p) => p.is_identity(),
            Self::ColorHsl(p) => p.is_identity(),
            Self::Lut3d(p) => p.is_identity(),
            Self::BlackAndWhite(p) => p.is_identity(),
            Self::Vignette(p) => p.is_identity(),
            Self::CropRotate(p) => p.is_identity(),
        }
    }

    /// Retrieve the slider parameter descriptors and current values for this tool.
    pub fn slider_params(&self) -> Vec<(ParamDescriptor, f32)> {
        match self {
            Self::RawDevelop(p) => vec![
                (
                    ParamDescriptor {
                        name: "Exposure",
                        key: "exposure",
                        min: -8.0,
                        max: 8.0,
                        default: 0.0,
                        step: 0.1,
                        unit: "EV",
                    },
                    p.exposure_ev,
                ),
                (
                    ParamDescriptor {
                        name: "Highlights",
                        key: "highlights",
                        min: -100.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.highlights,
                ),
                (
                    ParamDescriptor {
                        name: "Shadows",
                        key: "shadows",
                        min: -100.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.shadows,
                ),
                (
                    ParamDescriptor {
                        name: "Contrast",
                        key: "contrast",
                        min: -100.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.contrast,
                ),
                (
                    ParamDescriptor {
                        name: "Structure",
                        key: "structure",
                        min: 0.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.structure,
                ),
                (
                    ParamDescriptor {
                        name: "Temperature",
                        key: "temperature",
                        min: -100.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.temperature,
                ),
                (
                    ParamDescriptor {
                        name: "Tint",
                        key: "tint",
                        min: -100.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.tint,
                ),
                (
                    ParamDescriptor {
                        name: "Saturation",
                        key: "saturation",
                        min: -100.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.saturation,
                ),
            ],
            Self::TuneImage(p) => vec![
                (
                    ParamDescriptor {
                        name: "Brightness",
                        key: "brightness",
                        min: -100.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.brightness,
                ),
                (
                    ParamDescriptor {
                        name: "Contrast",
                        key: "contrast",
                        min: -100.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.contrast,
                ),
                (
                    ParamDescriptor {
                        name: "Saturation",
                        key: "saturation",
                        min: -100.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.saturation,
                ),
                (
                    ParamDescriptor {
                        name: "Ambiance",
                        key: "ambiance",
                        min: -100.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.ambiance,
                ),
                (
                    ParamDescriptor {
                        name: "Highlights",
                        key: "highlights",
                        min: -100.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.highlights,
                ),
                (
                    ParamDescriptor {
                        name: "Shadows",
                        key: "shadows",
                        min: -100.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.shadows,
                ),
                (
                    ParamDescriptor {
                        name: "Warmth",
                        key: "warmth",
                        min: -100.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.warmth,
                ),
            ],
            Self::Details(p) => vec![
                (
                    ParamDescriptor {
                        name: "Structure",
                        key: "structure",
                        min: -100.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.structure,
                ),
                (
                    ParamDescriptor {
                        name: "Sharpening",
                        key: "sharpening",
                        min: 0.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.sharpening,
                ),
            ],
            Self::WhiteBalance(p) => vec![
                (
                    ParamDescriptor {
                        name: "Temperature",
                        key: "temperature",
                        min: -100.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.temperature,
                ),
                (
                    ParamDescriptor {
                        name: "Tint",
                        key: "tint",
                        min: -100.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.tint,
                ),
            ],
            Self::ColorHsl(p) => {
                let mut list = Vec::new();
                let names = [
                    "Red", "Orange", "Yellow", "Green", "Cyan", "Blue", "Purple", "Magenta",
                ];
                for (i, band_name) in names.iter().enumerate() {
                    list.push((
                        ParamDescriptor {
                            name: Box::leak(format!("{band_name} Hue").into_boxed_str()),
                            key: "hue",
                            min: -100.0,
                            max: 100.0,
                            default: 0.0,
                            step: 1.0,
                            unit: "",
                        },
                        p.bands[i].hue,
                    ));
                    list.push((
                        ParamDescriptor {
                            name: Box::leak(format!("{band_name} Saturation").into_boxed_str()),
                            key: "saturation",
                            min: -100.0,
                            max: 100.0,
                            default: 0.0,
                            step: 1.0,
                            unit: "",
                        },
                        p.bands[i].saturation,
                    ));
                    list.push((
                        ParamDescriptor {
                            name: Box::leak(format!("{band_name} Luminance").into_boxed_str()),
                            key: "luminance",
                            min: -100.0,
                            max: 100.0,
                            default: 0.0,
                            step: 1.0,
                            unit: "",
                        },
                        p.bands[i].luminance,
                    ));
                }
                list
            }
            Self::Lut3d(p) => vec![(
                ParamDescriptor {
                    name: "Strength",
                    key: "strength",
                    min: 0.0,
                    max: 100.0,
                    default: 100.0,
                    step: 1.0,
                    unit: "%",
                },
                p.strength,
            )],
            Self::BlackAndWhite(p) => vec![
                (
                    ParamDescriptor {
                        name: "Brightness",
                        key: "brightness",
                        min: -100.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.brightness,
                ),
                (
                    ParamDescriptor {
                        name: "Contrast",
                        key: "contrast",
                        min: -100.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.contrast,
                ),
                (
                    ParamDescriptor {
                        name: "Grain",
                        key: "grain",
                        min: 0.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.grain,
                ),
            ],
            Self::Vignette(p) => vec![
                (
                    ParamDescriptor {
                        name: "Outer Brightness",
                        key: "outer_brightness",
                        min: -100.0,
                        max: 100.0,
                        default: -50.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.outer_brightness,
                ),
                (
                    ParamDescriptor {
                        name: "Inner Brightness",
                        key: "inner_brightness",
                        min: -100.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.inner_brightness,
                ),
            ],
            Self::CropRotate(p) => vec![(
                ParamDescriptor {
                    name: "Straighten",
                    key: "straighten",
                    min: -45.0,
                    max: 45.0,
                    default: 0.0,
                    step: 0.1,
                    unit: "°",
                },
                p.straighten_deg,
            )],
            Self::Curves(_) => Vec::new(),
        }
    }

    /// Set a slider parameter value by index.
    pub fn set_slider_param(&mut self, index: usize, value: f32) {
        match self {
            Self::RawDevelop(p) => match index {
                0 => p.exposure_ev = value.clamp(-8.0, 8.0),
                1 => p.highlights = value.clamp(-100.0, 100.0),
                2 => p.shadows = value.clamp(-100.0, 100.0),
                3 => p.contrast = value.clamp(-100.0, 100.0),
                4 => p.structure = value.clamp(0.0, 100.0),
                5 => p.temperature = value.clamp(-100.0, 100.0),
                6 => p.tint = value.clamp(-100.0, 100.0),
                7 => p.saturation = value.clamp(-100.0, 100.0),
                _ => {}
            },
            Self::TuneImage(p) => match index {
                0 => p.brightness = value.clamp(-100.0, 100.0),
                1 => p.contrast = value.clamp(-100.0, 100.0),
                2 => p.saturation = value.clamp(-100.0, 100.0),
                3 => p.ambiance = value.clamp(-100.0, 100.0),
                4 => p.highlights = value.clamp(-100.0, 100.0),
                5 => p.shadows = value.clamp(-100.0, 100.0),
                6 => p.warmth = value.clamp(-100.0, 100.0),
                _ => {}
            },
            Self::Details(p) => match index {
                0 => p.structure = value.clamp(-100.0, 100.0),
                1 => p.sharpening = value.clamp(0.0, 100.0),
                _ => {}
            },
            Self::WhiteBalance(p) => match index {
                0 => p.temperature = value.clamp(-100.0, 100.0),
                1 => p.tint = value.clamp(-100.0, 100.0),
                _ => {}
            },
            Self::ColorHsl(p) => {
                let band_idx = index / 3;
                let sub_idx = index % 3;
                if band_idx < 8 {
                    let val = value.clamp(-100.0, 100.0);
                    match sub_idx {
                        0 => p.bands[band_idx].hue = val,
                        1 => p.bands[band_idx].saturation = val,
                        2 => p.bands[band_idx].luminance = val,
                        _ => {}
                    }
                }
            }
            Self::Lut3d(p) => {
                if index == 0 {
                    p.strength = value.clamp(0.0, 100.0);
                }
            }
            Self::BlackAndWhite(p) => match index {
                0 => p.brightness = value.clamp(-100.0, 100.0),
                1 => p.contrast = value.clamp(-100.0, 100.0),
                2 => p.grain = value.clamp(0.0, 100.0),
                _ => {}
            },
            Self::Vignette(p) => match index {
                0 => p.outer_brightness = value.clamp(-100.0, 100.0),
                1 => p.inner_brightness = value.clamp(-100.0, 100.0),
                _ => {}
            },
            Self::CropRotate(p) => {
                if index == 0 {
                    p.straighten_deg = value.clamp(-45.0, 45.0);
                }
            }
            Self::Curves(_) => {}
        }
    }
}
