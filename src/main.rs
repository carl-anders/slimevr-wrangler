#![deny(clippy::all)]

use iced::{
    time,
    widget::{
        button, checkbox, container, horizontal_space, scrollable, slider, text, text_input,
        Column, Container, Row, Scrollable, Space, Svg,
    },
    window, Alignment, Color, Element, Font, Length, Size, Subscription, Task as Command,
};

use circle::circle;
use iced_aw::Wrap;
use joycon::{Battery, DeviceStatus, ServerStatus};
use needle::Needle;
use settings::WranglerSettings;
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
mod circle;
mod needle;
mod settings;
mod style;
mod update;

const WINDOW_SIZE: Size = Size {
    width: 980.0,
    height: 700.0,
};

pub const ICONS: &[u8] = include_bytes!("../assets/icons.ttf");
pub const ICON: &[u8; 16384] = include_bytes!("../assets/icon_64.rgba8");

pub fn main() -> iced::Result {
    /*
    let rgba8 = image_rs::io::Reader::open("assets/icon.png").unwrap().decode().unwrap().to_rgba8();
    std::fs::write("assets/icon_64.rgba8", rgba8.into_raw());
    */
    let window_settings = window::Settings {
        size: WINDOW_SIZE,
        min_size: Some(WINDOW_SIZE),
        icon: window::icon::from_rgba(ICON.to_vec(), 64, 64).ok(),
        ..window::Settings::default()
    };
    let run = iced::application("SlimeVR Wrangler", MainState::update, MainState::view)
        .subscription(MainState::subscription)
        .window(window_settings)
        .antialiasing(true)
        .font(ICONS)
        .run_with(MainState::new);
    match run {
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
    SettingsResetToggled(bool),
    SettingsIdsToggled(bool),
}

#[derive(Default)]
struct MainState {
    joycon: Option<joycon::Wrapper>,
    joycon_boxes: JoyconBoxes,
    search_dots: usize,
    settings_show: bool,
    server_connected: ServerStatus,
    server_address: String,

    settings: settings::Handler,
    update_found: Option<String>,
    blacklist_info: blacklist::BlacklistResult,
}
impl MainState {
    fn new() -> (Self, Command<Message>) {
        let mut new = Self::default();
        new.joycon = Some(joycon::Wrapper::new(new.settings.clone()));
        new.server_address = format!("{}", new.settings.load().get_socket_address());
        (
            new,
            Command::batch(vec![
                Command::perform(update::check_updates(), Message::UpdateFound),
                Command::perform(blacklist::check_blacklist(), Message::BlacklistChecked),
            ]),
        )
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::SettingsPressed => {
                self.settings_show = !self.settings_show;
            }
            Message::Tick(_time) => {
                if let Some(ref ji) = self.joycon {
                    if let Some(res) = ji.poll_status() {
                        self.joycon_boxes.statuses = res;
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
            Message::SettingsResetToggled(new) => {
                self.settings.change(|ws| ws.send_reset = new);
            }
            Message::SettingsIdsToggled(new) => {
                self.settings.change(|ws| ws.keep_ids = new);
            }
        }
        Command::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(vec![
            time::every(Duration::from_millis(500)).map(Message::Dot),
            time::every(Duration::from_millis(50)).map(Message::Tick),
        ])
    }

    fn view(&self) -> Element<Message> {
        let mut app = Column::new().push(top_bar(self.update_found.clone()));

        if self.blacklist_info.visible() {
            app = app.push(blacklist_bar(&self.blacklist_info));
        }

        app.push(
            if self.settings_show {
                container(self.settings_screen()).padding(20)
            } else {
                container(self.joycon_screen())
            }
            .width(Length::Fill)
            .height(Length::Fill)
            .style(style::container_darker),
        )
        .push(bottom_bar(
            self.server_connected,
            &".".repeat(self.search_dots),
            &self.server_address,
        ))
        .into()
    }
    fn joycon_screen(&self) -> Scrollable<'_, Message> {
        let boxes = self.joycon_boxes.view(&self.settings.load());
        let grid = boxes.into_iter().fold(Wrap::new(), |wrap, bax| {
            wrap.push(container(bax).padding(10))
        });
        let list = Column::new().padding(10).width(Length::Fill).push(grid);

        let list = list.push(
            container(text(format!(
                "Searching for Joycon controllers{}\n\
                    Please pair controllers in the bluetooth \
                    settings of Windows if they don't show up here.",
                ".".repeat(self.search_dots)
            )))
            .padding(10),
        );
        scrollable(list).height(Length::Fill)
    }
    fn settings_screen(&self) -> Column<'_, Message> {
        Column::new()
            .spacing(20)
            .push(address(&self.settings.load().address))
            .push(checkbox(
                "Send yaw reset command to SlimeVR Server after B or UP button press.",
                self.settings.load().send_reset).on_toggle(
                Message::SettingsResetToggled)
            )
            .push(checkbox(
                "Save mounting location on server. Requires SlimeVR Server v0.6.1 or newer. Restart Wrangler after changing this.",
                self.settings.load().keep_ids).on_toggle(
                Message::SettingsIdsToggled,
            ))
    }
}

fn address<'a>(input_value: &str) -> Column<'a, Message> {
    let address = text_input("127.0.0.1:6969", input_value)
        .on_input(Message::AddressChange)
        .width(Length::Fixed(300.0))
        .padding(10);

    let address_row = Row::new()
        .spacing(10)
        .align_y(Alignment::Center)
        .push("SlimeVR Server address:")
        .push(address)
        .push("Restart Wrangler after changing this.");
    let mut allc = Column::new().push(address_row).spacing(10);

    if input_value.parse::<SocketAddr>().is_err() {
        allc = allc.push(
            container(text(
                "Address is not a valid ip with port number! Using default instead (127.0.0.1:6969).",
            ))
            .style(style::text_yellow),
        );
    }
    allc
}
fn top_bar<'a>(update: Option<String>) -> Container<'a, Message> {
    let mut top_column = Row::new()
        .align_y(Alignment::Center)
        .push(text("SlimeVR Wrangler").size(24));

    if let Some(u) = update {
        let update_btn = button(text("Update"))
            .style(style::button_primary)
            .on_press(Message::UpdatePressed);
        top_column = top_column
            .push(Space::with_width(Length::Fixed(20.0)))
            .push(text(format!("New update found! Version: {u}. ")))
            .push(update_btn);
    }

    let settings = button(text("Settings"))
        .style(style::button_settings)
        .on_press(Message::SettingsPressed);
    top_column = top_column.push(horizontal_space()).push(settings);

    container(top_column)
        .width(Length::Fill)
        .padding(20)
        .style(style::container_highlight)
}

fn blacklist_bar<'a>(result: &blacklist::BlacklistResult) -> Container<'a, Message> {
    let mut row = Row::new()
        .align_y(Alignment::Center)
        .push(text(result.info.clone()))
        .push(Space::with_width(Length::Fixed(20.0)));
    if result.fix_button {
        row = row.push(
            button(text("Fix blacklist"))
                .style(style::button_primary)
                .on_press(Message::BlacklistFixPressed),
        );
    }
    container(row)
        .width(Length::Fill)
        .padding(20)
        .style(style::container_info)
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
        .style(style::container_info)
}

#[derive(Debug)]
struct JoyconBoxes {
    pub statuses: Vec<joycon::Status>,
    svg_handler: joycon::Svg,
    needle_handler: Needle,
}

impl Default for JoyconBoxes {
    fn default() -> Self {
        Self {
            statuses: vec![],
            svg_handler: joycon::Svg::new(),
            needle_handler: Needle::new(),
        }
    }
}

impl JoyconBoxes {
    fn view<'a>(&'a self, settings: &WranglerSettings) -> Vec<Container<'a, Message>> {
        self.statuses
            .iter()
            .map(|status| {
                container(single_box_view(
                    status,
                    &self.svg_handler,
                    &self.needle_handler,
                    settings.joycon_scale_get(&status.serial_number),
                    settings.joycon_rotation_get(&status.serial_number),
                ))
                .height(Length::Fixed(335.0))
                .width(Length::Fixed(300.0))
                .padding(10)
                .style(style::item_normal)
            })
            .collect()
    }
}

fn single_box_view<'a>(
    status: &joycon::Status,
    svg_handler: &joycon::Svg,
    needle_handler: &Needle,
    scale: f64,
    mount_rot: i32,
) -> Column<'a, Message> {
    let sn = status.serial_number.clone();

    let buttons = Row::new()
        .spacing(10)
        .push(
            button(text("↺").font(Font::with_name("fontello")))
                .on_press(Message::JoyconRotate(sn.clone(), false))
                .style(style::button_primary),
        )
        .push(
            button(text("↻").font(Font::with_name("fontello")))
                .on_press(Message::JoyconRotate(sn.clone(), true))
                .style(style::button_primary),
        );

    let svg = Svg::new(svg_handler.get(&status.design, mount_rot));

    let left = Column::new()
        .spacing(10)
        .align_x(Alignment::Center)
        .push(buttons)
        .push(svg)
        .width(Length::Fixed(130.0));

    let rot = status.rotation;
    let values = Row::with_children(
        [("Roll", rot.0), ("Pitch", rot.1), ("Yaw", -rot.2)]
            .iter()
            .map(|(name, val)| {
                let ival = (*val as i32).rem_euclid(360);

                Column::new()
                    .push(text(name.to_string()))
                    .push(
                        Svg::new(needle_handler.get(ival))
                            .width(Length::Fixed(25.0))
                            .height(Length::Fixed(25.0)),
                    )
                    .push(text(format!("{ival}")))
                    .spacing(10)
                    .align_x(Alignment::Center)
                    .width(Length::Fill)
                    .into()
            })
            .collect::<Vec<_>>(),
    );

    let circle = circle(
        8.0,
        match status.status {
            DeviceStatus::Disconnected | DeviceStatus::NoIMU => Color::from_rgb8(0xff, 0x38, 0x4A),
            DeviceStatus::LaggyIMU => Color::from_rgb8(0xff, 0xe3, 0x3c),
            DeviceStatus::Healthy => Color::from_rgb8(0x3d, 0xff, 0x81),
        },
    );
    let circle_cont = container(circle)
        .width(Length::Fixed(16.0))
        .height(Length::Fixed(16.0));

    let top = Row::new()
        .spacing(5)
        .push(circle_cont)
        .push(left)
        .push(values)
        .height(Length::Fixed(150.0));

    let battery_text =
        container(text(format!("{:?}", status.battery))).style(match status.battery {
            Battery::Empty | Battery::Critical => style::text_orange,
            Battery::Low => style::text_yellow,
            Battery::Medium | Battery::Full => style::text_green,
        });

    let status_text = container(text(format!("{}", status.status))).style(match status.status {
        DeviceStatus::Disconnected | DeviceStatus::NoIMU => style::text_orange,
        DeviceStatus::LaggyIMU => style::text_yellow,
        DeviceStatus::Healthy => style::text_green,
    });

    let bottom = Column::new()
        .spacing(10)
        .push(
            slider(0.8..=1.2, scale, move |c| {
                Message::JoyconScale(sn.clone(), c)
            })
            .step(0.001),
        )
        .push(text(format!("Rotation scale ratio: {scale:.3}")))
        .push(
            text(
                "Change this if the tracker in vr moves less or more than your irl joycon. Higher value = more movement.",
            )
            .size(14),
        )
        .push(Row::new().push(text("Battery level: ")).push(battery_text))
        .push(Row::new().push(text("Status: ")).push(status_text));

    Column::new().spacing(10).push(top).push(bottom)
}
