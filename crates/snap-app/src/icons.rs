use iced::mouse;
use iced::widget::canvas::{self, Frame, Geometry, LineCap, LineJoin, Path, Stroke};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Size, Theme};
use snap_core::ops::ToolId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Icon {
    ApertureLogo,
    FolderOpen,
    Sparkles,
    Sliders,
    Details,
    SplineCurve,
    Droplet,
    ColorWheel,
    Cube3D,
    ContrastBw,
    Vignette,
    Crop,
    SunRaw,
    Undo,
    Redo,
    Eye,
    EyeOff,
    Layers,
    Download,
    Save,
    Copy,
    Paste,
    Star,
    StarFilled,
    Check,
    Close,
    ChevronLeft,
    ChevronUp,
    ChevronDown,
    Pipette,
    Crosshair,
    RotateCcw,
    RotateCw,
    FlipH,
    FlipV,
    Trash,
    SidebarRight,
    ZoomIn,
    ZoomOut,
    Maximize,
    ImageFile,
    Histogram,
    ColorGrade,
    Dehaze,
    TonalContrast,
    Perspective,
    Expand,
    Selective,
    Brush,
    Healing,
    Halation,
    Bloom,
    GrainyFilm,
    Noir,
    HdrScape,
    GlamourGlow,
    Drama,
    Vintage,
    LensBlur,
    MotionBlur,
    DoubleExposure,
    Retrolux,
    Grunge,
    Portrait,
    HeadPose,
    Frames,
    TextOverlay,
    Mask,
    Shuffle,
}

impl Icon {
    pub fn for_tool(tool: ToolId) -> Self {
        match tool {
            ToolId::RawDevelop => Self::SunRaw,
            ToolId::TuneImage => Self::Sliders,
            ToolId::Details => Self::Details,
            ToolId::Curves => Self::SplineCurve,
            ToolId::WhiteBalance => Self::Droplet,
            ToolId::ColorHsl => Self::ColorWheel,
            ToolId::Lut3d => Self::Cube3D,
            ToolId::BlackAndWhite => Self::ContrastBw,
            ToolId::Vignette => Self::Vignette,
            ToolId::CropRotate => Self::Crop,
            ToolId::ColorGrade => Self::ColorGrade,
            ToolId::Dehaze => Self::Dehaze,
            ToolId::TonalContrast => Self::TonalContrast,
            ToolId::Perspective => Self::Perspective,
            ToolId::Expand => Self::Expand,
            ToolId::Selective => Self::Selective,
            ToolId::Brush => Self::Brush,
            ToolId::Healing => Self::Healing,
            ToolId::Halation => Self::Halation,
            ToolId::Bloom => Self::Bloom,
            ToolId::GrainyFilm => Self::GrainyFilm,
            ToolId::Noir => Self::Noir,
            ToolId::HdrScape => Self::HdrScape,
            ToolId::GlamourGlow => Self::GlamourGlow,
            ToolId::Drama => Self::Drama,
            ToolId::Vintage => Self::Vintage,
            ToolId::LensBlur => Self::LensBlur,
            ToolId::MotionBlur => Self::MotionBlur,
            ToolId::DoubleExposure => Self::DoubleExposure,
            ToolId::Retrolux => Self::Retrolux,
            ToolId::Grunge => Self::Grunge,
            ToolId::Portrait => Self::Portrait,
            ToolId::HeadPose => Self::HeadPose,
            ToolId::Frames => Self::Frames,
            ToolId::TextOverlay => Self::TextOverlay,
        }
    }
}

pub fn icon<'a, Message: 'a>(kind: Icon, size: f32, color: Color) -> Element<'a, Message> {
    iced::widget::canvas(IconCanvas { kind, color })
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .into()
}

struct IconCanvas {
    kind: Icon,
    color: Color,
}

impl<Message> canvas::Program<Message> for IconCanvas {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let w = bounds.width;
        let h = bounds.height;
        let mut frame = Frame::new(renderer, Size::new(w, h));
        if w <= 0.0 || h <= 0.0 {
            return vec![frame.into_geometry()];
        }

        let dim = w.min(h);
        let offset_x = (w - dim) * 0.5;
        let offset_y = (h - dim) * 0.5;
        let sw = (dim / 24.0 * 1.85).clamp(1.25, 2.4);

        let stroke = Stroke::default()
            .with_color(self.color)
            .with_width(sw)
            .with_line_cap(LineCap::Round)
            .with_line_join(LineJoin::Round);

        let p = |x: f32, y: f32| -> Point {
            Point::new(offset_x + (x / 24.0) * dim, offset_y + (y / 24.0) * dim)
        };
        let d = |v: f32| -> f32 { (v / 24.0) * dim };

        match self.kind {
            Icon::ApertureLogo => {
                let center = p(12.0, 12.0);
                frame.stroke(&Path::circle(center, d(9.0)), stroke);
                for i in 0..6 {
                    let a1 = (i as f32) * std::f32::consts::PI / 3.0;
                    let a2 = a1 + std::f32::consts::PI / 3.0;
                    let p_outer = Point::new(center.x + d(9.0) * a1.cos(), center.y + d(9.0) * a1.sin());
                    let p_inner = Point::new(center.x + d(3.6) * a2.cos(), center.y + d(3.6) * a2.sin());
                    frame.stroke(&Path::line(p_outer, p_inner), stroke);
                }
            }
            Icon::FolderOpen => {
                let back = Path::new(|b| {
                    b.move_to(p(3.0, 16.0));
                    b.line_to(p(3.0, 6.0));
                    b.line_to(p(8.5, 6.0));
                    b.line_to(p(10.5, 8.5));
                    b.line_to(p(21.0, 8.5));
                    b.line_to(p(21.0, 13.0));
                });
                frame.stroke(&back, stroke);

                let flap = Path::new(|b| {
                    b.move_to(p(2.5, 19.0));
                    b.line_to(p(5.0, 11.5));
                    b.line_to(p(21.5, 11.5));
                    b.line_to(p(19.0, 19.0));
                    b.close();
                });
                frame.stroke(&flap, stroke);
            }
            Icon::Sparkles => {
                let star1 = Path::new(|b| {
                    let top = p(9.5, 3.5);
                    let right = p(17.5, 11.5);
                    let bottom = p(9.5, 19.5);
                    let left = p(1.5, 11.5);
                    let c = p(9.5, 11.5);

                    b.move_to(top);
                    b.quadratic_curve_to(Point::new(c.x + d(1.2), c.y - d(1.2)), right);
                    b.quadratic_curve_to(Point::new(c.x + d(1.2), c.y + d(1.2)), bottom);
                    b.quadratic_curve_to(Point::new(c.x - d(1.2), c.y + d(1.2)), left);
                    b.quadratic_curve_to(Point::new(c.x - d(1.2), c.y - d(1.2)), top);
                    b.close();
                });
                frame.stroke(&star1, stroke);

                let star2 = Path::new(|b| {
                    let top = p(18.5, 2.5);
                    let right = p(21.5, 5.5);
                    let bottom = p(18.5, 8.5);
                    let left = p(15.5, 5.5);
                    let c = p(18.5, 5.5);

                    b.move_to(top);
                    b.quadratic_curve_to(Point::new(c.x + d(0.5), c.y - d(0.5)), right);
                    b.quadratic_curve_to(Point::new(c.x + d(0.5), c.y + d(0.5)), bottom);
                    b.quadratic_curve_to(Point::new(c.x - d(0.5), c.y + d(0.5)), left);
                    b.quadratic_curve_to(Point::new(c.x - d(0.5), c.y - d(0.5)), top);
                    b.close();
                });
                frame.stroke(&star2, stroke);
            }
            Icon::Sliders => {
                // Track 1
                frame.stroke(&Path::line(p(3.0, 6.0), p(5.8, 6.0)), stroke);
                frame.stroke(&Path::line(p(10.2, 6.0), p(21.0, 6.0)), stroke);
                frame.stroke(&Path::circle(p(8.0, 6.0), d(2.2)), stroke);

                // Track 2
                frame.stroke(&Path::line(p(3.0, 12.0), p(13.8, 12.0)), stroke);
                frame.stroke(&Path::line(p(18.2, 12.0), p(21.0, 12.0)), stroke);
                frame.stroke(&Path::circle(p(16.0, 12.0), d(2.2)), stroke);

                // Track 3
                frame.stroke(&Path::line(p(3.0, 18.0), p(8.8, 18.0)), stroke);
                frame.stroke(&Path::line(p(13.2, 18.0), p(21.0, 18.0)), stroke);
                frame.stroke(&Path::circle(p(11.0, 18.0), d(2.2)), stroke);
            }
            Icon::Details => {
                let gem = Path::new(|b| {
                    b.move_to(p(6.0, 4.0));
                    b.line_to(p(18.0, 4.0));
                    b.line_to(p(21.5, 9.5));
                    b.line_to(p(12.0, 20.5));
                    b.line_to(p(2.5, 9.5));
                    b.close();
                });
                frame.stroke(&gem, stroke);

                frame.stroke(&Path::line(p(2.5, 9.5), p(21.5, 9.5)), stroke);

                let facets = Path::new(|b| {
                    b.move_to(p(6.0, 4.0));
                    b.line_to(p(9.0, 9.5));
                    b.line_to(p(12.0, 20.5));

                    b.move_to(p(18.0, 4.0));
                    b.line_to(p(15.0, 9.5));
                    b.line_to(p(12.0, 20.5));

                    b.move_to(p(9.0, 9.5));
                    b.line_to(p(12.0, 4.0));
                    b.line_to(p(15.0, 9.5));
                });
                frame.stroke(&facets, stroke);
            }
            Icon::SplineCurve => {
                let curve = Path::new(|b| {
                    b.move_to(p(4.0, 20.0));
                    b.bezier_curve_to(p(11.0, 20.0), p(13.0, 4.0), p(20.0, 4.0));
                });
                frame.stroke(&curve, stroke);

                frame.stroke(&Path::circle(p(4.0, 20.0), d(2.0)), stroke);
                frame.fill(&Path::circle(p(4.0, 20.0), d(1.2)), self.color);

                frame.stroke(&Path::circle(p(20.0, 4.0), d(2.0)), stroke);
                frame.fill(&Path::circle(p(20.0, 4.0), d(1.2)), self.color);

                frame.fill(&Path::circle(p(12.0, 12.0), d(1.5)), self.color);
            }
            Icon::Droplet => {
                let droplet = Path::new(|b| {
                    b.move_to(p(12.0, 3.0));
                    b.bezier_curve_to(p(15.5, 7.5), p(19.0, 12.5), p(19.0, 15.5));
                    b.bezier_curve_to(p(19.0, 19.3), p(15.8, 21.5), p(12.0, 21.5));
                    b.bezier_curve_to(p(8.2, 21.5), p(5.0, 19.3), p(5.0, 15.5));
                    b.bezier_curve_to(p(5.0, 12.5), p(8.5, 7.5), p(12.0, 3.0));
                    b.close();
                });
                frame.stroke(&droplet, stroke);

                let sheen = Path::new(|b| {
                    b.move_to(p(9.0, 13.0));
                    b.bezier_curve_to(p(8.0, 15.0), p(8.5, 17.5), p(10.5, 19.0));
                });
                frame.stroke(
                    &sheen,
                    Stroke::default()
                        .with_color(self.color)
                        .with_width(sw * 0.75)
                        .with_line_cap(LineCap::Round),
                );
            }
            Icon::ColorWheel => {
                let center = p(12.0, 12.0);
                let r_out = d(9.0);
                let r_in = d(3.8);
                frame.stroke(&Path::circle(center, r_out), stroke);
                frame.stroke(&Path::circle(center, r_in), stroke);

                for i in 0..6 {
                    let angle = (i as f32) * std::f32::consts::PI / 3.0;
                    let p1 = Point::new(center.x + r_in * angle.cos(), center.y + r_in * angle.sin());
                    let p2 = Point::new(center.x + r_out * angle.cos(), center.y + r_out * angle.sin());
                    frame.stroke(&Path::line(p1, p2), stroke);
                }
            }
            Icon::Cube3D => {
                let center = p(12.0, 12.0);
                let r = d(9.0);

                let mut vertices = [Point::ORIGIN; 6];
                for i in 0..6 {
                    let angle = -std::f32::consts::FRAC_PI_2 + (i as f32) * (std::f32::consts::PI / 3.0);
                    vertices[i] = Point::new(center.x + r * angle.cos(), center.y + r * angle.sin());
                }

                let hex = Path::new(|b| {
                    b.move_to(vertices[0]);
                    for v in &vertices[1..] {
                        b.line_to(*v);
                    }
                    b.close();
                });
                frame.stroke(&hex, stroke);

                frame.stroke(&Path::line(center, vertices[0]), stroke);
                frame.stroke(&Path::line(center, vertices[2]), stroke);
                frame.stroke(&Path::line(center, vertices[4]), stroke);
            }
            Icon::ContrastBw => {
                let center = p(12.0, 12.0);
                let r = d(8.5);

                let half = Path::new(|b| {
                    b.move_to(p(12.0, 3.5));
                    b.bezier_curve_to(p(7.3, 3.5), p(3.5, 7.3), p(3.5, 12.0));
                    b.bezier_curve_to(p(3.5, 16.7), p(7.3, 20.5), p(12.0, 20.5));
                    b.line_to(p(12.0, 3.5));
                    b.close();
                });
                frame.fill(&half, self.color);

                frame.stroke(&Path::circle(center, r), stroke);
                frame.stroke(&Path::line(p(12.0, 3.5), p(12.0, 20.5)), stroke);
            }
            Icon::Vignette => {
                let outer = Path::new(|b| {
                    b.move_to(p(5.0, 3.5));
                    b.line_to(p(19.0, 3.5));
                    b.bezier_curve_to(p(20.5, 3.5), p(20.5, 5.0), p(20.5, 5.0));
                    b.line_to(p(20.5, 19.0));
                    b.bezier_curve_to(p(20.5, 20.5), p(19.0, 20.5), p(19.0, 20.5));
                    b.line_to(p(5.0, 20.5));
                    b.bezier_curve_to(p(3.5, 20.5), p(3.5, 19.0), p(3.5, 19.0));
                    b.line_to(p(3.5, 5.0));
                    b.bezier_curve_to(p(3.5, 3.5), p(5.0, 3.5), p(5.0, 3.5));
                    b.close();
                });
                frame.stroke(&outer, stroke);

                let oval = Path::new(|b| {
                    b.move_to(p(12.0, 7.0));
                    b.bezier_curve_to(p(15.5, 7.0), p(18.0, 9.2), p(18.0, 12.0));
                    b.bezier_curve_to(p(18.0, 14.8), p(15.5, 17.0), p(12.0, 17.0));
                    b.bezier_curve_to(p(8.5, 17.0), p(6.0, 14.8), p(6.0, 12.0));
                    b.bezier_curve_to(p(6.0, 9.2), p(8.5, 7.0), p(12.0, 7.0));
                    b.close();
                });
                frame.stroke(&oval, stroke);

                frame.fill(&Path::circle(p(12.0, 12.0), d(1.5)), self.color);
            }
            Icon::Crop => {
                let l1 = Path::new(|b| {
                    b.move_to(p(6.0, 2.0));
                    b.line_to(p(6.0, 18.0));
                    b.line_to(p(22.0, 18.0));
                });
                frame.stroke(&l1, stroke);

                let l2 = Path::new(|b| {
                    b.move_to(p(2.0, 6.0));
                    b.line_to(p(18.0, 6.0));
                    b.line_to(p(18.0, 22.0));
                });
                frame.stroke(&l2, stroke);
            }
            Icon::SunRaw => {
                let center = p(12.0, 12.0);
                frame.stroke(&Path::circle(center, d(4.5)), stroke);

                let r_inner = d(6.8);
                let r_outer = d(9.8);
                for i in 0..8 {
                    let angle = (i as f32) * (std::f32::consts::PI / 4.0);
                    let p1 = Point::new(center.x + r_inner * angle.cos(), center.y + r_inner * angle.sin());
                    let p2 = Point::new(center.x + r_outer * angle.cos(), center.y + r_outer * angle.sin());
                    frame.stroke(&Path::line(p1, p2), stroke);
                }
            }
            Icon::Undo => {
                let head = Path::new(|b| {
                    b.move_to(p(8.0, 5.0));
                    b.line_to(p(4.0, 9.0));
                    b.line_to(p(8.0, 13.0));
                });
                frame.stroke(&head, stroke);

                let arc = Path::new(|b| {
                    b.move_to(p(5.0, 9.0));
                    b.bezier_curve_to(p(11.0, 6.0), p(20.0, 8.5), p(20.0, 15.0));
                    b.line_to(p(20.0, 19.0));
                });
                frame.stroke(&arc, stroke);
            }
            Icon::Redo => {
                let head = Path::new(|b| {
                    b.move_to(p(16.0, 5.0));
                    b.line_to(p(20.0, 9.0));
                    b.line_to(p(16.0, 13.0));
                });
                frame.stroke(&head, stroke);

                let arc = Path::new(|b| {
                    b.move_to(p(19.0, 9.0));
                    b.bezier_curve_to(p(13.0, 6.0), p(4.0, 8.5), p(4.0, 15.0));
                    b.line_to(p(4.0, 19.0));
                });
                frame.stroke(&arc, stroke);
            }
            Icon::Eye => {
                let almond = Path::new(|b| {
                    b.move_to(p(2.5, 12.0));
                    b.quadratic_curve_to(p(12.0, 4.5), p(21.5, 12.0));
                    b.quadratic_curve_to(p(12.0, 19.5), p(2.5, 12.0));
                    b.close();
                });
                frame.stroke(&almond, stroke);
                frame.stroke(&Path::circle(p(12.0, 12.0), d(3.2)), stroke);
                frame.fill(&Path::circle(p(12.0, 12.0), d(1.5)), self.color);
            }
            Icon::EyeOff => {
                frame.stroke(&Path::line(p(3.5, 3.5), p(20.5, 20.5)), stroke);

                let left = Path::new(|b| {
                    b.move_to(p(2.5, 12.0));
                    b.quadratic_curve_to(p(6.5, 8.0), p(9.5, 7.0));
                    b.move_to(p(2.5, 12.0));
                    b.quadratic_curve_to(p(6.5, 16.0), p(9.5, 17.0));
                });
                frame.stroke(&left, stroke);

                let right = Path::new(|b| {
                    b.move_to(p(14.5, 7.0));
                    b.quadratic_curve_to(p(17.5, 8.0), p(21.5, 12.0));
                    b.move_to(p(14.5, 17.0));
                    b.quadratic_curve_to(p(17.5, 16.0), p(21.5, 12.0));
                });
                frame.stroke(&right, stroke);

                let pupil = Path::new(|b| {
                    b.move_to(p(10.5, 9.5));
                    b.bezier_curve_to(p(12.0, 8.8), p(13.8, 9.2), p(14.8, 10.5));
                    b.move_to(p(9.2, 13.5));
                    b.bezier_curve_to(p(10.2, 14.8), p(12.0, 15.2), p(13.5, 14.5));
                });
                frame.stroke(&pupil, stroke);
            }
            Icon::Layers => {
                let top_diamond = Path::new(|b| {
                    b.move_to(p(12.0, 2.5));
                    b.line_to(p(21.5, 7.5));
                    b.line_to(p(12.0, 12.5));
                    b.line_to(p(2.5, 7.5));
                    b.close();
                });
                frame.stroke(&top_diamond, stroke);

                let mid_chevron = Path::new(|b| {
                    b.move_to(p(2.5, 12.0));
                    b.line_to(p(12.0, 17.0));
                    b.line_to(p(21.5, 12.0));
                });
                frame.stroke(&mid_chevron, stroke);

                let bot_chevron = Path::new(|b| {
                    b.move_to(p(2.5, 16.5));
                    b.line_to(p(12.0, 21.5));
                    b.line_to(p(21.5, 16.5));
                });
                frame.stroke(&bot_chevron, stroke);
            }
            Icon::Download => {
                frame.stroke(&Path::line(p(12.0, 3.5), p(12.0, 14.5)), stroke);
                let head = Path::new(|b| {
                    b.move_to(p(7.0, 10.0));
                    b.line_to(p(12.0, 15.0));
                    b.line_to(p(17.0, 10.0));
                });
                frame.stroke(&head, stroke);

                let tray = Path::new(|b| {
                    b.move_to(p(4.0, 14.0));
                    b.line_to(p(4.0, 20.0));
                    b.line_to(p(20.0, 20.0));
                    b.line_to(p(20.0, 14.0));
                });
                frame.stroke(&tray, stroke);
            }
            Icon::Save => {
                let disk = Path::new(|b| {
                    b.move_to(p(4.0, 20.0));
                    b.line_to(p(4.0, 4.0));
                    b.line_to(p(16.5, 4.0));
                    b.line_to(p(20.0, 7.5));
                    b.line_to(p(20.0, 20.0));
                    b.close();
                });
                frame.stroke(&disk, stroke);

                let top_notch = Path::new(|b| {
                    b.move_to(p(8.0, 4.0));
                    b.line_to(p(8.0, 9.0));
                    b.line_to(p(15.0, 9.0));
                    b.line_to(p(15.0, 4.0));
                });
                frame.stroke(&top_notch, stroke);

                let bottom_window = Path::new(|b| {
                    b.move_to(p(7.0, 20.0));
                    b.line_to(p(7.0, 13.5));
                    b.line_to(p(17.0, 13.5));
                    b.line_to(p(17.0, 20.0));
                });
                frame.stroke(&bottom_window, stroke);
            }
            Icon::Copy => {
                let front = Path::new(|b| {
                    b.move_to(p(9.5, 8.0));
                    b.line_to(p(18.5, 8.0));
                    b.line_to(p(20.0, 9.5));
                    b.line_to(p(20.0, 18.5));
                    b.line_to(p(18.5, 20.0));
                    b.line_to(p(9.5, 20.0));
                    b.line_to(p(8.0, 18.5));
                    b.line_to(p(8.0, 9.5));
                    b.close();
                });
                frame.stroke(&front, stroke);

                let back = Path::new(|b| {
                    b.move_to(p(5.0, 16.0));
                    b.line_to(p(4.0, 15.0));
                    b.line_to(p(4.0, 5.5));
                    b.line_to(p(5.5, 4.0));
                    b.line_to(p(15.0, 4.0));
                    b.line_to(p(16.0, 5.0));
                });
                frame.stroke(&back, stroke);
            }
            Icon::Paste => {
                let clip = Path::new(|b| {
                    b.move_to(p(9.0, 4.0));
                    b.line_to(p(9.0, 2.5));
                    b.line_to(p(15.0, 2.5));
                    b.line_to(p(15.0, 4.0));
                    b.close();
                });
                frame.stroke(&clip, stroke);

                let board = Path::new(|b| {
                    b.move_to(p(9.0, 5.0));
                    b.line_to(p(5.0, 5.0));
                    b.line_to(p(5.0, 21.0));
                    b.line_to(p(19.0, 21.0));
                    b.line_to(p(19.0, 5.0));
                    b.line_to(p(15.0, 5.0));
                });
                frame.stroke(&board, stroke);

                frame.stroke(&Path::line(p(8.0, 11.0), p(16.0, 11.0)), stroke);
                frame.stroke(&Path::line(p(8.0, 15.0), p(13.5, 15.0)), stroke);
            }
            Icon::Star => {
                let star = star_path(p(12.0, 12.5), d(8.5), d(3.8));
                frame.stroke(&star, stroke);
            }
            Icon::StarFilled => {
                let star = star_path(p(12.0, 12.5), d(8.5), d(3.8));
                frame.fill(&star, self.color);
                frame.stroke(&star, stroke);
            }
            Icon::Check => {
                let check = Path::new(|b| {
                    b.move_to(p(4.5, 12.5));
                    b.line_to(p(9.5, 17.5));
                    b.line_to(p(19.5, 6.5));
                });
                frame.stroke(&check, stroke);
            }
            Icon::Close => {
                frame.stroke(&Path::line(p(5.5, 5.5), p(18.5, 18.5)), stroke);
                frame.stroke(&Path::line(p(18.5, 5.5), p(5.5, 18.5)), stroke);
            }
            Icon::ChevronLeft => {
                let chevron = Path::new(|b| {
                    b.move_to(p(15.0, 5.0));
                    b.line_to(p(8.0, 12.0));
                    b.line_to(p(15.0, 19.0));
                });
                frame.stroke(&chevron, stroke);
            }
            Icon::ChevronUp => {
                let chevron = Path::new(|b| {
                    b.move_to(p(5.0, 15.0));
                    b.line_to(p(12.0, 8.0));
                    b.line_to(p(19.0, 15.0));
                });
                frame.stroke(&chevron, stroke);
            }
            Icon::ChevronDown => {
                let chevron = Path::new(|b| {
                    b.move_to(p(5.0, 9.0));
                    b.line_to(p(12.0, 16.0));
                    b.line_to(p(19.0, 9.0));
                });
                frame.stroke(&chevron, stroke);
            }
            Icon::Pipette => {
                let body = Path::new(|b| {
                    b.move_to(p(3.0, 21.0));
                    b.line_to(p(5.5, 16.5));
                    b.line_to(p(13.0, 9.0));
                    b.bezier_curve_to(p(14.0, 6.0), p(18.0, 3.0), p(20.0, 5.0));
                    b.bezier_curve_to(p(22.0, 7.0), p(19.0, 11.0), p(16.0, 12.0));
                    b.line_to(p(8.5, 19.5));
                    b.close();
                });
                frame.stroke(&body, stroke);

                frame.stroke(&Path::line(p(11.5, 10.5), p(14.5, 13.5)), stroke);

                frame.stroke(
                    &Path::line(p(7.0, 17.0), p(9.0, 15.0)),
                    Stroke::default()
                        .with_color(self.color)
                        .with_width(sw * 0.8)
                        .with_line_cap(LineCap::Round),
                );
            }
            Icon::Crosshair => {
                let center = p(12.0, 12.0);
                frame.stroke(&Path::circle(center, d(6.5)), stroke);

                frame.stroke(&Path::line(p(12.0, 2.0), p(12.0, 6.5)), stroke);
                frame.stroke(&Path::line(p(12.0, 17.5), p(12.0, 22.0)), stroke);
                frame.stroke(&Path::line(p(2.0, 12.0), p(6.5, 12.0)), stroke);
                frame.stroke(&Path::line(p(17.5, 12.0), p(22.0, 12.0)), stroke);

                frame.fill(&Path::circle(center, d(1.2)), self.color);
            }
            Icon::RotateCcw => {
                let head = Path::new(|b| {
                    b.move_to(p(5.0, 10.0));
                    b.line_to(p(5.0, 5.0));
                    b.line_to(p(10.0, 5.0));
                });
                frame.stroke(&head, stroke);

                let arc = Path::new(|b| {
                    b.move_to(p(6.0, 6.0));
                    b.bezier_curve_to(p(9.5, 4.0), p(14.5, 4.0), p(17.5, 7.0));
                    b.bezier_curve_to(p(20.5, 10.0), p(20.5, 15.5), p(17.5, 18.5));
                    b.bezier_curve_to(p(14.5, 21.5), p(9.5, 21.5), p(6.0, 18.0));
                    b.bezier_curve_to(p(4.5, 16.5), p(4.0, 14.5), p(4.5, 13.0));
                });
                frame.stroke(&arc, stroke);
            }
            Icon::RotateCw => {
                let head = Path::new(|b| {
                    b.move_to(p(14.0, 5.0));
                    b.line_to(p(19.0, 5.0));
                    b.line_to(p(19.0, 10.0));
                });
                frame.stroke(&head, stroke);

                let arc = Path::new(|b| {
                    b.move_to(p(18.0, 6.0));
                    b.bezier_curve_to(p(14.5, 4.0), p(9.5, 4.0), p(6.5, 7.0));
                    b.bezier_curve_to(p(3.5, 10.0), p(3.5, 15.5), p(6.5, 18.5));
                    b.bezier_curve_to(p(9.5, 21.5), p(14.5, 21.5), p(18.0, 18.0));
                    b.bezier_curve_to(p(19.5, 16.5), p(20.0, 14.5), p(19.5, 13.0));
                });
                frame.stroke(&arc, stroke);
            }
            Icon::FlipH => {
                frame.stroke(&Path::line(p(12.0, 3.0), p(12.0, 21.0)), stroke);

                let left = Path::new(|b| {
                    b.move_to(p(3.5, 12.0));
                    b.line_to(p(9.5, 6.0));
                    b.line_to(p(9.5, 18.0));
                    b.close();
                });
                frame.stroke(&left, stroke);

                let right = Path::new(|b| {
                    b.move_to(p(20.5, 12.0));
                    b.line_to(p(14.5, 6.0));
                    b.line_to(p(14.5, 18.0));
                    b.close();
                });
                frame.stroke(&right, stroke);
            }
            Icon::FlipV => {
                frame.stroke(&Path::line(p(3.0, 12.0), p(21.0, 12.0)), stroke);

                let top = Path::new(|b| {
                    b.move_to(p(12.0, 3.5));
                    b.line_to(p(6.0, 9.5));
                    b.line_to(p(18.0, 9.5));
                    b.close();
                });
                frame.stroke(&top, stroke);

                let bottom = Path::new(|b| {
                    b.move_to(p(12.0, 20.5));
                    b.line_to(p(6.0, 14.5));
                    b.line_to(p(18.0, 14.5));
                    b.close();
                });
                frame.stroke(&bottom, stroke);
            }
            Icon::Trash => {
                frame.stroke(&Path::line(p(9.0, 3.0), p(15.0, 3.0)), stroke);
                frame.stroke(&Path::line(p(4.0, 6.0), p(20.0, 6.0)), stroke);

                let bin = Path::new(|b| {
                    b.move_to(p(5.5, 6.0));
                    b.line_to(p(6.8, 19.5));
                    b.line_to(p(17.2, 19.5));
                    b.line_to(p(18.5, 6.0));
                });
                frame.stroke(&bin, stroke);

                frame.stroke(&Path::line(p(10.0, 9.5), p(10.0, 16.0)), stroke);
                frame.stroke(&Path::line(p(14.0, 9.5), p(14.0, 16.0)), stroke);
            }
            Icon::SidebarRight => {
                let frame_path = Path::new(|b| {
                    b.move_to(p(5.0, 3.5));
                    b.line_to(p(19.0, 3.5));
                    b.line_to(p(20.5, 5.0));
                    b.line_to(p(20.5, 19.0));
                    b.line_to(p(19.0, 20.5));
                    b.line_to(p(5.0, 20.5));
                    b.line_to(p(3.5, 19.0));
                    b.line_to(p(3.5, 5.0));
                    b.close();
                });
                frame.stroke(&frame_path, stroke);

                frame.stroke(&Path::line(p(15.5, 3.5), p(15.5, 20.5)), stroke);

                frame.stroke(&Path::line(p(17.5, 7.5), p(19.0, 7.5)), stroke);
                frame.stroke(&Path::line(p(17.5, 11.0), p(19.0, 11.0)), stroke);
            }
            Icon::ZoomIn => {
                let center = p(10.5, 10.5);
                frame.stroke(&Path::circle(center, d(6.0)), stroke);
                frame.stroke(&Path::line(p(15.0, 15.0), p(21.0, 21.0)), stroke);
                frame.stroke(&Path::line(p(7.5, 10.5), p(13.5, 10.5)), stroke);
                frame.stroke(&Path::line(p(10.5, 7.5), p(10.5, 13.5)), stroke);
            }
            Icon::ZoomOut => {
                let center = p(10.5, 10.5);
                frame.stroke(&Path::circle(center, d(6.0)), stroke);
                frame.stroke(&Path::line(p(15.0, 15.0), p(21.0, 21.0)), stroke);
                frame.stroke(&Path::line(p(7.5, 10.5), p(13.5, 10.5)), stroke);
            }
            Icon::Maximize => {
                let tl = Path::new(|b| {
                    b.move_to(p(4.0, 9.0));
                    b.line_to(p(4.0, 4.0));
                    b.line_to(p(9.0, 4.0));
                });
                frame.stroke(&tl, stroke);

                let tr = Path::new(|b| {
                    b.move_to(p(15.0, 4.0));
                    b.line_to(p(20.0, 4.0));
                    b.line_to(p(20.0, 9.0));
                });
                frame.stroke(&tr, stroke);

                let br = Path::new(|b| {
                    b.move_to(p(20.0, 15.0));
                    b.line_to(p(20.0, 20.0));
                    b.line_to(p(15.0, 20.0));
                });
                frame.stroke(&br, stroke);

                let bl = Path::new(|b| {
                    b.move_to(p(9.0, 20.0));
                    b.line_to(p(4.0, 20.0));
                    b.line_to(p(4.0, 15.0));
                });
                frame.stroke(&bl, stroke);
            }
            Icon::ImageFile => {
                let frame_path = Path::new(|b| {
                    b.move_to(p(5.0, 4.0));
                    b.line_to(p(19.0, 4.0));
                    b.line_to(p(20.5, 5.5));
                    b.line_to(p(20.5, 18.5));
                    b.line_to(p(19.0, 20.0));
                    b.line_to(p(5.0, 20.0));
                    b.line_to(p(3.5, 18.5));
                    b.line_to(p(3.5, 5.5));
                    b.close();
                });
                frame.stroke(&frame_path, stroke);

                frame.stroke(&Path::circle(p(8.5, 8.5), d(1.8)), stroke);

                let mountains = Path::new(|b| {
                    b.move_to(p(3.5, 17.0));
                    b.line_to(p(9.0, 11.5));
                    b.line_to(p(13.0, 15.5));
                    b.line_to(p(16.5, 12.0));
                    b.line_to(p(20.5, 16.5));
                });
                frame.stroke(&mountains, stroke);
            }
            Icon::Histogram => {
                frame.stroke(&Path::line(p(3.0, 20.0), p(21.0, 20.0)), stroke);

                frame.stroke(&Path::line(p(5.5, 15.0), p(5.5, 20.0)), stroke);
                frame.stroke(&Path::line(p(9.5, 6.5), p(9.5, 20.0)), stroke);
                frame.stroke(&Path::line(p(13.5, 10.5), p(13.5, 20.0)), stroke);
                frame.stroke(&Path::line(p(17.5, 14.0), p(17.5, 20.0)), stroke);

                let envelope = Path::new(|b| {
                    b.move_to(p(3.0, 18.0));
                    b.bezier_curve_to(p(6.0, 14.0), p(8.0, 5.5), p(10.0, 5.5));
                    b.bezier_curve_to(p(12.0, 5.5), p(12.5, 9.5), p(14.0, 9.5));
                    b.bezier_curve_to(p(16.0, 9.5), p(16.5, 13.5), p(19.0, 13.5));
                    b.bezier_curve_to(p(20.0, 13.5), p(20.5, 16.0), p(21.0, 17.5));
                });
                frame.stroke(
                    &envelope,
                    Stroke::default()
                        .with_color(self.color)
                        .with_width(sw * 0.75)
                        .with_line_cap(LineCap::Round),
                );
            }
            Icon::ColorGrade => {
                let r = d(3.8);
                let c_top = p(12.0, 7.8);
                let c_left = p(7.8, 15.2);
                let c_right = p(16.2, 15.2);

                // Three interlocking color grading wheels
                frame.stroke(&Path::circle(c_top, r), stroke);
                frame.stroke(&Path::circle(c_left, r), stroke);
                frame.stroke(&Path::circle(c_right, r), stroke);

                // Reticle crosshairs inside each
                let fine_stroke = Stroke::default()
                    .with_color(self.color)
                    .with_width(sw * 0.7)
                    .with_line_cap(LineCap::Round);

                frame.stroke(&Path::line(p(12.0, 5.2), p(12.0, 10.4)), fine_stroke);
                frame.stroke(&Path::line(p(9.4, 7.8), p(14.6, 7.8)), fine_stroke);

                frame.stroke(&Path::line(p(7.8, 12.6), p(7.8, 17.8)), fine_stroke);
                frame.stroke(&Path::line(p(5.2, 15.2), p(10.4, 15.2)), fine_stroke);

                frame.stroke(&Path::line(p(16.2, 12.6), p(16.2, 17.8)), fine_stroke);
                frame.stroke(&Path::line(p(13.6, 15.2), p(18.8, 15.2)), fine_stroke);

                // Balance puck indicator dots
                frame.fill(&Path::circle(p(12.8, 7.2), d(1.0)), self.color);
                frame.fill(&Path::circle(p(7.2, 15.8), d(1.0)), self.color);
                frame.fill(&Path::circle(p(16.8, 14.6), d(1.0)), self.color);
            }
            Icon::Dehaze => {
                // Sun arc emerging above horizon
                let sun = Path::new(|b| {
                    b.move_to(p(7.5, 11.0));
                    b.bezier_curve_to(p(7.5, 6.5), p(16.5, 6.5), p(16.5, 11.0));
                });
                frame.stroke(&sun, stroke);

                // Sun rays piercing haze
                frame.stroke(&Path::line(p(12.0, 5.0), p(12.0, 2.5)), stroke);
                frame.stroke(&Path::line(p(7.5, 7.0), p(5.5, 5.0)), stroke);
                frame.stroke(&Path::line(p(16.5, 7.0), p(18.5, 5.0)), stroke);

                // Atmospheric haze layers
                frame.stroke(&Path::line(p(3.0, 11.0), p(21.0, 11.0)), stroke);

                let haze1 = Path::new(|b| {
                    b.move_to(p(4.0, 14.5));
                    b.bezier_curve_to(p(8.0, 13.5), p(10.0, 15.5), p(14.0, 14.5));
                    b.bezier_curve_to(p(17.0, 13.8), p(18.5, 15.2), p(20.0, 14.5));
                });
                frame.stroke(&haze1, stroke);

                let haze2 = Path::new(|b| {
                    b.move_to(p(5.5, 18.0));
                    b.bezier_curve_to(p(9.0, 17.0), p(11.5, 19.0), p(15.0, 18.0));
                    b.line_to(p(18.5, 18.0));
                });
                frame.stroke(&haze2, stroke);

                frame.stroke(&Path::line(p(8.0, 21.0), p(16.0, 21.0)), stroke);
            }
            Icon::TonalContrast => {
                // 3 stacked tonal step bars
                let bar1 = Path::new(|b| {
                    b.move_to(p(4.0, 3.5));
                    b.line_to(p(20.0, 3.5));
                    b.line_to(p(20.0, 7.5));
                    b.line_to(p(4.0, 7.5));
                    b.close();
                });
                frame.stroke(&bar1, stroke);

                let bar2 = Path::new(|b| {
                    b.move_to(p(4.0, 10.0));
                    b.line_to(p(20.0, 10.0));
                    b.line_to(p(20.0, 14.0));
                    b.line_to(p(4.0, 14.0));
                    b.close();
                });
                frame.stroke(&bar2, stroke);

                let bar3 = Path::new(|b| {
                    b.move_to(p(4.0, 16.5));
                    b.line_to(p(20.0, 16.5));
                    b.line_to(p(20.0, 20.5));
                    b.line_to(p(4.0, 20.5));
                    b.close();
                });
                frame.stroke(&bar3, stroke);

                // Progressive stepped density fills
                let fill1 = Path::new(|b| {
                    b.move_to(p(4.0, 3.5));
                    b.line_to(p(9.5, 3.5));
                    b.line_to(p(10.7, 7.5));
                    b.line_to(p(4.0, 7.5));
                    b.close();
                });
                frame.fill(&fill1, self.color);

                let fill2 = Path::new(|b| {
                    b.move_to(p(4.0, 10.0));
                    b.line_to(p(11.4, 10.0));
                    b.line_to(p(12.6, 14.0));
                    b.line_to(p(4.0, 14.0));
                    b.close();
                });
                frame.fill(&fill2, self.color);

                let fill3 = Path::new(|b| {
                    b.move_to(p(4.0, 16.5));
                    b.line_to(p(13.4, 16.5));
                    b.line_to(p(14.6, 20.5));
                    b.line_to(p(4.0, 20.5));
                    b.close();
                });
                frame.fill(&fill3, self.color);

                // Sharp diagonal contrast cut line
                frame.stroke(&Path::line(p(9.0, 2.0), p(15.0, 22.0)), stroke);
            }
            Icon::Perspective => {
                // Converging keystone perspective trapezoid
                let keystone = Path::new(|b| {
                    b.move_to(p(7.0, 5.0));
                    b.line_to(p(17.0, 5.0));
                    b.line_to(p(20.5, 19.0));
                    b.line_to(p(3.5, 19.0));
                    b.close();
                });
                frame.stroke(&keystone, stroke);

                // Converging vertical guideline
                frame.stroke(&Path::line(p(12.0, 5.0), p(12.0, 19.0)), stroke);

                // Horizontal perspective grid rungs
                frame.stroke(&Path::line(p(5.9, 9.5), p(18.1, 9.5)), stroke);
                frame.stroke(&Path::line(p(4.8, 14.0), p(19.2, 14.0)), stroke);

                // Alignment guide extension ticks
                frame.stroke(&Path::line(p(7.0, 2.5), p(7.0, 5.0)), stroke);
                frame.stroke(&Path::line(p(17.0, 2.5), p(17.0, 5.0)), stroke);
                frame.stroke(&Path::line(p(3.5, 19.0), p(3.5, 21.5)), stroke);
                frame.stroke(&Path::line(p(20.5, 19.0), p(20.5, 21.5)), stroke);
            }
            Icon::Expand => {
                // Inner canvas boundary
                let inner = Path::new(|b| {
                    b.move_to(p(7.5, 7.5));
                    b.line_to(p(16.5, 7.5));
                    b.line_to(p(16.5, 16.5));
                    b.line_to(p(7.5, 16.5));
                    b.close();
                });
                frame.stroke(&inner, stroke);

                // 4 outward expansion corner arrows
                // Top-Left
                frame.stroke(&Path::line(p(7.5, 7.5), p(3.5, 3.5)), stroke);
                let head_tl = Path::new(|b| {
                    b.move_to(p(3.5, 7.0));
                    b.line_to(p(3.5, 3.5));
                    b.line_to(p(7.0, 3.5));
                });
                frame.stroke(&head_tl, stroke);

                // Top-Right
                frame.stroke(&Path::line(p(16.5, 7.5), p(20.5, 3.5)), stroke);
                let head_tr = Path::new(|b| {
                    b.move_to(p(17.0, 3.5));
                    b.line_to(p(20.5, 3.5));
                    b.line_to(p(20.5, 7.0));
                });
                frame.stroke(&head_tr, stroke);

                // Bottom-Right
                frame.stroke(&Path::line(p(16.5, 16.5), p(20.5, 20.5)), stroke);
                let head_br = Path::new(|b| {
                    b.move_to(p(20.5, 17.0));
                    b.line_to(p(20.5, 20.5));
                    b.line_to(p(17.0, 20.5));
                });
                frame.stroke(&head_br, stroke);

                // Bottom-Left
                frame.stroke(&Path::line(p(7.5, 16.5), p(3.5, 20.5)), stroke);
                let head_bl = Path::new(|b| {
                    b.move_to(p(3.5, 17.0));
                    b.line_to(p(3.5, 20.5));
                    b.line_to(p(7.0, 20.5));
                });
                frame.stroke(&head_bl, stroke);
            }
            Icon::Selective => {
                let center = p(12.0, 12.0);

                // U-Point central control pin
                frame.fill(&Path::circle(center, d(1.8)), self.color);
                frame.stroke(&Path::circle(center, d(4.2)), stroke);

                // Pin stem guide
                frame.stroke(&Path::line(p(12.0, 3.0), p(12.0, 7.8)), stroke);

                // Segmented outer radius circle
                let r_out = d(9.0);
                for i in 0..8 {
                    let a1 = (i as f32) * std::f32::consts::PI / 4.0 + 0.12;
                    let a2 = (i as f32 + 1.0) * std::f32::consts::PI / 4.0 - 0.12;
                    let p1 = Point::new(center.x + r_out * a1.cos(), center.y + r_out * a1.sin());
                    let p2 = Point::new(center.x + r_out * a2.cos(), center.y + r_out * a2.sin());
                    let mid_a = (a1 + a2) * 0.5;
                    let p_ctrl = Point::new(center.x + (r_out / 0.96) * mid_a.cos(), center.y + (r_out / 0.96) * mid_a.sin());
                    let arc = Path::new(|b| {
                        b.move_to(p1);
                        b.quadratic_curve_to(p_ctrl, p2);
                    });
                    frame.stroke(&arc, stroke);
                }

                // Cardinal reticle ticks
                frame.stroke(&Path::line(p(12.0, 1.5), p(12.0, 3.0)), stroke);
                frame.stroke(&Path::line(p(12.0, 21.0), p(12.0, 22.5)), stroke);
                frame.stroke(&Path::line(p(1.5, 12.0), p(3.0, 12.0)), stroke);
                frame.stroke(&Path::line(p(21.0, 12.0), p(22.5, 12.0)), stroke);
            }
            Icon::Brush => {
                // Brush handle & ferrule
                let handle = Path::new(|b| {
                    b.move_to(p(11.5, 11.0));
                    b.line_to(p(17.5, 5.0));
                    b.quadratic_curve_to(p(19.5, 3.0), p(20.5, 4.0));
                    b.quadratic_curve_to(p(21.0, 5.0), p(19.0, 7.0));
                    b.line_to(p(13.0, 12.5));
                });
                frame.stroke(&handle, stroke);

                // Ferrule collar
                frame.stroke(&Path::line(p(10.5, 12.0), p(13.5, 9.0)), stroke);

                // Bristle tip
                let bristles = Path::new(|b| {
                    b.move_to(p(10.5, 12.0));
                    b.bezier_curve_to(p(9.0, 14.5), p(6.5, 16.5), p(4.5, 18.0));
                    b.bezier_curve_to(p(6.0, 16.0), p(9.5, 12.5), p(13.5, 9.0));
                });
                frame.stroke(&bristles, stroke);
                frame.stroke(
                    &Path::line(p(5.0, 17.5), p(10.0, 12.5)),
                    Stroke::default()
                        .with_color(self.color)
                        .with_width(sw * 0.7)
                        .with_line_cap(LineCap::Round),
                );

                // Smooth paint stroke arc
                let paint_stroke = Path::new(|b| {
                    b.move_to(p(3.5, 20.5));
                    b.quadratic_curve_to(p(10.0, 22.0), p(16.5, 18.5));
                    b.quadratic_curve_to(p(20.0, 16.5), p(20.5, 14.5));
                });
                frame.stroke(&paint_stroke, stroke);
            }
            Icon::Healing => {
                // Retouch adhesive bandage (angled at 45 degrees)
                let bandage = Path::new(|b| {
                    b.move_to(p(5.5, 16.0));
                    b.line_to(p(16.0, 5.5));
                    b.quadratic_curve_to(p(18.5, 3.0), p(20.5, 5.0));
                    b.quadratic_curve_to(p(21.0, 7.0), p(18.5, 9.5));
                    b.line_to(p(8.0, 20.0));
                    b.quadratic_curve_to(p(5.5, 22.5), p(3.5, 20.5));
                    b.quadratic_curve_to(p(3.0, 18.5), p(5.5, 16.0));
                    b.close();
                });
                frame.stroke(&bandage, stroke);

                // Perforated center patch
                let pad = Path::new(|b| {
                    b.move_to(p(9.5, 12.0));
                    b.line_to(p(12.0, 9.5));
                    b.line_to(p(14.5, 12.0));
                    b.line_to(p(12.0, 14.5));
                    b.close();
                });
                frame.stroke(
                    &pad,
                    Stroke::default()
                        .with_color(self.color)
                        .with_width(sw * 0.8)
                        .with_line_cap(LineCap::Round),
                );

                // Center patch dots
                frame.fill(&Path::circle(p(12.0, 12.0), d(1.0)), self.color);
                frame.fill(&Path::circle(p(10.8, 10.8), d(0.7)), self.color);
                frame.fill(&Path::circle(p(13.2, 13.2), d(0.7)), self.color);

                // Wing perforations
                frame.fill(&Path::circle(p(7.0, 17.5), d(0.8)), self.color);
                frame.fill(&Path::circle(p(17.0, 7.5), d(0.8)), self.color);

                // Cross accent ticks
                let fine_accent = Stroke::default()
                    .with_color(self.color)
                    .with_width(sw * 0.75)
                    .with_line_cap(LineCap::Round);
                frame.stroke(&Path::line(p(5.0, 9.0), p(9.0, 5.0)), fine_accent);
                frame.stroke(&Path::line(p(15.0, 19.0), p(19.0, 15.0)), fine_accent);
            }
            Icon::Halation => {
                let center = p(12.0, 12.0);

                // Bright highlight core
                frame.fill(&Path::circle(center, d(3.2)), self.color);

                // Inner glow boundary
                frame.stroke(&Path::circle(center, d(5.8)), stroke);

                // Outer glowing halo
                frame.stroke(
                    &Path::circle(center, d(8.8)),
                    Stroke::default()
                        .with_color(self.color)
                        .with_width(sw * 0.85)
                        .with_line_cap(LineCap::Round),
                );

                // Halation optical diffusion rays
                for i in 0..8 {
                    let angle = (i as f32) * std::f32::consts::PI / 4.0;
                    let p1 = Point::new(center.x + d(9.3) * angle.cos(), center.y + d(9.3) * angle.sin());
                    let p2 = Point::new(center.x + d(11.2) * angle.cos(), center.y + d(11.2) * angle.sin());
                    frame.stroke(&Path::line(p1, p2), stroke);
                }
            }
            Icon::Bloom => {
                let center = p(12.0, 12.0);

                // Soft surrounding diffusion ring
                frame.stroke(
                    &Path::circle(center, d(8.5)),
                    Stroke::default()
                        .with_color(self.color)
                        .with_width(sw * 0.75)
                        .with_line_cap(LineCap::Round),
                );

                // 4-point optical star flare
                let flare = Path::new(|b| {
                    b.move_to(p(12.0, 3.5));
                    b.quadratic_curve_to(p(12.0, 11.0), p(20.5, 12.0));
                    b.quadratic_curve_to(p(12.0, 13.0), p(12.0, 20.5));
                    b.quadratic_curve_to(p(12.0, 13.0), p(3.5, 12.0));
                    b.quadratic_curve_to(p(12.0, 11.0), p(12.0, 3.5));
                });
                frame.stroke(&flare, stroke);

                // Bright center core
                frame.fill(&Path::circle(center, d(1.6)), self.color);

                // Diagonal micro-flares
                frame.stroke(&Path::line(p(9.5, 9.5), p(7.0, 7.0)), stroke);
                frame.stroke(&Path::line(p(14.5, 9.5), p(17.0, 7.0)), stroke);
                frame.stroke(&Path::line(p(14.5, 14.5), p(17.0, 17.0)), stroke);
                frame.stroke(&Path::line(p(9.5, 14.5), p(7.0, 17.0)), stroke);
            }
            Icon::GrainyFilm => {
                // 35mm film strip outer contour
                let strip = Path::new(|b| {
                    b.move_to(p(5.0, 3.0));
                    b.line_to(p(19.0, 3.0));
                    b.quadratic_curve_to(p(20.5, 3.0), p(20.5, 4.5));
                    b.line_to(p(20.5, 19.5));
                    b.quadratic_curve_to(p(20.5, 21.0), p(19.0, 21.0));
                    b.line_to(p(5.0, 21.0));
                    b.quadratic_curve_to(p(3.5, 21.0), p(3.5, 19.5));
                    b.line_to(p(3.5, 4.5));
                    b.quadratic_curve_to(p(3.5, 3.0), p(5.0, 3.0));
                    b.close();
                });
                frame.stroke(&strip, stroke);

                // Sprocket channel margins
                frame.stroke(&Path::line(p(7.5, 3.0), p(7.5, 21.0)), stroke);
                frame.stroke(&Path::line(p(16.5, 3.0), p(16.5, 21.0)), stroke);

                // Left sprocket holes
                for y_pos in [5.5, 11.0, 16.5] {
                    let hole = Path::new(|b| {
                        b.move_to(p(4.5, y_pos));
                        b.line_to(p(6.5, y_pos));
                        b.line_to(p(6.5, y_pos + 2.0));
                        b.line_to(p(4.5, y_pos + 2.0));
                        b.close();
                    });
                    frame.stroke(&hole, stroke);
                }

                // Right sprocket holes
                for y_pos in [5.5, 11.0, 16.5] {
                    let hole = Path::new(|b| {
                        b.move_to(p(17.5, y_pos));
                        b.line_to(p(19.5, y_pos));
                        b.line_to(p(19.5, y_pos + 2.0));
                        b.line_to(p(17.5, y_pos + 2.0));
                        b.close();
                    });
                    frame.stroke(&hole, stroke);
                }

                // Film emulsion grain dots inside the frame
                for &(gx, gy) in &[
                    (9.5, 6.5),
                    (14.0, 7.5),
                    (11.5, 10.0),
                    (14.5, 12.5),
                    (9.5, 14.5),
                    (13.0, 16.5),
                    (10.5, 18.5),
                ] {
                    frame.fill(&Path::circle(p(gx, gy), d(0.85)), self.color);
                }
            }
            Icon::Noir => {
                let center = p(12.0, 12.0);
                let r = d(8.5);

                // Dramatic chiaroscuro: solid filled shadow hemisphere
                let shadow_half = Path::new(|b| {
                    b.move_to(p(18.0, 6.0));
                    b.bezier_curve_to(p(21.5, 9.5), p(21.5, 14.5), p(18.0, 18.0));
                    b.bezier_curve_to(p(14.5, 21.5), p(9.5, 21.5), p(6.0, 18.0));
                    b.close();
                });
                frame.fill(&shadow_half, self.color);

                // Circle boundary & diagonal shadow terminator line
                frame.stroke(&Path::circle(center, r), stroke);
                frame.stroke(&Path::line(p(6.0, 18.0), p(18.0, 6.0)), stroke);

                // Dramatic venetian-blind light slats on the bright side
                frame.stroke(&Path::line(p(5.0, 11.0), p(11.0, 5.0)), stroke);
                frame.stroke(&Path::line(p(7.0, 15.0), p(15.0, 7.0)), stroke);
            }
            Icon::HdrScape => {
                // Dynamic range sun emerging behind mountain crest
                let sun_arc = Path::new(|b| {
                    b.move_to(p(6.5, 8.5));
                    b.bezier_curve_to(p(6.5, 5.5), p(12.5, 5.5), p(12.5, 8.5));
                });
                frame.stroke(&sun_arc, stroke);

                // Dynamic range radiance rays
                frame.stroke(&Path::line(p(9.5, 4.5), p(9.5, 2.0)), stroke);
                frame.stroke(&Path::line(p(6.5, 5.5), p(4.5, 3.5)), stroke);
                frame.stroke(&Path::line(p(12.5, 5.5), p(14.5, 3.5)), stroke);
                frame.stroke(&Path::line(p(15.0, 7.5), p(18.0, 6.0)), stroke);
                frame.stroke(&Path::line(p(4.0, 7.5), p(2.0, 7.0)), stroke);

                // Majestic foreground mountain peaks
                let peaks = Path::new(|b| {
                    b.move_to(p(2.5, 19.0));
                    b.line_to(p(9.5, 8.5));
                    b.line_to(p(14.5, 15.5));
                    b.line_to(p(17.5, 11.5));
                    b.line_to(p(21.5, 19.0));
                });
                frame.stroke(&peaks, stroke);

                // Mountain ridge shadow cuts
                frame.stroke(&Path::line(p(9.5, 8.5), p(9.0, 19.0)), stroke);
                frame.stroke(&Path::line(p(17.5, 11.5), p(17.0, 19.0)), stroke);

                // Horizon ground line
                frame.stroke(&Path::line(p(2.0, 19.0), p(22.0, 19.0)), stroke);
            }
            Icon::GlamourGlow => {
                let center = p(12.0, 12.0);

                // Soft circular diffusion halo
                frame.stroke(
                    &Path::circle(center, d(8.5)),
                    Stroke::default()
                        .with_color(self.color)
                        .with_width(sw * 0.75)
                        .with_line_cap(LineCap::Round),
                );

                // Glamour diamond sparkle
                let sparkle = Path::new(|b| {
                    b.move_to(p(12.0, 4.5));
                    b.quadratic_curve_to(p(12.0, 11.0), p(19.5, 12.0));
                    b.quadratic_curve_to(p(12.0, 13.0), p(12.0, 19.5));
                    b.quadratic_curve_to(p(12.0, 13.0), p(4.5, 12.0));
                    b.quadratic_curve_to(p(12.0, 11.0), p(12.0, 4.5));
                });
                frame.stroke(&sparkle, stroke);

                // Bright center glint
                frame.fill(&Path::circle(center, d(1.5)), self.color);

                // Accent glimmer starlet
                let fine_stroke = Stroke::default()
                    .with_color(self.color)
                    .with_width(sw * 0.7)
                    .with_line_cap(LineCap::Round);
                frame.stroke(&Path::line(p(17.0, 5.0), p(17.0, 9.0)), fine_stroke);
                frame.stroke(&Path::line(p(15.0, 7.0), p(19.0, 7.0)), fine_stroke);
            }
            Icon::Drama => {
                // Heavy atmospheric storm cloud
                let cloud = Path::new(|b| {
                    b.move_to(p(4.0, 13.0));
                    b.bezier_curve_to(p(3.0, 10.0), p(6.0, 7.5), p(8.5, 8.0));
                    b.bezier_curve_to(p(9.5, 5.0), p(14.5, 5.0), p(16.0, 7.5));
                    b.bezier_curve_to(p(19.0, 7.5), p(21.0, 10.0), p(20.0, 13.0));
                    b.line_to(p(4.0, 13.0));
                });
                frame.stroke(&cloud, stroke);

                // Dramatic lightning burst
                let lightning = Path::new(|b| {
                    b.move_to(p(13.0, 11.0));
                    b.line_to(p(9.5, 16.0));
                    b.line_to(p(13.5, 16.0));
                    b.line_to(p(10.5, 22.0));
                    b.line_to(p(15.5, 15.0));
                    b.line_to(p(12.0, 15.0));
                    b.close();
                });
                frame.fill(&lightning, self.color);

                // Angled dramatic contrast beams
                frame.stroke(&Path::line(p(5.5, 15.5), p(4.0, 19.5)), stroke);
                frame.stroke(&Path::line(p(18.5, 15.5), p(17.0, 19.5)), stroke);
            }
            Icon::Vintage => {
                // Retro rangefinder camera body
                let body = Path::new(|b| {
                    b.move_to(p(5.5, 7.0));
                    b.line_to(p(18.5, 7.0));
                    b.quadratic_curve_to(p(20.0, 7.0), p(20.0, 8.5));
                    b.line_to(p(20.0, 18.0));
                    b.quadratic_curve_to(p(20.0, 19.5), p(18.5, 19.5));
                    b.line_to(p(5.5, 19.5));
                    b.quadratic_curve_to(p(4.0, 19.5), p(4.0, 18.0));
                    b.line_to(p(4.0, 8.5));
                    b.quadratic_curve_to(p(4.0, 7.0), p(5.5, 7.0));
                    b.close();
                });
                frame.stroke(&body, stroke);

                // Top shutter button & dial controls
                frame.stroke(&Path::line(p(6.5, 4.5), p(6.5, 7.0)), stroke);
                frame.stroke(&Path::line(p(16.5, 4.5), p(19.0, 4.5)), stroke);

                // Viewfinder optical window
                let vf = Path::new(|b| {
                    b.move_to(p(15.0, 8.5));
                    b.line_to(p(18.5, 8.5));
                    b.line_to(p(18.5, 11.0));
                    b.line_to(p(15.0, 11.0));
                    b.close();
                });
                frame.stroke(&vf, stroke);

                // Central vintage optic lens
                let lens_center = p(11.0, 13.5);
                frame.stroke(&Path::circle(lens_center, d(4.5)), stroke);
                frame.stroke(&Path::circle(lens_center, d(2.2)), stroke);
                frame.fill(&Path::circle(lens_center, d(1.0)), self.color);

                // Vintage body leatherette accent
                frame.stroke(&Path::line(p(4.0, 11.0), p(6.5, 11.0)), stroke);
            }
            Icon::LensBlur => {
                let focus_center = p(10.5, 11.5);

                // In-focus optical circle & aperture iris blades
                frame.stroke(&Path::circle(focus_center, d(4.2)), stroke);
                frame.fill(&Path::circle(focus_center, d(1.4)), self.color);

                frame.stroke(&Path::line(p(6.8, 10.0), p(12.5, 7.6)), stroke);
                frame.stroke(&Path::line(p(12.5, 7.6), p(14.2, 13.5)), stroke);
                frame.stroke(&Path::line(p(14.2, 13.5), p(6.8, 10.0)), stroke);

                // Defocused/blurred outer ring
                frame.stroke(
                    &Path::circle(focus_center, d(7.8)),
                    Stroke::default()
                        .with_color(self.color)
                        .with_width(sw * 0.75)
                        .with_line_cap(LineCap::Round),
                );

                // Floating circular bokeh specular discs
                frame.stroke(&Path::circle(p(18.0, 6.5), d(3.0)), stroke);
                frame.fill(&Path::circle(p(18.5, 16.5), d(1.8)), self.color);
                frame.stroke(&Path::circle(p(4.5, 18.5), d(1.5)), stroke);
            }
            Icon::MotionBlur => {
                let ball = p(16.5, 12.0);

                // Speeding subject sphere
                frame.stroke(&Path::circle(ball, d(4.8)), stroke);
                frame.fill(&Path::circle(ball, d(1.6)), self.color);

                // Trailing horizontal speed blur lines
                frame.stroke(&Path::line(p(2.5, 7.5), p(13.0, 7.5)), stroke);
                frame.stroke(&Path::line(p(6.0, 9.5), p(12.0, 9.5)), stroke);
                frame.stroke(&Path::line(p(2.5, 12.0), p(11.5, 12.0)), stroke);
                frame.stroke(&Path::line(p(5.0, 14.5), p(12.0, 14.5)), stroke);
                frame.stroke(&Path::line(p(2.5, 16.5), p(13.0, 16.5)), stroke);

                // Ambient velocity streaks
                frame.stroke(&Path::line(p(11.0, 5.5), p(18.5, 5.5)), stroke);
                frame.stroke(&Path::line(p(11.0, 18.5), p(18.5, 18.5)), stroke);
            }
            Icon::DoubleExposure => {
                // Primary base photo frame (bottom-left)
                let frame1 = Path::new(|b| {
                    b.move_to(p(5.0, 6.5));
                    b.line_to(p(15.0, 6.5));
                    b.quadratic_curve_to(p(16.5, 6.5), p(16.5, 8.0));
                    b.line_to(p(16.5, 18.0));
                    b.quadratic_curve_to(p(16.5, 19.5), p(15.0, 19.5));
                    b.line_to(p(5.0, 19.5));
                    b.quadratic_curve_to(p(3.5, 19.5), p(3.5, 18.0));
                    b.line_to(p(3.5, 8.0));
                    b.quadratic_curve_to(p(3.5, 6.5), p(5.0, 6.5));
                    b.close();
                });
                frame.stroke(&frame1, stroke);

                // Mountain scene inside first exposure
                let mtn = Path::new(|b| {
                    b.move_to(p(3.5, 18.0));
                    b.line_to(p(9.0, 11.5));
                    b.line_to(p(14.5, 18.0));
                });
                frame.stroke(&mtn, stroke);

                // Secondary overlapping exposure frame (top-right)
                let frame2 = Path::new(|b| {
                    b.move_to(p(9.0, 3.5));
                    b.line_to(p(19.0, 3.5));
                    b.quadratic_curve_to(p(20.5, 3.5), p(20.5, 5.0));
                    b.line_to(p(20.5, 15.0));
                    b.quadratic_curve_to(p(20.5, 16.5), p(19.0, 16.5));
                    b.line_to(p(9.0, 16.5));
                    b.quadratic_curve_to(p(7.5, 16.5), p(7.5, 15.0));
                    b.line_to(p(7.5, 5.0));
                    b.quadratic_curve_to(p(7.5, 3.5), p(9.0, 3.5));
                    b.close();
                });
                frame.stroke(&frame2, stroke);

                // Sun disc in secondary exposure
                frame.fill(&Path::circle(p(15.5, 8.5), d(2.5)), self.color);
            }
            Icon::Retrolux => {
                // Vintage border
                let retro_frame = Path::new(|b| {
                    b.move_to(p(5.0, 3.5));
                    b.line_to(p(19.0, 3.5));
                    b.quadratic_curve_to(p(20.5, 3.5), p(20.5, 5.0));
                    b.line_to(p(20.5, 19.0));
                    b.quadratic_curve_to(p(20.5, 20.5), p(19.0, 20.5));
                    b.line_to(p(5.0, 20.5));
                    b.quadratic_curve_to(p(3.5, 20.5), p(3.5, 19.0));
                    b.line_to(p(3.5, 5.0));
                    b.quadratic_curve_to(p(3.5, 3.5), p(5.0, 3.5));
                    b.close();
                });
                frame.stroke(&retro_frame, stroke);

                // Top-left light leak sunburst
                frame.stroke(&Path::circle(p(3.5, 3.5), d(3.5)), stroke);
                frame.fill(&Path::circle(p(3.5, 3.5), d(1.8)), self.color);

                // Diagonal light leak rays
                frame.stroke(&Path::line(p(3.5, 3.5), p(13.5, 5.5)), stroke);
                frame.stroke(&Path::line(p(3.5, 3.5), p(16.0, 9.5)), stroke);
                frame.stroke(&Path::line(p(3.5, 3.5), p(15.0, 15.0)), stroke);
                frame.stroke(&Path::line(p(3.5, 3.5), p(9.5, 16.0)), stroke);
                frame.stroke(&Path::line(p(3.5, 3.5), p(5.5, 13.5)), stroke);

                // Retro dust & leak flare orbs
                frame.stroke(&Path::circle(p(16.5, 16.5), d(1.8)), stroke);
                frame.fill(&Path::circle(p(13.0, 18.0), d(0.8)), self.color);
            }
            Icon::Grunge => {
                // Distressed card outline
                let grunge_frame = Path::new(|b| {
                    b.move_to(p(5.0, 3.5));
                    b.line_to(p(19.0, 3.5));
                    b.line_to(p(20.5, 5.0));
                    b.line_to(p(20.5, 19.0));
                    b.line_to(p(19.0, 20.5));
                    b.line_to(p(5.0, 20.5));
                    b.line_to(p(3.5, 19.0));
                    b.line_to(p(3.5, 5.0));
                    b.close();
                });
                frame.stroke(&grunge_frame, stroke);

                // Harsh diagonal slash scratches
                frame.stroke(&Path::line(p(5.0, 19.0), p(19.0, 5.0)), stroke);
                frame.stroke(&Path::line(p(8.0, 20.0), p(20.0, 8.0)), stroke);
                frame.stroke(&Path::line(p(7.0, 6.0), p(17.0, 16.0)), stroke);

                // Distressed texture cuts & grit
                frame.stroke(&Path::line(p(5.5, 12.0), p(9.5, 10.0)), stroke);
                frame.stroke(&Path::line(p(14.5, 14.0), p(18.5, 12.0)), stroke);
                frame.stroke(&Path::line(p(10.0, 17.5), p(13.0, 18.5)), stroke);
            }
            Icon::Portrait => {
                let center = p(12.0, 12.0);

                // Circular portrait cameo ring
                frame.stroke(&Path::circle(center, d(9.0)), stroke);

                // Head silhouette
                frame.stroke(&Path::circle(p(12.0, 8.5), d(3.2)), stroke);

                // Shoulders & bust silhouette
                let bust = Path::new(|b| {
                    b.move_to(p(5.2, 19.0));
                    b.bezier_curve_to(p(6.0, 15.0), p(8.5, 13.5), p(12.0, 13.5));
                    b.bezier_curve_to(p(15.5, 13.5), p(18.0, 15.0), p(18.8, 19.0));
                });
                frame.stroke(&bust, stroke);

                // Collar neckline
                let collar = Path::new(|b| {
                    b.move_to(p(10.0, 13.5));
                    b.quadratic_curve_to(p(12.0, 15.5), p(14.0, 13.5));
                });
                frame.stroke(&collar, stroke);
            }
            Icon::HeadPose => {
                let center = p(12.0, 12.0);

                // 3D head wireframe sphere
                frame.stroke(&Path::circle(center, d(8.5)), stroke);

                // Vertical pitch/tilt axis arc
                let pitch = Path::new(|b| {
                    b.move_to(p(12.0, 3.5));
                    b.bezier_curve_to(p(16.5, 7.5), p(16.5, 16.5), p(12.0, 20.5));
                });
                frame.stroke(&pitch, stroke);

                // Horizontal yaw/turn axis arc
                let yaw = Path::new(|b| {
                    b.move_to(p(3.5, 12.0));
                    b.bezier_curve_to(p(8.0, 14.5), p(16.0, 14.5), p(20.5, 12.0));
                });
                frame.stroke(&yaw, stroke);

                // Eye level alignment line
                frame.stroke(
                    &Path::line(p(8.0, 9.5), p(16.0, 9.5)),
                    Stroke::default()
                        .with_color(self.color)
                        .with_width(sw * 0.7)
                        .with_line_cap(LineCap::Round),
                );

                // 3D pose vector arrow
                frame.stroke(&Path::line(p(14.5, 13.5), p(18.5, 9.5)), stroke);
                let head_arr = Path::new(|b| {
                    b.move_to(p(16.0, 9.0));
                    b.line_to(p(18.5, 9.5));
                    b.line_to(p(18.0, 12.0));
                });
                frame.stroke(&head_arr, stroke);
            }
            Icon::Frames => {
                // Outer gallery picture frame
                let outer = Path::new(|b| {
                    b.move_to(p(3.0, 3.0));
                    b.line_to(p(21.0, 3.0));
                    b.line_to(p(21.0, 21.0));
                    b.line_to(p(3.0, 21.0));
                    b.close();
                });
                frame.stroke(&outer, stroke);

                // Inner mat opening
                let inner = Path::new(|b| {
                    b.move_to(p(7.0, 7.0));
                    b.line_to(p(17.0, 7.0));
                    b.line_to(p(17.0, 17.0));
                    b.line_to(p(7.0, 17.0));
                    b.close();
                });
                frame.stroke(&inner, stroke);

                // Corner 45-degree miter joints
                frame.stroke(&Path::line(p(3.0, 3.0), p(7.0, 7.0)), stroke);
                frame.stroke(&Path::line(p(21.0, 3.0), p(17.0, 7.0)), stroke);
                frame.stroke(&Path::line(p(21.0, 21.0), p(17.0, 17.0)), stroke);
                frame.stroke(&Path::line(p(3.0, 21.0), p(7.0, 17.0)), stroke);

                // Miniature artwork inside frame
                let fine_stroke = Stroke::default()
                    .with_color(self.color)
                    .with_width(sw * 0.75)
                    .with_line_cap(LineCap::Round);
                let art = Path::new(|b| {
                    b.move_to(p(7.0, 14.5));
                    b.line_to(p(10.5, 11.0));
                    b.line_to(p(13.0, 13.5));
                    b.line_to(p(15.0, 12.0));
                    b.line_to(p(17.0, 14.0));
                });
                frame.stroke(&art, fine_stroke);
                frame.stroke(&Path::circle(p(9.5, 9.5), d(1.0)), fine_stroke);
            }
            Icon::TextOverlay => {
                // Typography layout corner brackets
                let tl = Path::new(|b| {
                    b.move_to(p(3.5, 7.5));
                    b.line_to(p(3.5, 3.5));
                    b.line_to(p(7.5, 3.5));
                });
                frame.stroke(&tl, stroke);

                let tr = Path::new(|b| {
                    b.move_to(p(16.5, 3.5));
                    b.line_to(p(20.5, 3.5));
                    b.line_to(p(20.5, 7.5));
                });
                frame.stroke(&tr, stroke);

                let br = Path::new(|b| {
                    b.move_to(p(20.5, 16.5));
                    b.line_to(p(20.5, 20.5));
                    b.line_to(p(16.5, 20.5));
                });
                frame.stroke(&br, stroke);

                let bl = Path::new(|b| {
                    b.move_to(p(7.5, 20.5));
                    b.line_to(p(3.5, 20.5));
                    b.line_to(p(3.5, 16.5));
                });
                frame.stroke(&bl, stroke);

                // Geometric serif capital 'T' (pure vector paths, zero font rendering)
                frame.stroke(&Path::line(p(7.0, 7.5), p(17.0, 7.5)), stroke);
                frame.stroke(&Path::line(p(12.0, 7.5), p(12.0, 17.5)), stroke);
                frame.stroke(&Path::line(p(7.0, 7.5), p(7.0, 9.0)), stroke);
                frame.stroke(&Path::line(p(17.0, 7.5), p(17.0, 9.0)), stroke);
                frame.stroke(&Path::line(p(9.5, 17.5), p(14.5, 17.5)), stroke);
            }
            Icon::Mask => {
                // Layer card boundary
                let card = Path::new(|b| {
                    b.move_to(p(5.0, 3.5));
                    b.line_to(p(19.0, 3.5));
                    b.quadratic_curve_to(p(20.5, 3.5), p(20.5, 5.0));
                    b.line_to(p(20.5, 19.0));
                    b.quadratic_curve_to(p(20.5, 20.5), p(19.0, 20.5));
                    b.line_to(p(5.0, 20.5));
                    b.quadratic_curve_to(p(3.5, 20.5), p(3.5, 19.0));
                    b.line_to(p(3.5, 5.0));
                    b.quadratic_curve_to(p(3.5, 3.5), p(5.0, 3.5));
                    b.close();
                });
                frame.stroke(&card, stroke);

                // Reveal mask circular boundary
                let mask_center = p(12.0, 12.0);
                frame.stroke(&Path::circle(mask_center, d(5.5)), stroke);

                // Left hemisphere solid mask fill
                let mask_half = Path::new(|b| {
                    b.move_to(p(12.0, 6.5));
                    b.bezier_curve_to(p(8.96, 6.5), p(6.5, 8.96), p(6.5, 12.0));
                    b.bezier_curve_to(p(6.5, 15.04), p(8.96, 17.5), p(12.0, 17.5));
                    b.close();
                });
                frame.fill(&mask_half, self.color);

                // Center mask divide line
                frame.stroke(&Path::line(p(12.0, 6.5), p(12.0, 17.5)), stroke);

                // Right hemisphere revealed layer slats
                frame.stroke(&Path::line(p(12.0, 9.5), p(16.5, 9.5)), stroke);
                frame.stroke(&Path::line(p(12.0, 12.0), p(17.5, 12.0)), stroke);
                frame.stroke(&Path::line(p(12.0, 14.5), p(16.5, 14.5)), stroke);
            }
            Icon::Shuffle => {
                // Crossing arrow 1 (top-left crossing down-right)
                let arrow_top = Path::new(|b| {
                    b.move_to(p(3.5, 7.0));
                    b.line_to(p(7.5, 7.0));
                    b.bezier_curve_to(p(10.5, 7.0), p(13.5, 17.0), p(16.5, 17.0));
                    b.line_to(p(20.5, 17.0));
                });
                frame.stroke(&arrow_top, stroke);

                let head_bot = Path::new(|b| {
                    b.move_to(p(17.0, 14.0));
                    b.line_to(p(20.5, 17.0));
                    b.line_to(p(17.0, 20.0));
                });
                frame.stroke(&head_bot, stroke);

                // Crossing arrow 2 (bottom-left crossing up-right, with clean overlap gap)
                let arrow_bot_1 = Path::new(|b| {
                    b.move_to(p(3.5, 17.0));
                    b.line_to(p(7.5, 17.0));
                    b.bezier_curve_to(p(9.0, 17.0), p(10.2, 14.5), p(10.8, 13.5));
                });
                frame.stroke(&arrow_bot_1, stroke);

                let arrow_bot_2 = Path::new(|b| {
                    b.move_to(p(13.2, 10.5));
                    b.bezier_curve_to(p(13.8, 9.5), p(15.0, 7.0), p(16.5, 7.0));
                    b.line_to(p(20.5, 7.0));
                });
                frame.stroke(&arrow_bot_2, stroke);

                let head_top = Path::new(|b| {
                    b.move_to(p(17.0, 4.0));
                    b.line_to(p(20.5, 7.0));
                    b.line_to(p(17.0, 10.0));
                });
                frame.stroke(&head_top, stroke);
            }
        }

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        mouse::Interaction::default()
    }
}

fn star_path(center: Point, r_out: f32, r_in: f32) -> Path {
    Path::new(|b| {
        for i in 0..10 {
            let angle = -std::f32::consts::FRAC_PI_2 + (i as f32) * (std::f32::consts::PI / 5.0);
            let r = if i % 2 == 0 { r_out } else { r_in };
            let pt = Point::new(center.x + r * angle.cos(), center.y + r * angle.sin());
            if i == 0 {
                b.move_to(pt);
            } else {
                b.line_to(pt);
            }
        }
        b.close();
    })
}
