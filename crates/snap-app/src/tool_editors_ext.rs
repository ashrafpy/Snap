use iced::alignment::Horizontal;
use iced::mouse;
use iced::widget::canvas::{self, Action, Frame, Geometry, Path, Stroke};
use iced::widget::{button, checkbox, column, container, row, slider, space, text, text_input};
use iced::{Alignment, Color, Element, Length, Point, Rectangle, Renderer, Theme};
use snap_core::mask::SmartMaskKind;
use snap_core::ops::EditOp;
use snap_core::ops_ext::*;
use uuid::Uuid;

use crate::icons::{icon, Icon};
use crate::theme::{self, palette};

// -----------------------------------------------------------------------------
// Events
// -----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum ToolEditorEvent {
    /// Live update during interactive drag (e.g. Color Grade wheel drag or text input).
    PreviewOp(EditOp),
    /// Discrete or completed edit that should also push to ToolSessionHistory.
    CommitOp(EditOp),
    /// Toggle click-on-canvas mode to place a new U-Point Selective pin.
    ToggleSelectiveAddPin(bool),
    /// Open file picker to choose a secondary image for Double Exposure.
    PickDoubleExposureImage,
    /// Layer Mask inspector actions (for View Edits per-node masking):
    MaskSetPaintMode { node_id: Uuid, active: bool },
    MaskSetEraser(bool),
    MaskSetBrushRadius(f32),
    MaskSetBrushHardness(f32),
    MaskSetTargetOpacity(f32),
    MaskToggleRedOverlay(bool),
    MaskInvert(Uuid),
    MaskClear(Uuid),
    MaskResetFull(Uuid),
    MaskApplySmart { node_id: Uuid, kind: SmartMaskKind },
}

// -----------------------------------------------------------------------------
// Helpers: HSV Color Conversion
// -----------------------------------------------------------------------------

fn hsv_to_rgb(h_deg: f32, s: f32, v: f32) -> Color {
    let h = ((h_deg % 360.0) + 360.0) % 360.0;
    let s = s.clamp(0.0, 1.0);
    let v = v.clamp(0.0, 1.0);
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match (h / 60.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    Color::from_rgb(r + m, g + m, b + m)
}

fn text_color_preset_rgb(preset: TextColorPreset) -> Color {
    match preset {
        TextColorPreset::White => Color::from_rgb(0.96, 0.96, 0.96),
        TextColorPreset::Black => Color::from_rgb(0.12, 0.12, 0.14),
        TextColorPreset::Cream => Color::from_rgb(0.96, 0.93, 0.84),
        TextColorPreset::Gold => Color::from_rgb(0.94, 0.76, 0.28),
        TextColorPreset::Coral => Color::from_rgb(0.95, 0.45, 0.38),
        TextColorPreset::Cyan => Color::from_rgb(0.25, 0.78, 0.92),
        TextColorPreset::Sage => Color::from_rgb(0.55, 0.72, 0.60),
        TextColorPreset::Slate => Color::from_rgb(0.45, 0.52, 0.62),
    }
}

// -----------------------------------------------------------------------------
// Interactive Color Grade Wheel (Canvas Widget)
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Default)]
pub struct ColorWheelState {
    pub is_dragging: bool,
}

struct ColorWheelCanvas {
    params: ColorGradeParams,
}

impl canvas::Program<ToolEditorEvent> for ColorWheelCanvas {
    type State = ColorWheelState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<Action<ToolEditorEvent>> {
        let w = bounds.width;
        let h = bounds.height;
        if w <= 0.0 || h <= 0.0 {
            return None;
        }

        let cx = w / 2.0;
        let cy = h / 2.0;
        let radius = (w.min(h) / 2.0 - 12.0).max(10.0);

        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let Some(cursor_pos) = cursor.position_in(bounds) else {
                    return None;
                };
                let dx = cursor_pos.x - cx;
                let dy = cursor_pos.y - cy;
                let dist = (dx * dx + dy * dy).sqrt();

                if dist <= radius + 14.0 {
                    state.is_dragging = true;
                    let sat = ((dist / radius).clamp(0.0, 1.0) * 100.0).round();
                    let mut hue = dy.atan2(dx).to_degrees();
                    if hue < 0.0 {
                        hue += 360.0;
                    }
                    hue = hue.round();
                    if hue >= 360.0 {
                        hue = 0.0;
                    }

                    let mut new_p = self.params.clone();
                    match new_p.active_wheel {
                        ColorWheelZone::Shadows => {
                            new_p.shadows.hue_deg = hue;
                            new_p.shadows.saturation = sat;
                        }
                        ColorWheelZone::Midtones => {
                            new_p.midtones.hue_deg = hue;
                            new_p.midtones.saturation = sat;
                        }
                        ColorWheelZone::Highlights => {
                            new_p.highlights.hue_deg = hue;
                            new_p.highlights.saturation = sat;
                        }
                        ColorWheelZone::Global => {
                            new_p.global.hue_deg = hue;
                            new_p.global.saturation = sat;
                        }
                    }

                    return Some(Action::publish(ToolEditorEvent::PreviewOp(
                        EditOp::ColorGrade(new_p),
                    )));
                }
            }

            canvas::Event::Mouse(mouse::Event::CursorMoved { position }) => {
                if state.is_dragging {
                    let cursor_x = position.x - bounds.x;
                    let cursor_y = position.y - bounds.y;
                    let dx = cursor_x - cx;
                    let dy = cursor_y - cy;
                    let dist = (dx * dx + dy * dy).sqrt();

                    let sat = ((dist / radius).clamp(0.0, 1.0) * 100.0).round();
                    let mut hue = dy.atan2(dx).to_degrees();
                    if hue < 0.0 {
                        hue += 360.0;
                    }
                    hue = hue.round();
                    if hue >= 360.0 {
                        hue = 0.0;
                    }

                    let mut new_p = self.params.clone();
                    match new_p.active_wheel {
                        ColorWheelZone::Shadows => {
                            new_p.shadows.hue_deg = hue;
                            new_p.shadows.saturation = sat;
                        }
                        ColorWheelZone::Midtones => {
                            new_p.midtones.hue_deg = hue;
                            new_p.midtones.saturation = sat;
                        }
                        ColorWheelZone::Highlights => {
                            new_p.highlights.hue_deg = hue;
                            new_p.highlights.saturation = sat;
                        }
                        ColorWheelZone::Global => {
                            new_p.global.hue_deg = hue;
                            new_p.global.saturation = sat;
                        }
                    }

                    return Some(Action::publish(ToolEditorEvent::PreviewOp(
                        EditOp::ColorGrade(new_p),
                    )));
                }
            }

            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if state.is_dragging {
                    state.is_dragging = false;

                    let cursor_pt = cursor.position_in(bounds).unwrap_or(Point::new(cx, cy));
                    let dx = cursor_pt.x - cx;
                    let dy = cursor_pt.y - cy;
                    let dist = (dx * dx + dy * dy).sqrt();

                    let sat = ((dist / radius).clamp(0.0, 1.0) * 100.0).round();
                    let mut hue = dy.atan2(dx).to_degrees();
                    if hue < 0.0 {
                        hue += 360.0;
                    }
                    hue = hue.round();
                    if hue >= 360.0 {
                        hue = 0.0;
                    }

                    let mut new_p = self.params.clone();
                    match new_p.active_wheel {
                        ColorWheelZone::Shadows => {
                            new_p.shadows.hue_deg = hue;
                            new_p.shadows.saturation = sat;
                        }
                        ColorWheelZone::Midtones => {
                            new_p.midtones.hue_deg = hue;
                            new_p.midtones.saturation = sat;
                        }
                        ColorWheelZone::Highlights => {
                            new_p.highlights.hue_deg = hue;
                            new_p.highlights.saturation = sat;
                        }
                        ColorWheelZone::Global => {
                            new_p.global.hue_deg = hue;
                            new_p.global.saturation = sat;
                        }
                    }

                    return Some(Action::publish(ToolEditorEvent::CommitOp(
                        EditOp::ColorGrade(new_p),
                    )));
                }
            }

            _ => {}
        }

        None
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let w = bounds.width;
        let h = bounds.height;
        if w <= 0.0 || h <= 0.0 {
            return vec![frame.into_geometry()];
        }

        let cx = w / 2.0;
        let cy = h / 2.0;
        let center = Point::new(cx, cy);
        let radius = (w.min(h) / 2.0 - 12.0).max(10.0);

        // 1. Sunken background circular disk
        let disk = Path::circle(center, radius);
        frame.fill(&disk, palette::BG_SUNKEN);
        frame.stroke(
            &disk,
            Stroke::default()
                .with_color(palette::BORDER_STRONG)
                .with_width(1.0),
        );

        // 2. Concentric saturation guide rings (25%, 50%, 75%)
        let ring_stroke = Stroke::default()
            .with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.06))
            .with_width(1.0);
        for frac in [0.25, 0.50, 0.75] {
            let ring = Path::circle(center, radius * frac);
            frame.stroke(&ring, ring_stroke);
        }

        // 3. Subtle center crosshairs
        let crosshair_stroke = Stroke::default()
            .with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.08))
            .with_width(1.0);

        let mut h_line = canvas::path::Builder::new();
        h_line.move_to(Point::new(cx - radius, cy));
        h_line.line_to(Point::new(cx + radius, cy));
        frame.stroke(&h_line.build(), crosshair_stroke);

        let mut v_line = canvas::path::Builder::new();
        v_line.move_to(Point::new(cx, cy - radius));
        v_line.line_to(Point::new(cx, cy + radius));
        frame.stroke(&v_line.build(), crosshair_stroke);

        // 4. Darkroom Chromatic Ring: 36 outer ticks with hue-mapped colors
        for i in 0..36 {
            let deg = i as f32 * 10.0;
            let rad = deg.to_radians();
            let is_primary = i % 6 == 0;
            let inner_r = if is_primary {
                radius - 8.0
            } else {
                radius - 4.5
            };
            let stroke_w = if is_primary { 2.5 } else { 1.5 };
            let col = hsv_to_rgb(deg, 0.90, 0.90);

            let p1 = Point::new(cx + inner_r * rad.cos(), cy + inner_r * rad.sin());
            let p2 = Point::new(cx + (radius - 1.0) * rad.cos(), cy + (radius - 1.0) * rad.sin());

            let mut tick = canvas::path::Builder::new();
            tick.move_to(p1);
            tick.line_to(p2);
            frame.stroke(
                &tick.build(),
                Stroke::default().with_color(col).with_width(stroke_w),
            );
        }

        // 5. Center neutral dot
        let center_dot = Path::circle(center, 3.0);
        frame.fill(&center_dot, Color::from_rgba(1.0, 1.0, 1.0, 0.25));

        // 6. Active draggable color puck
        let active_val = match self.params.active_wheel {
            ColorWheelZone::Shadows => self.params.shadows,
            ColorWheelZone::Midtones => self.params.midtones,
            ColorWheelZone::Highlights => self.params.highlights,
            ColorWheelZone::Global => self.params.global,
        };

        let puck_rad = active_val.hue_deg.to_radians();
        let puck_dist = (active_val.saturation / 100.0).clamp(0.0, 1.0) * radius;
        let puck_pt = Point::new(
            cx + puck_dist * puck_rad.cos(),
            cy + puck_dist * puck_rad.sin(),
        );

        // Leash from center to puck if displaced
        if puck_dist > 3.0 {
            let mut leash = canvas::path::Builder::new();
            leash.move_to(center);
            leash.line_to(puck_pt);
            frame.stroke(
                &leash.build(),
                Stroke::default()
                    .with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.20))
                    .with_width(1.0),
            );
        }

        // Puck drop shadow
        let puck_shadow = Path::circle(puck_pt, 9.0);
        frame.fill(&puck_shadow, Color::from_rgba(0.0, 0.0, 0.0, 0.50));

        // Puck outer ring
        let puck_ring = Path::circle(puck_pt, 7.5);
        frame.stroke(
            &puck_ring,
            Stroke::default().with_color(Color::WHITE).with_width(2.0),
        );

        // Puck inner fill
        let puck_fill_color = if active_val.saturation > 1.0 {
            hsv_to_rgb(active_val.hue_deg, active_val.saturation / 100.0, 0.95)
        } else {
            palette::BG_SURFACE
        };
        let puck_core = Path::circle(puck_pt, 5.5);
        frame.fill(&puck_core, puck_fill_color);

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if state.is_dragging {
            mouse::Interaction::Grabbing
        } else if cursor.is_over(bounds) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}

// -----------------------------------------------------------------------------
// Individual Tool Custom Editors
// -----------------------------------------------------------------------------

fn view_color_grade_controls<'a>(p: &'a ColorGradeParams) -> Element<'a, ToolEditorEvent> {
    // 4-zone segmented selector (Shadows, Midtones, Highlights, Global)
    let mut tab_row = row![].spacing(2);
    for &zone in ColorWheelZone::ALL {
        let is_sel = p.active_wheel == zone;
        let mut new_p = p.clone();
        new_p.active_wheel = zone;
        let btn = button(
            container(text(zone.display_name()).size(11))
                .width(Length::Fill)
                .align_x(Horizontal::Center),
        )
        .padding([5, 4])
        .style(theme::segmented_tab_button(is_sel))
        .width(Length::FillPortion(1))
        .on_press(ToolEditorEvent::PreviewOp(EditOp::ColorGrade(new_p)));
        tab_row = tab_row.push(btn);
    }
    let zone_selector = container(tab_row)
        .padding(2)
        .style(theme::sunken_track_style)
        .width(Length::Fill);

    // Color Wheel Canvas
    let wheel_widget = iced::widget::canvas(ColorWheelCanvas {
        params: p.clone(),
    })
    .width(Length::Fixed(190.0))
    .height(Length::Fixed(190.0));

    let wheel_card = container(wheel_widget)
        .width(Length::Fill)
        .align_x(Horizontal::Center)
        .padding([4, 0]);

    let active_val = match p.active_wheel {
        ColorWheelZone::Shadows => p.shadows,
        ColorWheelZone::Midtones => p.midtones,
        ColorWheelZone::Highlights => p.highlights,
        ColorWheelZone::Global => p.global,
    };

    let readout_row = row![
        text(format!("Hue: {:.0} deg", active_val.hue_deg))
            .size(11)
            .color(palette::TEXT_MUTED),
        space::horizontal(),
        text(format!("Sat: {:.0}%", active_val.saturation))
            .size(11)
            .color(palette::TEXT_MUTED),
    ]
    .align_y(Alignment::Center);

    // Luminance slider for the active wheel
    let new_p_lum = p.clone();
    let lum_slider = row![
        text("Lum").size(11).color(palette::TEXT_MUTED),
        slider(-100.0..=100.0_f32, active_val.luminance, move |v| {
            let mut cp = new_p_lum.clone();
            match cp.active_wheel {
                ColorWheelZone::Shadows => cp.shadows.luminance = v,
                ColorWheelZone::Midtones => cp.midtones.luminance = v,
                ColorWheelZone::Highlights => cp.highlights.luminance = v,
                ColorWheelZone::Global => cp.global.luminance = v,
            }
            ToolEditorEvent::PreviewOp(EditOp::ColorGrade(cp))
        })
        .step(1.0_f32)
        .style(theme::studio_slider(palette::ACCENT)),
        text(format!("{:+0.0}", active_val.luminance))
            .size(11)
            .color(palette::TEXT_MUTED),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    // Reset Wheel button
    let mut reset_p = p.clone();
    match reset_p.active_wheel {
        ColorWheelZone::Shadows => reset_p.shadows = ColorWheelValue::default(),
        ColorWheelZone::Midtones => reset_p.midtones = ColorWheelValue::default(),
        ColorWheelZone::Highlights => reset_p.highlights = ColorWheelValue::default(),
        ColorWheelZone::Global => reset_p.global = ColorWheelValue::default(),
    }
    let reset_btn = button(
        container(
            row![
                icon(Icon::RotateCcw, 12.0, palette::TEXT_SECONDARY),
                text("Reset Wheel").size(11).color(palette::TEXT_SECONDARY),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .align_x(Horizontal::Center),
    )
    .padding([5, 10])
    .style(theme::secondary_button)
    .width(Length::Fill)
    .on_press(ToolEditorEvent::CommitOp(EditOp::ColorGrade(reset_p)));

    column![
        zone_selector,
        wheel_card,
        readout_row,
        lum_slider,
        reset_btn,
    ]
    .spacing(8)
    .into()
}

fn view_perspective_controls<'a>(p: &'a PerspectiveParams) -> Element<'a, ToolEditorEvent> {
    let mut mode_row = row![].spacing(6);
    for &mode in PerspectiveFillMode::ALL {
        let is_sel = p.edge_fill == mode;
        let mut new_p = p.clone();
        new_p.edge_fill = mode;
        let btn = button(
            container(text(mode.display_name()).size(11))
                .width(Length::Fill)
                .align_x(Horizontal::Center),
        )
        .padding([5, 8])
        .style(theme::pill_chip_button(is_sel))
        .width(Length::FillPortion(1))
        .on_press(ToolEditorEvent::CommitOp(EditOp::Perspective(new_p)));
        mode_row = mode_row.push(btn);
    }

    let mut reset_p = p.clone();
    reset_p.vertical = 0.0;
    reset_p.horizontal = 0.0;
    reset_p.rotate_deg = 0.0;
    reset_p.scale = 1.0;

    let reset_btn = button(
        container(
            row![
                icon(Icon::RotateCcw, 12.0, palette::TEXT_SECONDARY),
                text("Reset Perspective").size(11).color(palette::TEXT_SECONDARY),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .align_x(Horizontal::Center),
    )
    .padding([5, 10])
    .style(theme::secondary_button)
    .width(Length::Fill)
    .on_press(ToolEditorEvent::CommitOp(EditOp::Perspective(reset_p)));

    column![
        text("EDGE FILL MODE").size(11).color(palette::TEXT_MUTED),
        mode_row,
        reset_btn,
    ]
    .spacing(8)
    .into()
}

fn view_expand_controls<'a>(p: &'a ExpandParams) -> Element<'a, ToolEditorEvent> {
    let mut mode_row = row![].spacing(6);
    for &mode in ExpandFillMode::ALL {
        let is_sel = p.fill_mode == mode;
        let mut new_p = p.clone();
        new_p.fill_mode = mode;
        let btn = button(
            container(text(mode.display_name()).size(11))
                .width(Length::Fill)
                .align_x(Horizontal::Center),
        )
        .padding([5, 8])
        .style(theme::pill_chip_button(is_sel))
        .width(Length::FillPortion(1))
        .on_press(ToolEditorEvent::CommitOp(EditOp::Expand(new_p)));
        mode_row = mode_row.push(btn);
    }

    let mut reset_p = p.clone();
    reset_p.top = 0.0;
    reset_p.bottom = 0.0;
    reset_p.left = 0.0;
    reset_p.right = 0.0;

    let reset_btn = button(
        container(
            row![
                icon(Icon::RotateCcw, 12.0, palette::TEXT_SECONDARY),
                text("Reset Expansion").size(11).color(palette::TEXT_SECONDARY),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .align_x(Horizontal::Center),
    )
    .padding([5, 10])
    .style(theme::secondary_button)
    .width(Length::Fill)
    .on_press(ToolEditorEvent::CommitOp(EditOp::Expand(reset_p)));

    column![
        text("CANVAS EXPAND FILL").size(11).color(palette::TEXT_MUTED),
        mode_row,
        reset_btn,
    ]
    .spacing(8)
    .into()
}

fn view_selective_controls<'a>(
    p: &'a SelectiveParams,
    selective_add_pin_mode: bool,
) -> Element<'a, ToolEditorEvent> {
    let add_pin_btn = button(
        container(
            row![
                icon(
                    Icon::Selective,
                    14.0,
                    if selective_add_pin_mode {
                        Color::WHITE
                    } else {
                        palette::TEXT_PRIMARY
                    },
                ),
                text(if selective_add_pin_mode {
                    "Click Photo to Place"
                } else {
                    "+ Add Pin"
                })
                .size(11)
                .color(if selective_add_pin_mode {
                    Color::WHITE
                } else {
                    palette::TEXT_PRIMARY
                }),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .align_x(Horizontal::Center),
    )
    .padding([6, 10])
    .style(if selective_add_pin_mode {
        theme::accent_button
    } else {
        theme::secondary_button
    })
    .width(Length::FillPortion(1))
    .on_press(ToolEditorEvent::ToggleSelectiveAddPin(
        !selective_add_pin_mode,
    ));

    let mut header_row = row![add_pin_btn].spacing(6).align_y(Alignment::Center);

    if !p.pins.is_empty() {
        let mut new_p = p.clone();
        if new_p.active_pin < new_p.pins.len() {
            new_p.pins.remove(new_p.active_pin);
            if new_p.active_pin >= new_p.pins.len() && !new_p.pins.is_empty() {
                new_p.active_pin = new_p.pins.len() - 1;
            }
        }
        let delete_pin_btn = button(
            container(
                row![
                    icon(Icon::Trash, 12.0, palette::DANGER),
                    text("Delete Pin").size(11).color(palette::DANGER),
                ]
                .spacing(4)
                .align_y(Alignment::Center),
            )
            .align_x(Horizontal::Center),
        )
        .padding([6, 8])
        .style(theme::subtle_danger_button)
        .on_press(ToolEditorEvent::CommitOp(EditOp::Selective(new_p)));

        header_row = header_row.push(delete_pin_btn);
    }

    let mut content = column![header_row].spacing(8);

    if p.pins.is_empty() {
        content = content.push(
            text("Click '+ Add Pin' then click photo to place a U-Point")
                .size(11)
                .color(palette::TEXT_MUTED),
        );
    } else {
        let mut pins_col = column![].spacing(6);
        let num_pins = p.pins.len().min(MAX_SELECTIVE_PINS);
        let indices: Vec<usize> = (0..num_pins).collect();
        for chunk in indices.chunks(4) {
            let mut r = row![].spacing(6);
            for &idx in chunk {
                let is_sel = p.active_pin == idx;
                let mut new_p = p.clone();
                new_p.active_pin = idx;
                let btn = button(
                    container(text(format!("#{}", idx + 1)).size(11))
                        .width(Length::Fill)
                        .align_x(Horizontal::Center),
                )
                .padding([5, 6])
                .style(theme::pill_chip_button(is_sel))
                .width(Length::FillPortion(1))
                .on_press(ToolEditorEvent::PreviewOp(EditOp::Selective(new_p)));
                r = r.push(btn);
            }
            for _ in chunk.len()..4 {
                r = r.push(container(text("").size(1)).width(Length::FillPortion(1)));
            }
            pins_col = pins_col.push(r);
        }

        if let Some(pin) = p.pins.get(p.active_pin) {
            let info = text(format!(
                "Pin #{} at ({:.0}%, {:.0}%) | Radius: {:.0}%",
                p.active_pin + 1,
                pin.x * 100.0,
                pin.y * 100.0,
                pin.radius * 100.0
            ))
            .size(10)
            .color(palette::TEXT_MUTED);
            pins_col = pins_col.push(info);
        }

        content = content.push(pins_col);
    }

    content.into()
}

fn view_brush_controls<'a>(p: &'a BrushParams) -> Element<'a, ToolEditorEvent> {
    let mut chan_col = column![
        text("BRUSH CHANNEL").size(11).color(palette::TEXT_MUTED),
    ]
    .spacing(6);

    for chunk in BrushChannel::ALL.chunks(2) {
        let mut r = row![].spacing(6);
        for &ch in chunk {
            let is_sel = p.channel == ch;
            let mut new_p = p.clone();
            new_p.channel = ch;
            let btn = button(
                container(text(ch.display_name()).size(11))
                    .width(Length::Fill)
                    .align_x(Horizontal::Center),
            )
            .padding([5, 8])
            .style(theme::pill_chip_button(is_sel))
            .width(Length::FillPortion(1))
            .on_press(ToolEditorEvent::CommitOp(EditOp::Brush(new_p)));
            r = r.push(btn);
        }
        chan_col = chan_col.push(r);
    }

    // Preset steps (-1.0, -0.5, +0.5, +1.0)
    let step_presets = [
        (-100.0_f32, "-1.0"),
        (-50.0_f32, "-0.5"),
        (50.0_f32, "+0.5"),
        (100.0_f32, "+1.0"),
    ];
    let mut step_row = row![].spacing(6);
    for &(val, label) in &step_presets {
        let is_sel = (p.step - val).abs() < 1e-2;
        let mut new_p = p.clone();
        new_p.step = val;
        let btn = button(
            container(text(label).size(11))
                .width(Length::Fill)
                .align_x(Horizontal::Center),
        )
        .padding([4, 6])
        .style(theme::pill_chip_button(is_sel))
        .width(Length::FillPortion(1))
        .on_press(ToolEditorEvent::CommitOp(EditOp::Brush(new_p)));
        step_row = step_row.push(btn);
    }

    let mut eraser_p = p.clone();
    eraser_p.eraser = !p.eraser;
    let eraser_btn = button(
        container(
            row![
                icon(
                    Icon::Brush,
                    12.0,
                    if p.eraser {
                        Color::WHITE
                    } else {
                        palette::TEXT_SECONDARY
                    },
                ),
                text(if p.eraser { "Eraser Active" } else { "Eraser Mode" })
                    .size(11)
                    .color(if p.eraser {
                        Color::WHITE
                    } else {
                        palette::TEXT_SECONDARY
                    }),
            ]
            .spacing(5)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .align_x(Horizontal::Center),
    )
    .padding([5, 8])
    .style(theme::pill_chip_button(p.eraser))
    .width(Length::FillPortion(1))
    .on_press(ToolEditorEvent::CommitOp(EditOp::Brush(eraser_p)));

    let mut clear_p = p.clone();
    clear_p.map.clear();
    let clear_btn = button(
        container(
            row![
                icon(Icon::Trash, 12.0, palette::DANGER),
                text("Clear Brush").size(11).color(palette::DANGER),
            ]
            .spacing(5)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .align_x(Horizontal::Center),
    )
    .padding([5, 8])
    .style(theme::subtle_danger_button)
    .width(Length::FillPortion(1))
    .on_press(ToolEditorEvent::CommitOp(EditOp::Brush(clear_p)));

    let action_row = row![eraser_btn, clear_btn].spacing(6);

    column![
        chan_col,
        text("STEP INTENSITY PRESETS").size(11).color(palette::TEXT_MUTED),
        step_row,
        action_row,
    ]
    .spacing(8)
    .into()
}

fn view_healing_controls<'a>(p: &'a HealingParams) -> Element<'a, ToolEditorEvent> {
    let count_text = text(format!("Strokes: {}", p.strokes.len()))
        .size(11)
        .color(palette::TEXT_MUTED);

    let mut undo_p = p.clone();
    let can_undo = !undo_p.strokes.is_empty();
    if can_undo {
        undo_p.strokes.pop();
    }
    let mut undo_btn = button(
        container(
            row![
                icon(
                    Icon::Undo,
                    12.0,
                    if can_undo {
                        palette::TEXT_PRIMARY
                    } else {
                        palette::TEXT_MUTED
                    },
                ),
                text("Undo Last").size(11).color(if can_undo {
                    palette::TEXT_PRIMARY
                } else {
                    palette::TEXT_MUTED
                }),
            ]
            .spacing(5)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .align_x(Horizontal::Center),
    )
    .padding([5, 8])
    .style(theme::secondary_button)
    .width(Length::FillPortion(1));

    if can_undo {
        undo_btn = undo_btn.on_press(ToolEditorEvent::CommitOp(EditOp::Healing(undo_p)));
    }

    let mut clear_p = p.clone();
    let can_clear = !clear_p.strokes.is_empty();
    clear_p.strokes.clear();
    let mut clear_btn = button(
        container(
            row![
                icon(
                    Icon::Trash,
                    12.0,
                    if can_clear {
                        palette::DANGER
                    } else {
                        palette::TEXT_MUTED
                    },
                ),
                text("Clear All").size(11).color(if can_clear {
                    palette::DANGER
                } else {
                    palette::TEXT_MUTED
                }),
            ]
            .spacing(5)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .align_x(Horizontal::Center),
    )
    .padding([5, 8])
    .style(theme::subtle_danger_button)
    .width(Length::FillPortion(1));

    if can_clear {
        clear_btn = clear_btn.on_press(ToolEditorEvent::CommitOp(EditOp::Healing(clear_p)));
    }

    let action_row = row![undo_btn, clear_btn].spacing(6);

    let hint = text("Drag over blemishes on canvas to heal")
        .size(11)
        .color(palette::TEXT_MUTED);

    column![
        row![
            text("HEALING PATCH").size(11).color(palette::TEXT_MUTED),
            space::horizontal(),
            count_text,
        ]
        .align_y(Alignment::Center),
        action_row,
        hint,
    ]
    .spacing(8)
    .into()
}

fn view_grainy_film_controls<'a>(p: &'a GrainyFilmParams) -> Element<'a, ToolEditorEvent> {
    let mut stocks_col = column![
        text("FILM STOCK PRESETS").size(11).color(palette::TEXT_MUTED),
    ]
    .spacing(6);

    for chunk in FilmStock::ALL.chunks(2) {
        let mut r = row![].spacing(6);
        for &stock in chunk {
            let is_sel = p.stock == stock;
            let mut new_p = p.clone();
            new_p.stock = stock;
            let btn = button(
                container(text(stock.display_name()).size(11))
                    .width(Length::Fill)
                    .align_x(Horizontal::Center),
            )
            .padding([5, 6])
            .style(theme::pill_chip_button(is_sel))
            .width(Length::FillPortion(1))
            .on_press(ToolEditorEvent::CommitOp(EditOp::GrainyFilm(new_p)));
            r = r.push(btn);
        }
        stocks_col = stocks_col.push(r);
    }

    stocks_col.into()
}

fn view_noir_controls<'a>(p: &'a NoirParams) -> Element<'a, ToolEditorEvent> {
    let mut presets_col = column![
        text("NOIR PRESETS").size(11).color(palette::TEXT_MUTED),
    ]
    .spacing(6);

    for chunk in NoirPreset::ALL.chunks(2) {
        let mut r = row![].spacing(6);
        for &preset in chunk {
            let is_sel = p.preset == preset;
            let mut new_p = p.clone();
            new_p.preset = preset;
            let btn = button(
                container(text(preset.display_name()).size(11))
                    .width(Length::Fill)
                    .align_x(Horizontal::Center),
            )
            .padding([5, 8])
            .style(theme::pill_chip_button(is_sel))
            .width(Length::FillPortion(1))
            .on_press(ToolEditorEvent::CommitOp(EditOp::Noir(new_p)));
            r = r.push(btn);
        }
        presets_col = presets_col.push(r);
    }

    presets_col.into()
}

fn view_hdr_scape_controls<'a>(p: &'a HdrScapeParams) -> Element<'a, ToolEditorEvent> {
    let mut presets_col = column![
        text("HDR SCAPE PRESETS").size(11).color(palette::TEXT_MUTED),
    ]
    .spacing(6);

    for chunk in HdrScapePreset::ALL.chunks(2) {
        let mut r = row![].spacing(6);
        for &preset in chunk {
            let is_sel = p.preset == preset;
            let mut new_p = p.clone();
            new_p.preset = preset;
            let btn = button(
                container(text(preset.display_name()).size(11))
                    .width(Length::Fill)
                    .align_x(Horizontal::Center),
            )
            .padding([5, 8])
            .style(theme::pill_chip_button(is_sel))
            .width(Length::FillPortion(1))
            .on_press(ToolEditorEvent::CommitOp(EditOp::HdrScape(new_p)));
            r = r.push(btn);
        }
        presets_col = presets_col.push(r);
    }

    presets_col.into()
}

fn view_glamour_glow_controls<'a>(p: &'a GlamourGlowParams) -> Element<'a, ToolEditorEvent> {
    let mut style_row = row![].spacing(6);
    for s in 1..=5 {
        let is_sel = p.style == s;
        let mut new_p = p.clone();
        new_p.style = s;
        let btn = button(
            container(text(format!("Style {}", s)).size(11))
                .width(Length::Fill)
                .align_x(Horizontal::Center),
        )
        .padding([5, 6])
        .style(theme::pill_chip_button(is_sel))
        .width(Length::FillPortion(1))
        .on_press(ToolEditorEvent::CommitOp(EditOp::GlamourGlow(new_p)));
        style_row = style_row.push(btn);
    }

    column![
        text("GLOW STYLE").size(11).color(palette::TEXT_MUTED),
        style_row,
    ]
    .spacing(6)
    .into()
}

fn view_drama_controls<'a>(p: &'a DramaParams) -> Element<'a, ToolEditorEvent> {
    let mut presets_col = column![
        text("DRAMA PRESETS").size(11).color(palette::TEXT_MUTED),
    ]
    .spacing(6);

    for chunk in DramaPreset::ALL.chunks(2) {
        let mut r = row![].spacing(6);
        for &preset in chunk {
            let is_sel = p.preset == preset;
            let mut new_p = p.clone();
            new_p.preset = preset;
            let btn = button(
                container(text(preset.display_name()).size(11))
                    .width(Length::Fill)
                    .align_x(Horizontal::Center),
            )
            .padding([5, 8])
            .style(theme::pill_chip_button(is_sel))
            .width(Length::FillPortion(1))
            .on_press(ToolEditorEvent::CommitOp(EditOp::Drama(new_p)));
            r = r.push(btn);
        }
        presets_col = presets_col.push(r);
    }

    presets_col.into()
}

fn view_vintage_controls<'a>(p: &'a VintageParams) -> Element<'a, ToolEditorEvent> {
    let mut styles_col = column![
        text("VINTAGE STYLES").size(11).color(palette::TEXT_MUTED),
    ]
    .spacing(6);

    let styles: Vec<u8> = (0..=11).collect();
    for chunk in styles.chunks(4) {
        let mut r = row![].spacing(6);
        for &s in chunk {
            let is_sel = p.style == s;
            let mut new_p = p.clone();
            new_p.style = s;
            let btn = button(
                container(text(format!("S{}", s + 1)).size(11))
                    .width(Length::Fill)
                    .align_x(Horizontal::Center),
            )
            .padding([5, 6])
            .style(theme::pill_chip_button(is_sel))
            .width(Length::FillPortion(1))
            .on_press(ToolEditorEvent::CommitOp(EditOp::Vintage(new_p)));
            r = r.push(btn);
        }
        styles_col = styles_col.push(r);
    }

    styles_col.into()
}

fn view_lens_blur_controls<'a>(p: &'a LensBlurParams) -> Element<'a, ToolEditorEvent> {
    let mut shape_row = row![].spacing(6);
    for &shape in LensBlurShape::ALL {
        let is_sel = p.shape == shape;
        let mut new_p = p.clone();
        new_p.shape = shape;
        let btn = button(
            container(text(shape.display_name()).size(11))
                .width(Length::Fill)
                .align_x(Horizontal::Center),
        )
        .padding([5, 8])
        .style(theme::pill_chip_button(is_sel))
        .width(Length::FillPortion(1))
        .on_press(ToolEditorEvent::CommitOp(EditOp::LensBlur(new_p)));
        shape_row = shape_row.push(btn);
    }

    let badge = container(
        text("Bokeh Boost Optical Kernel")
            .size(10)
            .color(palette::TEXT_MUTED),
    )
    .padding([4, 8])
    .style(theme::badge_pill_style(
        palette::BG_SUNKEN,
        palette::BORDER_SUBTLE,
    ));

    column![
        text("BLUR APERTURE SHAPE").size(11).color(palette::TEXT_MUTED),
        shape_row,
        container(badge).width(Length::Fill).align_x(Horizontal::Center),
    ]
    .spacing(8)
    .into()
}

fn view_motion_blur_controls<'a>(p: &'a MotionBlurParams) -> Element<'a, ToolEditorEvent> {
    let mut mode_row = row![].spacing(6);
    for &mode in MotionBlurMode::ALL {
        let is_sel = p.mode == mode;
        let mut new_p = p.clone();
        new_p.mode = mode;
        let btn = button(
            container(text(mode.display_name()).size(11))
                .width(Length::Fill)
                .align_x(Horizontal::Center),
        )
        .padding([5, 8])
        .style(theme::pill_chip_button(is_sel))
        .width(Length::FillPortion(1))
        .on_press(ToolEditorEvent::CommitOp(EditOp::MotionBlur(new_p)));
        mode_row = mode_row.push(btn);
    }

    column![
        text("MOTION BLUR MODE").size(11).color(palette::TEXT_MUTED),
        mode_row,
    ]
    .spacing(8)
    .into()
}

fn view_double_exposure_controls<'a>(p: &'a DoubleExposureParams) -> Element<'a, ToolEditorEvent> {
    let pick_btn = button(
        container(
            row![
                icon(Icon::ImageFile, 14.0, palette::ACCENT),
                text("Choose Second Photo...").size(11).color(palette::TEXT_PRIMARY),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .align_x(Horizontal::Center),
    )
    .padding([6, 10])
    .style(theme::secondary_button)
    .width(Length::Fill)
    .on_press(ToolEditorEvent::PickDoubleExposureImage);

    let filename_text = match p.secondary_path.as_deref() {
        Some(path) => {
            let name = std::path::Path::new(path)
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or(path);
            format!("Overlay: {}", name)
        }
        None => "No overlay image selected".to_string(),
    };

    let filename_badge = container(
        text(filename_text)
            .size(10)
            .color(if p.secondary_path.is_some() {
                palette::TEXT_SECONDARY
            } else {
                palette::TEXT_MUTED
            }),
    )
    .padding([4, 8])
    .style(theme::badge_pill_style(
        palette::BG_SUNKEN,
        palette::BORDER_SUBTLE,
    ));

    let mut blend_col = column![
        text("BLEND MODE").size(11).color(palette::TEXT_MUTED),
    ]
    .spacing(6);

    for chunk in DoubleExposureBlend::ALL.chunks(3) {
        let mut r = row![].spacing(6);
        for &blend in chunk {
            let is_sel = p.blend_mode == blend;
            let mut new_p = p.clone();
            new_p.blend_mode = blend;
            let btn = button(
                container(text(blend.display_name()).size(11))
                    .width(Length::Fill)
                    .align_x(Horizontal::Center),
            )
            .padding([5, 6])
            .style(theme::pill_chip_button(is_sel))
            .width(Length::FillPortion(1))
            .on_press(ToolEditorEvent::CommitOp(EditOp::DoubleExposure(new_p)));
            r = r.push(btn);
        }
        blend_col = blend_col.push(r);
    }

    column![
        pick_btn,
        container(filename_badge).width(Length::Fill).align_x(Horizontal::Center),
        blend_col,
    ]
    .spacing(8)
    .into()
}

fn view_retrolux_controls<'a>(p: &'a RetroluxParams) -> Element<'a, ToolEditorEvent> {
    let mut styles_col = column![
        text("RETROLUX STYLES").size(11).color(palette::TEXT_MUTED),
    ]
    .spacing(6);

    let styles: Vec<u8> = (0..=12).collect();
    for chunk in styles.chunks(5) {
        let mut r = row![].spacing(6);
        for &s in chunk {
            let is_sel = p.style == s;
            let mut new_p = p.clone();
            new_p.style = s;
            let btn = button(
                container(text(format!("{}", s + 1)).size(11))
                    .width(Length::Fill)
                    .align_x(Horizontal::Center),
            )
            .padding([5, 6])
            .style(theme::pill_chip_button(is_sel))
            .width(Length::FillPortion(1))
            .on_press(ToolEditorEvent::CommitOp(EditOp::Retrolux(new_p)));
            r = r.push(btn);
        }
        for _ in chunk.len()..5 {
            r = r.push(container(text("").size(1)).width(Length::FillPortion(1)));
        }
        styles_col = styles_col.push(r);
    }

    let mut shuffle_p = p.clone();
    shuffle_p.seed = shuffle_p.seed.wrapping_mul(1664525).wrapping_add(1013904223);

    let shuffle_btn = button(
        container(
            row![
                icon(Icon::Shuffle, 14.0, palette::ACCENT),
                text("Shuffle Scratches & Leaks").size(11).color(palette::TEXT_PRIMARY),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .align_x(Horizontal::Center),
    )
    .padding([6, 10])
    .style(theme::secondary_button)
    .width(Length::Fill)
    .on_press(ToolEditorEvent::CommitOp(EditOp::Retrolux(shuffle_p)));

    column![
        styles_col,
        shuffle_btn,
    ]
    .spacing(8)
    .into()
}

fn view_grunge_controls<'a>(p: &'a GrungeParams) -> Element<'a, ToolEditorEvent> {
    let mut style_row = row![].spacing(6);
    for s in 0..=4 {
        let is_sel = p.style == s;
        let mut new_p = p.clone();
        new_p.style = s;
        let btn = button(
            container(text(format!("Style {}", s + 1)).size(11))
                .width(Length::Fill)
                .align_x(Horizontal::Center),
        )
        .padding([5, 6])
        .style(theme::pill_chip_button(is_sel))
        .width(Length::FillPortion(1))
        .on_press(ToolEditorEvent::CommitOp(EditOp::Grunge(new_p)));
        style_row = style_row.push(btn);
    }

    let mut shuffle_p = p.clone();
    shuffle_p.seed = shuffle_p.seed.wrapping_add(1);

    let shuffle_btn = button(
        container(
            row![
                icon(Icon::Shuffle, 14.0, palette::ACCENT),
                text("Shuffle Texture Seed").size(11).color(palette::TEXT_PRIMARY),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .align_x(Horizontal::Center),
    )
    .padding([6, 10])
    .style(theme::secondary_button)
    .width(Length::Fill)
    .on_press(ToolEditorEvent::CommitOp(EditOp::Grunge(shuffle_p)));

    column![
        text("GRUNGE TEXTURE STYLE").size(11).color(palette::TEXT_MUTED),
        style_row,
        shuffle_btn,
    ]
    .spacing(8)
    .into()
}

fn view_portrait_controls<'a>(p: &'a PortraitParams) -> Element<'a, ToolEditorEvent> {
    let mut tones_col = column![
        text("SKIN TONE PRESET").size(11).color(palette::TEXT_MUTED),
    ]
    .spacing(6);

    for chunk in PortraitSkinTone::ALL.chunks(3) {
        let mut r = row![].spacing(6);
        for &tone in chunk {
            let is_sel = p.skin_tone == tone;
            let mut new_p = p.clone();
            new_p.skin_tone = tone;
            let btn = button(
                container(text(tone.display_name()).size(11))
                    .width(Length::Fill)
                    .align_x(Horizontal::Center),
            )
            .padding([5, 6])
            .style(theme::pill_chip_button(is_sel))
            .width(Length::FillPortion(1))
            .on_press(ToolEditorEvent::CommitOp(EditOp::Portrait(new_p)));
            r = r.push(btn);
        }
        tones_col = tones_col.push(r);
    }

    tones_col.into()
}

fn view_frames_controls<'a>(p: &'a FramesParams) -> Element<'a, ToolEditorEvent> {
    let mut frames_col = column![
        text("FRAME BORDER STYLE").size(11).color(palette::TEXT_MUTED),
    ]
    .spacing(6);

    for chunk in FrameStyle::ALL.chunks(2) {
        let mut r = row![].spacing(6);
        for &style in chunk {
            let is_sel = p.style == style;
            let mut new_p = p.clone();
            new_p.style = style;
            let btn = button(
                container(text(style.display_name()).size(11))
                    .width(Length::Fill)
                    .align_x(Horizontal::Center),
            )
            .padding([5, 8])
            .style(theme::pill_chip_button(is_sel))
            .width(Length::FillPortion(1))
            .on_press(ToolEditorEvent::CommitOp(EditOp::Frames(new_p)));
            r = r.push(btn);
        }
        frames_col = frames_col.push(r);
    }

    let inner_p = p.clone();
    let inner_check = checkbox(p.inner_line)
        .label("Inner Accent Line")
        .style(theme::studio_checkbox)
        .size(14)
        .on_toggle(move |val| {
            let mut cp = inner_p.clone();
            cp.inner_line = val;
            ToolEditorEvent::CommitOp(EditOp::Frames(cp))
        });

    column![
        frames_col,
        inner_check,
    ]
    .spacing(8)
    .into()
}

fn view_text_overlay_controls<'a>(p: &'a TextOverlayParams) -> Element<'a, ToolEditorEvent> {
    let text_p = p.clone();
    let text_inp = text_input("Headline text...", &p.text)
        .size(12)
        .on_input(move |s| {
            let mut cp = text_p.clone();
            cp.text = s;
            ToolEditorEvent::PreviewOp(EditOp::TextOverlay(cp))
        })
        .style(theme::studio_text_input)
        .width(Length::Fill);

    let subtext_p = p.clone();
    let subtext_inp = text_input("Subtext / year...", &p.subtext)
        .size(12)
        .on_input(move |s| {
            let mut cp = subtext_p.clone();
            cp.subtext = s;
            ToolEditorEvent::PreviewOp(EditOp::TextOverlay(cp))
        })
        .style(theme::studio_text_input)
        .width(Length::Fill);

    let mut styles_col = column![
        text("TYPOGRAPHY LAYOUT").size(11).color(palette::TEXT_MUTED),
    ]
    .spacing(6);

    for chunk in TextStyle::ALL.chunks(2) {
        let mut r = row![].spacing(6);
        for &style in chunk {
            let is_sel = p.style == style;
            let mut new_p = p.clone();
            new_p.style = style;
            let btn = button(
                container(text(style.display_name()).size(11))
                    .width(Length::Fill)
                    .align_x(Horizontal::Center),
            )
            .padding([5, 8])
            .style(theme::pill_chip_button(is_sel))
            .width(Length::FillPortion(1))
            .on_press(ToolEditorEvent::CommitOp(EditOp::TextOverlay(new_p)));
            r = r.push(btn);
        }
        styles_col = styles_col.push(r);
    }

    // Color swatches row
    let mut swatches_row = row![].spacing(6).align_y(Alignment::Center);
    for &preset in TextColorPreset::ALL {
        let is_sel = p.color_preset == preset;
        let dot_col = text_color_preset_rgb(preset);
        let mut new_p = p.clone();
        new_p.color_preset = preset;
        let btn = button(text("").size(1))
            .padding([6, 6])
            .style(theme::color_dot_button(dot_col, is_sel))
            .on_press(ToolEditorEvent::CommitOp(EditOp::TextOverlay(new_p)));
        swatches_row = swatches_row.push(btn);
    }
    let swatches_section = column![
        text("TEXT COLOR").size(11).color(palette::TEXT_MUTED),
        container(swatches_row).width(Length::Fill).align_x(Horizontal::Center),
    ]
    .spacing(6);

    let invert_p = p.clone();
    let invert_check = checkbox(p.invert_mask)
        .label("Knockout Mask (Invert)")
        .style(theme::studio_checkbox)
        .size(14)
        .on_toggle(move |val| {
            let mut cp = invert_p.clone();
            cp.invert_mask = val;
            ToolEditorEvent::CommitOp(EditOp::TextOverlay(cp))
        });

    column![
        text("TEXT CONTENT").size(11).color(palette::TEXT_MUTED),
        text_inp,
        subtext_inp,
        styles_col,
        swatches_section,
        invert_check,
    ]
    .spacing(8)
    .into()
}

fn view_head_pose_controls<'a>(p: &'a HeadPoseParams) -> Element<'a, ToolEditorEvent> {
    let mut reset_p = p.clone();
    reset_p.yaw_x = 0.0;
    reset_p.pitch_y = 0.0;
    reset_p.smile = 0.0;
    reset_p.focal_length = 0.0;

    let reset_btn = button(
        container(
            row![
                icon(Icon::RotateCcw, 12.0, palette::TEXT_SECONDARY),
                text("Reset Head Pose").size(11).color(palette::TEXT_SECONDARY),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .align_x(Horizontal::Center),
    )
    .padding([5, 10])
    .style(theme::secondary_button)
    .width(Length::Fill)
    .on_press(ToolEditorEvent::CommitOp(EditOp::HeadPose(reset_p)));

    column![
        text("HEAD POSE ADJUSTMENT").size(11).color(palette::TEXT_MUTED),
        reset_btn,
    ]
    .spacing(8)
    .into()
}

// -----------------------------------------------------------------------------
// Deliverable 1: view_tool_custom_controls
// -----------------------------------------------------------------------------

pub fn view_tool_custom_controls<'a>(
    op: &'a EditOp,
    selective_add_pin_mode: bool,
) -> Option<Element<'a, ToolEditorEvent>> {
    match op {
        EditOp::ColorGrade(p) => Some(view_color_grade_controls(p)),
        EditOp::Perspective(p) => Some(view_perspective_controls(p)),
        EditOp::Expand(p) => Some(view_expand_controls(p)),
        EditOp::Selective(p) => Some(view_selective_controls(p, selective_add_pin_mode)),
        EditOp::Brush(p) => Some(view_brush_controls(p)),
        EditOp::Healing(p) => Some(view_healing_controls(p)),
        EditOp::GrainyFilm(p) => Some(view_grainy_film_controls(p)),
        EditOp::Noir(p) => Some(view_noir_controls(p)),
        EditOp::HdrScape(p) => Some(view_hdr_scape_controls(p)),
        EditOp::GlamourGlow(p) => Some(view_glamour_glow_controls(p)),
        EditOp::Drama(p) => Some(view_drama_controls(p)),
        EditOp::Vintage(p) => Some(view_vintage_controls(p)),
        EditOp::LensBlur(p) => Some(view_lens_blur_controls(p)),
        EditOp::MotionBlur(p) => Some(view_motion_blur_controls(p)),
        EditOp::DoubleExposure(p) => Some(view_double_exposure_controls(p)),
        EditOp::Retrolux(p) => Some(view_retrolux_controls(p)),
        EditOp::Grunge(p) => Some(view_grunge_controls(p)),
        EditOp::Portrait(p) => Some(view_portrait_controls(p)),
        EditOp::Frames(p) => Some(view_frames_controls(p)),
        EditOp::TextOverlay(p) => Some(view_text_overlay_controls(p)),
        EditOp::HeadPose(p) => Some(view_head_pose_controls(p)),
        _ => None,
    }
}

// -----------------------------------------------------------------------------
// Deliverable 2: Layer Mask & Smart Masking (4.0) Inspector
// -----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct MaskInspectorProps<'a> {
    pub node_id: Uuid,
    pub has_mask: bool,
    pub is_paint_mode: bool,
    pub is_eraser: bool,
    pub brush_radius: f32,
    pub brush_hardness: f32,
    pub target_opacity: f32,
    pub red_overlay: bool,
    pub node_title: &'a str,
}

impl Default for MaskInspectorProps<'static> {
    fn default() -> Self {
        Self {
            node_id: Uuid::nil(),
            has_mask: false,
            is_paint_mode: false,
            is_eraser: false,
            brush_radius: 0.10,
            brush_hardness: 0.50,
            target_opacity: 1.0,
            red_overlay: true,
            node_title: "",
        }
    }
}

impl<'a> MaskInspectorProps<'a> {
    pub fn new(node_id: Uuid) -> Self {
        Self {
            node_id,
            has_mask: false,
            is_paint_mode: false,
            is_eraser: false,
            brush_radius: 0.10,
            brush_hardness: 0.50,
            target_opacity: 1.0,
            red_overlay: true,
            node_title: "",
        }
    }

    pub fn with_title(mut self, title: &'a str) -> Self {
        self.node_title = title;
        self
    }
}

pub fn view_layer_mask_inspector<'a>(
    props: MaskInspectorProps<'a>,
) -> Element<'a, ToolEditorEvent> {
    let paint_active = props.is_paint_mode;

    // Header with Paint Mask toggle button
    let paint_toggle_btn = button(
        container(
            row![
                icon(
                    if paint_active {
                        Icon::Brush
                    } else {
                        Icon::Mask
                    },
                    13.0,
                    if paint_active {
                        Color::WHITE
                    } else {
                        palette::ACCENT
                    },
                ),
                text(if paint_active {
                    "Exit Paint"
                } else {
                    "Paint Mask"
                })
                .size(11)
                .color(if paint_active {
                    Color::WHITE
                } else {
                    palette::ACCENT
                }),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .align_x(Horizontal::Center),
    )
    .padding([6, 10])
    .style(if paint_active {
        theme::accent_button
    } else {
        theme::secondary_button
    })
    .width(Length::FillPortion(1))
    .on_press(ToolEditorEvent::MaskSetPaintMode {
        node_id: props.node_id,
        active: !paint_active,
    });

    let header_row = row![
        text(if props.node_title.is_empty() {
            "LAYER MASK"
        } else {
            props.node_title
        })
        .size(11)
        .color(palette::TEXT_MUTED),
        space::horizontal(),
        paint_toggle_btn,
    ]
    .align_y(Alignment::Center);

    // Segmented toggle: Paint vs Erase
    let paint_tab = button(
        container(text("Paint").size(11))
            .width(Length::Fill)
            .align_x(Horizontal::Center),
    )
    .padding([5, 8])
    .style(theme::segmented_tab_button(!props.is_eraser))
    .width(Length::FillPortion(1))
    .on_press(ToolEditorEvent::MaskSetEraser(false));

    let erase_tab = button(
        container(text("Erase").size(11))
            .width(Length::Fill)
            .align_x(Horizontal::Center),
    )
    .padding([5, 8])
    .style(theme::segmented_tab_button(props.is_eraser))
    .width(Length::FillPortion(1))
    .on_press(ToolEditorEvent::MaskSetEraser(true));

    let mode_tabs = container(row![paint_tab, erase_tab].spacing(2))
        .padding(2)
        .style(theme::sunken_track_style)
        .width(Length::Fill);

    // Red Overlay Checkbox
    let red_overlay_check = checkbox(props.red_overlay)
        .label("Red Mask Overlay")
        .style(theme::studio_checkbox)
        .size(14)
        .on_toggle(ToolEditorEvent::MaskToggleRedOverlay);

    // Quick Snapseed Brush Opacity step pills: 0%, 25%, 50%, 75%, 100%
    let opacity_steps = [
        (0.0_f32, "0%"),
        (0.25_f32, "25%"),
        (0.50_f32, "50%"),
        (0.75_f32, "75%"),
        (1.0_f32, "100%"),
    ];
    let mut opacity_row = row![].spacing(4);
    for &(val, label) in &opacity_steps {
        let is_sel = (props.target_opacity - val).abs() < 1e-2
            || (props.target_opacity - val * 100.0).abs() < 1e-2;
        let btn = button(
            container(text(label).size(10))
                .width(Length::Fill)
                .align_x(Horizontal::Center),
        )
        .padding([4, 4])
        .style(theme::pill_chip_button(is_sel))
        .width(Length::FillPortion(1))
        .on_press(ToolEditorEvent::MaskSetTargetOpacity(val));
        opacity_row = opacity_row.push(btn);
    }
    let opacity_section = column![
        text("BRUSH OPACITY").size(10).color(palette::TEXT_MUTED),
        opacity_row,
    ]
    .spacing(4);

    // Brush Radius (0.01..=0.30) & Hardness (0.0..=1.0) sliders
    let radius_slider = row![
        text("Size").size(11).color(palette::TEXT_MUTED),
        slider(0.01..=0.30_f32, props.brush_radius, |v| {
            ToolEditorEvent::MaskSetBrushRadius(v)
        })
        .step(0.01_f32)
        .style(theme::studio_slider(palette::ACCENT)),
        text(format!("{:.0}%", props.brush_radius * 100.0))
            .size(11)
            .color(palette::TEXT_MUTED),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let hardness_slider = row![
        text("Hard").size(11).color(palette::TEXT_MUTED),
        slider(0.0..=1.0_f32, props.brush_hardness, |v| {
            ToolEditorEvent::MaskSetBrushHardness(v)
        })
        .step(0.01_f32)
        .style(theme::studio_slider(palette::ACCENT)),
        text(format!("{:.0}%", props.brush_hardness * 100.0))
            .size(11)
            .color(palette::TEXT_MUTED),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    // Action row: Invert, Clear (0%), Fill (100%)
    let invert_btn = button(
        container(text("Invert").size(11))
            .width(Length::Fill)
            .align_x(Horizontal::Center),
    )
    .padding([5, 6])
    .style(theme::secondary_button)
    .width(Length::FillPortion(1))
    .on_press(ToolEditorEvent::MaskInvert(props.node_id));

    let clear_btn = button(
        container(text("Clear (0%)").size(11))
            .width(Length::Fill)
            .align_x(Horizontal::Center),
    )
    .padding([5, 6])
    .style(theme::subtle_danger_button)
    .width(Length::FillPortion(1))
    .on_press(ToolEditorEvent::MaskClear(props.node_id));

    let fill_btn = button(
        container(text("Fill (100%)").size(11))
            .width(Length::Fill)
            .align_x(Horizontal::Center),
    )
    .padding([5, 6])
    .style(theme::secondary_button)
    .width(Length::FillPortion(1))
    .on_press(ToolEditorEvent::MaskResetFull(props.node_id));

    let action_row = row![invert_btn, clear_btn, fill_btn].spacing(6);

    // Smart Mask (1-Tap Isolation)
    let smart_header = row![
        icon(Icon::Sparkles, 13.0, palette::AMBER),
        text("SMART MASK (1-TAP ISOLATION)").size(11).color(palette::TEXT_MUTED),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    let mut smart_col = column![smart_header].spacing(6);

    // All 7 SmartMaskKind variants: Sky, SubjectCenter, Background, Highlights, Midtones, Shadows, SkinTones
    let row1_kinds = [
        SmartMaskKind::Sky,
        SmartMaskKind::SubjectCenter,
        SmartMaskKind::Background,
    ];
    let mut row1 = row![].spacing(6);
    for kind in row1_kinds {
        let btn = button(
            container(text(kind.display_name()).size(11))
                .width(Length::Fill)
                .align_x(Horizontal::Center),
        )
        .padding([5, 6])
        .style(theme::secondary_button)
        .width(Length::FillPortion(1))
        .on_press(ToolEditorEvent::MaskApplySmart {
            node_id: props.node_id,
            kind,
        });
        row1 = row1.push(btn);
    }

    let row2_kinds = [
        SmartMaskKind::Highlights,
        SmartMaskKind::Midtones,
        SmartMaskKind::Shadows,
    ];
    let mut row2 = row![].spacing(6);
    for kind in row2_kinds {
        let btn = button(
            container(text(kind.display_name()).size(11))
                .width(Length::Fill)
                .align_x(Horizontal::Center),
        )
        .padding([5, 6])
        .style(theme::secondary_button)
        .width(Length::FillPortion(1))
        .on_press(ToolEditorEvent::MaskApplySmart {
            node_id: props.node_id,
            kind,
        });
        row2 = row2.push(btn);
    }

    let skin_kind = SmartMaskKind::SkinTones;
    let skin_btn = button(
        container(
            row![
                icon(Icon::Portrait, 12.0, palette::ACCENT),
                text("Skin Tones Isolation").size(11).color(palette::TEXT_PRIMARY),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .align_x(Horizontal::Center),
    )
    .padding([5, 8])
    .style(theme::secondary_button)
    .width(Length::Fill)
    .on_press(ToolEditorEvent::MaskApplySmart {
        node_id: props.node_id,
        kind: skin_kind,
    });

    smart_col = smart_col.push(row1).push(row2).push(skin_btn);

    container(
        column![
            header_row,
            mode_tabs,
            red_overlay_check,
            opacity_section,
            radius_slider,
            hardness_slider,
            action_row,
            smart_col,
        ]
        .spacing(10),
    )
    .padding(12)
    .style(theme::surface_card_style)
    .width(Length::Fill)
    .into()
}
