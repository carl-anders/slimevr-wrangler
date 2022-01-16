use super::imu::JoyconAxisData;
use super::{ChannelInfo, JoyconData, JoyconDesign, JoyconDesignType, JoyconDeviceInfo};
use joycon_rs::prelude::*;
use std::sync::mpsc;

// Gyro: 2000dps
// Accel: 8G
// https://github.com/dekuNukem/Nintendo_Switch_Reverse_Engineering/blob/master/imu_sensor_notes.md

// Convert to acceleration in G
fn acc(n: i16) -> f64 {
    n as f64 * 0.00024414435f64 // 16000/65535/1000
}
// Convert to acceleration in radians/s
// TODO: add option for different numbers - or find the right magic
fn gyro(n: i16) -> f64 {
    n as f64
    * 0.07000839246f64 // 4588/65535 - degrees/s
    * (std::f64::consts::PI / 180.0f64) // radians/s
}

fn joycon_thread(standard: StandardFullMode<SimpleJoyConDriver>, tx: mpsc::Sender<ChannelInfo>) {
    let sn = standard.driver().joycon().serial_number().to_owned();
    loop {
        match standard.read_input_report() {
            Ok(report) => {
                if report.common.input_report_id == 48 {
                    let imu_data = report
                        .extra
                        .data
                        .iter()
                        .map(|data| JoyconAxisData {
                            accel_x: acc(data.accel_x),
                            accel_y: acc(data.accel_y),
                            accel_z: acc(data.accel_z),
                            gyro_x: gyro(data.gyro_1),
                            gyro_y: gyro(data.gyro_2),
                            gyro_z: gyro(data.gyro_3),
                        })
                        .collect::<Vec<_>>()
                        .as_slice()
                        .try_into()
                        .unwrap();
                    let data = JoyconData {
                        serial_number: sn.clone(),
                        //battery_level: report.common.battery.level,
                        imu_data,
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

pub fn spawn_thread(tx: mpsc::Sender<ChannelInfo>) {
    let manager = JoyConManager::get_instance();
    let devices = {
        let lock = manager.lock();
        match lock {
            Ok(manager) => manager.new_devices(),
            Err(_) => return,
        }
    };
    let _drop = devices.iter().try_for_each::<_, JoyConResult<()>>(|d| {
        let driver = SimpleJoyConDriver::new(&d)?;
        let joycon = driver.joycon();
        let color = joycon.color().clone();
        let info = JoyconDeviceInfo {
            serial_number: joycon.serial_number().to_owned(),
            design: JoyconDesign {
                color: format!(
                    "#{:02x}{:02x}{:02x}",
                    color.body[0], color.body[1], color.body[2]
                ),
                design_type: match joycon.device_type() {
                    JoyConDeviceType::JoyConL | JoyConDeviceType::ProCon => JoyconDesignType::Left,
                    JoyConDeviceType::JoyConR => JoyconDesignType::Right,
                },
            },
        };
        drop(joycon);
        let tx = tx.clone();
        tx.send(ChannelInfo::Connected(info)).unwrap();

        let standard = StandardFullMode::new(driver)?;
        std::thread::spawn(move || joycon_thread(standard, tx));

        Ok(())
    });
}
