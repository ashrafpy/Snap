use std::collections::HashMap;
use snap_core::curves::CurvesParams;
use snap_core::image_types::{HistogramData, LinearImage, Lut3D};
use snap_core::mask::{BrushMap, LayerMask};
use snap_core::ops::{CropRotateParams, EditOp};
use snap_core::ops_ext::*;
use snap_core::stack::{EditNode, EditStack};
use wgpu::util::DeviceExt;

use crate::error::GpuError;
use crate::lut_cache::GpuLutCache;
use crate::shaders::*;
use crate::shaders_p2::*;
use crate::shaders_p3_p4::*;
use crate::types::*;

#[derive(Clone, Debug, PartialEq)]
struct NodeFingerprint {
    id: [u8; 16],
    enabled: bool,
    opacity: u32, // quantized opacity for stable hash
    op_tag: &'static str,
    payload_hash: u64,
    mask_hash: Option<u64>,
    brush_hash: u64,
    secondary_gen: u64,
}

#[derive(Clone, Debug, PartialEq)]
struct DirtyCacheKey {
    image_id: u64,
    width: u32,
    height: u32,
    bypass_edits: bool,
    nodes: Vec<NodeFingerprint>,
}

pub struct GpuPipelineEngine {
    // LUT Cache
    pub lut_cache: GpuLutCache,

    // Source image
    uploaded_image_id: Option<u64>,
    source_texture: Option<wgpu::Texture>,
    source_view: Option<wgpu::TextureView>,
    source_dims: (u32, u32),

    // Ping-pong compute textures
    tex_a: Option<wgpu::Texture>,
    view_a: Option<wgpu::TextureView>,
    tex_b: Option<wgpu::Texture>,
    view_b: Option<wgpu::TextureView>,
    tex_temp: Option<wgpu::Texture>,
    view_temp: Option<wgpu::TextureView>,
    tex_pre: Option<wgpu::Texture>,
    view_pre: Option<wgpu::TextureView>,
    allocated_dims: (u32, u32),

    // Samplers
    linear_clamp_sampler: wgpu::Sampler,

    // Viewport uniform buffer
    viewport_buf: wgpu::Buffer,

    // Histogram buffers
    hist_storage_buf: wgpu::Buffer,
    hist_staging_buf: wgpu::Buffer,

    // Bind group layouts
    compute_bgl: wgpu::BindGroupLayout,
    curves_bgl: wgpu::BindGroupLayout,
    lut_3d_bgl: wgpu::BindGroupLayout,
    histogram_bgl: wgpu::BindGroupLayout,
    crop_transform_bgl: wgpu::BindGroupLayout,
    viewport_bgl: wgpu::BindGroupLayout,
    brush_bgl: wgpu::BindGroupLayout,
    mask_blend_bgl: wgpu::BindGroupLayout,

    // Compute pipelines (Phase 1)
    raw_develop_pipeline: wgpu::ComputePipeline,
    white_balance_pipeline: wgpu::ComputePipeline,
    tune_image_pipeline: wgpu::ComputePipeline,
    color_hsl_pipeline: wgpu::ComputePipeline,
    curves_pipeline: wgpu::ComputePipeline,
    lut_3d_pipeline: wgpu::ComputePipeline,
    details_pipeline: wgpu::ComputePipeline,
    black_and_white_pipeline: wgpu::ComputePipeline,
    vignette_pipeline: wgpu::ComputePipeline,
    histogram_pipeline: wgpu::ComputePipeline,
    crop_transform_pipeline: wgpu::ComputePipeline,

    // Compute pipelines (Phase 2)
    color_grade_pipeline: wgpu::ComputePipeline,
    dehaze_pipeline: wgpu::ComputePipeline,
    tonal_contrast_pipeline: wgpu::ComputePipeline,
    halation_pipeline: wgpu::ComputePipeline,
    bloom_pipeline: wgpu::ComputePipeline,
    grainy_film_pipeline: wgpu::ComputePipeline,
    noir_pipeline: wgpu::ComputePipeline,
    selective_pipeline: wgpu::ComputePipeline,
    brush_pipeline: wgpu::ComputePipeline,
    mask_blend_pipeline: wgpu::ComputePipeline,

    // Compute pipelines (Phase 3 & 4)
    healing_pipeline: wgpu::ComputePipeline,
    perspective_pipeline: wgpu::ComputePipeline,
    expand_pipeline: wgpu::ComputePipeline,
    lens_blur_pipeline: wgpu::ComputePipeline,
    motion_blur_pipeline: wgpu::ComputePipeline,
    hdr_scape_pipeline: wgpu::ComputePipeline,
    glamour_glow_pipeline: wgpu::ComputePipeline,
    drama_pipeline: wgpu::ComputePipeline,
    vintage_pipeline: wgpu::ComputePipeline,
    retrolux_pipeline: wgpu::ComputePipeline,
    grunge_pipeline: wgpu::ComputePipeline,
    portrait_pipeline: wgpu::ComputePipeline,
    head_pose_pipeline: wgpu::ComputePipeline,
    frames_pipeline: wgpu::ComputePipeline,
    double_exposure_pipeline: wgpu::ComputePipeline,
    text_overlay_pipeline: wgpu::ComputePipeline,

    // Auxiliary caches
    secondary_images: HashMap<String, (u64, wgpu::Texture, wgpu::TextureView, (u32, u32))>,
    secondary_hashes: HashMap<String, u64>,
    secondary_images_order: Vec<String>,
    secondary_gen_counter: u64,
    _fallback_black_tex: wgpu::Texture,
    fallback_black_view: wgpu::TextureView,

    mask_cache: HashMap<u64, (wgpu::Texture, wgpu::TextureView)>,
    mask_cache_order: Vec<u64>,
    brush_cache: HashMap<u64, (wgpu::Texture, wgpu::TextureView)>,
    brush_cache_order: Vec<u64>,
    text_cache: HashMap<u64, (wgpu::Texture, wgpu::TextureView)>,
    text_cache_order: Vec<u64>,

    // Viewport render pipelines cached per target format
    viewport_shader_module: wgpu::ShaderModule,
    viewport_pipeline_layout: wgpu::PipelineLayout,
    viewport_pipelines: HashMap<wgpu::TextureFormat, wgpu::RenderPipeline>,

    // Dirty state cache
    last_dirty_key: Option<DirtyCacheKey>,
    cached_output_is_source: bool,
    cached_output_in_a: bool,
    dirty_generation: u64,
}

impl GpuPipelineEngine {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target_format: Option<wgpu::TextureFormat>,
    ) -> Self {
        let lut_cache = GpuLutCache::new(device, queue);

        let linear_clamp_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("gpu_linear_clamp_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // Viewport uniform buffer
        let viewport_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("viewport_buf"),
            size: std::mem::size_of::<ViewportUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Histogram storage and staging buffer
        let hist_storage_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("hist_storage_buf"),
            size: 1024 * 4,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let hist_staging_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("hist_staging_buf"),
            size: 1024 * 4,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Bind group layouts
        let compute_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("compute_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba16Float,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let curves_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("curves_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba16Float,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let lut_3d_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("lut_3d_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba16Float,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D3,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let histogram_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("histogram_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let crop_transform_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("crop_transform_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba16Float,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let viewport_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("viewport_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let brush_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("brush_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba16Float,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let mask_blend_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("mask_blend_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba16Float,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let fallback_black_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("gpu_fallback_black_tex"),
            size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let black_row = [0u8; 256];
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &fallback_black_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &black_row,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
        );
        let fallback_black_view = fallback_black_tex.create_view(&wgpu::TextureViewDescriptor::default());

        // Helper to compile compute pipeline
        let create_compute = |label: &str, bgl: &wgpu::BindGroupLayout, code: &str, entry: &str| {
            let full_code = format!("{}\n{}", COMMON_WGSL, code);
            let sm = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(full_code.into()),
            });
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(&format!("{}_layout", label)),
                bind_group_layouts: &[bgl],
                push_constant_ranges: &[],
            });
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(label),
                layout: Some(&layout),
                module: &sm,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            })
        };

        let raw_develop_pipeline = create_compute("raw_develop_pipeline", &compute_bgl, RAW_DEVELOP_WGSL, "cs_raw_develop");
        let white_balance_pipeline = create_compute("white_balance_pipeline", &compute_bgl, WHITE_BALANCE_WGSL, "cs_white_balance");
        let tune_image_pipeline = create_compute("tune_image_pipeline", &compute_bgl, TUNE_IMAGE_WGSL, "cs_tune_image");
        let color_hsl_pipeline = create_compute("color_hsl_pipeline", &compute_bgl, COLOR_HSL_WGSL, "cs_color_hsl");
        let curves_pipeline = create_compute("curves_pipeline", &curves_bgl, CURVES_WGSL, "cs_curves");
        let lut_3d_pipeline = create_compute("lut_3d_pipeline", &lut_3d_bgl, LUT_3D_WGSL, "cs_lut_3d");
        let details_pipeline = create_compute("details_pipeline", &compute_bgl, DETAILS_WGSL, "cs_details");
        let black_and_white_pipeline = create_compute("black_and_white_pipeline", &compute_bgl, BLACK_AND_WHITE_WGSL, "cs_black_and_white");
        let vignette_pipeline = create_compute("vignette_pipeline", &compute_bgl, VIGNETTE_WGSL, "cs_vignette");
        let histogram_pipeline = create_compute("histogram_pipeline", &histogram_bgl, HISTOGRAM_WGSL, "cs_histogram");
        let crop_transform_pipeline = create_compute("crop_transform_pipeline", &crop_transform_bgl, CROP_TRANSFORM_WGSL, "cs_crop_transform");

        // Phase 2 compute pipelines
        let color_grade_pipeline = create_compute("color_grade_pipeline", &compute_bgl, COLOR_GRADE_WGSL, "cs_color_grade");
        let dehaze_pipeline = create_compute("dehaze_pipeline", &compute_bgl, DEHAZE_WGSL, "cs_dehaze");
        let tonal_contrast_pipeline = create_compute("tonal_contrast_pipeline", &compute_bgl, TONAL_CONTRAST_WGSL, "cs_tonal_contrast");
        let halation_pipeline = create_compute("halation_pipeline", &compute_bgl, HALATION_WGSL, "cs_halation");
        let bloom_pipeline = create_compute("bloom_pipeline", &compute_bgl, BLOOM_WGSL, "cs_bloom");
        let grainy_film_pipeline = create_compute("grainy_film_pipeline", &compute_bgl, GRAINY_FILM_WGSL, "cs_grainy_film");
        let noir_pipeline = create_compute("noir_pipeline", &compute_bgl, NOIR_WGSL, "cs_noir");
        let selective_pipeline = create_compute("selective_pipeline", &compute_bgl, SELECTIVE_WGSL, "cs_selective");
        let brush_pipeline = create_compute("brush_pipeline", &brush_bgl, BRUSH_WGSL, "cs_brush");
        let mask_blend_pipeline = create_compute("mask_blend_pipeline", &mask_blend_bgl, MASK_BLEND_WGSL, "cs_mask_blend");

        // Phase 3 & 4 compute pipelines
        let healing_pipeline = create_compute("healing_pipeline", &compute_bgl, HEALING_WGSL, "cs_healing");
        let perspective_pipeline = create_compute("perspective_pipeline", &compute_bgl, PERSPECTIVE_WGSL, "cs_perspective");
        let expand_pipeline = create_compute("expand_pipeline", &compute_bgl, EXPAND_WGSL, "cs_expand");
        let lens_blur_pipeline = create_compute("lens_blur_pipeline", &compute_bgl, LENS_BLUR_WGSL, "cs_lens_blur");
        let motion_blur_pipeline = create_compute("motion_blur_pipeline", &compute_bgl, MOTION_BLUR_WGSL, "cs_motion_blur");
        let hdr_scape_pipeline = create_compute("hdr_scape_pipeline", &compute_bgl, HDR_SCAPE_WGSL, "cs_hdr_scape");
        let glamour_glow_pipeline = create_compute("glamour_glow_pipeline", &compute_bgl, GLAMOUR_GLOW_WGSL, "cs_glamour_glow");
        let drama_pipeline = create_compute("drama_pipeline", &compute_bgl, DRAMA_WGSL, "cs_drama");
        let vintage_pipeline = create_compute("vintage_pipeline", &compute_bgl, VINTAGE_WGSL, "cs_vintage");
        let retrolux_pipeline = create_compute("retrolux_pipeline", &compute_bgl, RETROLUX_WGSL, "cs_retrolux");
        let grunge_pipeline = create_compute("grunge_pipeline", &compute_bgl, GRUNGE_WGSL, "cs_grunge");
        let portrait_pipeline = create_compute("portrait_pipeline", &compute_bgl, PORTRAIT_WGSL, "cs_portrait");
        let head_pose_pipeline = create_compute("head_pose_pipeline", &compute_bgl, HEAD_POSE_WGSL, "cs_head_pose");
        let frames_pipeline = create_compute("frames_pipeline", &compute_bgl, FRAMES_WGSL, "cs_frames");
        let double_exposure_pipeline = create_compute("double_exposure_pipeline", &curves_bgl, DOUBLE_EXPOSURE_WGSL, "cs_double_exposure");
        let text_overlay_pipeline = create_compute("text_overlay_pipeline", &curves_bgl, TEXT_OVERLAY_WGSL, "cs_text_overlay");

        // Viewport shader and layout
        let full_viewport_code = format!("{}\n{}", COMMON_WGSL, VIEWPORT_WGSL);
        let viewport_shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("viewport_shader_module"),
            source: wgpu::ShaderSource::Wgsl(full_viewport_code.into()),
        });
        let viewport_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("viewport_pipeline_layout"),
            bind_group_layouts: &[&viewport_bgl],
            push_constant_ranges: &[],
        });

        let mut viewport_pipelines = HashMap::new();
        let default_target_format = target_format.unwrap_or(wgpu::TextureFormat::Rgba8UnormSrgb);
        let initial_pipeline = Self::build_viewport_pipeline(
            device,
            &viewport_pipeline_layout,
            &viewport_shader_module,
            default_target_format,
        );
        viewport_pipelines.insert(default_target_format, initial_pipeline);

        Self {
            lut_cache,
            uploaded_image_id: None,
            source_texture: None,
            source_view: None,
            source_dims: (0, 0),
            tex_a: None,
            view_a: None,
            tex_b: None,
            view_b: None,
            tex_temp: None,
            view_temp: None,
            tex_pre: None,
            view_pre: None,
            allocated_dims: (0, 0),
            linear_clamp_sampler,
            viewport_buf,
            hist_storage_buf,
            hist_staging_buf,
            compute_bgl,
            curves_bgl,
            lut_3d_bgl,
            histogram_bgl,
            crop_transform_bgl,
            viewport_bgl,
            brush_bgl,
            mask_blend_bgl,
            raw_develop_pipeline,
            white_balance_pipeline,
            tune_image_pipeline,
            color_hsl_pipeline,
            curves_pipeline,
            lut_3d_pipeline,
            details_pipeline,
            black_and_white_pipeline,
            vignette_pipeline,
            histogram_pipeline,
            crop_transform_pipeline,
            color_grade_pipeline,
            dehaze_pipeline,
            tonal_contrast_pipeline,
            halation_pipeline,
            bloom_pipeline,
            grainy_film_pipeline,
            noir_pipeline,
            selective_pipeline,
            brush_pipeline,
            mask_blend_pipeline,
            healing_pipeline,
            perspective_pipeline,
            expand_pipeline,
            lens_blur_pipeline,
            motion_blur_pipeline,
            hdr_scape_pipeline,
            glamour_glow_pipeline,
            drama_pipeline,
            vintage_pipeline,
            retrolux_pipeline,
            grunge_pipeline,
            portrait_pipeline,
            head_pose_pipeline,
            frames_pipeline,
            double_exposure_pipeline,
            text_overlay_pipeline,
            secondary_images: HashMap::new(),
            secondary_hashes: HashMap::new(),
            secondary_images_order: Vec::new(),
            secondary_gen_counter: 0,
            _fallback_black_tex: fallback_black_tex,
            fallback_black_view,
            mask_cache: HashMap::new(),
            mask_cache_order: Vec::new(),
            brush_cache: HashMap::new(),
            brush_cache_order: Vec::new(),
            text_cache: HashMap::new(),
            text_cache_order: Vec::new(),
            viewport_shader_module,
            viewport_pipeline_layout,
            viewport_pipelines,
            last_dirty_key: None,
            cached_output_is_source: true,
            cached_output_in_a: false,
            dirty_generation: 0,
        }
    }

    fn build_viewport_pipeline(
        device: &wgpu::Device,
        layout: &wgpu::PipelineLayout,
        module: &wgpu::ShaderModule,
        format: wgpu::TextureFormat,
    ) -> wgpu::RenderPipeline {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(&format!("viewport_render_pipeline_{:?}", format)),
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        })
    }

    /// Dirty generation counter incremented whenever compute passes re-evaluate.
    pub fn dirty_generation(&self) -> u64 {
        self.dirty_generation
    }

    /// Upload a 3D LUT to the GPU cache.
    pub fn upload_lut(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, lut: &Lut3D) -> Result<(), GpuError> {
        self.lut_cache.upload_lut(device, queue, lut)
    }

    /// Upload a source image into the engine's internal texture cache.
    pub fn upload_source_image(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        image_id: u64,
        image: &LinearImage,
    ) {
        if self.uploaded_image_id == Some(image_id) && self.source_dims == (image.width, image.height) {
            return;
        }

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("gpu_source_texture"),
            size: wgpu::Extent3d {
                width: image.width,
                height: image.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let raw_bytes: &[u8] = bytemuck::cast_slice(&image.pixels);
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            raw_bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(image.width * 8),
                rows_per_image: Some(image.height),
            },
            wgpu::Extent3d {
                width: image.width,
                height: image.height,
                depth_or_array_layers: 1,
            },
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        self.uploaded_image_id = Some(image_id);
        self.source_texture = Some(texture);
        self.source_view = Some(view);
        self.source_dims = (image.width, image.height);
        self.ensure_ping_pong_textures(device, image.width, image.height);
        self.last_dirty_key = None;
    }

    fn ensure_ping_pong_textures(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if self.allocated_dims == (width, height) && self.tex_a.is_some() && self.tex_pre.is_some() {
            return;
        }

        let create_tex = |label: &str| {
            let tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba16Float,
                usage: wgpu::TextureUsages::STORAGE_BINDING
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC
                    | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
            (tex, view)
        };

        let (a_tex, a_view) = create_tex("gpu_tex_a");
        let (b_tex, b_view) = create_tex("gpu_tex_b");
        let (temp_tex, temp_view) = create_tex("gpu_tex_temp");

        let pre_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("gpu_tex_pre"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let pre_view = pre_tex.create_view(&wgpu::TextureViewDescriptor::default());

        self.tex_a = Some(a_tex);
        self.view_a = Some(a_view);
        self.tex_b = Some(b_tex);
        self.view_b = Some(b_view);
        self.tex_temp = Some(temp_tex);
        self.view_temp = Some(temp_view);
        self.tex_pre = Some(pre_tex);
        self.view_pre = Some(pre_view);
        self.allocated_dims = (width, height);
        self.last_dirty_key = None;
    }

    /// Upload a secondary image for Double Exposure operations.
    pub fn upload_secondary_image(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path_key: &str,
        image: &LinearImage,
    ) {
        let dims = (image.width, image.height);
        let sample_hash = {
            let len = image.pixels.len();
            if len == 0 {
                0u64
            } else {
                let mut h = len as u64;
                let p0 = image.pixels[0];
                let p_mid = image.pixels[len / 2];
                let p_last = image.pixels[len - 1];
                h ^= (p0[0].to_bits() as u64) | ((p0[1].to_bits() as u64) << 16) | ((p0[2].to_bits() as u64) << 32);
                h = h.wrapping_mul(0x100000001b3);
                h ^= (p_mid[0].to_bits() as u64) | ((p_mid[1].to_bits() as u64) << 16) | ((p_mid[2].to_bits() as u64) << 32);
                h = h.wrapping_mul(0x100000001b3);
                h ^= (p_last[0].to_bits() as u64) | ((p_last[1].to_bits() as u64) << 16) | ((p_last[2].to_bits() as u64) << 32);
                h
            }
        };

        if let Some((_, _, _, existing_dims)) = self.secondary_images.get(path_key) {
            if *existing_dims == dims && self.secondary_hashes.get(path_key) == Some(&sample_hash) {
                return;
            }
        }

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&format!("gpu_secondary_image_{}", path_key)),
            size: wgpu::Extent3d {
                width: image.width,
                height: image.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let raw_bytes: &[u8] = bytemuck::cast_slice(&image.pixels);
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            raw_bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(image.width * 8),
                rows_per_image: Some(image.height),
            },
            wgpu::Extent3d {
                width: image.width,
                height: image.height,
                depth_or_array_layers: 1,
            },
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.secondary_gen_counter = self.secondary_gen_counter.wrapping_add(1);

        if self.secondary_images.len() >= 32 && !self.secondary_images.contains_key(path_key) {
            if let Some(oldest) = self.secondary_images_order.first().cloned() {
                self.secondary_images.remove(&oldest);
                self.secondary_hashes.remove(&oldest);
                self.secondary_images_order.retain(|k| k != &oldest);
            }
        }

        self.secondary_images.insert(
            path_key.to_string(),
            (self.secondary_gen_counter, texture, view, dims),
        );
        self.secondary_hashes.insert(path_key.to_string(), sample_hash);
        if !self.secondary_images_order.contains(&path_key.to_string()) {
            self.secondary_images_order.push(path_key.to_string());
        }
    }

    fn ensure_mask(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, mask: &LayerMask) {
        let hash = mask.content_hash();
        if !self.mask_cache.contains_key(&hash) {
            let tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("gpu_layer_mask_tex"),
                size: wgpu::Extent3d { width: 256, height: 256, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::R8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &tex,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                mask.as_bytes(),
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(256),
                },
                wgpu::Extent3d { width: 256, height: 256, depth_or_array_layers: 1 },
            );
            let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
            if self.mask_cache.len() >= 64 {
                if let Some(oldest) = self.mask_cache_order.first().copied() {
                    self.mask_cache.remove(&oldest);
                    self.mask_cache_order.retain(|&k| k != oldest);
                }
            }
            self.mask_cache.insert(hash, (tex, view));
            self.mask_cache_order.push(hash);
        }
    }

    fn ensure_brush_map(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, map: &BrushMap) {
        let hash = map.content_hash();
        if !self.brush_cache.contains_key(&hash) {
            let f32_vec = map.to_f32_vec();
            let raw_bytes: &[u8] = bytemuck::cast_slice(&f32_vec);
            let tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("gpu_brush_map_tex"),
                size: wgpu::Extent3d { width: 256, height: 256, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::R32Float,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &tex,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                raw_bytes,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256 * 4),
                    rows_per_image: Some(256),
                },
                wgpu::Extent3d { width: 256, height: 256, depth_or_array_layers: 1 },
            );
            let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
            if self.brush_cache.len() >= 64 {
                if let Some(oldest) = self.brush_cache_order.first().copied() {
                    self.brush_cache.remove(&oldest);
                    self.brush_cache_order.retain(|&k| k != oldest);
                }
            }
            self.brush_cache.insert(hash, (tex, view));
            self.brush_cache_order.push(hash);
        }
    }

    fn ensure_text_overlay(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        params: &TextOverlayParams,
        width: u32,
        height: u32,
    ) {
        let aspect = width as f32 / (height.max(1) as f32);
        let hash = crate::text_raster::compute_text_cache_key(params) ^ (aspect.to_bits() as u64);
        if !self.text_cache.contains_key(&hash) {
            let raster = crate::text_raster::rasterize_text_overlay(params, aspect);
            let tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("gpu_text_overlay_tex"),
                size: wgpu::Extent3d {
                    width: raster.width,
                    height: raster.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &tex,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &raster.rgba,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(raster.width * 4),
                    rows_per_image: Some(raster.height),
                },
                wgpu::Extent3d {
                    width: raster.width,
                    height: raster.height,
                    depth_or_array_layers: 1,
                },
            );
            let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
            if self.text_cache.len() >= 64 {
                if let Some(oldest) = self.text_cache_order.first().copied() {
                    self.text_cache.remove(&oldest);
                    self.text_cache_order.retain(|&k| k != oldest);
                }
            }
            self.text_cache.insert(hash, (tex, view));
            self.text_cache_order.push(hash);
        }
    }

    fn create_curve_texture(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        curves: &CurvesParams,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let baked = curves.bake_lut_1024();
        let curve_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("gpu_curve_1024_tex"),
            size: wgpu::Extent3d {
                width: 1024,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &curve_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(&baked),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(1024 * 8),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: 1024,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
        let curve_view = curve_texture.create_view(&wgpu::TextureViewDescriptor::default());
        (curve_texture, curve_view)
    }

    /// Process the edit stack on the uploaded source image.
    /// Uses dirty caching: zero compute passes execute when edits haven't changed!
    pub fn process_stack(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        stack: &EditStack,
        bypass_edits: bool,
    ) -> Result<&wgpu::TextureView, GpuError> {
        let (width, height) = self.source_dims;
        if width == 0 || height == 0 || self.source_view.is_none() {
            return Err(GpuError::InvalidDimensions(width, height));
        }

        // Build dirty fingerprint
        let effective_nodes: Vec<&EditNode> = if bypass_edits {
            Vec::new()
        } else {
            stack.nodes.iter().filter(|n| n.enabled && !n.op.is_identity()).collect()
        };

        let node_fingerprints: Vec<NodeFingerprint> = effective_nodes
            .iter()
            .map(|n| {
                let (op_tag, brush_hash, secondary_gen) = match &n.op {
                    EditOp::RawDevelop(_) => ("raw", 0, 0),
                    EditOp::WhiteBalance(_) => ("wb", 0, 0),
                    EditOp::TuneImage(_) => ("tune", 0, 0),
                    EditOp::ColorHsl(_) => ("hsl", 0, 0),
                    EditOp::Curves(_) => ("curves", 0, 0),
                    EditOp::Lut3d(_) => ("lut", 0, 0),
                    EditOp::Details(_) => ("details", 0, 0),
                    EditOp::BlackAndWhite(_) => ("bw", 0, 0),
                    EditOp::Vignette(_) => ("vig", 0, 0),
                    EditOp::CropRotate(_) => ("crop", 0, 0),

                    EditOp::ColorGrade(_) => ("color_grade", 0, 0),
                    EditOp::Dehaze(_) => ("dehaze", 0, 0),
                    EditOp::TonalContrast(_) => ("tonal_contrast", 0, 0),
                    EditOp::Selective(_) => ("selective", 0, 0),
                    EditOp::Brush(p) => ("brush", p.map.content_hash(), 0),
                    EditOp::Healing(_) => ("healing", 0, 0),

                    EditOp::Halation(_) => ("halation", 0, 0),
                    EditOp::Bloom(_) => ("bloom", 0, 0),
                    EditOp::GrainyFilm(_) => ("grainy_film", 0, 0),
                    EditOp::Noir(_) => ("noir", 0, 0),
                    EditOp::HdrScape(_) => ("hdr_scape", 0, 0),
                    EditOp::GlamourGlow(_) => ("glamour_glow", 0, 0),
                    EditOp::Drama(_) => ("drama", 0, 0),
                    EditOp::Vintage(_) => ("vintage", 0, 0),
                    EditOp::DoubleExposure(p) => {
                        let sec_gen = p
                            .secondary_path
                            .as_ref()
                            .and_then(|path| self.secondary_images.get(path))
                            .map(|(g, _, _, _)| *g)
                            .unwrap_or(0);
                        ("double_exposure", 0, sec_gen)
                    }
                    EditOp::Retrolux(_) => ("retrolux", 0, 0),
                    EditOp::Grunge(_) => ("grunge", 0, 0),

                    EditOp::Perspective(_) => ("perspective", 0, 0),
                    EditOp::Expand(_) => ("expand", 0, 0),
                    EditOp::LensBlur(_) => ("lens_blur", 0, 0),
                    EditOp::MotionBlur(_) => ("motion_blur", 0, 0),

                    EditOp::Portrait(_) => ("portrait", 0, 0),
                    EditOp::HeadPose(_) => ("head_pose", 0, 0),
                    EditOp::Frames(_) => ("frames", 0, 0),
                    EditOp::TextOverlay(_) => ("text_overlay", 0, 0),
                };

                let mask_hash = n.mask.as_ref().map(|m| m.content_hash());

                // Stable payload hash via string representation
                let s = format!("{:?}", n.op);
                let mut hash = 0xcbf29ce484222325u64;
                for b in s.as_bytes() {
                    hash = (hash ^ (*b as u64)).wrapping_mul(0x100000001b3);
                }

                NodeFingerprint {
                    id: *n.id.as_bytes(),
                    enabled: n.enabled,
                    opacity: (n.opacity * 1000.0).round() as u32,
                    op_tag,
                    payload_hash: hash,
                    mask_hash,
                    brush_hash,
                    secondary_gen,
                }
            })
            .collect();

        let current_key = DirtyCacheKey {
            image_id: self.uploaded_image_id.unwrap_or(0),
            width,
            height,
            bypass_edits,
            nodes: node_fingerprints,
        };

        // If dirty state matches cached state, return existing processed view immediately!
        if self.last_dirty_key.as_ref() == Some(&current_key) {
            return self.get_current_output_view();
        }

        self.dirty_generation = self.dirty_generation.wrapping_add(1);

        if effective_nodes.is_empty() {
            self.cached_output_is_source = true;
            self.last_dirty_key = Some(current_key);
            return self.get_current_output_view();
        }

        // We have active edits to evaluate
        self.ensure_ping_pong_textures(device, width, height);

        // Pre-upload any masks, brush maps, or text overlays required by active nodes
        for node in &effective_nodes {
            if let Some(mask) = &node.mask {
                if !mask.is_uniform_full() {
                    self.ensure_mask(device, queue, mask);
                }
            }
            match &node.op {
                EditOp::Brush(p) => {
                    self.ensure_brush_map(device, queue, &p.map);
                }
                EditOp::TextOverlay(p) => {
                    self.ensure_text_overlay(device, queue, p, width, height);
                }
                _ => {}
            }
        }

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("gpu_process_stack_encoder"),
        });

        // Ping-pong state tracking
        let mut first_pass = true;
        let mut in_a = false;

        let workgroups_x = (width + 15) / 16;
        let workgroups_y = (height + 15) / 16;

        let mut retained_buffers = Vec::new();
        let mut retained_textures = Vec::new();

        for node in &effective_nodes {
            if matches!(node.op, EditOp::CropRotate(_)) {
                continue;
            }

            let opacity = node.opacity.clamp(0.0, 1.0);
            let has_custom_mask = node.mask.as_ref().is_some_and(|m| !m.is_uniform_full());
            let op_opacity = if has_custom_mask { 1.0 } else { opacity };

            let (src_view, src_tex) = if first_pass {
                (self.source_view.as_ref().unwrap(), self.source_texture.as_ref().unwrap())
            } else if in_a {
                (self.view_a.as_ref().unwrap(), self.tex_a.as_ref().unwrap())
            } else {
                (self.view_b.as_ref().unwrap(), self.tex_b.as_ref().unwrap())
            };

            let (post_view, other_view, is_other_view_a) = if first_pass {
                (self.view_a.as_ref().unwrap(), self.view_b.as_ref().unwrap(), false)
            } else if in_a {
                (self.view_b.as_ref().unwrap(), self.view_a.as_ref().unwrap(), true)
            } else {
                (self.view_a.as_ref().unwrap(), self.view_b.as_ref().unwrap(), false)
            };

            let dst_view = post_view;

            if has_custom_mask {
                encoder.copy_texture_to_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: src_tex,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    wgpu::TexelCopyTextureInfo {
                        texture: self.tex_pre.as_ref().unwrap(),
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                );
            }

            macro_rules! dispatch_compute_bgl {
                ($pipeline:expr, $uniforms:expr, $label:expr) => {{
                    let buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some(concat!("gpu_", $label, "_buf")),
                        contents: bytemuck::bytes_of(&$uniforms),
                        usage: wgpu::BufferUsages::UNIFORM,
                    });
                    let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some(concat!($label, "_bg")),
                        layout: &self.compute_bgl,
                        entries: &[
                            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(src_view) },
                            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(dst_view) },
                            wgpu::BindGroupEntry { binding: 2, resource: buf.as_entire_binding() },
                        ],
                    });
                    let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some(concat!($label, "_cpass")),
                        timestamp_writes: None,
                    });
                    cpass.set_pipeline($pipeline);
                    cpass.set_bind_group(0, &bg, &[]);
                    cpass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
                    drop(cpass);
                    retained_buffers.push(buf);
                }};
            }

            macro_rules! dispatch_separable_bgl {
                ($pipeline:expr, $u1:expr, $u2:expr, $label:expr) => {{
                    let buf1 = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some(concat!("gpu_", $label, "_buf_pass1")),
                        contents: bytemuck::bytes_of(&$u1),
                        usage: wgpu::BufferUsages::UNIFORM,
                    });
                    let bg1 = device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some(concat!($label, "_bg_pass1")),
                        layout: &self.compute_bgl,
                        entries: &[
                            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(src_view) },
                            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(self.view_temp.as_ref().unwrap()) },
                            wgpu::BindGroupEntry { binding: 2, resource: buf1.as_entire_binding() },
                        ],
                    });
                    {
                        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                            label: Some(concat!($label, "_cpass1")),
                            timestamp_writes: None,
                        });
                        cpass.set_pipeline($pipeline);
                        cpass.set_bind_group(0, &bg1, &[]);
                        cpass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
                    }
                    let buf2 = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some(concat!("gpu_", $label, "_buf_pass2")),
                        contents: bytemuck::bytes_of(&$u2),
                        usage: wgpu::BufferUsages::UNIFORM,
                    });
                    let bg2 = device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some(concat!($label, "_bg_pass2")),
                        layout: &self.compute_bgl,
                        entries: &[
                            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(self.view_temp.as_ref().unwrap()) },
                            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(dst_view) },
                            wgpu::BindGroupEntry { binding: 2, resource: buf2.as_entire_binding() },
                        ],
                    });
                    {
                        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                            label: Some(concat!($label, "_cpass2")),
                            timestamp_writes: None,
                        });
                        cpass.set_pipeline($pipeline);
                        cpass.set_bind_group(0, &bg2, &[]);
                        cpass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
                    }
                    retained_buffers.push(buf1);
                    retained_buffers.push(buf2);
                }};
            }

            match &node.op {
                EditOp::RawDevelop(p) => {
                    let u = RawDevelopUniforms {
                        exposure_ev: p.exposure_ev,
                        highlights: p.highlights,
                        shadows: p.shadows,
                        contrast: p.contrast,
                        temperature: p.temperature,
                        tint: p.tint,
                        saturation: p.saturation,
                        opacity: op_opacity,
                    };
                    dispatch_compute_bgl!(&self.raw_develop_pipeline, u, "raw_develop");
                }

                EditOp::WhiteBalance(p) => {
                    let u = WhiteBalanceUniforms {
                        temperature: p.temperature,
                        tint: p.tint,
                        opacity: op_opacity,
                        _pad: 0.0,
                    };
                    dispatch_compute_bgl!(&self.white_balance_pipeline, u, "white_balance");
                }

                EditOp::TuneImage(p) => {
                    let u = TuneImageUniforms {
                        brightness: p.brightness,
                        contrast: p.contrast,
                        saturation: p.saturation,
                        ambiance: p.ambiance,
                        highlights: p.highlights,
                        shadows: p.shadows,
                        warmth: p.warmth,
                        opacity: op_opacity,
                    };
                    dispatch_compute_bgl!(&self.tune_image_pipeline, u, "tune_image");
                }

                EditOp::ColorHsl(p) => {
                    let mut bands_gpu = [ColorHslBandGpu { hue: 0.0, saturation: 0.0, luminance: 0.0, _pad: 0.0 }; 8];
                    for i in 0..8 {
                        bands_gpu[i].hue = p.bands[i].hue;
                        bands_gpu[i].saturation = p.bands[i].saturation;
                        bands_gpu[i].luminance = p.bands[i].luminance;
                    }
                    let u = ColorHslUniforms {
                        bands: bands_gpu,
                        opacity: op_opacity,
                        _pad0: 0.0,
                        _pad1: 0.0,
                        _pad2: 0.0,
                    };
                    dispatch_compute_bgl!(&self.color_hsl_pipeline, u, "color_hsl");
                }

                EditOp::BlackAndWhite(p) => {
                    let u = BlackAndWhiteUniforms {
                        channel_weights: p.color_filter.channel_weights(),
                        brightness: p.brightness,
                        contrast: p.contrast,
                        grain: p.grain,
                        opacity: op_opacity,
                        _pad: 0.0,
                    };
                    dispatch_compute_bgl!(&self.black_and_white_pipeline, u, "black_and_white");
                }

                EditOp::Vignette(p) => {
                    let u = VignetteUniforms {
                        center: p.center,
                        inner_radius: p.inner_radius,
                        outer_radius: p.outer_radius,
                        outer_brightness: p.outer_brightness,
                        inner_brightness: p.inner_brightness,
                        aspect: width as f32 / height as f32,
                        opacity: op_opacity,
                    };
                    dispatch_compute_bgl!(&self.vignette_pipeline, u, "vignette");
                }

                EditOp::ColorGrade(p) => {
                    let u = ColorGradeUniforms::from_params(p, op_opacity);
                    dispatch_compute_bgl!(&self.color_grade_pipeline, u, "color_grade");
                }

                EditOp::Dehaze(p) => {
                    let u = DehazeUniforms::from_params(p, op_opacity);
                    dispatch_compute_bgl!(&self.dehaze_pipeline, u, "dehaze");
                }

                EditOp::GrainyFilm(p) => {
                    let u = GrainyFilmUniforms::from_params(p, op_opacity);
                    dispatch_compute_bgl!(&self.grainy_film_pipeline, u, "grainy_film");
                }

                EditOp::Noir(p) => {
                    let u = NoirUniforms::from_params(p, op_opacity);
                    dispatch_compute_bgl!(&self.noir_pipeline, u, "noir");
                }

                EditOp::Selective(p) => {
                    let u = SelectiveUniforms::from_params(p, op_opacity);
                    dispatch_compute_bgl!(&self.selective_pipeline, u, "selective");
                }

                EditOp::Healing(p) => {
                    let u = HealingUniforms::from_params(p, width, height, op_opacity);
                    dispatch_compute_bgl!(&self.healing_pipeline, u, "healing");
                }

                EditOp::Perspective(p) => {
                    let u = PerspectiveUniforms::from_params(p, width, height, op_opacity);
                    dispatch_compute_bgl!(&self.perspective_pipeline, u, "perspective");
                }

                EditOp::Expand(p) => {
                    let u = ExpandUniforms::from_params(p, width, height, op_opacity);
                    dispatch_compute_bgl!(&self.expand_pipeline, u, "expand");
                }

                EditOp::LensBlur(p) => {
                    let u = LensBlurUniforms::from_params(p, width, height, op_opacity);
                    dispatch_compute_bgl!(&self.lens_blur_pipeline, u, "lens_blur");
                }

                EditOp::MotionBlur(p) => {
                    let u = MotionBlurUniforms::from_params(p, width, height, op_opacity);
                    dispatch_compute_bgl!(&self.motion_blur_pipeline, u, "motion_blur");
                }

                EditOp::Drama(p) => {
                    let u = DramaUniforms::from_params(p, op_opacity);
                    dispatch_compute_bgl!(&self.drama_pipeline, u, "drama");
                }

                EditOp::Vintage(p) => {
                    let u = VintageUniforms::from_params(p, op_opacity);
                    dispatch_compute_bgl!(&self.vintage_pipeline, u, "vintage");
                }

                EditOp::Retrolux(p) => {
                    let u = RetroluxUniforms::from_params(p, op_opacity);
                    dispatch_compute_bgl!(&self.retrolux_pipeline, u, "retrolux");
                }

                EditOp::Grunge(p) => {
                    let u = GrungeUniforms::from_params(p, op_opacity);
                    dispatch_compute_bgl!(&self.grunge_pipeline, u, "grunge");
                }

                EditOp::Portrait(p) => {
                    let u = PortraitUniforms::from_params(p, width, height, op_opacity);
                    dispatch_compute_bgl!(&self.portrait_pipeline, u, "portrait");
                }

                EditOp::HeadPose(p) => {
                    let u = HeadPoseUniforms::from_params(p, width, height, op_opacity);
                    dispatch_compute_bgl!(&self.head_pose_pipeline, u, "head_pose");
                }

                EditOp::Frames(p) => {
                    let u = FramesUniforms::from_params(p, width, height, op_opacity);
                    dispatch_compute_bgl!(&self.frames_pipeline, u, "frames");
                }

                // 6 Separable 2-Pass Ops:
                EditOp::Details(p) => {
                    let u1 = DetailsUniforms {
                        structure: p.structure,
                        sharpening: p.sharpening,
                        opacity: op_opacity,
                        pass_idx: 0,
                        width: width as f32,
                        height: height as f32,
                        _pad0: 0.0,
                        _pad1: 0.0,
                    };
                    let u2 = DetailsUniforms {
                        structure: p.structure,
                        sharpening: p.sharpening,
                        opacity: op_opacity,
                        pass_idx: 1,
                        width: width as f32,
                        height: height as f32,
                        _pad0: 0.0,
                        _pad1: 0.0,
                    };
                    dispatch_separable_bgl!(&self.details_pipeline, u1, u2, "details");
                }

                EditOp::TonalContrast(p) => {
                    let u1 = TonalContrastUniforms::from_params(p, op_opacity, 0);
                    let u2 = TonalContrastUniforms::from_params(p, op_opacity, 1);
                    dispatch_separable_bgl!(&self.tonal_contrast_pipeline, u1, u2, "tonal_contrast");
                }

                EditOp::Halation(p) => {
                    let u1 = HalationUniforms::from_params(p, op_opacity, 0);
                    let u2 = HalationUniforms::from_params(p, op_opacity, 1);
                    dispatch_separable_bgl!(&self.halation_pipeline, u1, u2, "halation");
                }

                EditOp::Bloom(p) => {
                    let u1 = BloomUniforms::from_params(p, op_opacity, 0);
                    let u2 = BloomUniforms::from_params(p, op_opacity, 1);
                    dispatch_separable_bgl!(&self.bloom_pipeline, u1, u2, "bloom");
                }

                EditOp::HdrScape(p) => {
                    let u1 = HdrScapeUniforms::from_params(p, 0, op_opacity);
                    let u2 = HdrScapeUniforms::from_params(p, 1, op_opacity);
                    dispatch_separable_bgl!(&self.hdr_scape_pipeline, u1, u2, "hdr_scape");
                }

                EditOp::GlamourGlow(p) => {
                    let u1 = GlamourGlowUniforms::from_params(p, 0, op_opacity);
                    let u2 = GlamourGlowUniforms::from_params(p, 1, op_opacity);
                    dispatch_separable_bgl!(&self.glamour_glow_pipeline, u1, u2, "glamour_glow");
                }

                // Custom BGL Ops:
                EditOp::Curves(p) => {
                    let (curve_texture, curve_view) = Self::create_curve_texture(device, queue, p);
                    let uniforms = CurvesUniforms {
                        opacity: op_opacity,
                        _pad0: 0.0,
                        _pad1: 0.0,
                        _pad2: 0.0,
                    };
                    let curves_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("gpu_curves_buf"),
                        contents: bytemuck::bytes_of(&uniforms),
                        usage: wgpu::BufferUsages::UNIFORM,
                    });

                    let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("curves_bg"),
                        layout: &self.curves_bgl,
                        entries: &[
                            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(src_view) },
                            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(dst_view) },
                            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&curve_view) },
                            wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(&self.linear_clamp_sampler) },
                            wgpu::BindGroupEntry { binding: 4, resource: curves_buf.as_entire_binding() },
                        ],
                    });

                    let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("curves_cpass"),
                        timestamp_writes: None,
                    });
                    cpass.set_pipeline(&self.curves_pipeline);
                    cpass.set_bind_group(0, &bg, &[]);
                    cpass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
                    drop(cpass);

                    retained_textures.push(curve_texture);
                    retained_buffers.push(curves_buf);
                }

                EditOp::Lut3d(p) => {
                    let gpu_lut = self.lut_cache.get_or_fallback(&p.lut_id);
                    let input_space_u32 = match p.input_space {
                        snap_core::image_types::LutColorSpace::Srgb => 0u32,
                        snap_core::image_types::LutColorSpace::Rec709 => 1u32,
                        snap_core::image_types::LutColorSpace::LinearRec2020 => 2u32,
                        snap_core::image_types::LutColorSpace::AppleLog => 3u32,
                        snap_core::image_types::LutColorSpace::SonySLog3 => 4u32,
                        snap_core::image_types::LutColorSpace::CanonLog2 => 5u32,
                    };
                    let uniforms = Lut3dUniforms {
                        domain_min: gpu_lut.domain_min,
                        lut_size: gpu_lut.size as f32,
                        domain_max: gpu_lut.domain_max,
                        strength: p.strength,
                        input_space: input_space_u32,
                        opacity: op_opacity,
                        _pad0: 0.0,
                        _pad1: 0.0,
                    };
                    let lut_3d_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("gpu_lut_3d_buf"),
                        contents: bytemuck::bytes_of(&uniforms),
                        usage: wgpu::BufferUsages::UNIFORM,
                    });

                    let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("lut_3d_bg"),
                        layout: &self.lut_3d_bgl,
                        entries: &[
                            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(src_view) },
                            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(dst_view) },
                            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&gpu_lut.view) },
                            wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(self.lut_cache.sampler()) },
                            wgpu::BindGroupEntry { binding: 4, resource: lut_3d_buf.as_entire_binding() },
                        ],
                    });

                    let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("lut_3d_cpass"),
                        timestamp_writes: None,
                    });
                    cpass.set_pipeline(&self.lut_3d_pipeline);
                    cpass.set_bind_group(0, &bg, &[]);
                    cpass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
                    drop(cpass);

                    retained_buffers.push(lut_3d_buf);
                }

                EditOp::Brush(p) => {
                    let brush_hash = p.map.content_hash();
                    let brush_view = &self.brush_cache.get(&brush_hash).unwrap().1;
                    let uniforms = BrushUniforms::from_params(p, op_opacity);
                    let brush_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("gpu_brush_buf"),
                        contents: bytemuck::bytes_of(&uniforms),
                        usage: wgpu::BufferUsages::UNIFORM,
                    });

                    let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("brush_bg"),
                        layout: &self.brush_bgl,
                        entries: &[
                            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(src_view) },
                            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(dst_view) },
                            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(brush_view) },
                            wgpu::BindGroupEntry { binding: 3, resource: brush_buf.as_entire_binding() },
                        ],
                    });

                    let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("brush_cpass"),
                        timestamp_writes: None,
                    });
                    cpass.set_pipeline(&self.brush_pipeline);
                    cpass.set_bind_group(0, &bg, &[]);
                    cpass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
                    drop(cpass);

                    retained_buffers.push(brush_buf);
                }

                EditOp::DoubleExposure(p) => {
                    let (has_sec, sec_view) = if let Some(path) = &p.secondary_path {
                        if let Some((_, _, view, _)) = self.secondary_images.get(path) {
                            (true, view)
                        } else {
                            (false, &self.fallback_black_view)
                        }
                    } else {
                        (false, &self.fallback_black_view)
                    };

                    let uniforms = DoubleExposureUniforms::from_params(p, has_sec, op_opacity);
                    let de_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("gpu_double_exposure_buf"),
                        contents: bytemuck::bytes_of(&uniforms),
                        usage: wgpu::BufferUsages::UNIFORM,
                    });

                    let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("double_exposure_bg"),
                        layout: &self.curves_bgl,
                        entries: &[
                            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(src_view) },
                            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(dst_view) },
                            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(sec_view) },
                            wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(&self.linear_clamp_sampler) },
                            wgpu::BindGroupEntry { binding: 4, resource: de_buf.as_entire_binding() },
                        ],
                    });

                    let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("double_exposure_cpass"),
                        timestamp_writes: None,
                    });
                    cpass.set_pipeline(&self.double_exposure_pipeline);
                    cpass.set_bind_group(0, &bg, &[]);
                    cpass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
                    drop(cpass);

                    retained_buffers.push(de_buf);
                }

                EditOp::TextOverlay(p) => {
                    let aspect = width as f32 / (height.max(1) as f32);
                    let text_hash = crate::text_raster::compute_text_cache_key(p) ^ (aspect.to_bits() as u64);
                    let text_view = &self.text_cache.get(&text_hash).unwrap().1;
                    let uniforms = TextOverlayUniforms::from_params(p, op_opacity);
                    let text_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("gpu_text_overlay_buf"),
                        contents: bytemuck::bytes_of(&uniforms),
                        usage: wgpu::BufferUsages::UNIFORM,
                    });

                    let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("text_overlay_bg"),
                        layout: &self.curves_bgl,
                        entries: &[
                            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(src_view) },
                            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(dst_view) },
                            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(text_view) },
                            wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(&self.linear_clamp_sampler) },
                            wgpu::BindGroupEntry { binding: 4, resource: text_buf.as_entire_binding() },
                        ],
                    });

                    let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("text_overlay_cpass"),
                        timestamp_writes: None,
                    });
                    cpass.set_pipeline(&self.text_overlay_pipeline);
                    cpass.set_bind_group(0, &bg, &[]);
                    cpass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
                    drop(cpass);

                    retained_buffers.push(text_buf);
                }

                EditOp::CropRotate(_) => {
                    // Handled in viewport quad or full-res export, skipped here
                    continue;
                }
            }

            if has_custom_mask {
                let mask = node.mask.as_ref().unwrap();
                let mask_hash = mask.content_hash();
                let mask_view = &self.mask_cache.get(&mask_hash).unwrap().1;

                let mask_uniforms = MaskBlendUniforms::from_params(opacity, true);
                let mask_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("gpu_mask_blend_buf"),
                    contents: bytemuck::bytes_of(&mask_uniforms),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

                let mask_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("mask_blend_bg"),
                    layout: &self.mask_blend_bgl,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(self.view_pre.as_ref().unwrap()),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(post_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::TextureView(other_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: wgpu::BindingResource::TextureView(mask_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 4,
                            resource: wgpu::BindingResource::Sampler(&self.linear_clamp_sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 5,
                            resource: mask_buf.as_entire_binding(),
                        },
                    ],
                });

                {
                    let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("mask_blend_cpass"),
                        timestamp_writes: None,
                    });
                    cpass.set_pipeline(&self.mask_blend_pipeline);
                    cpass.set_bind_group(0, &mask_bg, &[]);
                    cpass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
                }

                retained_buffers.push(mask_buf);
                first_pass = false;
                in_a = is_other_view_a;
            } else {
                if first_pass {
                    first_pass = false;
                    in_a = true;
                } else {
                    in_a = !in_a;
                }
            }
        }

        queue.submit(Some(encoder.finish()));
        drop(retained_buffers);
        drop(retained_textures);

        self.cached_output_is_source = first_pass;
        self.cached_output_in_a = in_a;
        self.last_dirty_key = Some(current_key);

        self.get_current_output_view()
    }

    pub fn get_current_output_view(&self) -> Result<&wgpu::TextureView, GpuError> {
        if self.cached_output_is_source {
            self.source_view
                .as_ref()
                .ok_or_else(|| GpuError::TextureAllocationFailed("Source texture missing".into()))
        } else if self.cached_output_in_a {
            self.view_a
                .as_ref()
                .ok_or_else(|| GpuError::TextureAllocationFailed("Tex A missing".into()))
        } else {
            self.view_b
                .as_ref()
                .ok_or_else(|| GpuError::TextureAllocationFailed("Tex B missing".into()))
        }
    }

    pub fn get_current_output_texture(&self) -> Result<&wgpu::Texture, GpuError> {
        if self.cached_output_is_source {
            self.source_texture
                .as_ref()
                .ok_or_else(|| GpuError::TextureAllocationFailed("Source texture missing".into()))
        } else if self.cached_output_in_a {
            self.tex_a
                .as_ref()
                .ok_or_else(|| GpuError::TextureAllocationFailed("Tex A missing".into()))
        } else {
            self.tex_b
                .as_ref()
                .ok_or_else(|| GpuError::TextureAllocationFailed("Tex B missing".into()))
        }
    }

    /// Compute 256-bin RGB and Luma histogram synchronously on current processed texture.
    pub fn compute_histogram_sync(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<HistogramData, GpuError> {
        let (width, height) = self.source_dims;
        if width == 0 || height == 0 {
            return Err(GpuError::InvalidDimensions(width, height));
        }

        let output_view = self.get_current_output_view()?;

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("gpu_histogram_encoder"),
        });

        // Zero out the histogram storage buffer
        encoder.clear_buffer(&self.hist_storage_buf, 0, None);

        let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("histogram_bg"),
            layout: &self.histogram_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(output_view) },
                wgpu::BindGroupEntry { binding: 1, resource: self.hist_storage_buf.as_entire_binding() },
            ],
        });

        let workgroups_x = (width + 15) / 16;
        let workgroups_y = (height + 15) / 16;

        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("histogram_cpass"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&self.histogram_pipeline);
            cpass.set_bind_group(0, &bg, &[]);
            cpass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }

        // Copy storage buffer to staging buffer for readback
        encoder.copy_buffer_to_buffer(
            &self.hist_storage_buf,
            0,
            &self.hist_staging_buf,
            0,
            1024 * 4,
        );

        queue.submit(Some(encoder.finish()));

        // Map staging buffer
        let slice = self.hist_staging_buf.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |res| {
            let _ = tx.send(res);
        });

        let _ = device.poll(wgpu::PollType::wait_indefinitely());

        rx.recv()
            .map_err(|e| GpuError::BufferAsyncError(format!("Channel recv error: {:?}", e)))?
            .map_err(|e| GpuError::BufferAsyncError(format!("Buffer map error: {:?}", e)))?;

        let mapped = slice.get_mapped_range();
        let bins: &[u32] = bytemuck::cast_slice(&mapped);

        let mut r = [0u32; 256];
        let mut g = [0u32; 256];
        let mut b = [0u32; 256];
        let mut luma = [0u32; 256];

        r.copy_from_slice(&bins[0..256]);
        g.copy_from_slice(&bins[256..512]);
        b.copy_from_slice(&bins[512..768]);
        luma.copy_from_slice(&bins[768..1024]);

        drop(mapped);
        self.hist_staging_buf.unmap();

        Ok(HistogramData::from_bins(r, g, b, luma))
    }

    /// Render viewport quad into `target: &wgpu::TextureView` within `clip_bounds: [u32; 4]` (`x, y, width, height`).
    pub fn render_viewport(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        target_format: wgpu::TextureFormat,
        clip_bounds: [u32; 4],
        params: &ViewportParams,
    ) {
        let (src_w, src_h) = self.source_dims;
        if src_w == 0 || src_h == 0 || clip_bounds[2] == 0 || clip_bounds[3] == 0 {
            return;
        }

        let vp_width = clip_bounds[2];
        let vp_height = clip_bounds[3];

        // Ensure pipeline for target format exists
        if !self.viewport_pipelines.contains_key(&target_format) {
            let pipe = Self::build_viewport_pipeline(
                device,
                &self.viewport_pipeline_layout,
                &self.viewport_shader_module,
                target_format,
            );
            self.viewport_pipelines.insert(target_format, pipe);
        }
        let pipeline = &self.viewport_pipelines[&target_format];

        let output_view = match self.get_current_output_view() {
            Ok(v) => v,
            Err(_) => return,
        };

        let inscribed_scale = CropRotateParams::inscribed_max_scale(
            src_w as f32 / src_h as f32,
            params.crop.straighten_deg,
        );

        let flip_flags = (if params.crop.flip_h { 1u32 } else { 0u32 })
            | (if params.crop.flip_v { 2u32 } else { 0u32 });

        let uniforms = ViewportUniforms {
            viewport_size: [vp_width as f32, vp_height as f32],
            image_size: [src_w as f32, src_h as f32],
            crop_rect: [
                params.crop.rect.x,
                params.crop.rect.y,
                params.crop.rect.width,
                params.crop.rect.height,
            ],
            pan: params.pan,
            zoom: params.zoom.max(0.01),
            straighten_rad: params.crop.straighten_deg.to_radians(),
            inscribed_scale,
            quarter_turns: params.crop.quarter_turns_cw as u32,
            flip_flags,
            crop_preview_mode: if params.crop_preview_mode { 1 } else { 0 },
            output_is_srgb: if target_format.is_srgb() { 1 } else { 0 },
            _pad0: 0.0,
            _pad1: 0.0,
            _pad2: 0.0,
        };

        queue.write_buffer(&self.viewport_buf, 0, bytemuck::bytes_of(&uniforms));

        let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("viewport_render_bg"),
            layout: &self.viewport_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(output_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.linear_clamp_sampler) },
                wgpu::BindGroupEntry { binding: 2, resource: self.viewport_buf.as_entire_binding() },
            ],
        });

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("viewport_rpass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            rpass.set_viewport(
                clip_bounds[0] as f32,
                clip_bounds[1] as f32,
                vp_width as f32,
                vp_height as f32,
                0.0,
                1.0,
            );
            rpass.set_scissor_rect(clip_bounds[0], clip_bounds[1], vp_width, vp_height);
            rpass.set_pipeline(pipeline);
            rpass.set_bind_group(0, &bg, &[]);
            rpass.draw(0..3, 0..1);
        }
    }

    /// Helper to render the viewport without an external command encoder.
    pub fn render_viewport_standalone(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target: &wgpu::TextureView,
        target_format: wgpu::TextureFormat,
        clip_bounds: [u32; 4],
        params: &ViewportParams,
    ) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("viewport_standalone_encoder"),
        });
        self.render_viewport(device, queue, &mut encoder, target, target_format, clip_bounds, params);
        queue.submit(Some(encoder.finish()));
    }

    /// Map canvas mouse click coordinates (relative to clip_bounds top-left)
    /// to normalized `[0..1]` image UV coordinates.
    pub fn screen_pos_to_image_uv(
        screen_pos: [f32; 2],
        clip_bounds: [u32; 4],
        image_w: u32,
        image_h: u32,
        params: &ViewportParams,
    ) -> Option<[f32; 2]> {
        let vp_w = clip_bounds[2] as f32;
        let vp_h = clip_bounds[3] as f32;
        if vp_w <= 0.0 || vp_h <= 0.0 || image_w == 0 || image_h == 0 {
            return None;
        }

        let turns = params.crop.quarter_turns_cw % 4;
        let (base_w, base_h) = if turns == 1 || turns == 3 {
            (image_h as f32, image_w as f32)
        } else {
            (image_w as f32, image_h as f32)
        };

        let (disp_w, disp_h) = if !params.crop_preview_mode {
            (base_w * params.crop.rect.width, base_h * params.crop.rect.height)
        } else {
            (base_w, base_h)
        };

        let img_aspect = disp_w / disp_h.max(1e-4);
        let vp_aspect = vp_w / vp_h.max(1e-4);

        let (fit_w, fit_h) = if img_aspect > vp_aspect {
            (vp_w, vp_w / img_aspect)
        } else {
            (vp_h * img_aspect, vp_h)
        };

        let quad_w = fit_w * params.zoom.max(0.01);
        let quad_h = fit_h * params.zoom.max(0.01);

        let center_x = vp_w * 0.5 + params.pan[0];
        let center_y = vp_h * 0.5 + params.pan[1];

        let rel_x = screen_pos[0] - center_x;
        let rel_y = screen_pos[1] - center_y;

        let quad_u = rel_x / quad_w + 0.5;
        let quad_v = rel_y / quad_h + 0.5;

        if quad_u < 0.0 || quad_u > 1.0 || quad_v < 0.0 || quad_v > 1.0 {
            return None;
        }

        let (mut u_base, mut v_base) = if !params.crop_preview_mode {
            (
                params.crop.rect.x + quad_u * params.crop.rect.width,
                params.crop.rect.y + quad_v * params.crop.rect.height,
            )
        } else {
            (quad_u, quad_v)
        };

        // Undo flip
        if params.crop.flip_h {
            u_base = 1.0 - u_base;
        }
        if params.crop.flip_v {
            v_base = 1.0 - v_base;
        }

        // Undo quarter turns
        let mut u2 = u_base - 0.5;
        let mut v2 = v_base - 0.5;
        if turns == 1 {
            let temp = u2;
            u2 = v2;
            v2 = -temp;
        } else if turns == 2 {
            u2 = -u2;
            v2 = -v2;
        } else if turns == 3 {
            let temp = u2;
            u2 = -v2;
            v2 = temp;
        }

        // Undo straighten
        let s = CropRotateParams::inscribed_max_scale(
            image_w as f32 / image_h as f32,
            params.crop.straighten_deg,
        );
        let dx = u2 * s;
        let dy = v2 * s;

        let aspect = image_w as f32 / image_h as f32;
        let dx_phys = dx * aspect;
        let dy_phys = dy;

        let angle_rad = params.crop.straighten_deg.to_radians();
        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();
        let rot_x = dx_phys * cos_a + dy_phys * sin_a;
        let rot_y = -dx_phys * sin_a + dy_phys * cos_a;

        let u_src = (0.5 + rot_x / aspect).clamp(0.0, 1.0);
        let v_src = (0.5 + rot_y).clamp(0.0, 1.0);

        Some([u_src, v_src])
    }

    /// Run full-resolution geometry transformation (crop/straighten/flip/rotate)
    pub fn execute_crop_transform(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        src_tex_view: &wgpu::TextureView,
        src_w: u32,
        src_h: u32,
        dst_tex_view: &wgpu::TextureView,
        dst_w: u32,
        dst_h: u32,
        crop: &CropRotateParams,
    ) {
        let inscribed_scale = CropRotateParams::inscribed_max_scale(
            src_w as f32 / src_h as f32,
            crop.straighten_deg,
        );

        let flip_flags = (if crop.flip_h { 1u32 } else { 0u32 })
            | (if crop.flip_v { 2u32 } else { 0u32 });

        let uniforms = CropTransformUniforms {
            crop_rect: [
                crop.rect.x,
                crop.rect.y,
                crop.rect.width,
                crop.rect.height,
            ],
            src_size: [src_w as f32, src_h as f32],
            dst_size: [dst_w as f32, dst_h as f32],
            straighten_rad: crop.straighten_deg.to_radians(),
            inscribed_scale,
            quarter_turns: crop.quarter_turns_cw as u32,
            flip_flags,
        };

        let crop_transform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("gpu_crop_transform_buf"),
            contents: bytemuck::bytes_of(&uniforms),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("crop_transform_bg"),
            layout: &self.crop_transform_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(src_tex_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(dst_tex_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&self.linear_clamp_sampler) },
                wgpu::BindGroupEntry { binding: 3, resource: crop_transform_buf.as_entire_binding() },
            ],
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("crop_transform_encoder"),
        });

        let workgroups_x = (dst_w + 15) / 16;
        let workgroups_y = (dst_h + 15) / 16;

        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("crop_transform_cpass"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&self.crop_transform_pipeline);
            cpass.set_bind_group(0, &bg, &[]);
            cpass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }

        queue.submit(Some(encoder.finish()));
    }
}
