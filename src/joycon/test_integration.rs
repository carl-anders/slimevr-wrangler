use std::{sync::mpsc, thread, time::Duration};

use super::{
    communication::{ChannelData, ChannelInfo},
    imu::JoyconAxisData,
    Battery, JoyconDesign, JoyconDesignType,
};

fn spawn_test(tx: mpsc::Sender<ChannelData>, color: String, sn: String, z_change: f64) {
    tx.send(ChannelData {
        serial_number: sn.clone(),
        info: ChannelInfo::Connected(JoyconDesign {
            color,
            design_type: JoyconDesignType::Left,
        }),
    })
    .unwrap();

    loop {
        let d = JoyconAxisData {
            accel_x: 0.0,
            accel_y: -1.0,
            accel_z: 0.0,
            gyro_x: 0.0,
            gyro_y: 0.0,
            gyro_z: z_change,
        };
        tx.send(ChannelData {
            serial_number: sn.clone(),
            info: ChannelInfo::ImuData([d, d, d]),
        })
        .unwrap();

        tx.send(ChannelData {
            serial_number: sn.clone(),
            info: ChannelInfo::Battery(Battery::Medium),
        })
        .unwrap();

        thread::sleep(Duration::from_millis(16));
        if d.accel_x > 1.0 {
            break;
        }
    }
}

pub fn test_controllers(tx: mpsc::Sender<ChannelData>) {
    let controllers = vec![
        ("#aacc20", "test_0", 0.05),
        ("#aa20cc", "test_1", 0.04),
        ("#20aacc", "test_2", 0.06),
        ("#20ccaa", "test_3", 0.065),
        ("#ccaa20", "test_4", 0.055),
        ("#cc20aa", "test_5", 0.045),
    ];
    for c in controllers {
        let tx_clone = tx.clone();
        std::thread::spawn(move || spawn_test(tx_clone, c.0.into(), c.1.into(), c.2));
    }
}
