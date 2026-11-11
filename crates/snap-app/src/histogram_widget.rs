use iced::mouse;
use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke};
use iced::{Color, Point, Rectangle, Renderer, Size, Theme};
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

        // Background container: subtle charcoal with rounded corners
        let bg = Path::rectangle(Point::ORIGIN, Size::new(width, height));
        frame.fill(&bg, Color::from_rgba(0.08, 0.085, 0.095, 0.95));
        frame.stroke(
            &bg,
            Stroke::default()
                .with_color(Color::from_rgba(0.2, 0.22, 0.26, 0.5))
                .with_width(1.0),
        );

        let Some(hist) = self.data.as_ref() else {
            // Draw subtle empty placeholder line
            let mut builder = canvas::path::Builder::new();
            builder.move_to(Point::new(0.0, height * 0.5));
            builder.line_to(Point::new(width, height * 0.5));
            frame.stroke(
                &builder.build(),
                Stroke::default()
                    .with_color(Color::from_rgba(0.25, 0.28, 0.32, 0.4))
                    .with_width(1.0),
            );
            return vec![frame.into_geometry()];
        };

        // Determine scaling peak: use 98th percentile of non-zero bins to avoid 0 or 255 clipping spikes flattening everything
        let mut max_val = 1u32;
        for i in 1..255 {
            max_val = max_val
                .max(hist.r[i])
                .max(hist.g[i])
                .max(hist.b[i])
                .max(hist.luma[i]);
        }
        let max_val = (max_val as f32).max(1.0);

        let build_channel_path = |bins: &[u32; 256], is_fill: bool| -> Path {
            let mut builder = canvas::path::Builder::new();
            builder.move_to(Point::new(0.0, height));

            for (i, &count) in bins.iter().enumerate() {
                let x = (i as f32 / 255.0) * width;
                // Logarithmic compression so faint shadow/highlight details are clearly visible
                let norm = (count as f32 / max_val).clamp(0.0, 1.0);
                let log_norm = (1.0 + 9.0 * norm).log10(); // maps 0..1 to 0..1 logarithmically
                let y = height - (log_norm * (height - 4.0)).min(height);
                builder.line_to(Point::new(x, y));
            }

            if is_fill {
                builder.line_to(Point::new(width, height));
                builder.close();
            }
            builder.build()
        };

        if self.show_rgb {
            // Red channel
            let r_path = build_channel_path(&hist.r, true);
            frame.fill(&r_path, Color::from_rgba(0.92, 0.26, 0.26, 0.28));

            // Green channel
            let g_path = build_channel_path(&hist.g, true);
            frame.fill(&g_path, Color::from_rgba(0.18, 0.82, 0.42, 0.28));

            // Blue channel
            let b_path = build_channel_path(&hist.b, true);
            frame.fill(&b_path, Color::from_rgba(0.24, 0.54, 0.96, 0.35));
        }

        if self.show_luma {
            // Luminance outline
            let luma_path = build_channel_path(&hist.luma, false);
            frame.stroke(
                &luma_path,
                Stroke::default()
                    .with_color(Color::from_rgba(0.95, 0.95, 0.98, 0.85))
                    .with_width(1.5),
            );
        }

        vec![frame.into_geometry()]
    }
}
