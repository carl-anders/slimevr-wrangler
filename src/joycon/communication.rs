use std::{
    collections::HashMap,
    net::{SocketAddr, UdpSocket},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use deku::{DekuContainerRead, DekuContainerWrite};
use nalgebra::{UnitQuaternion, Vector3};

use crate::{settings, slime::deku::PacketType};

use super::{
    imu::{Imu, JoyconAxisData},
    JoyconDesign,
};

#[derive(Debug, Clone)]
pub struct JoyconStatus {
    pub connected: bool,
    pub rotation: (f64, f64, f64),
    pub design: JoyconDesign,
    pub mount_rotation: i32,
    pub serial_number: String,
}

#[derive(Debug, Clone)]
pub struct JoyconDeviceInfo {
    pub serial_number: String,
    pub design: JoyconDesign,
}

#[derive(Debug)]
struct Device {
    imu: Imu,
    design: JoyconDesign,
    id: u8,
}

impl Device {
    pub fn handshake(&self, socket: &UdpSocket, address: &SocketAddr) {
        let sensor_info = PacketType::SensorInfo {
            packet_id: 0,
            sensor_id: self.id,
            sensor_status: 1,
            sensor_type: 0,
        };
        socket
            .send_to(&sensor_info.to_bytes().unwrap(), address)
            .unwrap();
    }
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
/*
fn serial_number_to_mac(serial: &str) -> [u8; 6] {
    let mut hasher = Md5::new();
    hasher.update(serial);
    hasher.finalize()[0..6].try_into().unwrap()
}
*/

fn parse_message(
    msg: ChannelInfo,
    devices: &mut HashMap<String, Device>,
    socket: &UdpSocket,
    address: &SocketAddr,
    settings: &settings::WranglerSettings,
) {
    match msg {
        ChannelInfo::Connected(device_info) => {
            if devices.contains_key(&device_info.serial_number) {
                devices.get_mut(&device_info.serial_number).unwrap().imu = Imu::new();
                return;
            }
            let id = devices.len() as _;
            let device = Device {
                design: device_info.design,
                imu: Imu::new(),
                id,
            };
            device.handshake(socket, address);
            devices.insert(device_info.serial_number, device);
        }
        ChannelInfo::Data(data) => {
            if let Some(device) = devices.get_mut(&data.serial_number) {
                for frame in data.imu_data {
                    device.imu.update(frame);
                }

                let joycon_rotation = settings.joycon_rotation_get(&data.serial_number);
                let rotated_quat = if joycon_rotation > 0 {
                    device.imu.rotation
                        * UnitQuaternion::from_axis_angle(
                            &Vector3::z_axis(),
                            joycon_rotation as f64 * (std::f64::consts::TAU / 360.0),
                        )
                } else {
                    device.imu.rotation
                };

                let rotation_packet = PacketType::RotationData {
                    packet_id: 0,
                    sensor_id: device.id,
                    data_type: 1,
                    quat: (*rotated_quat).into(),
                    calibration_info: 0,
                };
                socket
                    .send_to(&rotation_packet.to_bytes().unwrap(), address)
                    .unwrap();
            }
        }
    }
}

fn slime_handshake(socket: &UdpSocket, address: &SocketAddr) {
    let handshake = PacketType::Handshake {
        packet_id: 0,
        board: 0,
        imu: 0,
        mcu_type: 0,
        imu_info: (0, 0, 0),
        build: 0,
        firmware: "slimevr-wrangler".to_string().into(),
        mac_address: [0x00, 0x0F, 0x00, 0x0F, 0x00, 0x0F],
    };
    socket
        .send_to(&handshake.to_bytes().unwrap(), address)
        .unwrap();
}

pub fn main_thread(
    receive: mpsc::Receiver<ChannelInfo>,
    output_tx: mpsc::Sender<Vec<JoyconStatus>>,
    settings: settings::Handler,
) {
    let mut devices: HashMap<String, Device> = HashMap::new();

    let addrs = [
        SocketAddr::from(([0, 0, 0, 0], 47589)),
        SocketAddr::from(([0, 0, 0, 0], 0)),
    ];
    let socket = UdpSocket::bind(&addrs[..]).unwrap();
    socket.set_nonblocking(true).ok();
    let address = {
        settings
            .load()
            .address
            .clone()
            .parse::<SocketAddr>()
            .unwrap_or_else(|_| "127.0.0.1:6969".parse().unwrap())
    };

    let mut connected = false;
    let mut last_handshake = Instant::now() - Duration::from_secs(60);
    let mut last_ping = Instant::now();
    let mut buf = [0; 512];

    loop {
        let settings = settings.load();
        if !connected && last_handshake.elapsed().as_secs() >= 3 {
            last_handshake = Instant::now();
            slime_handshake(&socket, &address);
            for device in devices.values() {
                device.handshake(&socket, &address);
            }
        }
        while let Ok(len) = socket.recv(&mut buf) {
            connected = true;
            if let Ok((_, PacketType::Ping { id: _ })) = PacketType::from_bytes((&buf, 0)) {
                last_ping = Instant::now();
                socket.send_to(&buf[0..len], address).unwrap();
            }
        }
        if connected && last_ping.elapsed().as_secs() >= 3 {
            connected = false;
        }

        let mut received_message = false;
        for msg in receive.try_iter() {
            received_message = true;
            parse_message(msg, &mut devices, &socket, &address, &settings);
        }

        if received_message {
            let mut statuses = Vec::new();
            for (serial_number, device) in &devices {
                statuses.push(JoyconStatus {
                    connected: true,
                    rotation: device.imu.euler_angles_deg(),
                    design: device.design.clone(),
                    mount_rotation: settings.joycon_rotation_get(serial_number),
                    serial_number: serial_number.clone(),
                });
            }
            let _drop = output_tx.send(statuses);
        } else {
            thread::sleep(Duration::from_micros(500));
        }
    }
}
