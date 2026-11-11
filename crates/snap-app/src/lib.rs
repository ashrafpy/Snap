pub mod app;
pub mod canvas_shader;
pub mod curves_widget;
pub mod histogram_widget;
pub mod sample_scene;
pub mod theme;
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
}
