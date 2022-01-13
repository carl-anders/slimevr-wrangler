use ahrs::{Ahrs, Madgwick};
use nalgebra::{Quaternion, UnitQuaternion, Vector3};


fn deg(r: f64) -> f64 {
    r * (180.0f64 / std::f64::consts::PI)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JoyconAxisData {
    pub accel_x: f64,
    pub accel_y: f64,
    pub accel_z: f64,
    pub gyro_x: f64,
    pub gyro_y: f64,
    pub gyro_z: f64,
}

#[derive(Debug)]
pub struct Imu {
    mad: Madgwick<f64>,
    pub rotation: UnitQuaternion<f64>,
}
impl Imu {
    pub fn new() -> Self {
        // TODO: Lägg till uppdatering med intern kalibrering
        // TODO: Lägg till Mahony som alternativ?
        Self {
            mad: Madgwick::new(0.005f64, 0.1f64),
            rotation: UnitQuaternion::new_unchecked(Quaternion::new(
                1.0f64, 0.0f64, 0.0f64, 0.0f64,
            )),
        }
    }
    pub fn update(&mut self, frame: JoyconAxisData) {
        let gyro = Vector3::new(frame.gyro_x, frame.gyro_y, frame.gyro_z);
        let acc = Vector3::new(frame.accel_x, frame.accel_y, frame.accel_z);
        let rot = self.mad.update_imu(&gyro, &acc);
        match rot {
            Ok(r) => self.rotation = *r,
            Err(e) => {
                println!(
                    "Found IMU Frame with error: (Ignore this if it happens only once or twice)"
                );
                println!("{:?}", frame);
                println!("{}", gyro);
                println!("{}", acc);
                println!("{}", e);
            }
        }
    }
    // euler_angles: roll, pitch, yaw
    pub fn euler_angles_deg(&self) -> (f64, f64, f64) {
        let ea = self.rotation.euler_angles();
        (deg(ea.0), deg(ea.1), deg(ea.2))
    }
}
