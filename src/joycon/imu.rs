use std::time::Duration;

use joycon_quat::types::Timestamp;
use nalgebra::{Quaternion, UnitQuaternion, Vector3};
use vqf_cxx::{VQFBuilder, VQF};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JoyconAxisData {
    pub accel_x: f64,
    pub accel_y: f64,
    pub accel_z: f64,
    pub gyro_x: f64,
    pub gyro_y: f64,
    pub gyro_z: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JoyconQuatData {
    pub accel_x: f64,
    pub accel_y: f64,
    pub accel_z: f64,
    pub quat: UnitQuaternion<f64>,
}

pub struct Imu {
    vqf: VQF,
    pub rotation: UnitQuaternion<f64>,
    last_raw: UnitQuaternion<f64>,
}
impl Imu {
    const SAMPLE_SEC: f64 = Duration::from_millis(5).as_secs_f64();

    pub fn new() -> Self {
        Self {
            vqf: VQFBuilder::new(Self::SAMPLE_SEC).build(),
            rotation: UnitQuaternion::identity(),
            last_raw: UnitQuaternion::identity(),
        }
    }
    pub fn update(&mut self, frame: JoyconAxisData) {
        let gyro = Vector3::new(frame.gyro_x, frame.gyro_y, frame.gyro_z);
        let acc = Vector3::new(frame.accel_x, frame.accel_y, frame.accel_z);
        self.vqf.update_6dof(&gyro.data.0[0], &acc.data.0[0]);
        self.rotation = UnitQuaternion::new_unchecked(self.vqf.get_quat_6d().into());
    }

    pub fn update_quat(&mut self, quats: [JoyconQuatData; 3], ts: Timestamp) {
        // the timestamp goes up by ~12 every 15ms. Determine how many 15ms periods have passed.
        let repeat_count = (u8::from(ts.count()) as f64 / 12.0).ceil() as u8;

        for frame in quats {
            let a = self.last_raw.rotation_to(&frame.quat);
            let b = self.last_raw.rotation_to(&frame.quat.inverse());

            self.last_raw = frame.quat;

            // The conjugation status of the quaternion is effectively random due to the use of quatcompress,
            // so let's compute both and find the lowest angle.
            let mut current = std::cmp::min_by(a, b, |a, b| a.angle().total_cmp(&b.angle()));

            // is this necessary???
            // if current.w < 0.0 {
            //     current.inverse_mut();
            // }

            let mut gyro = current.scaled_axis() / Self::SAMPLE_SEC;

            // The axis don't appear to be exactly the same, for some reason: let's correct for that.
            let tmp_x = -gyro.z;

            gyro.z = -gyro.x;
            gyro.x = tmp_x;

            //println!("gyro: {:?}", gyro);

            let acc = Vector3::new(frame.accel_x, frame.accel_y, frame.accel_z);

            for _ in 0..repeat_count {
                self.vqf.update_6dof(&gyro.data.0[0], &acc.data.0[0]);
            }
        }
        self.rotation = UnitQuaternion::new_unchecked(self.vqf.get_quat_6d().into());
    }
    // euler_angles: roll, pitch, yaw
    pub fn euler_angles_deg(&self) -> (f64, f64, f64) {
        let ea = self.rotation.euler_angles();
        (ea.0.to_degrees(), ea.1.to_degrees(), ea.2.to_degrees())
    }
}
