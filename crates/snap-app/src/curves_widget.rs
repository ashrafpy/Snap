use iced::border;
use iced::mouse;
use iced::widget::canvas::{self, Action, Frame, Geometry, LineCap, LineJoin, Path, Stroke};
use iced::{Color, Point, Rectangle, Renderer, Theme};
use snap_core::curves::{CurveChannel, CurveChannelId, CurvePoint, CurvePreset, CurvesParams};
use snap_core::image_types::HistogramData;

const ALL_CHANNELS: [CurveChannelId; 5] = [
    CurveChannelId::Rgb,
    CurveChannelId::Red,
    CurveChannelId::Green,
    CurveChannelId::Blue,
    CurveChannelId::Luminance,
];

fn channel_for_id<'b>(params: &'b CurvesParams, id: CurveChannelId) -> &'b CurveChannel {
    match id {
        CurveChannelId::Rgb => &params.rgb,
        CurveChannelId::Red => &params.red,
        CurveChannelId::Green => &params.green,
        CurveChannelId::Blue => &params.blue,
        CurveChannelId::Luminance => &params.luma,
    }
}

fn channel_color_for(id: CurveChannelId) -> Color {
    match id {
        CurveChannelId::Rgb => Color::from_rgb8(243, 244, 246),       // #F3F4F6
        CurveChannelId::Red => Color::from_rgb8(248, 113, 113),       // #F87171
        CurveChannelId::Green => Color::from_rgb8(52, 211, 153),      // #34D399
        CurveChannelId::Blue => Color::from_rgb8(96, 165, 250),       // #60A5FA
        CurveChannelId::Luminance => Color::from_rgb8(251, 191, 36),  // #FBBF24
    }
}

fn is_same_channel(a: CurveChannelId, b: CurveChannelId) -> bool {
    matches!(
        (a, b),
        (CurveChannelId::Rgb, CurveChannelId::Rgb)
            | (CurveChannelId::Red, CurveChannelId::Red)
            | (CurveChannelId::Green, CurveChannelId::Green)
            | (CurveChannelId::Blue, CurveChannelId::Blue)
            | (CurveChannelId::Luminance, CurveChannelId::Luminance)
    )
}

fn is_channel_identity(channel: &CurveChannel) -> bool {
    if channel.points.len() == 2 {
        let p0 = &channel.points[0];
        let p1 = &channel.points[1];
        (p0.x - 0.0).abs() < 1e-4
            && (p0.y - 0.0).abs() < 1e-4
            && (p1.x - 1.0).abs() < 1e-4
            && (p1.y - 1.0).abs() < 1e-4
    } else {
        false
    }
}

#[derive(Debug, Clone)]
pub enum CurvesEvent {
    PointAdded {
        channel: CurveChannelId,
        point: CurvePoint,
    },
    PointMoved {
        channel: CurveChannelId,
        index: usize,
        point: CurvePoint,
    },
    PointRemoved {
        channel: CurveChannelId,
        index: usize,
    },
    SelectChannel(CurveChannelId),
    ApplyPreset(CurvePreset),
    ResetChannel(CurveChannelId),
}

#[derive(Default)]
pub struct CurvesState {
    pub dragging_point_idx: Option<usize>,
    pub hover_point_idx: Option<usize>,
}

pub struct CurvesWidget<'a> {
    pub params: &'a CurvesParams,
    pub current_channel: CurveChannelId,
    pub histogram: &'a Option<HistogramData>,
    pub on_event: Box<dyn Fn(CurvesEvent) -> crate::app::Message + 'a>,
}

impl<'a> CurvesWidget<'a> {
    pub fn new(
        params: &'a CurvesParams,
        current_channel: CurveChannelId,
        histogram: &'a Option<HistogramData>,
        on_event: impl Fn(CurvesEvent) -> crate::app::Message + 'a,
    ) -> Self {
        Self {
            params,
            current_channel,
            histogram,
            on_event: Box::new(on_event),
        }
    }

    fn channel(&self) -> &CurveChannel {
        channel_for_id(self.params, self.current_channel)
    }

    fn channel_color(&self) -> Color {
        channel_color_for(self.current_channel)
    }
}

impl<'a> canvas::Program<crate::app::Message> for CurvesWidget<'a> {
    type State = CurvesState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<Action<crate::app::Message>> {
        let Some(cursor_pos) = cursor.position_in(bounds) else {
            if state.dragging_point_idx.is_some() {
                state.dragging_point_idx = None;
            }
            state.hover_point_idx = None;
            return None;
        };

        let norm_x = (cursor_pos.x / bounds.width).clamp(0.0, 1.0);
        let norm_y = (1.0 - (cursor_pos.y / bounds.height)).clamp(0.0, 1.0);
        let channel = self.channel();

        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                // Check if clicking near an existing control point
                let mut clicked_idx = None;
                for (idx, p) in channel.points.iter().enumerate() {
                    let px = p.x * bounds.width;
                    let py = (1.0 - p.y) * bounds.height;
                    let dist_sq = (cursor_pos.x - px).powi(2) + (cursor_pos.y - py).powi(2);
                    if dist_sq <= 144.0 {
                        // within 12px radius
                        clicked_idx = Some(idx);
                        break;
                    }
                }

                if let Some(idx) = clicked_idx {
                    state.dragging_point_idx = Some(idx);
                    return Some(Action::request_redraw());
                } else {
                    // Check if an existing control point already has (p.x - norm_x).abs() < 0.015
                    let mut close_x_idx = None;
                    for (idx, p) in channel.points.iter().enumerate() {
                        if (p.x - norm_x).abs() < 0.015 {
                            close_x_idx = Some(idx);
                            break;
                        }
                    }

                    if let Some(idx) = close_x_idx {
                        state.dragging_point_idx = Some(idx);
                        return Some(Action::request_redraw());
                    } else {
                        // Clicked on empty curve space -> add new control point!
                        let pt = CurvePoint::new(norm_x, norm_y);
                        let msg = (self.on_event)(CurvesEvent::PointAdded {
                            channel: self.current_channel,
                            point: pt,
                        });
                        state.dragging_point_idx = Some(channel.points.len()); // will resolve on re-render
                        return Some(Action::publish(msg));
                    }
                }
            }

            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let Some(idx) = state.dragging_point_idx {
                    if idx < channel.points.len() {
                        let is_first = idx == 0;
                        let is_last = idx == channel.points.len() - 1;

                        let clamped_x = if is_first {
                            0.0
                        } else if is_last {
                            1.0
                        } else {
                            // Keep interior point between its neighbors with slight padding
                            let min_x = channel.points[idx - 1].x + 0.005;
                            let max_x = channel.points[idx + 1].x - 0.005;
                            if max_x >= min_x { norm_x.clamp(min_x, max_x) } else { (min_x + max_x) * 0.5 }
                        };

                        let pt = CurvePoint::new(clamped_x, norm_y);
                        let msg = (self.on_event)(CurvesEvent::PointMoved {
                            channel: self.current_channel,
                            index: idx,
                            point: pt,
                        });
                        return Some(Action::publish(msg));
                    }
                } else {
                    // Hover detection
                    let mut hovered = None;
                    for (idx, p) in channel.points.iter().enumerate() {
                        let px = p.x * bounds.width;
                        let py = (1.0 - p.y) * bounds.height;
                        let dist_sq = (cursor_pos.x - px).powi(2) + (cursor_pos.y - py).powi(2);
                        if dist_sq <= 144.0 {
                            hovered = Some(idx);
                            break;
                        }
                    }
                    if state.hover_point_idx != hovered {
                        state.hover_point_idx = hovered;
                        return Some(Action::request_redraw());
                    }
                }
            }

            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if state.dragging_point_idx.is_some() {
                    state.dragging_point_idx = None;
                    return Some(Action::request_redraw());
                }
            }

            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
                // Delete interior control point
                for (idx, p) in channel.points.iter().enumerate() {
                    if idx == 0 || idx == channel.points.len() - 1 {
                        continue; // endpoints cannot be deleted
                    }
                    let px = p.x * bounds.width;
                    let py = (1.0 - p.y) * bounds.height;
                    let dist_sq = (cursor_pos.x - px).powi(2) + (cursor_pos.y - py).powi(2);
                    if dist_sq <= 144.0 {
                        state.dragging_point_idx = None;
                        state.hover_point_idx = None;
                        let msg = (self.on_event)(CurvesEvent::PointRemoved {
                            channel: self.current_channel,
                            index: idx,
                        });
                        return Some(Action::publish(msg));
                    }
                }
            }

            _ => {}
        }

        None
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
        let width = bounds.width;
        let height = bounds.height;

        // Rounded darkroom frame (#0D0F13) with subtle border
        let bg = Path::rounded_rectangle(Point::ORIGIN, bounds.size(), border::Radius::from(10.0));
        frame.fill(&bg, Color::from_rgb8(13, 15, 19));
        frame.stroke(
            &bg,
            Stroke::default()
                .with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.08))
                .with_width(1.0),
        );

        // Soft smoothed background histogram silhouette (0.08 white alpha fill)
        if let Some(hist) = self.histogram.as_ref() {
            let bins = match self.current_channel {
                CurveChannelId::Rgb | CurveChannelId::Luminance => &hist.luma,
                CurveChannelId::Red => &hist.r,
                CurveChannelId::Green => &hist.g,
                CurveChannelId::Blue => &hist.b,
            };

            let mut smoothed = [0.0f32; 256];
            for i in 0..256 {
                let prev = if i > 0 { bins[i - 1] as f32 } else { bins[0] as f32 };
                let curr = bins[i] as f32;
                let next = if i < 255 { bins[i + 1] as f32 } else { bins[255] as f32 };
                smoothed[i] = 0.25 * prev + 0.5 * curr + 0.25 * next;
            }

            let mut max_val = 1.0f32;
            for i in 1..255 {
                max_val = max_val.max(smoothed[i]);
            }

            let mut builder = canvas::path::Builder::new();
            builder.move_to(Point::new(0.0, height));
            for (i, &val) in smoothed.iter().enumerate() {
                let x = (i as f32 / 255.0) * width;
                let norm = (val / max_val).clamp(0.0, 1.0);
                let comp = norm.powf(0.60);
                let y = height - comp * (height - 4.0);
                builder.line_to(Point::new(x, y));
            }
            builder.line_to(Point::new(width, height));
            builder.close();
            frame.fill(&builder.build(), Color::from_rgba(1.0, 1.0, 1.0, 0.08));
        }

        // 4x4 exposure grid (25%, 50%, 75% horizontal and vertical lines at 0.055 white alpha)
        let grid_stroke = Stroke::default()
            .with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.055))
            .with_width(1.0);
        for i in 1..4 {
            let frac = i as f32 * 0.25;
            let x = frac * width;
            let y = frac * height;

            let mut v_line = canvas::path::Builder::new();
            v_line.move_to(Point::new(x, 0.0));
            v_line.line_to(Point::new(x, height));
            frame.stroke(&v_line.build(), grid_stroke);

            let mut h_line = canvas::path::Builder::new();
            h_line.move_to(Point::new(0.0, y));
            h_line.line_to(Point::new(width, y));
            frame.stroke(&h_line.build(), grid_stroke);
        }

        // Subtle diagonal baseline (0, height) -> (width, 0) at 0.13 white alpha
        let mut diag = canvas::path::Builder::new();
        diag.move_to(Point::new(0.0, height));
        diag.line_to(Point::new(width, 0.0));
        frame.stroke(
            &diag.build(),
            Stroke::default()
                .with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.13))
                .with_width(1.0),
        );

        // Inactive non-identity channels drawn at 0.35 alpha (1.25px width)
        for &other_id in &ALL_CHANNELS {
            if is_same_channel(other_id, self.current_channel) {
                continue;
            }
            let other_channel = channel_for_id(self.params, other_id);
            if is_channel_identity(other_channel) {
                continue;
            }

            let mut inactive_builder = canvas::path::Builder::new();
            let samples = 128;
            for i in 0..=samples {
                let t = i as f32 / samples as f32;
                let y_val = other_channel.eval(t);
                let px = t * width;
                let py = (1.0 - y_val) * height;

                if i == 0 {
                    inactive_builder.move_to(Point::new(px, py));
                } else {
                    inactive_builder.line_to(Point::new(px, py));
                }
            }
            let c = channel_color_for(other_id);
            frame.stroke(
                &inactive_builder.build(),
                Stroke::default()
                    .with_color(Color { a: 0.35, ..c })
                    .with_width(1.25)
                    .with_line_cap(LineCap::Round)
                    .with_line_join(LineJoin::Round),
            );
        }

        let channel = self.channel();
        let color = self.channel_color();

        // Active channel spline across 128 samples with 2.25px width, LineCap::Round, LineJoin::Round
        let mut curve_builder = canvas::path::Builder::new();
        let samples = 128;
        for i in 0..=samples {
            let t = i as f32 / samples as f32;
            let y_val = channel.eval(t);
            let px = t * width;
            let py = (1.0 - y_val) * height;

            if i == 0 {
                curve_builder.move_to(Point::new(px, py));
            } else {
                curve_builder.line_to(Point::new(px, py));
            }
        }
        frame.stroke(
            &curve_builder.build(),
            Stroke::default()
                .with_color(color)
                .with_width(2.25)
                .with_line_cap(LineCap::Round)
                .with_line_join(LineJoin::Round),
        );

        // Control point handles: soft outer halo (radius: 10.0 when active) + crisp circular ring (radius: 5.2)
        for (idx, p) in channel.points.iter().enumerate() {
            let px = p.x * width;
            let py = (1.0 - p.y) * height;
            let center = Point::new(px, py);

            let is_hover = state.hover_point_idx == Some(idx);
            let is_drag = state.dragging_point_idx == Some(idx);
            let is_active = is_hover || is_drag;

            if is_active {
                // Soft outer halo when hovered/dragged
                let halo = Path::circle(center, 10.0);
                frame.fill(
                    &halo,
                    Color {
                        a: if is_drag { 0.40 } else { 0.22 },
                        ..color
                    },
                );
            }

            // Crisp circular ring (radius: 5.2)
            let circle = Path::circle(center, 5.2);
            let fill_color = if is_active {
                Color::WHITE
            } else {
                Color::from_rgb8(13, 15, 19)
            };
            frame.fill(&circle, fill_color);

            let stroke_color = if is_active {
                Color::WHITE
            } else {
                color
            };
            frame.stroke(
                &circle,
                Stroke::default()
                    .with_color(stroke_color)
                    .with_width(2.0),
            );
        }

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if state.dragging_point_idx.is_some() {
            return mouse::Interaction::Grabbing;
        }
        if let Some(pos) = cursor.position_in(bounds) {
            for p in &self.channel().points {
                let px = p.x * bounds.width;
                let py = (1.0 - p.y) * bounds.height;
                let dist_sq = (pos.x - px).powi(2) + (pos.y - py).powi(2);
                if dist_sq <= 144.0 {
                    return mouse::Interaction::Pointer;
                }
            }
            return mouse::Interaction::Crosshair;
        }
        mouse::Interaction::default()
    }
}
