use super::imu::JoyconAxisData;
use super::{ChannelInfo, JoyconData, JoyconDesign, JoyconDesignType, JoyconDeviceInfo};
use joycon_rs::joycon::device::calibration::imu::IMUCalibration;
use joycon_rs::joycon::lights::{LightUp, Lights};
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

fn joycon_thread(
    standard: StandardFullMode<SimpleJoyConDriver>,
    tx: mpsc::Sender<ChannelInfo>,
    calib: IMUCalibration,
) {
    let sn = standard.driver().joycon().serial_number().to_owned();
    let calib = match calib {
        IMUCalibration::Available {
            acc_origin_position: ao,
            gyro_origin_position: go,
            ..
        } => ([ao.x, ao.y, ao.z], [go.x, go.y, go.z]),
        IMUCalibration::Unavailable => ([0, 0, 0], [0, 0, 0]),
    };
    loop {
        match standard.read_input_report() {
            Ok(report) => {
                if report.common.input_report_id == 48 {
                    let imu_data = report
                        .extra
                        .data
                        .iter()
                        .map(|data| JoyconAxisData {
                            accel_x: acc(data.accel_x - calib.0[0]),
                            accel_y: acc(data.accel_y - calib.0[1]),
                            accel_z: acc(data.accel_z - calib.0[2]),
                            gyro_x: gyro(data.gyro_1 - calib.1[0]),
                            gyro_y: gyro(data.gyro_2 - calib.1[1]),
                            gyro_z: gyro(data.gyro_3 - calib.1[2]),
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
                println!("JoyCon disconnected");
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
        let mut driver = SimpleJoyConDriver::new(&d)?;
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

        let mut calib = joycon.imu_user_calibration().clone();
        if calib == IMUCalibration::Unavailable {
            calib = joycon.imu_factory_calibration().clone();
        }
        drop(joycon);

        let tx = tx.clone();
        tx.send(ChannelInfo::Connected(info)).unwrap();

        driver
            .set_player_lights(&[LightUp::LED0, LightUp::LED3], &[])
            .ok();

        let standard = StandardFullMode::new(driver)?;
        std::thread::spawn(move || joycon_thread(standard, tx, calib));

        Ok(())
    });
}
