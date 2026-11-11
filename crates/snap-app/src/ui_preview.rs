use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use iced::Size;
use iced_graphics::Viewport;
use iced_runtime::user_interface::Cache;
use iced_runtime::UserInterface;
use snap_core::curves::CurvePreset;
use snap_core::ops::ToolId;

use crate::app::{Message, SnapApp, Tab};
use crate::curves_widget::CurvesEvent;
use crate::theme;
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

/// Captures all 7 high-fidelity UI preview scenarios to PNG at 1440x900 resolution.
pub fn capture_all_previews(out_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(out_dir)?;

    let mut studio = OffscreenStudio::new(1440, 900)?;

    println!("Capturing UI preview 1/7: Welcome Studio...");
    // 1. preview_01_welcome.png
    let (mut app, _) = SnapApp::boot(None);
    app.config.recent_files = vec![
        PathBuf::from("/home/user/Pictures/dolomites_sunrise.dng"),
        PathBuf::from("/home/user/Pictures/street_portrait.jpg"),
    ];
    studio.render_scenario_to_png(app, &out_dir.join("preview_01_welcome.png"))?;

    println!("Capturing UI preview 2/7: Tools Grid...");
    // 2. preview_02_tools_grid.png
    let (mut app, _) = SnapApp::boot(None);
    let _ = app.update(Message::LoadDemoScene);
    app.current_tab = Tab::Tools;
    studio.render_scenario_to_png(app, &out_dir.join("preview_02_tools_grid.png"))?;

    println!("Capturing UI preview 3/7: Tune Image HUD...");
    // 3. preview_03_tune_hud.png
    let (mut app, _) = SnapApp::boot(None);
    let _ = app.update(Message::LoadDemoScene);
    let _ = app.update(Message::OpenTool(ToolId::TuneImage));
    let _ = app.update(Message::SetSliderParamLive(0, 22.0));
    let _ = app.update(Message::SetSliderParamLive(4, 38.0));
    let _ = app.update(Message::SetVerticalPickerActive(true));
    studio.render_scenario_to_png(app, &out_dir.join("preview_03_tune_hud.png"))?;

    println!("Capturing UI preview 4/7: Curves...");
    // 4. preview_04_curves.png
    let (mut app, _) = SnapApp::boot(None);
    let _ = app.update(Message::LoadDemoScene);
    let _ = app.update(Message::OpenTool(ToolId::Curves));
    let _ = app.update(Message::CurvesWidgetEvent(CurvesEvent::ApplyPreset(CurvePreset::Punch)));
    studio.render_scenario_to_png(app, &out_dir.join("preview_04_curves.png"))?;

    println!("Capturing UI preview 5/7: 3D LUT...");
    // 5. preview_05_hsl_lut.png
    let (mut app, _) = SnapApp::boot(None);
    let _ = app.update(Message::LoadDemoScene);
    let _ = app.update(Message::OpenTool(ToolId::Lut3d));
    let first_lut_id = app.lut_library.first().map(|l| l.id.clone()).unwrap_or_default();
    let _ = app.update(Message::SelectLut(first_lut_id));
    let _ = app.update(Message::SetLutStrength(85.0));
    studio.render_scenario_to_png(app, &out_dir.join("preview_05_hsl_lut.png"))?;

    println!("Capturing UI preview 6/7: Looks...");
    // 6. preview_06_looks.png
    let (mut app, _) = SnapApp::boot(None);
    let _ = app.update(Message::LoadDemoScene);
    let _ = app.update(Message::ApplyLook("pop".into()));
    app.current_tab = Tab::Looks;
    studio.render_scenario_to_png(app, &out_dir.join("preview_06_looks.png"))?;

    println!("Capturing UI preview 7/7: View Edits Stack...");
    // 7. preview_07_view_edits.png
    let (mut app, _) = SnapApp::boot(None);
    let _ = app.update(Message::LoadDemoScene);
    let _ = app.update(Message::OpenTool(ToolId::TuneImage));
    let _ = app.update(Message::SetSliderParamLive(0, 15.0));
    let _ = app.update(Message::ApplyActiveTool);
    let _ = app.update(Message::OpenTool(ToolId::Details));
    let _ = app.update(Message::SetSliderParamLive(0, 25.0));
    let _ = app.update(Message::ApplyActiveTool);
    let _ = app.update(Message::OpenTool(ToolId::Lut3d));
    let first_lut_id = app.lut_library.first().map(|l| l.id.clone()).unwrap_or_default();
    let _ = app.update(Message::SelectLut(first_lut_id));
    let _ = app.update(Message::ApplyActiveTool);
    app.current_tab = Tab::ViewEdits;
    studio.render_scenario_to_png(app, &out_dir.join("preview_07_view_edits.png"))?;

    println!("All 7 UI previews successfully captured to {}", out_dir.display());
    Ok(())
}
