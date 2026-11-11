use std::collections::HashMap;
use snap_core::curves::CurvesParams;
use snap_core::image_types::{HistogramData, LinearImage, Lut3D};
use snap_core::ops::{CropRotateParams, EditOp};
use snap_core::stack::{EditNode, EditStack};
use wgpu::util::DeviceExt;

use crate::error::GpuError;
use crate::lut_cache::GpuLutCache;
use crate::shaders::*;
use crate::types::*;

#[derive(Clone, Debug, PartialEq)]
struct NodeFingerprint {
    id: [u8; 16],
    enabled: bool,
    opacity: u32, // quantized opacity for stable hash
    op_tag: &'static str,
    payload_hash: u64,
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

    // Compute pipelines
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
        if self.allocated_dims == (width, height) && self.tex_a.is_some() {
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

        self.tex_a = Some(a_tex);
        self.view_a = Some(a_view);
        self.tex_b = Some(b_tex);
        self.view_b = Some(b_view);
        self.tex_temp = Some(temp_tex);
        self.view_temp = Some(temp_view);
        self.allocated_dims = (width, height);
        self.last_dirty_key = None;
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
                let op_tag = match &n.op {
                    EditOp::RawDevelop(_) => "raw",
                    EditOp::WhiteBalance(_) => "wb",
                    EditOp::TuneImage(_) => "tune",
                    EditOp::ColorHsl(_) => "hsl",
                    EditOp::Curves(_) => "curves",
                    EditOp::Lut3d(_) => "lut",
                    EditOp::Details(_) => "details",
                    EditOp::BlackAndWhite(_) => "bw",
                    EditOp::Vignette(_) => "vig",
                    EditOp::CropRotate(_) => "crop",
                };
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

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("gpu_process_stack_encoder"),
        });

        // Current ping-pong state
        // ping = true means read from tex_a, write to tex_b
        // ping = false means read from tex_b, write to tex_a
        let mut first_pass = true;
        let mut in_a = false;

        let workgroups_x = (width + 15) / 16;
        let workgroups_y = (height + 15) / 16;

        let mut retained_buffers = Vec::new();
        let mut retained_textures = Vec::new();

        for node in &effective_nodes {
            let opacity = node.opacity.clamp(0.0, 1.0);

            let (src_view, dst_view) = if first_pass {
                (self.source_view.as_ref().unwrap(), self.view_a.as_ref().unwrap())
            } else if in_a {
                (self.view_a.as_ref().unwrap(), self.view_b.as_ref().unwrap())
            } else {
                (self.view_b.as_ref().unwrap(), self.view_a.as_ref().unwrap())
            };

            match &node.op {
                EditOp::RawDevelop(p) => {
                    let uniforms = RawDevelopUniforms {
                        exposure_ev: p.exposure_ev,
                        highlights: p.highlights,
                        shadows: p.shadows,
                        contrast: p.contrast,
                        temperature: p.temperature,
                        tint: p.tint,
                        saturation: p.saturation,
                        opacity,
                    };
                    let raw_develop_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("gpu_raw_develop_buf"),
                        contents: bytemuck::bytes_of(&uniforms),
                        usage: wgpu::BufferUsages::UNIFORM,
                    });

                    let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("raw_develop_bg"),
                        layout: &self.compute_bgl,
                        entries: &[
                            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(src_view) },
                            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(dst_view) },
                            wgpu::BindGroupEntry { binding: 2, resource: raw_develop_buf.as_entire_binding() },
                        ],
                    });

                    let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("raw_develop_cpass"),
                        timestamp_writes: None,
                    });
                    cpass.set_pipeline(&self.raw_develop_pipeline);
                    cpass.set_bind_group(0, &bg, &[]);
                    cpass.dispatch_workgroups(workgroups_x, workgroups_y, 1);

                    retained_buffers.push(raw_develop_buf);
                }

                EditOp::WhiteBalance(p) => {
                    let uniforms = WhiteBalanceUniforms {
                        temperature: p.temperature,
                        tint: p.tint,
                        opacity,
                        _pad: 0.0,
                    };
                    let white_balance_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("gpu_white_balance_buf"),
                        contents: bytemuck::bytes_of(&uniforms),
                        usage: wgpu::BufferUsages::UNIFORM,
                    });

                    let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("white_balance_bg"),
                        layout: &self.compute_bgl,
                        entries: &[
                            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(src_view) },
                            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(dst_view) },
                            wgpu::BindGroupEntry { binding: 2, resource: white_balance_buf.as_entire_binding() },
                        ],
                    });

                    let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("white_balance_cpass"),
                        timestamp_writes: None,
                    });
                    cpass.set_pipeline(&self.white_balance_pipeline);
                    cpass.set_bind_group(0, &bg, &[]);
                    cpass.dispatch_workgroups(workgroups_x, workgroups_y, 1);

                    retained_buffers.push(white_balance_buf);
                }

                EditOp::TuneImage(p) => {
                    let uniforms = TuneImageUniforms {
                        brightness: p.brightness,
                        contrast: p.contrast,
                        saturation: p.saturation,
                        ambiance: p.ambiance,
                        highlights: p.highlights,
                        shadows: p.shadows,
                        warmth: p.warmth,
                        opacity,
                    };
                    let tune_image_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("gpu_tune_image_buf"),
                        contents: bytemuck::bytes_of(&uniforms),
                        usage: wgpu::BufferUsages::UNIFORM,
                    });

                    let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("tune_image_bg"),
                        layout: &self.compute_bgl,
                        entries: &[
                            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(src_view) },
                            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(dst_view) },
                            wgpu::BindGroupEntry { binding: 2, resource: tune_image_buf.as_entire_binding() },
                        ],
                    });

                    let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("tune_image_cpass"),
                        timestamp_writes: None,
                    });
                    cpass.set_pipeline(&self.tune_image_pipeline);
                    cpass.set_bind_group(0, &bg, &[]);
                    cpass.dispatch_workgroups(workgroups_x, workgroups_y, 1);

                    retained_buffers.push(tune_image_buf);
                }

                EditOp::ColorHsl(p) => {
                    let mut bands_gpu = [ColorHslBandGpu { hue: 0.0, saturation: 0.0, luminance: 0.0, _pad: 0.0 }; 8];
                    for i in 0..8 {
                        bands_gpu[i].hue = p.bands[i].hue;
                        bands_gpu[i].saturation = p.bands[i].saturation;
                        bands_gpu[i].luminance = p.bands[i].luminance;
                    }
                    let uniforms = ColorHslUniforms {
                        bands: bands_gpu,
                        opacity,
                        _pad0: 0.0,
                        _pad1: 0.0,
                        _pad2: 0.0,
                    };
                    let color_hsl_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("gpu_color_hsl_buf"),
                        contents: bytemuck::bytes_of(&uniforms),
                        usage: wgpu::BufferUsages::UNIFORM,
                    });

                    let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("color_hsl_bg"),
                        layout: &self.compute_bgl,
                        entries: &[
                            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(src_view) },
                            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(dst_view) },
                            wgpu::BindGroupEntry { binding: 2, resource: color_hsl_buf.as_entire_binding() },
                        ],
                    });

                    let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("color_hsl_cpass"),
                        timestamp_writes: None,
                    });
                    cpass.set_pipeline(&self.color_hsl_pipeline);
                    cpass.set_bind_group(0, &bg, &[]);
                    cpass.dispatch_workgroups(workgroups_x, workgroups_y, 1);

                    retained_buffers.push(color_hsl_buf);
                }

                EditOp::Curves(p) => {
                    let (curve_texture, curve_view) = Self::create_curve_texture(device, queue, p);
                    let uniforms = CurvesUniforms {
                        opacity,
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
                        opacity,
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

                    retained_buffers.push(lut_3d_buf);
                }

                EditOp::Details(p) => {
                    // Two-pass separable bilateral filter
                    // Pass 1: Horizontal into tex_temp
                    let u_pass1 = DetailsUniforms {
                        structure: p.structure,
                        sharpening: p.sharpening,
                        opacity,
                        pass_idx: 0,
                        width: width as f32,
                        height: height as f32,
                        _pad0: 0.0,
                        _pad1: 0.0,
                    };
                    let details_buf_pass1 = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("gpu_details_buf_pass1"),
                        contents: bytemuck::bytes_of(&u_pass1),
                        usage: wgpu::BufferUsages::UNIFORM,
                    });

                    let bg1 = device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("details_bg_pass1"),
                        layout: &self.compute_bgl,
                        entries: &[
                            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(src_view) },
                            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(self.view_temp.as_ref().unwrap()) },
                            wgpu::BindGroupEntry { binding: 2, resource: details_buf_pass1.as_entire_binding() },
                        ],
                    });

                    {
                        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                            label: Some("details_cpass1"),
                            timestamp_writes: None,
                        });
                        cpass.set_pipeline(&self.details_pipeline);
                        cpass.set_bind_group(0, &bg1, &[]);
                        cpass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
                    }

                    // Pass 2: Vertical from tex_temp into dst_view
                    let u_pass2 = DetailsUniforms {
                        structure: p.structure,
                        sharpening: p.sharpening,
                        opacity,
                        pass_idx: 1,
                        width: width as f32,
                        height: height as f32,
                        _pad0: 0.0,
                        _pad1: 0.0,
                    };
                    let details_buf_pass2 = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("gpu_details_buf_pass2"),
                        contents: bytemuck::bytes_of(&u_pass2),
                        usage: wgpu::BufferUsages::UNIFORM,
                    });

                    let bg2 = device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("details_bg_pass2"),
                        layout: &self.compute_bgl,
                        entries: &[
                            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(self.view_temp.as_ref().unwrap()) },
                            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(dst_view) },
                            wgpu::BindGroupEntry { binding: 2, resource: details_buf_pass2.as_entire_binding() },
                        ],
                    });

                    {
                        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                            label: Some("details_cpass2"),
                            timestamp_writes: None,
                        });
                        cpass.set_pipeline(&self.details_pipeline);
                        cpass.set_bind_group(0, &bg2, &[]);
                        cpass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
                    }

                    retained_buffers.push(details_buf_pass1);
                    retained_buffers.push(details_buf_pass2);
                }

                EditOp::BlackAndWhite(p) => {
                    let uniforms = BlackAndWhiteUniforms {
                        channel_weights: p.color_filter.channel_weights(),
                        brightness: p.brightness,
                        contrast: p.contrast,
                        grain: p.grain,
                        opacity,
                        _pad: 0.0,
                    };
                    let black_and_white_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("gpu_black_and_white_buf"),
                        contents: bytemuck::bytes_of(&uniforms),
                        usage: wgpu::BufferUsages::UNIFORM,
                    });

                    let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("black_and_white_bg"),
                        layout: &self.compute_bgl,
                        entries: &[
                            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(src_view) },
                            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(dst_view) },
                            wgpu::BindGroupEntry { binding: 2, resource: black_and_white_buf.as_entire_binding() },
                        ],
                    });

                    let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("black_and_white_cpass"),
                        timestamp_writes: None,
                    });
                    cpass.set_pipeline(&self.black_and_white_pipeline);
                    cpass.set_bind_group(0, &bg, &[]);
                    cpass.dispatch_workgroups(workgroups_x, workgroups_y, 1);

                    retained_buffers.push(black_and_white_buf);
                }

                EditOp::Vignette(p) => {
                    let uniforms = VignetteUniforms {
                        center: p.center,
                        inner_radius: p.inner_radius,
                        outer_radius: p.outer_radius,
                        outer_brightness: p.outer_brightness,
                        inner_brightness: p.inner_brightness,
                        aspect: width as f32 / height as f32,
                        opacity,
                    };
                    let vignette_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("gpu_vignette_buf"),
                        contents: bytemuck::bytes_of(&uniforms),
                        usage: wgpu::BufferUsages::UNIFORM,
                    });

                    let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("vignette_bg"),
                        layout: &self.compute_bgl,
                        entries: &[
                            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(src_view) },
                            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(dst_view) },
                            wgpu::BindGroupEntry { binding: 2, resource: vignette_buf.as_entire_binding() },
                        ],
                    });

                    let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("vignette_cpass"),
                        timestamp_writes: None,
                    });
                    cpass.set_pipeline(&self.vignette_pipeline);
                    cpass.set_bind_group(0, &bg, &[]);
                    cpass.dispatch_workgroups(workgroups_x, workgroups_y, 1);

                    retained_buffers.push(vignette_buf);
                }

                EditOp::CropRotate(_) => {
                    // CropRotate is handled in viewport render quad or full-res export, not here!
                    continue;
                }
            }

            if first_pass {
                first_pass = false;
                in_a = true;
            } else {
                in_a = !in_a;
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
