#![deny(clippy::all)]

use iced::{
    executor,
    theme::{self, Theme},
    time,
    widget::{
        button, canvas, container, horizontal_space, scrollable, slider, text, text_input, Column,
        Container, Row, Svg,
    },
    window, Alignment, Application, Command, Element, Font, Length, Settings, Subscription,
};

use itertools::Itertools;
use joycon::ServerStatus;
use joycon_rs::prelude::input_report_mode::BatteryLevel;
use needle::Needle;
use std::{
    io::{
        self,
        prelude::{Read, Write},
    },
    net::SocketAddr,
    time::{Duration, Instant},
};
mod joycon;
mod steam_blacklist;
use steam_blacklist as blacklist;
mod needle;
mod settings;
mod style;
mod update;

const WINDOW_SIZE: (u32, u32) = (980, 700);

pub const ICONS: Font = Font::External {
    name: "Icons",
    bytes: include_bytes!("../assets/icons.ttf"),
};
pub const ICON: &[u8; 16384] = include_bytes!("../assets/icon_64.rgba8");

pub fn main() -> iced::Result {
    /*
    let rgba8 = image_rs::io::Reader::open("assets/icon.png").unwrap().decode().unwrap().to_rgba8();
    std::fs::write("assets/icon_64.rgba8", rgba8.into_raw());
    */
    let settings = Settings {
        window: window::Settings {
            min_size: Some(WINDOW_SIZE),
            size: WINDOW_SIZE,
            icon: window::Icon::from_rgba(ICON.to_vec(), 64, 64).ok(),
            ..window::Settings::default()
        },
        antialiasing: true,
        ..Settings::default()
    };
    match MainState::run(settings) {
        Ok(a) => Ok(a),
        Err(e) => {
            println!("{e:?}");
            print!("Press enter to continue...");
            io::stdout().flush().unwrap();
            let _ = io::stdin().read(&mut [0u8]).unwrap();
            Err(e)
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    EventOccurred(iced_native::Event),
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
    joycon: Option<joycon::Wrapper>,
    joycon_boxes: Vec<JoyconBox>,
    joycon_svg: joycon::Svg,
    num_columns: usize,
    search_dots: usize,
    settings_show: bool,
    server_connected: ServerStatus,
    server_address: String,

    settings: settings::Handler,
    update_found: Option<String>,
    blacklist_info: blacklist::BlacklistResult,
    needles: Vec<Needle>,
}
impl Application for MainState {
    type Executor = executor::Default;
    type Flags = ();
    type Message = Message;
    type Theme = Theme;

    fn new(_: Self::Flags) -> (Self, Command<Self::Message>) {
        let mut new = Self {
            num_columns: 3,
            joycon_svg: joycon::Svg::new(),
            ..Self::default()
        };
        new.joycon = Some(joycon::Wrapper::new(new.settings.clone()));
        new.server_address = format!("{}", new.settings.load().get_socket_address());
        for i in 0..360 {
            new.needles.push(Needle::new(i));
        }
        (
            new,
            Command::batch(vec![
                Command::perform(update::check_updates(), Message::UpdateFound),
                Command::perform(blacklist::check_blacklist(), Message::BlacklistChecked),
            ]),
        )
    }

    fn title(&self) -> String {
        String::from("SlimeVR Wrangler")
    }
    fn theme(&self) -> Theme {
        Theme::Dark
    }

    fn update(&mut self, message: Message) -> Command<Self::Message> {
        match message {
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
                    if let Some(mut res) = ji.poll_status() {
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
                    if let Some(connected) = ji.poll_server() {
                        self.server_connected = connected;
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
        let search_dots = ".".repeat(self.search_dots);
        let main_container =
        if self.settings_show {
            let all_settings = Column::new()
                .spacing(20)
                .push(address(&self.settings.load().address))
                .push(text(
                    "You need to restart this program after changing this.",
                ));
            container(all_settings).padding(20)
        } else {
            let settings = self.settings.load();
            let mut boxes: Vec<Container<Message>> = Vec::new();
            for joycon_box in &self.joycon_boxes {
                let scale = settings.joycon_scale_get(&joycon_box.status.serial_number);
                boxes.push(
                    contain(joycon_box.view(&self.joycon_svg, scale, &self.needles))
                        .style(style::item_normal as for<'r> fn(&'r _) -> _),
                );
            }

            let list = float_list(self.num_columns, boxes).push(
                    text(format!(
                        "Searching for Joycon controllers{search_dots}\n\
                        Please pair controllers in the bluetooth settings of Windows if they don't show up here.",
                    ))
                );

            let scrollable = scrollable(list).height(Length::Fill);

            container(scrollable)
        }
            .width(Length::Fill)
            .height(Length::Fill)
            .style(style::container_darker as for<'r> fn(&'r _) -> _);

        let top_bar = top_bar(self.update_found.clone());

        let mut app = Column::new().push(top_bar);
        if self.blacklist_info.visible() {
            app = app.push(blacklist_bar(&self.blacklist_info));
        }

        let bottom_bar = bottom_bar(self.server_connected, &search_dots, &self.server_address);

        app.push(main_container).push(bottom_bar).into()
    }
}

fn contain<'a, M: 'a, T>(content: T) -> Container<'a, M>
where
    T: Into<Element<'a, M>>,
{
    container(content)
        .height(Length::Units(300))
        .width(Length::Units(300))
        .padding(10)
}
fn float_list(columns: usize, boxes: Vec<Container<'_, Message>>) -> Column<'_, Message> {
    let mut list = Column::new().padding(20).spacing(20).width(Length::Fill);
    for chunk in &boxes.into_iter().chunks(columns) {
        let mut row: Row<Message> = Row::new().spacing(20);

        for bax in chunk {
            row = row.push(bax);
        }
        list = list.push(row);
    }
    list
}
fn address<'a>(input_value: &str) -> Column<'a, Message> {
    let address_info = text("Enter a valid ip with port number:");
    let address = text_input("127.0.0.1:6969", input_value, Message::AddressChange)
        .width(Length::Units(500))
        .padding(10);

    let mut allc = Column::new().spacing(10).push(address_info).push(address);

    if input_value.parse::<SocketAddr>().is_err() {
        allc = allc.push(
            container(text(
                "Input not a valid ip with port number! Using default instead (127.0.0.1:6969).",
            ))
            .style(style::text_yellow as for<'r> fn(&'r _) -> _),
        );
    }
    allc
}
fn top_bar<'a>(update: Option<String>) -> Container<'a, Message> {
    let mut top_column = Row::new()
        .align_items(Alignment::Center)
        .push(text("SlimeVR Wrangler").size(24));

    if let Some(u) = update {
        let update_btn = button(text("Update"))
            .style(theme::Button::Custom(Box::new(style::PrimaryButton)))
            .on_press(Message::UpdatePressed);
        top_column = top_column
            .push(horizontal_space(Length::Units(20)))
            .push(text(format!("New update found! Version: {u}. ")))
            .push(update_btn);
    }

    let settings = button(text("Settings"))
        .style(theme::Button::Custom(Box::new(style::PrimaryButton)))
        .on_press(Message::SettingsPressed);
    top_column = top_column
        .push(horizontal_space(Length::Fill))
        .push(settings);

    container(top_column)
        .width(Length::Fill)
        .padding(20)
        .style(style::container_highlight as for<'r> fn(&'r _) -> _)
}

fn blacklist_bar<'a>(result: &blacklist::BlacklistResult) -> Container<'a, Message> {
    let mut row = Row::new()
        .align_items(Alignment::Center)
        .push(text(result.info.clone()))
        .push(horizontal_space(Length::Units(20)));
    if result.fix_button {
        row = row.push(
            button(text("Fix blacklist"))
                .style(theme::Button::Custom(Box::new(style::PrimaryButton)))
                .on_press(Message::BlacklistFixPressed),
        );
    }
    container(row)
        .width(Length::Fill)
        .padding(20)
        .style(style::container_info as for<'r> fn(&'r _) -> _)
}

fn bottom_bar<'a>(
    connected: ServerStatus,
    search_dots: &String,
    address: &String,
) -> Container<'a, Message> {
    let status = Row::new()
        .push(text("Connection to SlimeVR Server: "))
        .push(container(text(format!("{connected:?}"))).style(
            if connected == ServerStatus::Connected {
                style::text_green
            } else {
                style::text_yellow
            },
        ))
        .push(text(if connected == ServerStatus::Connected {
            format!(" to {address}.")
        } else {
            format!(". Trying to connect to {address}{search_dots}")
        }));
    container(status)
        .width(Length::Fill)
        .padding(20)
        .style(style::container_info as for<'r> fn(&'r _) -> _)
}

#[derive(Debug, Clone)]
struct JoyconBox {
    pub status: joycon::Status,
}

impl JoyconBox {
    const fn new(status: joycon::Status) -> Self {
        Self { status }
    }
    fn view<'a>(
        &'a self,
        svg_handler: &joycon::Svg,
        scale: f64,
        needles: &'a [Needle],
    ) -> Column<Message> {
        let sn = self.status.serial_number.clone();

        let buttons = Row::new()
            .spacing(10)
            .push(
                button(text("↺").font(ICONS))
                    .on_press(Message::JoyconRotate(sn.clone(), false))
                    .style(theme::Button::Custom(Box::new(style::PrimaryButton))),
            )
            .push(
                button(text("↻").font(ICONS))
                    .on_press(Message::JoyconRotate(sn.clone(), true))
                    .style(theme::Button::Custom(Box::new(style::PrimaryButton))),
            );

        let svg = Svg::new(svg_handler.get(&self.status.design, self.status.mount_rotation));

        let left = Column::new()
            .spacing(10)
            .align_items(Alignment::Center)
            .push(buttons)
            .push(svg)
            .width(Length::Units(140));

        let rot = self.status.rotation;
        let values = Row::with_children(
            [("Roll", rot.0), ("Pitch", rot.1), ("Yaw", -rot.2)]
                .iter()
                .map(|(name, val)| {
                    let ival = (*val as i32).rem_euclid(360) as usize;
                    let needle = needles.get(ival).unwrap_or_else(|| &needles[0]);

                    Column::new()
                        .push(text(name))
                        .push(
                            canvas(needle)
                                .width(Length::Units(25))
                                .height(Length::Units(25)),
                        )
                        .push(text(format!("{ival}")))
                        .spacing(10)
                        .align_items(Alignment::Center)
                        .width(Length::Fill)
                        .into()
                })
                .collect(),
        );

        let top = Row::new()
            .spacing(10)
            .push(left)
            .push(values)
            .height(Length::Units(150));

        let battery_text = container(text(format!("{:?}", self.status.battery_level))).style(
            match self.status.battery_level {
                BatteryLevel::Empty | BatteryLevel::Critical => style::text_orange,
                BatteryLevel::Low => style::text_yellow,
                BatteryLevel::Medium | BatteryLevel::Full => style::text_green,
            },
        );
        let bottom = Column::new()
            .spacing(10)
            .push(
                slider(0.8..=1.2, scale, move |c| {
                    Message::JoyconScale(sn.clone(), c)
                })
                .step(0.001),
            )
            .push(text(format!("Rotation scale ratio: {scale:.3}")))
            .push(text("Change this if the tracker in vr moves less or more than your irl joycon. Higher value = more movement.").size(14))
            .push(Row::new().push(text("Battery level: ")).push(battery_text));

        Column::new().spacing(10).push(top).push(bottom)
    }
}
