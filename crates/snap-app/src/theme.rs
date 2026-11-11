use iced::border;
use iced::theme::Palette;
use iced::widget::{button, checkbox, container, rule, scrollable, slider, text_input};
use iced::{Background, Border, Color, Shadow, Theme, Vector};

/// Exact Snap Studio palette tokens.
pub mod palette {
    use iced::Color;

    /// Compile-time helper for constructing `Color` from 8-bit RGB components.
    pub const fn rgb8(r: u8, g: u8, b: u8) -> Color {
        Color {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: 1.0,
        }
    }

    /// Compile-time helper for constructing `Color` from RGBA floats.
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Color {
        Color { r, g, b, a }
    }

    pub const BG_CANVAS: Color = rgb8(11, 12, 15);
    pub const BG_CHROME: Color = rgb8(19, 21, 26);
    pub const BG_SUNKEN: Color = rgb8(13, 15, 19);
    pub const BG_SURFACE: Color = rgb8(27, 30, 38);
    pub const BG_SURFACE_HOVER: Color = rgb8(36, 40, 51);
    pub const BG_SURFACE_ACTIVE: Color = rgb8(26, 38, 59);

    pub const BORDER_SUBTLE: Color = rgba(1.0, 1.0, 1.0, 0.07);
    pub const BORDER_STRONG: Color = rgba(1.0, 1.0, 1.0, 0.14);

    pub const ACCENT: Color = rgb8(79, 142, 247);
    pub const ACCENT_HOVER: Color = rgb8(104, 160, 250);
    pub const ACCENT_MUTED: Color = rgba(0.310, 0.557, 0.969, 0.16);

    pub const AMBER: Color = rgb8(245, 158, 11);
    pub const DANGER: Color = rgb8(248, 113, 113);
    pub const SUCCESS: Color = rgb8(52, 211, 153);

    pub const TEXT_PRIMARY: Color = rgb8(243, 244, 246);
    pub const TEXT_SECONDARY: Color = rgb8(156, 163, 175);
    pub const TEXT_MUTED: Color = rgb8(107, 114, 128);
}

// Re-export palette tokens at module level
pub use palette::*;

/// Creates the custom iced `Theme` for Snap Studio Dark.
pub fn snap_theme() -> Theme {
    Theme::custom(
        "Snap Studio Dark",
        Palette {
            background: palette::BG_CANVAS,
            text: palette::TEXT_PRIMARY,
            primary: palette::ACCENT,
            success: palette::SUCCESS,
            warning: palette::AMBER,
            danger: palette::DANGER,
        },
    )
}

// -----------------------------------------------------------------------------
// Container Styles
// -----------------------------------------------------------------------------

pub fn app_backdrop(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(palette::BG_CANVAS)),
        text_color: Some(palette::TEXT_PRIMARY),
        border: Border::default(),
        shadow: Shadow::default(),
        snap: false,
    }
}

pub fn canvas_backdrop(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(palette::BG_CANVAS)),
        text_color: Some(palette::TEXT_PRIMARY),
        border: Border::default(),
        shadow: Shadow::default(),
        snap: false,
    }
}

pub fn top_bar_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(palette::BG_CHROME)),
        border: border::rounded(0.0).color(palette::BORDER_SUBTLE).width(1.0),
        text_color: Some(palette::TEXT_PRIMARY),
        shadow: Shadow::default(),
        snap: false,
    }
}

pub fn inspector_dock_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(palette::BG_CHROME)),
        border: border::rounded(0.0).color(palette::BORDER_SUBTLE).width(1.0),
        text_color: Some(palette::TEXT_PRIMARY),
        shadow: Shadow::default(),
        snap: false,
    }
}

pub fn sunken_track_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(palette::BG_SUNKEN)),
        border: border::rounded(8.0).color(palette::BORDER_SUBTLE).width(1.0),
        text_color: Some(palette::TEXT_SECONDARY),
        shadow: Shadow::default(),
        snap: false,
    }
}

pub fn surface_card_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(palette::BG_SURFACE)),
        border: border::rounded(8.0).color(palette::BORDER_SUBTLE).width(1.0),
        text_color: Some(palette::TEXT_PRIMARY),
        shadow: Shadow::default(),
        snap: false,
    }
}

pub fn active_surface_card_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(palette::BG_SURFACE_ACTIVE)),
        border: border::rounded(8.0).color(palette::ACCENT).width(1.5),
        text_color: Some(palette::TEXT_PRIMARY),
        shadow: Shadow::default(),
        snap: false,
    }
}

pub fn floating_glass_pill(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color {
            r: 19.0 / 255.0,
            g: 21.0 / 255.0,
            b: 26.0 / 255.0,
            a: 0.88,
        })),
        border: border::rounded(20.0).color(palette::BORDER_STRONG).width(1.0),
        text_color: Some(palette::TEXT_PRIMARY),
        shadow: Shadow {
            color: Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.40,
            },
            offset: Vector::new(0.0, 4.0),
            blur_radius: 12.0,
        },
        snap: false,
    }
}

pub fn floating_hud_panel(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color {
            r: 13.0 / 255.0,
            g: 15.0 / 255.0,
            b: 19.0 / 255.0,
            a: 0.92,
        })),
        border: border::rounded(10.0).color(palette::BORDER_STRONG).width(1.0),
        text_color: Some(palette::TEXT_PRIMARY),
        shadow: Shadow {
            color: Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.50,
            },
            offset: Vector::new(0.0, 6.0),
            blur_radius: 16.0,
        },
        snap: false,
    }
}

pub fn welcome_hero_card(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(palette::BG_SURFACE)),
        border: border::rounded(12.0).color(palette::BORDER_STRONG).width(1.0),
        text_color: Some(palette::TEXT_PRIMARY),
        shadow: Shadow {
            color: Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.35,
            },
            offset: Vector::new(0.0, 8.0),
            blur_radius: 20.0,
        },
        snap: false,
    }
}

pub fn dropzone_card_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(palette::BG_SUNKEN)),
        border: border::rounded(12.0).color(palette::ACCENT_MUTED).width(2.0),
        text_color: Some(palette::TEXT_SECONDARY),
        shadow: Shadow::default(),
        snap: false,
    }
}

pub fn badge_pill_style(bg: Color, border_col: Color) -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: Some(Background::Color(bg)),
        border: border::rounded(12.0).color(border_col).width(1.0),
        text_color: Some(palette::TEXT_PRIMARY),
        shadow: Shadow::default(),
        snap: false,
    }
}

// -----------------------------------------------------------------------------
// Button Styles
// -----------------------------------------------------------------------------

pub fn accent_button(_theme: &Theme, status: button::Status) -> button::Style {
    let (bg, text) = match status {
        button::Status::Hovered => (palette::ACCENT_HOVER, Color::WHITE),
        button::Status::Pressed => (palette::ACCENT, Color::WHITE),
        button::Status::Disabled => (
            Color {
                r: 79.0 / 255.0,
                g: 142.0 / 255.0,
                b: 247.0 / 255.0,
                a: 0.30,
            },
            palette::TEXT_MUTED,
        ),
        button::Status::Active => (palette::ACCENT, Color::WHITE),
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: text,
        border: border::rounded(6.0),
        shadow: Shadow::default(),
        snap: false,
    }
}

pub fn secondary_button(_theme: &Theme, status: button::Status) -> button::Style {
    let (bg, text) = match status {
        button::Status::Hovered => (palette::BG_SURFACE_HOVER, palette::TEXT_PRIMARY),
        button::Status::Pressed => (palette::BG_SURFACE_ACTIVE, palette::TEXT_PRIMARY),
        button::Status::Disabled => (
            Color {
                r: 27.0 / 255.0,
                g: 30.0 / 255.0,
                b: 38.0 / 255.0,
                a: 0.40,
            },
            palette::TEXT_MUTED,
        ),
        button::Status::Active => (palette::BG_SURFACE, palette::TEXT_PRIMARY),
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: text,
        border: border::rounded(6.0).color(palette::BORDER_SUBTLE).width(1.0),
        shadow: Shadow::default(),
        snap: false,
    }
}

pub fn subtle_danger_button(_theme: &Theme, status: button::Status) -> button::Style {
    let (bg, text) = match status {
        button::Status::Hovered => (
            Color {
                r: 248.0 / 255.0,
                g: 113.0 / 255.0,
                b: 113.0 / 255.0,
                a: 0.22,
            },
            palette::DANGER,
        ),
        button::Status::Pressed => (
            Color {
                r: 248.0 / 255.0,
                g: 113.0 / 255.0,
                b: 113.0 / 255.0,
                a: 0.35,
            },
            palette::DANGER,
        ),
        button::Status::Disabled => (Color::TRANSPARENT, palette::TEXT_MUTED),
        button::Status::Active => (
            Color {
                r: 248.0 / 255.0,
                g: 113.0 / 255.0,
                b: 113.0 / 255.0,
                a: 0.10,
            },
            palette::DANGER,
        ),
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: text,
        border: border::rounded(6.0)
            .color(Color {
                r: 248.0 / 255.0,
                g: 113.0 / 255.0,
                b: 113.0 / 255.0,
                a: 0.30,
            })
            .width(1.0),
        shadow: Shadow::default(),
        snap: false,
    }
}

pub fn ghost_icon_button(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let (bg, text, border_col) = if active {
            (
                palette::BG_SURFACE_ACTIVE,
                palette::ACCENT,
                palette::ACCENT,
            )
        } else {
            match status {
                button::Status::Hovered => (
                    palette::BG_SURFACE_HOVER,
                    palette::TEXT_PRIMARY,
                    palette::BORDER_SUBTLE,
                ),
                button::Status::Pressed => (
                    palette::BG_SURFACE,
                    palette::TEXT_PRIMARY,
                    palette::BORDER_SUBTLE,
                ),
                button::Status::Disabled => (
                    Color::TRANSPARENT,
                    palette::TEXT_MUTED,
                    Color::TRANSPARENT,
                ),
                button::Status::Active => (
                    Color::TRANSPARENT,
                    palette::TEXT_SECONDARY,
                    Color::TRANSPARENT,
                ),
            }
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: text,
            border: border::rounded(6.0).color(border_col).width(if active {
                1.0
            } else if status == button::Status::Hovered {
                1.0
            } else {
                0.0
            }),
            shadow: Shadow::default(),
            snap: false,
        }
    }
}

pub fn segmented_tab_button(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let (bg, text, border_col) = if active {
            (
                Color::from_rgb8(38, 43, 56), // #262B38 sleek elevated solid charcoal pill
                palette::TEXT_PRIMARY,
                palette::BORDER_STRONG,       // subtle 1.0px border (no harsh blue box)
            )
        } else {
            match status {
                button::Status::Hovered => (
                    palette::BG_SURFACE,
                    palette::TEXT_PRIMARY,
                    palette::BORDER_SUBTLE,
                ),
                button::Status::Pressed => (
                    palette::BG_SURFACE_HOVER,
                    palette::TEXT_PRIMARY,
                    palette::BORDER_SUBTLE,
                ),
                button::Status::Disabled => (
                    Color::TRANSPARENT,
                    palette::TEXT_MUTED,
                    Color::TRANSPARENT,
                ),
                button::Status::Active => (
                    Color::TRANSPARENT,
                    palette::TEXT_SECONDARY,
                    Color::TRANSPARENT,
                ),
            }
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: text,
            border: border::rounded(6.0)
                .color(border_col)
                .width(if active { 1.0 } else { 0.0 }),
            shadow: Shadow::default(),
            snap: false,
        }
    }
}

pub fn favorite_tab_button(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let (bg, text, border_col) = if active {
            (
                Color::from_rgb8(30, 43, 69), // #1E2B45 tint
                palette::TEXT_PRIMARY,
                palette::BORDER_STRONG,
            )
        } else {
            match status {
                button::Status::Hovered => (
                    palette::BG_SURFACE,
                    palette::TEXT_PRIMARY,
                    palette::BORDER_SUBTLE,
                ),
                button::Status::Pressed => (
                    palette::BG_SURFACE_HOVER,
                    palette::TEXT_PRIMARY,
                    palette::BORDER_SUBTLE,
                ),
                button::Status::Disabled => (
                    Color::TRANSPARENT,
                    palette::TEXT_MUTED,
                    Color::TRANSPARENT,
                ),
                button::Status::Active => (
                    Color::TRANSPARENT,
                    palette::TEXT_SECONDARY,
                    Color::TRANSPARENT,
                ),
            }
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: text,
            border: border::rounded(6.0)
                .color(border_col)
                .width(if active { 1.0 } else { 0.0 }),
            shadow: Shadow::default(),
            snap: false,
        }
    }
}

pub fn tool_tile_button(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let (bg, text, border_col, border_w) = if active {
            (
                palette::BG_SURFACE_ACTIVE,
                palette::TEXT_PRIMARY,
                palette::ACCENT,
                1.5,
            )
        } else {
            match status {
                button::Status::Hovered => (
                    palette::BG_SURFACE_HOVER,
                    palette::TEXT_PRIMARY,
                    palette::BORDER_STRONG,
                    1.0,
                ),
                button::Status::Pressed => (
                    palette::BG_SURFACE_ACTIVE,
                    palette::TEXT_PRIMARY,
                    palette::ACCENT,
                    1.0,
                ),
                button::Status::Disabled => (
                    palette::BG_SUNKEN,
                    palette::TEXT_MUTED,
                    palette::BORDER_SUBTLE,
                    1.0,
                ),
                button::Status::Active => (
                    palette::BG_SURFACE,
                    palette::TEXT_SECONDARY,
                    palette::BORDER_SUBTLE,
                    1.0,
                ),
            }
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: text,
            border: border::rounded(8.0).color(border_col).width(border_w),
            shadow: Shadow::default(),
            snap: false,
        }
    }
}

pub fn look_card_button(
    accent: Color,
    active: bool,
) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let (bg, text, border_col, border_w) = if active {
            (
                palette::BG_SURFACE_ACTIVE,
                palette::TEXT_PRIMARY,
                accent,
                2.0,
            )
        } else {
            match status {
                button::Status::Hovered => (
                    palette::BG_SURFACE_HOVER,
                    palette::TEXT_PRIMARY,
                    palette::BORDER_STRONG,
                    1.0,
                ),
                button::Status::Pressed => (
                    palette::BG_SURFACE_ACTIVE,
                    palette::TEXT_PRIMARY,
                    accent,
                    1.0,
                ),
                button::Status::Disabled => (
                    palette::BG_SUNKEN,
                    palette::TEXT_MUTED,
                    palette::BORDER_SUBTLE,
                    1.0,
                ),
                button::Status::Active => (
                    palette::BG_SURFACE,
                    palette::TEXT_SECONDARY,
                    palette::BORDER_SUBTLE,
                    1.0,
                ),
            }
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: text,
            border: border::rounded(8.0).color(border_col).width(border_w),
            shadow: Shadow::default(),
            snap: false,
        }
    }
}

pub fn pill_chip_button(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let (bg, text, border_col) = if active {
            (palette::ACCENT, Color::WHITE, palette::ACCENT)
        } else {
            match status {
                button::Status::Hovered => (
                    palette::BG_SURFACE_HOVER,
                    palette::TEXT_PRIMARY,
                    palette::BORDER_STRONG,
                ),
                button::Status::Pressed => (
                    palette::BG_SURFACE_ACTIVE,
                    palette::TEXT_PRIMARY,
                    palette::BORDER_STRONG,
                ),
                button::Status::Disabled => (
                    palette::BG_SUNKEN,
                    palette::TEXT_MUTED,
                    palette::BORDER_SUBTLE,
                ),
                button::Status::Active => (
                    palette::BG_SURFACE,
                    palette::TEXT_SECONDARY,
                    palette::BORDER_SUBTLE,
                ),
            }
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: text,
            border: border::rounded(14.0).color(border_col).width(1.0),
            shadow: Shadow::default(),
            snap: false,
        }
    }
}

pub fn color_dot_button(
    dot_color: Color,
    active: bool,
) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let (border_col, border_w) = if active {
            (Color::WHITE, 2.0)
        } else {
            match status {
                button::Status::Hovered => (palette::BORDER_STRONG, 1.5),
                _ => (palette::BORDER_SUBTLE, 1.0),
            }
        };
        button::Style {
            background: Some(Background::Color(dot_color)),
            text_color: Color::TRANSPARENT,
            border: border::rounded(12.0).color(border_col).width(border_w),
            shadow: Shadow::default(),
            snap: false,
        }
    }
}

// -----------------------------------------------------------------------------
// Widget Styles
// -----------------------------------------------------------------------------

pub fn studio_slider(accent: Color) -> impl Fn(&Theme, slider::Status) -> slider::Style {
    move |_theme, status| {
        let (radius, border_width) = match status {
            slider::Status::Dragged => (7.5, 2.0),
            slider::Status::Hovered => (7.0, 1.5),
            slider::Status::Active => (6.0, 1.0),
        };
        slider::Style {
            rail: slider::Rail {
                backgrounds: (
                    Background::Color(accent),
                    Background::Color(palette::BG_SUNKEN),
                ),
                width: 4.0,
                border: border::rounded(2.0),
            },
            handle: slider::Handle {
                shape: slider::HandleShape::Circle { radius },
                background: Background::Color(Color::WHITE),
                border_width,
                border_color: accent,
            },
        }
    }
}

pub fn thin_scrollable(_theme: &Theme, status: scrollable::Status) -> scrollable::Style {
    let scroller_color = match status {
        scrollable::Status::Dragged { .. } => Color {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 0.45,
        },
        scrollable::Status::Hovered { .. } => Color {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 0.28,
        },
        scrollable::Status::Active { .. } => Color {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 0.16,
        },
    };
    let rail = scrollable::Rail {
        background: None,
        border: Border::default(),
        scroller: scrollable::Scroller {
            background: Background::Color(scroller_color),
            border: border::rounded(3.0),
        },
    };
    scrollable::Style {
        container: container::Style::default(),
        vertical_rail: rail,
        horizontal_rail: rail,
        gap: None,
        auto_scroll: scrollable::AutoScroll {
            background: Background::Color(Color::TRANSPARENT),
            border: Border::default(),
            shadow: Shadow::default(),
            icon: Color::TRANSPARENT,
        },
    }
}

pub fn studio_text_input(_theme: &Theme, status: text_input::Status) -> text_input::Style {
    let (bg, border_col, border_w) = match status {
        text_input::Status::Focused { .. } => (palette::BG_SUNKEN, palette::ACCENT, 1.5),
        text_input::Status::Hovered => (palette::BG_SUNKEN, palette::BORDER_STRONG, 1.0),
        text_input::Status::Disabled => (palette::BG_CHROME, palette::BORDER_SUBTLE, 1.0),
        text_input::Status::Active => (palette::BG_SUNKEN, palette::BORDER_SUBTLE, 1.0),
    };
    text_input::Style {
        background: Background::Color(bg),
        border: border::rounded(6.0).color(border_col).width(border_w),
        icon: palette::TEXT_MUTED,
        placeholder: palette::TEXT_MUTED,
        value: palette::TEXT_PRIMARY,
        selection: palette::ACCENT_MUTED,
    }
}

pub fn studio_checkbox(_theme: &Theme, status: checkbox::Status) -> checkbox::Style {
    let (is_checked, is_hovered, is_disabled) = match status {
        checkbox::Status::Active { is_checked } => (is_checked, false, false),
        checkbox::Status::Hovered { is_checked } => (is_checked, true, false),
        checkbox::Status::Disabled { is_checked } => (is_checked, false, true),
    };

    let (bg, border_col, icon_col) = if is_disabled {
        (
            palette::BG_SUNKEN,
            palette::BORDER_SUBTLE,
            if is_checked {
                palette::TEXT_MUTED
            } else {
                Color::TRANSPARENT
            },
        )
    } else if is_checked {
        (
            if is_hovered {
                palette::ACCENT_HOVER
            } else {
                palette::ACCENT
            },
            if is_hovered {
                palette::ACCENT_HOVER
            } else {
                palette::ACCENT
            },
            Color::WHITE,
        )
    } else {
        (
            if is_hovered {
                palette::BG_SURFACE_HOVER
            } else {
                palette::BG_SURFACE
            },
            if is_hovered {
                palette::BORDER_STRONG
            } else {
                palette::BORDER_SUBTLE
            },
            Color::TRANSPARENT,
        )
    };

    checkbox::Style {
        background: Background::Color(bg),
        icon_color: icon_col,
        border: border::rounded(4.0).color(border_col).width(1.0),
        text_color: Some(if is_disabled {
            palette::TEXT_MUTED
        } else {
            palette::TEXT_PRIMARY
        }),
    }
}

pub fn subtle_rule(_theme: &Theme) -> rule::Style {
    rule::Style {
        color: palette::BORDER_SUBTLE,
        radius: border::Radius::from(0.0),
        fill_mode: rule::FillMode::Full,
        snap: true,
    }
}

// -----------------------------------------------------------------------------
// Legacy Aliases for Seamless Backward Compatibility
// -----------------------------------------------------------------------------

pub const BG_APP: Color = palette::BG_CANVAS;
pub const BG_HEADER: Color = palette::BG_CHROME;
pub const BG_CARD: Color = palette::BG_SURFACE;
pub const BG_CARD_HOVER: Color = palette::BG_SURFACE_HOVER;
pub const BG_PANEL: Color = palette::BG_CHROME;
pub const BORDER_ACTIVE: Color = palette::ACCENT;
pub const ACCENT_BLUE: Color = palette::ACCENT;
pub const ACCENT_BLUE_HOVER: Color = palette::ACCENT_HOVER;
pub const ACCENT_GREEN: Color = palette::SUCCESS;
pub const ACCENT_GREEN_HOVER: Color = palette::SUCCESS;
pub const ACCENT_AMBER: Color = palette::AMBER;
pub const ACCENT_RED: Color = palette::DANGER;
pub const ACCENT_RED_HOVER: Color = palette::DANGER;

pub fn app_container(theme: &Theme) -> container::Style {
    app_backdrop(theme)
}

pub fn header_bar(theme: &Theme) -> container::Style {
    top_bar_style(theme)
}

pub fn inspector_panel(theme: &Theme) -> container::Style {
    inspector_dock_style(theme)
}

pub fn card_container(theme: &Theme) -> container::Style {
    surface_card_style(theme)
}

pub fn active_card_container(theme: &Theme) -> container::Style {
    active_surface_card_style(theme)
}

pub fn hud_overlay_container(theme: &Theme) -> container::Style {
    floating_hud_panel(theme)
}

pub fn pill_container(theme: &Theme) -> container::Style {
    floating_glass_pill(theme)
}

pub fn primary_button(theme: &Theme, status: button::Status) -> button::Style {
    accent_button(theme, status)
}

pub fn apply_button(theme: &Theme, status: button::Status) -> button::Style {
    accent_button(theme, status)
}

pub fn cancel_button(theme: &Theme, status: button::Status) -> button::Style {
    secondary_button(theme, status)
}

pub fn ghost_button(theme: &Theme, status: button::Status) -> button::Style {
    ghost_icon_button(false)(theme, status)
}

pub fn tab_button(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    segmented_tab_button(active)
}

pub fn pill_button(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    pill_chip_button(active)
}
