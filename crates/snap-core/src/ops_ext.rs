use serde::{Deserialize, Serialize};

use crate::mask::BrushMap;

// ----------------------------------------------------------------------------
// 1. Color Grading
// ----------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum ColorWheelZone {
    #[default]
    Shadows,
    Midtones,
    Highlights,
    Global,
}

impl ColorWheelZone {
    pub const ALL: &'static [Self] = &[
        Self::Shadows,
        Self::Midtones,
        Self::Highlights,
        Self::Global,
    ];

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Shadows => "Shadows",
            Self::Midtones => "Midtones",
            Self::Highlights => "Highlights",
            Self::Global => "Global",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug, Default)]
pub struct ColorWheelValue {
    pub hue_deg: f32,    // 0.0..=360.0
    pub saturation: f32, // 0.0..=100.0
    pub luminance: f32,  // -100.0..=100.0
}

impl ColorWheelValue {
    pub fn is_identity(&self) -> bool {
        self.saturation == 0.0 && self.luminance == 0.0
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct ColorGradeParams {
    pub shadows: ColorWheelValue,
    pub midtones: ColorWheelValue,
    pub highlights: ColorWheelValue,
    pub global: ColorWheelValue,
    pub balance: f32,  // -100.0..=100.0
    pub blending: f32, // 0.0..=100.0
    pub active_wheel: ColorWheelZone,
}

impl Default for ColorGradeParams {
    fn default() -> Self {
        Self {
            shadows: ColorWheelValue::default(),
            midtones: ColorWheelValue::default(),
            highlights: ColorWheelValue::default(),
            global: ColorWheelValue::default(),
            balance: 0.0,
            blending: 50.0,
            active_wheel: ColorWheelZone::Shadows,
        }
    }
}

impl ColorGradeParams {
    pub fn is_identity(&self) -> bool {
        self.shadows.is_identity()
            && self.midtones.is_identity()
            && self.highlights.is_identity()
            && self.global.is_identity()
    }
}

// ----------------------------------------------------------------------------
// 2. Dehaze
// ----------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
pub struct DehazeParams {
    pub strength: f32,      // -100.0..=100.0
    pub warmth_offset: f32, // -100.0..=100.0
}

impl DehazeParams {
    pub fn is_identity(&self) -> bool {
        self.strength == 0.0 && self.warmth_offset == 0.0
    }

    pub fn initial_active() -> Self {
        Self {
            strength: 30.0,
            warmth_offset: 0.0,
        }
    }
}

// ----------------------------------------------------------------------------
// 3. Tonal Contrast
// ----------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
pub struct TonalContrastParams {
    pub high_tones: f32,          // -100.0..=100.0
    pub mid_tones: f32,           // -100.0..=100.0
    pub low_tones: f32,           // -100.0..=100.0
    pub protect_shadows: f32,     // 0.0..=100.0
    pub protect_highlights: f32,  // 0.0..=100.0
}

impl TonalContrastParams {
    pub fn is_identity(&self) -> bool {
        self.high_tones == 0.0 && self.mid_tones == 0.0 && self.low_tones == 0.0
    }

    pub fn initial_active() -> Self {
        Self {
            high_tones: 20.0,
            mid_tones: 30.0,
            low_tones: 20.0,
            protect_shadows: 20.0,
            protect_highlights: 20.0,
        }
    }
}

// ----------------------------------------------------------------------------
// 4. Perspective
// ----------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum PerspectiveFillMode {
    #[default]
    SmartMirror,
    Black,
    White,
}

impl PerspectiveFillMode {
    pub const ALL: &'static [Self] = &[Self::SmartMirror, Self::Black, Self::White];

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::SmartMirror => "Smart Mirror",
            Self::Black => "Black",
            Self::White => "White",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct PerspectiveParams {
    pub vertical: f32,                  // -100.0..=100.0
    pub horizontal: f32,                // -100.0..=100.0
    pub rotate_deg: f32,                // -45.0..=45.0
    pub scale: f32,                     // 0.5..=2.0
    pub edge_fill: PerspectiveFillMode,
}

impl Default for PerspectiveParams {
    fn default() -> Self {
        Self {
            vertical: 0.0,
            horizontal: 0.0,
            rotate_deg: 0.0,
            scale: 1.0,
            edge_fill: PerspectiveFillMode::SmartMirror,
        }
    }
}

impl PerspectiveParams {
    pub fn is_identity(&self) -> bool {
        self.vertical == 0.0
            && self.horizontal == 0.0
            && self.rotate_deg == 0.0
            && (self.scale - 1.0).abs() < 1e-4
    }
}

// ----------------------------------------------------------------------------
// 5. Expand
// ----------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum ExpandFillMode {
    #[default]
    SmartEdge,
    Black,
    White,
}

impl ExpandFillMode {
    pub const ALL: &'static [Self] = &[Self::SmartEdge, Self::Black, Self::White];

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::SmartEdge => "Smart Edge",
            Self::Black => "Black",
            Self::White => "White",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
pub struct ExpandParams {
    pub top: f32,                  // 0.0..=1.0
    pub bottom: f32,               // 0.0..=1.0
    pub left: f32,                 // 0.0..=1.0
    pub right: f32,                // 0.0..=1.0
    pub fill_mode: ExpandFillMode,
}

impl ExpandParams {
    pub fn is_identity(&self) -> bool {
        self.top == 0.0 && self.bottom == 0.0 && self.left == 0.0 && self.right == 0.0
    }
}

// ----------------------------------------------------------------------------
// 6. Selective
// ----------------------------------------------------------------------------

pub const MAX_SELECTIVE_PINS: usize = 16;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
pub struct SelectivePin {
    pub x: f32,          // 0.0..=1.0
    pub y: f32,          // 0.0..=1.0
    pub radius: f32,     // 0.01..=1.0
    pub brightness: f32, // -100.0..=100.0
    pub contrast: f32,   // -100.0..=100.0
    pub saturation: f32, // -100.0..=100.0
    pub structure: f32,  // -100.0..=100.0
}

impl Default for SelectivePin {
    fn default() -> Self {
        Self {
            x: 0.5,
            y: 0.5,
            radius: 0.25,
            brightness: 0.0,
            contrast: 0.0,
            saturation: 0.0,
            structure: 0.0,
        }
    }
}

impl SelectivePin {
    pub fn is_identity(&self) -> bool {
        self.brightness == 0.0
            && self.contrast == 0.0
            && self.saturation == 0.0
            && self.structure == 0.0
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
pub struct SelectiveParams {
    pub pins: Vec<SelectivePin>,
    pub active_pin: usize,
}

impl SelectiveParams {
    pub fn is_identity(&self) -> bool {
        self.pins.is_empty() || self.pins.iter().all(|p| p.is_identity())
    }

    pub fn initial_active() -> Self {
        Self {
            pins: vec![SelectivePin::default()],
            active_pin: 0,
        }
    }
}

// ----------------------------------------------------------------------------
// 7. Brush
// ----------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum BrushChannel {
    #[default]
    DodgeBurn,
    Exposure,
    Temperature,
    Saturation,
}

impl BrushChannel {
    pub const ALL: &'static [Self] = &[
        Self::DodgeBurn,
        Self::Exposure,
        Self::Temperature,
        Self::Saturation,
    ];

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::DodgeBurn => "Dodge & Burn",
            Self::Exposure => "Exposure",
            Self::Temperature => "Temperature",
            Self::Saturation => "Saturation",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct BrushParams {
    pub channel: BrushChannel,
    pub step: f32,    // -100.0..=100.0
    pub radius: f32,  // 0.01..=0.5
    pub feather: f32, // 0.0..=1.0
    pub eraser: bool,
    pub map: BrushMap,
}

impl Default for BrushParams {
    fn default() -> Self {
        Self {
            channel: BrushChannel::DodgeBurn,
            step: 10.0,
            radius: 0.10,
            feather: 0.50,
            eraser: false,
            map: BrushMap::new(),
        }
    }
}

impl BrushParams {
    pub fn is_identity(&self) -> bool {
        self.map.is_empty()
    }
}

// ----------------------------------------------------------------------------
// 8. Healing
// ----------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct HealStroke {
    pub points: Vec<[f32; 2]>,
    pub radius: f32,
    pub source_offset: Option<[f32; 2]>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct HealingParams {
    pub strokes: Vec<HealStroke>,
    pub brush_radius: f32,
}

impl Default for HealingParams {
    fn default() -> Self {
        Self {
            strokes: Vec::new(),
            brush_radius: 0.05,
        }
    }
}

impl HealingParams {
    pub fn is_identity(&self) -> bool {
        self.strokes.is_empty()
    }
}

// ----------------------------------------------------------------------------
// 9. Halation
// ----------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct HalationParams {
    pub threshold: f32, // 0.0..=100.0
    pub strength: f32,  // 0.0..=100.0
    pub spread: f32,    // 0.0..=100.0
}

impl Default for HalationParams {
    fn default() -> Self {
        Self {
            threshold: 80.0,
            strength: 0.0,
            spread: 40.0,
        }
    }
}

impl HalationParams {
    pub fn is_identity(&self) -> bool {
        self.strength == 0.0
    }

    pub fn initial_active() -> Self {
        Self {
            threshold: 75.0,
            strength: 40.0,
            spread: 40.0,
        }
    }
}

// ----------------------------------------------------------------------------
// 10. Bloom
// ----------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct BloomParams {
    pub strength: f32,        // 0.0..=100.0
    pub size: f32,            // 0.0..=100.0
    pub highlight_range: f32, // 0.0..=100.0
}

impl Default for BloomParams {
    fn default() -> Self {
        Self {
            strength: 0.0,
            size: 30.0,
            highlight_range: 70.0,
        }
    }
}

impl BloomParams {
    pub fn is_identity(&self) -> bool {
        self.strength == 0.0
    }

    pub fn initial_active() -> Self {
        Self {
            strength: 35.0,
            size: 30.0,
            highlight_range: 70.0,
        }
    }
}

// ----------------------------------------------------------------------------
// 11. Grainy Film
// ----------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum FilmStock {
    #[default]
    K100,
    K400,
    F200,
    Velvia50,
    Cinestill800T,
    IlfordHP5,
    CrossProcess,
    Polaroid,
}

impl FilmStock {
    pub const ALL: &'static [Self] = &[
        Self::K100,
        Self::K400,
        Self::F200,
        Self::Velvia50,
        Self::Cinestill800T,
        Self::IlfordHP5,
        Self::CrossProcess,
        Self::Polaroid,
    ];

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::K100 => "Kodak 100",
            Self::K400 => "Kodak 400",
            Self::F200 => "Fuji 200",
            Self::Velvia50 => "Velvia 50",
            Self::Cinestill800T => "CineStill 800T",
            Self::IlfordHP5 => "Ilford HP5",
            Self::CrossProcess => "Cross Process",
            Self::Polaroid => "Polaroid",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
pub struct GrainyFilmParams {
    pub stock: FilmStock,
    pub grain: f32,          // 0.0..=100.0
    pub style_strength: f32, // 0.0..=100.0
}

impl GrainyFilmParams {
    pub fn is_identity(&self) -> bool {
        self.grain == 0.0 && self.style_strength == 0.0
    }

    pub fn initial_active() -> Self {
        Self {
            stock: FilmStock::K400,
            grain: 30.0,
            style_strength: 50.0,
        }
    }
}

// ----------------------------------------------------------------------------
// 12. Noir
// ----------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum NoirPreset {
    #[default]
    Dark,
    Light,
    Film,
    Harsh,
}

impl NoirPreset {
    pub const ALL: &'static [Self] = &[Self::Dark, Self::Light, Self::Film, Self::Harsh];

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Dark => "Dark",
            Self::Light => "Light",
            Self::Film => "Film",
            Self::Harsh => "Harsh",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
pub struct NoirParams {
    pub preset: NoirPreset,
    pub brightness: f32, // -100.0..=100.0
    pub contrast: f32,   // -100.0..=100.0
    pub wash: f32,       // 0.0..=100.0
    pub grain: f32,      // 0.0..=100.0
}

impl NoirParams {
    pub fn is_identity(&self) -> bool {
        false // Noir is a creative B&W conversion filter
    }

    pub fn initial_active() -> Self {
        Self {
            preset: NoirPreset::Dark,
            brightness: 0.0,
            contrast: 20.0,
            wash: 10.0,
            grain: 25.0,
        }
    }
}

// ----------------------------------------------------------------------------
// 13. HDR Scape
// ----------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum HdrScapePreset {
    #[default]
    Nature,
    People,
    Fine,
    Strong,
}

impl HdrScapePreset {
    pub const ALL: &'static [Self] = &[Self::Nature, Self::People, Self::Fine, Self::Strong];

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Nature => "Nature",
            Self::People => "People",
            Self::Fine => "Fine",
            Self::Strong => "Strong",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
pub struct HdrScapeParams {
    pub preset: HdrScapePreset,
    pub filter_strength: f32, // 0.0..=100.0
    pub brightness: f32,      // -100.0..=100.0
    pub saturation: f32,      // -100.0..=100.0
}

impl HdrScapeParams {
    pub fn is_identity(&self) -> bool {
        self.filter_strength == 0.0 && self.brightness == 0.0 && self.saturation == 0.0
    }

    pub fn initial_active() -> Self {
        Self {
            preset: HdrScapePreset::Nature,
            filter_strength: 50.0,
            brightness: 0.0,
            saturation: 10.0,
        }
    }
}

// ----------------------------------------------------------------------------
// 14. Glamour Glow
// ----------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct GlamourGlowParams {
    pub style: u8,       // 1..=5
    pub glow: f32,       // 0.0..=100.0
    pub saturation: f32, // -100.0..=100.0
    pub warmth: f32,     // -100.0..=100.0
}

impl Default for GlamourGlowParams {
    fn default() -> Self {
        Self {
            style: 1,
            glow: 0.0,
            saturation: 0.0,
            warmth: 0.0,
        }
    }
}

impl GlamourGlowParams {
    pub fn is_identity(&self) -> bool {
        self.glow == 0.0 && self.saturation == 0.0 && self.warmth == 0.0
    }

    pub fn initial_active() -> Self {
        Self {
            style: 1,
            glow: 40.0,
            saturation: 10.0,
            warmth: 15.0,
        }
    }
}

// ----------------------------------------------------------------------------
// 15. Drama
// ----------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum DramaPreset {
    #[default]
    Drama1,
    Drama2,
    Bright1,
    Bright2,
    Dark1,
    Dark2,
}

impl DramaPreset {
    pub const ALL: &'static [Self] = &[
        Self::Drama1,
        Self::Drama2,
        Self::Bright1,
        Self::Bright2,
        Self::Dark1,
        Self::Dark2,
    ];

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Drama1 => "Drama 1",
            Self::Drama2 => "Drama 2",
            Self::Bright1 => "Bright 1",
            Self::Bright2 => "Bright 2",
            Self::Dark1 => "Dark 1",
            Self::Dark2 => "Dark 2",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
pub struct DramaParams {
    pub preset: DramaPreset,
    pub filter_strength: f32, // 0.0..=100.0
    pub saturation: f32,      // -100.0..=100.0
}

impl DramaParams {
    pub fn is_identity(&self) -> bool {
        self.filter_strength == 0.0 && self.saturation == 0.0
    }

    pub fn initial_active() -> Self {
        Self {
            preset: DramaPreset::Drama1,
            filter_strength: 50.0,
            saturation: -10.0,
        }
    }
}

// ----------------------------------------------------------------------------
// 16. Vintage
// ----------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct VintageParams {
    pub style: u8,                 // 0..=11
    pub brightness: f32,           // -100.0..=100.0
    pub saturation: f32,           // -100.0..=100.0
    pub style_strength: f32,       // 0.0..=100.0
    pub vignette_strength: f32,    // 0.0..=100.0
    pub center_size: f32,          // 0.0..=100.0
}

impl Default for VintageParams {
    fn default() -> Self {
        Self {
            style: 0,
            brightness: 0.0,
            saturation: 0.0,
            style_strength: 0.0,
            vignette_strength: 0.0,
            center_size: 50.0,
        }
    }
}

impl VintageParams {
    pub fn is_identity(&self) -> bool {
        self.style_strength == 0.0
            && self.brightness == 0.0
            && self.saturation == 0.0
            && self.vignette_strength == 0.0
    }

    pub fn initial_active() -> Self {
        Self {
            style: 1,
            brightness: 0.0,
            saturation: -10.0,
            style_strength: 60.0,
            vignette_strength: 30.0,
            center_size: 50.0,
        }
    }
}

// ----------------------------------------------------------------------------
// 17. Lens Blur
// ----------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum LensBlurShape {
    #[default]
    Elliptical,
    Linear,
}

impl LensBlurShape {
    pub const ALL: &'static [Self] = &[Self::Elliptical, Self::Linear];

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Elliptical => "Elliptical",
            Self::Linear => "Linear",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct LensBlurParams {
    pub shape: LensBlurShape,
    pub center: [f32; 2],
    pub inner_radius: f32,
    pub transition: f32,
    pub angle_deg: f32,
    pub blur_strength: f32,     // 0.0..=100.0
    pub bokeh_boost: f32,       // 0.0..=100.0
    pub vignette_strength: f32, // 0.0..=100.0
}

impl Default for LensBlurParams {
    fn default() -> Self {
        Self {
            shape: LensBlurShape::Elliptical,
            center: [0.5, 0.5],
            inner_radius: 0.25,
            transition: 0.35,
            angle_deg: 0.0,
            blur_strength: 0.0,
            bokeh_boost: 0.0,
            vignette_strength: 0.0,
        }
    }
}

impl LensBlurParams {
    pub fn is_identity(&self) -> bool {
        self.blur_strength == 0.0 && self.bokeh_boost == 0.0 && self.vignette_strength == 0.0
    }

    pub fn initial_active() -> Self {
        Self {
            shape: LensBlurShape::Elliptical,
            center: [0.5, 0.5],
            inner_radius: 0.25,
            transition: 0.35,
            angle_deg: 0.0,
            blur_strength: 30.0,
            bokeh_boost: 10.0,
            vignette_strength: 15.0,
        }
    }
}

// ----------------------------------------------------------------------------
// 18. Motion Blur
// ----------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum MotionBlurMode {
    #[default]
    FullScene,
    RadialPan,
}

impl MotionBlurMode {
    pub const ALL: &'static [Self] = &[Self::FullScene, Self::RadialPan];

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::FullScene => "Full Scene",
            Self::RadialPan => "Radial Pan",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct MotionBlurParams {
    pub mode: MotionBlurMode,
    pub angle_deg: f32,  // -180.0..=180.0
    pub strength: f32,   // 0.0..=100.0
    pub center: [f32; 2],
}

impl Default for MotionBlurParams {
    fn default() -> Self {
        Self {
            mode: MotionBlurMode::FullScene,
            angle_deg: 0.0,
            strength: 0.0,
            center: [0.5, 0.5],
        }
    }
}

impl MotionBlurParams {
    pub fn is_identity(&self) -> bool {
        self.strength == 0.0
    }

    pub fn initial_active() -> Self {
        Self {
            mode: MotionBlurMode::FullScene,
            angle_deg: 0.0,
            strength: 25.0,
            center: [0.5, 0.5],
        }
    }
}

// ----------------------------------------------------------------------------
// 19. Double Exposure
// ----------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum DoubleExposureBlend {
    #[default]
    Default,
    Lighten,
    Darken,
    Add,
    Subtract,
    Blend,
}

impl DoubleExposureBlend {
    pub const ALL: &'static [Self] = &[
        Self::Default,
        Self::Lighten,
        Self::Darken,
        Self::Add,
        Self::Subtract,
        Self::Blend,
    ];

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::Lighten => "Lighten",
            Self::Darken => "Darken",
            Self::Add => "Add",
            Self::Subtract => "Subtract",
            Self::Blend => "Blend",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
pub struct DoubleExposureParams {
    pub secondary_path: Option<String>,
    pub blend_mode: DoubleExposureBlend,
    pub opacity: f32, // 0.0..=100.0
}

impl DoubleExposureParams {
    pub fn is_identity(&self) -> bool {
        self.secondary_path.is_none() || self.opacity <= 0.0
    }

    pub fn initial_active() -> Self {
        Self {
            secondary_path: None,
            blend_mode: DoubleExposureBlend::Default,
            opacity: 50.0,
        }
    }
}

// ----------------------------------------------------------------------------
// 20. Retrolux
// ----------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct RetroluxParams {
    pub style: u8,           // 0..=12
    pub brightness: f32,     // -100.0..=100.0
    pub saturation: f32,     // -100.0..=100.0
    pub contrast: f32,       // -100.0..=100.0
    pub style_strength: f32, // 0.0..=100.0
    pub scratches: f32,      // 0.0..=100.0
    pub light_leaks: f32,    // 0.0..=100.0
    pub seed: u32,
}

impl Default for RetroluxParams {
    fn default() -> Self {
        Self {
            style: 0,
            brightness: 0.0,
            saturation: 0.0,
            contrast: 0.0,
            style_strength: 0.0,
            scratches: 0.0,
            light_leaks: 0.0,
            seed: 42,
        }
    }
}

impl RetroluxParams {
    pub fn is_identity(&self) -> bool {
        self.style_strength == 0.0
            && self.scratches == 0.0
            && self.light_leaks == 0.0
            && self.brightness == 0.0
            && self.saturation == 0.0
            && self.contrast == 0.0
    }

    pub fn initial_active() -> Self {
        Self {
            style: 1,
            brightness: 0.0,
            saturation: -15.0,
            contrast: 10.0,
            style_strength: 50.0,
            scratches: 35.0,
            light_leaks: 40.0,
            seed: 42,
        }
    }
}

// ----------------------------------------------------------------------------
// 21. Grunge
// ----------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct GrungeParams {
    pub style: u8,              // 0..=4
    pub brightness: f32,        // -100.0..=100.0
    pub contrast: f32,          // -100.0..=100.0
    pub saturation: f32,        // -100.0..=100.0
    pub texture_strength: f32,  // 0.0..=100.0
    pub seed: u32,
}

impl Default for GrungeParams {
    fn default() -> Self {
        Self {
            style: 0,
            brightness: 0.0,
            contrast: 0.0,
            saturation: 0.0,
            texture_strength: 0.0,
            seed: 123,
        }
    }
}

impl GrungeParams {
    pub fn is_identity(&self) -> bool {
        self.texture_strength == 0.0
            && self.brightness == 0.0
            && self.contrast == 0.0
            && self.saturation == 0.0
    }

    pub fn initial_active() -> Self {
        Self {
            style: 1,
            brightness: -10.0,
            contrast: 20.0,
            saturation: -20.0,
            texture_strength: 50.0,
            seed: 123,
        }
    }
}

// ----------------------------------------------------------------------------
// 22. Portrait
// ----------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum PortraitSkinTone {
    #[default]
    None,
    Pale,
    Fair,
    Medium,
    Tan,
    Deep,
}

impl PortraitSkinTone {
    pub const ALL: &'static [Self] = &[
        Self::None,
        Self::Pale,
        Self::Fair,
        Self::Medium,
        Self::Tan,
        Self::Deep,
    ];

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Pale => "Pale",
            Self::Fair => "Fair",
            Self::Medium => "Medium",
            Self::Tan => "Tan",
            Self::Deep => "Deep",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct PortraitParams {
    pub face_center: [f32; 2],
    pub face_radius: f32,
    pub face_spotlight: f32,     // 0.0..=100.0
    pub skin_smoothing: f32,     // 0.0..=100.0
    pub eye_clarity: f32,        // 0.0..=100.0
    pub skin_tone: PortraitSkinTone,
}

impl Default for PortraitParams {
    fn default() -> Self {
        Self {
            face_center: [0.5, 0.4],
            face_radius: 0.20,
            face_spotlight: 0.0,
            skin_smoothing: 0.0,
            eye_clarity: 0.0,
            skin_tone: PortraitSkinTone::None,
        }
    }
}

impl PortraitParams {
    pub fn is_identity(&self) -> bool {
        self.face_spotlight == 0.0
            && self.skin_smoothing == 0.0
            && self.eye_clarity == 0.0
            && self.skin_tone == PortraitSkinTone::None
    }

    pub fn initial_active() -> Self {
        Self {
            face_center: [0.5, 0.4],
            face_radius: 0.20,
            face_spotlight: 20.0,
            skin_smoothing: 30.0,
            eye_clarity: 25.0,
            skin_tone: PortraitSkinTone::Fair,
        }
    }
}

// ----------------------------------------------------------------------------
// 23. Head Pose
// ----------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct HeadPoseParams {
    pub face_center: [f32; 2],
    pub yaw_x: f32,        // -100.0..=100.0
    pub pitch_y: f32,      // -100.0..=100.0
    pub smile: f32,        // -100.0..=100.0
    pub focal_length: f32, // -100.0..=100.0
}

impl Default for HeadPoseParams {
    fn default() -> Self {
        Self {
            face_center: [0.5, 0.4],
            yaw_x: 0.0,
            pitch_y: 0.0,
            smile: 0.0,
            focal_length: 0.0,
        }
    }
}

impl HeadPoseParams {
    pub fn is_identity(&self) -> bool {
        self.yaw_x == 0.0 && self.pitch_y == 0.0 && self.smile == 0.0 && self.focal_length == 0.0
    }

    pub fn initial_active() -> Self {
        Self {
            face_center: [0.5, 0.4],
            yaw_x: 0.0,
            pitch_y: 0.0,
            smile: 15.0,
            focal_length: 0.0,
        }
    }
}

// ----------------------------------------------------------------------------
// 24. Frames
// ----------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum FrameStyle {
    #[default]
    CleanWhite,
    MatteBlack,
    Polaroid,
    Film35mm,
    InstantDate,
    GalleryLine,
    RoughEdge,
    Letterbox,
}

impl FrameStyle {
    pub const ALL: &'static [Self] = &[
        Self::CleanWhite,
        Self::MatteBlack,
        Self::Polaroid,
        Self::Film35mm,
        Self::InstantDate,
        Self::GalleryLine,
        Self::RoughEdge,
        Self::Letterbox,
    ];

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::CleanWhite => "Clean White",
            Self::MatteBlack => "Matte Black",
            Self::Polaroid => "Polaroid",
            Self::Film35mm => "35mm Film",
            Self::InstantDate => "Instant Date",
            Self::GalleryLine => "Gallery Line",
            Self::RoughEdge => "Rough Edge",
            Self::Letterbox => "Letterbox",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct FramesParams {
    pub style: FrameStyle,
    pub width: f32,         // 0.0..=100.0
    pub inner_line: bool,
    pub aspect_pad: f32,    // 0.0..=100.0
}

impl Default for FramesParams {
    fn default() -> Self {
        Self {
            style: FrameStyle::CleanWhite,
            width: 0.0,
            inner_line: false,
            aspect_pad: 0.0,
        }
    }
}

impl FramesParams {
    pub fn is_identity(&self) -> bool {
        self.width <= 0.0 && self.aspect_pad <= 0.0 && !self.inner_line
    }

    pub fn initial_active() -> Self {
        Self {
            style: FrameStyle::CleanWhite,
            width: 25.0,
            inner_line: false,
            aspect_pad: 0.0,
        }
    }
}

// ----------------------------------------------------------------------------
// 25. Text Overlay
// ----------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum TextStyle {
    #[default]
    ModernBold,
    EditorialSerif,
    MinimalBox,
    StampFrame,
    CinemaSub,
    RetroBadge,
    VerticalTag,
    Signature,
}

impl TextStyle {
    pub const ALL: &'static [Self] = &[
        Self::ModernBold,
        Self::EditorialSerif,
        Self::MinimalBox,
        Self::StampFrame,
        Self::CinemaSub,
        Self::RetroBadge,
        Self::VerticalTag,
        Self::Signature,
    ];

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::ModernBold => "Modern Bold",
            Self::EditorialSerif => "Editorial Serif",
            Self::MinimalBox => "Minimal Box",
            Self::StampFrame => "Stamp Frame",
            Self::CinemaSub => "Cinema Sub",
            Self::RetroBadge => "Retro Badge",
            Self::VerticalTag => "Vertical Tag",
            Self::Signature => "Signature",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum TextColorPreset {
    #[default]
    White,
    Black,
    Cream,
    Gold,
    Coral,
    Cyan,
    Sage,
    Slate,
}

impl TextColorPreset {
    pub const ALL: &'static [Self] = &[
        Self::White,
        Self::Black,
        Self::Cream,
        Self::Gold,
        Self::Coral,
        Self::Cyan,
        Self::Sage,
        Self::Slate,
    ];

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::White => "White",
            Self::Black => "Black",
            Self::Cream => "Cream",
            Self::Gold => "Gold",
            Self::Coral => "Coral",
            Self::Cyan => "Cyan",
            Self::Sage => "Sage",
            Self::Slate => "Slate",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct TextOverlayParams {
    pub text: String,
    pub subtext: String,
    pub style: TextStyle,
    pub center: [f32; 2],
    pub scale: f32,                // 0.1..=3.0
    pub opacity: f32,              // 0.0..=100.0
    pub invert_mask: bool,
    pub color_preset: TextColorPreset,
}

impl Default for TextOverlayParams {
    fn default() -> Self {
        Self {
            text: String::new(),
            subtext: String::new(),
            style: TextStyle::ModernBold,
            center: [0.5, 0.5],
            scale: 1.0,
            opacity: 0.0,
            invert_mask: false,
            color_preset: TextColorPreset::White,
        }
    }
}

impl TextOverlayParams {
    pub fn is_identity(&self) -> bool {
        self.text.is_empty() || self.opacity <= 0.0
    }

    pub fn initial_active() -> Self {
        Self {
            text: "Snap".to_string(),
            subtext: "Photography".to_string(),
            style: TextStyle::ModernBold,
            center: [0.5, 0.5],
            scale: 1.0,
            opacity: 90.0,
            invert_mask: false,
            color_preset: TextColorPreset::White,
        }
    }
}
