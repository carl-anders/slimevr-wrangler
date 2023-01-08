use std::{
    collections::HashMap,
    net::{SocketAddr, UdpSocket},
    sync::mpsc,
    time::{Duration, Instant},
};

use itertools::Itertools;
use joycon_rs::prelude::input_report_mode::BatteryLevel;
use nalgebra::{UnitQuaternion, Vector3};
use protocol::deku::{DekuContainerRead, DekuContainerWrite};
use protocol::PacketType;

use super::{
    imu::{Imu, JoyconAxisData},
    JoyconDesign,
};
use crate::settings::{self, WranglerSettings};

#[derive(Debug, Clone)]
pub struct Status {
    pub rotation: (f64, f64, f64),
    pub design: JoyconDesign,
    pub mount_rotation: i32,
    pub serial_number: String,
    pub battery_level: BatteryLevel,
}

struct Device {
    imu: Imu,
    design: JoyconDesign,
    id: u8,
    battery_level: BatteryLevel,
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
pub struct ChannelData {
    pub serial_number: String,
    pub info: ChannelInfo,
}

#[derive(Debug, Clone)]
pub enum ChannelInfo {
    Connected(JoyconDesign),
    ImuData([JoyconAxisData; 3]),
    BatteryLevel(BatteryLevel),
    Reset,
}
/*
fn serial_number_to_mac(serial: &str) -> [u8; 6] {
    let mut hasher = Md5::new();
    hasher.update(serial);
    hasher.finalize()[0..6].try_into().unwrap()
}
*/

#[derive(Debug, Copy, Clone)]
struct Xyz {
    x: f64,
    y: f64,
    z: f64,
}

fn calc_acceleration(
    rotation: UnitQuaternion<f64>,
    axisdata: &JoyconAxisData,
    rad_rotation: f64,
) -> Xyz {
    let a = rotation.coords;
    let (x, y, z, w) = (a.x, a.y, a.z, a.w);
    let gravity = [
        2.0 * ((-x) * (-z) - w * y),
        -2.0 * (w * (-x) + y * (-z)),
        w * w - x * x - y * y + z * z,
    ];
    let vector = Xyz {
        x: axisdata.accel_x - gravity[0],
        y: axisdata.accel_y - gravity[1],
        z: axisdata.accel_z - gravity[2],
    };

    let rad_rotation = -rad_rotation;
    Xyz {
        x: vector.x * rad_rotation.cos() - vector.y * rad_rotation.sin(),
        y: vector.x * rad_rotation.sin() + vector.y * rad_rotation.cos(),
        z: vector.z,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ServerStatus {
    #[default]
    Disconnected,
    Unknown,
    Connected,
}

pub struct Communication {
    receive: mpsc::Receiver<ChannelData>,
    status_tx: mpsc::Sender<Vec<Status>>,
    server_tx: mpsc::Sender<ServerStatus>,
    settings: settings::Handler,

    devices: HashMap<String, Device>,

    socket: UdpSocket,
    address: SocketAddr,
    connected: ServerStatus,
    last_handshake: Instant,
    last_ping: Instant,
    last_reset: Instant,
}
impl Communication {
    pub fn start(
        receive: mpsc::Receiver<ChannelData>,
        status_tx: mpsc::Sender<Vec<Status>>,
        server_tx: mpsc::Sender<ServerStatus>,
        settings: settings::Handler,
    ) {
        let addrs = [
            SocketAddr::from(([0, 0, 0, 0], 47589)),
            SocketAddr::from(([0, 0, 0, 0], 0)),
        ];
        let socket = UdpSocket::bind(&addrs[..]).unwrap();
        socket.set_nonblocking(true).ok();
        let address = { settings.load().get_socket_address() };

        server_tx.send(ServerStatus::Disconnected).ok();

        Self {
            receive,
            status_tx,
            server_tx,
            settings,
            devices: HashMap::new(),
            socket,
            address,
            connected: ServerStatus::Disconnected,
            last_handshake: Instant::now().checked_sub(Duration::from_secs(60)).unwrap(),
            last_ping: Instant::now(),
            last_reset: Instant::now(),
        }
        .main_loop();
    }

    fn send_handshake(&self) {
        let handshake = PacketType::Handshake {
            packet_id: 0,
            board: 0,
            imu: 0,
            mcu_type: 0,
            imu_info: (0, 0, 0),
            build: 9,
            firmware: "slimevr-wrangler".to_string().into(),
            mac_address: [0x00, 0x0F, 0x00, 0x0F, 0x00, 0x0F],
        };
        self.socket
            .send_to(&handshake.to_bytes().unwrap(), self.address)
            .unwrap();
    }

    fn send_reset(&self) {
        let handshake = PacketType::UserAction {
            packet_id: 0,
            typ: 3,
        };
        self.socket
            .send_to(&handshake.to_bytes().unwrap(), self.address)
            .unwrap();
    }

    fn parse_message(&mut self, msg: ChannelData, settings: &WranglerSettings) {
        let sn = msg.serial_number;
        match msg.info {
            ChannelInfo::Connected(design) => {
                if self.devices.contains_key(&sn) {
                    let device = self.devices.get_mut(&sn).unwrap();
                    device.imu = Imu::new();
                    device.battery_level = BatteryLevel::Full;
                    return;
                }
                let id = self.devices.len() as _;
                let device = Device {
                    design,
                    imu: Imu::new(),
                    id,
                    battery_level: BatteryLevel::Full,
                };
                device.handshake(&self.socket, &self.address);
                self.devices.insert(sn, device);
            }
            ChannelInfo::ImuData(imu_data) => {
                if let Some(device) = self.devices.get_mut(&sn) {
                    for frame in imu_data {
                        device.imu.update(frame);
                    }

                    let joycon_rotation = settings.joycon_rotation_get(&sn);
                    let rad_rotation = (joycon_rotation as f64).to_radians();
                    let rotated_quat = if joycon_rotation > 0 {
                        device.imu.rotation
                            * UnitQuaternion::from_axis_angle(&Vector3::z_axis(), rad_rotation)
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
                    self.socket
                        .send_to(&rotation_packet.to_bytes().unwrap(), self.address)
                        .unwrap();

                    let acc = calc_acceleration(device.imu.rotation, &imu_data[2], rad_rotation);
                    /* if std::env::args().any(|a| &a == "debug") {
                        if acc.x.abs() > 3.0 || acc.y.abs() > 3.0 || acc.z.abs() > 3.0 {
                            println!("x: {:.3}, y: {:.3}, z: {:.3}", acc.x, acc.y, acc.z);
                        }
                    } */

                    let acceleration_packet = PacketType::Acceleration {
                        packet_id: 0,
                        vector: (acc.x as f32, acc.y as f32, acc.z as f32),
                        sensor_id: Some(device.id),
                    };
                    self.socket
                        .send_to(&acceleration_packet.to_bytes().unwrap(), self.address)
                        .unwrap();
                }
            }
            ChannelInfo::BatteryLevel(battery_level) => {
                if let Some(device) = self.devices.get_mut(&sn) {
                    device.battery_level = battery_level;
                }
            }
            ChannelInfo::Reset => {
                if settings.send_reset && self.last_reset.elapsed().as_secs() >= 2 {
                    self.last_reset = Instant::now();
                    self.send_reset();
                }
            }
        }
    }

    pub fn main_loop(&mut self) {
        let mut buf = [0; 512];

        // Spin sleeper with 1ns accuracy = don't actually spin sleep much, but set minimum windows timeout to 1ms instead of 15ms
        let light_sleeper = spin_sleep::SpinSleeper::new(1)
            .with_spin_strategy(spin_sleep::SpinStrategy::YieldThread);

        let mut last_ui_send = Instant::now();

        loop {
            let settings = self.settings.load();
            if self.connected != ServerStatus::Connected
                && self.last_handshake.elapsed().as_secs() >= 3
            {
                self.last_handshake = Instant::now();
                self.send_handshake();
                for device in self.devices.values().sorted_by_key(|d| d.id) {
                    device.handshake(&self.socket, &self.address);
                }
            }
            while let Ok(len) = self.socket.recv(&mut buf) {
                if self.connected == ServerStatus::Disconnected {
                    self.connected = ServerStatus::Unknown;
                    self.server_tx.send(self.connected).ok();
                }
                let b = PacketType::from_bytes((&buf, 0));
                match b {
                    Ok((_, PacketType::Ping { id: _ })) => {
                        self.last_ping = Instant::now();
                        self.socket.send_to(&buf[0..len], self.address).unwrap();
                    }
                    Ok((_, PacketType::HandshakeResponse)) => {
                        self.connected = ServerStatus::Connected;
                        self.server_tx.send(self.connected).ok();
                    }
                    _ => {}
                }
            }
            if self.connected != ServerStatus::Disconnected
                && self.last_ping.elapsed().as_secs() >= 3
            {
                self.connected = ServerStatus::Disconnected;
                self.server_tx.send(self.connected).ok();
            }

            let messages: Vec<_> = self.receive.try_iter().collect();
            if !messages.is_empty() || last_ui_send.elapsed().as_millis() > 100 {
                for msg in messages {
                    self.parse_message(msg, &settings);
                }

                last_ui_send = Instant::now();
                let mut statuses = Vec::new();
                for (serial_number, device) in &self.devices {
                    statuses.push(Status {
                        rotation: device.imu.euler_angles_deg(),
                        design: device.design.clone(),
                        mount_rotation: settings.joycon_rotation_get(serial_number),
                        serial_number: serial_number.clone(),
                        battery_level: device.battery_level,
                    });
                }
                self.status_tx.send(statuses).ok();
            } else {
                light_sleeper.sleep(Duration::from_millis(2));
            }
        }
    }
}
