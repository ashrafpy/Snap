use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use iced::Size;
use iced_graphics::Viewport;
use iced_runtime::user_interface::Cache;
use iced_runtime::UserInterface;
use snap_core::curves::CurvePreset;
use snap_core::mask::SmartMaskKind;
use snap_core::ops::{EditOp, ToolId};
use snap_core::ops_ext::{
    ColorGradeParams, ColorWheelValue, ColorWheelZone, FilmStock, FrameStyle, FramesParams,
    GrainyFilmParams, LensBlurParams, LensBlurShape, SelectiveParams, SelectivePin,
    TextColorPreset, TextOverlayParams, TextStyle,
};

use crate::app::{Message, SnapApp, Tab};
use crate::curves_widget::CurvesEvent;
use crate::theme;
use crate::tool_editors_ext::ToolEditorEvent;
use crate::views;

/// Offscreen Vulkan/WGPU rendering harness for pixel-perfect UI preview captures.
pub struct OffscreenStudio {
    renderer: iced::Renderer,
    viewport: Viewport,
    theme: iced::Theme,
}

impl OffscreenStudio {
    pub fn new(width: u32, height: u32) -> Result<Self> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .context("Failed to find suitable Vulkan/GPU adapter for offscreen rendering")?;

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("offscreen_ui_preview_device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::MemoryUsage,
                trace: wgpu::Trace::Off,
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
            },
        ))
        .context("Failed to request WGPU device")?;

        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let engine = iced_wgpu::Engine::new(
            &adapter,
            device,
            queue,
            format,
            None,
            iced_graphics::Shell::headless(),
        );

        let wgpu_renderer = iced_wgpu::Renderer::new(
            engine,
            iced::Font::DEFAULT,
            iced::Pixels(16.0),
        );

        let renderer = iced::Renderer::Primary(wgpu_renderer);
        let viewport = Viewport::with_physical_size(Size::new(width, height), 1.0);
        let theme = theme::snap_theme();

        Ok(Self {
            renderer,
            viewport,
            theme,
        })
    }

    pub fn render_scenario_to_png(&mut self, mut app: SnapApp, out_path: &Path) -> Result<()> {
        let logical_size = self.viewport.logical_size();
        let bg_color = theme::palette::BG_CANVAS;
        let text_color = self.theme.palette().text;

        // Pass 1: Warm-up pass to compile/render shader pipeline and compute live GPU histogram
        {
            let cache = Cache::new();
            let mut ui = UserInterface::build(
                views::view(&app),
                logical_size,
                cache,
                &mut self.renderer,
            );
            ui.draw(
                &mut self.renderer,
                &self.theme,
                &iced_core::renderer::Style { text_color },
                iced::mouse::Cursor::Unavailable,
            );
            let _ = match &mut self.renderer {
                iced::Renderer::Primary(w) => w.screenshot(&self.viewport, bg_color),
                _ => unreachable!(),
            };
        }

        // Copy computed histogram if populated by shader pipeline
        if let Ok(mut lock) = app.hist_slot.lock() {
            if let Some(hist) = lock.take() {
                app.histogram = Some(hist);
                app.hist_dirty = false;
            }
        }

        // Pass 2: Final pass with live GPU histogram and rendered HDR photo
        let bytes = {
            let cache = Cache::new();
            let mut ui = UserInterface::build(
                views::view(&app),
                logical_size,
                cache,
                &mut self.renderer,
            );
            ui.draw(
                &mut self.renderer,
                &self.theme,
                &iced_core::renderer::Style { text_color },
                iced::mouse::Cursor::Unavailable,
            );
            match &mut self.renderer {
                iced::Renderer::Primary(w) => w.screenshot(&self.viewport, bg_color),
                _ => unreachable!(),
            }
        };

        let width = self.viewport.physical_size().width;
        let height = self.viewport.physical_size().height;

        let img = image::RgbaImage::from_raw(width, height, bytes)
            .context("Failed to construct image::RgbaImage from raw RGBA bytes")?;

        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        img.save(out_path)
            .with_context(|| format!("Failed to save screenshot PNG to {}", out_path.display()))?;

        println!(
            "Captured UI preview ({}x{}): {}",
            width,
            height,
            out_path.display()
        );

        Ok(())
    }
}

/// Captures all 12 high-fidelity UI preview scenarios to PNG at 1440x900 resolution.
pub fn capture_all_previews(out_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(out_dir)?;

    let mut studio = OffscreenStudio::new(1440, 900)?;

    println!("Capturing UI preview 1/12: Welcome Studio...");
    // 1. preview_01_welcome.png
    let (mut app, _) = SnapApp::boot(None);
    app.config.recent_files = vec![
        PathBuf::from("/home/user/Pictures/dolomites_sunrise.dng"),
        PathBuf::from("/home/user/Pictures/street_portrait.jpg"),
    ];
    studio.render_scenario_to_png(app, &out_dir.join("preview_01_welcome.png"))?;

    println!("Capturing UI preview 2/12: Tools Grid...");
    // 2. preview_02_tools_grid.png (Categorized 35-Tool Studio Grid)
    let (mut app, _) = SnapApp::boot(None);
    let _ = app.update(Message::LoadDemoScene);
    app.current_tab = Tab::Tools;
    studio.render_scenario_to_png(app, &out_dir.join("preview_02_tools_grid.png"))?;

    println!("Capturing UI preview 3/12: Tune Image HUD...");
    // 3. preview_03_tune_hud.png
    let (mut app, _) = SnapApp::boot(None);
    let _ = app.update(Message::LoadDemoScene);
    let _ = app.update(Message::OpenTool(ToolId::TuneImage));
    let _ = app.update(Message::SetSliderParamLive(0, 22.0));
    let _ = app.update(Message::SetSliderParamLive(4, 38.0));
    let _ = app.update(Message::SetVerticalPickerActive(true));
    studio.render_scenario_to_png(app, &out_dir.join("preview_03_tune_hud.png"))?;

    println!("Capturing UI preview 4/12: Curves...");
    // 4. preview_04_curves.png
    let (mut app, _) = SnapApp::boot(None);
    let _ = app.update(Message::LoadDemoScene);
    let _ = app.update(Message::OpenTool(ToolId::Curves));
    let _ = app.update(Message::CurvesWidgetEvent(CurvesEvent::ApplyPreset(CurvePreset::Punch)));
    studio.render_scenario_to_png(app, &out_dir.join("preview_04_curves.png"))?;

    println!("Capturing UI preview 5/12: 3D LUT...");
    // 5. preview_05_hsl_lut.png
    let (mut app, _) = SnapApp::boot(None);
    let _ = app.update(Message::LoadDemoScene);
    let _ = app.update(Message::OpenTool(ToolId::Lut3d));
    let first_lut_id = app.lut_library.first().map(|l| l.id.clone()).unwrap_or_default();
    let _ = app.update(Message::SelectLut(first_lut_id));
    let _ = app.update(Message::SetLutStrength(85.0));
    studio.render_scenario_to_png(app, &out_dir.join("preview_05_hsl_lut.png"))?;

    println!("Capturing UI preview 6/12: Looks...");
    // 6. preview_06_looks.png
    let (mut app, _) = SnapApp::boot(None);
    let _ = app.update(Message::LoadDemoScene);
    let _ = app.update(Message::ApplyLook("pop".into()));
    app.current_tab = Tab::Looks;
    studio.render_scenario_to_png(app, &out_dir.join("preview_06_looks.png"))?;

    println!("Capturing UI preview 7/12: View Edits Stack (Layer Mask + Smart Mask)...");
    // 7. preview_07_view_edits.png (Expanded Layer Mask + 7 Smart Mask inspector & ruby-red mask overlay)
    let (mut app, _) = SnapApp::boot(None);
    let _ = app.update(Message::LoadDemoScene);
    let _ = app.update(Message::OpenTool(ToolId::TuneImage));
    let _ = app.update(Message::SetSliderParamLive(0, 15.0));
    let _ = app.update(Message::ApplyActiveTool);
    let _ = app.update(Message::OpenTool(ToolId::Details));
    let _ = app.update(Message::SetSliderParamLive(0, 25.0));
    let _ = app.update(Message::ApplyActiveTool);
    let _ = app.update(Message::OpenTool(ToolId::ColorGrade));
    let mut grade_op = ColorGradeParams::default();
    grade_op.shadows.saturation = 35.0;
    grade_op.shadows.hue_deg = 200.0;
    let _ = app.update(Message::ToolEditorExt(ToolEditorEvent::CommitOp(EditOp::ColorGrade(grade_op))));
    let _ = app.update(Message::ApplyActiveTool);

    // Expand mask on node 1 (Details) and apply Sky smart mask with ruby-red overlay
    if let Some(node) = app.document_history.current().nodes.get(1) {
        let node_id = node.id;
        let _ = app.update(Message::ToggleNodeMaskPanel(node_id));
        let _ = app.update(Message::ToolEditorExt(ToolEditorEvent::MaskApplySmart {
            node_id,
            kind: SmartMaskKind::Sky,
        }));
        let _ = app.update(Message::ToolEditorExt(ToolEditorEvent::MaskToggleRedOverlay(true)));
    }
    app.current_tab = Tab::ViewEdits;
    studio.render_scenario_to_png(app, &out_dir.join("preview_07_view_edits.png"))?;

    println!("Capturing UI preview 8/12: 3-Way Color Grade Wheels...");
    // 8. preview_08_color_grade_wheels.png
    let (mut app, _) = SnapApp::boot(None);
    let _ = app.update(Message::LoadDemoScene);
    let _ = app.update(Message::OpenTool(ToolId::ColorGrade));
    let mut grade = ColorGradeParams::default();
    grade.shadows = ColorWheelValue { hue_deg: 195.0, saturation: 48.0, luminance: -12.0 };
    grade.highlights = ColorWheelValue { hue_deg: 42.0, saturation: 52.0, luminance: 18.0 };
    grade.active_wheel = ColorWheelZone::Highlights;
    let _ = app.update(Message::ToolEditorExt(ToolEditorEvent::CommitOp(EditOp::ColorGrade(grade))));
    app.current_tab = Tab::Tools;
    studio.render_scenario_to_png(app, &out_dir.join("preview_08_color_grade_wheels.png"))?;

    println!("Capturing UI preview 9/12: Selective U-Point Pins...");
    // 9. preview_09_selective_upoint.png
    let (mut app, _) = SnapApp::boot(None);
    let _ = app.update(Message::LoadDemoScene);
    let _ = app.update(Message::OpenTool(ToolId::Selective));
    let mut sel = SelectiveParams::default();
    sel.pins = vec![
        SelectivePin { x: 0.38, y: 0.42, radius: 0.22, brightness: 28.0, contrast: 15.0, saturation: 10.0, structure: 8.0 },
        SelectivePin { x: 0.68, y: 0.58, radius: 0.26, brightness: -18.0, contrast: 20.0, saturation: -12.0, structure: 0.0 },
    ];
    sel.active_pin = 0;
    let _ = app.update(Message::ToolEditorExt(ToolEditorEvent::CommitOp(EditOp::Selective(sel))));
    app.current_tab = Tab::Tools;
    studio.render_scenario_to_png(app, &out_dir.join("preview_09_selective_upoint.png"))?;

    println!("Capturing UI preview 10/12: Halation & Grainy Film...");
    // 10. preview_10_halation_grain.png
    let (mut app, _) = SnapApp::boot(None);
    let _ = app.update(Message::LoadDemoScene);
    let _ = app.update(Message::OpenTool(ToolId::GrainyFilm));
    let film = GrainyFilmParams {
        stock: FilmStock::Cinestill800T,
        grain: 45.0,
        style_strength: 60.0,
    };
    let _ = app.update(Message::ToolEditorExt(ToolEditorEvent::CommitOp(EditOp::GrainyFilm(film))));
    let _ = app.update(Message::ApplyActiveTool);
    let _ = app.update(Message::OpenTool(ToolId::Halation));
    let _ = app.update(Message::SetSliderParamLive(0, 68.0));
    let _ = app.update(Message::SetSliderParamLive(1, 75.0));
    let _ = app.update(Message::SetSliderParamLive(2, 55.0));
    app.current_tab = Tab::Tools;
    studio.render_scenario_to_png(app, &out_dir.join("preview_10_halation_grain.png"))?;

    println!("Capturing UI preview 11/12: Lens Blur & Bokeh...");
    // 11. preview_11_lens_blur_bokeh.png
    let (mut app, _) = SnapApp::boot(None);
    let _ = app.update(Message::LoadDemoScene);
    let _ = app.update(Message::OpenTool(ToolId::LensBlur));
    let mut lblur = LensBlurParams::default();
    lblur.shape = LensBlurShape::Elliptical;
    lblur.center = [0.52, 0.46];
    lblur.inner_radius = 0.22;
    lblur.transition = 0.32;
    lblur.angle_deg = 15.0;
    lblur.blur_strength = 65.0;
    lblur.bokeh_boost = 55.0;
    lblur.vignette_strength = 20.0;
    let _ = app.update(Message::ToolEditorExt(ToolEditorEvent::CommitOp(EditOp::LensBlur(lblur))));
    app.current_tab = Tab::Tools;
    studio.render_scenario_to_png(app, &out_dir.join("preview_11_lens_blur_bokeh.png"))?;

    println!("Capturing UI preview 12/12: Frames & Text Overlay...");
    // 12. preview_12_frames_text.png
    let (mut app, _) = SnapApp::boot(None);
    let _ = app.update(Message::LoadDemoScene);
    let _ = app.update(Message::OpenTool(ToolId::Frames));
    let frames = FramesParams {
        style: FrameStyle::GalleryLine,
        width: 35.0,
        inner_line: true,
        aspect_pad: 20.0,
    };
    let _ = app.update(Message::ToolEditorExt(ToolEditorEvent::CommitOp(EditOp::Frames(frames))));
    let _ = app.update(Message::ApplyActiveTool);
    let _ = app.update(Message::OpenTool(ToolId::TextOverlay));
    let text_op = TextOverlayParams {
        text: "SNAP STUDIO".to_string(),
        subtext: "HDR REPAIR 2026".to_string(),
        style: TextStyle::EditorialSerif,
        center: [0.50, 0.78],
        scale: 1.25,
        opacity: 90.0,
        invert_mask: false,
        color_preset: TextColorPreset::Gold,
    };
    let _ = app.update(Message::ToolEditorExt(ToolEditorEvent::CommitOp(EditOp::TextOverlay(text_op))));
    app.current_tab = Tab::Tools;
    studio.render_scenario_to_png(app, &out_dir.join("preview_12_frames_text.png"))?;

    println!("All 12 UI previews successfully captured to {}", out_dir.display());
    Ok(())
}
