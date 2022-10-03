#[cfg(test)]
mod tests {
    use crate::slime::deku::PacketType;
    use deku::{DekuContainerRead, DekuContainerWrite};
    use md5::{Digest, Md5};
    use nalgebra::{Quaternion, UnitQuaternion};

    #[test]
    fn handshake() {
        let mut hasher = Md5::new();
        hasher.update(b"This is a joycon serial number");
        let mac: [u8; 6] = hasher.finalize()[0..6].try_into().unwrap();
        let handshake = PacketType::Handshake {
            packet_id: 1,
            board: 2,
            imu: 3,
            mcu_type: 4,
            imu_info: (5, 6, 7),
            build: 8,
            firmware: "test".to_string().into(),
            mac_address: mac,
        };
        let data: Vec<u8> = vec![
            0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 4, 0, 0, 0, 5, 0,
            0, 0, 6, 0, 0, 0, 7, 0, 0, 0, 8, 4, 116, 101, 115, 116, 121, 34, 164, 250, 231, 204,
        ];

        assert_eq!(handshake.to_bytes().unwrap(), data);
    }
    #[test]
    fn quat() {
        let quat = UnitQuaternion::new_unchecked(Quaternion::new(1.0f64, 0.0f64, 0.0f64, 0.0f64));
        let rotation = PacketType::Rotation {
            packet_id: 1,
            quat: (*quat.quaternion()).into(),
        };

        let data: Vec<u8> = vec![
            0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 63, 128, 0, 0,
        ];

        assert_eq!(rotation.to_bytes().unwrap(), data);
    }
    #[test]
    fn sensor_info() {
        let sensor_info = PacketType::SensorInfo {
            packet_id: 1,
            sensor_id: 64,
            sensor_status: 3,
            sensor_type: 5,
        };

        let data: Vec<u8> = vec![0, 0, 0, 15, 0, 0, 0, 0, 0, 0, 0, 1, 64, 3, 5];

        assert_eq!(sensor_info.to_bytes().unwrap(), data);
    }
    #[test]
    fn quat_fancy() {
        let quat = UnitQuaternion::new_unchecked(Quaternion::new(1.0f64, 0.0f64, 0.0f64, 0.0f64));
        let rotation = PacketType::RotationData {
            packet_id: 1,
            sensor_id: 64,
            data_type: 1,
            quat: (*quat.quaternion()).into(),
            calibration_info: 0,
        };

        let data: Vec<u8> = vec![
            0, 0, 0, 17, 0, 0, 0, 0, 0, 0, 0, 1, 64, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 63,
            128, 0, 0, 0,
        ];

        assert_eq!(rotation.to_bytes().unwrap(), data);
    }
    #[test]
    fn test_ping() {
        let data = [0, 0, 0, 10, 1, 2, 3, 4];
        let result = PacketType::from_bytes((&data, 0)).unwrap().1;

        let ping = PacketType::Ping { id: 16909060 };
        assert_eq!(result, ping);
    }
}
