mod test_deku;

pub use deku;

use std::string::FromUtf8Error;

use deku::prelude::*;

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "endian", ctx = "endian: deku::ctx::Endian")]
pub struct SlimeQuaternion {
    pub i: f32,
    pub j: f32,
    pub k: f32,
    pub w: f32,
}

#[allow(unused_macros)]
macro_rules! impl_Nalgebra {
    () => {
        use super::*;
        impl From<Quaternion<f64>> for SlimeQuaternion {
            fn from(q: Quaternion<f64>) -> Self {
                Self {
                    i: q.i as _,
                    j: q.j as _,
                    k: q.k as _,
                    w: q.w as _,
                }
            }
        }
        impl From<SlimeQuaternion> for Quaternion<f64> {
            fn from(q: SlimeQuaternion) -> Self {
                Self::new(q.w as _, q.i as _, q.j as _, q.k as _)
            }
        }
    };
}
#[cfg(feature = "nalgebra032")]
mod nalgebra032_impls {
    use nalgebra032::Quaternion;
    impl_Nalgebra!();
}
#[cfg(feature = "nalgebra031")]
mod nalgebra031_impls {
    use nalgebra031::Quaternion;
    impl_Nalgebra!();
}
#[cfg(feature = "nalgebra030")]
mod nalgebra030_impls {
    use nalgebra030::Quaternion;
    impl_Nalgebra!();
}

#[derive(PartialEq, Eq, Debug, DekuRead, DekuWrite)]
#[deku(endian = "endian", ctx = "endian: deku::ctx::Endian")]
pub struct SlimeString {
    #[deku(update = "self.data.len()")]
    count: u8,
    #[deku(count = "count")]
    data: Vec<u8>,
}
impl From<String> for SlimeString {
    fn from(s: String) -> Self {
        let bytes = s.into_bytes();
        Self {
            count: bytes.len() as _,
            data: bytes,
        }
    }
}
impl SlimeString {
    #[allow(dead_code)]
    fn to_string(&self) -> Result<String, FromUtf8Error> {
        String::from_utf8(self.data.clone())
    }
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(type = "u32")]
#[deku(endian = "big")]
pub enum PacketType {
    #[deku(id = "1")]
    Rotation {
        packet_id: u64,
        quat: SlimeQuaternion,
    },
    #[deku(id = "3")]
    Handshake {
        packet_id: u64,
        board: i32,
        imu: i32,
        mcu_type: i32,
        imu_info: (i32, i32, i32),
        build: i32,
        firmware: SlimeString,
        mac_address: [u8; 6],
    },
    #[deku(id = "4")]
    Acceleration {
        packet_id: u64,
        vector: (f32, f32, f32),
        sensor_id: Option<u8>,
    },
    #[deku(id = "10")]
    Ping { id: u32 },
    #[deku(id = "15")]
    SensorInfo {
        packet_id: u64,
        sensor_id: u8,
        sensor_status: u8,
        sensor_type: u8,
    },
    #[deku(id = "17")]
    RotationData {
        packet_id: u64,
        sensor_id: u8,
        data_type: u8,
        quat: SlimeQuaternion,
        calibration_info: u8,
    },
    #[deku(id = "21")]
    UserAction { packet_id: u64, typ: u8 },
    #[deku(id = "55076217")] // u8 array with [3, 'H', 'e', 'y'] as u32
    HandshakeResponse,
}
