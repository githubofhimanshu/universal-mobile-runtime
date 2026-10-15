use super::{IPartitionAdapter, PartitionConfig, PartitionMount};
use crate::host_detector::HostPlatformDetector;

/// Partition adapter for macOS Virtualization.framework.
/// Configures MicroVM instances on macOS with Metal-backed graphics and Rosetta 2 directory share on Apple Silicon.
pub struct MacVirtAdapter {
    is_supported: bool,
    is_apple_silicon: bool,
}

impl MacVirtAdapter {
    pub fn new() -> Self {
        let caps = HostPlatformDetector::detect_current();
        Self {
            is_supported: caps.hardware_virtualization_available && caps.host_os == crate::host_detector::HostOs::MacOS,
            is_apple_silicon: caps.host_cpu == umr_core::CpuArchitecture::Arm64,
        }
    }

    pub fn with_settings(supported: bool, apple_silicon: bool) -> Self {
        Self {
            is_supported: supported,
            is_apple_silicon: apple_silicon,
        }
    }

    pub fn is_rosetta_directory_share_enabled(&self) -> bool {
        self.is_apple_silicon
    }
}

impl Default for MacVirtAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl IPartitionAdapter for MacVirtAdapter {
    fn name(&self) -> &str {
        "mac-virtualization-adapter"
    }

    fn is_supported(&self) -> bool {
        self.is_supported
    }

    fn create_partition_config(&self, session_id: &str, vcpus: usize, memory_mb: u64) -> PartitionConfig {
        let safe_vcpus = vcpus.clamp(1, 8);
        let safe_memory = memory_mb.max(2048);

        let mut mounts = vec![
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
        ];

        if self.is_apple_silicon {
            mounts.push(PartitionMount {
                host_path: "/Library/Apple/usr/libexec/oah/build".to_string(),
                guest_path: "/rosetta".to_string(),
                read_only: true,
            });
        }

        PartitionConfig {
            partition_id: format!("vz-android-{}", session_id),
            adapter_name: self.name().to_string(),
            vcpu_count: safe_vcpus,
            memory_mb: safe_memory,
            enable_gpu_acceleration: true,
            virtio_vsock_cid: 3,
            virtio_vsock_port: 5555,
            kernel_cmdline: "console=hvc0 root=/dev/vda rw init=/init androidboot.hardware=virtio androidboot.selinux=permissive vz.rosetta=1".to_string(),
            mount_points: mounts,
        }
    }
}

