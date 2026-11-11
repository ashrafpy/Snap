use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use iced::{Point, Size, Task};
use uuid::Uuid;

use snap_core::config::{AppConfig, ExportFormat, ExportOptions, ResizeLongEdge};
use snap_core::curves::CurveChannelId;
use snap_core::image_types::{HistogramData, LinearImage, Lut3D, LutColorSpace};
use snap_core::looks::{built_in_looks, LookId, LookPreset};
use snap_core::mask::LayerMask;
use snap_core::ops::{
    AspectRatio, BwColorFilter, BwPreset, CropRect, CropRotateParams, EditOp, HslBand,
    Lut3dParams, ToolId, TuneParams, WhiteBalanceParams,
};
use snap_core::ops_ext::HealStroke;
use snap_core::stack::{EditNode, EditStack, HistoryManager, ToolSessionHistory};
use snap_gpu::headless::HeadlessGpu;
use snap_gpu::types::ViewportParams;
use crate::canvas_overlays::CanvasOverlayEvent;
use crate::curves_widget::CurvesEvent;
use crate::sample_scene::generate_sample_hdr_scene;
use crate::tool_editors_ext::ToolEditorEvent;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Looks,
    Tools,
    Export,
    ViewEdits,
}

#[derive(Clone, Debug)]
pub struct ActiveToolSession {
    pub node_id: Option<Uuid>,
    pub tool: ToolId,
    pub live_op: EditOp,
    pub history: ToolSessionHistory,
    pub selected_param_idx: usize,
    pub vertical_picker_active: bool,
    pub horizontal_adjust_active: bool,
    pub eyedropper_active: bool,
    pub vignette_center_mode: bool,
    pub current_curve_channel: CurveChannelId,
    pub current_hsl_band: HslBand,
}

impl ActiveToolSession {
    pub fn new_tool(tool: ToolId) -> Self {
        let op = EditOp::default_for_tool(tool);
        Self {
            node_id: None,
            tool,
            live_op: op.clone(),
            history: ToolSessionHistory::new(op),
            selected_param_idx: 0,
            vertical_picker_active: false,
            horizontal_adjust_active: false,
            eyedropper_active: false,
            vignette_center_mode: false,
            current_curve_channel: CurveChannelId::Rgb,
            current_hsl_band: HslBand::Red,
        }
    }

    pub fn for_node(node: &EditNode) -> Self {
        let tool = node.op.tool_id();
        Self {
            node_id: Some(node.id),
            tool,
            live_op: node.op.clone(),
            history: ToolSessionHistory::new(node.op.clone()),
            selected_param_idx: 0,
            vertical_picker_active: false,
            horizontal_adjust_active: false,
            eyedropper_active: false,
            vignette_center_mode: false,
            current_curve_channel: CurveChannelId::Rgb,
            current_hsl_band: HslBand::Red,
        }
    }
}

pub struct SnapApp {
    pub config: AppConfig,
    pub current_image_path: Option<PathBuf>,
    pub source_image: Option<Arc<LinearImage>>,
    pub image_id: u64,
    pub document_history: HistoryManager<EditStack>,
    pub active_tool_session: Option<ActiveToolSession>,
    pub viewport_params: ViewportParams,
    pub bypass_edits: bool,
    pub show_histogram: bool,
    pub histogram: Option<HistogramData>,
    pub hist_slot: Arc<Mutex<Option<HistogramData>>>,
    pub hist_dirty: bool,
    pub lut_library: Vec<Lut3D>,
    pub custom_looks: Vec<LookPreset>,
    pub copied_stack: Option<EditStack>,
    pub current_tab: Tab,
    pub active_view_edits_step: Option<usize>,
    pub export_options: ExportOptions,
    pub status_message: Option<(String, Instant)>,
    pub is_exporting: bool,
    pub save_look_name_input: String,
    pub selective_add_pin_mode: bool,
    pub secondary_images: HashMap<String, Arc<LinearImage>>,
    pub mask_edit_node_id: Option<Uuid>,
    pub mask_paint_mode: bool,
    pub mask_eraser: bool,
    pub mask_brush_radius: f32,
    pub mask_brush_hardness: f32,
    pub mask_target_opacity: f32,
    pub mask_red_overlay: bool,
    pub active_heal_stroke: Option<HealStroke>,
    pub mask_stroke_backup: Option<EditStack>,
}

#[derive(Debug, Clone)]
pub enum Message {
    // File I/O
    OpenFileDialog,
    OpenRecentFile(PathBuf),
    FileLoaded(Result<(PathBuf, LinearImage, Option<EditStack>), String>),
    LoadDemoScene,
    FileDropped(PathBuf),
    SaveSidecar,
    SidecarSaved(Result<PathBuf, String>),

    // Tabs & Navigation
    SelectTab(Tab),
    ToggleSidebar,
    ToggleHistogram,

    // Active Tool Session
    OpenTool(ToolId),
    EditExistingNode(Uuid),
    ApplyActiveTool,
    CancelActiveTool,
    SetSliderParamLive(usize, f32),
    SetSliderParam(usize, f32),
    SelectSliderParam(usize),
    SetVerticalPickerActive(bool),
    SetHorizontalAdjustActive(bool),
    ToolUndo,
    ToolRedo,
    Undo,
    Redo,

    // Gestures & Viewport
    CanvasZoomWheel {
        delta_y: f32,
        cursor_pos: Point,
        bounds_size: Size,
    },
    SetPan([f32; 2]),
    FitView,
    Zoom100,
    ZoomStep(f32),
    CanvasGestureEnd,
    CanvasClickEyedropper { norm_x: f32, norm_y: f32 },
    CanvasClickVignetteCenter { norm_x: f32, norm_y: f32 },
    SetCropRect(CropRect),
    SetBypassEdits(bool),
    ToggleBypassEdits,

    // Curves
    CurvesWidgetEvent(CurvesEvent),

    // Tool-specific Actions
    TuneAuto,
    ToggleEyedropper,
    ToggleVignetteCenter,
    SelectHslBand(HslBand),
    SetHslValue(usize, f32),
    ImportLutDialog,
    LutImported(Result<Lut3D, String>),
    SelectLut(String),
    SetLutStrength(f32),
    SetLutSpace(LutColorSpace),
    SelectBwPreset(BwPreset),
    SelectBwFilter(BwColorFilter),
    SetCropAspect(AspectRatio),
    SwapCropAspect,
    RotateCw,
    RotateCcw,
    FlipH,
    FlipV,
    ResetCrop,

    // Looks
    ApplyLook(LookId),
    LookNameInputChanged(String),
    SaveCurrentLook,
    CopyStack,
    PasteStack,

    // Favorites
    ToggleFavorite(ToolId),

    // View Edits Stack
    ToggleNode(Uuid),
    SetNodeOpacity(Uuid, f32),
    DeleteNode(Uuid),
    MoveNodeUp(Uuid),
    MoveNodeDown(Uuid),
    PreviewStep(Option<usize>),

    // Export
    SetExportFormat(ExportFormat),
    SetExportResize(ResizeLongEdge),
    SetExportPreserveExif(bool),
    StartExport,
    ExportFinished(Result<(PathBuf, u32, u32, std::time::Duration), String>),

    // Phase 2-4 Extensions
    ToolEditorExt(ToolEditorEvent),
    CanvasOverlay(CanvasOverlayEvent),
    ToggleNodeMaskPanel(Uuid),
    ImportXmpDialog,
    XmpFileSelected(Option<PathBuf>),
    DoubleExposureFileSelected(Option<PathBuf>),
}

impl SnapApp {
    pub fn boot(initial_path: Option<PathBuf>) -> (Self, Task<Message>) {
        let config = AppConfig::load();
        let custom_looks = AppConfig::load_custom_looks().unwrap_or_default();
        let lut_library = snap_io::load_all_luts();

        let app = Self {
            config,
            current_image_path: None,
            source_image: None,
            image_id: 1,
            document_history: HistoryManager::new(EditStack::default()),
            active_tool_session: None,
            viewport_params: {
                let mut vp = ViewportParams::default();
                vp.zoom = 0.90;
                vp
            },
            bypass_edits: false,
            show_histogram: true,
            histogram: None,
            hist_slot: Arc::new(Mutex::new(None)),
            hist_dirty: false,
            lut_library,
            custom_looks,
            copied_stack: None,
            current_tab: Tab::Looks,
            active_view_edits_step: None,
            export_options: ExportOptions::default(),
            status_message: None,
            is_exporting: false,
            save_look_name_input: String::new(),
            selective_add_pin_mode: false,
            secondary_images: HashMap::new(),
            mask_edit_node_id: None,
            mask_paint_mode: false,
            mask_eraser: false,
            mask_brush_radius: 0.07,
            mask_brush_hardness: 0.55,
            mask_target_opacity: 1.0,
            mask_red_overlay: false,
            active_heal_stroke: None,
            mask_stroke_backup: None,
        };

        if let Some(path) = initial_path {
            if snap_io::is_supported_image_path(&path) {
                let p = path.clone();
                let task = Task::perform(
                    async move { Self::load_image_and_sidecar(p).await },
                    Message::FileLoaded,
                );
                return (app, task);
            } else if snap_io::is_supported_lut_path(&path) {
                let p = path.clone();
                let task = Task::perform(
                    async move { Self::install_and_load_lut(p).await },
                    Message::LutImported,
                );
                return (app, task);
            } else if snap_io::is_xmp_path(&path) {
                let p = path.clone();
                let task = Task::perform(
                    async move { Some(p) },
                    Message::XmpFileSelected,
                );
                return (app, task);
            }
        }

        (app, Task::none())
    }

    pub fn preload_secondary_images_for_stack(&mut self, stack: &EditStack) {
        for node in &stack.nodes {
            if let EditOp::DoubleExposure(ref p) = node.op {
                if let Some(ref path_str) = p.secondary_path {
                    if !self.secondary_images.contains_key(path_str) {
                        if let Ok(img) = snap_io::load_image(std::path::Path::new(path_str)) {
                            self.secondary_images.insert(path_str.clone(), Arc::new(img));
                        }
                    }
                }
            }
        }
    }

    pub fn effective_stack(&self) -> EditStack {
        let mut base_stack = self.document_history.current().clone();

        // If step-by-step preview is active in View Edits
        if let Some(step_idx) = self.active_view_edits_step {
            base_stack.nodes.truncate(step_idx + 1);
        }

        // Overlay active tool session op
        if let Some(ref session) = self.active_tool_session {
            let active_op = session.live_op.clone();
            if let Some(id) = session.node_id {
                base_stack.update_node(id, active_op);
            } else {
                base_stack.push_op(active_op);
            }
        }

        base_stack
    }

    pub fn set_status(&mut self, text: impl Into<String>) {
        self.status_message = Some((text.into(), Instant::now()));
    }

    // Async file helper
    async fn load_image_and_sidecar(
        path: PathBuf,
    ) -> Result<(PathBuf, LinearImage, Option<EditStack>), String> {
        tokio::task::spawn_blocking(move || {
            let img = snap_io::load_image(&path).map_err(|e| e.to_string())?;
            let sidecar = snap_io::load_sidecar(&path).unwrap_or(None);
            Ok((path, img, sidecar))
        })
        .await
        .map_err(|e| e.to_string())?
    }

    // Async LUT install helper
    async fn install_and_load_lut(path: PathBuf) -> Result<Lut3D, String> {
        tokio::task::spawn_blocking(move || {
            let mut installed = snap_io::install_lut_file(&path).map_err(|e| e.to_string())?;
            installed.pop().ok_or_else(|| "No LUT found in file".to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        // Clear old hist slot if prepared
        if let Ok(mut lock) = self.hist_slot.try_lock() {
            if let Some(hist) = lock.take() {
                self.histogram = Some(hist);
                self.hist_dirty = false;
            }
        }

        match message {
            Message::OpenFileDialog => {
                let task = Task::perform(
                    async {
                        let handle = rfd::AsyncFileDialog::new()
                            .add_filter(
                                "Supported Images",
                                &[
                                    "jpg", "jpeg", "png", "tiff", "tif", "webp", "bmp", "dng",
                                    "cr2", "cr3", "nef", "arw", "raf", "orf", "rw2",
                                ],
                            )
                            .pick_file()
                            .await;

                        if let Some(file) = handle {
                            let path = file.path().to_path_buf();
                            Self::load_image_and_sidecar(path).await
                        } else {
                            Err("File selection cancelled".to_string())
                        }
                    },
                    Message::FileLoaded,
                );
                return task;
            }

            Message::OpenRecentFile(path) => {
                let p = path.clone();
                return Task::perform(
                    async move { Self::load_image_and_sidecar(p).await },
                    Message::FileLoaded,
                );
            }

            Message::FileLoaded(Ok((path, img, sidecar))) => {
                self.current_image_path = Some(path.clone());
                self.source_image = Some(Arc::new(img));
                self.image_id = self.image_id.wrapping_add(1);
                self.active_tool_session = None;
                self.hist_dirty = true;
                let mut vp = ViewportParams::default();
                vp.zoom = 0.90;
                self.viewport_params = vp;

                // Add to recent files in config
                if let Some(pos) = self.config.recent_files.iter().position(|p| p == &path) {
                    self.config.recent_files.remove(pos);
                }
                self.config.recent_files.insert(0, path.clone());
                self.config.recent_files.truncate(10);
                let _ = self.config.save();

                // Load sidecar if present
                if let Some(stack) = sidecar {
                    self.preload_secondary_images_for_stack(&stack);
                    self.document_history = HistoryManager::new(stack);
                    self.set_status(format!("Opened {} (with .snap edits)", path.display()));
                } else {
                    self.document_history = HistoryManager::new(EditStack::default());
                    self.set_status(format!("Opened {}", path.display()));
                }
            }

            Message::FileLoaded(Err(err)) => {
                if err != "File selection cancelled" {
                    self.set_status(format!("Error loading photo: {err}"));
                }
            }

            Message::LoadDemoScene => {
                let img = generate_sample_hdr_scene();
                self.source_image = Some(Arc::new(img));
                self.image_id = self.image_id.wrapping_add(1);
                self.current_image_path = None;
                self.document_history = HistoryManager::new(EditStack::default());
                self.active_tool_session = None;
                self.hist_dirty = true;
                let mut vp = ViewportParams::default();
                vp.zoom = 0.90;
                self.viewport_params = vp;
                self.set_status("Loaded Sample HDR Test Scene (1600x1066 Linear Rec.2020)");
            }

            Message::FileDropped(path) => {
                if snap_io::is_supported_image_path(&path) {
                    let p = path.clone();
                    return Task::perform(
                        async move { Self::load_image_and_sidecar(p).await },
                        Message::FileLoaded,
                    );
                } else if snap_io::is_supported_lut_path(&path) {
                    let p = path.clone();
                    return Task::perform(
                        async move { Self::install_and_load_lut(p).await },
                        Message::LutImported,
                    );
                } else if snap_io::is_xmp_path(&path) {
                    let p = path.clone();
                    return Task::perform(
                        async move { Some(p) },
                        Message::XmpFileSelected,
                    );
                } else {
                    self.set_status(format!("Unsupported file: {}", path.display()));
                }
            }

            Message::SaveSidecar => {
                if let Some(ref path) = self.current_image_path {
                    let stack = self.document_history.current().clone();
                    let target_path = path.clone();
                    return Task::perform(
                        async move {
                            tokio::task::spawn_blocking(move || {
                                snap_io::save_sidecar(&target_path, &stack)
                                    .map(|_| snap_io::sidecar_path_for(&target_path))
                                    .map_err(|e| e.to_string())
                            })
                            .await
                            .map_err(|e| e.to_string())?
                        },
                        Message::SidecarSaved,
                    );
                }
            }

            Message::SidecarSaved(Ok(path)) => {
                self.set_status(format!("Saved sidecar: {}", path.display()));
            }

            Message::SidecarSaved(Err(e)) => {
                self.set_status(format!("Failed to save sidecar: {e}"));
            }

            Message::SelectTab(tab) => {
                self.current_tab = tab;
                if tab != Tab::Tools {
                    self.selective_add_pin_mode = false;
                }
                if tab != Tab::ViewEdits {
                    self.mask_paint_mode = false;
                }
            }

            Message::ToggleSidebar => {
                self.config.show_sidebar = !self.config.show_sidebar;
                let _ = self.config.save();
            }

            Message::ToggleHistogram => {
                self.show_histogram = !self.show_histogram;
            }

            Message::OpenTool(tool) => {
                self.selective_add_pin_mode = false;
                self.active_heal_stroke = None;
                self.active_tool_session = Some(ActiveToolSession::new_tool(tool));
                self.hist_dirty = true;
                if tool == ToolId::CropRotate {
                    self.viewport_params.crop_preview_mode = true;
                }
            }

            Message::EditExistingNode(id) => {
                self.selective_add_pin_mode = false;
                self.active_heal_stroke = None;
                if let Some(node) = self.document_history.current().nodes.iter().find(|n| n.id == id) {
                    self.active_tool_session = Some(ActiveToolSession::for_node(node));
                    self.hist_dirty = true;
                    if node.op.tool_id() == ToolId::CropRotate {
                        self.viewport_params.crop_preview_mode = true;
                    }
                }
            }

            Message::ApplyActiveTool => {
                self.selective_add_pin_mode = false;
                self.active_heal_stroke = None;
                if let Some(session) = self.active_tool_session.take() {
                    let op = session.live_op;
                    let mut stack = self.document_history.current().clone();

                    if let Some(id) = session.node_id {
                        stack.update_node(id, op);
                    } else if !op.is_identity() {
                        stack.push_op(op);
                    }

                    self.preload_secondary_images_for_stack(&stack);
                    self.document_history.commit(stack);
                    self.viewport_params.crop_preview_mode = false;
                    self.hist_dirty = true;
                    self.set_status(format!("Applied {}", session.tool.display_name()));
                }
            }

            Message::CancelActiveTool => {
                self.selective_add_pin_mode = false;
                self.active_heal_stroke = None;
                if let Some(session) = self.active_tool_session.take() {
                    self.viewport_params.crop_preview_mode = false;
                    self.hist_dirty = true;
                    self.set_status(format!("Cancelled {}", session.tool.display_name()));
                }
            }

            Message::SetSliderParamLive(idx, val) => {
                if let Some(ref mut session) = self.active_tool_session {
                    session.live_op.set_slider_param(idx, val);
                    self.hist_dirty = true;
                }
            }

            Message::SetSliderParam(idx, val) => {
                if let Some(ref mut session) = self.active_tool_session {
                    session.live_op.set_slider_param(idx, val);
                    if &session.live_op != session.history.current() {
                        session.history.commit(session.live_op.clone());
                    }
                    self.hist_dirty = true;
                }
            }

            Message::SelectSliderParam(idx) => {
                if let Some(ref mut session) = self.active_tool_session {
                    session.selected_param_idx = idx;
                }
            }

            Message::SetVerticalPickerActive(active) => {
                if let Some(ref mut session) = self.active_tool_session {
                    session.vertical_picker_active = active;
                }
            }

            Message::SetHorizontalAdjustActive(active) => {
                if let Some(ref mut session) = self.active_tool_session {
                    session.horizontal_adjust_active = active;
                }
            }

            Message::ToolUndo => {
                if let Some(ref mut session) = self.active_tool_session {
                    if let Some(prev) = session.history.undo() {
                        session.live_op = prev.clone();
                        self.hist_dirty = true;
                    }
                }
            }

            Message::ToolRedo => {
                if let Some(ref mut session) = self.active_tool_session {
                    if let Some(next) = session.history.redo() {
                        session.live_op = next.clone();
                        self.hist_dirty = true;
                    }
                }
            }

            Message::Undo => {
                if self.active_tool_session.is_some() {
                    return self.update(Message::ToolUndo);
                } else if self.document_history.can_undo() {
                    let _ = self.document_history.undo();
                    self.hist_dirty = true;
                    self.set_status("Undo");
                }
            }

            Message::Redo => {
                if self.active_tool_session.is_some() {
                    return self.update(Message::ToolRedo);
                } else if self.document_history.can_redo() {
                    let _ = self.document_history.redo();
                    self.hist_dirty = true;
                    self.set_status("Redo");
                }
            }

            Message::CanvasZoomWheel {
                delta_y,
                cursor_pos,
                bounds_size,
            } => {
                let old_zoom = self.viewport_params.zoom;
                let factor = if delta_y > 0.0 { 1.15 } else { 1.0 / 1.15 };
                let new_zoom = (old_zoom * factor).clamp(0.08, 10.0);

                // Zoom relative to cursor point
                let center_x = bounds_size.width * 0.5 + self.viewport_params.pan[0];
                let center_y = bounds_size.height * 0.5 + self.viewport_params.pan[1];

                let rel_x = cursor_pos.x - center_x;
                let rel_y = cursor_pos.y - center_y;

                let zoom_ratio = new_zoom / old_zoom;
                self.viewport_params.pan[0] -= rel_x * (zoom_ratio - 1.0);
                self.viewport_params.pan[1] -= rel_y * (zoom_ratio - 1.0);
                self.viewport_params.zoom = new_zoom;
            }

            Message::SetPan(pan) => {
                self.viewport_params.pan = pan;
            }

            Message::FitView => {
                self.viewport_params.pan = [0.0, 0.0];
                self.viewport_params.zoom = 0.90;
            }

            Message::Zoom100 => {
                self.viewport_params.pan = [0.0, 0.0];
                self.viewport_params.zoom = 1.0;
            }

            Message::ZoomStep(factor) => {
                self.viewport_params.zoom = (self.viewport_params.zoom * factor).clamp(0.08, 10.0);
            }

            Message::CanvasGestureEnd => {
                if let Some(ref mut session) = self.active_tool_session {
                    session.vertical_picker_active = false;
                    session.horizontal_adjust_active = false;
                    if &session.live_op != session.history.current() {
                        session.history.commit(session.live_op.clone());
                    }
                }
            }

            Message::CanvasClickEyedropper { norm_x, norm_y } => {
                if let Some(ref img) = self.source_image {
                    let px_x = ((norm_x * img.width as f32) as u32).min(img.width.saturating_sub(1));
                    let px_y = ((norm_y * img.height as f32) as u32).min(img.height.saturating_sub(1));
                    let px_idx = (px_y * img.width + px_x) as usize;
                    if px_idx < img.pixels.len() {
                        let px = img.pixels[px_idx];
                        let rgb = [px[0].to_f32(), px[1].to_f32(), px[2].to_f32()];
                        let wb = WhiteBalanceParams::from_neutral_rec2020_sample(rgb);

                        if let Some(ref mut session) = self.active_tool_session {
                            if let EditOp::WhiteBalance(p) = &mut session.live_op {
                                p.temperature = wb.temperature;
                                p.tint = wb.tint;
                            }
                            if &session.live_op != session.history.current() {
                                session.history.commit(session.live_op.clone());
                            }
                            session.eyedropper_active = false;
                            self.hist_dirty = true;
                            self.set_status(format!(
                                "Eyedropper neutralized: Temp {:+.1}, Tint {:+.1}",
                                wb.temperature, wb.tint
                            ));
                        }
                    }
                }
            }

            Message::CanvasClickVignetteCenter { norm_x, norm_y } => {
                if let Some(ref mut session) = self.active_tool_session {
                    if let EditOp::Vignette(p) = &mut session.live_op {
                        p.center = [norm_x, norm_y];
                    }
                    if &session.live_op != session.history.current() {
                        session.history.commit(session.live_op.clone());
                    }
                    session.vignette_center_mode = false;
                    self.hist_dirty = true;
                    self.set_status(format!("Vignette center set to ({:.2}, {:.2})", norm_x, norm_y));
                }
            }

            Message::SetCropRect(rect) => {
                self.viewport_params.crop.rect = rect;
                if let Some(ref mut session) = self.active_tool_session {
                    if let EditOp::CropRotate(p) = &mut session.live_op {
                        p.rect = rect;
                    }
                }
            }

            Message::SetBypassEdits(bypass) => {
                self.bypass_edits = bypass;
            }

            Message::ToggleBypassEdits => {
                self.bypass_edits = !self.bypass_edits;
            }

            Message::CurvesWidgetEvent(ev) => {
                match ev {
                    CurvesEvent::PointAdded { channel, point } => {
                        if let Some(ref mut session) = self.active_tool_session {
                            if let EditOp::Curves(c) = &mut session.live_op {
                                match channel {
                                    CurveChannelId::Rgb => { c.rgb.add_or_move_point(point.x, point.y); }
                                    CurveChannelId::Red => { c.red.add_or_move_point(point.x, point.y); }
                                    CurveChannelId::Green => { c.green.add_or_move_point(point.x, point.y); }
                                    CurveChannelId::Blue => { c.blue.add_or_move_point(point.x, point.y); }
                                    CurveChannelId::Luminance => { c.luma.add_or_move_point(point.x, point.y); }
                                }
                            }
                            if &session.live_op != session.history.current() {
                                session.history.commit(session.live_op.clone());
                            }
                            self.hist_dirty = true;
                        }
                    }

                    CurvesEvent::PointMoved { channel, index, point } => {
                        if let Some(ref mut session) = self.active_tool_session {
                            if let EditOp::Curves(c) = &mut session.live_op {
                                let ch = match channel {
                                    CurveChannelId::Rgb => &mut c.rgb,
                                    CurveChannelId::Red => &mut c.red,
                                    CurveChannelId::Green => &mut c.green,
                                    CurveChannelId::Blue => &mut c.blue,
                                    CurveChannelId::Luminance => &mut c.luma,
                                };
                                if index < ch.points.len() {
                                    ch.points[index] = point;
                                }
                            }
                            self.hist_dirty = true;
                        }
                    }

                    CurvesEvent::PointRemoved { channel, index } => {
                        if let Some(ref mut session) = self.active_tool_session {
                            if let EditOp::Curves(c) = &mut session.live_op {
                                match channel {
                                    CurveChannelId::Rgb => { c.rgb.remove_point(index); }
                                    CurveChannelId::Red => { c.red.remove_point(index); }
                                    CurveChannelId::Green => { c.green.remove_point(index); }
                                    CurveChannelId::Blue => { c.blue.remove_point(index); }
                                    CurveChannelId::Luminance => { c.luma.remove_point(index); }
                                }
                            }
                            if &session.live_op != session.history.current() {
                                session.history.commit(session.live_op.clone());
                            }
                            self.hist_dirty = true;
                        }
                    }

                    CurvesEvent::SelectChannel(channel) => {
                        if let Some(ref mut session) = self.active_tool_session {
                            session.current_curve_channel = channel;
                        }
                    }

                    CurvesEvent::ApplyPreset(preset) => {
                        if let Some(ref mut session) = self.active_tool_session {
                            let params = preset.to_curves_params();
                            session.live_op = EditOp::Curves(params);
                            session.history.commit(session.live_op.clone());
                            self.hist_dirty = true;
                        }
                    }

                    CurvesEvent::ResetChannel(channel) => {
                        if let Some(ref mut session) = self.active_tool_session {
                            if let EditOp::Curves(c) = &mut session.live_op {
                                match channel {
                                    CurveChannelId::Rgb => c.rgb = Default::default(),
                                    CurveChannelId::Red => c.red = Default::default(),
                                    CurveChannelId::Green => c.green = Default::default(),
                                    CurveChannelId::Blue => c.blue = Default::default(),
                                    CurveChannelId::Luminance => c.luma = Default::default(),
                                }
                            }
                            if &session.live_op != session.history.current() {
                                session.history.commit(session.live_op.clone());
                            }
                            self.hist_dirty = true;
                        }
                    }
                }
            }

            Message::TuneAuto => {
                if let Some(ref hist) = self.histogram {
                    let auto_params = TuneParams::auto_from_histogram(hist);
                    if let Some(ref mut session) = self.active_tool_session {
                        session.live_op = EditOp::TuneImage(auto_params);
                        session.history.commit(session.live_op.clone());
                        self.hist_dirty = true;
                        self.set_status("Auto-Adjust tone & ambiance applied!");
                    }
                }
            }

            Message::ToggleEyedropper => {
                if let Some(ref mut session) = self.active_tool_session {
                    session.eyedropper_active = !session.eyedropper_active;
                    if session.eyedropper_active {
                        self.set_status("Click canvas to pick neutral gray");
                    }
                }
            }

            Message::ToggleVignetteCenter => {
                if let Some(ref mut session) = self.active_tool_session {
                    session.vignette_center_mode = !session.vignette_center_mode;
                    if session.vignette_center_mode {
                        self.set_status("Click canvas to place vignette center");
                    }
                }
            }

            Message::SelectHslBand(band) => {
                if let Some(ref mut session) = self.active_tool_session {
                    session.current_hsl_band = band;
                }
            }

            Message::SetHslValue(param_idx, val) => {
                if let Some(ref mut session) = self.active_tool_session {
                    let band_idx = session.current_hsl_band as usize;
                    let full_idx = band_idx * 3 + param_idx;
                    session.live_op.set_slider_param(full_idx, val);
                    self.hist_dirty = true;
                }
            }

            Message::ImportLutDialog => {
                let task = Task::perform(
                    async {
                        let handle = rfd::AsyncFileDialog::new()
                            .add_filter("3D LUTs", &["cube", "3dl", "png", "zip"])
                            .pick_file()
                            .await;

                        if let Some(file) = handle {
                            Self::install_and_load_lut(file.path().to_path_buf()).await
                        } else {
                            Err("Cancelled".to_string())
                        }
                    },
                    Message::LutImported,
                );
                return task;
            }

            Message::LutImported(Ok(lut)) => {
                let lut_id = lut.id.clone();
                let lut_name = lut.name.clone();
                self.lut_library.push(lut);

                // Open 3D LUT tool with this LUT selected
                let mut session = ActiveToolSession::new_tool(ToolId::Lut3d);
                let op = EditOp::Lut3d(Lut3dParams {
                    lut_id,
                    lut_name: lut_name.clone(),
                    strength: 100.0,
                    input_space: LutColorSpace::Srgb,
                });
                session.live_op = op.clone();
                session.history.commit(op);
                self.active_tool_session = Some(session);
                self.current_tab = Tab::Tools;
                self.hist_dirty = true;
                self.set_status(format!("Imported and selected LUT: {lut_name}"));
            }

            Message::LutImported(Err(err)) => {
                if err != "Cancelled" {
                    self.set_status(format!("LUT import error: {err}"));
                }
            }

            Message::SelectLut(id) => {
                if let Some(ref mut session) = self.active_tool_session {
                    if let EditOp::Lut3d(p) = &mut session.live_op {
                        p.lut_id = id.clone();
                        if let Some(found) = self.lut_library.iter().find(|l| l.id == id) {
                            p.lut_name = found.name.clone();
                            p.input_space = found.input_space;
                        }
                    }
                    if &session.live_op != session.history.current() {
                        session.history.commit(session.live_op.clone());
                    }
                    self.hist_dirty = true;
                }
            }

            Message::SetLutStrength(strength) => {
                if let Some(ref mut session) = self.active_tool_session {
                    if let EditOp::Lut3d(p) = &mut session.live_op {
                        p.strength = strength.clamp(0.0, 100.0);
                    }
                    if &session.live_op != session.history.current() {
                        session.history.commit(session.live_op.clone());
                    }
                    self.hist_dirty = true;
                }
            }

            Message::SetLutSpace(space) => {
                if let Some(ref mut session) = self.active_tool_session {
                    if let EditOp::Lut3d(p) = &mut session.live_op {
                        p.input_space = space;
                    }
                    if &session.live_op != session.history.current() {
                        session.history.commit(session.live_op.clone());
                    }
                    self.hist_dirty = true;
                }
            }

            Message::SelectBwPreset(preset) => {
                if let Some(ref mut session) = self.active_tool_session {
                    session.live_op = EditOp::BlackAndWhite(preset.to_params());
                    session.history.commit(session.live_op.clone());
                    self.hist_dirty = true;
                }
            }

            Message::SelectBwFilter(filter) => {
                if let Some(ref mut session) = self.active_tool_session {
                    if let EditOp::BlackAndWhite(p) = &mut session.live_op {
                        p.color_filter = filter;
                    }
                    if &session.live_op != session.history.current() {
                        session.history.commit(session.live_op.clone());
                    }
                    self.hist_dirty = true;
                }
            }

            Message::SetCropAspect(aspect) => {
                self.viewport_params.crop.aspect = aspect;
                if let Some(ref img) = self.source_image {
                    self.viewport_params.crop.constrain_rect(img.width, img.height);
                }
                if let Some(ref mut session) = self.active_tool_session {
                    if session.tool == ToolId::CropRotate {
                        session.live_op = EditOp::CropRotate(self.viewport_params.crop.clone());
                        session.history.commit(session.live_op.clone());
                        self.hist_dirty = true;
                    }
                }
            }

            Message::SwapCropAspect => {
                self.viewport_params.crop.aspect_landscape = !self.viewport_params.crop.aspect_landscape;
                if let Some(ref img) = self.source_image {
                    self.viewport_params.crop.constrain_rect(img.width, img.height);
                }
                if let Some(ref mut session) = self.active_tool_session {
                    if session.tool == ToolId::CropRotate {
                        session.live_op = EditOp::CropRotate(self.viewport_params.crop.clone());
                        session.history.commit(session.live_op.clone());
                        self.hist_dirty = true;
                    }
                }
            }

            Message::RotateCw => {
                self.viewport_params.crop.quarter_turns_cw =
                    (self.viewport_params.crop.quarter_turns_cw + 1) % 4;
                if let Some(ref mut session) = self.active_tool_session {
                    if session.tool == ToolId::CropRotate {
                        session.live_op = EditOp::CropRotate(self.viewport_params.crop.clone());
                        session.history.commit(session.live_op.clone());
                        self.hist_dirty = true;
                    }
                }
            }

            Message::RotateCcw => {
                self.viewport_params.crop.quarter_turns_cw =
                    (self.viewport_params.crop.quarter_turns_cw + 3) % 4;
                if let Some(ref mut session) = self.active_tool_session {
                    if session.tool == ToolId::CropRotate {
                        session.live_op = EditOp::CropRotate(self.viewport_params.crop.clone());
                        session.history.commit(session.live_op.clone());
                        self.hist_dirty = true;
                    }
                }
            }

            Message::FlipH => {
                self.viewport_params.crop.flip_h = !self.viewport_params.crop.flip_h;
                if let Some(ref mut session) = self.active_tool_session {
                    if session.tool == ToolId::CropRotate {
                        session.live_op = EditOp::CropRotate(self.viewport_params.crop.clone());
                        session.history.commit(session.live_op.clone());
                        self.hist_dirty = true;
                    }
                }
            }

            Message::FlipV => {
                self.viewport_params.crop.flip_v = !self.viewport_params.crop.flip_v;
                if let Some(ref mut session) = self.active_tool_session {
                    if session.tool == ToolId::CropRotate {
                        session.live_op = EditOp::CropRotate(self.viewport_params.crop.clone());
                        session.history.commit(session.live_op.clone());
                        self.hist_dirty = true;
                    }
                }
            }

            Message::ResetCrop => {
                self.viewport_params.crop = CropRotateParams::default();
                if let Some(ref mut session) = self.active_tool_session {
                    if session.tool == ToolId::CropRotate {
                        session.live_op = EditOp::CropRotate(self.viewport_params.crop.clone());
                        session.history.commit(session.live_op.clone());
                        self.hist_dirty = true;
                    }
                }
            }

            Message::ApplyLook(id) => {
                let look = built_in_looks()
                    .into_iter()
                    .chain(self.custom_looks.clone())
                    .find(|l| l.id == id);

                if let Some(l) = look {
                    self.preload_secondary_images_for_stack(&l.stack);
                    self.document_history.commit(l.stack);
                    self.hist_dirty = true;
                    self.set_status(format!("Applied look: {}", l.name));
                }
            }

            Message::LookNameInputChanged(name) => {
                self.save_look_name_input = name;
            }

            Message::SaveCurrentLook => {
                let name = self.save_look_name_input.trim();
                if !name.is_empty() {
                    let id = name.to_ascii_lowercase().replace(' ', "-");
                    let look = LookPreset {
                        id,
                        name: name.to_string(),
                        built_in: false,
                        stack: self.document_history.current().clone(),
                    };
                    if let Ok(()) = AppConfig::save_custom_look(&look) {
                        self.custom_looks.push(look);
                        self.set_status(format!("Saved custom look '{name}'"));
                        self.save_look_name_input.clear();
                    }
                }
            }

            Message::CopyStack => {
                self.copied_stack = Some(self.document_history.current().clone());
                self.set_status("Copied edit stack to clipboard");
            }

            Message::PasteStack => {
                if let Some(ref stack) = self.copied_stack {
                    self.document_history.commit(stack.clone());
                    self.hist_dirty = true;
                    self.set_status("Pasted edit stack");
                }
            }

            Message::ToggleFavorite(tool) => {
                if let Some(idx) = self.config.favorites.iter().position(|&t| t == tool) {
                    self.config.favorites.remove(idx);
                } else {
                    self.config.favorites.push(tool);
                }
                let _ = self.config.save();
            }

            Message::ToggleNode(id) => {
                let mut stack = self.document_history.current().clone();
                let _ = stack.toggle_node(id);
                self.document_history.commit(stack);
                self.hist_dirty = true;
            }

            Message::SetNodeOpacity(id, opacity) => {
                let mut stack = self.document_history.current().clone();
                if let Some(n) = stack.nodes.iter_mut().find(|n| n.id == id) {
                    n.opacity = opacity.clamp(0.0, 1.0);
                }
                self.document_history.commit(stack);
                self.hist_dirty = true;
            }

            Message::DeleteNode(id) => {
                let mut stack = self.document_history.current().clone();
                let _ = stack.remove_node(id);
                self.document_history.commit(stack);
                self.hist_dirty = true;
                self.set_status("Deleted edit step");
            }

            Message::MoveNodeUp(id) => {
                let mut stack = self.document_history.current().clone();
                if let Some(idx) = stack.nodes.iter().position(|n| n.id == id) {
                    if idx > 0 {
                        stack.move_node(idx, idx - 1);
                        self.document_history.commit(stack);
                        self.hist_dirty = true;
                    }
                }
            }

            Message::MoveNodeDown(id) => {
                let mut stack = self.document_history.current().clone();
                if let Some(idx) = stack.nodes.iter().position(|n| n.id == id) {
                    if idx + 1 < stack.nodes.len() {
                        stack.move_node(idx, idx + 1);
                        self.document_history.commit(stack);
                        self.hist_dirty = true;
                    }
                }
            }

            Message::PreviewStep(step) => {
                self.active_view_edits_step = step;
                self.hist_dirty = true;
            }

            Message::SetExportFormat(format) => {
                self.export_options.format = format;
            }

            Message::SetExportResize(resize) => {
                self.export_options.resize = resize;
            }

            Message::SetExportPreserveExif(preserve) => {
                self.export_options.preserve_exif = preserve;
            }

            Message::StartExport => {
                let Some(source) = self.source_image.clone() else {
                    return Task::none();
                };

                let effective_stack = self.effective_stack();
                let options = self.export_options;
                let needed_luts = self.lut_library.clone();

                let default_name = self
                    .current_image_path
                    .as_ref()
                    .and_then(|p| p.file_stem().and_then(|s| s.to_str()))
                    .unwrap_or("snap_export")
                    .to_string();

                let ext = match options.format {
                    ExportFormat::Jpeg { .. } => "jpg",
                    ExportFormat::Png8 | ExportFormat::Png16 => "png",
                    ExportFormat::Tiff16 => "tif",
                };

                self.is_exporting = true;
                self.set_status("Exporting full-resolution image on GPU...");

                let task = Task::perform(
                    async move {
                        let handle = rfd::AsyncFileDialog::new()
                            .set_file_name(&format!("{default_name}_edited.{ext}"))
                            .save_file()
                            .await;

                        let Some(file) = handle else {
                            return Err("Export cancelled".to_string());
                        };

                        let out_path = file.path().to_path_buf();
                        let start_t = Instant::now();

                        tokio::task::spawn_blocking(move || {
                            let mut headless = HeadlessGpu::new().map_err(|e| e.to_string())?;
                            for node in &effective_stack.nodes {
                                if let EditOp::DoubleExposure(ref p) = node.op {
                                    if let Some(ref path_str) = p.secondary_path {
                                        if let Ok(sec) = snap_io::load_image(std::path::Path::new(path_str)) {
                                            headless.upload_secondary_image(path_str, &sec);
                                        }
                                    }
                                }
                            }
                            let exported = headless
                                .export_full_res(&source, &effective_stack, &needed_luts)
                                .map_err(|e| e.to_string())?;

                            snap_io::export_linear_image(&exported, &out_path, &options)
                                .map_err(|e| e.to_string())?;

                            let dur = start_t.elapsed();
                            Ok((out_path, exported.width, exported.height, dur))
                        })
                        .await
                        .map_err(|e| e.to_string())?
                    },
                    Message::ExportFinished,
                );
                return task;
            }

            Message::ExportFinished(res) => {
                self.is_exporting = false;
                match res {
                    Ok((path, w, h, dur)) => {
                        self.set_status(format!(
                            "Exported {} ({}x{}) in {:.2}s",
                            path.display(),
                            w,
                            h,
                            dur.as_secs_f32()
                        ));
                    }
                    Err(e) => {
                        if e != "Export cancelled" {
                            self.set_status(format!("Export failed: {e}"));
                        }
                    }
                }
            }

            Message::ImportXmpDialog => {
                let task = Task::perform(
                    async {
                        let handle = rfd::AsyncFileDialog::new()
                            .add_filter("Adobe XMP Presets", &["xmp"])
                            .pick_file()
                            .await;
                        handle.map(|f| f.path().to_path_buf())
                    },
                    Message::XmpFileSelected,
                );
                return task;
            }

            Message::XmpFileSelected(Some(path)) => {
                let res = snap_io::install_xmp_preset(&path)
                    .or_else(|_| snap_io::import_xmp_preset(&path));
                match res {
                    Ok(preset) => {
                        let name = preset.name.clone();
                        if let Some(pos) = self.custom_looks.iter().position(|l| l.id == preset.id) {
                            self.custom_looks[pos] = preset.clone();
                        } else {
                            self.custom_looks.push(preset.clone());
                        }
                        self.preload_secondary_images_for_stack(&preset.stack);
                        self.document_history.commit(preset.stack);
                        self.hist_dirty = true;
                        self.set_status(format!("Imported and applied XMP preset: {name}"));
                    }
                    Err(e) => {
                        self.set_status(format!("Failed to import XMP preset: {e}"));
                    }
                }
            }

            Message::XmpFileSelected(None) => {}

            Message::DoubleExposureFileSelected(Some(path)) => {
                let path_str = path.to_string_lossy().to_string();
                if let Ok(img) = snap_io::load_image(&path) {
                    let arc_img = Arc::new(img);
                    self.secondary_images.insert(path_str.clone(), arc_img);
                    if let Some(ref mut session) = self.active_tool_session {
                        if let EditOp::DoubleExposure(ref mut p) = session.live_op {
                            p.secondary_path = Some(path_str.clone());
                        }
                        if let EditOp::DoubleExposure(p) = session.history.current_mut() {
                            p.secondary_path = Some(path_str);
                        }
                        self.hist_dirty = true;
                    }
                    self.set_status(format!("Loaded secondary photo: {}", path.display()));
                } else {
                    self.set_status(format!("Failed to load secondary photo: {}", path.display()));
                }
            }

            Message::DoubleExposureFileSelected(None) => {}

            Message::ToggleNodeMaskPanel(id) => {
                if self.mask_edit_node_id == Some(id) {
                    self.mask_edit_node_id = None;
                    self.mask_paint_mode = false;
                } else {
                    self.mask_edit_node_id = Some(id);
                }
            }

            Message::ToolEditorExt(event) => match event {
                ToolEditorEvent::PreviewOp(op) => {
                    if let Some(ref mut session) = self.active_tool_session {
                        session.live_op = op;
                        self.hist_dirty = true;
                    }
                }
                ToolEditorEvent::CommitOp(op) => {
                    if let Some(ref mut session) = self.active_tool_session {
                        session.live_op = op.clone();
                        session.history.commit(op);
                        self.hist_dirty = true;
                    }
                }
                ToolEditorEvent::ToggleSelectiveAddPin(v) => {
                    self.selective_add_pin_mode = v;
                }
                ToolEditorEvent::PickDoubleExposureImage => {
                    let task = Task::perform(
                        async {
                            let handle = rfd::AsyncFileDialog::new()
                                .add_filter(
                                    "Supported Images",
                                    &[
                                        "jpg", "jpeg", "png", "tiff", "tif", "webp", "bmp", "dng",
                                        "cr2", "cr3", "nef", "arw", "raf", "orf", "rw2",
                                    ],
                                )
                                .pick_file()
                                .await;
                            handle.map(|f| f.path().to_path_buf())
                        },
                        Message::DoubleExposureFileSelected,
                    );
                    return task;
                }
                ToolEditorEvent::MaskSetPaintMode { node_id, active } => {
                    self.mask_edit_node_id = if active { Some(node_id) } else { None };
                    self.mask_paint_mode = active;
                }
                ToolEditorEvent::MaskSetEraser(v) => {
                    self.mask_eraser = v;
                }
                ToolEditorEvent::MaskSetBrushRadius(v) => {
                    self.mask_brush_radius = v;
                }
                ToolEditorEvent::MaskSetBrushHardness(v) => {
                    self.mask_brush_hardness = v;
                }
                ToolEditorEvent::MaskSetTargetOpacity(v) => {
                    self.mask_target_opacity = v;
                }
                ToolEditorEvent::MaskToggleRedOverlay(v) => {
                    self.mask_red_overlay = v;
                }
                ToolEditorEvent::MaskInvert(node_id) => {
                    let mut stack = self.document_history.current().clone();
                    if let Some(node) = stack.node_by_id_mut(node_id) {
                        if let Some(ref mut mask) = node.mask {
                            mask.invert();
                        } else {
                            let mut mask = LayerMask::full();
                            mask.invert();
                            node.mask = Some(mask);
                        }
                        self.document_history.commit(stack);
                        self.hist_dirty = true;
                    }
                }
                ToolEditorEvent::MaskClear(node_id) => {
                    let mut stack = self.document_history.current().clone();
                    if let Some(node) = stack.node_by_id_mut(node_id) {
                        node.mask = Some(LayerMask::empty());
                        self.document_history.commit(stack);
                        self.hist_dirty = true;
                    }
                }
                ToolEditorEvent::MaskResetFull(node_id) => {
                    let mut stack = self.document_history.current().clone();
                    if let Some(node) = stack.node_by_id_mut(node_id) {
                        node.mask = Some(LayerMask::full());
                        self.document_history.commit(stack);
                        self.hist_dirty = true;
                    }
                }
                ToolEditorEvent::MaskApplySmart { node_id, kind } => {
                    if let Some(img) = self.source_image.as_deref() {
                        let mask = LayerMask::from_smart_mask(kind, img);
                        let mut stack = self.document_history.current().clone();
                        if stack.set_node_mask(node_id, Some(mask)) {
                            self.document_history.commit(stack);
                            self.hist_dirty = true;
                            self.set_status(format!("Applied {} smart mask", kind.display_name()));
                        }
                    }
                }
            },

            Message::CanvasOverlay(event) => match event {
                CanvasOverlayEvent::PreviewOp(op) => {
                    if let Some(ref mut session) = self.active_tool_session {
                        session.live_op = op;
                        self.hist_dirty = true;
                    }
                }
                CanvasOverlayEvent::CommitOp(op) => {
                    if let Some(ref mut session) = self.active_tool_session {
                        session.live_op = op.clone();
                        session.history.commit(op);
                        self.hist_dirty = true;
                    }
                }
                CanvasOverlayEvent::BrushStrokeSegment { from, to } => {
                    if let Some(ref mut session) = self.active_tool_session {
                        if let EditOp::Brush(ref mut p) = session.live_op {
                            p.map.paint_stroke(
                                from[0],
                                from[1],
                                to[0],
                                to[1],
                                p.radius,
                                p.feather,
                                p.step,
                                p.eraser,
                            );
                            self.hist_dirty = true;
                        }
                    }
                }
                CanvasOverlayEvent::BrushStrokeEnd => {
                    if let Some(ref mut session) = self.active_tool_session {
                        if let EditOp::Brush(_) = session.live_op {
                            session.history.commit(session.live_op.clone());
                            self.hist_dirty = true;
                        }
                    }
                }
                CanvasOverlayEvent::HealStrokeStart { uv } => {
                    if let Some(ref mut session) = self.active_tool_session {
                        if let EditOp::Healing(ref mut p) = session.live_op {
                            let stroke = HealStroke {
                                points: vec![uv],
                                radius: p.brush_radius,
                                source_offset: None,
                            };
                            p.strokes.push(stroke.clone());
                            self.active_heal_stroke = Some(stroke);
                            self.hist_dirty = true;
                        }
                    }
                }
                CanvasOverlayEvent::HealStrokeMove { uv } => {
                    if let Some(ref mut stroke) = self.active_heal_stroke {
                        stroke.points.push(uv);
                    }
                    if let Some(ref mut session) = self.active_tool_session {
                        if let EditOp::Healing(ref mut p) = session.live_op {
                            if let Some(last) = p.strokes.last_mut() {
                                last.points.push(uv);
                            }
                            self.hist_dirty = true;
                        }
                    }
                }
                CanvasOverlayEvent::HealStrokeEnd => {
                    self.active_heal_stroke = None;
                    if let Some(ref mut session) = self.active_tool_session {
                        if let EditOp::Healing(_) = session.live_op {
                            session.history.commit(session.live_op.clone());
                            self.hist_dirty = true;
                        }
                    }
                }
                CanvasOverlayEvent::MaskStrokeSegment { node_id, from, to } => {
                    if self.mask_stroke_backup.is_none() {
                        self.mask_stroke_backup = Some(self.document_history.current().clone());
                    }
                    let eff_opacity = if self.mask_eraser {
                        0.0
                    } else {
                        self.mask_target_opacity
                    };
                    let stack = self.document_history.current_mut();
                    if let Some(node) = stack.node_by_id_mut(node_id) {
                        if node.mask.is_none() {
                            let init_mask = if self.mask_eraser || self.mask_target_opacity < 0.5 {
                                LayerMask::full()
                            } else {
                                LayerMask::empty()
                            };
                            node.mask = Some(init_mask);
                        }
                        if let Some(ref mut mask) = node.mask {
                            mask.paint_stroke(
                                from[0],
                                from[1],
                                to[0],
                                to[1],
                                self.mask_brush_radius,
                                self.mask_brush_hardness,
                                eff_opacity,
                            );
                            self.hist_dirty = true;
                        }
                    }
                }
                CanvasOverlayEvent::MaskStrokeEnd { node_id: _ } => {
                    if let Some(backup) = self.mask_stroke_backup.take() {
                        let modified = self.document_history.current().clone();
                        *self.document_history.current_mut() = backup;
                        self.document_history.commit(modified);
                        self.hist_dirty = true;
                    }
                }
            },
        }

        Task::none()
    }
}
