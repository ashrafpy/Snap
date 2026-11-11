use serde::{Deserialize, Serialize};

use crate::curves::CurvesParams;
use crate::image_types::{HistogramData, LutColorSpace};
pub use crate::ops_ext::*;

/// The 6 primary tool categories organizing all 35 Snap editing operations.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ToolCategory {
    CoreColor,
    Geometry,
    LocalRetouch,
    FilmCreative,
    BlurOptics,
    PortraitFinish,
}

impl ToolCategory {
    pub const ALL: &'static [ToolCategory] = &[
        ToolCategory::CoreColor,
        ToolCategory::Geometry,
        ToolCategory::LocalRetouch,
        ToolCategory::FilmCreative,
        ToolCategory::BlurOptics,
        ToolCategory::PortraitFinish,
    ];

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::CoreColor => "Core & Color",
            Self::Geometry => "Geometry",
            Self::LocalRetouch => "Local & Retouch",
            Self::FilmCreative => "Film & Creative",
            Self::BlurOptics => "Blur & Optics",
            Self::PortraitFinish => "Portrait & Finish",
        }
    }
}

/// Identifiers for all 35 supported Snap editing tools.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ToolId {
    // Core & Color (10)
    RawDevelop,
    TuneImage,
    Details,
    Curves,
    WhiteBalance,
    ColorHsl,
    Lut3d,
    ColorGrade,
    Dehaze,
    TonalContrast,

    // Geometry (3)
    CropRotate,
    Perspective,
    Expand,

    // Local & Retouch (3)
    Selective,
    Brush,
    Healing,

    // Film & Creative (13)
    BlackAndWhite,
    Vignette,
    Halation,
    Bloom,
    GrainyFilm,
    Noir,
    HdrScape,
    GlamourGlow,
    Drama,
    Vintage,
    DoubleExposure,
    Retrolux,
    Grunge,

    // Blur & Optics (2)
    LensBlur,
    MotionBlur,

    // Portrait & Finish (4)
    Portrait,
    HeadPose,
    Frames,
    TextOverlay,
}

impl ToolId {
    pub const ALL: &'static [ToolId] = &[
        // Core & Color
        ToolId::RawDevelop,
        ToolId::TuneImage,
        ToolId::Details,
        ToolId::Curves,
        ToolId::WhiteBalance,
        ToolId::ColorHsl,
        ToolId::Lut3d,
        ToolId::ColorGrade,
        ToolId::Dehaze,
        ToolId::TonalContrast,
        // Geometry
        ToolId::CropRotate,
        ToolId::Perspective,
        ToolId::Expand,
        // Local & Retouch
        ToolId::Selective,
        ToolId::Brush,
        ToolId::Healing,
        // Film & Creative
        ToolId::BlackAndWhite,
        ToolId::Vignette,
        ToolId::Halation,
        ToolId::Bloom,
        ToolId::GrainyFilm,
        ToolId::Noir,
        ToolId::HdrScape,
        ToolId::GlamourGlow,
        ToolId::Drama,
        ToolId::Vintage,
        ToolId::DoubleExposure,
        ToolId::Retrolux,
        ToolId::Grunge,
        // Blur & Optics
        ToolId::LensBlur,
        ToolId::MotionBlur,
        // Portrait & Finish
        ToolId::Portrait,
        ToolId::HeadPose,
        ToolId::Frames,
        ToolId::TextOverlay,
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
            Self::ColorGrade => "Color Grade",
            Self::Dehaze => "Dehaze",
            Self::TonalContrast => "Tonal Contrast",

            Self::CropRotate => "Crop & Rotate",
            Self::Perspective => "Perspective",
            Self::Expand => "Expand",

            Self::Selective => "Selective",
            Self::Brush => "Brush",
            Self::Healing => "Healing",

            Self::BlackAndWhite => "Black & White",
            Self::Vignette => "Vignette",
            Self::Halation => "Halation",
            Self::Bloom => "Bloom",
            Self::GrainyFilm => "Grainy Film",
            Self::Noir => "Noir",
            Self::HdrScape => "HDR Scape",
            Self::GlamourGlow => "Glamour Glow",
            Self::Drama => "Drama",
            Self::Vintage => "Vintage",
            Self::DoubleExposure => "Double Exposure",
            Self::Retrolux => "Retrolux",
            Self::Grunge => "Grunge",

            Self::LensBlur => "Lens Blur",
            Self::MotionBlur => "Motion Blur",

            Self::Portrait => "Portrait",
            Self::HeadPose => "Head Pose",
            Self::Frames => "Frames",
            Self::TextOverlay => "Text Overlay",
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
            Self::ColorGrade => "grade",
            Self::Dehaze => "dhz",
            Self::TonalContrast => "tonal",

            Self::CropRotate => "crop",
            Self::Perspective => "persp",
            Self::Expand => "expd",

            Self::Selective => "sel",
            Self::Brush => "brsh",
            Self::Healing => "heal",

            Self::BlackAndWhite => "bw",
            Self::Vignette => "vig",
            Self::Halation => "halo",
            Self::Bloom => "blm",
            Self::GrainyFilm => "film",
            Self::Noir => "noir",
            Self::HdrScape => "hdr",
            Self::GlamourGlow => "glow",
            Self::Drama => "drm",
            Self::Vintage => "vint",
            Self::DoubleExposure => "dbx",
            Self::Retrolux => "rlx",
            Self::Grunge => "grng",

            Self::LensBlur => "lblur",
            Self::MotionBlur => "mblur",

            Self::Portrait => "port",
            Self::HeadPose => "pose",
            Self::Frames => "frm",
            Self::TextOverlay => "txt",
        }
    }

    pub fn tool_category(&self) -> ToolCategory {
        match self {
            Self::RawDevelop
            | Self::TuneImage
            | Self::Details
            | Self::Curves
            | Self::WhiteBalance
            | Self::ColorHsl
            | Self::Lut3d
            | Self::ColorGrade
            | Self::Dehaze
            | Self::TonalContrast => ToolCategory::CoreColor,

            Self::CropRotate | Self::Perspective | Self::Expand => ToolCategory::Geometry,

            Self::Selective | Self::Brush | Self::Healing => ToolCategory::LocalRetouch,

            Self::BlackAndWhite
            | Self::Vignette
            | Self::Halation
            | Self::Bloom
            | Self::GrainyFilm
            | Self::Noir
            | Self::HdrScape
            | Self::GlamourGlow
            | Self::Drama
            | Self::Vintage
            | Self::DoubleExposure
            | Self::Retrolux
            | Self::Grunge => ToolCategory::FilmCreative,

            Self::LensBlur | Self::MotionBlur => ToolCategory::BlurOptics,

            Self::Portrait | Self::HeadPose | Self::Frames | Self::TextOverlay => {
                ToolCategory::PortraitFinish
            }
        }
    }

    pub fn category(&self) -> &'static str {
        self.tool_category().display_name()
    }

    pub fn in_category(cat: ToolCategory) -> Vec<ToolId> {
        Self::ALL
            .iter()
            .copied()
            .filter(|t| t.tool_category() == cat)
            .collect()
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

        let temp_diff = b - r;
        let temperature = (temp_diff * 100.0).clamp(-100.0, 100.0);

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

        let brightness = ((128.0 - p50) / 128.0 * 35.0).clamp(-40.0, 40.0);

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

/// Unified enum for any editing operation across all 35 Snap tools.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum EditOp {
    // Core & Color (10)
    RawDevelop(RawDevelopParams),
    TuneImage(TuneParams),
    Details(DetailsParams),
    Curves(CurvesParams),
    WhiteBalance(WhiteBalanceParams),
    ColorHsl(ColorHslParams),
    Lut3d(Lut3dParams),
    ColorGrade(ColorGradeParams),
    Dehaze(DehazeParams),
    TonalContrast(TonalContrastParams),

    // Geometry (3)
    CropRotate(CropRotateParams),
    Perspective(PerspectiveParams),
    Expand(ExpandParams),

    // Local & Retouch (3)
    Selective(SelectiveParams),
    Brush(BrushParams),
    Healing(HealingParams),

    // Film & Creative (13)
    BlackAndWhite(BwParams),
    Vignette(VignetteParams),
    Halation(HalationParams),
    Bloom(BloomParams),
    GrainyFilm(GrainyFilmParams),
    Noir(NoirParams),
    HdrScape(HdrScapeParams),
    GlamourGlow(GlamourGlowParams),
    Drama(DramaParams),
    Vintage(VintageParams),
    DoubleExposure(DoubleExposureParams),
    Retrolux(RetroluxParams),
    Grunge(GrungeParams),

    // Blur & Optics (2)
    LensBlur(LensBlurParams),
    MotionBlur(MotionBlurParams),

    // Portrait & Finish (4)
    Portrait(PortraitParams),
    HeadPose(HeadPoseParams),
    Frames(FramesParams),
    TextOverlay(TextOverlayParams),
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
            Self::ColorGrade(_) => ToolId::ColorGrade,
            Self::Dehaze(_) => ToolId::Dehaze,
            Self::TonalContrast(_) => ToolId::TonalContrast,

            Self::CropRotate(_) => ToolId::CropRotate,
            Self::Perspective(_) => ToolId::Perspective,
            Self::Expand(_) => ToolId::Expand,

            Self::Selective(_) => ToolId::Selective,
            Self::Brush(_) => ToolId::Brush,
            Self::Healing(_) => ToolId::Healing,

            Self::BlackAndWhite(_) => ToolId::BlackAndWhite,
            Self::Vignette(_) => ToolId::Vignette,
            Self::Halation(_) => ToolId::Halation,
            Self::Bloom(_) => ToolId::Bloom,
            Self::GrainyFilm(_) => ToolId::GrainyFilm,
            Self::Noir(_) => ToolId::Noir,
            Self::HdrScape(_) => ToolId::HdrScape,
            Self::GlamourGlow(_) => ToolId::GlamourGlow,
            Self::Drama(_) => ToolId::Drama,
            Self::Vintage(_) => ToolId::Vintage,
            Self::DoubleExposure(_) => ToolId::DoubleExposure,
            Self::Retrolux(_) => ToolId::Retrolux,
            Self::Grunge(_) => ToolId::Grunge,

            Self::LensBlur(_) => ToolId::LensBlur,
            Self::MotionBlur(_) => ToolId::MotionBlur,

            Self::Portrait(_) => ToolId::Portrait,
            Self::HeadPose(_) => ToolId::HeadPose,
            Self::Frames(_) => ToolId::Frames,
            Self::TextOverlay(_) => ToolId::TextOverlay,
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
            ToolId::ColorGrade => Self::ColorGrade(ColorGradeParams::default()),
            ToolId::Dehaze => Self::Dehaze(DehazeParams::initial_active()),
            ToolId::TonalContrast => Self::TonalContrast(TonalContrastParams::initial_active()),

            ToolId::CropRotate => Self::CropRotate(CropRotateParams::default()),
            ToolId::Perspective => Self::Perspective(PerspectiveParams::default()),
            ToolId::Expand => Self::Expand(ExpandParams::default()),

            ToolId::Selective => Self::Selective(SelectiveParams::initial_active()),
            ToolId::Brush => Self::Brush(BrushParams::default()),
            ToolId::Healing => Self::Healing(HealingParams::default()),

            ToolId::BlackAndWhite => Self::BlackAndWhite(BwParams::default()),
            ToolId::Vignette => Self::Vignette(VignetteParams::initial_active()),
            ToolId::Halation => Self::Halation(HalationParams::initial_active()),
            ToolId::Bloom => Self::Bloom(BloomParams::initial_active()),
            ToolId::GrainyFilm => Self::GrainyFilm(GrainyFilmParams::initial_active()),
            ToolId::Noir => Self::Noir(NoirParams::initial_active()),
            ToolId::HdrScape => Self::HdrScape(HdrScapeParams::initial_active()),
            ToolId::GlamourGlow => Self::GlamourGlow(GlamourGlowParams::initial_active()),
            ToolId::Drama => Self::Drama(DramaParams::initial_active()),
            ToolId::Vintage => Self::Vintage(VintageParams::initial_active()),
            ToolId::DoubleExposure => Self::DoubleExposure(DoubleExposureParams::initial_active()),
            ToolId::Retrolux => Self::Retrolux(RetroluxParams::initial_active()),
            ToolId::Grunge => Self::Grunge(GrungeParams::initial_active()),

            ToolId::LensBlur => Self::LensBlur(LensBlurParams::initial_active()),
            ToolId::MotionBlur => Self::MotionBlur(MotionBlurParams::initial_active()),

            ToolId::Portrait => Self::Portrait(PortraitParams::initial_active()),
            ToolId::HeadPose => Self::HeadPose(HeadPoseParams::initial_active()),
            ToolId::Frames => Self::Frames(FramesParams::initial_active()),
            ToolId::TextOverlay => Self::TextOverlay(TextOverlayParams::initial_active()),
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
            Self::ColorGrade(p) => p.is_identity(),
            Self::Dehaze(p) => p.is_identity(),
            Self::TonalContrast(p) => p.is_identity(),

            Self::CropRotate(p) => p.is_identity(),
            Self::Perspective(p) => p.is_identity(),
            Self::Expand(p) => p.is_identity(),

            Self::Selective(p) => p.is_identity(),
            Self::Brush(p) => p.is_identity(),
            Self::Healing(p) => p.is_identity(),

            Self::BlackAndWhite(p) => p.is_identity(),
            Self::Vignette(p) => p.is_identity(),
            Self::Halation(p) => p.is_identity(),
            Self::Bloom(p) => p.is_identity(),
            Self::GrainyFilm(p) => p.is_identity(),
            Self::Noir(p) => p.is_identity(),
            Self::HdrScape(p) => p.is_identity(),
            Self::GlamourGlow(p) => p.is_identity(),
            Self::Drama(p) => p.is_identity(),
            Self::Vintage(p) => p.is_identity(),
            Self::DoubleExposure(p) => p.is_identity(),
            Self::Retrolux(p) => p.is_identity(),
            Self::Grunge(p) => p.is_identity(),

            Self::LensBlur(p) => p.is_identity(),
            Self::MotionBlur(p) => p.is_identity(),

            Self::Portrait(p) => p.is_identity(),
            Self::HeadPose(p) => p.is_identity(),
            Self::Frames(p) => p.is_identity(),
            Self::TextOverlay(p) => p.is_identity(),
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
            Self::Curves(_) => Vec::new(),
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
            Self::ColorGrade(p) => {
                let wheel = match p.active_wheel {
                    ColorWheelZone::Shadows => &p.shadows,
                    ColorWheelZone::Midtones => &p.midtones,
                    ColorWheelZone::Highlights => &p.highlights,
                    ColorWheelZone::Global => &p.global,
                };
                vec![
                    (
                        ParamDescriptor {
                            name: "Hue",
                            key: "hue",
                            min: 0.0,
                            max: 360.0,
                            default: 0.0,
                            step: 1.0,
                            unit: "deg",
                        },
                        wheel.hue_deg,
                    ),
                    (
                        ParamDescriptor {
                            name: "Saturation",
                            key: "saturation",
                            min: 0.0,
                            max: 100.0,
                            default: 0.0,
                            step: 1.0,
                            unit: "%",
                        },
                        wheel.saturation,
                    ),
                    (
                        ParamDescriptor {
                            name: "Luminance",
                            key: "luminance",
                            min: -100.0,
                            max: 100.0,
                            default: 0.0,
                            step: 1.0,
                            unit: "",
                        },
                        wheel.luminance,
                    ),
                    (
                        ParamDescriptor {
                            name: "Balance",
                            key: "balance",
                            min: -100.0,
                            max: 100.0,
                            default: 0.0,
                            step: 1.0,
                            unit: "",
                        },
                        p.balance,
                    ),
                    (
                        ParamDescriptor {
                            name: "Blending",
                            key: "blending",
                            min: 0.0,
                            max: 100.0,
                            default: 50.0,
                            step: 1.0,
                            unit: "%",
                        },
                        p.blending,
                    ),
                ]
            }
            Self::Dehaze(p) => vec![
                (
                    ParamDescriptor {
                        name: "Strength",
                        key: "strength",
                        min: -100.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.strength,
                ),
                (
                    ParamDescriptor {
                        name: "Warmth Offset",
                        key: "warmth_offset",
                        min: -100.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.warmth_offset,
                ),
            ],
            Self::TonalContrast(p) => vec![
                (
                    ParamDescriptor {
                        name: "High Tones",
                        key: "high_tones",
                        min: -100.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.high_tones,
                ),
                (
                    ParamDescriptor {
                        name: "Mid Tones",
                        key: "mid_tones",
                        min: -100.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.mid_tones,
                ),
                (
                    ParamDescriptor {
                        name: "Low Tones",
                        key: "low_tones",
                        min: -100.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.low_tones,
                ),
                (
                    ParamDescriptor {
                        name: "Protect Shadows",
                        key: "protect_shadows",
                        min: 0.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.protect_shadows,
                ),
                (
                    ParamDescriptor {
                        name: "Protect Highlights",
                        key: "protect_highlights",
                        min: 0.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.protect_highlights,
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
                    unit: "deg",
                },
                p.straighten_deg,
            )],
            Self::Perspective(p) => vec![
                (
                    ParamDescriptor {
                        name: "Vertical",
                        key: "vertical",
                        min: -100.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.vertical,
                ),
                (
                    ParamDescriptor {
                        name: "Horizontal",
                        key: "horizontal",
                        min: -100.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.horizontal,
                ),
                (
                    ParamDescriptor {
                        name: "Rotate",
                        key: "rotate",
                        min: -45.0,
                        max: 45.0,
                        default: 0.0,
                        step: 0.1,
                        unit: "deg",
                    },
                    p.rotate_deg,
                ),
                (
                    ParamDescriptor {
                        name: "Scale",
                        key: "scale",
                        min: 0.5,
                        max: 2.0,
                        default: 1.0,
                        step: 0.01,
                        unit: "",
                    },
                    p.scale,
                ),
            ],
            Self::Expand(p) => vec![
                (
                    ParamDescriptor {
                        name: "Top",
                        key: "top",
                        min: 0.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "%",
                    },
                    p.top * 100.0,
                ),
                (
                    ParamDescriptor {
                        name: "Bottom",
                        key: "bottom",
                        min: 0.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "%",
                    },
                    p.bottom * 100.0,
                ),
                (
                    ParamDescriptor {
                        name: "Left",
                        key: "left",
                        min: 0.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "%",
                    },
                    p.left * 100.0,
                ),
                (
                    ParamDescriptor {
                        name: "Right",
                        key: "right",
                        min: 0.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "%",
                    },
                    p.right * 100.0,
                ),
            ],
            Self::Selective(p) => {
                if p.active_pin < p.pins.len() {
                    let pin = &p.pins[p.active_pin];
                    vec![
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
                            pin.brightness,
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
                            pin.contrast,
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
                            pin.saturation,
                        ),
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
                            pin.structure,
                        ),
                        (
                            ParamDescriptor {
                                name: "Radius",
                                key: "radius",
                                min: 1.0,
                                max: 100.0,
                                default: 25.0,
                                step: 1.0,
                                unit: "%",
                            },
                            pin.radius * 100.0,
                        ),
                    ]
                } else {
                    Vec::new()
                }
            }
            Self::Brush(p) => vec![
                (
                    ParamDescriptor {
                        name: "Radius",
                        key: "radius",
                        min: 1.0,
                        max: 50.0,
                        default: 10.0,
                        step: 1.0,
                        unit: "%",
                    },
                    p.radius * 100.0,
                ),
                (
                    ParamDescriptor {
                        name: "Feather",
                        key: "feather",
                        min: 0.0,
                        max: 100.0,
                        default: 50.0,
                        step: 1.0,
                        unit: "%",
                    },
                    p.feather * 100.0,
                ),
                (
                    ParamDescriptor {
                        name: "Step Intensity",
                        key: "step",
                        min: -100.0,
                        max: 100.0,
                        default: 10.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.step,
                ),
            ],
            Self::Healing(p) => vec![(
                ParamDescriptor {
                    name: "Brush Radius",
                    key: "brush_radius",
                    min: 1.0,
                    max: 50.0,
                    default: 5.0,
                    step: 1.0,
                    unit: "%",
                },
                p.brush_radius * 100.0,
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
            Self::Halation(p) => vec![
                (
                    ParamDescriptor {
                        name: "Threshold",
                        key: "threshold",
                        min: 0.0,
                        max: 100.0,
                        default: 80.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.threshold,
                ),
                (
                    ParamDescriptor {
                        name: "Strength",
                        key: "strength",
                        min: 0.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.strength,
                ),
                (
                    ParamDescriptor {
                        name: "Spread",
                        key: "spread",
                        min: 0.0,
                        max: 100.0,
                        default: 40.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.spread,
                ),
            ],
            Self::Bloom(p) => vec![
                (
                    ParamDescriptor {
                        name: "Strength",
                        key: "strength",
                        min: 0.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.strength,
                ),
                (
                    ParamDescriptor {
                        name: "Size",
                        key: "size",
                        min: 0.0,
                        max: 100.0,
                        default: 30.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.size,
                ),
                (
                    ParamDescriptor {
                        name: "Highlight Range",
                        key: "highlight_range",
                        min: 0.0,
                        max: 100.0,
                        default: 70.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.highlight_range,
                ),
            ],
            Self::GrainyFilm(p) => vec![
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
                (
                    ParamDescriptor {
                        name: "Style Strength",
                        key: "style_strength",
                        min: 0.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.style_strength,
                ),
            ],
            Self::Noir(p) => vec![
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
                        name: "Wash",
                        key: "wash",
                        min: 0.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.wash,
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
            Self::HdrScape(p) => vec![
                (
                    ParamDescriptor {
                        name: "Filter Strength",
                        key: "filter_strength",
                        min: 0.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.filter_strength,
                ),
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
            Self::GlamourGlow(p) => vec![
                (
                    ParamDescriptor {
                        name: "Glow",
                        key: "glow",
                        min: 0.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.glow,
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
            Self::Drama(p) => vec![
                (
                    ParamDescriptor {
                        name: "Filter Strength",
                        key: "filter_strength",
                        min: 0.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.filter_strength,
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
            Self::Vintage(p) => vec![
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
                        name: "Style Strength",
                        key: "style_strength",
                        min: 0.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.style_strength,
                ),
                (
                    ParamDescriptor {
                        name: "Vignette Strength",
                        key: "vignette_strength",
                        min: 0.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.vignette_strength,
                ),
            ],
            Self::DoubleExposure(p) => vec![(
                ParamDescriptor {
                    name: "Opacity",
                    key: "opacity",
                    min: 0.0,
                    max: 100.0,
                    default: 0.0,
                    step: 1.0,
                    unit: "%",
                },
                p.opacity,
            )],
            Self::Retrolux(p) => vec![
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
                        name: "Style Strength",
                        key: "style_strength",
                        min: 0.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.style_strength,
                ),
                (
                    ParamDescriptor {
                        name: "Scratches",
                        key: "scratches",
                        min: 0.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.scratches,
                ),
                (
                    ParamDescriptor {
                        name: "Light Leaks",
                        key: "light_leaks",
                        min: 0.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.light_leaks,
                ),
            ],
            Self::Grunge(p) => vec![
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
                        name: "Texture Strength",
                        key: "texture_strength",
                        min: 0.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.texture_strength,
                ),
            ],
            Self::LensBlur(p) => vec![
                (
                    ParamDescriptor {
                        name: "Blur Strength",
                        key: "blur_strength",
                        min: 0.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.blur_strength,
                ),
                (
                    ParamDescriptor {
                        name: "Transition",
                        key: "transition",
                        min: 0.0,
                        max: 100.0,
                        default: 35.0,
                        step: 1.0,
                        unit: "%",
                    },
                    p.transition * 100.0,
                ),
                (
                    ParamDescriptor {
                        name: "Bokeh Boost",
                        key: "bokeh_boost",
                        min: 0.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.bokeh_boost,
                ),
                (
                    ParamDescriptor {
                        name: "Vignette Strength",
                        key: "vignette_strength",
                        min: 0.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.vignette_strength,
                ),
            ],
            Self::MotionBlur(p) => vec![
                (
                    ParamDescriptor {
                        name: "Strength",
                        key: "strength",
                        min: 0.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.strength,
                ),
                (
                    ParamDescriptor {
                        name: "Angle",
                        key: "angle",
                        min: -180.0,
                        max: 180.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "deg",
                    },
                    p.angle_deg,
                ),
            ],
            Self::Portrait(p) => vec![
                (
                    ParamDescriptor {
                        name: "Face Spotlight",
                        key: "face_spotlight",
                        min: 0.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.face_spotlight,
                ),
                (
                    ParamDescriptor {
                        name: "Skin Smoothing",
                        key: "skin_smoothing",
                        min: 0.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.skin_smoothing,
                ),
                (
                    ParamDescriptor {
                        name: "Eye Clarity",
                        key: "eye_clarity",
                        min: 0.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.eye_clarity,
                ),
            ],
            Self::HeadPose(p) => vec![
                (
                    ParamDescriptor {
                        name: "Yaw",
                        key: "yaw_x",
                        min: -100.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.yaw_x,
                ),
                (
                    ParamDescriptor {
                        name: "Pitch",
                        key: "pitch_y",
                        min: -100.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.pitch_y,
                ),
                (
                    ParamDescriptor {
                        name: "Smile",
                        key: "smile",
                        min: -100.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.smile,
                ),
                (
                    ParamDescriptor {
                        name: "Focal Length",
                        key: "focal_length",
                        min: -100.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "",
                    },
                    p.focal_length,
                ),
            ],
            Self::Frames(p) => vec![
                (
                    ParamDescriptor {
                        name: "Width",
                        key: "width",
                        min: 0.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "%",
                    },
                    p.width,
                ),
                (
                    ParamDescriptor {
                        name: "Aspect Pad",
                        key: "aspect_pad",
                        min: 0.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "%",
                    },
                    p.aspect_pad,
                ),
            ],
            Self::TextOverlay(p) => vec![
                (
                    ParamDescriptor {
                        name: "Scale",
                        key: "scale",
                        min: 10.0,
                        max: 300.0,
                        default: 100.0,
                        step: 1.0,
                        unit: "%",
                    },
                    p.scale * 100.0,
                ),
                (
                    ParamDescriptor {
                        name: "Opacity",
                        key: "opacity",
                        min: 0.0,
                        max: 100.0,
                        default: 0.0,
                        step: 1.0,
                        unit: "%",
                    },
                    p.opacity,
                ),
            ],
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
            Self::ColorGrade(p) => {
                let wheel = match p.active_wheel {
                    ColorWheelZone::Shadows => &mut p.shadows,
                    ColorWheelZone::Midtones => &mut p.midtones,
                    ColorWheelZone::Highlights => &mut p.highlights,
                    ColorWheelZone::Global => &mut p.global,
                };
                match index {
                    0 => wheel.hue_deg = value.clamp(0.0, 360.0),
                    1 => wheel.saturation = value.clamp(0.0, 100.0),
                    2 => wheel.luminance = value.clamp(-100.0, 100.0),
                    3 => p.balance = value.clamp(-100.0, 100.0),
                    4 => p.blending = value.clamp(0.0, 100.0),
                    _ => {}
                }
            }
            Self::Dehaze(p) => match index {
                0 => p.strength = value.clamp(-100.0, 100.0),
                1 => p.warmth_offset = value.clamp(-100.0, 100.0),
                _ => {}
            },
            Self::TonalContrast(p) => match index {
                0 => p.high_tones = value.clamp(-100.0, 100.0),
                1 => p.mid_tones = value.clamp(-100.0, 100.0),
                2 => p.low_tones = value.clamp(-100.0, 100.0),
                3 => p.protect_shadows = value.clamp(0.0, 100.0),
                4 => p.protect_highlights = value.clamp(0.0, 100.0),
                _ => {}
            },
            Self::CropRotate(p) => {
                if index == 0 {
                    p.straighten_deg = value.clamp(-45.0, 45.0);
                }
            }
            Self::Perspective(p) => match index {
                0 => p.vertical = value.clamp(-100.0, 100.0),
                1 => p.horizontal = value.clamp(-100.0, 100.0),
                2 => p.rotate_deg = value.clamp(-45.0, 45.0),
                3 => p.scale = value.clamp(0.5, 2.0),
                _ => {}
            },
            Self::Expand(p) => match index {
                0 => p.top = (value / 100.0).clamp(0.0, 1.0),
                1 => p.bottom = (value / 100.0).clamp(0.0, 1.0),
                2 => p.left = (value / 100.0).clamp(0.0, 1.0),
                3 => p.right = (value / 100.0).clamp(0.0, 1.0),
                _ => {}
            },
            Self::Selective(p) => {
                if p.active_pin < p.pins.len() {
                    let pin = &mut p.pins[p.active_pin];
                    match index {
                        0 => pin.brightness = value.clamp(-100.0, 100.0),
                        1 => pin.contrast = value.clamp(-100.0, 100.0),
                        2 => pin.saturation = value.clamp(-100.0, 100.0),
                        3 => pin.structure = value.clamp(-100.0, 100.0),
                        4 => pin.radius = (value / 100.0).clamp(0.01, 1.0),
                        _ => {}
                    }
                }
            }
            Self::Brush(p) => match index {
                0 => p.radius = (value / 100.0).clamp(0.01, 0.5),
                1 => p.feather = (value / 100.0).clamp(0.0, 1.0),
                2 => p.step = value.clamp(-100.0, 100.0),
                _ => {}
            },
            Self::Healing(p) => {
                if index == 0 {
                    p.brush_radius = (value / 100.0).clamp(0.01, 0.5);
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
            Self::Halation(p) => match index {
                0 => p.threshold = value.clamp(0.0, 100.0),
                1 => p.strength = value.clamp(0.0, 100.0),
                2 => p.spread = value.clamp(0.0, 100.0),
                _ => {}
            },
            Self::Bloom(p) => match index {
                0 => p.strength = value.clamp(0.0, 100.0),
                1 => p.size = value.clamp(0.0, 100.0),
                2 => p.highlight_range = value.clamp(0.0, 100.0),
                _ => {}
            },
            Self::GrainyFilm(p) => match index {
                0 => p.grain = value.clamp(0.0, 100.0),
                1 => p.style_strength = value.clamp(0.0, 100.0),
                _ => {}
            },
            Self::Noir(p) => match index {
                0 => p.brightness = value.clamp(-100.0, 100.0),
                1 => p.contrast = value.clamp(-100.0, 100.0),
                2 => p.wash = value.clamp(0.0, 100.0),
                3 => p.grain = value.clamp(0.0, 100.0),
                _ => {}
            },
            Self::HdrScape(p) => match index {
                0 => p.filter_strength = value.clamp(0.0, 100.0),
                1 => p.brightness = value.clamp(-100.0, 100.0),
                2 => p.saturation = value.clamp(-100.0, 100.0),
                _ => {}
            },
            Self::GlamourGlow(p) => match index {
                0 => p.glow = value.clamp(0.0, 100.0),
                1 => p.saturation = value.clamp(-100.0, 100.0),
                2 => p.warmth = value.clamp(-100.0, 100.0),
                _ => {}
            },
            Self::Drama(p) => match index {
                0 => p.filter_strength = value.clamp(0.0, 100.0),
                1 => p.saturation = value.clamp(-100.0, 100.0),
                _ => {}
            },
            Self::Vintage(p) => match index {
                0 => p.brightness = value.clamp(-100.0, 100.0),
                1 => p.saturation = value.clamp(-100.0, 100.0),
                2 => p.style_strength = value.clamp(0.0, 100.0),
                3 => p.vignette_strength = value.clamp(0.0, 100.0),
                _ => {}
            },
            Self::DoubleExposure(p) => {
                if index == 0 {
                    p.opacity = value.clamp(0.0, 100.0);
                }
            }
            Self::Retrolux(p) => match index {
                0 => p.brightness = value.clamp(-100.0, 100.0),
                1 => p.saturation = value.clamp(-100.0, 100.0),
                2 => p.contrast = value.clamp(-100.0, 100.0),
                3 => p.style_strength = value.clamp(0.0, 100.0),
                4 => p.scratches = value.clamp(0.0, 100.0),
                5 => p.light_leaks = value.clamp(0.0, 100.0),
                _ => {}
            },
            Self::Grunge(p) => match index {
                0 => p.brightness = value.clamp(-100.0, 100.0),
                1 => p.contrast = value.clamp(-100.0, 100.0),
                2 => p.saturation = value.clamp(-100.0, 100.0),
                3 => p.texture_strength = value.clamp(0.0, 100.0),
                _ => {}
            },
            Self::LensBlur(p) => match index {
                0 => p.blur_strength = value.clamp(0.0, 100.0),
                1 => p.transition = (value / 100.0).clamp(0.0, 1.0),
                2 => p.bokeh_boost = value.clamp(0.0, 100.0),
                3 => p.vignette_strength = value.clamp(0.0, 100.0),
                _ => {}
            },
            Self::MotionBlur(p) => match index {
                0 => p.strength = value.clamp(0.0, 100.0),
                1 => p.angle_deg = value.clamp(-180.0, 180.0),
                _ => {}
            },
            Self::Portrait(p) => match index {
                0 => p.face_spotlight = value.clamp(0.0, 100.0),
                1 => p.skin_smoothing = value.clamp(0.0, 100.0),
                2 => p.eye_clarity = value.clamp(0.0, 100.0),
                _ => {}
            },
            Self::HeadPose(p) => match index {
                0 => p.yaw_x = value.clamp(-100.0, 100.0),
                1 => p.pitch_y = value.clamp(-100.0, 100.0),
                2 => p.smile = value.clamp(-100.0, 100.0),
                3 => p.focal_length = value.clamp(-100.0, 100.0),
                _ => {}
            },
            Self::Frames(p) => match index {
                0 => p.width = value.clamp(0.0, 100.0),
                1 => p.aspect_pad = value.clamp(0.0, 100.0),
                _ => {}
            },
            Self::TextOverlay(p) => match index {
                0 => p.scale = (value / 100.0).clamp(0.1, 3.0),
                1 => p.opacity = value.clamp(0.0, 100.0),
                _ => {}
            },
            Self::Curves(_) => {}
        }
    }
}
