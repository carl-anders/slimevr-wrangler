use std::{
    collections::HashSet,
    sync::{mpsc, Arc},
    time::{Duration, Instant, SystemTime},
};
use tokio::{sync::Mutex, time::interval};

use evdev::{enumerate, EventStream, InputEventKind, Key};
use upower_dbus::{DeviceProxy, UPowerProxy};

use crate::settings;

use super::{
    imu::JoyconAxisData, Battery, ChannelData, ChannelInfo, JoyconDesign, JoyconDesignType,
};

// Resolution definitions from hid-nintendo.c from linux:
// https://github.com/torvalds/linux/blob/master/drivers/hid/hid-nintendo.c
fn acc(n: i32) -> f64 {
    n as f64 / 4096f64 // JC_IMU_ACCEL_RES_PER_G
}
fn gyro(n: i32, scale: f64) -> f64 {
    (n as f64 * scale / 14247f64) // JC_IMU_GYRO_RES_PER_DPS
        .to_radians()
}

const USB_VENDOR_ID_NINTENDO: u16 = 0x057e;
// Soon™️
#[allow(dead_code)]
const USB_DEVICE_ID_NINTENDO_WIIMOTE: u16 = 0x0306;
#[allow(dead_code)]
const USB_DEVICE_ID_NINTENDO_WIIMOTE2: u16 = 0x0330;
const USB_DEVICE_ID_NINTENDO_JOYCONL: u16 = 0x2006;
const USB_DEVICE_ID_NINTENDO_JOYCONR: u16 = 0x2007;
const USB_DEVICE_ID_NINTENDO_PROCON: u16 = 0x2009;
const USB_DEVICE_ID_NINTENDO_CHRGGRIP: u16 = 0x200E;

fn convert_design(product_code: u16) -> JoyconDesignType {
    match product_code {
        USB_DEVICE_ID_NINTENDO_JOYCONL => JoyconDesignType::Left,
        USB_DEVICE_ID_NINTENDO_JOYCONR | USB_DEVICE_ID_NINTENDO_CHRGGRIP => JoyconDesignType::Right,
        USB_DEVICE_ID_NINTENDO_PROCON => JoyconDesignType::Pro,
        _ => unreachable!(),
    }
}

fn convert_battery(battery: upower_dbus::BatteryLevel) -> Battery {
    match battery {
        upower_dbus::BatteryLevel::Full | upower_dbus::BatteryLevel::High => Battery::Full,
        upower_dbus::BatteryLevel::Normal => Battery::Medium,
        upower_dbus::BatteryLevel::Low => Battery::Low,
        upower_dbus::BatteryLevel::Critical => Battery::Critical,
        upower_dbus::BatteryLevel::Unknown | upower_dbus::BatteryLevel::None => Battery::Empty,
    }
}

async fn joycon_listener(tx: mpsc::Sender<ChannelData>, mut input: EventStream) {
    let mac = input.device().unique_name().unwrap().to_string(); // Joycons always have unique name

    while let Ok(ev) = input.next_event().await {
        if let InputEventKind::Key(key) = ev.kind() {
            // if DPAD_UP or BTN_SOUTH and button is lifted
            if (key == Key::BTN_DPAD_UP || key == Key::BTN_SOUTH) && ev.value() == 0 {
                tx.send(ChannelData {
                    serial_number: mac.clone(),
                    info: ChannelInfo::Reset,
                })
                .unwrap();
            }
        }
    }

    tx.send(ChannelData {
        serial_number: mac,
        info: ChannelInfo::Disconnected,
    })
    .unwrap();
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
    let mut last_event = input.device().get_abs_state().unwrap();

    while let Ok(ev) = input.next_event().await {
        // If it's the same timestamp, just skip and remember the event
        if ev.timestamp() == sys_time {
            last_event = input.device().get_abs_state().unwrap();
            continue;
        }
        sys_time = ev.timestamp();

        let gyro_scale_factor = settings.load().joycon_scale_get(&mac);
        // We grab the last event so we actually announce it on the tx
        let axis = last_event;
        last_event = input.device().get_abs_state().unwrap();

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

async fn check_batteries(tx: mpsc::Sender<ChannelData>, macs: &HashSet<String>) {
    let connection = zbus::Connection::system().await.unwrap();
    let upower = UPowerProxy::new(&connection).await.unwrap();

    for upower_dev in upower.enumerate_devices().await.unwrap() {
        let device = DeviceProxy::new(&connection, upower_dev.clone())
            .await
            .unwrap();
        let Ok(serial) = device.serial().await else { continue; };

        if macs.contains(&serial) {
            let level = convert_battery(device.battery_level().await.unwrap());
            tx.send(ChannelData {
                serial_number: serial,
                info: ChannelInfo::Battery(level),
            })
            .unwrap();
        }
    }
}

#[tokio::main]
pub async fn spawn_thread(tx: mpsc::Sender<ChannelData>, settings: settings::Handler) {
    if !users::group_access_list()
        .unwrap_or_default()
        .iter()
        .any(|group| group.name() == "input")
    {
        println!("\x1b[0;31m[ERROR]\x1b[0m Current user not in \"input\" group.");
        println!("You need to add your user to the \"input\" group to use Wrangler.");
    }

    let mut slow_stream = interval(Duration::from_secs(2));
    let paths = Arc::new(Mutex::new(HashSet::new()));
    let mut battery_macs = HashSet::new();
    let mut battery_check = Instant::now();

    loop {
        // Wait 2 seconds for enumerating
        slow_stream.tick().await;
        for (path, mut device) in enumerate() {
            // Check if device is a nintendo one or it's already in the paths hashset
            // then check if its any of the supported switch joysticks
            if (device.input_id().vendor() != USB_VENDOR_ID_NINTENDO
                || paths.lock().await.contains(&path))
                || (device.input_id().product() != USB_DEVICE_ID_NINTENDO_JOYCONL
                    && device.input_id().product() != USB_DEVICE_ID_NINTENDO_JOYCONR
                    && device.input_id().product() != USB_DEVICE_ID_NINTENDO_PROCON
                    && device.input_id().product() != USB_DEVICE_ID_NINTENDO_CHRGGRIP)
            {
                continue;
            }

            if device.grab().is_err() {
                println!(
                    "Joycon {:?} is in use by another program.",
                    device.unique_name()
                );
                continue;
            }

            paths.lock().await.insert(path.clone());
            let tx = tx.clone();
            let settings = settings.clone();

            // The device name is defined on all nintendo devices in the kernel,
            // so unwrap shouldn't fail...
            if device.name().unwrap().contains("IMU") {
                // Make IMU event listener
                let stream = device.into_event_stream().unwrap();
                let paths = paths.clone();
                tokio::spawn(async move {
                    imu_listener(tx, settings, stream).await;
                    paths.lock().await.remove(&path);
                });
            } else {
                let mac = device.unique_name().unwrap().to_string();

                // Announce that a new device was connected
                tx.send(ChannelData {
                    serial_number: mac.clone(),
                    info: ChannelInfo::Connected(JoyconDesign {
                        color: "#828282".to_string(),
                        design_type: convert_design(device.input_id().product()),
                    }),
                })
                .unwrap();

                // Listen to events of the joycon
                let stream = device.into_event_stream().unwrap();

                let paths = paths.clone();
                tokio::spawn(async move {
                    joycon_listener(tx, stream).await;
                    paths.lock().await.remove(&path);
                });

                // Add to list of batteries to check and check directly
                battery_macs.insert(mac);
                battery_check = Instant::now();
            }
        }
        if battery_check <= Instant::now() {
            battery_check += Duration::from_secs(60 * 5);
            check_batteries(tx.clone(), &battery_macs).await;
        }
    }
}
