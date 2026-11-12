use iced::alignment::{Horizontal, Vertical};
use iced::widget::{
    button, checkbox, column, container, row, rule, scrollable, slider, space, stack, text,
    text_input,
};
use iced::{Alignment, Color, Element, Length};
use snap_core::config::{ExportFormat, ResizeLongEdge};
use snap_core::curves::{CurveChannelId, CurvePreset};
use snap_core::image_types::LutColorSpace;
use snap_core::looks::built_in_looks;
use snap_core::ops::{
    AspectRatio, BwColorFilter, BwPreset, EditOp, HslBand, ToolId,
};

use crate::app::{ActiveToolSession, Message, SnapApp, Tab};
use crate::canvas_shader::CanvasShaderProgram;
use crate::curves_widget::{CurvesEvent, CurvesWidget};
use crate::histogram_widget::HistogramWidget;
use crate::icons::{icon, Icon};
use crate::theme::{self, palette};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

pub fn tool_short_label(tool: ToolId) -> &'static str {
    match tool {
        ToolId::RawDevelop => "RAW",
        ToolId::TuneImage => "Tune",
        ToolId::Details => "Details",
        ToolId::Curves => "Curves",
        ToolId::WhiteBalance => "WB",
        ToolId::ColorHsl => "HSL",
        ToolId::Lut3d => "3D LUT",
        ToolId::BlackAndWhite => "B&W",
        ToolId::Vignette => "Vignette",
        ToolId::CropRotate => "Crop",
        ToolId::ColorGrade => "Grade",
        ToolId::Dehaze => "Dehaze",
        ToolId::TonalContrast => "Tonal",
        ToolId::Perspective => "Persp",
        ToolId::Expand => "Expand",
        ToolId::Selective => "Select",
        ToolId::Brush => "Brush",
        ToolId::Healing => "Heal",
        ToolId::Halation => "Halo",
        ToolId::Bloom => "Bloom",
        ToolId::GrainyFilm => "Film",
        ToolId::Noir => "Noir",
        ToolId::HdrScape => "HDR",
        ToolId::GlamourGlow => "Glow",
        ToolId::Drama => "Drama",
        ToolId::Vintage => "Vintage",
        ToolId::DoubleExposure => "Dbl Exp",
        ToolId::Retrolux => "Retrolux",
        ToolId::Grunge => "Grunge",
        ToolId::LensBlur => "Lens Blur",
        ToolId::MotionBlur => "Motion",
        ToolId::Portrait => "Portrait",
        ToolId::HeadPose => "Pose",
        ToolId::Frames => "Frames",
        ToolId::TextOverlay => "Text",
    }
}

pub fn look_accent_color(id: &str) -> Color {
    match id {
        "portrait" => Color::from_rgb(0.95, 0.45, 0.38),   // Warm Coral
        "pop" => Color::from_rgb(0.18, 0.65, 0.98),        // Vibrant Cyan-Blue
        "morning" => Color::from_rgb(0.96, 0.72, 0.18),    // Golden Amber
        "faded-matte" | "faded_matte" => Color::from_rgb(0.52, 0.70, 0.58), // Sage
        "silhouette" => Color::from_rgb(0.38, 0.42, 0.88), // Indigo
        "smooth" => Color::from_rgb(0.72, 0.48, 0.92),     // Soft Violet
        "fine-art" | "fine_art" => Color::from_rgb(0.80, 0.82, 0.86), // Monochrome Silver
        "structure" => Color::from_rgb(0.92, 0.58, 0.22),  // Warm Ochre
        "drama" => Color::from_rgb(0.85, 0.28, 0.28),      // Deep Crimson
        "vintage" => Color::from_rgb(0.82, 0.68, 0.45),    // Vintage Sepia
        "push-process" | "push_process" => Color::from_rgb(0.24, 0.82, 0.64), // Emerald
        _ => palette::ACCENT,
    }
}

fn hsl_band_color(band: HslBand) -> Color {
    match band {
        HslBand::Red => Color::from_rgb(0.92, 0.26, 0.26),
        HslBand::Orange => Color::from_rgb(0.96, 0.55, 0.16),
        HslBand::Yellow => Color::from_rgb(0.95, 0.85, 0.20),
        HslBand::Green => Color::from_rgb(0.20, 0.80, 0.40),
        HslBand::Cyan => Color::from_rgb(0.18, 0.80, 0.85),
        HslBand::Blue => Color::from_rgb(0.24, 0.54, 0.96),
        HslBand::Purple => Color::from_rgb(0.65, 0.35, 0.95),
        HslBand::Magenta => Color::from_rgb(0.90, 0.30, 0.70),
    }
}

fn hsl_band_name(band: HslBand) -> &'static str {
    match band {
        HslBand::Red => "Red",
        HslBand::Orange => "Orange",
        HslBand::Yellow => "Yellow",
        HslBand::Green => "Green",
        HslBand::Cyan => "Cyan",
        HslBand::Blue => "Blue",
        HslBand::Purple => "Purple",
        HslBand::Magenta => "Magenta",
    }
}

fn bw_filter_color(filter: BwColorFilter) -> Color {
    match filter {
        BwColorFilter::Neutral => Color::from_rgb(0.75, 0.75, 0.75),
        BwColorFilter::Red => Color::from_rgb(0.92, 0.26, 0.26),
        BwColorFilter::Orange => Color::from_rgb(0.96, 0.55, 0.16),
        BwColorFilter::Yellow => Color::from_rgb(0.95, 0.85, 0.20),
        BwColorFilter::Green => Color::from_rgb(0.20, 0.80, 0.40),
        BwColorFilter::Blue => Color::from_rgb(0.24, 0.54, 0.96),
    }
}

fn aspect_ratio_label(aspect: AspectRatio) -> &'static str {
    match aspect {
        AspectRatio::Free => "Free",
        AspectRatio::Original => "Original",
        AspectRatio::Square => "1:1",
        AspectRatio::R3x2 => "3:2",
        AspectRatio::R4x3 => "4:3",
        AspectRatio::R16x9 => "16:9",
        AspectRatio::R5x4 => "5:4",
        AspectRatio::R7x5 => "7:5",
        AspectRatio::Din => "DIN",
    }
}

// ---------------------------------------------------------------------------
// Root View Entrypoint
// ---------------------------------------------------------------------------

pub fn view(app: &SnapApp) -> Element<'_, Message> {
    if app.source_image.is_none() {
        return render_welcome_studio(app);
    }

    let top_bar = render_darkroom_top_bar(app);
    let photo_stage = render_photo_stage(app);

    let workspace_row: Element<'_, Message> = if app.config.show_sidebar {
        let inspector = render_inspector(app);
        row![
            photo_stage,
            container(inspector)
                .width(Length::Fixed(348.0))
                .height(Length::Fill)
                .style(theme::inspector_dock_style),
        ]
        .spacing(0)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    } else {
        photo_stage
    };

    container(
        column![
            top_bar,
            workspace_row,
        ]
        .spacing(0),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(theme::app_backdrop)
    .into()
}

// ---------------------------------------------------------------------------
// Mode A: Welcome Studio (app.source_image.is_none())
// ---------------------------------------------------------------------------

fn render_welcome_top_bar<'a>() -> Element<'a, Message> {
    let logo = icon(Icon::ApertureLogo, 20.0, palette::ACCENT);
    let brand = text("Snap").size(16).color(palette::TEXT_PRIMARY);
    let pill = container(
        text("RAW & LUT Studio").size(10).color(palette::TEXT_MUTED),
    )
    .padding([3, 8])
    .style(theme::badge_pill_style(palette::BG_SUNKEN, palette::BORDER_SUBTLE));

    let left = row![logo, brand, pill].spacing(10).align_y(Alignment::Center);

    let right = text("v0.1 / Linear Rec.2020 Vulkan")
        .size(11)
        .color(palette::TEXT_MUTED);

    container(
        row![
            left,
            space::horizontal(),
            right,
        ]
        .align_y(Alignment::Center),
    )
    .height(Length::Fixed(48.0))
    .padding([0, 14])
    .style(theme::top_bar_style)
    .width(Length::Fill)
    .into()
}

fn render_welcome_studio<'a>(app: &'a SnapApp) -> Element<'a, Message> {
    let top_bar = render_welcome_top_bar();

    let brand_badge = container(
        icon(Icon::ApertureLogo, 32.0, palette::ACCENT),
    )
    .padding([10, 14])
    .style(theme::badge_pill_style(palette::ACCENT_MUTED, palette::ACCENT));

    let title = text("Snap Studio").size(26).color(palette::TEXT_PRIMARY);
    let subtitle = text("Fast, non-destructive RAW, 3D LUT, and tactile photo editing on Vulkan.")
        .size(14)
        .color(palette::TEXT_SECONDARY);

    let open_btn = button(
        row![
            icon(Icon::FolderOpen, 16.0, Color::WHITE),
            text("Open Photo...").size(14).color(Color::WHITE),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .padding([11, 20])
    .style(theme::accent_button)
    .on_press(Message::OpenFileDialog);

    let sample_btn = button(
        row![
            icon(Icon::Sparkles, 16.0, palette::ACCENT),
            text("Try Sample HDR Scene").size(14).color(palette::ACCENT),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .padding([11, 20])
    .style(theme::secondary_button)
    .on_press(Message::LoadDemoScene);

    let cta_row = row![open_btn, sample_btn]
        .spacing(12)
        .align_y(Alignment::Center);

    let dropzone = container(
        row![
            icon(Icon::ImageFile, 18.0, palette::TEXT_MUTED),
            text("Drop any JPEG, PNG, TIFF, WebP, RAW (.DNG, .CR2, .ARW, .NEF), or .CUBE 3D LUT anywhere")
                .size(12)
                .color(palette::TEXT_MUTED),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    )
    .padding([14, 18])
    .style(theme::dropzone_card_style)
    .width(Length::Fill);

    let mut hero_col = column![
        brand_badge,
        title,
        subtitle,
        cta_row,
        dropzone,
    ]
    .spacing(22)
    .align_x(Alignment::Center);

    if !app.config.recent_files.is_empty() {
        let mut recents_col = column![
            text("RECENT PHOTOS").size(11).color(palette::TEXT_MUTED),
        ]
        .spacing(6);

        for path in app.config.recent_files.iter().take(4) {
            let filename = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("photo");
            let parent_dir = path
                .parent()
                .and_then(|p| p.to_str())
                .unwrap_or("");

            let item_btn = button(
                row![
                    icon(Icon::ImageFile, 14.0, palette::ACCENT),
                    text(filename).size(13).color(palette::TEXT_PRIMARY),
                    space::horizontal(),
                    text(parent_dir).size(11).color(palette::TEXT_MUTED),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .padding([8, 12])
            .style(theme::secondary_button)
            .width(Length::Fill)
            .on_press(Message::OpenRecentFile(path.clone()));

            recents_col = recents_col.push(item_btn);
        }

        hero_col = hero_col.push(recents_col);
    }

    let hero_card = container(hero_col)
        .max_width(580)
        .padding(36)
        .style(theme::welcome_hero_card);

    let center_stage = container(hero_card)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center);

    container(
        column![top_bar, center_stage].spacing(0),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(theme::app_backdrop)
    .into()
}

// ---------------------------------------------------------------------------
// Mode B: Darkroom Top Studio Bar (48px)
// ---------------------------------------------------------------------------

fn render_darkroom_top_bar<'a>(app: &'a SnapApp) -> Element<'a, Message> {
    let logo = icon(Icon::ApertureLogo, 20.0, palette::ACCENT);
    let brand = text("Snap").size(15).color(palette::TEXT_PRIMARY);

    let file_info_str = if let Some(ref path) = app.current_image_path {
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("Photo");
        let dims = if let Some(ref img) = app.source_image {
            format!("  {} x {}", img.width, img.height)
        } else {
            String::new()
        };
        format!("{name}{dims}")
    } else if let Some(ref img) = app.source_image {
        format!("Alpine Sunset HDR  {} x {}", img.width, img.height)
    } else {
        "Photo".to_string()
    };

    let file_pill = container(
        text(file_info_str).size(12).color(palette::TEXT_SECONDARY),
    )
    .padding([3, 8])
    .style(theme::badge_pill_style(palette::BG_SUNKEN, palette::BORDER_SUBTLE));

    let left_group = row![logo, brand, file_pill]
        .spacing(10)
        .align_y(Alignment::Center);

    let mut fav_row = row![].spacing(2).align_y(Alignment::Center);
    for fav_tool in &app.config.favorites {
        let is_active_tool = app
            .active_tool_session
            .as_ref()
            .map(|s| s.tool == *fav_tool)
            .unwrap_or(false);
        let icon_col = if is_active_tool {
            palette::ACCENT
        } else {
            palette::TEXT_SECONDARY
        };
        let text_col = if is_active_tool {
            palette::TEXT_PRIMARY
        } else {
            palette::TEXT_SECONDARY
        };

        let btn = button(
            container(
                row![
                    icon(Icon::for_tool(*fav_tool), 14.0, icon_col),
                    text(tool_short_label(*fav_tool)).size(11).color(text_col),
                ]
                .spacing(5)
                .align_y(Alignment::Center),
            )
            .align_x(Horizontal::Center),
        )
        .padding([5, 10])
        .style(theme::favorite_tab_button(is_active_tool))
        .on_press(Message::OpenTool(*fav_tool));

        fav_row = fav_row.push(btn);
    }

    let center_group = container(fav_row)
        .padding(3)
        .style(theme::sunken_track_style);

    let can_undo = app.active_tool_session.is_some() || app.document_history.can_undo();
    let can_redo = app.active_tool_session.is_some() || app.document_history.can_redo();

    let mut undo_btn = button(icon(
        Icon::Undo,
        15.0,
        if can_undo { palette::TEXT_PRIMARY } else { palette::TEXT_MUTED },
    ))
    .padding([5, 8])
    .style(theme::ghost_icon_button(false));
    if can_undo {
        undo_btn = undo_btn.on_press(Message::Undo);
    }

    let mut redo_btn = button(icon(
        Icon::Redo,
        15.0,
        if can_redo { palette::TEXT_PRIMARY } else { palette::TEXT_MUTED },
    ))
    .padding([5, 8])
    .style(theme::ghost_icon_button(false));
    if can_redo {
        redo_btn = redo_btn.on_press(Message::Redo);
    }

    let compare_icon = if app.bypass_edits { Icon::EyeOff } else { Icon::Eye };
    let compare_color = if app.bypass_edits { palette::AMBER } else { palette::TEXT_SECONDARY };
    let compare_btn = button(
        row![
            icon(compare_icon, 15.0, compare_color),
            text("Original").size(12).color(compare_color),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .padding([5, 10])
    .style(theme::ghost_icon_button(app.bypass_edits))
    .on_press(Message::ToggleBypassEdits);

    let node_count = app.document_history.current().nodes.len();
    let is_view_edits = app.current_tab == Tab::ViewEdits && app.active_tool_session.is_none();
    let edits_color = if is_view_edits { palette::ACCENT } else { palette::TEXT_SECONDARY };
    let edits_btn = button(
        row![
            icon(Icon::Layers, 15.0, edits_color),
            text(format!("{node_count}")).size(12).color(edits_color),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .padding([5, 10])
    .style(theme::ghost_icon_button(is_view_edits))
    .on_press(Message::SelectTab(Tab::ViewEdits));

    let open_btn = button(icon(Icon::FolderOpen, 15.0, palette::TEXT_SECONDARY))
        .padding([5, 8])
        .style(theme::ghost_icon_button(false))
        .on_press(Message::OpenFileDialog);

    let sidebar_toggle_btn = button(icon(
        Icon::SidebarRight,
        15.0,
        if app.config.show_sidebar { palette::ACCENT } else { palette::TEXT_MUTED },
    ))
    .padding([5, 8])
    .style(theme::ghost_icon_button(app.config.show_sidebar))
    .on_press(Message::ToggleSidebar);

    let export_msg = if app.current_tab == Tab::Export {
        Message::StartExport
    } else {
        Message::SelectTab(Tab::Export)
    };
    let export_btn = button(
        row![
            icon(Icon::Download, 14.0, Color::WHITE),
            text("Export").size(12).color(Color::WHITE),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .padding([6, 14])
    .style(theme::accent_button)
    .on_press(export_msg);

    let mut right_row = row![
        undo_btn,
        redo_btn,
        compare_btn,
        edits_btn,
        open_btn,
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    if app.current_image_path.is_some() {
        let save_btn = button(icon(Icon::Save, 15.0, palette::TEXT_SECONDARY))
            .padding([5, 8])
            .style(theme::ghost_icon_button(false))
            .on_press(Message::SaveSidecar);
        right_row = right_row.push(save_btn);
    }

    right_row = right_row.push(sidebar_toggle_btn);
    right_row = right_row.push(export_btn);

    container(
        row![
            left_group,
            space::horizontal(),
            center_group,
            space::horizontal(),
            right_row,
        ]
        .align_y(Alignment::Center),
    )
    .height(Length::Fixed(48.0))
    .padding([0, 14])
    .style(theme::top_bar_style)
    .width(Length::Fill)
    .into()
}

// ---------------------------------------------------------------------------
// Canvas Stack Layers
// ---------------------------------------------------------------------------

fn render_floating_top_overlay<'a>(app: &'a SnapApp) -> Element<'a, Message> {
    let mut col = column![].spacing(8).align_x(Alignment::Center);

    if app.bypass_edits {
        let amber_badge = container(
            text("ORIGINAL (UNEDITED)")
                .size(11)
                .color(palette::AMBER),
        )
        .padding([6, 14])
        .style(theme::badge_pill_style(palette::BG_SUNKEN, palette::AMBER));

        col = col.push(amber_badge);
    } else if let Some(ref session) = app.active_tool_session {
        let sliders = session.live_op.slider_params();
        if !sliders.is_empty() && session.selected_param_idx < sliders.len() {
            let (desc, val) = &sliders[session.selected_param_idx];

            let tool_icon = icon(Icon::for_tool(session.tool), 14.0, palette::ACCENT);
            let name_text = text(desc.name).size(12).color(palette::TEXT_PRIMARY);

            let val_str = if desc.unit == "%" {
                format!("{:.0}%", val)
            } else if desc.unit == "deg" {
                format!("{:+.*} deg", 1, val)
            } else if desc.unit == "EV" {
                format!("{:+.*} EV", 1, val)
            } else if (desc.step - 1.0).abs() < 1e-3 || desc.step >= 1.0 {
                format!("{:+.*}", 0, *val as i32)
            } else {
                format!("{:+.*}", 1, val)
            };
            let val_text = text(val_str).size(12).color(palette::ACCENT);

            let top_row = row![
                tool_icon,
                name_text,
                space::horizontal(),
                val_text,
            ]
            .spacing(8)
            .align_y(Alignment::Center);

            let range = desc.max - desc.min;
            let pct = if range > 1e-4 {
                ((*val - desc.min) / range).clamp(0.0, 1.0)
            } else {
                0.5
            };
            let fill_units = ((pct * 1000.0).round() as u16).max(1);
            let rest_units = 1000u16.saturating_sub(fill_units).max(1);

            let progress_bar = container(
                row![
                    container(text("").size(1))
                        .width(Length::FillPortion(fill_units))
                        .height(Length::Fixed(3.0))
                        .style(theme::badge_pill_style(palette::ACCENT, Color::TRANSPARENT)),
                    container(text("").size(1))
                        .width(Length::FillPortion(rest_units))
                        .height(Length::Fixed(3.0))
                        .style(theme::badge_pill_style(palette::BG_SUNKEN, Color::TRANSPARENT)),
                ]
            )
            .width(Length::Fill)
            .height(Length::Fixed(3.0));

            let pill = container(
                column![top_row, progress_bar].spacing(6),
            )
            .width(Length::Fixed(300.0))
            .padding([8, 16])
            .style(theme::floating_glass_pill);

            col = col.push(pill);

            if session.eyedropper_active {
                let hint = container(
                    text("Click neutral gray pixel on photo")
                        .size(11)
                        .color(palette::ACCENT),
                )
                .padding([4, 12])
                .style(theme::floating_glass_pill);
                col = col.push(hint);
            } else if session.vignette_center_mode {
                let hint = container(
                    text("Click photo to place vignette center")
                        .size(11)
                        .color(palette::ACCENT),
                )
                .padding([4, 12])
                .style(theme::floating_glass_pill);
                col = col.push(hint);
            }
        }
    }

    container(col)
        .width(Length::Fill)
        .padding(14)
        .align_x(Horizontal::Center)
        .into()
}

fn render_status_toast<'a>(app: &'a SnapApp) -> Element<'a, Message> {
    if let Some((ref msg, instant)) = app.status_message {
        if instant.elapsed().as_secs_f32() < 4.0 {
            let status_pill = container(
                text(msg.as_str()).size(11).color(palette::TEXT_SECONDARY),
            )
            .padding([6, 14])
            .style(theme::floating_glass_pill);

            return container(status_pill)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(14)
                .align_x(Horizontal::Right)
                .align_y(Vertical::Bottom)
                .into();
        }
    }
    column![].into()
}

fn render_vertical_picker_overlay<'a>(app: &'a SnapApp) -> Element<'a, Message> {
    let Some(ref session) = app.active_tool_session else {
        return column![].into();
    };

    if !session.vertical_picker_active {
        return column![].into();
    }

    let sliders = session.live_op.slider_params();
    if sliders.is_empty() {
        return column![].into();
    }

    let mut picker_col = column![].spacing(4);
    for (idx, (desc, val)) in sliders.iter().enumerate() {
        let is_sel = idx == session.selected_param_idx;
        let name_col = if is_sel { palette::ACCENT } else { palette::TEXT_SECONDARY };
        let val_col = if is_sel { palette::ACCENT } else { palette::TEXT_MUTED };

        let val_str = if desc.unit == "%" {
            format!("{:.0}%", val)
        } else if desc.unit == "deg" {
            format!("{:+.*} deg", 1, val)
        } else if desc.unit == "EV" {
            format!("{:+.*} EV", 1, val)
        } else if (desc.step - 1.0).abs() < 1e-3 || desc.step >= 1.0 {
            format!("{:+.*}", 0, *val as i32)
        } else {
            format!("{:+.*}", 1, val)
        };

        let row_content = row![
            text(desc.name).size(12).color(name_col),
            space::horizontal(),
            text(val_str).size(12).color(val_col),
        ]
        .align_y(Alignment::Center);

        let item = container(row_content)
            .width(Length::Fill)
            .padding([6, 10])
            .style(move |t| {
                if is_sel {
                    theme::badge_pill_style(palette::ACCENT_MUTED, palette::ACCENT)(t)
                } else {
                    theme::surface_card_style(t)
                }
            });

        picker_col = picker_col.push(item);
    }

    let hud_panel = container(picker_col)
        .width(Length::Fixed(250.0))
        .padding(12)
        .style(theme::floating_hud_panel);

    container(hud_panel)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
        .into()
}

fn render_viewport_dock<'a>(app: &'a SnapApp) -> Element<'a, Message> {
    let zoom_pct = format!("{:.0}%", app.viewport_params.zoom * 100.0);

    let zoom_out_btn = button(icon(Icon::ZoomOut, 14.0, palette::TEXT_SECONDARY))
        .padding([4, 6])
        .style(theme::ghost_icon_button(false))
        .on_press(Message::ZoomStep(0.8));

    let zoom_readout_btn = button(text(zoom_pct).size(12).color(palette::TEXT_PRIMARY))
        .padding([4, 8])
        .style(theme::ghost_icon_button(false))
        .on_press(Message::Zoom100);

    let zoom_in_btn = button(icon(Icon::ZoomIn, 14.0, palette::TEXT_SECONDARY))
        .padding([4, 6])
        .style(theme::ghost_icon_button(false))
        .on_press(Message::ZoomStep(1.25));

    let fit_btn = button(
        row![
            icon(Icon::Maximize, 13.0, palette::TEXT_SECONDARY),
            text("Fit").size(12).color(palette::TEXT_SECONDARY),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
    )
    .padding([4, 8])
    .style(theme::ghost_icon_button(false))
    .on_press(Message::FitView);

    let mut dock_row = row![zoom_out_btn, zoom_readout_btn, zoom_in_btn, fit_btn]
        .spacing(6)
        .align_y(Alignment::Center);

    if !app.config.show_sidebar && app.active_tool_session.is_some() {
        let cancel_btn = button(
            row![
                icon(Icon::Close, 13.0, palette::TEXT_SECONDARY),
                text("Cancel").size(12).color(palette::TEXT_SECONDARY),
            ]
            .spacing(4)
            .align_y(Alignment::Center),
        )
        .padding([4, 8])
        .style(theme::secondary_button)
        .on_press(Message::CancelActiveTool);

        let apply_btn = button(
            row![
                icon(Icon::Check, 13.0, Color::WHITE),
                text("Apply").size(12).color(Color::WHITE),
            ]
            .spacing(4)
            .align_y(Alignment::Center),
        )
        .padding([4, 8])
        .style(theme::accent_button)
        .on_press(Message::ApplyActiveTool);

        dock_row = dock_row.push(rule::vertical(1).style(theme::subtle_rule));
        dock_row = dock_row.push(cancel_btn);
        dock_row = dock_row.push(apply_btn);
    }

    let dock_pill = container(dock_row)
        .padding([4, 10])
        .style(theme::floating_glass_pill);

    container(
        column![
            space::vertical(),
            dock_pill,
        ]
        .align_x(Alignment::Center),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(14)
    .align_x(Horizontal::Center)
    .align_y(Vertical::Bottom)
    .into()
}

fn render_photo_stage<'a>(app: &'a SnapApp) -> Element<'a, Message> {
    let canvas_program = CanvasShaderProgram {
        image_id: Some(app.image_id),
        proxy_image: app.source_image.clone(),
        effective_stack: app.effective_stack(),
        bypass_edits: app.bypass_edits,
        viewport_params: app.viewport_params.clone(),
        needed_luts: app.lut_library.clone(),
        needed_secondary_images: app
            .secondary_images
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
        hist_slot: Some(app.hist_slot.clone()),
        hist_dirty: app.hist_dirty,
        active_tool: app.active_tool_session.as_ref().map(|s| s.tool),
        active_op: app
            .active_tool_session
            .as_ref()
            .map(|s| s.live_op.clone()),
        selected_param_idx: app
            .active_tool_session
            .as_ref()
            .map(|s| s.selected_param_idx)
            .unwrap_or(0),
        eyedropper_mode: app
            .active_tool_session
            .as_ref()
            .map(|s| s.eyedropper_active)
            .unwrap_or(false),
        vignette_center_mode: app
            .active_tool_session
            .as_ref()
            .map(|s| s.vignette_center_mode)
            .unwrap_or(false),
        _marker: std::marker::PhantomData,
    };

    let layer0: Element<'a, Message> = iced::widget::shader(canvas_program)
        .width(Length::Fill)
        .height(Length::Fill)
        .into();

    let mask_edit_node: Option<(uuid::Uuid, Option<&'a snap_core::mask::LayerMask>)> =
        if app.current_tab == Tab::ViewEdits {
            app.mask_edit_node_id.and_then(|id| {
                app.document_history
                    .current()
                    .nodes
                    .iter()
                    .find(|n| n.id == id)
                    .map(|n| (n.id, n.mask.as_ref()))
            })
        } else {
            None
        };

    let overlay_ctx = crate::canvas_overlays::CanvasOverlayContext {
        image_dims: app.source_image.as_ref().map(|img| (img.width, img.height)),
        viewport_params: app.viewport_params.clone(),
        active_tool: app.active_tool_session.as_ref().map(|s| s.tool),
        active_op: app.active_tool_session.as_ref().map(|s| &s.live_op),
        selective_add_pin_mode: app.selective_add_pin_mode,
        mask_edit_node,
        mask_paint_active: app.mask_paint_mode,
        mask_brush_radius: app.mask_brush_radius,
        show_red_mask_overlay: app.mask_red_overlay,
        bypass_edits: app.bypass_edits,
    };

    let overlay_layer: Element<'a, Message> =
        crate::canvas_overlays::view_canvas_overlay(overlay_ctx)
            .map(Message::CanvasOverlay);

    let layer1 = render_floating_top_overlay(app);
    let layer2 = render_vertical_picker_overlay(app);
    let layer3 = render_viewport_dock(app);
    let layer4 = render_status_toast(app);

    container(stack![layer0, overlay_layer, layer1, layer2, layer3, layer4])
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::canvas_backdrop)
        .into()
}

// ---------------------------------------------------------------------------
// Collapsible Histogram
// ---------------------------------------------------------------------------

fn render_collapsible_histogram<'a>(app: &'a SnapApp) -> Element<'a, Message> {
    let hist_icon = icon(Icon::Histogram, 14.0, palette::TEXT_SECONDARY);
    let hist_title = text("HISTOGRAM").size(11).color(palette::TEXT_SECONDARY);

    let r_dot = text("R").size(10).color(Color::from_rgb(0.92, 0.28, 0.28));
    let g_dot = text("G").size(10).color(Color::from_rgb(0.24, 0.82, 0.42));
    let b_dot = text("B").size(10).color(Color::from_rgb(0.24, 0.54, 0.96));
    let l_dot = text("L").size(10).color(palette::TEXT_MUTED);
    let legend = row![r_dot, g_dot, b_dot, l_dot].spacing(6);

    let chevron_icon = if app.show_histogram {
        Icon::ChevronUp
    } else {
        Icon::ChevronDown
    };
    let toggle_btn = button(icon(chevron_icon, 13.0, palette::TEXT_MUTED))
        .padding([2, 4])
        .style(theme::ghost_icon_button(false))
        .on_press(Message::ToggleHistogram);

    let header_row = row![
        hist_icon,
        hist_title,
        space::horizontal(),
        legend,
        toggle_btn,
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let header_container = container(header_row).padding([10, 14]);

    if app.show_histogram {
        let hist_canvas = container(
            iced::widget::canvas(HistogramWidget::new(&app.histogram))
                .width(Length::Fill)
                .height(Length::Fixed(78.0)),
        )
        .padding([0, 14]);

        column![
            header_container,
            hist_canvas,
            space::vertical().height(8),
        ]
        .into()
    } else {
        header_container.into()
    }
}

// ---------------------------------------------------------------------------
// Right Studio Inspector
// ---------------------------------------------------------------------------

fn render_inspector<'a>(app: &'a SnapApp) -> Element<'a, Message> {
    let hist_section = render_collapsible_histogram(app);
    let divider = rule::horizontal(1).style(theme::subtle_rule);

    let body_section: Element<'a, Message> = if let Some(ref session) = app.active_tool_session {
        render_active_tool_panel(app, session)
    } else {
        render_4tab_inspector(app)
    };

    column![
        hist_section,
        divider,
        container(body_section).width(Length::Fill).height(Length::Fill),
    ]
    .spacing(0)
    .into()
}

// ---------------------------------------------------------------------------
// Active Tool Panel
// ---------------------------------------------------------------------------

fn render_active_tool_panel<'a>(
    app: &'a SnapApp,
    session: &'a ActiveToolSession,
) -> Element<'a, Message> {
    let back_btn = button(icon(Icon::ChevronLeft, 16.0, palette::TEXT_SECONDARY))
        .padding([4, 6])
        .style(theme::ghost_icon_button(false))
        .on_press(Message::CancelActiveTool);

    let tool_icon = icon(Icon::for_tool(session.tool), 17.0, palette::ACCENT);
    let tool_title = text(session.tool.display_name())
        .size(15)
        .color(palette::TEXT_PRIMARY);

    let mut undo_btn = button(icon(
        Icon::Undo,
        14.0,
        if session.history.can_undo() { palette::TEXT_PRIMARY } else { palette::TEXT_MUTED },
    ))
    .padding([4, 6])
    .style(theme::ghost_icon_button(false));
    if session.history.can_undo() {
        undo_btn = undo_btn.on_press(Message::ToolUndo);
    }

    let mut redo_btn = button(icon(
        Icon::Redo,
        14.0,
        if session.history.can_redo() { palette::TEXT_PRIMARY } else { palette::TEXT_MUTED },
    ))
    .padding([4, 6])
    .style(theme::ghost_icon_button(false));
    if session.history.can_redo() {
        redo_btn = redo_btn.on_press(Message::ToolRedo);
    }

    let header_row = container(
        row![
            back_btn,
            tool_icon,
            tool_title,
            space::horizontal(),
            undo_btn,
            redo_btn,
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .padding([12, 14]);

    let controls_content = match session.tool {
        ToolId::TuneImage => render_tune_image_controls(session),
        ToolId::WhiteBalance => render_wb_controls(session),
        ToolId::Curves => render_curves_controls(app, session),
        ToolId::ColorHsl => render_hsl_controls(session),
        ToolId::Lut3d => render_lut_controls(app, session),
        ToolId::BlackAndWhite => render_bw_controls(session),
        ToolId::Vignette => render_vignette_controls(session),
        ToolId::CropRotate => render_crop_controls(app, session),
        _ => {
            if let Some(custom) = crate::tool_editors_ext::view_tool_custom_controls(
                &session.live_op,
                app.selective_add_pin_mode,
            ) {
                let sliders = render_standard_sliders(session);
                column![
                    custom.map(Message::ToolEditorExt),
                    sliders,
                ]
                .spacing(12)
                .into()
            } else {
                render_standard_sliders(session)
            }
        }
    };

    let controls_scroll = scrollable(
        container(controls_content)
            .padding([10, 14])
            .width(Length::Fill),
    )
    .style(theme::thin_scrollable)
    .height(Length::Fill);

    let cancel_btn = button(
        container(
            row![
                icon(Icon::Close, 14.0, palette::TEXT_SECONDARY),
                text("Cancel").size(13).color(palette::TEXT_SECONDARY),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .align_x(Horizontal::Center),
    )
    .padding([9, 14])
    .style(theme::secondary_button)
    .width(Length::Fill)
    .on_press(Message::CancelActiveTool);

    let apply_btn = button(
        container(
            row![
                icon(Icon::Check, 14.0, Color::WHITE),
                text("Apply").size(13).color(Color::WHITE),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .align_x(Horizontal::Center),
    )
    .padding([9, 14])
    .style(theme::accent_button)
    .width(Length::Fill)
    .on_press(Message::ApplyActiveTool);

    let footer = container(
        row![cancel_btn, apply_btn].spacing(10),
    )
    .padding([12, 14]);

    column![
        header_row,
        rule::horizontal(1).style(theme::subtle_rule),
        container(controls_scroll).width(Length::Fill).height(Length::Fill),
        rule::horizontal(1).style(theme::subtle_rule),
        footer,
    ]
    .spacing(0)
    .into()
}

// ---------------------------------------------------------------------------
// Standard Sliders
// ---------------------------------------------------------------------------

fn render_standard_sliders<'a>(session: &'a ActiveToolSession) -> Element<'a, Message> {
    let mut list = column![].spacing(10);
    let sliders = session.live_op.slider_params();

    for (idx, (desc, val)) in sliders.iter().enumerate() {
        let is_selected = idx == session.selected_param_idx;
        let name_col = if is_selected {
            palette::ACCENT
        } else {
            palette::TEXT_SECONDARY
        };

        let val_str = if desc.unit == "%" {
            format!("{:.0}%", val)
        } else if desc.unit == "deg" {
            format!("{:+.*} deg", 1, val)
        } else if desc.unit == "EV" {
            format!("{:+.*} EV", 1, val)
        } else if (desc.step - 1.0).abs() < 1e-3 || desc.step >= 1.0 {
            format!("{:+.*}", 0, *val as i32)
        } else {
            format!("{:+.*}", 1, val)
        };

        let label_btn = button(text(desc.name).size(12).color(name_col))
            .style(theme::ghost_icon_button(false))
            .padding([2, 4])
            .on_press(Message::SelectSliderParam(idx));

        let badge = container(
            text(val_str.trim().to_string())
                .size(11)
                .color(if is_selected { palette::ACCENT } else { palette::TEXT_PRIMARY }),
        )
        .padding([2, 6])
        .style(theme::badge_pill_style(palette::BG_SUNKEN, palette::BORDER_SUBTLE));

        let head_row = row![
            label_btn,
            space::horizontal(),
            badge,
        ]
        .align_y(Alignment::Center);

        let s = slider(desc.min..=desc.max, *val, move |v| {
            Message::SetSliderParamLive(idx, v)
        })
        .step(desc.step)
        .on_release(Message::CanvasGestureEnd)
        .style(theme::studio_slider(palette::ACCENT));

        list = list.push(column![head_row, s].spacing(4));
    }

    list.into()
}

// ---------------------------------------------------------------------------
// Tool-Specific Control Panels
// ---------------------------------------------------------------------------

fn render_tune_image_controls<'a>(session: &'a ActiveToolSession) -> Element<'a, Message> {
    let auto_btn = button(
        container(
            row![
                icon(Icon::Sparkles, 15.0, palette::ACCENT),
                text("Auto-Adjust Exposure & Tone").size(12).color(palette::ACCENT),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .align_x(Horizontal::Center),
    )
    .padding([8, 14])
    .style(theme::secondary_button)
    .width(Length::Fill)
    .on_press(Message::TuneAuto);

    column![
        auto_btn,
        render_standard_sliders(session),
    ]
    .spacing(14)
    .into()
}

fn render_wb_controls<'a>(session: &'a ActiveToolSession) -> Element<'a, Message> {
    let wb_btn = button(
        container(
            row![
                icon(
                    Icon::Pipette,
                    14.0,
                    if session.eyedropper_active { palette::ACCENT } else { palette::TEXT_SECONDARY },
                ),
                text(if session.eyedropper_active {
                    "Eyedropper Active: Click Canvas"
                } else {
                    "Pick Neutral Gray"
                })
                .size(12)
                .color(if session.eyedropper_active { palette::ACCENT } else { palette::TEXT_SECONDARY }),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .align_x(Horizontal::Center),
    )
    .padding([6, 12])
    .style(theme::pill_chip_button(session.eyedropper_active))
    .width(Length::Fill)
    .on_press(Message::ToggleEyedropper);

    column![
        wb_btn,
        render_standard_sliders(session),
    ]
    .spacing(14)
    .into()
}

fn render_curves_controls<'a>(
    app: &'a SnapApp,
    session: &'a ActiveToolSession,
) -> Element<'a, Message> {
    let channels = [
        (CurveChannelId::Rgb, "RGB"),
        (CurveChannelId::Red, "Red"),
        (CurveChannelId::Green, "Green"),
        (CurveChannelId::Blue, "Blue"),
        (CurveChannelId::Luminance, "Luma"),
    ];

    let mut ch_row = row![].spacing(4);
    for (ch_id, label) in channels {
        let is_sel = session.current_curve_channel == ch_id;
        let btn = button(text(label).size(11))
            .padding([4, 8])
            .style(theme::pill_chip_button(is_sel))
            .on_press(Message::CurvesWidgetEvent(CurvesEvent::SelectChannel(ch_id)));
        ch_row = ch_row.push(btn);
    }

    let EditOp::Curves(curves_params) = &session.live_op else {
        return text("No curves op active").into();
    };

    let curves_canvas = iced::widget::canvas(CurvesWidget::new(
        curves_params,
        session.current_curve_channel,
        &app.histogram,
        Message::CurvesWidgetEvent,
    ))
    .width(Length::Fill)
    .height(Length::Fixed(236.0));

    let reset_btn = button(
        text("Reset Channel").size(11).color(palette::TEXT_SECONDARY),
    )
    .padding([4, 10])
    .style(theme::secondary_button)
    .on_press(Message::CurvesWidgetEvent(CurvesEvent::ResetChannel(session.current_curve_channel)));

    let presets = [
        (CurvePreset::Neutral, "Neutral"),
        (CurvePreset::SoftContrast, "Soft Contrast"),
        (CurvePreset::HardContrast, "Hard Contrast"),
        (CurvePreset::Fade, "Fade"),
        (CurvePreset::Punch, "Punch"),
        (CurvePreset::CrossProcess, "Cross Process"),
        (CurvePreset::Brighten, "Brighten"),
        (CurvePreset::Darken, "Darken"),
        (CurvePreset::HighKey, "High Key"),
        (CurvePreset::LowKey, "Low Key"),
    ];

    let mut presets_col = column![
        text("CURVE PRESETS").size(11).color(palette::TEXT_MUTED),
    ]
    .spacing(6);

    for chunk in presets.chunks(2) {
        let mut p_row = row![].spacing(6);
        for (p, p_label) in chunk {
            let p_btn = button(
                container(text(*p_label).size(11))
                    .width(Length::Fill)
                    .align_x(Horizontal::Center),
            )
            .padding([4, 8])
            .style(theme::secondary_button)
            .width(Length::FillPortion(1))
            .on_press(Message::CurvesWidgetEvent(CurvesEvent::ApplyPreset(*p)));
            p_row = p_row.push(p_btn);
        }
        presets_col = presets_col.push(p_row);
    }

    column![
        ch_row,
        container(curves_canvas).width(Length::Fill),
        reset_btn,
        presets_col,
    ]
    .spacing(12)
    .into()
}

fn render_hsl_controls<'a>(session: &'a ActiveToolSession) -> Element<'a, Message> {
    let bands = [
        HslBand::Red,
        HslBand::Orange,
        HslBand::Yellow,
        HslBand::Green,
        HslBand::Cyan,
        HslBand::Blue,
        HslBand::Purple,
        HslBand::Magenta,
    ];

    let mut band_row = row![].spacing(6).align_y(Alignment::Center);
    for band in bands {
        let is_sel = session.current_hsl_band == band;
        let dot_col = hsl_band_color(band);
        let dot_btn = button(text("").size(1))
            .padding([6, 6])
            .style(theme::color_dot_button(dot_col, is_sel))
            .on_press(Message::SelectHslBand(band));
        band_row = band_row.push(dot_btn);
    }

    let active_band = session.current_hsl_band;
    let active_col = hsl_band_color(active_band);
    let active_badge = container(
        text(format!("{} Band", hsl_band_name(active_band)))
            .size(11)
            .color(active_col),
    )
    .padding([3, 10])
    .style(theme::badge_pill_style(palette::BG_SUNKEN, active_col));

    let EditOp::ColorHsl(hsl) = &session.live_op else {
        return text("No HSL active").into();
    };
    let shift = hsl.bands[active_band as usize];

    let hue_val_badge = container(
        text(format!("{:+.*}", 0, shift.hue as i32))
            .size(11)
            .color(active_col),
    )
    .padding([2, 6])
    .style(theme::badge_pill_style(palette::BG_SUNKEN, palette::BORDER_SUBTLE));

    let hue_slider = column![
        row![
            text("Hue").size(12).color(palette::TEXT_PRIMARY),
            space::horizontal(),
            hue_val_badge,
        ]
        .align_y(Alignment::Center),
        slider(-100.0..=100.0, shift.hue, |v| Message::SetHslValue(0, v))
            .step(1.0_f32)
            .on_release(Message::CanvasGestureEnd)
            .style(theme::studio_slider(active_col)),
    ]
    .spacing(4);

    let sat_val_badge = container(
        text(format!("{:+.*}", 0, shift.saturation as i32))
            .size(11)
            .color(active_col),
    )
    .padding([2, 6])
    .style(theme::badge_pill_style(palette::BG_SUNKEN, palette::BORDER_SUBTLE));

    let sat_slider = column![
        row![
            text("Saturation").size(12).color(palette::TEXT_PRIMARY),
            space::horizontal(),
            sat_val_badge,
        ]
        .align_y(Alignment::Center),
        slider(-100.0..=100.0, shift.saturation, |v| Message::SetHslValue(1, v))
            .step(1.0_f32)
            .on_release(Message::CanvasGestureEnd)
            .style(theme::studio_slider(active_col)),
    ]
    .spacing(4);

    let lum_val_badge = container(
        text(format!("{:+.*}", 0, shift.luminance as i32))
            .size(11)
            .color(active_col),
    )
    .padding([2, 6])
    .style(theme::badge_pill_style(palette::BG_SUNKEN, palette::BORDER_SUBTLE));

    let lum_slider = column![
        row![
            text("Luminance").size(12).color(palette::TEXT_PRIMARY),
            space::horizontal(),
            lum_val_badge,
        ]
        .align_y(Alignment::Center),
        slider(-100.0..=100.0, shift.luminance, |v| Message::SetHslValue(2, v))
            .step(1.0_f32)
            .on_release(Message::CanvasGestureEnd)
            .style(theme::studio_slider(active_col)),
    ]
    .spacing(4);

    column![
        container(band_row).width(Length::Fill).align_x(Horizontal::Center),
        container(active_badge).width(Length::Fill).align_x(Horizontal::Center),
        hue_slider,
        sat_slider,
        lum_slider,
    ]
    .spacing(14)
    .into()
}

fn render_lut_controls<'a>(app: &'a SnapApp, session: &'a ActiveToolSession) -> Element<'a, Message> {
    let import_btn = button(
        container(
            row![
                icon(Icon::FolderOpen, 15.0, palette::ACCENT),
                text("Import .cube / .3dl / Hald PNG...").size(12).color(palette::ACCENT),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .align_x(Horizontal::Center),
    )
    .padding([8, 14])
    .style(theme::secondary_button)
    .width(Length::Fill)
    .on_press(Message::ImportLutDialog);

    let EditOp::Lut3d(ref lut_params) = session.live_op else {
        return text("No LUT active").into();
    };

    let strength_badge = container(
        text(format!("{:.0}%", lut_params.strength)).size(11).color(palette::ACCENT),
    )
    .padding([2, 6])
    .style(theme::badge_pill_style(palette::BG_SUNKEN, palette::BORDER_SUBTLE));

    let strength_slider = column![
        row![
            text("Strength").size(12).color(palette::TEXT_PRIMARY),
            space::horizontal(),
            strength_badge,
        ]
        .align_y(Alignment::Center),
        slider(0.0..=100.0, lut_params.strength, Message::SetLutStrength)
            .step(1.0_f32)
            .on_release(Message::CanvasGestureEnd)
            .style(theme::studio_slider(palette::ACCENT)),
    ]
    .spacing(4);

    let spaces = [
        (LutColorSpace::Srgb, "sRGB"),
        (LutColorSpace::Rec709, "Rec.709"),
        (LutColorSpace::LinearRec2020, "Linear Rec.2020"),
        (LutColorSpace::AppleLog, "Apple Log"),
        (LutColorSpace::SonySLog3, "Sony S-Log3"),
        (LutColorSpace::CanonLog2, "Canon Log2"),
    ];

    let mut space_pills = column![
        text("INPUT COLOR SPACE").size(11).color(palette::TEXT_MUTED),
    ]
    .spacing(6);

    for chunk in spaces.chunks(3) {
        let mut p_row = row![].spacing(6);
        for (space, label) in chunk {
            let is_sel = lut_params.input_space == *space;
            let btn = button(text(*label).size(11))
                .padding([4, 6])
                .style(theme::pill_chip_button(is_sel))
                .width(Length::FillPortion(1))
                .on_press(Message::SetLutSpace(*space));
            p_row = p_row.push(btn);
        }
        space_pills = space_pills.push(p_row);
    }

    let mut luts_col = column![
        text("LOADED 3D LUTS").size(11).color(palette::TEXT_MUTED),
    ]
    .spacing(6);

    if app.lut_library.is_empty() {
        luts_col = luts_col.push(
            text("No 3D LUTs loaded. Click Import to add .cube or .3dl files!")
                .size(11)
                .color(palette::TEXT_MUTED),
        );
    } else {
        for lut in &app.lut_library {
            let is_sel = lut_params.lut_id == lut.id;

            let lut_card = button(
                row![
                    icon(Icon::Cube3D, 14.0, if is_sel { palette::ACCENT } else { palette::TEXT_MUTED }),
                    text(&lut.name).size(12).color(if is_sel { palette::TEXT_PRIMARY } else { palette::TEXT_SECONDARY }),
                    space::horizontal(),
                    container(text("33x33").size(10).color(palette::TEXT_MUTED))
                        .padding([2, 5])
                        .style(theme::badge_pill_style(palette::BG_SUNKEN, palette::BORDER_SUBTLE)),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .padding([7, 10])
            .style(theme::tool_tile_button(is_sel))
            .width(Length::Fill)
            .on_press(Message::SelectLut(lut.id.clone()));

            luts_col = luts_col.push(lut_card);
        }
    }

    column![
        import_btn,
        strength_slider,
        space_pills,
        luts_col,
    ]
    .spacing(14)
    .into()
}

fn render_bw_controls<'a>(session: &'a ActiveToolSession) -> Element<'a, Message> {
    let presets = [
        (BwPreset::Neutral, "Neutral"),
        (BwPreset::Contrast, "Contrast"),
        (BwPreset::Film, "Film"),
        (BwPreset::DarkSky, "Dark Sky"),
        (BwPreset::Bright, "Bright"),
        (BwPreset::Dark, "Dark"),
    ];

    let EditOp::BlackAndWhite(ref bw) = session.live_op else {
        return text("No B&W active").into();
    };

    let mut presets_col = column![
        text("B&W PRESETS").size(11).color(palette::TEXT_MUTED),
    ]
    .spacing(6);

    for chunk in presets.chunks(2) {
        let mut p_row = row![].spacing(6);
        for (p, label) in chunk {
            let is_sel = bw.preset == *p;
            let btn = button(
                container(text(*label).size(11))
                    .width(Length::Fill)
                    .align_x(Horizontal::Center),
            )
            .padding([5, 8])
            .style(theme::pill_chip_button(is_sel))
            .width(Length::FillPortion(1))
            .on_press(Message::SelectBwPreset(*p));
            p_row = p_row.push(btn);
        }
        presets_col = presets_col.push(p_row);
    }

    let filters = [
        (BwColorFilter::Neutral, "Neutral"),
        (BwColorFilter::Red, "Red"),
        (BwColorFilter::Orange, "Orange"),
        (BwColorFilter::Yellow, "Yellow"),
        (BwColorFilter::Green, "Green"),
        (BwColorFilter::Blue, "Blue"),
    ];

    let mut filter_row = row![].spacing(6).align_y(Alignment::Center);
    for (f, _label) in filters {
        let is_sel = bw.color_filter == f;
        let dot_col = bw_filter_color(f);
        let btn = button(text("").size(1))
            .padding([6, 6])
            .style(theme::color_dot_button(dot_col, is_sel))
            .on_press(Message::SelectBwFilter(f));
        filter_row = filter_row.push(btn);
    }

    let filter_section = column![
        text("OPTICAL COLOR FILTERS").size(11).color(palette::TEXT_MUTED),
        container(filter_row).width(Length::Fill).align_x(Horizontal::Center),
    ]
    .spacing(8);

    column![
        presets_col,
        filter_section,
        render_standard_sliders(session),
    ]
    .spacing(14)
    .into()
}

fn render_vignette_controls<'a>(session: &'a ActiveToolSession) -> Element<'a, Message> {
    let place_center_btn = button(
        container(
            row![
                icon(
                    Icon::Crosshair,
                    14.0,
                    if session.vignette_center_mode { palette::ACCENT } else { palette::TEXT_SECONDARY },
                ),
                text(if session.vignette_center_mode {
                    "Click Photo to Place Center"
                } else {
                    "Place Center on Canvas"
                })
                .size(12)
                .color(if session.vignette_center_mode { palette::ACCENT } else { palette::TEXT_SECONDARY }),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .align_x(Horizontal::Center),
    )
    .padding([7, 12])
    .style(theme::pill_chip_button(session.vignette_center_mode))
    .width(Length::Fill)
    .on_press(Message::ToggleVignetteCenter);

    column![
        place_center_btn,
        render_standard_sliders(session),
    ]
    .spacing(14)
    .into()
}

fn render_crop_controls<'a>(
    app: &'a SnapApp,
    session: &'a ActiveToolSession,
) -> Element<'a, Message> {
    let rot_ccw = button(icon(Icon::RotateCcw, 15.0, palette::TEXT_SECONDARY))
        .padding([6, 10])
        .style(theme::ghost_icon_button(false))
        .on_press(Message::RotateCcw);

    let rot_cw = button(icon(Icon::RotateCw, 15.0, palette::TEXT_SECONDARY))
        .padding([6, 10])
        .style(theme::ghost_icon_button(false))
        .on_press(Message::RotateCw);

    let flip_h = button(icon(Icon::FlipH, 15.0, palette::TEXT_SECONDARY))
        .padding([6, 10])
        .style(theme::ghost_icon_button(false))
        .on_press(Message::FlipH);

    let flip_v = button(icon(Icon::FlipV, 15.0, palette::TEXT_SECONDARY))
        .padding([6, 10])
        .style(theme::ghost_icon_button(false))
        .on_press(Message::FlipV);

    let swap_aspect = button(icon(Icon::Crop, 15.0, palette::TEXT_SECONDARY))
        .padding([6, 10])
        .style(theme::ghost_icon_button(false))
        .on_press(Message::SwapCropAspect);

    let transform_row = row![rot_ccw, rot_cw, flip_h, flip_v, swap_aspect]
        .spacing(6)
        .align_y(Alignment::Center);

    let aspects = [
        AspectRatio::Free,
        AspectRatio::Original,
        AspectRatio::Square,
        AspectRatio::R3x2,
        AspectRatio::R4x3,
        AspectRatio::R16x9,
        AspectRatio::R5x4,
        AspectRatio::R7x5,
        AspectRatio::Din,
    ];

    let mut aspect_grid = column![
        text("ASPECT RATIO").size(11).color(palette::TEXT_MUTED),
    ]
    .spacing(6);

    for chunk in aspects.chunks(3) {
        let mut a_row = row![].spacing(6);
        for a in chunk {
            let is_sel = app.viewport_params.crop.aspect == *a;
            let btn = button(
                container(text(aspect_ratio_label(*a)).size(11))
                    .width(Length::Fill)
                    .align_x(Horizontal::Center),
            )
            .padding([5, 6])
            .style(theme::pill_chip_button(is_sel))
            .width(Length::FillPortion(1))
            .on_press(Message::SetCropAspect(*a));
            a_row = a_row.push(btn);
        }
        aspect_grid = aspect_grid.push(a_row);
    }

    let reset_btn = button(
        container(text("Reset Crop").size(12).color(palette::TEXT_SECONDARY))
            .width(Length::Fill)
            .align_x(Horizontal::Center),
    )
    .padding([6, 12])
    .style(theme::secondary_button)
    .width(Length::Fill)
    .on_press(Message::ResetCrop);

    column![
        container(transform_row).width(Length::Fill).align_x(Horizontal::Center),
        aspect_grid,
        render_standard_sliders(session),
        reset_btn,
    ]
    .spacing(14)
    .into()
}

// ---------------------------------------------------------------------------
// 4-Tab Inspector (Looks | Tools | Edits | Export)
// ---------------------------------------------------------------------------

fn render_4tab_inspector<'a>(app: &'a SnapApp) -> Element<'a, Message> {
    let tab_bar = row![
        button(container(text("Looks").size(12)).width(Length::Fill).align_x(Horizontal::Center))
            .style(theme::segmented_tab_button(app.current_tab == Tab::Looks))
            .width(Length::FillPortion(1))
            .padding([6, 8])
            .on_press(Message::SelectTab(Tab::Looks)),
        button(container(text("Tools").size(12)).width(Length::Fill).align_x(Horizontal::Center))
            .style(theme::segmented_tab_button(app.current_tab == Tab::Tools))
            .width(Length::FillPortion(1))
            .padding([6, 8])
            .on_press(Message::SelectTab(Tab::Tools)),
        button(container(text("Edits").size(12)).width(Length::Fill).align_x(Horizontal::Center))
            .style(theme::segmented_tab_button(app.current_tab == Tab::ViewEdits))
            .width(Length::FillPortion(1))
            .padding([6, 8])
            .on_press(Message::SelectTab(Tab::ViewEdits)),
        button(container(text("Export").size(12)).width(Length::Fill).align_x(Horizontal::Center))
            .style(theme::segmented_tab_button(app.current_tab == Tab::Export))
            .width(Length::FillPortion(1))
            .padding([6, 8])
            .on_press(Message::SelectTab(Tab::Export)),
    ]
    .spacing(2);

    let switcher = container(
        container(tab_bar)
            .padding(3)
            .style(theme::sunken_track_style),
    )
    .padding([8, 14]);

    let tab_content: Element<'a, Message> = match app.current_tab {
        Tab::Looks => render_looks_tab(app),
        Tab::Tools => render_tools_tab(app),
        Tab::ViewEdits => render_view_edits_tab(app),
        Tab::Export => render_export_tab(app),
    };

    let scroll_content = scrollable(
        container(tab_content)
            .padding([6, 14])
            .width(Length::Fill),
    )
    .style(theme::thin_scrollable)
    .height(Length::Fill);

    column![
        switcher,
        container(scroll_content).width(Length::Fill).height(Length::Fill),
    ]
    .spacing(0)
    .into()
}

// ---------------------------------------------------------------------------
// Tab: Tools (2-Column Grid)
// ---------------------------------------------------------------------------

fn render_tools_tab<'a>(app: &'a SnapApp) -> Element<'a, Message> {
    let is_raw = app
        .current_image_path
        .as_ref()
        .map(|p| snap_io::is_raw_path(p))
        .unwrap_or(false);

    let mut main_col = column![].spacing(16);

    for &category in snap_core::ops::ToolCategory::ALL {
        let tools = ToolId::in_category(category);
        let cat_title = category.display_name().to_uppercase();
        let cat_count = tools.len();

        let header_row = row![
            text(cat_title).size(11).color(palette::TEXT_MUTED),
            space::horizontal(),
            container(
                text(format!("{cat_count}"))
                    .size(9)
                    .color(palette::TEXT_SECONDARY),
            )
            .padding([2, 6])
            .style(theme::badge_pill_style(palette::BG_SUNKEN, palette::BORDER_SUBTLE)),
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        let mut cat_col = column![header_row].spacing(8);

        for chunk in tools.chunks(2) {
            let mut pair_row = row![].spacing(8);
            for &tool in chunk {
                let is_fav = app.config.favorites.contains(&tool);
                let fav_icon = if is_fav { Icon::StarFilled } else { Icon::Star };
                let fav_col = if is_fav { palette::AMBER } else { palette::TEXT_MUTED };

                let is_active = app
                    .active_tool_session
                    .as_ref()
                    .map(|s| s.tool == tool)
                    .unwrap_or(false);

                let is_raw_develop = tool == ToolId::RawDevelop;
                let raw_badge: Option<Element<'a, Message>> = if is_raw_develop {
                    let badge_text = if is_raw { "RAW" } else { "RAW ONLY" };
                    let badge_col = if is_raw { palette::ACCENT } else { palette::TEXT_MUTED };
                    Some(
                        container(
                            text(badge_text)
                                .size(8)
                                .color(badge_col),
                        )
                        .padding([1, 4])
                        .style(theme::badge_pill_style(palette::BG_SUNKEN, palette::BORDER_SUBTLE))
                        .into(),
                    )
                } else {
                    None
                };

                let mut tile_inner = column![
                    icon(
                        Icon::for_tool(tool),
                        20.0,
                        if is_active { palette::ACCENT } else { palette::TEXT_SECONDARY },
                    ),
                    text(tool.display_name()).size(12.0).color(palette::TEXT_PRIMARY),
                ]
                .spacing(5)
                .align_x(Alignment::Center);

                if let Some(rb) = raw_badge {
                    tile_inner = tile_inner.push(rb);
                }

                let tile_content = container(tile_inner)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill);

                let tile_btn = button(tile_content)
                    .width(Length::Fill)
                    .height(Length::Fixed(76.0))
                    .padding(0)
                    .style(theme::tool_tile_button(is_active))
                    .on_press(Message::OpenTool(tool));

                let star_btn = button(icon(fav_icon, 13.0, fav_col))
                    .padding([3, 5])
                    .style(theme::ghost_icon_button(false))
                    .on_press(Message::ToggleFavorite(tool));

                let star_overlay = container(star_btn)
                    .width(Length::Fill)
                    .height(Length::Fixed(76.0))
                    .align_x(Horizontal::Right)
                    .align_y(Vertical::Top)
                    .padding(4);

                let cell: Element<'a, Message> = container(stack![tile_btn, star_overlay])
                    .width(Length::FillPortion(1))
                    .into();

                pair_row = pair_row.push(cell);
            }
            if chunk.len() == 1 {
                pair_row = pair_row.push(container(text("").size(1)).width(Length::FillPortion(1)));
            }
            cat_col = cat_col.push(pair_row);
        }

        main_col = main_col.push(cat_col);
    }

    main_col.into()
}

// ---------------------------------------------------------------------------
// Tab: Looks
// ---------------------------------------------------------------------------

fn render_looks_tab<'a>(app: &'a SnapApp) -> Element<'a, Message> {
    let copy_btn = button(
        container(
            row![
                icon(Icon::Copy, 13.0, palette::TEXT_SECONDARY),
                text("Copy Recipe").size(12).color(palette::TEXT_SECONDARY),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .align_x(Horizontal::Center),
    )
    .padding([6, 10])
    .style(theme::secondary_button)
    .width(Length::FillPortion(1))
    .on_press(Message::CopyStack);

    let paste_btn = button(
        container(
            row![
                icon(Icon::Paste, 13.0, palette::TEXT_SECONDARY),
                text("Paste Recipe").size(12).color(palette::TEXT_SECONDARY),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .align_x(Horizontal::Center),
    )
    .padding([6, 10])
    .style(theme::secondary_button)
    .width(Length::FillPortion(1))
    .on_press(Message::PasteStack);

    let import_xmp_btn = button(
        container(
            row![
                icon(Icon::Download, 13.0, palette::TEXT_SECONDARY),
                text("Import .XMP").size(12).color(palette::TEXT_SECONDARY),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .align_x(Horizontal::Center),
    )
    .padding([6, 10])
    .style(theme::secondary_button)
    .width(Length::FillPortion(1))
    .on_press(Message::ImportXmpDialog);

    let top_row = row![copy_btn, paste_btn, import_xmp_btn].spacing(6);

    let mut built_in_col = column![
        text("BUILT-IN LOOKS").size(11).color(palette::TEXT_MUTED),
    ]
    .spacing(6);

    let all_built_ins = built_in_looks();
    for chunk in all_built_ins.chunks(2) {
        let mut row_pair = row![].spacing(8);
        for look in chunk {
            let accent = look_accent_color(&look.id);
            let accent_strip = container(text("").size(1))
                .width(Length::Fill)
                .height(Length::Fixed(4.0))
                .style(theme::badge_pill_style(accent, Color::TRANSPARENT));

            let name_text = text(look.name.clone()).size(13).color(palette::TEXT_PRIMARY);
            let steps_text = text(format!("{} steps", look.stack.nodes.len()))
                .size(11)
                .color(palette::TEXT_MUTED);

            let card_btn = button(
                column![
                    accent_strip,
                    name_text,
                    steps_text,
                ]
                .spacing(4),
            )
            .padding([8, 10])
            .style(theme::look_card_button(accent, false))
            .width(Length::FillPortion(1))
            .on_press(Message::ApplyLook(look.id.clone()));

            row_pair = row_pair.push(card_btn);
        }
        if chunk.len() == 1 {
            row_pair = row_pair.push(container(text("").size(1)).width(Length::FillPortion(1)));
        }
        built_in_col = built_in_col.push(row_pair);
    }

    let mut custom_col = column![
        text("CUSTOM LOOKS").size(11).color(palette::TEXT_MUTED),
    ]
    .spacing(6);

    if !app.custom_looks.is_empty() {
        for chunk in app.custom_looks.chunks(2) {
            let mut row_pair = row![].spacing(8);
            for look in chunk {
                let accent = palette::ACCENT;
                let accent_strip = container(text("").size(1))
                    .width(Length::Fill)
                    .height(Length::Fixed(4.0))
                    .style(theme::badge_pill_style(accent, Color::TRANSPARENT));

                let name_text = text(look.name.clone()).size(13).color(palette::TEXT_PRIMARY);
                let steps_text = text(format!("{} steps", look.stack.nodes.len()))
                    .size(11)
                    .color(palette::TEXT_MUTED);

                let card_btn = button(
                    column![
                        accent_strip,
                        name_text,
                        steps_text,
                    ]
                    .spacing(4),
                )
                .padding([8, 10])
                .style(theme::look_card_button(accent, false))
                .width(Length::FillPortion(1))
                .on_press(Message::ApplyLook(look.id.clone()));

                row_pair = row_pair.push(card_btn);
            }
            if chunk.len() == 1 {
                row_pair = row_pair.push(container(text("").size(1)).width(Length::FillPortion(1)));
            }
            custom_col = custom_col.push(row_pair);
        }
    }

    let save_input = text_input("Look Name...", &app.save_look_name_input)
        .size(12)
        .on_input(Message::LookNameInputChanged)
        .style(theme::studio_text_input)
        .width(Length::Fill);

    let save_btn = button(text("Save").size(12).color(Color::WHITE))
        .padding([7, 14])
        .style(theme::accent_button)
        .on_press(Message::SaveCurrentLook);

    let save_row = row![save_input, save_btn].spacing(6).align_y(Alignment::Center);
    custom_col = custom_col.push(save_row);

    column![
        top_row,
        built_in_col,
        custom_col,
    ]
    .spacing(14)
    .into()
}

// ---------------------------------------------------------------------------
// Tab: View Edits Stack
// ---------------------------------------------------------------------------

fn render_view_edits_tab<'a>(app: &'a SnapApp) -> Element<'a, Message> {
    let stack = app.document_history.current();

    let header_row = row![
        text("EDIT HISTORY").size(11).color(palette::TEXT_MUTED),
        space::horizontal(),
        text(format!("{} edits", stack.nodes.len())).size(11).color(palette::TEXT_MUTED),
    ]
    .align_y(Alignment::Center);

    let is_base_active = app.active_view_edits_step.is_none();
    let base_card = button(
        row![
            icon(Icon::ImageFile, 15.0, palette::ACCENT),
            text("Original Photo").size(12).color(palette::TEXT_PRIMARY),
            space::horizontal(),
            text("Base").size(11).color(palette::TEXT_MUTED),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .padding([8, 12])
    .style(theme::tool_tile_button(is_base_active))
    .width(Length::Fill)
    .on_press(Message::PreviewStep(None));

    let mut layers_col = column![header_row, base_card].spacing(8);

    if stack.nodes.is_empty() {
        layers_col = layers_col.push(
            text("No edits in stack. Open tools from Tools or Looks to apply non-destructive edits!")
                .size(11)
                .color(palette::TEXT_MUTED),
        );
    } else {
        for (idx, node) in stack.nodes.iter().enumerate() {
            let id = node.id;
            let is_preview_step = app.active_view_edits_step == Some(idx);
            let card_style = if is_preview_step {
                theme::active_surface_card_style
            } else {
                theme::surface_card_style
            };

            let eye_icon = if node.enabled { Icon::Eye } else { Icon::EyeOff };
            let eye_color = if node.enabled { palette::ACCENT } else { palette::TEXT_MUTED };

            let toggle_btn = button(icon(eye_icon, 14.0, eye_color))
                .padding([2, 4])
                .style(theme::ghost_icon_button(false))
                .on_press(Message::ToggleNode(id));

            let tool_icon = icon(Icon::for_tool(node.op.tool_id()), 14.0, palette::ACCENT);
            let tool_name = text(format!("{}. {}", idx + 1, node.op.tool_id().display_name()))
                .size(12)
                .color(if node.enabled { palette::TEXT_PRIMARY } else { palette::TEXT_MUTED });

            let title_btn = button(tool_name)
                .padding([2, 4])
                .style(theme::ghost_icon_button(false))
                .on_press(Message::PreviewStep(Some(idx)));

            let up_btn = button(icon(Icon::ChevronUp, 12.0, palette::TEXT_MUTED))
                .padding([2, 4])
                .style(theme::ghost_icon_button(false))
                .on_press(Message::MoveNodeUp(id));

            let down_btn = button(icon(Icon::ChevronDown, 12.0, palette::TEXT_MUTED))
                .padding([2, 4])
                .style(theme::ghost_icon_button(false))
                .on_press(Message::MoveNodeDown(id));

            let edit_btn = button(icon(Icon::Sliders, 12.0, palette::TEXT_SECONDARY))
                .padding([2, 4])
                .style(theme::ghost_icon_button(false))
                .on_press(Message::EditExistingNode(id));

            let trash_btn = button(icon(Icon::Trash, 12.0, palette::DANGER))
                .padding([2, 4])
                .style(theme::subtle_danger_button)
                .on_press(Message::DeleteNode(id));

            let is_crop = node.op.tool_id() == ToolId::CropRotate;
            let is_mask_open = app.mask_edit_node_id == Some(id);
            let mask_highlighted = node.mask.is_some() || is_mask_open;
            let mask_color = if mask_highlighted {
                palette::ACCENT
            } else {
                palette::TEXT_MUTED
            };

            let mut top_row = row![
                toggle_btn,
                tool_icon,
                title_btn,
                space::horizontal(),
                up_btn,
                down_btn,
            ]
            .spacing(4)
            .align_y(Alignment::Center);

            if !is_crop {
                let mask_btn = button(icon(Icon::Mask, 12.0, mask_color))
                    .padding([2, 4])
                    .style(theme::ghost_icon_button(mask_highlighted))
                    .on_press(Message::ToggleNodeMaskPanel(id));
                top_row = top_row.push(mask_btn);
            }

            top_row = top_row.push(edit_btn).push(trash_btn);

            let opacity_row = row![
                text("Opacity").size(11).color(palette::TEXT_MUTED),
                slider(0.0..=1.0, node.opacity, move |v| Message::SetNodeOpacity(id, v))
                    .step(0.01_f32)
                    .style(theme::studio_slider(palette::ACCENT)),
                text(format!("{:.0}%", node.opacity * 100.0))
                    .size(11)
                    .color(palette::TEXT_MUTED),
            ]
            .spacing(8)
            .align_y(Alignment::Center);

            let mut card_col = column![top_row, opacity_row].spacing(6);

            if is_mask_open && !is_crop {
                let props = crate::tool_editors_ext::MaskInspectorProps {
                    node_id: id,
                    has_mask: node.mask.is_some(),
                    is_paint_mode: app.mask_paint_mode,
                    is_eraser: app.mask_eraser,
                    brush_radius: app.mask_brush_radius,
                    brush_hardness: app.mask_brush_hardness,
                    target_opacity: app.mask_target_opacity,
                    red_overlay: app.mask_red_overlay,
                    node_title: node.op.tool_id().display_name(),
                };
                card_col = card_col.push(
                    container(
                        crate::tool_editors_ext::view_layer_mask_inspector(props)
                            .map(Message::ToolEditorExt),
                    )
                    .padding([6, 0]),
                );
            }

            let card = container(card_col)
                .padding(10)
                .style(card_style)
                .width(Length::Fill);

            layers_col = layers_col.push(card);
        }
    }

    layers_col.into()
}

// ---------------------------------------------------------------------------
// Tab: Export
// ---------------------------------------------------------------------------

fn render_export_tab<'a>(app: &'a SnapApp) -> Element<'a, Message> {
    let formats = [
        (ExportFormat::Jpeg { quality: 95 }, "JPEG 95%"),
        (ExportFormat::Jpeg { quality: 85 }, "JPEG 85%"),
        (ExportFormat::Png8, "PNG 8-bit"),
        (ExportFormat::Png16, "PNG 16-bit"),
        (ExportFormat::Tiff16, "TIFF 16-bit"),
    ];

    let mut format_col = column![
        text("EXPORT FORMAT").size(11).color(palette::TEXT_MUTED),
    ]
    .spacing(6);

    for (fmt, label) in formats {
        let is_sel = app.export_options.format == fmt;
        let btn = button(
            container(text(label).size(12))
                .width(Length::Fill)
                .align_x(Horizontal::Center),
        )
        .padding([6, 10])
        .style(theme::pill_chip_button(is_sel))
        .width(Length::Fill)
        .on_press(Message::SetExportFormat(fmt));
        format_col = format_col.push(btn);
    }

    let resizes = [
        (ResizeLongEdge::Original, "Original"),
        (ResizeLongEdge::Px4000, "4096 px"),
        (ResizeLongEdge::Px2000, "2048 px"),
        (ResizeLongEdge::Px1920, "1080 px"),
    ];

    let mut resize_col = column![
        text("RESIZE (LONG EDGE)").size(11).color(palette::TEXT_MUTED),
    ]
    .spacing(6);

    let mut r_row = row![].spacing(6);
    for (res, label) in resizes {
        let is_sel = app.export_options.resize == res;
        let btn = button(
            container(text(label).size(11))
                .width(Length::Fill)
                .align_x(Horizontal::Center),
        )
        .padding([5, 8])
        .style(theme::pill_chip_button(is_sel))
        .width(Length::FillPortion(1))
        .on_press(Message::SetExportResize(res));
        r_row = r_row.push(btn);
    }
    resize_col = resize_col.push(r_row);

    let exif_check = checkbox(app.export_options.preserve_exif)
        .label("Preserve Camera EXIF & Metadata")
        .on_toggle(Message::SetExportPreserveExif)
        .style(theme::studio_checkbox)
        .size(14);

    let sidecar_btn = button(
        container(
            row![
                icon(Icon::Save, 14.0, palette::TEXT_SECONDARY),
                text("Save .snap Sidecar").size(12).color(palette::TEXT_SECONDARY),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .align_x(Horizontal::Center),
    )
    .padding([8, 14])
    .style(theme::secondary_button)
    .width(Length::Fill)
    .on_press(Message::SaveSidecar);

    let export_text = if app.is_exporting {
        "Exporting Full-Resolution Photo..."
    } else {
        "Export Full-Resolution Photo"
    };

    let mut export_btn = button(
        container(
            row![
                icon(Icon::Download, 15.0, Color::WHITE),
                text(export_text).size(13).color(Color::WHITE),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .align_x(Horizontal::Center),
    )
    .padding([11, 16])
    .style(theme::accent_button)
    .width(Length::Fill);

    if !app.is_exporting && app.source_image.is_some() {
        export_btn = export_btn.on_press(Message::StartExport);
    }

    column![
        format_col,
        resize_col,
        exif_check,
        sidecar_btn,
        export_btn,
    ]
    .spacing(14)
    .into()
}
