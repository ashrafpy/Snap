use std::sync::{Arc, Mutex};
use iced::mouse;
use iced::widget::shader::{self, Action, Viewport};
use iced::{Event, Point, Rectangle};
use snap_core::image_types::{HistogramData, LinearImage, Lut3D};
use snap_core::ops::{CropRect, EditOp, ToolId};
use snap_core::stack::EditStack;
use snap_gpu::engine::GpuPipelineEngine;
use snap_gpu::types::ViewportParams;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CropHandle {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    TopEdge,
    BottomEdge,
    LeftEdge,
    RightEdge,
    Interior,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GestureDragMode {
    None,
    Pan {
        start_pan: [f32; 2],
    },
    VerticalPicker {
        start_idx: usize,
        accumulated_dy: f32,
    },
    HorizontalAdjust {
        param_idx: usize,
        start_val: f32,
    },
    CropDrag {
        handle: CropHandle,
        start_rect: CropRect,
        start_norm: [f32; 2],
    },
}

#[derive(Default)]
pub struct CanvasShaderState {
    pub drag_start_pos: Option<Point>,
    pub last_pos: Point,
    pub drag_mode: Option<GestureDragMode>,
    pub is_shift_down: bool,
    pub is_alt_down: bool,
}

pub struct SnapShaderPipeline {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub format: wgpu::TextureFormat,
    pub engine: Mutex<GpuPipelineEngine>,
    pub physical_bounds: Mutex<[u32; 4]>,
    pub last_hist_gen: Mutex<u64>,
}

impl shader::Pipeline for SnapShaderPipeline {
    fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let engine = GpuPipelineEngine::new(device, queue, Some(format));
        Self {
            device: device.clone(),
            queue: queue.clone(),
            format,
            engine: Mutex::new(engine),
            physical_bounds: Mutex::new([0, 0, 0, 0]),
            last_hist_gen: Mutex::new(u64::MAX),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SnapShaderPrimitive {
    pub image_id: Option<u64>,
    pub proxy_image: Option<Arc<LinearImage>>,
    pub effective_stack: EditStack,
    pub bypass_edits: bool,
    pub viewport_params: ViewportParams,
    pub needed_luts: Vec<Lut3D>,
    pub hist_slot: Option<Arc<Mutex<Option<HistogramData>>>>,
    pub hist_dirty: bool,
    pub is_dragging: bool,
}

impl shader::Primitive for SnapShaderPrimitive {
    type Pipeline = SnapShaderPipeline;

    fn prepare(
        &self,
        pipeline: &mut Self::Pipeline,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bounds: &Rectangle,
        viewport: &Viewport,
    ) {
        let mut engine = pipeline.engine.lock().unwrap();

        // 1. Upload source proxy image if present and ID changed
        if let (Some(id), Some(img)) = (self.image_id, &self.proxy_image) {
            engine.upload_source_image(device, queue, id, img);
        }

        // 2. Upload any needed LUTs
        for lut in &self.needed_luts {
            let _ = engine.upload_lut(device, queue, lut);
        }

        // 3. Process edit stack compute passes
        let _ = engine.process_stack(device, queue, &self.effective_stack, self.bypass_edits);

        // 4. Update live GPU histogram only when generation changed, not dragging, and slot is empty
        let current_gen = engine.dirty_generation();
        let mut last_gen = pipeline.last_hist_gen.lock().unwrap();
        if current_gen != *last_gen && !self.is_dragging {
            if let Some(slot) = &self.hist_slot {
                let slot_empty = slot.lock().map(|lock| lock.is_none()).unwrap_or(false);
                if slot_empty {
                    if let Ok(hist) = engine.compute_histogram_sync(device, queue) {
                        if let Ok(mut lock) = slot.lock() {
                            *lock = Some(hist);
                        }
                        *last_gen = current_gen;
                    }
                }
            }
        }

        // 5. Compute physical bounds clamped accurately to viewport
        let scale = viewport.scale_factor();
        let phys_x = (bounds.x * scale).max(0.0) as u32;
        let phys_y = (bounds.y * scale).max(0.0) as u32;
        let vp_w = viewport.physical_size().width;
        let vp_h = viewport.physical_size().height;
        let phys_w = ((bounds.width * scale).max(0.0) as u32).min(vp_w.saturating_sub(phys_x));
        let phys_h = ((bounds.height * scale).max(0.0) as u32).min(vp_h.saturating_sub(phys_y));

        *pipeline.physical_bounds.lock().unwrap() = [phys_x, phys_y, phys_w, phys_h];
    }

    fn draw(
        &self,
        _pipeline: &Self::Pipeline,
        _render_pass: &mut wgpu::RenderPass<'_>,
    ) -> bool {
        // Return false so iced calls render(...) with command encoder!
        false
    }

    fn render(
        &self,
        pipeline: &Self::Pipeline,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        clip_bounds: &Rectangle<u32>,
    ) {
        let [px, py, pw, ph] = *pipeline.physical_bounds.lock().unwrap();
        let x0 = px.max(clip_bounds.x);
        let y0 = py.max(clip_bounds.y);
        let x1 = px.saturating_add(pw).min(clip_bounds.x.saturating_add(clip_bounds.width));
        let y1 = py.saturating_add(ph).min(clip_bounds.y.saturating_add(clip_bounds.height));
        if x1 <= x0 || y1 <= y0 {
            return;
        }
        let safe_bounds = [x0, y0, x1 - x0, y1 - y0];

        pipeline.engine.lock().unwrap().render_viewport(
            &pipeline.device,
            &pipeline.queue,
            encoder,
            target,
            pipeline.format,
            safe_bounds,
            &self.viewport_params,
        );
    }
}

pub struct CanvasShaderProgram<'a> {
    pub image_id: Option<u64>,
    pub proxy_image: Option<Arc<LinearImage>>,
    pub effective_stack: EditStack,
    pub bypass_edits: bool,
    pub viewport_params: ViewportParams,
    pub needed_luts: Vec<Lut3D>,
    pub hist_slot: Option<Arc<Mutex<Option<HistogramData>>>>,
    pub hist_dirty: bool,
    pub active_tool: Option<ToolId>,
    pub active_op: Option<EditOp>,
    pub selected_param_idx: usize,
    pub eyedropper_mode: bool,
    pub vignette_center_mode: bool,
    pub _marker: std::marker::PhantomData<&'a ()>,
}

impl<'a> CanvasShaderProgram<'a> {
    /// Convert widget screen coordinates to normalized image UV coordinates [0.0..=1.0].
    pub fn screen_to_image_uv(&self, pt: Point, bounds: Rectangle) -> Option<[f32; 2]> {
        let (img_w, img_h) = if let Some(ref img) = self.proxy_image {
            (img.width as f32, img.height as f32)
        } else {
            return None;
        };

        let turns = self.viewport_params.crop.quarter_turns_cw % 4;
        let (base_w, base_h) = if turns == 1 || turns == 3 {
            (img_h, img_w)
        } else {
            (img_w, img_h)
        };

        let (disp_w, disp_h) = if self.viewport_params.crop_preview_mode {
            (base_w, base_h)
        } else {
            (
                base_w * self.viewport_params.crop.rect.width,
                base_h * self.viewport_params.crop.rect.height,
            )
        };

        let img_aspect = disp_w / disp_h.max(1.0);
        let vp_aspect = bounds.width / bounds.height.max(1.0);

        let (fit_w, fit_h) = if img_aspect > vp_aspect {
            (bounds.width, bounds.width / img_aspect)
        } else {
            (bounds.height * img_aspect, bounds.height)
        };

        let quad_w = fit_w * self.viewport_params.zoom;
        let quad_h = fit_h * self.viewport_params.zoom;
        let center_x = bounds.width * 0.5 + self.viewport_params.pan[0];
        let center_y = bounds.height * 0.5 + self.viewport_params.pan[1];

        let quad_u = (pt.x - (center_x - quad_w * 0.5)) / quad_w;
        let quad_v = (pt.y - (center_y - quad_h * 0.5)) / quad_h;

        if (0.0..=1.0).contains(&quad_u) && (0.0..=1.0).contains(&quad_v) {
            if self.viewport_params.crop_preview_mode {
                Some([quad_u, quad_v])
            } else {
                let rect = self.viewport_params.crop.rect;
                Some([rect.x + quad_u * rect.width, rect.y + quad_v * rect.height])
            }
        } else {
            None
        }
    }
}

impl<'a> shader::Program<crate::app::Message> for CanvasShaderProgram<'a> {
    type State = CanvasShaderState;
    type Primitive = SnapShaderPrimitive;

    fn update(
        &self,
        state: &mut Self::State,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<Action<crate::app::Message>> {
        let Some(cursor_pos) = cursor.position_in(bounds) else {
            if state.drag_mode.is_some() {
                state.drag_mode = None;
                state.drag_start_pos = None;
                return Some(Action::publish(crate::app::Message::CanvasGestureEnd));
            }
            return None;
        };

        state.last_pos = cursor_pos;

        match event {
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let dy = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y,
                    mouse::ScrollDelta::Pixels { y, .. } => (*y / 30.0).clamp(-3.0, 3.0),
                };
                if dy.abs() > 1e-4 {
                    return Some(Action::publish(crate::app::Message::CanvasZoomWheel {
                        delta_y: dy,
                        cursor_pos,
                        bounds_size: bounds.size(),
                    }));
                }
            }

            Event::Mouse(mouse::Event::ButtonPressed(button)) => {
                state.drag_start_pos = Some(cursor_pos);

                if *button == mouse::Button::Middle || state.is_alt_down {
                    state.drag_mode = Some(GestureDragMode::Pan {
                        start_pan: self.viewport_params.pan,
                    });
                    return Some(Action::request_redraw());
                }

                if *button == mouse::Button::Left {
                    // 1. WB Eyedropper click mode
                    if self.eyedropper_mode {
                        if let Some([u, v]) = self.screen_to_image_uv(cursor_pos, bounds) {
                            return Some(Action::publish(
                                crate::app::Message::CanvasClickEyedropper { norm_x: u, norm_y: v },
                            ));
                        }
                    }

                    // 2. Vignette Center click mode
                    if self.vignette_center_mode {
                        if let Some([u, v]) = self.screen_to_image_uv(cursor_pos, bounds) {
                            return Some(Action::publish(
                                crate::app::Message::CanvasClickVignetteCenter {
                                    norm_x: u,
                                    norm_y: v,
                                },
                            ));
                        }
                    }

                    // 3. Crop tool active: check handles
                    if self.active_tool == Some(ToolId::CropRotate) {
                        if let Some([u, v]) = self.screen_to_image_uv(cursor_pos, bounds) {
                            let rect = self.viewport_params.crop.rect;
                            let margin = 0.05; // 5% handle threshold

                            let near_left = (u - rect.x).abs() < margin;
                            let near_right = (u - (rect.x + rect.width)).abs() < margin;
                            let near_top = (v - rect.y).abs() < margin;
                            let near_bottom = (v - (rect.y + rect.height)).abs() < margin;

                            let handle = if near_left && near_top {
                                Some(CropHandle::TopLeft)
                            } else if near_right && near_top {
                                Some(CropHandle::TopRight)
                            } else if near_left && near_bottom {
                                Some(CropHandle::BottomLeft)
                            } else if near_right && near_bottom {
                                Some(CropHandle::BottomRight)
                            } else if near_left {
                                Some(CropHandle::LeftEdge)
                            } else if near_right {
                                Some(CropHandle::RightEdge)
                            } else if near_top {
                                Some(CropHandle::TopEdge)
                            } else if near_bottom {
                                Some(CropHandle::BottomEdge)
                            } else if u > rect.x
                                && u < rect.x + rect.width
                                && v > rect.y
                                && v < rect.y + rect.height
                            {
                                Some(CropHandle::Interior)
                            } else {
                                None
                            };

                            if let Some(h) = handle {
                                state.drag_mode = Some(GestureDragMode::CropDrag {
                                    handle: h,
                                    start_rect: rect,
                                    start_norm: [u, v],
                                });
                                return Some(Action::request_redraw());
                            }
                        }
                    }

                    // 4. Default: Tool gesture or canvas pan
                    if self.active_op.is_some() {
                        state.drag_mode = Some(GestureDragMode::None);
                    } else {
                        // When no tool is active, plain left drag pans!
                        state.drag_mode = Some(GestureDragMode::Pan {
                            start_pan: self.viewport_params.pan,
                        });
                    }
                    return Some(Action::request_redraw());
                }
            }

            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let Some(start) = state.drag_start_pos else {
                    return None;
                };

                let dx = cursor_pos.x - start.x;
                let dy = cursor_pos.y - start.y;

                match state.drag_mode {
                    Some(GestureDragMode::Pan { start_pan }) => {
                        let new_pan = [start_pan[0] + dx, start_pan[1] + dy];
                        return Some(Action::publish(crate::app::Message::SetPan(new_pan)));
                    }

                    Some(GestureDragMode::None) => {
                        // Check initial gesture direction threshold (6px)
                        let dist_sq = dx * dx + dy * dy;
                        if dist_sq >= 36.0 {
                            if dy.abs() > dx.abs() {
                                // Vertical Parameter Picker
                                state.drag_mode = Some(GestureDragMode::VerticalPicker {
                                    start_idx: self.selected_param_idx,
                                    accumulated_dy: 0.0,
                                });
                                return Some(Action::publish(
                                    crate::app::Message::SetVerticalPickerActive(true),
                                ));
                            } else {
                                // Horizontal Slider Adjustment
                                let cur_val = if let Some(ref op) = self.active_op {
                                    let sliders = op.slider_params();
                                    if self.selected_param_idx < sliders.len() {
                                        sliders[self.selected_param_idx].1
                                    } else {
                                        0.0
                                    }
                                } else {
                                    0.0
                                };
                                state.drag_mode = Some(GestureDragMode::HorizontalAdjust {
                                    param_idx: self.selected_param_idx,
                                    start_val: cur_val,
                                });
                                return Some(Action::publish(
                                    crate::app::Message::SetHorizontalAdjustActive(true),
                                ));
                            }
                        }
                    }

                    Some(GestureDragMode::VerticalPicker {
                        start_idx,
                        ref mut accumulated_dy,
                    }) => {
                        *accumulated_dy = dy;
                        if let Some(ref op) = self.active_op {
                            let count = op.slider_params().len();
                            if count > 0 {
                                // Step every 35px
                                let step = (dy / 35.0).round() as i32;
                                let new_idx = (start_idx as i32 + step).clamp(0, count as i32 - 1)
                                    as usize;
                                if new_idx != self.selected_param_idx {
                                    return Some(Action::publish(
                                        crate::app::Message::SelectSliderParam(new_idx),
                                    ));
                                }
                            }
                        }
                    }

                    Some(GestureDragMode::HorizontalAdjust {
                        param_idx,
                        start_val,
                    }) => {
                        if let Some(ref op) = self.active_op {
                            let sliders = op.slider_params();
                            if param_idx < sliders.len() {
                                let desc = &sliders[param_idx].0;
                                let range = desc.max - desc.min;
                                let scale = if state.is_shift_down { 0.2 } else { 1.0 };
                                let delta = (dx / 300.0) * range * scale;
                                let new_val = (start_val + delta).clamp(desc.min, desc.max);
                                return Some(Action::publish(
                                    crate::app::Message::SetSliderParamLive(param_idx, new_val),
                                ));
                            }
                        }
                    }

                    Some(GestureDragMode::CropDrag {
                        handle,
                        start_rect,
                        start_norm,
                    }) => {
                        if let Some([cur_u, cur_v]) = self.screen_to_image_uv(cursor_pos, bounds) {
                            let du = cur_u - start_norm[0];
                            let dv = cur_v - start_norm[1];
                            let mut rect = start_rect;

                            match handle {
                                CropHandle::TopLeft => {
                                    let max_x = (start_rect.x + start_rect.width - 0.05).max(0.0);
                                    rect.x = (start_rect.x + du).clamp(0.0, max_x);
                                    let max_y = (start_rect.y + start_rect.height - 0.05).max(0.0);
                                    rect.y = (start_rect.y + dv).clamp(0.0, max_y);
                                    rect.width = start_rect.width - (rect.x - start_rect.x);
                                    rect.height = start_rect.height - (rect.y - start_rect.y);
                                }
                                CropHandle::TopRight => {
                                    let max_y = (start_rect.y + start_rect.height - 0.05).max(0.0);
                                    rect.y = (start_rect.y + dv).clamp(0.0, max_y);
                                    let max_w = (1.0 - rect.x).max(0.01);
                                    let min_w = 0.05_f32.min(max_w);
                                    rect.width = (start_rect.width + du).clamp(min_w, max_w);
                                    rect.height = start_rect.height - (rect.y - start_rect.y);
                                }
                                CropHandle::BottomLeft => {
                                    let max_x = (start_rect.x + start_rect.width - 0.05).max(0.0);
                                    rect.x = (start_rect.x + du).clamp(0.0, max_x);
                                    rect.width = start_rect.width - (rect.x - start_rect.x);
                                    let max_h = (1.0 - rect.y).max(0.01);
                                    let min_h = 0.05_f32.min(max_h);
                                    rect.height = (start_rect.height + dv).clamp(min_h, max_h);
                                }
                                CropHandle::BottomRight => {
                                    let max_w = (1.0 - rect.x).max(0.01);
                                    let min_w = 0.05_f32.min(max_w);
                                    rect.width = (start_rect.width + du).clamp(min_w, max_w);
                                    let max_h = (1.0 - rect.y).max(0.01);
                                    let min_h = 0.05_f32.min(max_h);
                                    rect.height = (start_rect.height + dv).clamp(min_h, max_h);
                                }
                                CropHandle::TopEdge => {
                                    let max_y = (start_rect.y + start_rect.height - 0.05).max(0.0);
                                    rect.y = (start_rect.y + dv).clamp(0.0, max_y);
                                    rect.height = start_rect.height - (rect.y - start_rect.y);
                                }
                                CropHandle::BottomEdge => {
                                    let max_h = (1.0 - rect.y).max(0.01);
                                    let min_h = 0.05_f32.min(max_h);
                                    rect.height = (start_rect.height + dv).clamp(min_h, max_h);
                                }
                                CropHandle::LeftEdge => {
                                    let max_x = (start_rect.x + start_rect.width - 0.05).max(0.0);
                                    rect.x = (start_rect.x + du).clamp(0.0, max_x);
                                    rect.width = start_rect.width - (rect.x - start_rect.x);
                                }
                                CropHandle::RightEdge => {
                                    let max_w = (1.0 - rect.x).max(0.01);
                                    let min_w = 0.05_f32.min(max_w);
                                    rect.width = (start_rect.width + du).clamp(min_w, max_w);
                                }
                                CropHandle::Interior => {
                                    let max_x = (1.0 - rect.width).max(0.0);
                                    rect.x = (start_rect.x + du).clamp(0.0, max_x);
                                    let max_y = (1.0 - rect.height).max(0.0);
                                    rect.y = (start_rect.y + dv).clamp(0.0, max_y);
                                }
                            }

                            return Some(Action::publish(crate::app::Message::SetCropRect(rect)));
                        }
                    }

                    _ => {}
                }
            }

            Event::Keyboard(iced::keyboard::Event::ModifiersChanged(mods)) => {
                state.is_shift_down = mods.shift();
                state.is_alt_down = mods.alt();
            }

            Event::Mouse(mouse::Event::ButtonReleased(_)) => {
                let was_dragging = state.drag_mode.is_some();
                state.drag_mode = None;
                state.drag_start_pos = None;

                if was_dragging {
                    return Some(Action::publish(crate::app::Message::CanvasGestureEnd));
                }
            }

            _ => {}
        }

        None
    }

    fn draw(
        &self,
        state: &Self::State,
        _cursor: mouse::Cursor,
        _bounds: Rectangle,
    ) -> Self::Primitive {
        SnapShaderPrimitive {
            image_id: self.image_id,
            proxy_image: self.proxy_image.clone(),
            effective_stack: self.effective_stack.clone(),
            bypass_edits: self.bypass_edits,
            viewport_params: self.viewport_params.clone(),
            needed_luts: self.needed_luts.clone(),
            hist_slot: self.hist_slot.clone(),
            hist_dirty: self.hist_dirty,
            is_dragging: state.drag_mode.is_some(),
        }
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if matches!(state.drag_mode, Some(GestureDragMode::Pan { .. })) {
            return mouse::Interaction::Grabbing;
        }
        if self.eyedropper_mode || self.vignette_center_mode {
            return mouse::Interaction::Crosshair;
        }
        if self.active_tool == Some(ToolId::CropRotate) {
            if let Some(pos) = cursor.position_in(bounds) {
                if let Some([u, v]) = self.screen_to_image_uv(pos, bounds) {
                    let rect = self.viewport_params.crop.rect;
                    let m = 0.05;
                    let near_l = (u - rect.x).abs() < m;
                    let near_r = (u - (rect.x + rect.width)).abs() < m;
                    let near_t = (v - rect.y).abs() < m;
                    let near_b = (v - (rect.y + rect.height)).abs() < m;

                    if (near_l && near_t) || (near_r && near_b) {
                        return mouse::Interaction::ResizingDiagonallyUp;
                    }
                    if (near_r && near_t) || (near_l && near_b) {
                        return mouse::Interaction::ResizingDiagonallyDown;
                    }
                    if near_l || near_r {
                        return mouse::Interaction::ResizingHorizontally;
                    }
                    if near_t || near_b {
                        return mouse::Interaction::ResizingVertically;
                    }
                    if u > rect.x && u < rect.x + rect.width && v > rect.y && v < rect.y + rect.height {
                        return mouse::Interaction::Move;
                    }
                }
            }
        }
        mouse::Interaction::default()
    }
}
