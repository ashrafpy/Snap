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
