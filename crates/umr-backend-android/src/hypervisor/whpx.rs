use super::{IPartitionAdapter, PartitionConfig, PartitionMount};
use crate::host_detector::HostPlatformDetector;

/// Partition adapter for Windows Hypervisor Platform (WHPX).
/// Creates MicroVM partition specifications for running headless AOSP kernels on Windows hosts.
pub struct WhpxAdapter {
    is_whpx_available: bool,
}

impl WhpxAdapter {
    pub fn new() -> Self {
        let caps = HostPlatformDetector::detect_current();
        Self {
            is_whpx_available: caps.hardware_virtualization_available && caps.host_os == crate::host_detector::HostOs::Windows,
        }
    }

    pub fn with_availability(available: bool) -> Self {
        Self {
            is_whpx_available: available,
        }
    }
}

impl Default for WhpxAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl IPartitionAdapter for WhpxAdapter {
    fn name(&self) -> &str {
        "whpx-microvm-adapter"
    }

    fn is_supported(&self) -> bool {
        self.is_whpx_available
    }

    fn create_partition_config(&self, session_id: &str, vcpus: usize, memory_mb: u64) -> PartitionConfig {
        let safe_vcpus = vcpus.clamp(1, 16);
        let safe_memory = memory_mb.max(1024);

        PartitionConfig {
            partition_id: format!("whpx-part-{}", session_id),
            adapter_name: self.name().to_string(),
            vcpu_count: safe_vcpus,
            memory_mb: safe_memory,
            enable_gpu_acceleration: true,
            virtio_vsock_cid: 3,
            virtio_vsock_port: 5555,
            kernel_cmdline: "console=hvc0 root=/dev/vda rw init=/init androidboot.hardware=virtio androidboot.selinux=permissive qemu=1".to_string(),
            mount_points: vec![
                PartitionMount {
                    host_path: format!("target/umr-sandbox/sessions/{}/system", session_id),
                    guest_path: "/system".to_string(),
                    read_only: true,
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

