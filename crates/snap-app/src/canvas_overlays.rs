//! Interactive viewport canvas overlays for Snap.
//!
//! Provides interactive vector overlays layered on top of `CanvasShaderProgram`, including:
//! - Translucent ruby-red LayerMask preview heatmap.
//! - Live dual-ring feathered brush cursor for Brush, Healing, and Mask painting.
//! - Draggable Nik U-Point pins and selectivity radius handles (`ToolId::Selective`).
//! - Interactive focus reticle, tilt-shift axis, and radius/transition handles (`ToolId::LensBlur`).
//! - Draggable motion center and angle arrow handle (`ToolId::MotionBlur`).
//! - Draggable face center and radius handle (`ToolId::Portrait`).
//! - Draggable face center and yaw/pitch vector handle (`ToolId::HeadPose`).
//! - Draggable center and scale handle (`ToolId::TextOverlay`).
//!
//! Strictly observes pass-through event discipline so wheel zooming, middle/Alt panning,
//! and background swipe gestures remain fluid and unobstructed.

use iced::mouse;
use iced::widget::canvas::{self, Action, Frame, Geometry, Path, Stroke};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Size, Theme};

use snap_core::mask::LayerMask;
use snap_core::ops::{EditOp, ToolId};
use snap_core::ops_ext::{
    LensBlurShape, SelectivePin, MAX_SELECTIVE_PINS,
};
use snap_gpu::types::ViewportParams;

use crate::theme::palette;

// ----------------------------------------------------------------------------
// Public Events
// ----------------------------------------------------------------------------

/// Interactive events emitted by canvas overlays to update live tool state.
#[derive(Debug, Clone)]
pub enum CanvasOverlayEvent {
    /// Live update of active tool op while dragging a handle/pin.
    PreviewOp(EditOp),
    /// Commit active tool op to ToolSessionHistory on drag/stroke completion or pin placement.
    CommitOp(EditOp),
    /// Paint a Brush tool stroke segment in normalized image UV [0..1].
    BrushStrokeSegment { from: [f32; 2], to: [f32; 2] },
    BrushStrokeEnd,
    /// Healing stroke lifecycle in normalized image UV [0..1].
    HealStrokeStart { uv: [f32; 2] },
    HealStrokeMove { uv: [f32; 2] },
    HealStrokeEnd,
    /// Per-node LayerMask brush stroke in View Edits mode.
    MaskStrokeSegment {
        node_id: uuid::Uuid,
        from: [f32; 2],
        to: [f32; 2],
    },
    MaskStrokeEnd {
        node_id: uuid::Uuid,
    },
}

// ----------------------------------------------------------------------------
// Overlay Context
// ----------------------------------------------------------------------------

/// Read-only snapshot of current app and viewport state passed into overlay rendering.
#[derive(Clone, Debug)]
pub struct CanvasOverlayContext<'a> {
    pub image_dims: Option<(u32, u32)>,
    pub viewport_params: ViewportParams,
    pub active_tool: Option<ToolId>,
    pub active_op: Option<&'a EditOp>,
    pub selective_add_pin_mode: bool,
    pub mask_edit_node: Option<(uuid::Uuid, Option<&'a LayerMask>)>,
    pub mask_paint_active: bool,
    pub mask_brush_radius: f32,
    pub show_red_mask_overlay: bool,
    pub bypass_edits: bool,
}

impl<'a> CanvasOverlayContext<'a> {
    /// Compute the exact image display quad rectangle in widget-local coordinates,
    /// replicating `CanvasShaderProgram::screen_to_image_uv` coordinate math.
    pub fn image_quad_rect(&self, bounds: Rectangle) -> Option<Rectangle> {
        let (img_w, img_h) = if let Some((w, h)) = self.image_dims {
            (w as f32, h as f32)
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

        Some(Rectangle {
            x: center_x - quad_w * 0.5,
            y: center_y - quad_h * 0.5,
            width: quad_w,
            height: quad_h,
        })
    }

    /// Convert widget screen coordinates to normalized image UV coordinates [0.0..=1.0].
    /// Returns `None` if the point falls outside the rendered image quad.
    pub fn screen_to_image_uv(&self, pt: Point, bounds: Rectangle) -> Option<[f32; 2]> {
        let quad = self.image_quad_rect(bounds)?;
        if quad.width <= 0.0 || quad.height <= 0.0 {
            return None;
        }

        let quad_u = (pt.x - quad.x) / quad.width;
        let quad_v = (pt.y - quad.y) / quad.height;

        if (0.0..=1.0).contains(&quad_u) && (0.0..=1.0).contains(&quad_v) {
            if self.viewport_params.crop_preview_mode {
                Some([quad_u, quad_v])
            } else {
                let rect = self.viewport_params.crop.rect;
                Some([
                    rect.x + quad_u * rect.width,
                    rect.y + quad_v * rect.height,
                ])
            }
        } else {
            None
        }
    }

    /// Convert widget screen coordinates to normalized image UV coordinates [0.0..=1.0],
    /// clamping quad coordinates during active drags so dragging near borders stays smooth.
    pub fn screen_to_image_uv_clamped(&self, pt: Point, bounds: Rectangle) -> Option<[f32; 2]> {
        let quad = self.image_quad_rect(bounds)?;
        if quad.width <= 0.0 || quad.height <= 0.0 {
            return None;
        }

        let quad_u = ((pt.x - quad.x) / quad.width).clamp(0.0, 1.0);
        let quad_v = ((pt.y - quad.y) / quad.height).clamp(0.0, 1.0);

        if self.viewport_params.crop_preview_mode {
            Some([quad_u, quad_v])
        } else {
            let rect = self.viewport_params.crop.rect;
            let u = (rect.x + quad_u * rect.width).clamp(0.0, 1.0);
            let v = (rect.y + quad_v * rect.height).clamp(0.0, 1.0);
            Some([u, v])
        }
    }

    /// Convert normalized image UV coordinates [0.0..=1.0] to widget screen coordinates.
    pub fn image_uv_to_screen(&self, uv: [f32; 2], bounds: Rectangle) -> Option<Point> {
        let quad = self.image_quad_rect(bounds)?;
        if quad.width <= 0.0 || quad.height <= 0.0 {
            return None;
        }

        let (quad_u, quad_v) = if self.viewport_params.crop_preview_mode {
            (uv[0], uv[1])
        } else {
            let rect = self.viewport_params.crop.rect;
            let rw = rect.width.max(1e-6);
            let rh = rect.height.max(1e-6);
            ((uv[0] - rect.x) / rw, (uv[1] - rect.y) / rh)
        };

        let screen_x = quad.x + quad_u * quad.width;
        let screen_y = quad.y + quad_v * quad.height;

        Some(Point::new(screen_x, screen_y))
    }
}

// ----------------------------------------------------------------------------
// Overlay View Constructor
// ----------------------------------------------------------------------------

/// Build an iced `Element` hosting the interactive viewport canvas overlays.
pub fn view_canvas_overlay<'a>(
    ctx: CanvasOverlayContext<'a>,
) -> Element<'a, CanvasOverlayEvent> {
    iced::widget::canvas(CanvasOverlayWidget::new(ctx))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

// ----------------------------------------------------------------------------
// Drag Target Enum & State
// ----------------------------------------------------------------------------

/// Identifier for currently active drag handle or continuous paint action.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DragTarget {
    // Painting strokes
    MaskStroke(uuid::Uuid),
    BrushStroke,
    HealStroke,
    // Selective Pins
    SelectivePin(usize),
    SelectiveRadius(usize),
    // LensBlur
    LensBlurCenter,
    LensBlurInnerRadius,
    LensBlurTransition,
    // MotionBlur
    MotionBlurCenter,
    MotionBlurAngle,
    // Portrait
    PortraitCenter,
    PortraitRadius,
    // HeadPose
    HeadPoseCenter,
    HeadPoseDirection,
    // TextOverlay
    TextOverlayCenter,
    TextOverlayScale,
}

/// Internal interactive state preserved across canvas frames.
#[derive(Default)]
pub struct CanvasOverlayState {
    pub drag_target: Option<DragTarget>,
    pub drag_start_cursor: Option<Point>,
    pub last_uv: [f32; 2],
    pub cursor_pos: Option<Point>,
    pub is_alt_down: bool,
    pub is_shift_down: bool,
    pub working_op: Option<EditOp>,
}

// ----------------------------------------------------------------------------
// Widget Implementation
// ----------------------------------------------------------------------------

pub struct CanvasOverlayWidget<'a> {
    pub ctx: CanvasOverlayContext<'a>,
}

impl<'a> CanvasOverlayWidget<'a> {
    pub fn new(ctx: CanvasOverlayContext<'a>) -> Self {
        Self { ctx }
    }

    pub fn image_quad_rect(&self, bounds: Rectangle) -> Option<Rectangle> {
        self.ctx.image_quad_rect(bounds)
    }

    pub fn screen_to_image_uv(&self, pt: Point, bounds: Rectangle) -> Option<[f32; 2]> {
        self.ctx.screen_to_image_uv(pt, bounds)
    }

    pub fn screen_to_image_uv_clamped(&self, pt: Point, bounds: Rectangle) -> Option<[f32; 2]> {
        self.ctx.screen_to_image_uv_clamped(pt, bounds)
    }

    pub fn image_uv_to_screen(&self, uv: [f32; 2], bounds: Rectangle) -> Option<Point> {
        self.ctx.image_uv_to_screen(uv, bounds)
    }

    /// Hit-test interactive handles of the currently active tool.
    fn hit_test(
        &self,
        pos: Point,
        bounds: Rectangle,
        state: &CanvasOverlayState,
    ) -> Option<DragTarget> {
        let op = state.working_op.as_ref().or(self.ctx.active_op)?;
        let quad = self.image_quad_rect(bounds)?;
        let scale = quad.width.min(quad.height);

        match op {
            EditOp::Selective(sel) => {
                // Priority: active pin radius handle
                if sel.active_pin < sel.pins.len() {
                    let pin = &sel.pins[sel.active_pin];
                    if let Some(pin_screen) = self.image_uv_to_screen([pin.x, pin.y], bounds) {
                        let r_px = (pin.radius * scale).max(8.0);
                        let handle_pt = Point::new(pin_screen.x + r_px, pin_screen.y);
                        if dist_sq(pos, handle_pt) <= 144.0 {
                            return Some(DragTarget::SelectiveRadius(sel.active_pin));
                        }
                    }
                }
                // Then check pin center badges
                for (idx, pin) in sel.pins.iter().enumerate() {
                    if let Some(pin_screen) = self.image_uv_to_screen([pin.x, pin.y], bounds) {
                        if dist_sq(pos, pin_screen) <= 225.0 {
                            return Some(DragTarget::SelectivePin(idx));
                        }
                    }
                }
                None
            }

            EditOp::LensBlur(lb) => {
                let center_pt = self.image_uv_to_screen(lb.center, bounds)?;
                let r_in_px = (lb.inner_radius * scale).max(6.0);
                let r_out_px = ((lb.inner_radius + lb.transition) * scale).max(r_in_px + 6.0);

                let (h_in, h_out) = match lb.shape {
                    LensBlurShape::Elliptical => (
                        Point::new(center_pt.x + r_in_px, center_pt.y),
                        Point::new(center_pt.x + r_out_px, center_pt.y),
                    ),
                    LensBlurShape::Linear => {
                        let rad = lb.angle_deg.to_radians();
                        let norm = (-rad.sin(), rad.cos());
                        (
                            Point::new(center_pt.x + norm.0 * r_in_px, center_pt.y + norm.1 * r_in_px),
                            Point::new(center_pt.x + norm.0 * r_out_px, center_pt.y + norm.1 * r_out_px),
                        )
                    }
                };

                if dist_sq(pos, center_pt) <= 196.0 {
                    Some(DragTarget::LensBlurCenter)
                } else if dist_sq(pos, h_in) <= 144.0 {
                    Some(DragTarget::LensBlurInnerRadius)
                } else if dist_sq(pos, h_out) <= 144.0 {
                    Some(DragTarget::LensBlurTransition)
                } else {
                    None
                }
            }

            EditOp::MotionBlur(mb) => {
                let center_pt = self.image_uv_to_screen(mb.center, bounds)?;
                let arrow_len = 65.0f32;
                let rad = mb.angle_deg.to_radians();
                let tip = Point::new(
                    center_pt.x + rad.cos() * arrow_len,
                    center_pt.y + rad.sin() * arrow_len,
                );

                if dist_sq(pos, center_pt) <= 196.0 {
                    Some(DragTarget::MotionBlurCenter)
                } else if dist_sq(pos, tip) <= 144.0 {
                    Some(DragTarget::MotionBlurAngle)
                } else {
                    None
                }
            }

            EditOp::Portrait(p) => {
                let center_pt = self.image_uv_to_screen(p.face_center, bounds)?;
                let r_px = (p.face_radius * scale).max(10.0);
                let h_pt = Point::new(center_pt.x + r_px, center_pt.y);

                if dist_sq(pos, center_pt) <= 196.0 {
                    Some(DragTarget::PortraitCenter)
                } else if dist_sq(pos, h_pt) <= 144.0 {
                    Some(DragTarget::PortraitRadius)
                } else {
                    None
                }
            }

            EditOp::HeadPose(hp) => {
                let center_pt = self.image_uv_to_screen(hp.face_center, bounds)?;
                let tip = Point::new(
                    center_pt.x + (hp.yaw_x / 100.0) * 55.0,
                    center_pt.y + (hp.pitch_y / 100.0) * 55.0,
                );

                if dist_sq(pos, center_pt) <= 196.0 {
                    Some(DragTarget::HeadPoseCenter)
                } else if dist_sq(pos, tip) <= 144.0 {
                    Some(DragTarget::HeadPoseDirection)
                } else {
                    None
                }
            }

            EditOp::TextOverlay(txt) => {
                let center_pt = self.image_uv_to_screen(txt.center, bounds)?;
                let w = 200.0 * txt.scale;
                let h = 70.0 * txt.scale;
                let corner_pt = Point::new(center_pt.x + w * 0.5, center_pt.y + h * 0.5);

                if dist_sq(pos, center_pt) <= 196.0 {
                    Some(DragTarget::TextOverlayCenter)
                } else if dist_sq(pos, corner_pt) <= 144.0 {
                    Some(DragTarget::TextOverlayScale)
                } else {
                    None
                }
            }

            _ => None,
        }
    }

    // ------------------------------------------------------------------------
    // Visual Overlay Drawing Subroutines
    // ------------------------------------------------------------------------

    /// Translucent ruby-red LayerMask preview heatmap.
    fn draw_mask_red_overlay(&self, frame: &mut Frame, bounds: Rectangle) {
        if !self.ctx.show_red_mask_overlay || self.ctx.bypass_edits {
            return;
        }

        let (_, mask_opt) = match self.ctx.mask_edit_node {
            Some((node_id, m)) => (node_id, m),
            None => return,
        };

        let quad = match self.image_quad_rect(bounds) {
            Some(q) => q,
            None => return,
        };

        const GRID: usize = 64;
        let cell_w = quad.width / GRID as f32;
        let cell_h = quad.height / GRID as f32;

        let crop_rect = self.ctx.viewport_params.crop.rect;
        let preview_mode = self.ctx.viewport_params.crop_preview_mode;

        for gy in 0..GRID {
            let v_frac = (gy as f32 + 0.5) / GRID as f32;
            let y0 = (quad.y + gy as f32 * cell_h).round();
            let y1 = (quad.y + (gy + 1) as f32 * cell_h).round();
            if y1 <= y0 {
                continue;
            }

            for gx in 0..GRID {
                let u_frac = (gx as f32 + 0.5) / GRID as f32;
                let x0 = (quad.x + gx as f32 * cell_w).round();
                let x1 = (quad.x + (gx + 1) as f32 * cell_w).round();
                if x1 <= x0 {
                    continue;
                }

                let (img_u, img_v) = if preview_mode {
                    (u_frac, v_frac)
                } else {
                    (
                        crop_rect.x + u_frac * crop_rect.width,
                        crop_rect.y + v_frac * crop_rect.height,
                    )
                };

                let val = if let Some(mask) = mask_opt {
                    mask.sample_bilinear(img_u, img_v)
                } else {
                    1.0
                };

                if val > 0.015 {
                    let color = Color {
                        r: 0.94,
                        g: 0.20,
                        b: 0.25,
                        a: 0.45 * val,
                    };
                    let cell_rect = Path::rectangle(
                        Point::new(x0, y0),
                        Size::new(x1 - x0, y1 - y0),
                    );
                    frame.fill(&cell_rect, color);
                }
            }
        }
    }

    /// Selective Nik U-Point pins and outer selectivity radius handle.
    fn draw_selective_overlays(
        &self,
        frame: &mut Frame,
        bounds: Rectangle,
        params: &snap_core::ops_ext::SelectiveParams,
    ) {
        let quad = match self.image_quad_rect(bounds) {
            Some(q) => q,
            None => return,
        };
        let scale = quad.width.min(quad.height);

        for (idx, pin) in params.pins.iter().enumerate() {
            let pin_screen = match self.image_uv_to_screen([pin.x, pin.y], bounds) {
                Some(pt) => pt,
                None => continue,
            };

            let is_active = idx == params.active_pin;

            if is_active {
                let r_px = (pin.radius * scale).max(8.0);
                let outer_circle = Path::circle(pin_screen, r_px);

                frame.fill(
                    &outer_circle,
                    Color {
                        r: 0.31,
                        g: 0.56,
                        b: 0.97,
                        a: 0.08,
                    },
                );
                frame.stroke(
                    &outer_circle,
                    Stroke::default()
                        .with_color(Color {
                            r: 0.31,
                            g: 0.56,
                            b: 0.97,
                            a: 0.70,
                        })
                        .with_width(1.5),
                );

                // Draggable radius handle on right edge
                let handle_pt = Point::new(pin_screen.x + r_px, pin_screen.y);
                let handle_circle = Path::circle(handle_pt, 6.0);
                frame.fill(&handle_circle, palette::ACCENT);
                frame.stroke(
                    &handle_circle,
                    Stroke::default()
                        .with_color(Color::WHITE)
                        .with_width(1.5),
                );
            }

            // Darkroom pin badge
            let badge_r = 13.0;
            let badge_circle = Path::circle(pin_screen, badge_r);
            let bg_color = if is_active {
                Color::from_rgb8(19, 21, 26)
            } else {
                Color {
                    r: 13.0 / 255.0,
                    g: 15.0 / 255.0,
                    b: 19.0 / 255.0,
                    a: 0.88,
                }
            };
            frame.fill(&badge_circle, bg_color);

            let border_stroke = if is_active {
                Stroke::default()
                    .with_color(palette::ACCENT)
                    .with_width(2.5)
            } else {
                Stroke::default()
                    .with_color(Color {
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
                        a: 0.65,
                    })
                    .with_width(1.5)
            };
            frame.stroke(&badge_circle, border_stroke);

            // Vector reticle tick marks
            let tick_color = if is_active {
                palette::ACCENT
            } else {
                Color {
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    a: 0.50,
                }
            };
            let tick_s = Stroke::default().with_color(tick_color).with_width(1.0);
            frame.stroke(
                &Path::line(
                    Point::new(pin_screen.x - 7.0, pin_screen.y),
                    Point::new(pin_screen.x - 4.0, pin_screen.y),
                ),
                tick_s,
            );
            frame.stroke(
                &Path::line(
                    Point::new(pin_screen.x + 4.0, pin_screen.y),
                    Point::new(pin_screen.x + 7.0, pin_screen.y),
                ),
                tick_s,
            );
            frame.stroke(
                &Path::line(
                    Point::new(pin_screen.x, pin_screen.y - 7.0),
                    Point::new(pin_screen.x, pin_screen.y - 4.0),
                ),
                tick_s,
            );
            frame.stroke(
                &Path::line(
                    Point::new(pin_screen.x, pin_screen.y + 4.0),
                    Point::new(pin_screen.x, pin_screen.y + 7.0),
                ),
                tick_s,
            );

            // Center target dot
            let dot_r = if is_active { 3.0 } else { 2.0 };
            let dot_color = if is_active {
                palette::ACCENT
            } else {
                Color::WHITE
            };
            frame.fill(&Path::circle(pin_screen, dot_r), dot_color);
        }
    }

    /// Lens Blur elliptical or linear tilt-shift handles.
    fn draw_lens_blur_overlays(
        &self,
        frame: &mut Frame,
        bounds: Rectangle,
        params: &snap_core::ops_ext::LensBlurParams,
    ) {
        let quad = match self.image_quad_rect(bounds) {
            Some(q) => q,
            None => return,
        };
        let center_pt = match self.image_uv_to_screen(params.center, bounds) {
            Some(pt) => pt,
            None => return,
        };
        let scale = quad.width.min(quad.height);
        let r_in_px = (params.inner_radius * scale).max(6.0);
        let r_out_px = ((params.inner_radius + params.transition) * scale).max(r_in_px + 6.0);

        match params.shape {
            LensBlurShape::Elliptical => {
                let inner_c = Path::circle(center_pt, r_in_px);
                frame.stroke(
                    &inner_c,
                    Stroke::default()
                        .with_color(Color {
                            r: 1.0,
                            g: 1.0,
                            b: 1.0,
                            a: 0.85,
                        })
                        .with_width(1.5),
                );

                let h_in = Point::new(center_pt.x + r_in_px, center_pt.y);
                let h_in_c = Path::circle(h_in, 5.5);
                frame.fill(&h_in_c, Color::WHITE);
                frame.stroke(
                    &h_in_c,
                    Stroke::default()
                        .with_color(palette::ACCENT)
                        .with_width(1.5),
                );

                let outer_c = Path::circle(center_pt, r_out_px);
                frame.fill(
                    &outer_c,
                    Color {
                        r: 0.31,
                        g: 0.56,
                        b: 0.97,
                        a: 0.04,
                    },
                );
                frame.stroke(
                    &outer_c,
                    Stroke::default()
                        .with_color(Color {
                            r: 1.0,
                            g: 1.0,
                            b: 1.0,
                            a: 0.45,
                        })
                        .with_width(1.0),
                );

                let h_out = Point::new(center_pt.x + r_out_px, center_pt.y);
                let h_out_c = Path::circle(h_out, 5.0);
                frame.fill(&h_out_c, Color::from_rgb8(19, 21, 26));
                frame.stroke(
                    &h_out_c,
                    Stroke::default()
                        .with_color(Color::WHITE)
                        .with_width(1.5),
                );
            }

            LensBlurShape::Linear => {
                let rad = params.angle_deg.to_radians();
                let dir = (rad.cos(), rad.sin());
                let norm = (-rad.sin(), rad.cos());
                let len = quad.width.max(quad.height) * 1.2;

                // Center focal line
                let p0 = Point::new(center_pt.x - dir.0 * len, center_pt.y - dir.1 * len);
                let p1 = Point::new(center_pt.x + dir.0 * len, center_pt.y + dir.1 * len);
                frame.stroke(
                    &Path::line(p0, p1),
                    Stroke::default()
                        .with_color(Color {
                            r: 0.31,
                            g: 0.56,
                            b: 0.97,
                            a: 0.70,
                        })
                        .with_width(1.2),
                );

                // Inner boundaries (+/- norm)
                for sign in [1.0f32, -1.0f32] {
                    let off_x = norm.0 * r_in_px * sign;
                    let off_y = norm.1 * r_in_px * sign;
                    let start = Point::new(
                        center_pt.x + off_x - dir.0 * len,
                        center_pt.y + off_y - dir.1 * len,
                    );
                    let end = Point::new(
                        center_pt.x + off_x + dir.0 * len,
                        center_pt.y + off_y + dir.1 * len,
                    );
                    frame.stroke(
                        &Path::line(start, end),
                        Stroke::default()
                            .with_color(Color {
                                r: 1.0,
                                g: 1.0,
                                b: 1.0,
                                a: 0.85,
                            })
                            .with_width(1.5),
                    );
                }

                // Outer transition boundaries (+/- norm)
                for sign in [1.0f32, -1.0f32] {
                    let off_x = norm.0 * r_out_px * sign;
                    let off_y = norm.1 * r_out_px * sign;
                    let start = Point::new(
                        center_pt.x + off_x - dir.0 * len,
                        center_pt.y + off_y - dir.1 * len,
                    );
                    let end = Point::new(
                        center_pt.x + off_x + dir.0 * len,
                        center_pt.y + off_y + dir.1 * len,
                    );
                    frame.stroke(
                        &Path::line(start, end),
                        Stroke::default()
                            .with_color(Color {
                                r: 1.0,
                                g: 1.0,
                                b: 1.0,
                                a: 0.40,
                            })
                            .with_width(1.0),
                    );
                }

                // Inner angle & radius handle
                let h_in = Point::new(
                    center_pt.x + norm.0 * r_in_px,
                    center_pt.y + norm.1 * r_in_px,
                );
                let h_in_c = Path::circle(h_in, 6.0);
                frame.fill(&h_in_c, Color::WHITE);
                frame.stroke(
                    &h_in_c,
                    Stroke::default()
                        .with_color(palette::ACCENT)
                        .with_width(1.5),
                );

                // Outer transition handle
                let h_out = Point::new(
                    center_pt.x + norm.0 * r_out_px,
                    center_pt.y + norm.1 * r_out_px,
                );
                let h_out_c = Path::circle(h_out, 5.0);
                frame.fill(&h_out_c, Color::from_rgb8(19, 21, 26));
                frame.stroke(
                    &h_out_c,
                    Stroke::default()
                        .with_color(Color::WHITE)
                        .with_width(1.5),
                );
            }
        }

        // Draggable focus center handle
        let c_circle = Path::circle(center_pt, 7.5);
        frame.fill(&c_circle, palette::ACCENT);
        frame.stroke(
            &c_circle,
            Stroke::default()
                .with_color(Color::WHITE)
                .with_width(1.5),
        );
        frame.fill(&Path::circle(center_pt, 2.0), Color::WHITE);
    }

    /// Motion Blur center reticle and angle arrow.
    fn draw_motion_blur_overlays(
        &self,
        frame: &mut Frame,
        bounds: Rectangle,
        params: &snap_core::ops_ext::MotionBlurParams,
    ) {
        let center_pt = match self.image_uv_to_screen(params.center, bounds) {
            Some(pt) => pt,
            None => return,
        };

        let arrow_len = 65.0f32;
        let rad = params.angle_deg.to_radians();
        let dir = (rad.cos(), rad.sin());
        let norm = (-rad.sin(), rad.cos());
        let tip = Point::new(
            center_pt.x + dir.0 * arrow_len,
            center_pt.y + dir.1 * arrow_len,
        );

        // Arrow shaft
        frame.stroke(
            &Path::line(center_pt, tip),
            Stroke::default()
                .with_color(Color::WHITE)
                .with_width(2.0),
        );

        // Arrowhead wings
        let wing1 = Point::new(
            tip.x - dir.0 * 12.0 + norm.0 * 6.0,
            tip.y - dir.1 * 12.0 + norm.1 * 6.0,
        );
        let wing2 = Point::new(
            tip.x - dir.0 * 12.0 - norm.0 * 6.0,
            tip.y - dir.1 * 12.0 - norm.1 * 6.0,
        );
        frame.stroke(
            &Path::line(tip, wing1),
            Stroke::default().with_color(Color::WHITE).with_width(2.0),
        );
        frame.stroke(
            &Path::line(tip, wing2),
            Stroke::default().with_color(Color::WHITE).with_width(2.0),
        );

        // Direction angle handle
        let tip_c = Path::circle(tip, 6.0);
        frame.fill(&tip_c, palette::ACCENT);
        frame.stroke(
            &tip_c,
            Stroke::default()
                .with_color(Color::WHITE)
                .with_width(1.5),
        );

        // Draggable center reticle
        let c_circle = Path::circle(center_pt, 8.0);
        frame.fill(&c_circle, Color::from_rgb8(19, 21, 26));
        frame.stroke(
            &c_circle,
            Stroke::default()
                .with_color(palette::ACCENT)
                .with_width(2.0),
        );
        frame.fill(&Path::circle(center_pt, 2.5), palette::ACCENT);

        let tick_s = Stroke::default().with_color(palette::ACCENT).with_width(1.2);
        frame.stroke(
            &Path::line(
                Point::new(center_pt.x - 13.0, center_pt.y),
                Point::new(center_pt.x - 8.0, center_pt.y),
            ),
            tick_s,
        );
        frame.stroke(
            &Path::line(
                Point::new(center_pt.x + 8.0, center_pt.y),
                Point::new(center_pt.x + 13.0, center_pt.y),
            ),
            tick_s,
        );
        frame.stroke(
            &Path::line(
                Point::new(center_pt.x, center_pt.y - 13.0),
                Point::new(center_pt.x, center_pt.y - 8.0),
            ),
            tick_s,
        );
        frame.stroke(
            &Path::line(
                Point::new(center_pt.x, center_pt.y + 8.0),
                Point::new(center_pt.x, center_pt.y + 13.0),
            ),
            tick_s,
        );
    }

    /// Portrait face center reticle and radius ring.
    fn draw_portrait_overlays(
        &self,
        frame: &mut Frame,
        bounds: Rectangle,
        params: &snap_core::ops_ext::PortraitParams,
    ) {
        let quad = match self.image_quad_rect(bounds) {
            Some(q) => q,
            None => return,
        };
        let center_pt = match self.image_uv_to_screen(params.face_center, bounds) {
            Some(pt) => pt,
            None => return,
        };
        let scale = quad.width.min(quad.height);
        let r_px = (params.face_radius * scale).max(10.0);

        let face_circle = Path::circle(center_pt, r_px);
        frame.fill(
            &face_circle,
            Color {
                r: 0.96,
                g: 0.62,
                b: 0.15,
                a: 0.06,
            },
        );
        frame.stroke(
            &face_circle,
            Stroke::default()
                .with_color(Color {
                    r: 0.96,
                    g: 0.62,
                    b: 0.15,
                    a: 0.75,
                })
                .with_width(1.5),
        );

        let h_pt = Point::new(center_pt.x + r_px, center_pt.y);
        let h_c = Path::circle(h_pt, 6.0);
        frame.fill(&h_c, Color::WHITE);
        frame.stroke(
            &h_c,
            Stroke::default()
                .with_color(palette::AMBER)
                .with_width(1.5),
        );

        let c_circle = Path::circle(center_pt, 7.5);
        frame.fill(&c_circle, palette::AMBER);
        frame.stroke(
            &c_circle,
            Stroke::default()
                .with_color(Color::WHITE)
                .with_width(1.5),
        );
        frame.fill(&Path::circle(center_pt, 2.0), Color::WHITE);
    }

    /// Head Pose face center and yaw/pitch vector.
    fn draw_head_pose_overlays(
        &self,
        frame: &mut Frame,
        bounds: Rectangle,
        params: &snap_core::ops_ext::HeadPoseParams,
    ) {
        let center_pt = match self.image_uv_to_screen(params.face_center, bounds) {
            Some(pt) => pt,
            None => return,
        };

        let dx = (params.yaw_x / 100.0) * 55.0;
        let dy = (params.pitch_y / 100.0) * 55.0;
        let tip = Point::new(center_pt.x + dx, center_pt.y + dy);

        frame.stroke(
            &Path::line(center_pt, tip),
            Stroke::default()
                .with_color(palette::ACCENT)
                .with_width(2.0),
        );

        let tip_c = Path::circle(tip, 6.0);
        frame.fill(&tip_c, Color::WHITE);
        frame.stroke(
            &tip_c,
            Stroke::default()
                .with_color(palette::ACCENT)
                .with_width(2.0),
        );

        let c_c = Path::circle(center_pt, 7.5);
        frame.fill(&c_c, palette::ACCENT);
        frame.stroke(
            &c_c,
            Stroke::default()
                .with_color(Color::WHITE)
                .with_width(1.5),
        );
        frame.fill(&Path::circle(center_pt, 2.0), Color::WHITE);
    }

    /// Text Overlay dashed bounding guide, center, and corner resize handle.
    fn draw_text_overlays(
        &self,
        frame: &mut Frame,
        bounds: Rectangle,
        params: &snap_core::ops_ext::TextOverlayParams,
    ) {
        let center_pt = match self.image_uv_to_screen(params.center, bounds) {
            Some(pt) => pt,
            None => return,
        };

        let box_w = 200.0 * params.scale;
        let box_h = 70.0 * params.scale;
        let top_left = Point::new(
            center_pt.x - box_w * 0.5,
            center_pt.y - box_h * 0.5,
        );

        let box_path = Path::rectangle(top_left, Size::new(box_w, box_h));
        frame.fill(
            &box_path,
            Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 0.04,
            },
        );
        frame.stroke(
            &box_path,
            Stroke::default()
                .with_color(Color {
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    a: 0.50,
                })
                .with_width(1.0),
        );

        let c_c = Path::circle(center_pt, 6.5);
        frame.fill(&c_c, palette::ACCENT);
        frame.stroke(
            &c_c,
            Stroke::default()
                .with_color(Color::WHITE)
                .with_width(1.5),
        );

        let br_pt = Point::new(
            center_pt.x + box_w * 0.5,
            center_pt.y + box_h * 0.5,
        );
        let br_c = Path::circle(br_pt, 5.5);
        frame.fill(&br_c, Color::WHITE);
        frame.stroke(
            &br_c,
            Stroke::default()
                .with_color(palette::ACCENT)
                .with_width(1.5),
        );
    }

    /// High-contrast feathered dual-ring cursor for Brush, Healing, and LayerMask painting.
    fn draw_brush_cursor(
        &self,
        frame: &mut Frame,
        bounds: Rectangle,
        state: &CanvasOverlayState,
    ) {
        let cursor_pos = match state.cursor_pos {
            Some(pos) => pos,
            None => return,
        };

        let is_brush = self.ctx.active_tool == Some(ToolId::Brush);
        let is_heal = self.ctx.active_tool == Some(ToolId::Healing);
        let is_mask = self.ctx.mask_paint_active;

        if !is_brush && !is_heal && !is_mask {
            return;
        }

        let quad = match self.image_quad_rect(bounds) {
            Some(q) => q,
            None => return,
        };
        let scale = quad.width.min(quad.height);

        let radius_norm = if is_mask {
            self.ctx.mask_brush_radius
        } else if is_brush {
            if let Some(EditOp::Brush(b)) = self.ctx.active_op {
                b.radius
            } else {
                0.10
            }
        } else if is_heal {
            if let Some(EditOp::Healing(h)) = self.ctx.active_op {
                h.brush_radius
            } else {
                0.05
            }
        } else {
            0.05
        };

        let r_px = (radius_norm * scale).clamp(4.0, 500.0);

        // Outer white or ruby-red ring
        let outer_c = Path::circle(cursor_pos, r_px);
        let ring_color = if is_mask {
            Color {
                r: 0.94,
                g: 0.20,
                b: 0.25,
                a: 0.90,
            }
        } else {
            Color::WHITE
        };
        frame.stroke(
            &outer_c,
            Stroke::default()
                .with_color(ring_color)
                .with_width(1.5),
        );

        // Inner dark contrast ring for high readability on bright photos
        if r_px > 3.0 {
            let inner_c = Path::circle(cursor_pos, r_px - 1.5);
            frame.stroke(
                &inner_c,
                Stroke::default()
                    .with_color(Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 0.65,
                    })
                    .with_width(1.0),
            );
        }

        // Center dot
        frame.fill(&Path::circle(cursor_pos, 1.5), Color::WHITE);
    }
}

// ----------------------------------------------------------------------------
// canvas::Program Implementation
// ----------------------------------------------------------------------------

impl<'a> canvas::Program<CanvasOverlayEvent> for CanvasOverlayWidget<'a> {
    type State = CanvasOverlayState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<Action<CanvasOverlayEvent>> {
        let cursor_pos = cursor.position_in(bounds);
        state.cursor_pos = cursor_pos;

        match event {
            // NEVER capture WheelScrolled -> pass through to CanvasShaderProgram underneath for zoom!
            canvas::Event::Mouse(mouse::Event::WheelScrolled { .. }) => None,

            // Track modifier keys for Alt-pan pass-through
            canvas::Event::Keyboard(iced::keyboard::Event::ModifiersChanged(mods)) => {
                state.is_shift_down = mods.shift();
                state.is_alt_down = mods.alt();
                None
            }

            // Left click handling
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let Some(pos) = cursor_pos else {
                    return None;
                };

                // NEVER capture Alt-left-click -> pass through to CanvasShaderProgram underneath for pan!
                if state.is_alt_down {
                    return None;
                }

                // 1. Mask painting mode in View Edits
                if self.ctx.mask_paint_active {
                    if let Some((node_id, _)) = self.ctx.mask_edit_node {
                        if let Some(uv) = self.screen_to_image_uv(pos, bounds) {
                            state.drag_target = Some(DragTarget::MaskStroke(node_id));
                            state.last_uv = uv;
                            state.drag_start_cursor = Some(pos);
                            return Some(
                                Action::publish(CanvasOverlayEvent::MaskStrokeSegment {
                                    node_id,
                                    from: uv,
                                    to: uv,
                                })
                                .and_capture(),
                            );
                        }
                    }
                    return None;
                }

                // 2. Brush tool painting
                if self.ctx.active_tool == Some(ToolId::Brush) {
                    if let Some(uv) = self.screen_to_image_uv(pos, bounds) {
                        state.drag_target = Some(DragTarget::BrushStroke);
                        state.last_uv = uv;
                        state.drag_start_cursor = Some(pos);
                        return Some(
                            Action::publish(CanvasOverlayEvent::BrushStrokeSegment {
                                from: uv,
                                to: uv,
                            })
                            .and_capture(),
                        );
                    }
                    return None;
                }

                // 3. Healing tool painting
                if self.ctx.active_tool == Some(ToolId::Healing) {
                    if let Some(uv) = self.screen_to_image_uv(pos, bounds) {
                        state.drag_target = Some(DragTarget::HealStroke);
                        state.last_uv = uv;
                        state.drag_start_cursor = Some(pos);
                        return Some(
                            Action::publish(CanvasOverlayEvent::HealStrokeStart { uv })
                                .and_capture(),
                        );
                    }
                    return None;
                }

                // 4. Check interactive handles for active tool
                if let Some(target) = self.hit_test(pos, bounds, state) {
                    state.drag_target = Some(target);
                    state.drag_start_cursor = Some(pos);
                    if let Some(op) = self.ctx.active_op {
                        state.working_op = Some(op.clone());
                    }

                    // If selecting a different Selective pin, emit PreviewOp immediately
                    if let DragTarget::SelectivePin(idx) = target {
                        if let Some(EditOp::Selective(ref mut sel)) = state.working_op {
                            sel.active_pin = idx;
                            return Some(
                                Action::publish(CanvasOverlayEvent::PreviewOp(EditOp::Selective(
                                    sel.clone(),
                                )))
                                .and_capture(),
                            );
                        }
                    }

                    return Some(Action::request_redraw().and_capture());
                }

                // 5. Selective Add-Pin Mode: Click empty photo space to append a new U-Point pin
                if self.ctx.active_tool == Some(ToolId::Selective)
                    && self.ctx.selective_add_pin_mode
                {
                    if let Some(uv) = self.screen_to_image_uv(pos, bounds) {
                        if let Some(EditOp::Selective(sel)) = self.ctx.active_op {
                            if sel.pins.len() < MAX_SELECTIVE_PINS {
                                let mut new_sel = sel.clone();
                                new_sel.pins.push(SelectivePin {
                                    x: uv[0],
                                    y: uv[1],
                                    radius: 0.25,
                                    brightness: 0.0,
                                    contrast: 0.0,
                                    saturation: 0.0,
                                    structure: 0.0,
                                });
                                new_sel.active_pin = new_sel.pins.len() - 1;
                                return Some(
                                    Action::publish(CanvasOverlayEvent::CommitOp(
                                        EditOp::Selective(new_sel),
                                    ))
                                    .and_capture(),
                                );
                            }
                        }
                    }
                }

                // PASS-THROUGH DISCIPLINE: No handle was hit and not in painting/add-pin mode.
                // Return None so CanvasShaderProgram underneath handles swipe gestures!
                None
            }

            // Continuous dragging lifecycle
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let Some(pos) = cursor_pos else {
                    return None;
                };

                let Some(target) = state.drag_target else {
                    // When not dragging, redraw brush cursor smoothly if over active canvas
                    if self.ctx.mask_paint_active
                        || self.ctx.active_tool == Some(ToolId::Brush)
                        || self.ctx.active_tool == Some(ToolId::Healing)
                    {
                        return Some(Action::request_redraw());
                    }
                    return None;
                };

                let quad = self.image_quad_rect(bounds);
                let scale = quad.map(|q| q.width.min(q.height)).unwrap_or(1.0);

                match target {
                    DragTarget::MaskStroke(node_id) => {
                        if let Some(uv) = self.screen_to_image_uv_clamped(pos, bounds) {
                            let from = state.last_uv;
                            state.last_uv = uv;
                            return Some(
                                Action::publish(CanvasOverlayEvent::MaskStrokeSegment {
                                    node_id,
                                    from,
                                    to: uv,
                                })
                                .and_capture(),
                            );
                        }
                    }

                    DragTarget::BrushStroke => {
                        if let Some(uv) = self.screen_to_image_uv_clamped(pos, bounds) {
                            let from = state.last_uv;
                            state.last_uv = uv;
                            return Some(
                                Action::publish(CanvasOverlayEvent::BrushStrokeSegment {
                                    from,
                                    to: uv,
                                })
                                .and_capture(),
                            );
                        }
                    }

                    DragTarget::HealStroke => {
                        if let Some(uv) = self.screen_to_image_uv_clamped(pos, bounds) {
                            state.last_uv = uv;
                            return Some(
                                Action::publish(CanvasOverlayEvent::HealStrokeMove { uv })
                                    .and_capture(),
                            );
                        }
                    }

                    DragTarget::SelectivePin(idx) => {
                        if let Some(uv) = self.screen_to_image_uv_clamped(pos, bounds) {
                            if let Some(EditOp::Selective(ref mut sel)) = state.working_op {
                                if idx < sel.pins.len() {
                                    sel.pins[idx].x = uv[0];
                                    sel.pins[idx].y = uv[1];
                                    return Some(
                                        Action::publish(CanvasOverlayEvent::PreviewOp(
                                            EditOp::Selective(sel.clone()),
                                        ))
                                        .and_capture(),
                                    );
                                }
                            }
                        }
                    }

                    DragTarget::SelectiveRadius(idx) => {
                        if let Some(EditOp::Selective(ref mut sel)) = state.working_op {
                            if idx < sel.pins.len() {
                                if let Some(pin_screen) =
                                    self.image_uv_to_screen([sel.pins[idx].x, sel.pins[idx].y], bounds)
                                {
                                    let dist = (dist_sq(pos, pin_screen)).sqrt();
                                    sel.pins[idx].radius = (dist / scale).clamp(0.01, 1.0);
                                    return Some(
                                        Action::publish(CanvasOverlayEvent::PreviewOp(
                                            EditOp::Selective(sel.clone()),
                                        ))
                                        .and_capture(),
                                    );
                                }
                            }
                        }
                    }

                    DragTarget::LensBlurCenter => {
                        if let Some(uv) = self.screen_to_image_uv_clamped(pos, bounds) {
                            if let Some(EditOp::LensBlur(ref mut lb)) = state.working_op {
                                lb.center = uv;
                                return Some(
                                    Action::publish(CanvasOverlayEvent::PreviewOp(
                                        EditOp::LensBlur(lb.clone()),
                                    ))
                                    .and_capture(),
                                );
                            }
                        }
                    }

                    DragTarget::LensBlurInnerRadius => {
                        if let Some(EditOp::LensBlur(ref mut lb)) = state.working_op {
                            if let Some(center_pt) = self.image_uv_to_screen(lb.center, bounds) {
                                let dist = (dist_sq(pos, center_pt)).sqrt();
                                lb.inner_radius = (dist / scale).clamp(0.01, 1.0);

                                if lb.shape == LensBlurShape::Linear {
                                    let dx = pos.x - center_pt.x;
                                    let dy = pos.y - center_pt.y;
                                    let angle = (dy.atan2(dx).to_degrees() - 90.0).rem_euclid(360.0);
                                    lb.angle_deg = if angle > 180.0 { angle - 360.0 } else { angle };
                                }

                                return Some(
                                    Action::publish(CanvasOverlayEvent::PreviewOp(
                                        EditOp::LensBlur(lb.clone()),
                                    ))
                                    .and_capture(),
                                );
                            }
                        }
                    }

                    DragTarget::LensBlurTransition => {
                        if let Some(EditOp::LensBlur(ref mut lb)) = state.working_op {
                            if let Some(center_pt) = self.image_uv_to_screen(lb.center, bounds) {
                                let dist = (dist_sq(pos, center_pt)).sqrt();
                                let outer = (dist / scale).clamp(lb.inner_radius + 0.01, 2.0);
                                lb.transition = (outer - lb.inner_radius).clamp(0.01, 1.0);
                                return Some(
                                    Action::publish(CanvasOverlayEvent::PreviewOp(
                                        EditOp::LensBlur(lb.clone()),
                                    ))
                                    .and_capture(),
                                );
                            }
                        }
                    }

                    DragTarget::MotionBlurCenter => {
                        if let Some(uv) = self.screen_to_image_uv_clamped(pos, bounds) {
                            if let Some(EditOp::MotionBlur(ref mut mb)) = state.working_op {
                                mb.center = uv;
                                return Some(
                                    Action::publish(CanvasOverlayEvent::PreviewOp(
                                        EditOp::MotionBlur(mb.clone()),
                                    ))
                                    .and_capture(),
                                );
                            }
                        }
                    }

                    DragTarget::MotionBlurAngle => {
                        if let Some(EditOp::MotionBlur(ref mut mb)) = state.working_op {
                            if let Some(center_pt) = self.image_uv_to_screen(mb.center, bounds) {
                                let dx = pos.x - center_pt.x;
                                let dy = pos.y - center_pt.y;
                                mb.angle_deg = dy.atan2(dx).to_degrees().clamp(-180.0, 180.0);
                                return Some(
                                    Action::publish(CanvasOverlayEvent::PreviewOp(
                                        EditOp::MotionBlur(mb.clone()),
                                    ))
                                    .and_capture(),
                                );
                            }
                        }
                    }

                    DragTarget::PortraitCenter => {
                        if let Some(uv) = self.screen_to_image_uv_clamped(pos, bounds) {
                            if let Some(EditOp::Portrait(ref mut p)) = state.working_op {
                                p.face_center = uv;
                                return Some(
                                    Action::publish(CanvasOverlayEvent::PreviewOp(
                                        EditOp::Portrait(p.clone()),
                                    ))
                                    .and_capture(),
                                );
                            }
                        }
                    }

                    DragTarget::PortraitRadius => {
                        if let Some(EditOp::Portrait(ref mut p)) = state.working_op {
                            if let Some(center_pt) = self.image_uv_to_screen(p.face_center, bounds) {
                                let dist = (dist_sq(pos, center_pt)).sqrt();
                                p.face_radius = (dist / scale).clamp(0.02, 0.8);
                                return Some(
                                    Action::publish(CanvasOverlayEvent::PreviewOp(
                                        EditOp::Portrait(p.clone()),
                                    ))
                                    .and_capture(),
                                );
                            }
                        }
                    }

                    DragTarget::HeadPoseCenter => {
                        if let Some(uv) = self.screen_to_image_uv_clamped(pos, bounds) {
                            if let Some(EditOp::HeadPose(ref mut hp)) = state.working_op {
                                hp.face_center = uv;
                                return Some(
                                    Action::publish(CanvasOverlayEvent::PreviewOp(
                                        EditOp::HeadPose(hp.clone()),
                                    ))
                                    .and_capture(),
                                );
                            }
                        }
                    }

                    DragTarget::HeadPoseDirection => {
                        if let Some(EditOp::HeadPose(ref mut hp)) = state.working_op {
                            if let Some(center_pt) = self.image_uv_to_screen(hp.face_center, bounds) {
                                let dx = pos.x - center_pt.x;
                                let dy = pos.y - center_pt.y;
                                hp.yaw_x = ((dx / 55.0) * 100.0).clamp(-100.0, 100.0);
                                hp.pitch_y = ((dy / 55.0) * 100.0).clamp(-100.0, 100.0);
                                return Some(
                                    Action::publish(CanvasOverlayEvent::PreviewOp(
                                        EditOp::HeadPose(hp.clone()),
                                    ))
                                    .and_capture(),
                                );
                            }
                        }
                    }

                    DragTarget::TextOverlayCenter => {
                        if let Some(uv) = self.screen_to_image_uv_clamped(pos, bounds) {
                            if let Some(EditOp::TextOverlay(ref mut txt)) = state.working_op {
                                txt.center = uv;
                                return Some(
                                    Action::publish(CanvasOverlayEvent::PreviewOp(
                                        EditOp::TextOverlay(txt.clone()),
                                    ))
                                    .and_capture(),
                                );
                            }
                        }
                    }

                    DragTarget::TextOverlayScale => {
                        if let Some(EditOp::TextOverlay(ref mut txt)) = state.working_op {
                            if let Some(center_pt) = self.image_uv_to_screen(txt.center, bounds) {
                                let dx = (pos.x - center_pt.x).abs();
                                let dy = (pos.y - center_pt.y).abs();
                                let dist = (dx * dx + dy * dy).sqrt();
                                let base = (100.0f32 * 100.0 + 35.0 * 35.0).sqrt();
                                txt.scale = (dist / base).clamp(0.1, 3.0);
                                return Some(
                                    Action::publish(CanvasOverlayEvent::PreviewOp(
                                        EditOp::TextOverlay(txt.clone()),
                                    ))
                                    .and_capture(),
                                );
                            }
                        }
                    }
                }

                None
            }

            // Left mouse release lifecycle: commit operations
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                let Some(target) = state.drag_target.take() else {
                    return None;
                };

                state.drag_start_cursor = None;

                match target {
                    DragTarget::MaskStroke(node_id) => {
                        state.working_op = None;
                        Some(
                            Action::publish(CanvasOverlayEvent::MaskStrokeEnd { node_id })
                                .and_capture(),
                        )
                    }

                    DragTarget::BrushStroke => {
                        state.working_op = None;
                        Some(Action::publish(CanvasOverlayEvent::BrushStrokeEnd).and_capture())
                    }

                    DragTarget::HealStroke => {
                        state.working_op = None;
                        Some(Action::publish(CanvasOverlayEvent::HealStrokeEnd).and_capture())
                    }

                    _ => {
                        if let Some(op) = state.working_op.take() {
                            Some(Action::publish(CanvasOverlayEvent::CommitOp(op)).and_capture())
                        } else {
                            Some(Action::request_redraw().and_capture())
                        }
                    }
                }
            }

            _ => None,
        }
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        // 1. Translucent ruby-red LayerMask preview heatmap
        self.draw_mask_red_overlay(&mut frame, bounds);

        // 2. Active tool vector handles (using live working op during drag if available)
        let active_op = state.working_op.as_ref().or(self.ctx.active_op);
        if let Some(op) = active_op {
            match op {
                EditOp::Selective(sel) => {
                    self.draw_selective_overlays(&mut frame, bounds, sel);
                }
                EditOp::LensBlur(lb) => {
                    self.draw_lens_blur_overlays(&mut frame, bounds, lb);
                }
                EditOp::MotionBlur(mb) => {
                    self.draw_motion_blur_overlays(&mut frame, bounds, mb);
                }
                EditOp::Portrait(p) => {
                    self.draw_portrait_overlays(&mut frame, bounds, p);
                }
                EditOp::HeadPose(hp) => {
                    self.draw_head_pose_overlays(&mut frame, bounds, hp);
                }
                EditOp::TextOverlay(txt) => {
                    self.draw_text_overlays(&mut frame, bounds, txt);
                }
                _ => {}
            }
        }

        // 3. Dual-ring feathered brush cursor
        self.draw_brush_cursor(&mut frame, bounds, state);

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if state.drag_target.is_some() {
            return mouse::Interaction::Grabbing;
        }

        let Some(pos) = cursor.position_in(bounds) else {
            return mouse::Interaction::default();
        };

        if self.ctx.mask_paint_active
            || self.ctx.active_tool == Some(ToolId::Brush)
            || self.ctx.active_tool == Some(ToolId::Healing)
        {
            return mouse::Interaction::Crosshair;
        }

        if self.ctx.active_tool == Some(ToolId::Selective) && self.ctx.selective_add_pin_mode {
            return mouse::Interaction::Crosshair;
        }

        if self.hit_test(pos, bounds, state).is_some() {
            return mouse::Interaction::Pointer;
        }

        mouse::Interaction::default()
    }
}

// ----------------------------------------------------------------------------
// Mathematical Helpers
// ----------------------------------------------------------------------------

#[inline]
fn dist_sq(a: Point, b: Point) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx * dx + dy * dy
}
