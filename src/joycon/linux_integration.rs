use std::{
    collections::HashSet,
    sync::mpsc,
    time::{Duration, SystemTime},
};
use tokio::time::{interval, sleep};

use evdev::{enumerate, Device, EventStream, InputEventKind, Key};

use crate::settings;

use super::{
    imu::JoyconAxisData, Battery, ChannelData, ChannelInfo, JoyconDesign, JoyconDesignType,
};

// Resolution definitions from hid-nintendo.c from linux:
// https://github.com/torvalds/linux/blob/master/drivers/hid/hid-nintendo.c
fn acc(n: i32) -> f64 {
    n as f64 * (1f64/4096f64) // JC_IMU_ACCEL_RES_PER_G
}
fn gyro(n: i32, scale: f64) -> f64 {
    (n as f64 / 1000f64 // Value is scaled in gyro by 1000
        * scale
        / 14.247f64) // JC_IMU_GYRO_RES_PER_DPS
            .to_radians()
}

const USB_VENDOR_ID_NINTENDO: u16 = 0x057e;
const USB_DEVICE_ID_NINTENDO_WIIMOTE: u16 = 0x0306;
const USB_DEVICE_ID_NINTENDO_WIIMOTE2: u16 = 0x0330;
const USB_DEVICE_ID_NINTENDO_JOYCONL: u16 = 0x2006;
const USB_DEVICE_ID_NINTENDO_JOYCONR: u16 = 0x2007;
const USB_DEVICE_ID_NINTENDO_PROCON: u16 = 0x2009;
const USB_DEVICE_ID_NINTENDO_CHRGGRIP: u16 = 0x200E;

fn convert_design(product_code: u16) -> JoyconDesignType {
    match product_code {
        USB_DEVICE_ID_NINTENDO_JOYCONL | USB_DEVICE_ID_NINTENDO_PROCON => JoyconDesignType::Left,
        USB_DEVICE_ID_NINTENDO_JOYCONR => JoyconDesignType::Right,
        _ => unreachable!(),
    }
}

async fn joycon_listener(
    tx: mpsc::Sender<ChannelData>,
    settings: settings::Handler,
    mut input: EventStream,
) {
    let mac = input.device().unique_name().unwrap().to_string(); // Joycons always have unique name
    let battery = Battery::Full; // can be fetched with upower
    loop {
        let ev = input.next_event().await.unwrap();

        if let InputEventKind::Key(key) = ev.kind() {
            if (key == Key::BTN_DPAD_UP || key == Key::BTN_SOUTH) && ev.value() == 0 {
                tx.send(ChannelData {
                    serial_number: mac.clone(),
                    info: ChannelInfo::Reset,
                })
                .unwrap();
            }
        }
    }
}


async fn imu_listener(
    tx: mpsc::Sender<ChannelData>,
    settings: settings::Handler,
    mut input: EventStream,
) {
    let mac = input.device().unique_name().unwrap().to_string(); // Joycons always have unique name
    let mut imu_array = [JoyconAxisData {
        accel_x: 0.0,
        accel_y: 0.0,
        accel_z: 0.0,
        gyro_x: 0.0,
        gyro_y: 0.0,
        gyro_z: 0.0,
    }; 3];
    let mut count = 0;
    let mut sys_time = SystemTime::now();
    loop {
        let ev = input.next_event().await.unwrap();
        if ev.timestamp() == sys_time {
            continue;
        }
        sys_time = ev.timestamp();

        let gyro_scale_factor = settings.load().joycon_scale_get(&mac);
        let axis = input.device().get_abs_state().unwrap();
        let accel_axis = &axis[..3];
        let gyro_axis = &axis[3..6];
        imu_array[count] = JoyconAxisData {
            accel_x: acc(accel_axis[0].value),
            accel_y: acc(accel_axis[1].value),
            accel_z: acc(accel_axis[2].value),
            gyro_x: gyro(gyro_axis[0].value, gyro_scale_factor),
            gyro_y: gyro(gyro_axis[1].value, gyro_scale_factor),
            gyro_z: gyro(gyro_axis[2].value, gyro_scale_factor),
        };
        println!("{:?}", imu_array[count]);
        count += 1;
        if count == 3 {
            count = 0;
            tx.send(ChannelData {
                serial_number: mac.clone(),
                info: ChannelInfo::ImuData(imu_array),
            })
            .unwrap();
        }
    }
}

#[tokio::main]
pub async fn spawn_thread(tx: mpsc::Sender<ChannelData>, settings: settings::Handler) {
    let mut slow_stream = interval(Duration::from_secs(5));
    let mut paths = HashSet::new();
    loop {
        slow_stream.tick().await;
        for (path, mut device) in enumerate() {
            if (device.input_id().vendor() != USB_VENDOR_ID_NINTENDO || paths.contains(&path))
                || (device.input_id().product() != USB_DEVICE_ID_NINTENDO_JOYCONL
                    && device.input_id().product() != USB_DEVICE_ID_NINTENDO_JOYCONR
                    && device.input_id().product() != USB_DEVICE_ID_NINTENDO_PROCON)
            {
                continue;
            }
            if device.grab().is_err() {
                println!(
                    "Joycon {:?} was grabbed by someone else already.",
                    device.unique_name()
                );
                continue;
            }
            paths.insert(path);
            let tx = tx.clone();
            let settings = settings.clone();
            // The device name is defined on all nintendo devices in the kernel,
            // so unwrap shouldn't fail...
            if device.name().unwrap().ends_with("IMU") {
                let stream = device.into_event_stream().unwrap();
                tokio::spawn(imu_listener(tx, settings, stream));
            } else {
                let mac = device.unique_name().unwrap().to_string();
                tx.send(ChannelData {
                    serial_number: mac,
                    info: ChannelInfo::Connected(JoyconDesign {
                        color: "#828282".to_string(),
                        design_type: convert_design(device.input_id().product()),
                    }),
                })
                .unwrap();
                let stream = device.into_event_stream().unwrap();
                tokio::spawn(joycon_listener(tx, settings, stream));
            }
        }
    }
}
