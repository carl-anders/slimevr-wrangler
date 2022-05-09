#![deny(clippy::all)]

use iced::{
    button, executor, scrollable, text_input, time, window, Align, Application, Button, Clipboard,
    Column, Command, Container, Element, Length, Row, Scrollable, Settings, Space, Subscription,
    Svg, Text, TextInput,
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

const WINDOW_SIZE: (u32, u32) = (980, 700);

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
    JoyconRotate(String, bool),
}

#[derive(Default)]
struct Buttons {
    enable_joycon: button::State,
    settings: button::State,
    update: button::State,
}

#[derive(Default)]
struct MainState {
    joycon: Option<JoyconIntegration>,
    joycon_boxes: Vec<JoyconBox>,
    joycon_svg: JoyconSvg,
    num_columns: usize,
    search_dots: usize,
    settings_show: bool,

    address_state: text_input::State,

    settings: settings::Handler,
    update_found: Option<String>,

    buttons: Buttons,
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
            Message::JoyconRotate(serial_number, direction) => {
                self.settings.change(|ws| {
                    ws.joycon_rotation_add(serial_number, if direction { 90 } else { -90 })
                });
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

    fn view(&mut self) -> Element<Message> {
        let mut main_container;
        if self.settings_show {
            let mut all_settings = Column::new().spacing(20).push(address(
                &mut self.address_state,
                &self.settings.load().address,
            ));
            if self.joycon.is_some() {
                all_settings = all_settings.push(Text::new("You need to restart this program to apply the settings as you have already initialized search for controllers."));
            }
            main_container = Container::new(all_settings).padding(20);
        } else {
            let search_dots = ".".repeat(self.search_dots);

            let mut boxes: Vec<Container<Message>> = Vec::new();

            if self.joycon.is_some() {
                for joycon_box in &mut self.joycon_boxes {
                    let svg = self
                        .joycon_svg
                        .get(&joycon_box.status.design, joycon_box.status.mount_rotation)
                        .clone();
                    boxes.push(contain(joycon_box.view(svg)).style(style::Item::Normal));
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
                            &mut self.buttons.enable_joycon,
                            Text::new("Search for Joy-Con's"),
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
            &mut self.buttons.settings,
            &mut self.buttons.update,
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
    let address_info = Text::new("Enter a valid ip with port number:");
    let address = TextInput::new(input, "127.0.0.1:6969", input_value, Message::AddressChange)
        .width(Length::Units(500))
        .padding(10);

    let mut allc = Column::new().spacing(10).push(address_info).push(address);

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

#[derive(Debug, Clone)]
struct JoyconBox {
    left: button::State,
    right: button::State,
    pub status: JoyconStatus,
}

impl JoyconBox {
    fn new(status: JoyconStatus) -> Self {
        Self {
            left: button::State::new(),
            right: button::State::new(),
            status,
        }
    }
    fn view(&mut self, svg: Svg) -> Row<Message> {
        let buttons = Row::new()
            .spacing(10)
            .push(
                Button::new(&mut self.left, Text::new("←"))
                    .on_press(Message::JoyconRotate(
                        self.status.serial_number.clone(),
                        false,
                    ))
                    .style(style::Button::Primary),
            )
            .push(
                Button::new(&mut self.right, Text::new("→"))
                    .on_press(Message::JoyconRotate(
                        self.status.serial_number.clone(),
                        true,
                    ))
                    .style(style::Button::Primary),
            );
        let left = Column::new()
            .spacing(10)
            .align_items(Align::Center)
            .push(buttons)
            .push(svg);

        Row::new().spacing(10).push(left).push(Text::new(format!(
            "roll: {:.0}\npitch: {:.0}\nyaw: {:.0}",
            self.status.rotation.0, self.status.rotation.1, self.status.rotation.2
        )))
    }
}
