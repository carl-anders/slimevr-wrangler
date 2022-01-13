use crate::slime::deku::PacketType;

use super::imu::{Imu, JoyconAxisData};
use super::{JoyconDesign, JoyconDesignType};
use deku::DekuContainerWrite;
use joycon::joycon_sys::spi::ControllerColor;
use joycon::{
    hidapi::HidApi,
    joycon_sys::{light, HID_IDS, NINTENDO_VENDOR_ID},
    JoyCon, IMU,
};
use md5::{Digest, Md5};
use std::collections::{HashMap, HashSet};
use std::net::{SocketAddr, UdpSocket};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct JoyconStatus {
    pub connected: bool,
    pub rotation: (f64, f64, f64),
    pub design: JoyconDesign,
}

#[derive(Debug, Clone)]
struct JoyconDeviceInfo {
    serial_number: String,
    design: JoyconDesign,
}

#[derive(Debug)]
struct Device {
    imu: Imu,
    socket: UdpSocket,
    design: JoyconDesign,
}

#[derive(Debug, Clone)]
struct JoyconData {
    serial_number: String,
    imu_data: [IMU; 3],
}

#[derive(Debug, Clone)]
enum ChannelInfo {
    Connected(JoyconDeviceInfo),
    Data(JoyconData),
}
fn serial_number_to_mac(serial: &str) -> [u8; 6] {
    let mut hasher = Md5::new();
    hasher.update(serial);
    hasher.finalize()[0..6].try_into().unwrap()
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JoyconAxisDataRawFloat {
    pub accel_x: f64,
    pub accel_y: f64,
    pub accel_z: f64,
    pub gyro_x: f64,
    pub gyro_y: f64,
    pub gyro_z: f64,
}

fn acc_f(n: f64) -> f64 {
    n * 9.82
}
fn gyro_f(n: f64) -> f64 {
    n * (std::f64::consts::PI / 180.0f64) // deg/s to rad/s
}
impl From<JoyconAxisDataRawFloat> for JoyconAxisData {
    fn from(item: JoyconAxisDataRawFloat) -> Self {
        Self {
            accel_x: acc_f(item.accel_x),
            accel_y: acc_f(item.accel_y),
            accel_z: acc_f(item.accel_z),
            gyro_x: gyro_f(item.gyro_x),
            gyro_y: gyro_f(item.gyro_y),
            gyro_z: gyro_f(item.gyro_z),
        }
    }
}

fn parse_message(msg: ChannelInfo, devices: &mut HashMap<String, Device>, address: &str) {
    let address = address
        .parse::<SocketAddr>()
        .unwrap_or_else(|_| "127.0.0.1:6969".parse().unwrap());
    match msg {
        ChannelInfo::Connected(device_info) => {
            let serial = device_info.serial_number.clone();
            let handshake = PacketType::Handshake {
                packet_id: 1,
                board: 0,
                imu: 0,
                mcu_type: 0,
                imu_info: (0, 0, 0),
                build: 0,
                firmware: "slimevr-wrangler".to_string().into(),
                mac_address: serial_number_to_mac(&serial),
            };
            let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
            socket
                .send_to(&handshake.to_bytes().unwrap(), address)
                .unwrap();
            devices.insert(
                serial,
                Device {
                    design: device_info.design,
                    imu: Imu::new(),
                    socket,
                },
            );
        }
        ChannelInfo::Data(data) => match devices.get_mut(&data.serial_number) {
            Some(device) => {
                for frame in data.imu_data {
                    device.imu.update(JoyconAxisDataRawFloat {
                        accel_x: frame.accel.x,
                        accel_y: frame.accel.y,
                        accel_z: frame.accel.z,
                        gyro_x: frame.gyro.x,
                        gyro_y: frame.gyro.y,
                        gyro_z: frame.gyro.z,
                    }.into())
                }

                let rotation = PacketType::Rotation {
                    packet_id: 1,
                    quat: (*device.imu.rotation).into(),
                };

                device
                    .socket
                    .send_to(&rotation.to_bytes().unwrap(), address)
                    .unwrap();
            }
            None => (),
        },
    }
}

fn main_thread(
    receive: mpsc::Receiver<ChannelInfo>,
    output_tx: mpsc::Sender<Vec<JoyconStatus>>,
    address: String,
) {
    let mut devices = HashMap::new();
    loop {
        let mut got_message = false;
        for _ in 0..2 {
            for msg in receive.try_iter() {
                got_message = true;
                parse_message(msg, &mut devices, &address);
            }
            if got_message {
                break;
            } else {
                thread::sleep(Duration::from_millis(2));
            }
        }

        if got_message {
            let mut statuses = Vec::new();
            for device in devices.values() {
                statuses.push(JoyconStatus {
                    connected: true,
                    rotation: device.imu.euler_angles_deg(),
                    design: device.design.clone(),
                });
            }
            let _ = output_tx.send(statuses);
        }
    }
}

fn joycon_thread(sn: String, mut joycon: JoyCon, tx: mpsc::Sender<ChannelInfo>) {
    loop {
        match joycon.tick() {
            Ok(report) => match report.imu {
                Some(imu_data) => {
                    tx.send(ChannelInfo::Data(JoyconData {
                        serial_number: sn.clone(),
                        imu_data,
                    }))
                    .unwrap();
                }
                None => {
                    println!("No IMU data");
                }
            },
            Err(e) => {
                println!("Tick error: {}", e);
            }
        }
    }
}

fn spawn_thread(tx: mpsc::Sender<ChannelInfo>) {
    let mut api = HidApi::new().unwrap();
    let mut connected_controllers: HashSet<String> = HashSet::new();
    loop {
        let _ = api.refresh_devices();
        thread::sleep(Duration::from_secs(1));
        for d in api.device_list() {
            if d.vendor_id() == NINTENDO_VENDOR_ID {
                println!("{:?}", d);
            }
        }
        let devices: Vec<_> = api
            .device_list()
            .filter(|d| d.vendor_id() == NINTENDO_VENDOR_ID && HID_IDS.contains(&d.product_id()))
            .filter(|d| match d.serial_number() {
                Some(sn) => !connected_controllers.contains(sn),
                None => false,
            })
            .collect();
        for device_info in devices {
            let device = match device_info.open_device(&api) {
                Ok(device) => device,
                Err(e) => {
                    println!("Could not open device: {}", e);
                    continue;
                }
            };
            let mut joycon = match JoyCon::new(device, device_info.clone()) {
                Ok(j) => j,
                Err(e) => {
                    println!("Error opening joycon: {}", e);
                    continue;
                }
            };
            match joycon.enable_imu() {
                Ok(_) => {}
                Err(e) => {
                    println!("Error enabling IMU: {}", e);
                    continue;
                }
            }
            match joycon.load_calibration() {
                Ok(_) => {}
                Err(e) => {
                    println!("Error loading calibration: {}", e);
                    continue;
                }
            }
            match joycon.set_player_light(light::PlayerLights::new(
                light::PlayerLight::On,
                light::PlayerLight::Off,
                light::PlayerLight::Off,
                light::PlayerLight::On,
            )) {
                Ok(_) => {}
                Err(e) => {
                    println!("Error setting lights: {}", e);
                }
            }

            let color: Result<ControllerColor, _> = joycon.read_spi();
            let body_color = match color {
                Ok(c) => format!("{}", c.body),
                Err(e) => {
                    println!("Error loading color: {}", e);
                    "#808080".to_string()
                }
            };

            let sn = device_info.serial_number().unwrap().to_string();

            connected_controllers.insert(sn.clone());

            let info = JoyconDeviceInfo {
                serial_number: sn.clone(),
                design: JoyconDesign {
                    color: body_color,
                    design_type: if joycon.supports_ir() {
                        JoyconDesignType::RIGHT
                    } else {
                        JoyconDesignType::LEFT
                    },
                },
            };
            let tx = tx.clone();
            tx.send(ChannelInfo::Connected(info)).unwrap();
            std::thread::spawn(move || joycon_thread(sn, joycon, tx));
        }
    }
}

fn startup(address: String) -> mpsc::Receiver<Vec<JoyconStatus>> {
    let (out_tx, out_rx) = mpsc::channel();
    let (tx, rx) = mpsc::channel();
    let _ = std::thread::spawn(move || main_thread(rx, out_tx, address));
    std::thread::spawn(move || spawn_thread(tx));
    out_rx
}

pub struct JoyconIntegration {
    rx: mpsc::Receiver<Vec<JoyconStatus>>,
}
impl JoyconIntegration {
    pub fn new(address: String) -> Self {
        Self {
            rx: startup(address),
        }
    }
    pub fn poll(&self) -> Option<Vec<JoyconStatus>> {
        self.rx.try_iter().last()
    }
}
