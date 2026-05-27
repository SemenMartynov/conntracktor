mod accelerator;
mod ethtool;
mod hardware;
mod soc;
pub mod system;

pub use accelerator::AccelerationStatus;
pub use hardware::HardwareInfo;
pub use soc::SocModel;
pub use system::{HostInfo, SystemStats};
