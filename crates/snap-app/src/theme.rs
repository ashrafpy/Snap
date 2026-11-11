use iced::border;
use iced::widget::button;
use iced::widget::container;
use iced::{Background, Color, Theme};

// Palette constants
pub const BG_APP: Color = Color::from_rgb(0.07, 0.075, 0.082); // #121315
pub const BG_HEADER: Color = Color::from_rgb(0.09, 0.095, 0.105); // #17181b
pub const BG_CARD: Color = Color::from_rgb(0.12, 0.125, 0.14); // #1f2024
pub const BG_CARD_HOVER: Color = Color::from_rgb(0.16, 0.17, 0.19); // #292b30
pub const BG_PANEL: Color = Color::from_rgb(0.10, 0.105, 0.118); // #1a1b1e

pub const BORDER_SUBTLE: Color = Color::from_rgba(0.24, 0.26, 0.30, 0.5);
pub const BORDER_ACTIVE: Color = Color::from_rgb(0.23, 0.51, 0.96); // #3b82f6

pub const TEXT_PRIMARY: Color = Color::from_rgb(0.95, 0.96, 0.98);
pub const TEXT_SECONDARY: Color = Color::from_rgb(0.68, 0.72, 0.78);
pub const TEXT_MUTED: Color = Color::from_rgb(0.45, 0.48, 0.54);

pub const ACCENT_BLUE: Color = Color::from_rgb(0.23, 0.51, 0.96); // Snapseed Blue
pub const ACCENT_BLUE_HOVER: Color = Color::from_rgb(0.18, 0.44, 0.88);
pub const ACCENT_GREEN: Color = Color::from_rgb(0.06, 0.73, 0.51); // #10b981
pub const ACCENT_GREEN_HOVER: Color = Color::from_rgb(0.05, 0.62, 0.43);
pub const ACCENT_AMBER: Color = Color::from_rgb(0.96, 0.62, 0.04); // #f59e0b
pub const ACCENT_RED: Color = Color::from_rgb(0.94, 0.27, 0.27); // #ef4444
pub const ACCENT_RED_HOVER: Color = Color::from_rgb(0.82, 0.20, 0.20);

// Container styles
pub fn app_container(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(BG_APP)),
        text_color: Some(TEXT_PRIMARY),
        ..Default::default()
    }
}

pub fn header_bar(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(BG_HEADER)),
        border: border::rounded(0).color(BORDER_SUBTLE).width(1.0),
        text_color: Some(TEXT_PRIMARY),
        ..Default::default()
    }
}

pub fn inspector_panel(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(BG_PANEL)),
        border: border::rounded(0).color(BORDER_SUBTLE).width(1.0),
        text_color: Some(TEXT_PRIMARY),
        ..Default::default()
    }
}

pub fn card_container(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(BG_CARD)),
        border: border::rounded(8.0).color(BORDER_SUBTLE).width(1.0),
        text_color: Some(TEXT_PRIMARY),
        ..Default::default()
    }
}

pub fn active_card_container(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(BG_CARD_HOVER)),
        border: border::rounded(8.0).color(BORDER_ACTIVE).width(1.5),
        text_color: Some(TEXT_PRIMARY),
        ..Default::default()
    }
}

pub fn hud_overlay_container(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba(0.07, 0.075, 0.085, 0.88))),
        border: border::rounded(10.0).color(Color::from_rgba(0.3, 0.35, 0.42, 0.6)).width(1.0),
        text_color: Some(TEXT_PRIMARY),
        ..Default::default()
    }
}

pub fn pill_container(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba(0.12, 0.13, 0.15, 0.92))),
        border: border::rounded(16.0).color(BORDER_SUBTLE).width(1.0),
        text_color: Some(TEXT_PRIMARY),
        ..Default::default()
    }
}

// Button styles
pub fn primary_button(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => ACCENT_BLUE_HOVER,
        button::Status::Pressed => ACCENT_BLUE,
        button::Status::Disabled => Color::from_rgba(0.23, 0.51, 0.96, 0.3),
        button::Status::Active => ACCENT_BLUE,
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: Color::WHITE,
        border: border::rounded(6.0),
        ..Default::default()
    }
}

pub fn apply_button(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => ACCENT_GREEN_HOVER,
        button::Status::Pressed => ACCENT_GREEN,
        button::Status::Disabled => Color::from_rgba(0.06, 0.73, 0.51, 0.3),
        button::Status::Active => ACCENT_GREEN,
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: Color::WHITE,
        border: border::rounded(6.0),
        ..Default::default()
    }
}

pub fn cancel_button(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => ACCENT_RED_HOVER,
        button::Status::Pressed => ACCENT_RED,
        button::Status::Disabled => Color::from_rgba(0.94, 0.27, 0.27, 0.3),
        button::Status::Active => ACCENT_RED,
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: Color::WHITE,
        border: border::rounded(6.0),
        ..Default::default()
    }
}

pub fn ghost_button(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => BG_CARD_HOVER,
        button::Status::Pressed => BG_CARD,
        button::Status::Disabled => Color::TRANSPARENT,
        button::Status::Active => Color::TRANSPARENT,
    };
    let text = match status {
        button::Status::Disabled => TEXT_MUTED,
        _ => TEXT_PRIMARY,
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: text,
        border: border::rounded(6.0).color(BORDER_SUBTLE).width(1.0),
        ..Default::default()
    }
}

pub fn tab_button(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let (bg, text, b_color) = if active {
            (BG_CARD_HOVER, ACCENT_BLUE, ACCENT_BLUE)
        } else {
            match status {
                button::Status::Hovered => (BG_CARD, TEXT_PRIMARY, BORDER_SUBTLE),
                _ => (Color::TRANSPARENT, TEXT_SECONDARY, Color::TRANSPARENT),
            }
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: text,
            border: border::rounded(6.0).color(b_color).width(if active { 1.0 } else { 0.0 }),
            ..Default::default()
        }
    }
}

pub fn pill_button(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let (bg, text) = if active {
            (ACCENT_BLUE, Color::WHITE)
        } else {
            match status {
                button::Status::Hovered => (BG_CARD_HOVER, TEXT_PRIMARY),
                _ => (BG_CARD, TEXT_SECONDARY),
            }
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: text,
            border: border::rounded(14.0).color(if active { ACCENT_BLUE } else { BORDER_SUBTLE }).width(1.0),
            ..Default::default()
        }
    }
}
