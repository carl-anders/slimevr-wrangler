use iced::border::{self};
use iced::{
    color,
    widget::{button, container},
    Color, Theme,
};

pub fn container_highlight(_theme: &Theme) -> container::Style {
    container_base(color!(0x663499))
}
pub fn container_info(_theme: &Theme) -> container::Style {
    container_base(color!(0x424445))
}
pub fn container_darker(_theme: &Theme) -> container::Style {
    container_base(color!(0x262829))
}
fn container_base(bg: Color) -> container::Style {
    container::Style {
        background: Some(bg.into()),
        text_color: Color::WHITE.into(),
        ..container::Style::default()
    }
}

pub fn item_normal(_theme: &Theme) -> container::Style {
    item_base(color!(0x36393F))
}
pub fn _item_special(_theme: &Theme) -> container::Style {
    item_base(color!(0x3C384A))
}
fn item_base(bg: Color) -> container::Style {
    container::Style {
        background: Some(bg.into()),
        text_color: Color::WHITE.into(),
        border: border::rounded(5.0),
        ..container::Style::default()
    }
}

pub fn text_orange(_theme: &Theme) -> container::Style {
    text_base(color!(0xff623c))
}
pub fn text_yellow(_theme: &Theme) -> container::Style {
    text_base(color!(0xffe33c))
}
pub fn text_green(_theme: &Theme) -> container::Style {
    text_base(color!(0xd3fbe1))
}
fn text_base(bg: Color) -> container::Style {
    container::Style {
        text_color: bg.into(),
        ..container::Style::default()
    }
}

pub fn button_primary(_theme: &Theme, status: button::Status) -> button::Style {
    let active = button::Style {
        background: Some(color!(0x663499).into()),
        border: border::rounded(3.0),
        text_color: Color::WHITE,
        ..button::Style::default()
    };
    let hovered = button::Style {
        background: Some(color!(0x502882).into()),
        text_color: Color::WHITE,
        ..active
    };
    match status {
        button::Status::Active => active,
        button::Status::Hovered => hovered,
        button::Status::Pressed => button::Style {
            border: border::color(Color::WHITE).width(1.0),
            ..hovered
        },
        button::Status::Disabled => button::Style {
            background: Some(color!(0x736780).into()),
            ..active
        },
    }
}
pub fn button_settings(_theme: &Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: None,
        text_color: Color::WHITE,
        ..button::Style::default()
    }
}
