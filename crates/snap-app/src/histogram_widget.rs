use iced::border;
use iced::mouse;
use iced::widget::canvas::{self, Frame, Geometry, LineCap, LineJoin, Path, Stroke};
use iced::{Color, Point, Rectangle, Renderer, Theme};
use snap_core::image_types::HistogramData;

/// Canvas widget for rendering 4-channel GPU histogram (Red, Green, Blue, and Luminance).
pub struct HistogramWidget<'a> {
    pub data: &'a Option<HistogramData>,
    pub show_luma: bool,
    pub show_rgb: bool,
}

impl<'a> HistogramWidget<'a> {
    pub fn new(data: &'a Option<HistogramData>) -> Self {
        Self {
            data,
            show_luma: true,
            show_rgb: true,
        }
    }
}

impl<'a, Message> canvas::Program<Message> for HistogramWidget<'a> {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let width = bounds.width;
        let height = bounds.height;

        // Rounded darkroom card (#0D0F13) with subtle border
        let bg = Path::rounded_rectangle(Point::ORIGIN, bounds.size(), border::Radius::from(8.0));
        frame.fill(&bg, Color::from_rgb8(13, 15, 19));
        frame.stroke(
            &bg,
            Stroke::default()
                .with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.07))
                .with_width(1.0),
        );

        // 3 subtle vertical exposure zone dividers at 0.25, 0.50, 0.75 width
        let zone_stroke = Stroke::default()
            .with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.045))
            .with_width(1.0);
        for frac in [0.25f32, 0.50, 0.75] {
            let x = frac * width;
            let mut divider = canvas::path::Builder::new();
            divider.move_to(Point::new(x, 0.0));
            divider.line_to(Point::new(x, height));
            frame.stroke(&divider.build(), zone_stroke);
        }

        let Some(hist) = self.data.as_ref() else {
            // Quiet horizontal baseline near height - 6.0 when no data
            let baseline_y = (height - 6.0).max(0.0);
            let mut baseline = canvas::path::Builder::new();
            baseline.move_to(Point::new(0.0, baseline_y));
            baseline.line_to(Point::new(width, baseline_y));
            frame.stroke(
                &baseline.build(),
                Stroke::default()
                    .with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.06))
                    .with_width(1.0),
            );
            return vec![frame.into_geometry()];
        };

        // Smooth 256 bins (0.25 * prev + 0.5 * curr + 0.25 * next)
        let smooth_bins = |raw: &[u32; 256]| -> [f32; 256] {
            let mut smoothed = [0.0f32; 256];
            for i in 0..256 {
                let prev = if i > 0 { raw[i - 1] as f32 } else { raw[0] as f32 };
                let curr = raw[i] as f32;
                let next = if i < 255 { raw[i + 1] as f32 } else { raw[255] as f32 };
                smoothed[i] = 0.25 * prev + 0.5 * curr + 0.25 * next;
            }
            smoothed
        };

        let r_smoothed = smooth_bins(&hist.r);
        let g_smoothed = smooth_bins(&hist.g);
        let b_smoothed = smooth_bins(&hist.b);
        let luma_smoothed = smooth_bins(&hist.luma);

        // Determine scaling peak using non-edge bins to prevent clipping flattening midtones
        let mut max_val = 1.0f32;
        for i in 1..255 {
            max_val = max_val
                .max(r_smoothed[i])
                .max(g_smoothed[i])
                .max(b_smoothed[i])
                .max(luma_smoothed[i]);
        }

        // Inset plotting area so peaks never clip harshly against borders
        let pad_x = 4.0f32;
        let pad_top = 6.0f32;
        let pad_bottom = 3.0f32;
        let plot_w = (width - 2.0 * pad_x).max(1.0);
        let plot_h = (height - pad_top - pad_bottom).max(1.0);
        let base_y = height - pad_bottom;

        let draw_channel = |frame: &mut Frame,
                            bins: &[f32; 256],
                            fill_color: Color,
                            stroke_color: Color,
                            stroke_width: f32| {
            let mut fill_builder = canvas::path::Builder::new();
            let mut stroke_builder = canvas::path::Builder::new();

            fill_builder.move_to(Point::new(pad_x, base_y));

            for (i, &val) in bins.iter().enumerate() {
                let x = pad_x + (i as f32 / 255.0) * plot_w;
                let norm = (val / max_val).clamp(0.0, 1.0);
                // Soft photographic compression
                let comp = norm.powf(0.60);
                let y = base_y - comp * plot_h;

                fill_builder.line_to(Point::new(x, y));
                if i == 0 {
                    stroke_builder.move_to(Point::new(x, y));
                } else {
                    stroke_builder.line_to(Point::new(x, y));
                }
            }

            fill_builder.line_to(Point::new(pad_x + plot_w, base_y));
            fill_builder.close();

            frame.fill(&fill_builder.build(), fill_color);
            frame.stroke(
                &stroke_builder.build(),
                Stroke::default()
                    .with_color(stroke_color)
                    .with_width(stroke_width)
                    .with_line_cap(LineCap::Round)
                    .with_line_join(LineJoin::Round),
            );
        };

        if self.show_rgb {
            // Red channel (#F87171 at 0.20 fill / 0.65 stroke, 1.2px)
            draw_channel(
                &mut frame,
                &r_smoothed,
                Color { a: 0.20, ..Color::from_rgb8(248, 113, 113) },
                Color { a: 0.65, ..Color::from_rgb8(248, 113, 113) },
                1.2,
            );

            // Green channel (#34D399 at 0.18 fill / 0.62 stroke, 1.2px)
            draw_channel(
                &mut frame,
                &g_smoothed,
                Color { a: 0.18, ..Color::from_rgb8(52, 211, 153) },
                Color { a: 0.62, ..Color::from_rgb8(52, 211, 153) },
                1.2,
            );

            // Blue channel (#60A5FA at 0.22 fill / 0.70 stroke, 1.2px)
            draw_channel(
                &mut frame,
                &b_smoothed,
                Color { a: 0.22, ..Color::from_rgb8(96, 165, 250) },
                Color { a: 0.70, ..Color::from_rgb8(96, 165, 250) },
                1.2,
            );
        }

        if self.show_luma {
            // Luminance channel (#F3F4F6 at 0.10 fill / 0.85 stroke, 1.4px)
            draw_channel(
                &mut frame,
                &luma_smoothed,
                Color { a: 0.10, ..Color::from_rgb8(243, 244, 246) },
                Color { a: 0.85, ..Color::from_rgb8(243, 244, 246) },
                1.4,
            );
        }

        vec![frame.into_geometry()]
    }
}
