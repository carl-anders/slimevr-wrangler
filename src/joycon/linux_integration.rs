use std::{collections::HashSet, sync::mpsc, time::Duration};
use tokio::time::{interval, sleep};

use evdev::{enumerate, Device, EventStream};

use crate::settings;

use super::{ChannelData, JoyconDesignType, Battery};

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

async fn joycon_listener(tx: mpsc::Sender<ChannelData>, settings: settings::Handler, mut input: EventStream) {
    let mac = input.device().unique_name().unwrap(); // Joycons always have unique name
    let device_type = convert_design(input.device().input_id().product());
    let battery = Battery::Full; // can be fetched with upower
    loop {
        let ev = input.next_event().await.unwrap();
        
    }
}

async fn imu_listener(tx: mpsc::Sender<ChannelData>, settings: settings::Handler, mut input: EventStream) {
    let mac = input.device().unique_name().unwrap(); // Joycons always have unique name

    loop {
        let ev = input.next_event().await.unwrap();
        
    }
}

#[tokio::main]
pub async fn spawn_thread(tx: mpsc::Sender<ChannelData>, settings: settings::Handler) {
    let mut slow_stream = interval(Duration::from_secs(5));
    let mut paths = HashSet::new();
    loop {
        slow_stream.tick().await;
        for (path, mut device) in enumerate() {
            if device.input_id().vendor() != USB_VENDOR_ID_NINTENDO
                || device.input_id().product() != USB_DEVICE_ID_NINTENDO_JOYCONL
                || device.input_id().product() != USB_DEVICE_ID_NINTENDO_JOYCONR
                || device.input_id().product() != USB_DEVICE_ID_NINTENDO_PROCON
                || paths.contains(&path)
            {
                continue;
            }
            if let Err(_) = device.grab() {
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
            }
        }
    }
}
