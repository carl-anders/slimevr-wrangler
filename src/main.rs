#![deny(clippy::all)]

use iced::{
    executor,
    pure::{
        button, column, container, horizontal_space, row, scrollable, slider, text, text_input,
        vertical_space,
        widget::{Column, Container, Row, Scrollable},
        Application, Element,
    },
    time, window, Alignment, Command, Font, Length, Settings, Subscription, Svg,
};
use itertools::Itertools;
use std::{
    net::SocketAddr,
    time::{Duration, Instant},
};
mod joycon;
mod steam_blacklist;
use crate::joycon::{JoyconIntegration, JoyconStatus, JoyconSvg};
use steam_blacklist as blacklist;
mod settings;
mod slime;
mod style;
mod update;

const WINDOW_SIZE: (u32, u32) = (980, 700);

pub const ICONS: Font = Font::External {
    name: "Icons",
    bytes: include_bytes!("../assets/icons.ttf"),
};

pub fn main() -> iced::Result {
    let settings = Settings {
        window: window::Settings {
            min_size: Some(WINDOW_SIZE),
            size: WINDOW_SIZE,
            ..window::Settings::default()
        },
        antialiasing: true,
        ..Settings::default()
    };
    MainState::run(settings)
}

#[derive(Debug, Clone)]
enum Message {
    EventOccurred(iced_native::Event),
    EnableJoyconsPressed,
    SettingsPressed,
    Tick(Instant),
    Dot(Instant),
    AddressChange(String),
    UpdateFound(Option<String>),
    UpdatePressed,
    BlacklistChecked(blacklist::BlacklistResult),
    BlacklistFixPressed,
    JoyconRotate(String, bool),
    JoyconScale(String, f64),
}

#[derive(Default)]
struct MainState {
    joycon: Option<JoyconIntegration>,
    joycon_boxes: Vec<JoyconBox>,
    joycon_svg: JoyconSvg,
    num_columns: usize,
    search_dots: usize,
    settings_show: bool,

    settings: settings::Handler,
    update_found: Option<String>,
    blacklist_info: blacklist::BlacklistResult,
}
impl Application for MainState {
    type Executor = executor::Default;
    type Flags = ();
    type Message = Message;

    fn new(_: Self::Flags) -> (Self, Command<Self::Message>) {
        (
            Self {
                num_columns: 3,
                joycon_svg: JoyconSvg::new(),
                ..Self::default()
            },
            Command::batch(vec![
                Command::perform(update::check_updates(), Message::UpdateFound),
                Command::perform(blacklist::check_blacklist(), Message::BlacklistChecked),
            ]),
        )
    }

    fn title(&self) -> String {
        String::from("SlimeVR Wrangler")
    }

    fn update(&mut self, message: Message) -> Command<Self::Message> {
        match message {
            Message::EnableJoyconsPressed => {
                self.joycon = Some(JoyconIntegration::new(self.settings.clone()));
            }
            Message::SettingsPressed => {
                self.settings_show = !self.settings_show;
            }
            Message::EventOccurred(iced_native::Event::Window(
                iced_native::window::Event::Resized { width, .. },
            )) => {
                if width >= WINDOW_SIZE.0 {
                    self.num_columns = ((width - 20) / (300 + 20)) as usize;
                }
            }
            Message::EventOccurred(_) => {}
            Message::Tick(_time) => {
                if let Some(ref ji) = self.joycon {
                    if let Some(mut res) = ji.poll() {
                        if res.len() == self.joycon_boxes.len() {
                            for i in 0..self.joycon_boxes.len() {
                                self.joycon_boxes[i].status = res.remove(0);
                            }
                        } else {
                            self.joycon_boxes = Vec::new();
                            for _ in 0..res.len() {
                                self.joycon_boxes.push(JoyconBox::new(res.remove(0)));
                            }
                        }
                    }
                }
            }
            Message::Dot(_time) => {
                self.search_dots = (self.search_dots + 1) % 4;
            }
            Message::AddressChange(value) => {
                self.settings.change(|ws| ws.address = value);
            }
            Message::UpdateFound(version) => {
                self.update_found = version;
            }
            Message::UpdatePressed => {
                self.update_found = None;
                update::update();
            }
            Message::BlacklistChecked(info) => {
                self.blacklist_info = info;
            }
            Message::BlacklistFixPressed => {
                self.blacklist_info =
                    blacklist::BlacklistResult::info("Updating steam config file.....");
                return Command::perform(blacklist::update_blacklist(), Message::BlacklistChecked);
            }
            Message::JoyconRotate(serial_number, direction) => {
                self.settings.change(|ws| {
                    ws.joycon_rotation_add(serial_number, if direction { 90 } else { -90 });
                });
            }
            Message::JoyconScale(serial_number, scale) => {
                self.settings
                    .change(|ws| ws.joycon_scale_set(serial_number, scale));
            }
        }
        Command::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        let mut subs: Vec<Subscription<Message>> = vec![
            iced_native::subscription::events().map(Message::EventOccurred),
            time::every(Duration::from_millis(500)).map(Message::Dot),
        ];
        if self.joycon.is_some() {
            subs.push(time::every(Duration::from_millis(50)).map(Message::Tick));
        }
        Subscription::batch(subs)
    }

    fn view(&self) -> Element<Message> {
        let mut main_container;
        if self.settings_show {
            let mut all_settings = column()
                .spacing(20)
                .push(address(&self.settings.load().address));
            if self.joycon.is_some() {
                all_settings = all_settings.push(text("You need to restart this program to apply the settings as you have already initialized search for controllers."));
            }
            main_container = container(all_settings).padding(20);
        } else {
            let search_dots = ".".repeat(self.search_dots);

            let mut boxes: Vec<Container<Message>> = Vec::new();

            if self.joycon.is_some() {
                for joycon_box in &self.joycon_boxes {
                    let svg = self
                        .joycon_svg
                        .get(&joycon_box.status.design, joycon_box.status.mount_rotation);
                    let scale = self
                        .settings
                        .load()
                        .joycon_scale_get(&joycon_box.status.serial_number);
                    boxes.push(contain(joycon_box.view(svg, scale)).style(style::Item::Normal));
                }
                boxes.push(
                    contain(
                    column()
                        .push(
                            text(format!(
                                "Looking for Joycon controllers{}\n\n\
                                Please pair controllers in the bluetooth settings of Windows if they don't show up here.",
                                search_dots
                            ))
                        )
                        .align_items(Alignment::Center),
                    ).style(style::Item::Special)
                );
            } else {
                let feature_enabler = column().spacing(10).push(text("Add new trackers")).push(
                    button(text("Search for Joy-Con's"))
                        .on_press(Message::EnableJoyconsPressed)
                        .style(style::Button::Primary),
                );
                boxes.push(contain(feature_enabler).style(style::Item::Special));
            }

            let list = float_list(self.num_columns, boxes);

            main_container = container(list);
        }

        main_container = main_container
            .width(Length::Fill)
            .height(Length::Fill)
            .style(style::Background::Darker);

        let top_bar = top_bar(self.update_found.clone());

        let mut app = column().push(top_bar);
        if self.blacklist_info.visible() {
            app = app.push(blacklist_bar(&self.blacklist_info));
        }
        app.push(main_container).into()
    }
}

fn contain<'a, M: 'a, T>(content: T) -> Container<'a, M>
where
    T: Into<Element<'a, M>>,
{
    container(content)
        .height(Length::Units(280))
        .width(Length::Units(300))
        .padding(10)
}
fn float_list(columns: usize, boxes: Vec<Container<'_, Message>>) -> Scrollable<'_, Message> {
    let mut list = column().padding(20).spacing(20).width(Length::Fill);
    for chunk in &boxes.into_iter().chunks(columns) {
        let mut row: Row<Message> = row().spacing(20);

        for bax in chunk {
            row = row.push(bax);
        }
        list = list.push(row);
    }
    scrollable(list).height(Length::Fill)
}
fn address<'a>(input_value: &str) -> Column<'a, Message> {
    let address_info = text("Enter a valid ip with port number:");
    let address = text_input("127.0.0.1:6969", input_value, Message::AddressChange)
        .width(Length::Units(500))
        .padding(10);

    let mut allc = column().spacing(10).push(address_info).push(address);

    if input_value.parse::<SocketAddr>().is_ok() {
        allc = allc.push(vertical_space(Length::Units(20)));
    } else {
        allc = allc.push(text(
            "Input not a valid ip with port number! Using default instead (127.0.0.1:6969).",
        ));
    }
    allc
}
fn top_bar<'a>(update: Option<String>) -> Container<'a, Message> {
    let mut top_column = row()
        .align_items(Alignment::Center)
        .push(text("SlimeVR Wrangler").size(24));

    if let Some(u) = update {
        let update_btn = button(text("Update"))
            .style(style::Button::Primary)
            .on_press(Message::UpdatePressed);
        top_column = top_column
            .push(horizontal_space(Length::Units(20)))
            .push(text(format!("New Update found! Version: {}. ", u)))
            .push(update_btn);
    }

    let settings = button(text("Settings"))
        .style(style::Settings::Primary)
        .on_press(Message::SettingsPressed);
    top_column = top_column
        .push(horizontal_space(Length::Fill))
        .push(settings);

    container(top_column)
        .width(Length::Fill)
        .padding(20)
        .style(style::Background::Highlight)
}

fn blacklist_bar<'a>(result: &blacklist::BlacklistResult) -> Container<'a, Message> {
    let mut row = row()
        .align_items(Alignment::Center)
        .push(text(result.info.clone()))
        .push(horizontal_space(Length::Units(20)));
    if result.fix_button {
        row = row.push(
            button(text("Fix blacklist"))
                .style(style::Button::Primary)
                .on_press(Message::BlacklistFixPressed),
        );
    }
    container(row)
        .width(Length::Fill)
        .padding(20)
        .style(style::Background::Info)
}

#[derive(Debug, Clone)]
struct JoyconBox {
    pub status: JoyconStatus,
}

impl JoyconBox {
    fn new(status: JoyconStatus) -> Self {
        Self { status }
    }
    fn view(&self, svg: Svg, scale: f64) -> Column<Message> {
        let sn = self.status.serial_number.clone();

        let buttons = row()
            .spacing(10)
            .push(
                button(text("↺").font(ICONS))
                    .on_press(Message::JoyconRotate(sn.clone(), false))
                    .style(style::Button::Primary),
            )
            .push(
                button(text("↻").font(ICONS))
                    .on_press(Message::JoyconRotate(sn.clone(), true))
                    .style(style::Button::Primary),
            );

        let left = column()
            .spacing(10)
            .align_items(Alignment::Center)
            .push(buttons)
            .push(svg)
            .width(Length::Units(150));

        let top = row()
            .spacing(10)
            .push(left)
            .push(text(format!(
                "Roll: {:.0}\nPitch: {:.0}\nYaw: {:.0}",
                self.status.rotation.0, self.status.rotation.1, self.status.rotation.2
            )))
            .height(Length::Units(160));

        let bottom = column()
            .spacing(10)
            .push(
                slider(0.8..=1.2, scale, move |c| {
                    Message::JoyconScale(sn.clone(), c)
                })
                .step(0.001),
            )
            .push(text(format!("Rotation scale ratio: {:.3}", scale)))
            .push(text("Change this if the tracker in vr moves less or more than your irl joycon. Higher value = more movement.").size(14));

        column().spacing(10).push(top).push(bottom)
    }
}
