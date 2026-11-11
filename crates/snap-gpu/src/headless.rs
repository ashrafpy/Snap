use snap_core::image_types::{LinearImage, Lut3D};
use snap_core::stack::EditStack;

use crate::engine::GpuPipelineEngine;
use crate::error::GpuError;

pub struct HeadlessGpu {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub engine: GpuPipelineEngine,
}

impl HeadlessGpu {
    pub fn new() -> Result<Self, GpuError> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .map_err(|_| GpuError::AdapterNotFound)?;

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("snap_headless_gpu_device"),
                ..Default::default()
            },
        ))
        .map_err(|e| GpuError::DeviceRequestFailed(e.to_string()))?;

        let engine = GpuPipelineEngine::new(&device, &queue, None);

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            engine,
        })
    }

    /// Read back an `Rgba16Float` texture from the GPU into a `LinearImage`.
    /// Automatically strips 256-byte `COPY_BYTES_PER_ROW_ALIGNMENT` row padding!
    pub fn readback_texture(
        &self,
        texture: &wgpu::Texture,
        width: u32,
        height: u32,
        metadata: snap_core::image_types::ImageMetadata,
    ) -> Result<LinearImage, GpuError> {
        let unpadded_bytes_per_row = width * 8; // 8 bytes per Rgba16Float texel
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT; // 256 bytes
        let padded_bytes_per_row = ((unpadded_bytes_per_row + align - 1) / align) * align;
        let buffer_size = (padded_bytes_per_row * height) as u64;

        let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback_staging_buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("readback_encoder"),
        });

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &staging_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        self.queue.submit(Some(encoder.finish()));

        let slice = staging_buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |res| {
            let _ = tx.send(res);
        });

        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());

        rx.recv()
            .map_err(|e| GpuError::BufferAsyncError(format!("Channel recv error: {:?}", e)))?
            .map_err(|e| GpuError::BufferAsyncError(format!("Buffer map error: {:?}", e)))?;

        let mapped = slice.get_mapped_range();
        let mut pixels = Vec::with_capacity((width * height) as usize);

        for y in 0..height {
            let row_start = (y * padded_bytes_per_row) as usize;
            let row_end = row_start + (unpadded_bytes_per_row as usize);
            let row_bytes = &mapped[row_start..row_end];
            let row_pixels: &[[half::f16; 4]] = bytemuck::cast_slice(row_bytes);
            pixels.extend_from_slice(row_pixels);
        }

        drop(mapped);
        staging_buffer.unmap();

        Ok(LinearImage::new(width, height, pixels, metadata)?)
    }

    /// Full-resolution export pipeline:
    /// Applies all active compute ops on the full-resolution `LinearImage`,
    /// then if `!stack.crop.is_identity()`, runs `crop_transform` compute pass,
    /// and reads back `LinearImage`.
    pub fn export_full_res(
        &mut self,
        source: &LinearImage,
        stack: &EditStack,
        luts: &[Lut3D],
    ) -> Result<LinearImage, GpuError> {
        // 1. Upload any provided LUTs
        for lut in luts {
            self.engine.upload_lut(&self.device, &self.queue, lut)?;
        }

        // 2. Upload source image
        let image_id = 1u64;
        self.engine.upload_source_image(&self.device, &self.queue, image_id, source);

        // 3. Process compute stack
        let _ = self.engine.process_stack(&self.device, &self.queue, stack, false)?;

        // 4. Check crop/geometry
        if stack.crop.is_identity() {
            let processed_tex = self.engine.get_current_output_texture()?;
            self.readback_texture(processed_tex, source.width, source.height, source.metadata.clone())
        } else {
            let turns = stack.crop.quarter_turns_cw % 4;
            let (base_w, base_h) = if turns == 1 || turns == 3 {
                (source.height, source.width)
            } else {
                (source.width, source.height)
            };

            let out_w = ((base_w as f32 * stack.crop.rect.width).round() as u32).max(1);
            let out_h = ((base_h as f32 * stack.crop.rect.height).round() as u32).max(1);

            let crop_dst_tex = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("gpu_export_crop_dst_tex"),
                size: wgpu::Extent3d {
                    width: out_w,
                    height: out_h,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba16Float,
                usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let crop_dst_view = crop_dst_tex.create_view(&wgpu::TextureViewDescriptor::default());

            let processed_view = self.engine.get_current_output_view()?;
            self.engine.execute_crop_transform(
                &self.device,
                &self.queue,
                processed_view,
                source.width,
                source.height,
                &crop_dst_view,
                out_w,
                out_h,
                &stack.crop,
            );

            let mut out_metadata = source.metadata.clone();
            out_metadata.original_width = out_w;
            out_metadata.original_height = out_h;

            self.readback_texture(&crop_dst_tex, out_w, out_h, out_metadata)
        }
    }
}
