use super::{IPartitionAdapter, PartitionConfig, PartitionMount};
use crate::host_detector::HostPlatformDetector;

/// Partition adapter for Linux native container execution.
/// Configures kernel namespaces, cgroups v2 resource controllers, and /dev/binderfs.
pub struct LinuxContainerAdapter {
    is_supported: bool,
}

impl LinuxContainerAdapter {
    pub fn new() -> Self {
        let caps = HostPlatformDetector::detect_current();
        Self {
            is_supported: caps.container_isolation_available && caps.host_os == crate::host_detector::HostOs::Linux,
        }
    }

    pub fn with_support(supported: bool) -> Self {
        Self {
            is_supported: supported,
        }
    }
}

impl Default for LinuxContainerAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl IPartitionAdapter for LinuxContainerAdapter {
    fn name(&self) -> &str {
        "linux-container-adapter"
    }

    fn is_supported(&self) -> bool {
        self.is_supported
    }

    fn create_partition_config(&self, session_id: &str, vcpus: usize, memory_mb: u64) -> PartitionConfig {
        let safe_vcpus = vcpus.clamp(1, 32);
        let safe_memory = memory_mb.max(512);

        PartitionConfig {
            partition_id: format!("lxc-android-{}", session_id),
            adapter_name: self.name().to_string(),
            vcpu_count: safe_vcpus,
            memory_mb: safe_memory,
            enable_gpu_acceleration: true,
            virtio_vsock_cid: 0, // Direct UNIX domain socket transport on Linux
            virtio_vsock_port: 0,
            kernel_cmdline: "cgroup_enable=cpuset cgroup_memory=1 systemd.unified_cgroup_hierarchy=1".to_string(),
            mount_points: vec![
                PartitionMount {
                    host_path: "/dev/binderfs".to_string(),
                    guest_path: "/dev/binderfs".to_string(),
                    read_only: false,
                },
                PartitionMount {
                    host_path: format!("target/umr-sandbox/sessions/{}/data", session_id),
                    guest_path: "/data".to_string(),
                    read_only: false,
                },
            ],
        }
    }
}

