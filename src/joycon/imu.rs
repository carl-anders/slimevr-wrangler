use ahrs::{Ahrs, Madgwick};
use nalgebra::{Quaternion, UnitQuaternion, Vector3};

// Gyro: 2000dps
// Accel: 8G
// https://github.com/dekuNukem/Nintendo_Switch_Reverse_Engineering/blob/master/imu_sensor_notes.md

// Convert to acceleration in G
fn acc(n: i16) -> f64 {
    n as f64 * 0.00024414435f64 // 16000/65535/1000
}
// Convert to acceleration in radians/s
// TODO: add option for different numbers - or find the right magic
fn gyro(n: i16) -> f64 {
    n as f64
    * 0.07000839246f64 // 4588/65535 - degrees/s
    * (std::f64::consts::PI / 180.0f64) // radians/s
}

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JoyconAxisDataRaw {
    pub accel_x: i16,
    pub accel_y: i16,
    pub accel_z: i16,
    pub gyro_x: i16,
    pub gyro_y: i16,
    pub gyro_z: i16,
}

impl From<JoyconAxisDataRaw> for JoyconAxisData {
    fn from(item: JoyconAxisDataRaw) -> Self {
        Self {
            accel_x: acc(item.accel_x),
            accel_y: acc(item.accel_y),
            accel_z: acc(item.accel_z),
            gyro_x: gyro(item.gyro_x),
            gyro_y: gyro(item.gyro_y),
            gyro_z: gyro(item.gyro_z),
        }
    }
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
