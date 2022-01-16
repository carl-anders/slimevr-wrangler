#![deny(clippy::all)]
#![allow(clippy::single_match)]

use iced::{
    button, executor, scrollable, text_input, time, window, Align, Application, Button, Clipboard,
    Column, Command, Container, Element, Length, Row, Scrollable, Settings, Space, Subscription,
    Text, TextInput,
};
use itertools::Itertools;
use std::{
    net::SocketAddr,
    time::{Duration, Instant},
};
mod joycon;
use crate::joycon::{JoyconIntegration, JoyconStatus, JoyconSvg};
mod settings;
mod slime;
mod style;
mod update;
use settings::WranglerSettings;

//const VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub fn main() -> iced::Result {
    let settings = Settings {
        window: window::Settings {
            min_size: Some((980, 700)),
            size: (980, 700),
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
    AddressChanged(String),
    UpdateFound(Option<String>),
    UpdatePressed,
}

#[derive(Default)]
struct MainState {
    joycon: Option<JoyconIntegration>,
    joycon_statuses: Vec<JoyconStatus>,
    joycon_svg: JoyconSvg,
    num_columns: usize,
    search_dots: usize,
    settings_show: bool,

    address_state: text_input::State,

    settings: WranglerSettings,
    update_found: Option<String>,

    button_enable_joycon: button::State,
    button_settings: button::State,
    button_update: button::State,
    scroll: scrollable::State,
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
                settings: WranglerSettings::new(),
                ..Self::default()
            },
            Command::perform(update::check_updates(), Message::UpdateFound),
        )
    }

    fn title(&self) -> String {
        String::from("SlimeVR Wrangler")
    }

    fn update(&mut self, message: Message, _: &mut Clipboard) -> Command<Self::Message> {
        match message {
            Message::EnableJoyconsPressed => {
                self.joycon = Some(JoyconIntegration::new(self.settings.address.clone()));
            }
            Message::SettingsPressed => {
                self.settings_show = !self.settings_show;
            }
            Message::EventOccurred(event) => match event {
                iced_native::Event::Window(iced_native::window::Event::Resized {
                    width, ..
                }) => {
                    self.num_columns = ((width - 20) / (300 + 20)) as usize;
                }
                _ => (),
            },
            Message::Tick(_time) => {
                if let Some(ref ji) = self.joycon {
                    if let Some(res) = ji.poll() {
                        self.joycon_statuses = res;
                    }
                }
            }
            Message::Dot(_time) => {
                self.search_dots = (self.search_dots + 1) % 4;
            }
            Message::AddressChanged(value) => {
                self.settings.address = value;
                self.settings.save();
            }
            Message::UpdateFound(version) => {
                self.update_found = version;
            }
            Message::UpdatePressed => {
                self.update_found = None;
                update::update();
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
            subs.push(time::every(Duration::from_millis(100)).map(Message::Tick));
        }
        Subscription::batch(subs)
    }

    fn view(&mut self) -> Element<Message> {
        let mut main_container;
        if self.settings_show {
            let mut all_settings = Column::new()
                .spacing(20)
                .push(address(&mut self.address_state, &self.settings.address));
            if self.joycon.is_some() {
                all_settings = all_settings.push(Text::new("You need to restart this program to apply the settings as you have already initialized search for controllers."));
            }
            main_container = Container::new(all_settings).padding(20);
        } else {
            let search_dots = ".".repeat(self.search_dots);

            let mut boxes: Vec<Container<Message>> = Vec::new();

            if self.joycon.is_some() {
                for status in self.joycon_statuses.clone() {
                    let info = Row::new()
                        .spacing(10)
                        .push(self.joycon_svg.get(&status.design).clone())
                        .push(Text::new(format!(
                            "roll: {:.0}\npitch: {:.0}\nyaw: {:.0}",
                            status.rotation.0, status.rotation.1, status.rotation.2
                        )));
                    boxes.push(contain(info).style(style::Item::Normal));
                }
                boxes.push(
                    contain(
                    Column::new()
                        .push(
                            Text::new(format!(
                                "Looking for Joycon controllers{}\n\n\
                                Please pair controllers in the bluetooth settings of Windows if they don't show up here.",
                                search_dots
                            ))
                        )
                        .align_items(Align::Center),
                    ).style(style::Item::Special)
                );
            } else {
                let feature_enabler = Column::new()
                    .spacing(10)
                    .push(Text::new("Add new trackers"))
                    .push(
                        Button::new(
                            &mut self.button_enable_joycon,
                            Text::new("Search for Joycons"),
                        )
                        .on_press(Message::EnableJoyconsPressed)
                        .style(style::Button::Primary),
                    );
                //feature_enabler = feature_enabler.push(Space::new(Length::Fill, Length::Units(30)))
                boxes.push(contain(feature_enabler).style(style::Item::Special));
            }

            let list = float_list(&mut self.scroll, self.num_columns, boxes);

            main_container = Container::new(list);
        }

        main_container = main_container
            .width(Length::Fill)
            .height(Length::Fill)
            .style(style::Background::Darker);

        let top_bar = top_bar(
            &mut self.button_settings,
            &mut self.button_update,
            self.update_found.clone(),
        );

        Column::new().push(top_bar).push(main_container).into()
    }
}

fn contain<'a, M: 'a, T>(content: T) -> Container<'a, M>
where
    T: Into<Element<'a, M>>,
{
    Container::new(content)
        .height(Length::Units(200))
        .width(Length::Units(300))
        .padding(10)
}
fn float_list<'a>(
    scroll_state: &'a mut scrollable::State,
    columns: usize,
    boxes: Vec<Container<'a, Message>>,
) -> Scrollable<'a, Message> {
    let mut list = Scrollable::new(scroll_state)
        .padding(20)
        .spacing(20)
        .width(Length::Fill)
        .height(Length::Fill);
    for chunk in &boxes.into_iter().chunks(columns) {
        let mut row: Row<Message> = Row::new().spacing(20);

        for bax in chunk {
            row = row.push(bax);
        }
        list = list.push(row);
    }
    list
}
fn address<'a>(input: &'a mut text_input::State, input_value: &str) -> Column<'a, Message> {
    let adress_info = Text::new("Enter a valid ip with port number:");
    let adress = TextInput::new(
        input,
        "127.0.0.1:6969",
        input_value,
        Message::AddressChanged,
    )
    .width(Length::Units(500))
    .padding(10);

    let mut allc = Column::new().spacing(10).push(adress_info).push(adress);

    if input_value.parse::<SocketAddr>().is_ok() {
        allc = allc.push(Space::new(Length::Fill, Length::Units(20)));
    } else {
        allc = allc.push(Text::new(
            "Input not a valid ip with port number! Using default instead (127.0.0.1:6969).",
        ));
    }
    allc
}
fn top_bar<'a>(
    button_settings: &'a mut button::State,
    button_update: &'a mut button::State,
    update: Option<String>,
) -> Container<'a, Message> {
    let mut top_column = Row::new()
        .align_items(Align::Center)
        .push(Text::new("SlimeVR Wrangler").size(24));

    if let Some(u) = update {
        let update_btn = Button::new(button_update, Text::new("Update"))
            .style(style::Button::Primary)
            .on_press(Message::UpdatePressed);
        top_column = top_column
            .push(Space::new(Length::Units(20), Length::Shrink))
            .push(Text::new(format!("New Update found! Version: {}. ", u)))
            .push(update_btn);
    }

    let settings = Button::new(button_settings, Text::new("Settings"))
        .style(style::Settings::Primary)
        .on_press(Message::SettingsPressed);
    top_column = top_column
        .push(Space::new(Length::Fill, Length::Shrink))
        .push(settings);

    Container::new(top_column)
        .width(Length::Fill)
        .padding(20)
        .style(style::Background::Highlight)
}
