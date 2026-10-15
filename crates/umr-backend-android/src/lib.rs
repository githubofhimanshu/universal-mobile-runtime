pub mod backend;
pub mod host_detector;
pub mod hypervisor;
pub mod package_manager;
pub mod transport;
pub mod watchdog;

pub use backend::AndroidVirtualizationBackend;
pub use host_detector::{HostCapabilities, HostOs, HostPlatformDetector, HypervisorKind};
pub use hypervisor::{
    IPartitionAdapter, LinuxContainerAdapter, MacVirtAdapter, PartitionConfig, PartitionMount,
    WhpxAdapter,
};
pub use package_manager::{AndroidPackageManager, InstalledPackageRecord};
pub use transport::{
    AdbTransport, CommandOutput, GuestTransport, MockGuestTransport, VsockTransport,
};
pub use watchdog::{GuestCrashInfo, WatchdogSupervisor};

