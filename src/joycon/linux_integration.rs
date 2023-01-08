use std::{collections::HashSet, sync::mpsc, time::Duration};
use tokio::time::{interval, sleep};

use evdev::enumerate;

use crate::settings;

use super::ChannelData;

const USB_VENDOR_ID_NINTENDO: u16 = 0x057e;
const USB_DEVICE_ID_NINTENDO_WIIMOTE: u16 = 0x0306;
const USB_DEVICE_ID_NINTENDO_WIIMOTE2: u16 = 0x0330;
const USB_DEVICE_ID_NINTENDO_JOYCONL: u16 = 0x2006;
const USB_DEVICE_ID_NINTENDO_JOYCONR: u16 = 0x2007;
const USB_DEVICE_ID_NINTENDO_PROCON: u16 = 0x2009;
const USB_DEVICE_ID_NINTENDO_CHRGGRIP: u16 = 0x200E;

#[tokio::main(worker_threads = 2)]
pub async fn spawn_thread(tx: mpsc::Sender<ChannelData>, settings: settings::Handler) {
    let mut slow_stream = interval(Duration::from_secs(3));
    let mut paths = HashSet::new();
    loop {
        slow_stream.tick().await;
        for (path, mut device) in enumerate() {
            if device.input_id().vendor() != USB_VENDOR_ID_NINTENDO
                || device.input_id().product() != USB_DEVICE_ID_NINTENDO_JOYCONL
                || device.input_id().product() != USB_DEVICE_ID_NINTENDO_JOYCONR
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
            // The device name is defined on all nintendo devices in the kernel,
            // so unwrap shouldn't fail...
            if device.name().unwrap().ends_with("IMU") {}
        }
    }
}
