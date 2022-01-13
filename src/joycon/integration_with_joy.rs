use super::communication::{ChannelInfo, JoyconData, JoyconDeviceInfo};
use super::imu::JoyconAxisData;
use super::{JoyconDesign, JoyconDesignType};
use joycon::joycon_sys::spi::ControllerColor;
use joycon::{
    hidapi::HidApi,
    joycon_sys::{light, HID_IDS, NINTENDO_VENDOR_ID},
    JoyCon,
};
use std::collections::HashSet;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

fn acc(n: f64) -> f64 {
    n * 9.82
}
fn gyro(n: f64) -> f64 {
    n * (std::f64::consts::PI / 180.0f64) // deg/s to rad/s
}

fn joycon_thread(sn: String, mut joycon: JoyCon, tx: mpsc::Sender<ChannelInfo>) {
    loop {
        match joycon.tick() {
            Ok(report) => match report.imu {
                Some(imu_data) => {
                    let imu_data = imu_data
                        .iter()
                        .map(|frame| JoyconAxisData {
                            accel_x: acc(frame.accel.x),
                            accel_y: acc(frame.accel.y),
                            accel_z: acc(frame.accel.z),
                            gyro_x: gyro(frame.gyro.x),
                            gyro_y: gyro(frame.gyro.y),
                            gyro_z: gyro(frame.gyro.z),
                        })
                        .collect::<Vec<_>>()
                        .as_slice()
                        .try_into()
                        .unwrap();

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

pub fn spawn_thread(tx: mpsc::Sender<ChannelInfo>) {
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
