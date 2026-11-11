use iced::mouse;
use iced::widget::canvas::{self, Action, Frame, Geometry, Path, Stroke};
use iced::{Color, Point, Rectangle, Renderer, Size, Theme};
use snap_core::curves::{CurveChannel, CurveChannelId, CurvePoint, CurvePreset, CurvesParams};
use snap_core::image_types::HistogramData;

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
        match self.current_channel {
            CurveChannelId::Rgb => &self.params.rgb,
            CurveChannelId::Red => &self.params.red,
            CurveChannelId::Green => &self.params.green,
            CurveChannelId::Blue => &self.params.blue,
            CurveChannelId::Luminance => &self.params.luma,
        }
    }

    fn channel_color(&self) -> Color {
        match self.current_channel {
            CurveChannelId::Rgb => Color::from_rgb(0.92, 0.92, 0.95),
            CurveChannelId::Red => Color::from_rgb(0.94, 0.28, 0.28),
            CurveChannelId::Green => Color::from_rgb(0.24, 0.86, 0.44),
            CurveChannelId::Blue => Color::from_rgb(0.32, 0.62, 0.98),
            CurveChannelId::Luminance => Color::from_rgb(0.96, 0.78, 0.22),
        }
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

        // Background
        let bg = Path::rectangle(Point::ORIGIN, Size::new(width, height));
        frame.fill(&bg, Color::from_rgba(0.07, 0.075, 0.085, 0.98));
        frame.stroke(
            &bg,
            Stroke::default()
                .with_color(Color::from_rgba(0.22, 0.24, 0.28, 0.7))
                .with_width(1.0),
        );

        // Background Histogram for current channel
        if let Some(hist) = self.histogram.as_ref() {
            let bins = match self.current_channel {
                CurveChannelId::Rgb | CurveChannelId::Luminance => &hist.luma,
                CurveChannelId::Red => &hist.r,
                CurveChannelId::Green => &hist.g,
                CurveChannelId::Blue => &hist.b,
            };

            let mut max_val = 1u32;
            for i in 1..255 {
                max_val = max_val.max(bins[i]);
            }
            let max_val = (max_val as f32).max(1.0);

            let mut builder = canvas::path::Builder::new();
            builder.move_to(Point::new(0.0, height));
            for (i, &count) in bins.iter().enumerate() {
                let x = (i as f32 / 255.0) * width;
                let norm = (count as f32 / max_val).clamp(0.0, 1.0);
                let log_norm = (1.0 + 9.0 * norm).log10();
                let y = height - log_norm * (height - 4.0);
                builder.line_to(Point::new(x, y));
            }
            builder.line_to(Point::new(width, height));
            builder.close();
            frame.fill(&builder.build(), Color::from_rgba(0.3, 0.35, 0.42, 0.18));
        }

        // Grid lines (quarter divisions: 25%, 50%, 75%)
        for i in 1..4 {
            let frac = i as f32 * 0.25;
            let x = frac * width;
            let y = frac * height;

            let mut v_line = canvas::path::Builder::new();
            v_line.move_to(Point::new(x, 0.0));
            v_line.line_to(Point::new(x, height));
            frame.stroke(
                &v_line.build(),
                Stroke::default()
                    .with_color(Color::from_rgba(0.18, 0.20, 0.24, 0.6))
                    .with_width(1.0),
            );

            let mut h_line = canvas::path::Builder::new();
            h_line.move_to(Point::new(0.0, y));
            h_line.line_to(Point::new(width, y));
            frame.stroke(
                &h_line.build(),
                Stroke::default()
                    .with_color(Color::from_rgba(0.18, 0.20, 0.24, 0.6))
                    .with_width(1.0),
            );
        }

        // Neutral baseline diagonal (0, 0) -> (1, 1)
        let mut diag = canvas::path::Builder::new();
        diag.move_to(Point::new(0.0, height));
        diag.line_to(Point::new(width, 0.0));
        frame.stroke(
            &diag.build(),
            Stroke::default()
                .with_color(Color::from_rgba(0.28, 0.30, 0.36, 0.45))
                .with_width(1.0),
        );

        let channel = self.channel();
        let color = self.channel_color();

        // Evaluated Spline Curve
        let mut curve_builder = canvas::path::Builder::new();
        let samples = 120;
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
            Stroke::default().with_color(color).with_width(2.2),
        );

        // Control Points
        for (idx, p) in channel.points.iter().enumerate() {
            let px = p.x * width;
            let py = (1.0 - p.y) * height;
            let center = Point::new(px, py);

            let is_hover = state.hover_point_idx == Some(idx);
            let is_drag = state.dragging_point_idx == Some(idx);

            if is_hover || is_drag {
                // Outer glow
                let glow = Path::circle(center, 9.0);
                frame.fill(
                    &glow,
                    Color::from_rgba(color.r, color.g, color.b, if is_drag { 0.45 } else { 0.25 }),
                );
            }

            // Core control point
            let circle = Path::circle(center, 5.0);
            frame.fill(&circle, Color::WHITE);
            frame.stroke(
                &circle,
                Stroke::default()
                    .with_color(Color::from_rgb(0.1, 0.1, 0.12))
                    .with_width(1.5),
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
