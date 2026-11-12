use serde::{Deserialize, Serialize};

use crate::curves::CurvePreset;
use crate::ops::{
    BwColorFilter, BwParams, BwPreset, DetailsParams, EditOp, TuneParams, VignetteParams,
    WhiteBalanceParams,
};
use crate::stack::EditStack;

pub type LookId = String;

/// A named look preset consisting of a pre-configured non-destructive EditStack.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LookPreset {
    pub id: LookId,
    pub name: String,
    pub built_in: bool,
    pub stack: EditStack,
}

/// Retrieve all 11 built-in Looks.
pub fn built_in_looks() -> Vec<LookPreset> {
    vec![
        // 1. Portrait
        {
            let mut stack = EditStack::default();
            stack.push_op(EditOp::TuneImage(TuneParams {
                brightness: 5.0,
                contrast: 5.0,
                saturation: 0.0,
                ambiance: 15.0,
                highlights: -10.0,
                shadows: 5.0,
                warmth: 12.0,
            }));
            stack.push_op(EditOp::Details(DetailsParams {
                structure: -12.0,
                sharpening: 15.0,
            }));
            LookPreset {
                id: "portrait".to_string(),
                name: "Portrait".to_string(),
                built_in: true,
                stack,
            }
        },
        // 2. Smooth
        {
            let mut stack = EditStack::default();
            stack.push_op(EditOp::Details(DetailsParams {
                structure: -25.0,
                sharpening: 10.0,
            }));
            stack.push_op(EditOp::TuneImage(TuneParams {
                brightness: 8.0,
                contrast: -10.0,
                saturation: 5.0,
                ambiance: 20.0,
                highlights: -5.0,
                shadows: 15.0,
                warmth: 0.0,
            }));
            LookPreset {
                id: "smooth".to_string(),
                name: "Smooth".to_string(),
                built_in: true,
                stack,
            }
        },
        // 3. Pop
        {
            let mut stack = EditStack::default();
            stack.push_op(EditOp::TuneImage(TuneParams {
                brightness: 0.0,
                contrast: 25.0,
                saturation: 20.0,
                ambiance: 20.0,
                highlights: -10.0,
                shadows: 10.0,
                warmth: 0.0,
            }));
            stack.push_op(EditOp::Details(DetailsParams {
                structure: 20.0,
                sharpening: 15.0,
            }));
            LookPreset {
                id: "pop".to_string(),
                name: "Pop".to_string(),
                built_in: true,
                stack,
            }
        },
        // 4. Accentuate
        {
            let mut stack = EditStack::default();
            stack.push_op(EditOp::TuneImage(TuneParams {
                brightness: 0.0,
                contrast: 18.0,
                saturation: 15.0,
                ambiance: 10.0,
                highlights: 10.0,
                shadows: 15.0,
                warmth: 0.0,
            }));
            stack.push_op(EditOp::Details(DetailsParams {
                structure: 30.0,
                sharpening: 20.0,
            }));
            LookPreset {
                id: "accentuate".to_string(),
                name: "Accentuate".to_string(),
                built_in: true,
                stack,
            }
        },
        // 5. Faded Glow
        {
            let mut stack = EditStack::default();
            stack.push_op(EditOp::Curves(CurvePreset::Fade.to_curves_params()));
            stack.push_op(EditOp::TuneImage(TuneParams {
                brightness: 10.0,
                contrast: -5.0,
                saturation: -10.0,
                ambiance: 10.0,
                highlights: 5.0,
                shadows: 10.0,
                warmth: 15.0,
            }));
            stack.push_op(EditOp::Vignette(VignetteParams {
                center: [0.5, 0.5],
                inner_radius: 0.35,
                outer_radius: 0.90,
                outer_brightness: -30.0,
                inner_brightness: 5.0,
            }));
            LookPreset {
                id: "faded_glow".to_string(),
                name: "Faded Glow".to_string(),
                built_in: true,
                stack,
            }
        },
        // 6. Morning
        {
            let mut stack = EditStack::default();
            stack.push_op(EditOp::WhiteBalance(WhiteBalanceParams {
                temperature: 22.0,
                tint: -5.0,
            }));
            stack.push_op(EditOp::TuneImage(TuneParams {
                brightness: 15.0,
                contrast: 5.0,
                saturation: 8.0,
                ambiance: 20.0,
                highlights: 12.0,
                shadows: 10.0,
                warmth: 8.0,
            }));
            LookPreset {
                id: "morning".to_string(),
                name: "Morning".to_string(),
                built_in: true,
                stack,
            }
        },
        // 7. Bright
        {
            let mut stack = EditStack::default();
            stack.push_op(EditOp::TuneImage(TuneParams {
                brightness: 35.0,
                contrast: 5.0,
                saturation: 5.0,
                ambiance: 10.0,
                highlights: -15.0,
                shadows: 25.0,
                warmth: 0.0,
            }));
            LookPreset {
                id: "bright".to_string(),
                name: "Bright".to_string(),
                built_in: true,
                stack,
            }
        },
        // 8. Fine Art
        {
            let mut stack = EditStack::default();
            stack.push_op(EditOp::BlackAndWhite(BwParams {
                preset: BwPreset::Film,
                color_filter: BwColorFilter::Yellow,
                brightness: 5.0,
                contrast: 25.0,
                grain: 20.0,
            }));
            stack.push_op(EditOp::Vignette(VignetteParams {
                center: [0.5, 0.5],
                inner_radius: 0.30,
                outer_radius: 0.85,
                outer_brightness: -35.0,
                inner_brightness: 0.0,
            }));
            LookPreset {
                id: "fine_art".to_string(),
                name: "Fine Art".to_string(),
                built_in: true,
                stack,
            }
        },
        // 9. Push
        {
            let mut stack = EditStack::default();
            stack.push_op(EditOp::TuneImage(TuneParams {
                brightness: 0.0,
                contrast: 40.0,
                saturation: 30.0,
                ambiance: 15.0,
                highlights: -20.0,
                shadows: 20.0,
                warmth: 0.0,
            }));
            LookPreset {
                id: "push".to_string(),
                name: "Push".to_string(),
                built_in: true,
                stack,
            }
        },
        // 10. Structure
        {
            let mut stack = EditStack::default();
            stack.push_op(EditOp::Details(DetailsParams {
                structure: 55.0,
                sharpening: 25.0,
            }));
            stack.push_op(EditOp::TuneImage(TuneParams {
                brightness: 0.0,
                contrast: 15.0,
                saturation: 5.0,
                ambiance: 10.0,
                highlights: -10.0,
                shadows: 15.0,
                warmth: 0.0,
            }));
            LookPreset {
                id: "structure".to_string(),
                name: "Structure".to_string(),
                built_in: true,
                stack,
            }
        },
        // 11. Silhouette
        {
            let mut stack = EditStack::default();
            stack.push_op(EditOp::TuneImage(TuneParams {
                brightness: -30.0,
                contrast: 50.0,
                saturation: 20.0,
                ambiance: -10.0,
                highlights: 10.0,
                shadows: -80.0,
                warmth: 0.0,
            }));
            stack.push_op(EditOp::Curves(CurvePreset::HardContrast.to_curves_params()));
            LookPreset {
                id: "silhouette".to_string(),
                name: "Silhouette".to_string(),
                built_in: true,
                stack,
            }
        },
    ]
}
