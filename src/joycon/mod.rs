//mod ui;
mod imu;

mod communication;
pub use communication::JoyconStatus;
use communication::*;

#[cfg(feature = "use-joycon-rs")]
mod integration;
#[cfg(feature = "use-joycon-rs")]
use integration::*;

#[cfg(all(not(feature = "use-joycon-rs"), feature = "use-joy"))]
mod integration_with_joy;
#[cfg(all(not(feature = "use-joycon-rs"), feature = "use-joy"))]
use integration_with_joy::*;

mod wrapper;
pub use wrapper::*;

mod svg;
pub use svg::*;
