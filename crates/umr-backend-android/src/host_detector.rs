use serde::{Deserialize, Serialize};
use std::env;
use umr_core::{CpuArchitecture, ExecutionMode, TargetAbi};

/// Classification of host operating system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostOs {
    Windows,
    Linux,
    MacOS,
    Unknown,
}

impl std::fmt::Display for HostOs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Windows => write!(f, "Windows"),
            Self::Linux => write!(f, "Linux"),
            Self::MacOS => write!(f, "macOS"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

/// The hypervisor or isolation driver available on the host.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HypervisorKind {
    /// Windows Hypervisor Platform (WHPX / WinHvPlatform.dll)
    Whpx,
    /// Linux Kernel-based Virtual Machine (/dev/kvm)
    Kvm,
    /// Linux native container isolation (namespaces + cgroups v2 + binderfs)
    LinuxContainer,
    /// Apple Virtualization.framework
    MacVirtualization,
    /// Fallback reference / software runner
    ReferenceSoftware,
}

impl std::fmt::Display for HypervisorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Whpx => write!(f, "Windows Hypervisor Platform (WHPX)"),
            Self::Kvm => write!(f, "Kernel-based Virtual Machine (KVM)"),
            Self::LinuxContainer => write!(f, "Linux Namespaces / cgroups v2 / Binderfs"),
            Self::MacVirtualization => write!(f, "Apple Virtualization.framework"),
            Self::ReferenceSoftware => write!(f, "Reference Local Runner"),
        }
    }
}

/// Host virtualization and hardware feature inventory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostCapabilities {
    pub host_os: HostOs,
    pub host_cpu: CpuArchitecture,
    pub hypervisor_kind: HypervisorKind,
    pub hardware_virtualization_available: bool,
    pub container_isolation_available: bool,
    pub logical_cpu_cores: usize,
    pub total_memory_mb: u64,
    pub recommended_execution_mode: ExecutionMode,
    pub supported_guest_abis: Vec<TargetAbi>,
}

/// System probe that identifies host OS, architecture, and virtualization primitives.
pub struct HostPlatformDetector;

impl HostPlatformDetector {
    /// Detects host capabilities for the currently running environment.
    pub fn detect_current() -> HostCapabilities {
        let host_os = Self::detect_os();
        let host_cpu = Self::detect_cpu();
        let logical_cpu_cores = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        
        let (hypervisor_kind, hw_virt, container_avail, exec_mode, abis) = match host_os {
            HostOs::Windows => {
                let whpx_avail = Self::probe_windows_whpx();
                let exec_mode = if whpx_avail {
                    ExecutionMode::LocalVirtualized
                } else {
                    ExecutionMode::LocalNative
                };
                let kind = if whpx_avail {
                    HypervisorKind::Whpx
                } else {
                    HypervisorKind::ReferenceSoftware
                };
                let abis = if host_cpu == CpuArchitecture::Arm64 {
                    vec![TargetAbi::Arm64V8a, TargetAbi::ArmeabiV7a]
                } else {
                    vec![
                        TargetAbi::X86_64,
                        TargetAbi::X86,
                        TargetAbi::Arm64V8a,
                        TargetAbi::ArmeabiV7a,
                    ]
                };
                (kind, whpx_avail, false, exec_mode, abis)
            }
            HostOs::Linux => {
                let binderfs_avail = Self::probe_linux_binderfs();
                let kvm_avail = Self::probe_linux_kvm();
                let (kind, exec_mode) = if binderfs_avail {
                    (HypervisorKind::LinuxContainer, ExecutionMode::LocalNative)
                } else if kvm_avail {
                    (HypervisorKind::Kvm, ExecutionMode::LocalVirtualized)
                } else {
                    (HypervisorKind::ReferenceSoftware, ExecutionMode::LocalNative)
                };
                let abis = if host_cpu == CpuArchitecture::Arm64 {
                    vec![TargetAbi::Arm64V8a, TargetAbi::ArmeabiV7a]
                } else {
                    vec![
                        TargetAbi::X86_64,
                        TargetAbi::X86,
                        TargetAbi::Arm64V8a,
                        TargetAbi::ArmeabiV7a,
                    ]
                };
                (kind, kvm_avail, binderfs_avail, exec_mode, abis)
            }
            HostOs::MacOS => {
                let mac_virt_avail = Self::probe_macos_virt();
                let exec_mode = if mac_virt_avail {
                    ExecutionMode::LocalVirtualized
                } else {
                    ExecutionMode::LocalNative
                };
                let kind = if mac_virt_avail {
                    HypervisorKind::MacVirtualization
                } else {
                    HypervisorKind::ReferenceSoftware
                };
                let mut abis = if host_cpu == CpuArchitecture::Arm64 {
                    // Apple Silicon can also execute x86_64 via Rosetta 2 directory sharing
                    vec![TargetAbi::Arm64V8a, TargetAbi::ArmeabiV7a, TargetAbi::X86_64]
                } else {
                    vec![TargetAbi::X86_64, TargetAbi::X86]
                };
                abis.dedup();
                (kind, mac_virt_avail, false, exec_mode, abis)
            }
            HostOs::Unknown => (
                HypervisorKind::ReferenceSoftware,
                false,
                false,
                ExecutionMode::LocalNative,
                vec![TargetAbi::X86_64],
            ),
        };

        // Estimate total memory conservatively (at least 8192 MB on modern dev machines)
        let total_memory_mb = 8192;

        HostCapabilities {
            host_os,
            host_cpu,
            hypervisor_kind,
            hardware_virtualization_available: hw_virt,
            container_isolation_available: container_avail,
            logical_cpu_cores,
            total_memory_mb,
            recommended_execution_mode: exec_mode,
            supported_guest_abis: abis,
        }
    }

    /// Evaluates custom host parameters for cross-platform simulation and testing.
    pub fn evaluate_custom(
        host_os: HostOs,
        host_cpu: CpuArchitecture,
        hw_virt_available: bool,
        container_available: bool,
        logical_cores: usize,
    ) -> HostCapabilities {
        let (hypervisor_kind, exec_mode, abis) = match host_os {
            HostOs::Windows => {
                if hw_virt_available {
                    (
                        HypervisorKind::Whpx,
                        ExecutionMode::LocalVirtualized,
                        if host_cpu == CpuArchitecture::Arm64 {
                            vec![TargetAbi::Arm64V8a, TargetAbi::ArmeabiV7a]
                        } else {
                            vec![TargetAbi::X86_64, TargetAbi::X86]
                        },
                    )
                } else {
                    (
                        HypervisorKind::ReferenceSoftware,
                        ExecutionMode::LocalNative,
                        vec![TargetAbi::X86_64],
                    )
                }
            }
            HostOs::Linux => {
                if container_available {
                    (
                        HypervisorKind::LinuxContainer,
                        ExecutionMode::LocalNative,
                        if host_cpu == CpuArchitecture::Arm64 {
                            vec![TargetAbi::Arm64V8a, TargetAbi::ArmeabiV7a]
                        } else {
                            vec![TargetAbi::X86_64, TargetAbi::X86]
                        },
                    )
                } else if hw_virt_available {
                    (
                        HypervisorKind::Kvm,
                        ExecutionMode::LocalVirtualized,
                        if host_cpu == CpuArchitecture::Arm64 {
                            vec![TargetAbi::Arm64V8a]
                        } else {
                            vec![TargetAbi::X86_64]
                        },
                    )
                } else {
                    (
                        HypervisorKind::ReferenceSoftware,
                        ExecutionMode::LocalNative,
                        vec![TargetAbi::X86_64],
                    )
                }
            }
            HostOs::MacOS => {
                if hw_virt_available {
                    (
                        HypervisorKind::MacVirtualization,
                        ExecutionMode::LocalVirtualized,
                        if host_cpu == CpuArchitecture::Arm64 {
                            vec![TargetAbi::Arm64V8a, TargetAbi::ArmeabiV7a, TargetAbi::X86_64]
                        } else {
                            vec![TargetAbi::X86_64, TargetAbi::X86]
                        },
                    )
                } else {
                    (
                        HypervisorKind::ReferenceSoftware,
                        ExecutionMode::LocalNative,
                        vec![TargetAbi::X86_64],
                    )
                }
            }
            HostOs::Unknown => (
                HypervisorKind::ReferenceSoftware,
                ExecutionMode::LocalNative,
                vec![TargetAbi::X86_64],
            ),
        };

        HostCapabilities {
            host_os,
            host_cpu,
            hypervisor_kind,
            hardware_virtualization_available: hw_virt_available,
            container_isolation_available: container_available,
            logical_cpu_cores: logical_cores,
            total_memory_mb: 8192,
            recommended_execution_mode: exec_mode,
            supported_guest_abis: abis,
        }
    }

    fn detect_os() -> HostOs {
        if cfg!(target_os = "windows") {
            HostOs::Windows
        } else if cfg!(target_os = "linux") {
            HostOs::Linux
        } else if cfg!(target_os = "macos") {
            HostOs::MacOS
        } else {
            HostOs::Unknown
        }
    }

    fn detect_cpu() -> CpuArchitecture {
        match env::consts::ARCH {
            "x86_64" => CpuArchitecture::X86_64,
            "aarch64" => CpuArchitecture::Arm64,
            "arm" => CpuArchitecture::Armv7,
            _ => CpuArchitecture::Unknown,
        }
    }

    fn probe_windows_whpx() -> bool {
        #[cfg(target_os = "windows")]
        {
            // On Windows 10/11, WHPX is provided by WinHvPlatform.dll.
            // Check if the system library exists in System32.
            let system_root = env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string());
            let dll_path = std::path::Path::new(&system_root)
                .join("System32")
                .join("WinHvPlatform.dll");
            dll_path.exists()
        }
        #[cfg(not(target_os = "windows"))]
        {
            false
        }
    }

    fn probe_linux_binderfs() -> bool {
        #[cfg(target_os = "linux")]
        {
            std::path::Path::new("/dev/binderfs").exists() || std::path::Path::new("/dev/binder").exists()
        }
        #[cfg(not(target_os = "linux"))]
        {
            false
        }
    }

    fn probe_linux_kvm() -> bool {
        #[cfg(target_os = "linux")]
        {
            std::path::Path::new("/dev/kvm").exists()
        }
        #[cfg(not(target_os = "linux"))]
        {
            false
        }
    }

    fn probe_macos_virt() -> bool {
        #[cfg(target_os = "macos")]
        {
            // Virtualization.framework is standard on macOS 11+
            true
        }
        #[cfg(not(target_os = "macos"))]
        {
            false
        }
    }
}
