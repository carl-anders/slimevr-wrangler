use crate::slime::deku::PacketType;

use super::imu::Imu;
use super::JoyconDesign;
use deku::DekuContainerWrite;
use joycon_rs::joycon::input_report_mode::standard_full_mode::IMUData;
use joycon_rs::joycon::input_report_mode::BatteryLevel;
use joycon_rs::prelude::*;
use md5::{Digest, Md5};
use std::collections::HashMap;
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
    device_type: JoyConDeviceType,
    color: device::color::Color,
}

#[derive(Debug)]
struct Device {
    battery_level: BatteryLevel,
    imu: Imu,
    socket: UdpSocket,
    design: JoyconDesign,
}

#[derive(Debug, Clone)]
struct JoyconData {
    serial_number: String,
    battery_level: BatteryLevel,
    axis_data: IMUData,
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
                    design: JoyconDesign {
                        colour: device_info.color.body,
                        design_type: device_info.device_type,
                    },
                    battery_level: BatteryLevel::Empty,
                    imu: Imu::new(),
                    socket,
                },
            );
        }
        ChannelInfo::Data(data) => match devices.get_mut(&data.serial_number) {
            Some(device) => {
                device.imu.update(data.axis_data);
                device.battery_level = data.battery_level;

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

fn joycon_thread(standard: StandardFullMode<SimpleJoyConDriver>, tx: mpsc::Sender<ChannelInfo>) {
    let sn = standard.driver().joycon().serial_number().to_owned();
    loop {
        match standard.read_input_report() {
            Ok(report) => {
                if report.common.input_report_id == 48 {
                    let data = JoyconData {
                        serial_number: sn.clone(),
                        battery_level: report.common.battery.level,
                        axis_data: report.extra,
                    };
                    tx.send(ChannelInfo::Data(data)).unwrap();
                }
            }
            Err(JoyConError::Disconnected) => {
                println!("Disconnected yo!");
                return;
            }
            _ => {}
        }
    }
}

fn spawn_thread(tx: mpsc::Sender<ChannelInfo>) {
    let manager = JoyConManager::get_instance();
    let devices = {
        let lock = manager.lock();
        match lock {
            Ok(manager) => manager.new_devices(),
            Err(_) => return,
        }
    };
    let _ = devices.iter().try_for_each::<_, JoyConResult<()>>(|d| {
        let driver = SimpleJoyConDriver::new(&d)?;
        let joycon = driver.joycon();
        let info = JoyconDeviceInfo {
            serial_number: joycon.serial_number().to_owned(),
            device_type: joycon.device_type(),
            color: joycon.color().clone(),
        };
        drop(joycon);
        let tx = tx.clone();
        tx.send(ChannelInfo::Connected(info)).unwrap();

        let standard = StandardFullMode::new(driver)?;
        std::thread::spawn(move || joycon_thread(standard, tx));

        Ok(())
    });
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
