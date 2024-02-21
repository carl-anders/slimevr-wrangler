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
}
impl Imu {
    pub fn new() -> Self {
        Self {
            vqf: VQFBuilder::new(0.005f64).build(),
            rotation: UnitQuaternion::new_unchecked(Quaternion::new(
                1.0f64, 0.0f64, 0.0f64, 0.0f64,
            )),
        }
    }
    pub fn update(&mut self, frame: JoyconAxisData) {
        let gyro = Vector3::new(frame.gyro_x, frame.gyro_y, frame.gyro_z);
        let acc = Vector3::new(frame.accel_x, frame.accel_y, frame.accel_z);
        self.vqf.update_6dof(&gyro.data.0[0], &acc.data.0[0]);
        self.rotation = UnitQuaternion::new_unchecked(self.vqf.get_quat_6d().into());
    }
    // euler_angles: roll, pitch, yaw
    pub fn euler_angles_deg(&self) -> (f64, f64, f64) {
        let ea = self.rotation.euler_angles();
        (ea.0.to_degrees(), ea.1.to_degrees(), ea.2.to_degrees())
    }
}
