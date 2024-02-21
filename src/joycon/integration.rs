use super::communication::ChannelData;
use super::imu::{JoyconAxisData, JoyconQuatData};
use super::{Battery, ChannelInfo, JoyconDesign, JoyconDesignType};
use crate::settings;
use joycon_rs::joycon::device::calibration::imu::IMUCalibration;
use joycon_rs::joycon::lights::{LightUp, Lights};
use joycon_rs::prelude::input_report_mode::{BatteryLevel, StandardInputReport};
use joycon_rs::prelude::joycon_features::{JoyConFeature, IMUConfig};
use joycon_rs::prelude::*;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;
use static_slicing::StaticRangeIndex;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Accel {
    pub accel_x: i16,
    pub accel_y: i16,
    pub accel_z: i16,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct AxisData {
    pub accel: Accel,
    pub rotation_fragment: [u8; 6],
}

impl From<[u8; 6]> for Accel {
    fn from(value: [u8; 6]) -> Self {
        let accel_x = i16::from_le_bytes([value[0], value[1]]);
        let accel_y = i16::from_le_bytes([value[2], value[3]]);
        let accel_z = i16::from_le_bytes([value[4], value[5]]);

        Accel {
            accel_x,
            accel_y,
            accel_z,
        }
    }
}

impl From<[u8; 12]> for AxisData {
    fn from(value: [u8; 12]) -> Self {
        
        let accel = Accel::from(value[StaticRangeIndex::<0, 6>]);
        let rotation_fragment = value[StaticRangeIndex::<6, 6>];

        AxisData {
            accel,
            rotation_fragment
        }
    }
}

#[derive(Debug, Clone)]
struct IMUData {
    pub data: [(Accel, joycon_quat::Quaternion); 3],
}

impl TryFrom<[u8; 349]> for IMUData {
    type Error = JoyConError;
    fn try_from(value: [u8; 349]) -> Result<Self, JoyConError> {
        let latest = AxisData::from(value[StaticRangeIndex::<0, 12>]);
        let a_5ms_older = AxisData::from(value[StaticRangeIndex::<12, 12>]);
        let a_10ms_older = AxisData::from(value[StaticRangeIndex::<24, 12>]);

        let rotation_data = {
            let mut concat = [0u8; 18];
            let (first, rest) = concat.split_at_mut(6);
            let (second, third) = rest.split_at_mut(6);
            first.copy_from_slice(&latest.rotation_fragment);
            second.copy_from_slice(&a_5ms_older.rotation_fragment);
            third.copy_from_slice(&a_10ms_older.rotation_fragment);
            concat
        };
        let ([quat_latest, quat_5ms, quat_10ms], _ts) = joycon_quat::Quaternion::parse(rotation_data).unwrap(); // TODO: throw error

        let data = [
            (latest.accel, quat_latest),
            (a_5ms_older.accel, quat_5ms),
            (a_10ms_older.accel, quat_10ms),
        ];

        Ok(IMUData { data })
    }
}

struct QuatFullMode<D: JoyConDriver> {
    driver: D,
}

impl<D: JoyConDriver> InputReportMode<D> for QuatFullMode<D> {
    type Report = StandardInputReport<IMUData>;
    type ArgsType = [u8; 1];
    const SUB_COMMAND: SubCommand = SubCommand::SetInputReportMode;
    const ARGS: Self::ArgsType = [0x02];

    fn new(mut driver: D) -> JoyConResult<Self> {
        // enable IMU(6-Axis sensor)
        let imf_enabled = driver
            .enabled_features()
            .iter()
            .any(|jf| matches!(jf, JoyConFeature::IMUFeature(_)));
        if !imf_enabled {
            driver.enable_feature(JoyConFeature::IMUFeature(IMUConfig::default()))?;
        }

        driver.set_valid_reply(false);

        driver.send_sub_command(Self::SUB_COMMAND, Self::ARGS.as_ref())?;

        Ok(QuatFullMode { driver })
    }

    fn driver(&self) -> &D {
        &self.driver
    }

    fn driver_mut(&mut self) -> &mut D {
        &mut self.driver
    }

    fn into_driver(self) -> D {
        self.driver
    }
}

// Gyro: 2000dps
// Accel: 8G
// https://github.com/dekuNukem/Nintendo_Switch_Reverse_Engineering/blob/master/imu_sensor_notes.md

// Convert to acceleration in G
fn acc(n: i16, offset: i16) -> f64 {
    let n = n.saturating_sub(offset);
    n as f64 * 0.00024414435f64 // 16000/65535/1000
}
// Convert to acceleration in radians/s
fn gyro(n: i16, offset: i16, scale: f64) -> f64 {
    n.saturating_sub(offset) as f64
    * scale
    // NOTE: 13371 is technically a value present in flash, in practice it seems to be constant.
    //* (936.0 / (13371 - offset) as f64) // to degrees/s
    * 0.07000839246f64 // 4588/65535 - degrees/s
    .to_radians() // radians/s
}

fn convert_battery(battery: BatteryLevel) -> Battery {
    match battery {
        BatteryLevel::Empty => Battery::Empty,
        BatteryLevel::Critical => Battery::Critical,
        BatteryLevel::Low => Battery::Low,
        BatteryLevel::Medium => Battery::Medium,
        BatteryLevel::Full => Battery::Full,
    }
}

fn convert_design(device_type: &JoyConDeviceType) -> JoyconDesignType {
    match device_type {
        JoyConDeviceType::JoyConL => JoyconDesignType::Left,
        JoyConDeviceType::JoyConR => JoyconDesignType::Right,
        JoyConDeviceType::ProCon => JoyconDesignType::Pro,
    }
}

fn quat_convert(quat: joycon_quat::Quaternion) -> nalgebra::UnitQuaternion<f64> {
    // NOTE: should already be approximately normalized, consider skipping
    nalgebra::UnitQuaternion::new_normalize(nalgebra::Quaternion::new(
        quat.0[0].to_num(),
        quat.0[1].to_num(),
        quat.0[2].to_num(),
        quat.0[3].to_num()
    ))
}

fn joycon_listen_loop(
    standard: QuatFullMode<SimpleJoyConDriver>,
    tx: &mpsc::Sender<ChannelData>,
    calib: IMUCalibration,
    settings: &settings::Handler,
) {
    let serial_number = standard.driver().joycon().serial_number().to_owned();
    let device_type = standard.driver().joycon().device_type();
    let calib = match calib {
        IMUCalibration::Available {
            acc_origin_position: ao,
            gyro_origin_position: go,
            ..
        } => ([ao.x, ao.y, ao.z], [go.x, go.y, go.z]),
        IMUCalibration::Unavailable => ([0, 0, 0], [0, 0, 0]),
    };
    let neg_right: fn(f64) -> f64 = match device_type {
        JoyConDeviceType::JoyConR => |v| -v,
        JoyConDeviceType::JoyConL | JoyConDeviceType::ProCon => |v| v,
    };
    let mut last_battery = None;
    loop {
        match standard.read_input_report() {
            Ok(report) => {
                if report.common.input_report_id == 48 {
                    if Some(report.common.battery.level) != last_battery {
                        last_battery = Some(report.common.battery.level);
                        tx.send(ChannelData::new(
                            serial_number.clone(),
                            ChannelInfo::Battery(convert_battery(report.common.battery.level)),
                        ))
                        .unwrap();
                    }
                    if report.common.pushed_buttons.contains(Buttons::Up)
                        || report.common.pushed_buttons.contains(Buttons::B)
                    {
                        tx.send(ChannelData::new(serial_number.clone(), ChannelInfo::Reset))
                            .unwrap();
                    }
                    let gyro_scale_factor = settings.load().joycon_scale_get(&serial_number);
                    let imu_data = report.extra.data.map(|data| JoyconQuatData {
                        accel_x: acc(data.0.accel_x, calib.0[0]),
                        accel_y: neg_right(acc(data.0.accel_y, calib.0[1])),
                        accel_z: neg_right(acc(data.0.accel_z, calib.0[2])),
                        quat: quat_convert(data.1)
                    });
                    // let imu_data = report.extra.data.map(|data| JoyconAxisData {
                    //     accel_x: acc(data.accel_x, calib.0[0]),
                    //     accel_y: neg_right(acc(data.accel_y, calib.0[1])),
                    //     accel_z: neg_right(acc(data.accel_z, calib.0[2])),
                    //     gyro_x: gyro(data.gyro_1, calib.1[0], gyro_scale_factor),
                    //     gyro_y: neg_right(gyro(data.gyro_2, calib.1[1], gyro_scale_factor)),
                    //     gyro_z: neg_right(gyro(data.gyro_3, calib.1[2], gyro_scale_factor)),
                    // });
                    // tx.send(ChannelData::new(
                    //     serial_number.clone(),
                    //     ChannelInfo::ImuData(imu_data),
                    // ))
                    // .unwrap();

                    tx.send(ChannelData::new(serial_number.clone(), ChannelInfo::QuatData(imu_data))).unwrap()
                }
            }
            Err(JoyConError::Disconnected) => {
                tx.send(ChannelData::new(serial_number, ChannelInfo::Disconnected))
                    .unwrap();
                return;
            }
            _ => {}
        }
    }
}

fn joycon_thread(
    d: Arc<Mutex<JoyConDevice>>,
    tx: mpsc::Sender<ChannelData>,
    settings: settings::Handler,
) {
    loop {
        if match d.lock() {
            Ok(d) => d,
            Err(d) => d.into_inner(),
        }
        .is_connected()
        {
            if let Ok(mut driver) = SimpleJoyConDriver::new(&d) {
                let joycon = driver.joycon();
                let color = joycon.color().clone();
                let design = JoyconDesign {
                    color: format!(
                        "#{:02x}{:02x}{:02x}",
                        color.body[0], color.body[1], color.body[2]
                    ),
                    design_type: convert_design(&joycon.device_type()),
                };

                let mut calib = joycon.imu_user_calibration().clone();
                if calib == IMUCalibration::Unavailable {
                    calib = joycon.imu_factory_calibration().clone();
                }

                tx.send(ChannelData {
                    serial_number: joycon.serial_number().to_owned(),
                    info: ChannelInfo::Connected(design),
                })
                .unwrap();

                drop(joycon);

                driver
                    .set_player_lights(&[LightUp::LED0, LightUp::LED3], &[])
                    .ok();

                if let Ok(standard) = QuatFullMode::new(driver) {
                    joycon_listen_loop(standard, &tx, calib, &settings);
                }
            }
        }
        // Joycon was disconnected, check for reconnection after 1 second
        thread::sleep(Duration::from_millis(1000));
    }
}

pub fn spawn_thread(tx: mpsc::Sender<ChannelData>, settings: settings::Handler) {
    let manager = JoyConManager::get_instance();
    let devices = {
        let lock = manager.lock();
        match lock {
            Ok(manager) => manager.new_devices(),
            Err(_) => return,
        }
    };
    for d in devices.iter() {
        let tx = tx.clone();
        let settings = settings.clone();
        thread::spawn(move || joycon_thread(d, tx, settings));
    }
}
