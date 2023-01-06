use iced::{
    widget::{button, container},
    Color, Theme,
};

pub fn container_highlight(_theme: &Theme) -> container::Appearance {
    container_base(Color::from_rgb8(0x66, 0x34, 0x99))
}
pub fn container_info(_theme: &Theme) -> container::Appearance {
    container_base(Color::from_rgb8(0x42, 0x44, 0x45))
}
pub fn container_darker(_theme: &Theme) -> container::Appearance {
    container_base(Color::from_rgb8(0x26, 0x28, 0x29))
}
fn container_base(bg: Color) -> container::Appearance {
    container::Appearance {
        background: bg.into(),
        text_color: Color::WHITE.into(),
        ..container::Appearance::default()
    }
}

pub fn item_normal(_theme: &Theme) -> container::Appearance {
    item_base(Color::from_rgb8(0x36, 0x39, 0x3F))
}
pub fn _item_special(_theme: &Theme) -> container::Appearance {
    item_base(Color::from_rgb8(0x3c, 0x38, 0x4A))
}
fn item_base(bg: Color) -> container::Appearance {
    container::Appearance {
        background: bg.into(),
        text_color: Color::WHITE.into(),
        border_radius: 5.0,
        ..container::Appearance::default()
    }
}

pub fn text_orange(_theme: &Theme) -> container::Appearance {
    text_base(Color::from_rgb8(0xff, 0x62, 0x3c))
}
pub fn text_yellow(_theme: &Theme) -> container::Appearance {
    text_base(Color::from_rgb8(0xff, 0xe3, 0x3c))
}
pub fn text_green(_theme: &Theme) -> container::Appearance {
    text_base(Color::from_rgb8(0xd3, 0xfb, 0xe1))
}
fn text_base(bg: Color) -> container::Appearance {
    container::Appearance {
        text_color: bg.into(),
        ..container::Appearance::default()
    }
}

pub struct PrimaryButton;

impl button::StyleSheet for PrimaryButton {
    type Style = Theme;
    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Color::from_rgb8(0x66, 0x34, 0x99).into(),
            border_radius: 3.0,
            text_color: Color::WHITE,
            ..button::Appearance::default()
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Color::from_rgb8(0x50, 0x28, 0x82).into(),
            text_color: Color::WHITE,
            ..self.active(style)
        }
    }

    fn pressed(&self, style: &Self::Style) -> button::Appearance {
        button::Appearance {
            border_width: 1.0,
            border_color: Color::WHITE,
            ..self.hovered(style)
        }
    }
}

pub struct SettingsButton;

impl button::StyleSheet for SettingsButton {
    type Style = Theme;
    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: None,
            border_radius: 0.0,
            border_width: 0.0,
            text_color: Color::WHITE,
            ..button::Appearance::default()
        }
    }
}
