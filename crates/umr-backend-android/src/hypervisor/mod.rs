pub mod linux_container;
pub mod mac_virt;
pub mod whpx;

pub use linux_container::LinuxContainerAdapter;
pub use mac_virt::MacVirtAdapter;
pub use whpx::WhpxAdapter;

use serde::{Deserialize, Serialize};

/// Generic partition configuration produced by host hypervisor/container adapters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartitionConfig {
    pub partition_id: String,
    pub adapter_name: String,
    pub vcpu_count: usize,
    pub memory_mb: u64,
    pub enable_gpu_acceleration: bool,
    pub virtio_vsock_cid: u32,
    pub virtio_vsock_port: u32,
    pub kernel_cmdline: String,
    pub mount_points: Vec<PartitionMount>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartitionMount {
    pub host_path: String,
    pub guest_path: String,
    pub read_only: bool,
}

/// Abstract contract for host virtualization/container partitioning.
pub trait IPartitionAdapter: Send + Sync {
    fn name(&self) -> &str;
    fn is_supported(&self) -> bool;
    fn create_partition_config(&self, session_id: &str, vcpus: usize, memory_mb: u64) -> PartitionConfig;
}

