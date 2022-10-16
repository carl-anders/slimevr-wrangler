//mod ui;
mod imu;

mod communication;
pub use communication::JoyconStatus;
use communication::*;

mod integration;
use integration::spawn_thread;
mod test_integration;

mod wrapper;
pub use wrapper::*;

mod svg;
pub use svg::*;
