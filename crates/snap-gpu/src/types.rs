use snap_core::ops::CropRotateParams;

/// Viewport rendering parameters.
#[derive(Clone, Debug, PartialEq)]
pub struct ViewportParams {
    pub zoom: f32, // 1.0 = Fit to bounds
    pub pan: [f32; 2],
    pub crop: CropRotateParams,
    pub crop_preview_mode: bool,
}

impl Default for ViewportParams {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: [0.0, 0.0],
            crop: CropRotateParams::default(),
            crop_preview_mode: false,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RawDevelopUniforms {
    pub exposure_ev: f32,
    pub highlights: f32,
    pub shadows: f32,
    pub contrast: f32,
    pub temperature: f32,
    pub tint: f32,
    pub saturation: f32,
    pub opacity: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct WhiteBalanceUniforms {
    pub temperature: f32,
    pub tint: f32,
    pub opacity: f32,
    pub _pad: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TuneImageUniforms {
    pub brightness: f32,
    pub contrast: f32,
    pub saturation: f32,
    pub ambiance: f32,
    pub highlights: f32,
    pub shadows: f32,
    pub warmth: f32,
    pub opacity: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ColorHslBandGpu {
    pub hue: f32,
    pub saturation: f32,
    pub luminance: f32,
    pub _pad: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ColorHslUniforms {
    pub bands: [ColorHslBandGpu; 8],
    pub opacity: f32,
    pub _pad0: f32,
    pub _pad1: f32,
    pub _pad2: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CurvesUniforms {
    pub opacity: f32,
    pub _pad0: f32,
    pub _pad1: f32,
    pub _pad2: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Lut3dUniforms {
    pub domain_min: [f32; 3],
    pub lut_size: f32,
    pub domain_max: [f32; 3],
    pub strength: f32,
    pub input_space: u32,
    pub opacity: f32,
    pub _pad0: f32,
    pub _pad1: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DetailsUniforms {
    pub structure: f32,
    pub sharpening: f32,
    pub opacity: f32,
    pub pass_idx: u32, // 0 = horizontal pass, 1 = vertical pass
    pub width: f32,
    pub height: f32,
    pub _pad0: f32,
    pub _pad1: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BlackAndWhiteUniforms {
    pub channel_weights: [f32; 3],
    pub brightness: f32,
    pub contrast: f32,
    pub grain: f32,
    pub opacity: f32,
    pub _pad: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VignetteUniforms {
    pub center: [f32; 2],
    pub inner_radius: f32,
    pub outer_radius: f32,
    pub outer_brightness: f32,
    pub inner_brightness: f32,
    pub aspect: f32,
    pub opacity: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CropTransformUniforms {
    pub crop_rect: [f32; 4], // x, y, width, height
    pub src_size: [f32; 2],
    pub dst_size: [f32; 2],
    pub straighten_rad: f32,
    pub inscribed_scale: f32,
    pub quarter_turns: u32,
    pub flip_flags: u32, // bit 0 = flip_h, bit 1 = flip_v
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ViewportUniforms {
    pub viewport_size: [f32; 2], // width, height in physical pixels
    pub image_size: [f32; 2],    // base image width, height
    pub crop_rect: [f32; 4],     // x, y, width, height
    pub pan: [f32; 2],
    pub zoom: f32,
    pub straighten_rad: f32,
    pub inscribed_scale: f32,
    pub quarter_turns: u32,
    pub flip_flags: u32,
    pub crop_preview_mode: u32,
    pub output_is_srgb: u32,
    pub _pad0: f32,
    pub _pad1: f32,
    pub _pad2: f32,
}
