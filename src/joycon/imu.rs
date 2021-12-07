use ahrs::{Ahrs, Madgwick};
use joycon_rs::joycon::input_report_mode::standard_full_mode::IMUData;
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
    pub fn update(&mut self, imu_data: IMUData) {
        for frame in imu_data.data {
            let gyro = Vector3::new(gyro(frame.gyro_1), gyro(frame.gyro_2), gyro(frame.gyro_3));
            let acc = Vector3::new(acc(frame.accel_x), acc(frame.accel_y), acc(frame.accel_z));
            let rot = self.mad.update_imu(&gyro, &acc);
            match rot {
                Ok(r) => self.rotation = r.clone(),
                Err(e) => {
                    println!("Found IMU Frame with error: (Ignore this if it happens only once or twice)");
                    println!("{:?}", frame);
                    println!("{}", gyro);
                    println!("{}", acc);
                    println!("{}", e);
                }
            }
        }
    }
    // euler_angles: roll, pitch, yaw
    pub fn euler_angles_deg(&self) -> (f64, f64, f64) {
        let ea = self.rotation.euler_angles();
        (deg(ea.0), deg(ea.1), deg(ea.2))
    }
}
