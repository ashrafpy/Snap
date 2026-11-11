pub mod app;
pub mod canvas_overlays;
pub mod canvas_shader;
pub mod curves_widget;
pub mod histogram_widget;
pub mod icons;
pub mod sample_scene;
pub mod theme;
pub mod tool_editors_ext;
pub mod ui_preview;
pub mod views;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use iced::Point;
    use snap_core::ops::{EditOp, ToolId};
    use snap_core::stack::EditStack;
    use crate::app::{ActiveToolSession, Message, SnapApp};
    use crate::canvas_shader::CanvasShaderProgram;
    use snap_gpu::types::ViewportParams;

    #[test]
    fn test_sample_hdr_scene_generation() {
        let img = sample_scene::generate_sample_hdr_scene();
        assert_eq!(img.width, 1600);
        assert_eq!(img.height, 1066);
        assert_eq!(img.pixels.len(), (1600 * 1066) as usize);

        // Verify camera metadata is properly initialized
        assert_eq!(img.metadata.camera_make.as_deref(), Some("Snap Procedural HDR"));
        assert_eq!(img.metadata.iso, Some(100));
        assert_eq!(img.metadata.original_width, 1600);
        assert_eq!(img.metadata.original_height, 1066);

        // Verify Sun Core has HDR brightness (> 1.0)
        let mut has_hdr_highlight = false;
        for px in img.pixels.iter() {
            if px[0].to_f32() > 1.5 || px[1].to_f32() > 1.5 {
                has_hdr_highlight = true;
                break;
            }
        }
        assert!(has_hdr_highlight, "Sample scene must contain HDR specular highlights");
    }

    #[test]
    fn test_active_tool_session_preview_composition() {
        let (mut app, _) = SnapApp::boot(None);
        let img = sample_scene::generate_sample_hdr_scene();
        app.source_image = Some(Arc::new(img));

        // Initial stack is empty
        assert!(app.effective_stack().is_empty());

        // Open Tune Image tool
        let _ = app.update(Message::OpenTool(ToolId::TuneImage));
        assert!(app.active_tool_session.is_some());

        // Adjust parameter live (Brightness +30)
        let _ = app.update(Message::SetSliderParamLive(0, 30.0));

        // Effective stack should have 1 node reflecting brightness 30.0
        let effective = app.effective_stack();
        assert_eq!(effective.nodes.len(), 1);
        if let EditOp::TuneImage(tune) = &effective.nodes[0].op {
            assert_eq!(tune.brightness, 30.0);
        } else {
            panic!("Expected TuneImage op");
        }

        // Base document history is not committed yet!
        assert!(app.document_history.current().is_empty());

        // Apply tool
        let _ = app.update(Message::ApplyActiveTool);
        assert!(app.active_tool_session.is_none());
        assert_eq!(app.document_history.current().nodes.len(), 1);

        // Undo
        let _ = app.update(Message::Undo);
        assert!(app.document_history.current().is_empty());

        // Redo
        let _ = app.update(Message::Redo);
        assert_eq!(app.document_history.current().nodes.len(), 1);
    }

    #[test]
    fn test_screen_to_image_uv_mapping() {
        let program = CanvasShaderProgram {
            image_id: Some(1),
            proxy_image: Some(Arc::new(sample_scene::generate_sample_hdr_scene())),
            effective_stack: EditStack::default(),
            bypass_edits: false,
            viewport_params: ViewportParams::default(),
            needed_luts: Vec::new(),
            needed_secondary_images: Vec::new(),
            hist_slot: None,
            hist_dirty: false,
            active_tool: None,
            active_op: None,
            selected_param_idx: 0,
            eyedropper_mode: false,
            vignette_center_mode: false,
            _marker: std::marker::PhantomData,
        };

        let bounds = iced::Rectangle::new(Point::new(0.0, 0.0), iced::Size::new(1000.0, 1000.0));

        // Center of canvas should map to center of image (0.5, 0.5)
        let center_uv = program.screen_to_image_uv(Point::new(500.0, 500.0), bounds);
        assert!(center_uv.is_some());
        let [u, v] = center_uv.unwrap();
        assert!((u - 0.5).abs() < 1e-3, "u: {u}");
        assert!((v - 0.5).abs() < 1e-3, "v: {v}");
    }

    #[test]
    fn test_in_tool_undo_redo_history() {
        let mut session = ActiveToolSession::new_tool(ToolId::TuneImage);
        assert!(!session.history.can_undo());

        // Commit change 1: Brightness = 10
        let mut op1 = session.history.current().clone();
        op1.set_slider_param(0, 10.0);
        session.history.commit(op1);
        assert!(session.history.can_undo());

        // Commit change 2: Brightness = 25
        let mut op2 = session.history.current().clone();
        op2.set_slider_param(0, 25.0);
        session.history.commit(op2);

        if let EditOp::TuneImage(tune) = session.history.current() {
            assert_eq!(tune.brightness, 25.0);
        }

        // In-tool undo -> Brightness = 10
        session.history.undo();
        if let EditOp::TuneImage(tune) = session.history.current() {
            assert_eq!(tune.brightness, 10.0);
        }

        // In-tool undo -> Brightness = 0 (default)
        session.history.undo();
        if let EditOp::TuneImage(tune) = session.history.current() {
            assert_eq!(tune.brightness, 0.0);
        }

        // In-tool redo -> Brightness = 10
        session.history.redo();
        if let EditOp::TuneImage(tune) = session.history.current() {
            assert_eq!(tune.brightness, 10.0);
        }
    }

    #[test]
    fn test_in_tool_live_slider_drag_undo() {
        let (mut app, _) = SnapApp::boot(None);
        let img = sample_scene::generate_sample_hdr_scene();
        app.source_image = Some(Arc::new(img));

        // 1. Open TuneImage tool
        let _ = app.update(Message::OpenTool(ToolId::TuneImage));
        assert!(app.active_tool_session.is_some());

        // 2. SetSliderParamLive(0, 42.0) (e.g. dragging brightness slider)
        let _ = app.update(Message::SetSliderParamLive(0, 42.0));

        // Effective stack and live_op reflect 42.0
        let effective = app.effective_stack();
        assert_eq!(effective.nodes.len(), 1);
        if let EditOp::TuneImage(ref tune) = effective.nodes[0].op {
            assert_eq!(tune.brightness, 42.0);
        } else {
            panic!("Expected TuneImage op");
        }

        let session = app.active_tool_session.as_ref().unwrap();
        if let EditOp::TuneImage(ref tune) = session.live_op {
            assert_eq!(tune.brightness, 42.0);
        }
        // Pre-drag value in history is still 0.0 before gesture ends
        if let EditOp::TuneImage(tune) = session.history.current() {
            assert_eq!(tune.brightness, 0.0);
        }

        // 3. CanvasGestureEnd commits the live op
        let _ = app.update(Message::CanvasGestureEnd);
        let session = app.active_tool_session.as_ref().unwrap();
        if let EditOp::TuneImage(tune) = session.history.current() {
            assert_eq!(tune.brightness, 42.0);
        }

        // 4. Undo reverts brightness back to 0.0
        let _ = app.update(Message::Undo);
        let session = app.active_tool_session.as_ref().unwrap();
        if let EditOp::TuneImage(ref tune) = session.live_op {
            assert_eq!(tune.brightness, 0.0);
        } else {
            panic!("Expected TuneImage op");
        }
        if let EditOp::TuneImage(tune) = session.history.current() {
            assert_eq!(tune.brightness, 0.0);
        }

        let effective_after_undo = app.effective_stack();
        if let EditOp::TuneImage(ref tune) = effective_after_undo.nodes[0].op {
            assert_eq!(tune.brightness, 0.0);
        }
    }

    #[test]
    fn test_color_grade_tool_state_transitions() {
        let (mut app, _) = SnapApp::boot(None);
        let img = sample_scene::generate_sample_hdr_scene();
        app.source_image = Some(Arc::new(img));

        // Open ColorGrade tool
        let _ = app.update(Message::OpenTool(ToolId::ColorGrade));
        assert!(app.active_tool_session.is_some());
        assert_eq!(app.active_tool_session.as_ref().unwrap().tool, ToolId::ColorGrade);

        // Preview live change via ToolEditorExt
        let mut grade = snap_core::ops_ext::ColorGradeParams::default();
        grade.shadows.saturation = 45.0;
        grade.shadows.hue_deg = 190.0;
        let _ = app.update(Message::ToolEditorExt(
            crate::tool_editors_ext::ToolEditorEvent::PreviewOp(EditOp::ColorGrade(grade.clone())),
        ));

        let effective = app.effective_stack();
        assert_eq!(effective.nodes.len(), 1);
        if let EditOp::ColorGrade(ref g) = effective.nodes[0].op {
            assert_eq!(g.shadows.saturation, 45.0);
            assert_eq!(g.shadows.hue_deg, 190.0);
        } else {
            panic!("Expected ColorGrade op");
        }

        // Commit live change
        let _ = app.update(Message::ToolEditorExt(
            crate::tool_editors_ext::ToolEditorEvent::CommitOp(EditOp::ColorGrade(grade)),
        ));

        // Apply active tool
        let _ = app.update(Message::ApplyActiveTool);
        assert!(app.active_tool_session.is_none());
        assert_eq!(app.document_history.current().nodes.len(), 1);

        // Undo removes node
        let _ = app.update(Message::Undo);
        assert!(app.document_history.current().is_empty());

        // Redo restores node
        let _ = app.update(Message::Redo);
        assert_eq!(app.document_history.current().nodes.len(), 1);
    }

    #[test]
    fn test_selective_pin_add_mode_transitions() {
        let (mut app, _) = SnapApp::boot(None);
        let img = sample_scene::generate_sample_hdr_scene();
        app.source_image = Some(Arc::new(img));

        let _ = app.update(Message::OpenTool(ToolId::Selective));
        assert!(!app.selective_add_pin_mode);

        // Toggle add pin mode on
        let _ = app.update(Message::ToolEditorExt(
            crate::tool_editors_ext::ToolEditorEvent::ToggleSelectiveAddPin(true),
        ));
        assert!(app.selective_add_pin_mode);

        // Commit a pin
        let mut sel = snap_core::ops_ext::SelectiveParams::default();
        sel.pins.push(snap_core::ops_ext::SelectivePin {
            x: 0.4,
            y: 0.6,
            radius: 0.2,
            brightness: 25.0,
            contrast: 0.0,
            saturation: 0.0,
            structure: 0.0,
        });
        let _ = app.update(Message::ToolEditorExt(
            crate::tool_editors_ext::ToolEditorEvent::CommitOp(EditOp::Selective(sel)),
        ));

        // Switching / closing tool resets selective_add_pin_mode
        let _ = app.update(Message::CancelActiveTool);
        assert!(!app.selective_add_pin_mode);
        assert!(app.active_tool_session.is_none());
    }

    #[test]
    fn test_layer_mask_smart_mask_transitions() {
        let (mut app, _) = SnapApp::boot(None);
        let img = sample_scene::generate_sample_hdr_scene();
        app.source_image = Some(Arc::new(img));

        // Apply Details tool
        let _ = app.update(Message::OpenTool(ToolId::Details));
        let _ = app.update(Message::SetSliderParamLive(0, 30.0));
        let _ = app.update(Message::ApplyActiveTool);
        assert_eq!(app.document_history.current().nodes.len(), 1);

        let node_id = app.document_history.current().nodes[0].id;
        assert!(app.document_history.current().nodes[0].mask.is_none());

        // Toggle mask panel
        let _ = app.update(Message::ToggleNodeMaskPanel(node_id));
        assert_eq!(app.mask_edit_node_id, Some(node_id));

        // Apply Smart Mask: Sky
        let _ = app.update(Message::ToolEditorExt(
            crate::tool_editors_ext::ToolEditorEvent::MaskApplySmart {
                node_id,
                kind: snap_core::mask::SmartMaskKind::Sky,
            },
        ));

        let current_node = &app.document_history.current().nodes[0];
        assert!(current_node.mask.is_some());
        let mask = current_node.mask.as_ref().unwrap();
        assert!(!mask.is_uniform_full());
        assert!(!mask.is_uniform_zero());

        // Invert mask
        let _ = app.update(Message::ToolEditorExt(
            crate::tool_editors_ext::ToolEditorEvent::MaskInvert(node_id),
        ));
        assert!(app.document_history.current().nodes[0].mask.is_some());

        // Clear mask
        let _ = app.update(Message::ToolEditorExt(
            crate::tool_editors_ext::ToolEditorEvent::MaskClear(node_id),
        ));
        assert!(app.document_history.current().nodes[0].mask.as_ref().unwrap().is_uniform_zero());

        // Reset full mask
        let _ = app.update(Message::ToolEditorExt(
            crate::tool_editors_ext::ToolEditorEvent::MaskResetFull(node_id),
        ));
        assert!(app.document_history.current().nodes[0].mask.as_ref().unwrap().is_uniform_full());
    }

    #[test]
    fn test_xmp_preset_import_and_apply() {
        let (mut app, _) = SnapApp::boot(None);
        let img = sample_scene::generate_sample_hdr_scene();
        app.source_image = Some(Arc::new(img));

        let xmp_xml = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about=""
    xmlns:crs="http://ns.adobe.com/camera-raw-settings/1.0/"
    crs:PresetType="Normal"
    crs:Cluster=""
    crs:UUID="TEST-XMP-UUID"
    crs:SupportsAmount="False"
    crs:ProcessVersion="11.0"
    crs:Exposure2012="+0.50"
    crs:Contrast2012="+25"
    crs:Highlights2012="-30"
    crs:Shadows2012="+20"
    crs:Temperature="6500"
    crs:Tint="+10">
   <crs:Name>
    <rdf:Alt>
     <rdf:li xml:lang="x-default">Warm Test Look</rdf:li>
    </rdf:Alt>
   </crs:Name>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>"#;

        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join(format!("snap_test_xmp_{}.xmp", uuid::Uuid::new_v4()));
        std::fs::write(&temp_path, xmp_xml).expect("Failed to write test xmp file");

        let _ = app.update(Message::XmpFileSelected(Some(temp_path.clone())));
        let _ = std::fs::remove_file(temp_path);

        // Check custom_looks contains the look
        assert!(app.custom_looks.iter().any(|l| l.name == "Warm Test Look"));

        // Check current stack reflects the preset
        let current_stack = app.document_history.current();
        assert!(!current_stack.is_empty());
    }
}
