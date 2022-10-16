use std::{sync::mpsc, thread, time::Duration};

use super::{
    communication::{ChannelInfo, JoyconData, JoyconDeviceInfo},
    imu::JoyconAxisData,
    JoyconDesign, JoyconDesignType,
};

fn spawn_test(tx: mpsc::Sender<ChannelInfo>, color: String, sn: String, z_change: f64) {
    let iinfo = JoyconDeviceInfo {
        serial_number: sn.clone(),
        design: JoyconDesign {
            color,
            design_type: JoyconDesignType::Left,
        },
    };
    tx.send(ChannelInfo::Connected(iinfo)).unwrap();

    loop {
        let d = JoyconAxisData {
            accel_x: 0.0,
            accel_y: -1.0,
            accel_z: 0.0,
            gyro_x: 0.0,
            gyro_y: 0.0,
            gyro_z: z_change,
        };
        let data = JoyconData {
            serial_number: sn.clone(),
            imu_data: [d, d, d],
        };
        tx.send(ChannelInfo::Data(data)).unwrap();
        thread::sleep(Duration::from_millis(16));
        if d.accel_x > 1.0 {
            break;
        }
    }
}

pub fn test_controllers(tx: mpsc::Sender<ChannelInfo>) {
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
