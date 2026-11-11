use std::collections::HashMap;
use std::sync::Arc;
use snap_core::image_types::Lut3D;
use crate::error::GpuError;

pub struct GpuLut3D {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub size: u32,
    pub domain_min: [f32; 3],
    pub domain_max: [f32; 3],
    pub input_space: snap_core::image_types::LutColorSpace,
}

pub struct GpuLutCache {
    luts: HashMap<String, GpuLut3D>,
    sampler: Arc<wgpu::Sampler>,
    fallback_lut: GpuLut3D,
}

impl GpuLutCache {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let sampler = Arc::new(device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("gpu_lut_3d_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        }));

        let identity_lut = Lut3D::identity(3);
        let fallback_lut = Self::create_gpu_lut(device, queue, &identity_lut)
            .expect("Fallback 3x3x3 identity LUT creation must succeed");

        Self {
            luts: HashMap::new(),
            sampler,
            fallback_lut,
        }
    }

    pub fn sampler(&self) -> &wgpu::Sampler {
        &self.sampler
    }

    pub fn get_or_fallback(&self, lut_id: &str) -> &GpuLut3D {
        if lut_id.is_empty() {
            &self.fallback_lut
        } else {
            self.luts.get(lut_id).unwrap_or(&self.fallback_lut)
        }
    }

    pub fn upload_lut(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, lut: &Lut3D) -> Result<(), GpuError> {
        let gpu_lut = Self::create_gpu_lut(device, queue, lut)?;
        self.luts.insert(lut.id.clone(), gpu_lut);
        Ok(())
    }

    fn create_gpu_lut(device: &wgpu::Device, queue: &wgpu::Queue, lut: &Lut3D) -> Result<GpuLut3D, GpuError> {
        let size = lut.size;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&format!("lut_3d_tex_{}", lut.id)),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: size,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D3,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let bytes_per_pixel = 8u32;
        let bytes_per_row = size * bytes_per_pixel;
        let raw_data: &[u8] = bytemuck::cast_slice(&lut.data);

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            raw_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(size),
            },
            wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: size,
            },
        );

        Ok(GpuLut3D {
            texture,
            view,
            size,
            domain_min: lut.domain_min,
            domain_max: lut.domain_max,
            input_space: lut.input_space,
        })
    }
}
