use std::{
    collections::HashMap,
    net::{SocketAddr, UdpSocket},
    sync::mpsc,
    thread,
    time::Duration,
};

use deku::DekuContainerWrite;
use md5::{Digest, Md5};

use crate::slime::deku::PacketType;

use super::{
    imu::{Imu, JoyconAxisData},
    JoyconDesign,
};

#[derive(Debug, Clone)]
pub struct JoyconStatus {
    pub connected: bool,
    pub rotation: (f64, f64, f64),
    pub design: JoyconDesign,
}

#[derive(Debug, Clone)]
pub struct JoyconDeviceInfo {
    pub serial_number: String,
    pub design: JoyconDesign,
}

#[derive(Debug)]
struct Device {
    imu: Imu,
    socket: UdpSocket,
    design: JoyconDesign,
}

#[derive(Debug, Clone)]
pub struct JoyconData {
    pub serial_number: String,
    pub imu_data: [JoyconAxisData; 3],
}

#[derive(Debug, Clone)]
pub enum ChannelInfo {
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
                    design: device_info.design,
                    imu: Imu::new(),
                    socket,
                },
            );
        }
        ChannelInfo::Data(data) => match devices.get_mut(&data.serial_number) {
            Some(device) => {
                for frame in data.imu_data {
                    device.imu.update(frame)
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

pub fn main_thread(
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
