use iced::widget::{
    button, checkbox, column, container, row, scrollable, slider, text, text_input,
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
use crate::curves_widget::CurvesWidget;
use crate::histogram_widget::HistogramWidget;
use crate::theme::*;

pub fn view(app: &SnapApp) -> Element<'_, Message> {
    let header = render_header(app);
    let center = render_center(app);
    let right = render_inspector(app);

    let content = row![
        container(center).width(Length::Fill).height(Length::Fill),
        container(right).width(Length::Fixed(360.0)).height(Length::Fill),
    ]
    .spacing(0);

    column![header, content].into()
}

fn render_header(app: &SnapApp) -> Element<'_, Message> {
    // Brand title
    let brand = text("SNAP")
        .size(18)
        .color(ACCENT_BLUE);

    // Current file or status
    let file_info = if let Some(ref path) = app.current_image_path {
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("Photo");
        let dims = if let Some(ref img) = app.source_image {
            format!(" — {}x{}", img.width, img.height)
        } else {
            String::new()
        };
        text(format!("{name}{dims}"))
            .size(13)
            .color(TEXT_SECONDARY)
    } else {
        text("No photo open").size(13).color(TEXT_MUTED)
    };

    // Favorites bar quick launch
    let mut fav_row = row![].spacing(6).align_y(Alignment::Center);
    for &tool in &app.config.favorites {
        let is_active = app
            .active_tool_session
            .as_ref()
            .map(|s| s.tool == tool)
            .unwrap_or(false);

        let btn = button(text(tool.short_code()).size(12))
            .padding([4, 8])
            .style(pill_button(is_active))
            .on_press(Message::OpenTool(tool));
        fav_row = fav_row.push(btn);
    }

    // Undo / Redo buttons
    let can_undo = app.active_tool_session.is_some() || app.document_history.can_undo();
    let can_redo = app.active_tool_session.is_some() || app.document_history.can_redo();

    let undo_btn = button(text("↶ Undo").size(12))
        .padding([5, 10])
        .style(ghost_button);
    let undo_btn = if can_undo {
        undo_btn.on_press(Message::Undo)
    } else {
        undo_btn
    };

    let redo_btn = button(text("↷ Redo").size(12))
        .padding([5, 10])
        .style(ghost_button);
    let redo_btn = if can_redo {
        redo_btn.on_press(Message::Redo)
    } else {
        redo_btn
    };

    // Hold-to-compare / Toggle compare
    let compare_btn = button(
        text(if app.bypass_edits {
            "● ORIGINAL"
        } else {
            "○ Compare"
        })
        .size(12)
        .color(if app.bypass_edits {
            ACCENT_AMBER
        } else {
            TEXT_PRIMARY
        }),
    )
    .padding([5, 10])
    .style(ghost_button)
    .on_press(Message::ToggleBypassEdits);

    // Open & Export quick buttons
    let open_btn = button(text("Open...").size(12))
        .padding([5, 12])
        .style(primary_button)
        .on_press(Message::OpenFileDialog);

    let export_btn = button(text("Export").size(12))
        .padding([5, 12])
        .style(ghost_button)
        .on_press(Message::SelectTab(Tab::Export));

    let status_str = if let Some((ref msg, _)) = app.status_message {
        text(msg).size(12).color(ACCENT_BLUE)
    } else {
        text("").size(12)
    };

    container(
        row![
            row![brand, file_info].spacing(12).align_y(Alignment::Center),
            scrollable(fav_row).direction(scrollable::Direction::Horizontal(Default::default())),
            row![
                status_str,
                undo_btn,
                redo_btn,
                compare_btn,
                open_btn,
                export_btn
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        ]
        .spacing(16)
        .align_y(Alignment::Center),
    )
    .padding([8, 16])
    .style(header_bar)
    .width(Length::Fill)
    .into()
}

fn render_center(app: &SnapApp) -> Element<'_, Message> {
    if app.source_image.is_none() {
        return render_welcome_screen(app);
    }

    // Live GPU canvas shader viewport
    let canvas_program = CanvasShaderProgram {
        image_id: Some(app.image_id),
        proxy_image: app.source_image.clone(),
        effective_stack: app.effective_stack(),
        bypass_edits: app.bypass_edits,
        viewport_params: app.viewport_params.clone(),
        needed_luts: app.lut_library.clone(),
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

    let shader_widget: Element<'_, Message> =
        iced::widget::shader(canvas_program)
            .width(Length::Fill)
            .height(Length::Fill)
            .into();

    // Floating overlays on top of the canvas
    let hud = render_hud_overlay(app);
    let bottom_bar = render_bottom_toolbar(app);

    column![
        container(
            column![
                hud,
                container(shader_widget).width(Length::Fill).height(Length::Fill),
            ]
            .spacing(0),
        )
        .width(Length::Fill)
        .height(Length::Fill),
        bottom_bar,
    ]
    .into()
}

fn render_hud_overlay(app: &SnapApp) -> Element<'_, Message> {
    let mut hud_col = column![].spacing(6).align_x(Alignment::Center);

    if app.bypass_edits {
        let pill = container(text("ORIGINAL (BYPASS EDITS)").size(11).color(ACCENT_AMBER))
            .padding([4, 12])
            .style(pill_container);
        hud_col = hud_col.push(pill);
    }

    if let Some(ref session) = app.active_tool_session {
        let sliders = session.live_op.slider_params();
        if !sliders.is_empty() && session.selected_param_idx < sliders.len() {
            let (desc, val) = &sliders[session.selected_param_idx];

            // Top Parameter Pill with mini slider
            let pill_content = row![
                text(desc.name).size(12).color(TEXT_SECONDARY),
                text(format!("{:+.*} {}", if desc.step < 0.5 { 1 } else { 0 }, val, desc.unit))
                    .size(12)
                    .color(TEXT_PRIMARY),
            ]
            .spacing(12)
            .align_y(Alignment::Center);

            let pill = container(pill_content)
                .padding([4, 16])
                .style(pill_container);
            hud_col = hud_col.push(pill);

            // Floating Vertical Parameter Picker (when vertical swipe is active)
            if session.vertical_picker_active {
                let mut picker_col = column![].spacing(4).padding(8);
                for (idx, (p_desc, p_val)) in sliders.iter().enumerate() {
                    let is_sel = idx == session.selected_param_idx;
                    let row_item = row![
                        text(p_desc.name)
                            .size(12)
                            .color(if is_sel { ACCENT_BLUE } else { TEXT_SECONDARY }),
                        text(format!("{:+.*}", if p_desc.step < 0.5 { 1 } else { 0 }, p_val))
                            .size(12)
                            .color(if is_sel { Color::WHITE } else { TEXT_MUTED }),
                    ]
                    .spacing(16)
                    .align_y(Alignment::Center);

                    let item_container = container(row_item)
                        .padding([4, 10])
                        .style(if is_sel { active_card_container } else { card_container });
                    picker_col = picker_col.push(item_container);
                }

                let picker_card = container(picker_col).style(hud_overlay_container);
                hud_col = hud_col.push(picker_card);
            }
        }
    }

    container(hud_col)
        .width(Length::Fill)
        .padding(8)
        .align_x(Alignment::Center)
        .into()
}

fn render_bottom_toolbar(app: &SnapApp) -> Element<'_, Message> {
    let fit_btn = button(text("Fit").size(12))
        .padding([4, 8])
        .style(ghost_button)
        .on_press(Message::FitView);

    let z100_btn = button(text("100%").size(12))
        .padding([4, 8])
        .style(ghost_button)
        .on_press(Message::Zoom100);

    let zoom_pct = (app.viewport_params.zoom * 100.0).round() as u32;
    let zoom_label = text(format!("{zoom_pct}%")).size(12).color(TEXT_SECONDARY);

    let img_dims = if let Some(ref img) = app.source_image {
        format!("{} x {} px", img.width, img.height)
    } else {
        String::new()
    };
    let dims_label = text(img_dims).size(12).color(TEXT_MUTED);

    container(
        row![
            row![fit_btn, z100_btn, zoom_label].spacing(8).align_y(Alignment::Center),
            dims_label,
        ]
        .spacing(16)
        .align_y(Alignment::Center),
    )
    .padding([6, 16])
    .style(header_bar)
    .width(Length::Fill)
    .into()
}

fn render_welcome_screen(app: &SnapApp) -> Element<'_, Message> {
    let title = text("Snap").size(36).color(ACCENT_BLUE);
    let subtitle = text("High-Performance Non-Destructive RAW & Photo Editor for Linux")
        .size(15)
        .color(TEXT_SECONDARY);

    let open_btn = button(
        row![text("📂 Open Photo...").size(14)]
            .align_y(Alignment::Center)
            .padding([10, 20]),
    )
    .style(primary_button)
    .on_press(Message::OpenFileDialog);

    let demo_btn = button(
        row![text("⚡ Load Sample HDR Test Scene").size(14)]
            .align_y(Alignment::Center)
            .padding([10, 20]),
    )
    .style(ghost_button)
    .on_press(Message::LoadDemoScene);

    // Recent Files List
    let mut recents_col = column![text("Recent Photos").size(14).color(TEXT_SECONDARY)].spacing(6);
    if app.config.recent_files.is_empty() {
        recents_col = recents_col.push(
            text("No recent photos yet. Drag and drop any image or click Open Photo to start!")
                .size(12)
                .color(TEXT_MUTED),
        );
    } else {
        for path in app.config.recent_files.iter().take(5) {
            let p_clone = path.clone();
            let label = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("image");
            let row_btn = button(
                row![
                    text("🖼").size(14),
                    text(label).size(13).color(TEXT_PRIMARY),
                    text(path.display().to_string()).size(11).color(TEXT_MUTED),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            )
            .padding([6, 12])
            .style(ghost_button)
            .on_press(Message::FileDropped(p_clone));
            recents_col = recents_col.push(row_btn);
        }
    }

    let drag_hint = text("Tip: You can drag and drop photos, RAW files (.dng, .cr2, .arw), and 3D LUTs (.cube, .3dl) directly into Snap!")
        .size(12)
        .color(TEXT_MUTED);

    let welcome_box = container(
        column![
            title,
            subtitle,
            row![open_btn, demo_btn].spacing(16),
            container(recents_col).padding(16).style(card_container),
            drag_hint,
        ]
        .spacing(24)
        .align_x(Alignment::Center),
    )
    .padding(48)
    .align_x(Alignment::Center)
    .align_y(Alignment::Center);

    container(welcome_box)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(app_container)
        .into()
}

fn render_inspector(app: &SnapApp) -> Element<'_, Message> {
    // 1. Live GPU Histogram widget at the top
    let hist_widget: Element<'_, Message> =
        iced::widget::canvas(HistogramWidget::new(&app.histogram))
            .width(Length::Fill)
            .height(Length::Fixed(90.0))
            .into();

    // 2. Navigation Tabs (Looks | Tools | Export | View Edits)
    let looks_tab = button(text("Looks").size(12))
        .padding([6, 12])
        .style(tab_button(app.current_tab == Tab::Looks))
        .on_press(Message::SelectTab(Tab::Looks));

    let tools_tab = button(text("Tools").size(12))
        .padding([6, 12])
        .style(tab_button(app.current_tab == Tab::Tools))
        .on_press(Message::SelectTab(Tab::Tools));

    let export_tab = button(text("Export").size(12))
        .padding([6, 12])
        .style(tab_button(app.current_tab == Tab::Export))
        .on_press(Message::SelectTab(Tab::Export));

    let edit_count = app.document_history.current().nodes.len();
    let edits_label = if edit_count > 0 {
        format!("View Edits ({edit_count})")
    } else {
        "View Edits".to_string()
    };
    let view_edits_tab = button(text(edits_label).size(12))
        .padding([6, 12])
        .style(tab_button(app.current_tab == Tab::ViewEdits))
        .on_press(Message::SelectTab(Tab::ViewEdits));

    let tab_bar = row![looks_tab, tools_tab, export_tab, view_edits_tab]
        .spacing(4)
        .padding([6, 10]);

    // 3. Tab Body
    let tab_content: Element<'_, Message> = match app.current_tab {
        Tab::Looks => render_looks_tab(app),
        Tab::Tools => render_tools_tab(app),
        Tab::Export => render_export_tab(app),
        Tab::ViewEdits => render_view_edits_tab(app),
    };

    container(
        column![
            container(hist_widget).padding(10),
            tab_bar,
            container(scrollable(tab_content))
                .width(Length::Fill)
                .height(Length::Fill),
        ]
        .spacing(0),
    )
    .style(inspector_panel)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn render_looks_tab(app: &SnapApp) -> Element<'_, Message> {
    let mut list = column![].spacing(8).padding(10);

    // Save current stack as look
    let save_row = row![
        text_input("New Look Name...", &app.save_look_name_input)
            .size(12)
            .on_input(Message::LookNameInputChanged),
        button(text("Save").size(12))
            .style(primary_button)
            .padding([5, 10])
            .on_press(Message::SaveCurrentLook),
    ]
    .spacing(8);
    list = list.push(container(save_row).padding(8).style(card_container));

    // Copy / Paste stack
    let copy_paste_row = row![
        button(text("📋 Copy Stack").size(12))
            .style(ghost_button)
            .padding([4, 8])
            .on_press(Message::CopyStack),
        button(text("📄 Paste Stack").size(12))
            .style(ghost_button)
            .padding([4, 8])
            .on_press(Message::PasteStack),
    ]
    .spacing(8);
    list = list.push(copy_paste_row);

    // 11 Classic Snapseed Built-in Looks
    list = list.push(text("Built-in Looks").size(13).color(TEXT_SECONDARY));
    for look in built_in_looks() {
        let id = look.id.clone();
        let name = look.name.clone();
        let node_count = look.stack.nodes.len();

        let card = button(
            row![
                text("✦").size(14).color(ACCENT_BLUE),
                column![
                    text(name).size(13).color(TEXT_PRIMARY),
                    text(format!("{node_count} non-destructive steps"))
                        .size(11)
                        .color(TEXT_MUTED),
                ]
                .spacing(2),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .padding([8, 12])
        .style(ghost_button)
        .width(Length::Fill)
        .on_press(Message::ApplyLook(id));

        list = list.push(card);
    }

    // Custom Looks
    if !app.custom_looks.is_empty() {
        list = list.push(text("Custom Looks").size(13).color(TEXT_SECONDARY));
        for look in &app.custom_looks {
            let id = look.id.clone();
            let name = look.name.clone();
            let card = button(
                row![
                    text("★").size(14).color(ACCENT_AMBER),
                    text(name).size(13).color(TEXT_PRIMARY),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            )
            .padding([8, 12])
            .style(ghost_button)
            .width(Length::Fill)
            .on_press(Message::ApplyLook(id));
            list = list.push(card);
        }
    }

    list.into()
}

fn render_tools_tab(app: &SnapApp) -> Element<'_, Message> {
    if let Some(ref session) = app.active_tool_session {
        return render_active_tool_session(app, session);
    }

    // Tool library categorized
    let mut list = column![].spacing(14).padding(10);

    let categories = ["Develop", "Tone", "Detail", "Color", "Creative", "Geometry"];

    for cat in categories {
        let tools_in_cat: Vec<ToolId> = ToolId::ALL
            .iter()
            .copied()
            .filter(|t| t.category() == cat)
            .collect();

        if tools_in_cat.is_empty() {
            continue;
        }

        let mut cat_col = column![text(cat).size(13).color(TEXT_SECONDARY)].spacing(6);

        for tool in tools_in_cat {
            let is_fav = app.config.favorites.contains(&tool);
            let fav_icon = if is_fav { "★" } else { "☆" };

            let star_btn = button(
                text(fav_icon)
                    .size(14)
                    .color(if is_fav { ACCENT_AMBER } else { TEXT_MUTED }),
            )
            .padding([4, 6])
            .style(ghost_button)
            .on_press(Message::ToggleFavorite(tool));

            let open_tool_btn = button(
                row![
                    text(tool.display_name()).size(13).color(TEXT_PRIMARY),
                ]
                .align_y(Alignment::Center),
            )
            .padding([8, 10])
            .style(ghost_button)
            .width(Length::Fill)
            .on_press(Message::OpenTool(tool));

            let item_row = row![open_tool_btn, star_btn]
                .spacing(4)
                .align_y(Alignment::Center);

            cat_col = cat_col.push(item_row);
        }

        list = list.push(cat_col);
    }

    list.into()
}

fn render_active_tool_session<'a>(
    app: &'a SnapApp,
    session: &'a ActiveToolSession,
) -> Element<'a, Message> {
    let mut col = column![].spacing(12).padding(10);

    // Header: Tool Name + In-Tool Undo/Redo
    let title = text(session.tool.display_name()).size(16).color(ACCENT_BLUE);
    let tool_undo = button(text("↶").size(14))
        .padding([4, 8])
        .style(ghost_button)
        .on_press(Message::ToolUndo);
    let tool_redo = button(text("↷").size(14))
        .padding([4, 8])
        .style(ghost_button)
        .on_press(Message::ToolRedo);

    let header_row = row![
        title,
        row![tool_undo, tool_redo].spacing(4).align_y(Alignment::Center),
    ]
    .spacing(12)
    .align_y(Alignment::Center);
    col = col.push(header_row);

    // Tool-Specific Editors
    match session.tool {
        ToolId::TuneImage => {
            col = col.push(render_tune_image_editor(session));
        }
        ToolId::Curves => {
            col = col.push(render_curves_editor(app, session));
        }
        ToolId::WhiteBalance => {
            col = col.push(render_wb_editor(session));
        }
        ToolId::ColorHsl => {
            col = col.push(render_hsl_editor(session));
        }
        ToolId::Lut3d => {
            col = col.push(render_lut_editor(app, session));
        }
        ToolId::BlackAndWhite => {
            col = col.push(render_bw_editor(session));
        }
        ToolId::Vignette => {
            col = col.push(render_vignette_editor(session));
        }
        ToolId::CropRotate => {
            col = col.push(render_crop_editor(app, session));
        }
        _ => {
            // Generic Slider List for RAW develop / Details
            col = col.push(render_slider_list(session));
        }
    }

    // Action Bar: Apply (Enter) and Cancel (Escape)
    let apply_btn = button(text("✓ Apply (Enter)").size(13))
        .padding([8, 16])
        .style(apply_button)
        .width(Length::Fill)
        .on_press(Message::ApplyActiveTool);

    let cancel_btn = button(text("✕ Cancel (Esc)").size(13))
        .padding([8, 16])
        .style(cancel_button)
        .width(Length::Fill)
        .on_press(Message::CancelActiveTool);

    let action_row = row![cancel_btn, apply_btn].spacing(10);
    col = col.push(container(action_row).padding([12, 0]));

    col.into()
}

fn render_slider_list<'a>(session: &'a ActiveToolSession) -> Element<'a, Message> {
    let mut list = column![].spacing(10);
    let sliders = session.live_op.slider_params();

    for (idx, (desc, val)) in sliders.iter().enumerate() {
        let is_selected = idx == session.selected_param_idx;
        let p_name = text(desc.name)
            .size(12)
            .color(if is_selected { ACCENT_BLUE } else { TEXT_SECONDARY });
        let p_val = text(format!("{:+.*} {}", if desc.step < 0.5 { 1 } else { 0 }, val, desc.unit))
            .size(12)
            .color(TEXT_PRIMARY);

        let row_head = row![p_name, p_val]
            .spacing(12)
            .align_y(Alignment::Center);

        let s = slider(desc.min..=desc.max, *val, move |v| {
            Message::SetSliderParamLive(idx, v)
        })
        .step(desc.step)
        .on_release(Message::CanvasGestureEnd);

        list = list.push(column![row_head, s].spacing(4));
    }

    list.into()
}

fn render_tune_image_editor<'a>(session: &'a ActiveToolSession) -> Element<'a, Message> {
    let auto_btn = button(
        row![text("🪄 Auto-Adjust Tone (Magic Wand)").size(12)]
            .align_y(Alignment::Center)
            .padding([6, 12]),
    )
    .style(primary_button)
    .width(Length::Fill)
    .on_press(Message::TuneAuto);

    column![auto_btn, render_slider_list(session)].spacing(12).into()
}

fn render_wb_editor<'a>(session: &'a ActiveToolSession) -> Element<'a, Message> {
    let eyedropper_btn = button(
        row![text(if session.eyedropper_active {
            "🎯 Pick Neutral: Click on Canvas"
        } else {
            "🔍 Pick Neutral Gray (Eyedropper)"
        })
        .size(12)]
        .align_y(Alignment::Center)
        .padding([6, 12]),
    )
    .style(if session.eyedropper_active {
        apply_button
    } else {
        ghost_button
    })
    .width(Length::Fill)
    .on_press(Message::ToggleEyedropper);

    column![eyedropper_btn, render_slider_list(session)]
        .spacing(12)
        .into()
}

fn render_curves_editor<'a>(
    app: &'a SnapApp,
    session: &'a ActiveToolSession,
) -> Element<'a, Message> {
    let EditOp::Curves(curves_params) = &session.live_op else {
        return text("No curves active").into();
    };

    // 1. Channel Tabs
    let channels = [
        (CurveChannelId::Rgb, "RGB"),
        (CurveChannelId::Red, "Red"),
        (CurveChannelId::Green, "Green"),
        (CurveChannelId::Blue, "Blue"),
        (CurveChannelId::Luminance, "Luma"),
    ];

    let mut ch_row = row![].spacing(4);
    for (id, label) in channels {
        let is_sel = session.current_curve_channel == id;
        let btn = button(text(label).size(11))
            .padding([4, 8])
            .style(pill_button(is_sel))
            .on_press(Message::CurvesWidgetEvent(
                crate::curves_widget::CurvesEvent::SelectChannel(id),
            ));
        ch_row = ch_row.push(btn);
    }

    // 2. Interactive Curves Widget Canvas
    let widget: Element<'_, Message> = iced::widget::canvas(CurvesWidget::new(
        curves_params,
        session.current_curve_channel,
        &app.histogram,
        Message::CurvesWidgetEvent,
    ))
    .width(Length::Fixed(320.0))
    .height(Length::Fixed(240.0))
    .into();

    // 3. Preset Pills
    let presets = [
        (CurvePreset::Neutral, "Neutral"),
        (CurvePreset::HardContrast, "Hard Contrast"),
        (CurvePreset::SoftContrast, "Soft Contrast"),
        (CurvePreset::Brighten, "Brighten"),
        (CurvePreset::Darken, "Darken"),
        (CurvePreset::HighKey, "High Key"),
        (CurvePreset::LowKey, "Low Key"),
        (CurvePreset::Fade, "Fade"),
        (CurvePreset::Punch, "Punch"),
        (CurvePreset::CrossProcess, "Cross Process"),
    ];

    let mut preset_row = row![].spacing(4);
    for (p, p_label) in presets {
        let btn = button(text(p_label).size(11))
            .padding([3, 6])
            .style(ghost_button)
            .on_press(Message::CurvesWidgetEvent(
                crate::curves_widget::CurvesEvent::ApplyPreset(p),
            ));
        preset_row = preset_row.push(btn);
    }

    let reset_btn = button(text("Reset Channel").size(11))
        .padding([4, 8])
        .style(ghost_button)
        .on_press(Message::CurvesWidgetEvent(
            crate::curves_widget::CurvesEvent::ResetChannel(session.current_curve_channel),
        ));

    column![
        ch_row,
        container(widget).padding(4),
        scrollable(preset_row).direction(scrollable::Direction::Horizontal(Default::default())),
        reset_btn,
    ]
    .spacing(8)
    .into()
}

fn render_hsl_editor<'a>(session: &'a ActiveToolSession) -> Element<'a, Message> {
    let bands = [
        (HslBand::Red, "Red"),
        (HslBand::Orange, "Orange"),
        (HslBand::Yellow, "Yellow"),
        (HslBand::Green, "Green"),
        (HslBand::Cyan, "Cyan"),
        (HslBand::Blue, "Blue"),
        (HslBand::Purple, "Purple"),
        (HslBand::Magenta, "Magenta"),
    ];

    let mut band_row = row![].spacing(4);
    for (band, name) in bands {
        let is_sel = session.current_hsl_band == band;
        let btn = button(text(name).size(11))
            .padding([4, 6])
            .style(pill_button(is_sel))
            .on_press(Message::SelectHslBand(band));
        band_row = band_row.push(btn);
    }

    let EditOp::ColorHsl(hsl) = &session.live_op else {
        return text("No HSL active").into();
    };

    let shift = hsl.bands[session.current_hsl_band as usize];

    let hue_slider = column![
        row![text("Hue").size(12).color(TEXT_SECONDARY), text(format!("{:+.0}", shift.hue)).size(12)],
        slider(-100.0..=100.0, shift.hue, |v| Message::SetHslValue(0, v)).step(1.0_f32).on_release(Message::CanvasGestureEnd),
    ].spacing(4);

    let sat_slider = column![
        row![text("Saturation").size(12).color(TEXT_SECONDARY), text(format!("{:+.0}", shift.saturation)).size(12)],
        slider(-100.0..=100.0, shift.saturation, |v| Message::SetHslValue(1, v)).step(1.0_f32).on_release(Message::CanvasGestureEnd),
    ].spacing(4);

    let lum_slider = column![
        row![text("Luminance").size(12).color(TEXT_SECONDARY), text(format!("{:+.0}", shift.luminance)).size(12)],
        slider(-100.0..=100.0, shift.luminance, |v| Message::SetHslValue(2, v)).step(1.0_f32).on_release(Message::CanvasGestureEnd),
    ].spacing(4);

    column![
        scrollable(band_row).direction(scrollable::Direction::Horizontal(Default::default())),
        hue_slider,
        sat_slider,
        lum_slider,
    ]
    .spacing(12)
    .into()
}

fn render_lut_editor<'a>(app: &'a SnapApp, session: &'a ActiveToolSession) -> Element<'a, Message> {
    let EditOp::Lut3d(lut_params) = &session.live_op else {
        return text("No LUT active").into();
    };

    let import_btn = button(text("Import .cube / .3dl / Hald PNG / .zip...").size(12))
        .padding([6, 12])
        .style(primary_button)
        .width(Length::Fill)
        .on_press(Message::ImportLutDialog);

    let mut lut_list = column![].spacing(4);
    for lut in &app.lut_library {
        let id = lut.id.clone();
        let is_sel = lut_params.lut_id == id;
        let name = lut.name.clone();

        let btn = button(
            row![
                text(if is_sel { "●" } else { "○" }).size(12).color(ACCENT_BLUE),
                text(name).size(12).color(if is_sel { TEXT_PRIMARY } else { TEXT_SECONDARY }),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .padding([4, 8])
        .style(ghost_button)
        .width(Length::Fill)
        .on_press(Message::SelectLut(id));

        lut_list = lut_list.push(btn);
    }

    let strength_slider = column![
        row![
            text("Strength").size(12).color(TEXT_SECONDARY),
            text(format!("{:.0}%", lut_params.strength)).size(12),
        ],
        slider(0.0..=100.0, lut_params.strength, Message::SetLutStrength)
            .step(1.0_f32)
            .on_release(Message::CanvasGestureEnd),
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
    let mut space_row = row![text("Input Space:").size(12).color(TEXT_SECONDARY)].spacing(6);
    for (space, label) in spaces {
        let is_sel = lut_params.input_space == space;
        let btn = button(text(label).size(10))
            .padding([3, 6])
            .style(pill_button(is_sel))
            .on_press(Message::SetLutSpace(space));
        space_row = space_row.push(btn);
    }

    column![
        import_btn,
        container(scrollable(lut_list).height(Length::Fixed(160.0)))
            .padding(4)
            .style(card_container),
        strength_slider,
        scrollable(space_row).direction(scrollable::Direction::Horizontal(Default::default())),
    ]
    .spacing(12)
    .into()
}

fn render_bw_editor<'a>(session: &'a ActiveToolSession) -> Element<'a, Message> {
    let EditOp::BlackAndWhite(bw) = &session.live_op else {
        return text("No B&W active").into();
    };

    let presets = [
        (BwPreset::Neutral, "Neutral"),
        (BwPreset::Contrast, "Contrast"),
        (BwPreset::Bright, "Bright"),
        (BwPreset::Dark, "Dark"),
        (BwPreset::Film, "Film"),
        (BwPreset::DarkSky, "Dark Sky"),
    ];

    let mut p_row = row![].spacing(4);
    for (p, label) in presets {
        let is_sel = bw.preset == p;
        let btn = button(text(label).size(11))
            .padding([4, 6])
            .style(pill_button(is_sel))
            .on_press(Message::SelectBwPreset(p));
        p_row = p_row.push(btn);
    }

    let filters = [
        (BwColorFilter::Neutral, "Neutral"),
        (BwColorFilter::Red, "Red"),
        (BwColorFilter::Orange, "Orange"),
        (BwColorFilter::Yellow, "Yellow"),
        (BwColorFilter::Green, "Green"),
        (BwColorFilter::Blue, "Blue"),
    ];

    let mut f_row = row![].spacing(4);
    for (f, label) in filters {
        let is_sel = bw.color_filter == f;
        let btn = button(text(label).size(11))
            .padding([4, 6])
            .style(pill_button(is_sel))
            .on_press(Message::SelectBwFilter(f));
        f_row = f_row.push(btn);
    }

    column![
        text("Presets").size(12).color(TEXT_SECONDARY),
        scrollable(p_row).direction(scrollable::Direction::Horizontal(Default::default())),
        text("Optical Color Filters").size(12).color(TEXT_SECONDARY),
        scrollable(f_row).direction(scrollable::Direction::Horizontal(Default::default())),
        render_slider_list(session),
    ]
    .spacing(10)
    .into()
}

fn render_vignette_editor<'a>(session: &'a ActiveToolSession) -> Element<'a, Message> {
    let center_btn = button(
        row![text(if session.vignette_center_mode {
            "🎯 Center: Click Canvas"
        } else {
            "📍 Click Canvas to Set Center"
        })
        .size(12)]
        .align_y(Alignment::Center)
        .padding([6, 12]),
    )
    .style(if session.vignette_center_mode {
        apply_button
    } else {
        ghost_button
    })
    .width(Length::Fill)
    .on_press(Message::ToggleVignetteCenter);

    column![center_btn, render_slider_list(session)].spacing(12).into()
}

fn render_crop_editor<'a>(app: &'a SnapApp, session: &'a ActiveToolSession) -> Element<'a, Message> {
    let aspects = [
        (AspectRatio::Free, "Free"),
        (AspectRatio::Original, "Original"),
        (AspectRatio::Square, "1:1"),
        (AspectRatio::Din, "DIN"),
        (AspectRatio::R3x2, "3:2"),
        (AspectRatio::R4x3, "4:3"),
        (AspectRatio::R5x4, "5:4"),
        (AspectRatio::R7x5, "7:5"),
        (AspectRatio::R16x9, "16:9"),
    ];

    let mut aspect_row = row![].spacing(4);
    for (a, label) in aspects {
        let is_sel = app.viewport_params.crop.aspect == a;
        let btn = button(text(label).size(11))
            .padding([4, 6])
            .style(pill_button(is_sel))
            .on_press(Message::SetCropAspect(a));
        aspect_row = aspect_row.push(btn);
    }

    let swap_btn = button(text("⇄ Swap Aspect").size(11))
        .padding([4, 8])
        .style(ghost_button)
        .on_press(Message::SwapCropAspect);

    let rotate_ccw = button(text("↺ 90° CCW").size(11))
        .padding([4, 8])
        .style(ghost_button)
        .on_press(Message::RotateCcw);

    let rotate_cw = button(text("↻ 90° CW").size(11))
        .padding([4, 8])
        .style(ghost_button)
        .on_press(Message::RotateCw);

    let flip_h = button(text("Flip H").size(11))
        .padding([4, 8])
        .style(ghost_button)
        .on_press(Message::FlipH);

    let flip_v = button(text("Flip V").size(11))
        .padding([4, 8])
        .style(ghost_button)
        .on_press(Message::FlipV);

    let reset_btn = button(text("Reset Crop").size(11))
        .padding([4, 8])
        .style(ghost_button)
        .on_press(Message::ResetCrop);

    column![
        text("Aspect Ratio").size(12).color(TEXT_SECONDARY),
        scrollable(aspect_row).direction(scrollable::Direction::Horizontal(Default::default())),
        row![swap_btn, rotate_ccw, rotate_cw].spacing(6),
        row![flip_h, flip_v, reset_btn].spacing(6),
        render_slider_list(session),
    ]
    .spacing(10)
    .into()
}

fn render_export_tab(app: &SnapApp) -> Element<'_, Message> {
    let mut col = column![].spacing(14).padding(10);

    // Format selection
    let formats = [
        (ExportFormat::Jpeg { quality: 100 }, "JPEG 100% (Lossless)"),
        (ExportFormat::Jpeg { quality: 95 }, "JPEG 95% (High Quality)"),
        (ExportFormat::Jpeg { quality: 80 }, "JPEG 80% (Compact Web)"),
        (ExportFormat::Png8, "PNG 8-bit"),
        (ExportFormat::Png16, "PNG 16-bit (Master)"),
        (ExportFormat::Tiff16, "TIFF 16-bit (Print)"),
    ];

    let mut format_col = column![text("Format").size(13).color(TEXT_SECONDARY)].spacing(4);
    for (fmt, label) in formats {
        let is_sel = app.export_options.format == fmt;
        let btn = button(
            row![
                text(if is_sel { "●" } else { "○" }).size(12).color(ACCENT_BLUE),
                text(label).size(12).color(if is_sel { TEXT_PRIMARY } else { TEXT_SECONDARY }),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .padding([4, 8])
        .style(ghost_button)
        .width(Length::Fill)
        .on_press(Message::SetExportFormat(fmt));
        format_col = format_col.push(btn);
    }
    col = col.push(container(format_col).padding(8).style(card_container));

    // Resize Long Edge
    let resizes = [
        (ResizeLongEdge::Original, "Original (Full Res)"),
        (ResizeLongEdge::Px4000, "4000px"),
        (ResizeLongEdge::Px2000, "2000px"),
        (ResizeLongEdge::Px1920, "1920px (FHD)"),
        (ResizeLongEdge::Px1366, "1366px"),
        (ResizeLongEdge::Px800, "800px"),
    ];

    let mut resize_col = column![text("Resize (Long Edge)").size(13).color(TEXT_SECONDARY)].spacing(4);
    for (res, label) in resizes {
        let is_sel = app.export_options.resize == res;
        let btn = button(
            row![
                text(if is_sel { "●" } else { "○" }).size(12).color(ACCENT_BLUE),
                text(label).size(12).color(if is_sel { TEXT_PRIMARY } else { TEXT_SECONDARY }),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .padding([4, 8])
        .style(ghost_button)
        .width(Length::Fill)
        .on_press(Message::SetExportResize(res));
        resize_col = resize_col.push(btn);
    }
    col = col.push(container(resize_col).padding(8).style(card_container));

    // Preserve EXIF
    let exif_check = checkbox(app.export_options.preserve_exif)
        .label("Preserve Camera EXIF Metadata")
        .on_toggle(Message::SetExportPreserveExif)
        .size(14);
    col = col.push(exif_check);

    // Full-Res GPU Export button
    let export_btn = button(
        row![text(if app.is_exporting {
            "⏳ Exporting..."
        } else {
            "🚀 Export Full-Resolution Image..."
        })
        .size(13)]
        .align_y(Alignment::Center)
        .padding([10, 16]),
    )
    .style(primary_button)
    .width(Length::Fill);

    let export_btn = if app.is_exporting || app.source_image.is_none() {
        export_btn
    } else {
        export_btn.on_press(Message::StartExport)
    };
    col = col.push(export_btn);

    // Save .snap sidecar
    let sidecar_btn = button(text("💾 Save .snap Sidecar (Ctrl+S)").size(12))
        .padding([8, 14])
        .style(ghost_button)
        .width(Length::Fill)
        .on_press(Message::SaveSidecar);
    col = col.push(sidecar_btn);

    col.into()
}

fn render_view_edits_tab(app: &SnapApp) -> Element<'_, Message> {
    let mut col = column![].spacing(8).padding(10);
    let stack = app.document_history.current();

    if stack.nodes.is_empty() {
        return container(
            column![
                text("No edits in stack yet.").size(13).color(TEXT_MUTED),
                text("Open any tool from the Tools or Looks tab to apply non-destructive adjustments!")
                    .size(12)
                    .color(TEXT_MUTED),
            ]
            .spacing(6),
        )
        .padding(16)
        .into();
    }

    col = col.push(
        row![
            text(format!("{} Edit Steps", stack.nodes.len()))
                .size(13)
                .color(TEXT_SECONDARY),
            button(text("Reset Preview").size(11))
                .style(ghost_button)
                .padding([2, 6])
                .on_press(Message::PreviewStep(None)),
        ]
        .spacing(12)
        .align_y(Alignment::Center),
    );

    for (idx, node) in stack.nodes.iter().enumerate() {
        let id = node.id;
        let is_preview_step = app.active_view_edits_step == Some(idx);

        let toggle_btn = button(text(if node.enabled { "👁" } else { "⊘" }).size(13))
            .padding([4, 6])
            .style(ghost_button)
            .on_press(Message::ToggleNode(id));

        let title_btn = button(
            row![
                text(format!("{}. {}", idx + 1, node.op.tool_id().display_name()))
                    .size(12)
                    .color(if node.enabled { TEXT_PRIMARY } else { TEXT_MUTED }),
            ]
            .align_y(Alignment::Center),
        )
        .padding([4, 6])
        .style(ghost_button)
        .width(Length::Fill)
        .on_press(Message::PreviewStep(Some(idx)));

        let edit_btn = button(text("Edit").size(11))
            .padding([3, 6])
            .style(ghost_button)
            .on_press(Message::EditExistingNode(id));

        let up_btn = button(text("▲").size(10))
            .padding([2, 4])
            .style(ghost_button)
            .on_press(Message::MoveNodeUp(id));

        let down_btn = button(text("▼").size(10))
            .padding([2, 4])
            .style(ghost_button)
            .on_press(Message::MoveNodeDown(id));

        let del_btn = button(text("✕").size(11).color(ACCENT_RED))
            .padding([3, 6])
            .style(ghost_button)
            .on_press(Message::DeleteNode(id));

        let top_line = row![toggle_btn, title_btn, edit_btn, up_btn, down_btn, del_btn]
            .spacing(4)
            .align_y(Alignment::Center);

        let opacity_slider = row![
            text("Opacity:").size(11).color(TEXT_MUTED),
            slider(0.0..=1.0, node.opacity, move |v| Message::SetNodeOpacity(id, v))
                .step(0.01_f32),
            text(format!("{:.0}%", node.opacity * 100.0)).size(11).color(TEXT_MUTED),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let card = container(column![top_line, opacity_slider].spacing(4))
            .padding(6)
            .style(if is_preview_step { active_card_container } else { card_container });

        col = col.push(card);
    }

    col.into()
}
