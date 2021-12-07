macro_rules! color_rgb {
    ($r:expr, $g:expr, $b:expr) => {
        Color::from_rgb($r as f32 / 255.0, $g as f32 / 255.0, $b as f32 / 255.0)
    };
}

use iced::{button, container, Color};

const ACTIVE: Color = color_rgb!(0x66, 0x34, 0x99);

const HOVERED: Color = color_rgb!(0x50, 0x28, 0x82);

pub enum Background {
    Highlight,
    Darker,
}

impl container::StyleSheet for Background {
    fn style(&self) -> container::Style {
        let bg = match self {
            Background::Highlight => Color::from_rgb8(0x66, 0x34, 0x99),
            Background::Darker => Color::from_rgb8(0x26, 0x28, 0x29),
        };
        container::Style {
            background: bg.into(),
            text_color: Color::WHITE.into(),
            ..container::Style::default()
        }
    }
}

pub enum Item {
    Normal,
    Special,
}

impl container::StyleSheet for Item {
    fn style(&self) -> container::Style {
        let bg = match self {
            Item::Normal => Color::from_rgb8(0x36, 0x39, 0x3F),
            Item::Special => Color::from_rgb8(0x3c, 0x38, 0x4A),
        };
        container::Style {
            background: bg.into(),
            text_color: Color::WHITE.into(),
            border_radius: 5.0,
            ..container::Style::default()
        }
    }
}

pub enum Button {
    Primary,
}

impl button::StyleSheet for Button {
    fn active(&self) -> button::Style {
        button::Style {
            background: ACTIVE.into(),
            border_radius: 3.0,
            text_color: Color::WHITE,
            ..button::Style::default()
        }
    }

    fn hovered(&self) -> button::Style {
        button::Style {
            background: HOVERED.into(),
            text_color: Color::WHITE,
            ..self.active()
        }
    }

    fn pressed(&self) -> button::Style {
        button::Style {
            border_width: 1.0,
            border_color: Color::WHITE,
            ..self.hovered()
        }
    }
}

pub enum Settings {
    Primary,
}

impl button::StyleSheet for Settings {
    fn active(&self) -> button::Style {
        button::Style {
            background: None,
            border_radius: 0.0,
            border_width: 0.0,
            text_color: Color::WHITE,
            ..button::Style::default()
        }
    }
}
