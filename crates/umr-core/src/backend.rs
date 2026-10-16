use serde::{Deserialize, Serialize};

use crate::error::{DiagnosticError, ErrorCode, UmrResult};
use crate::framebuffer::FrameBuffer;
use crate::input::InputEvent;
use crate::lifecycle::{LifecycleState, SessionRecord};
use crate::metadata::{CpuArchitecture, EncryptionStatus, PackageDetails, PackageMetadata, TargetAbi};

/// Classification of application execution mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionMode {
    /// Direct host execution with containerization or native libraries.
    LocalNative,
    /// Hardware-accelerated MicroVM or hypervisor partition (WHPX / Virtualization.framework).
    LocalVirtualized,
    /// Dynamic binary translation for foreign architecture (e.g. QEMU TCG).
    LocalTranslated,
    /// High-Level Emulation for narrow legacy cohorts (e.g. touchHLE for early 32-bit iOS).
    LocalHle,
    /// Execution on tethered physical device or remote runner, streamed over network/USB.
    /// MUST ALWAYS be explicitly labeled as remote in UI and logs.
    RemoteStream,
    /// Unsupported configuration.
    Unsupported,
}

impl std::fmt::Display for ExecutionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LocalNative => write!(f, "Local (Native)"),
            Self::LocalVirtualized => write!(f, "Local (Virtualized)"),
            Self::LocalTranslated => write!(f, "Local (Translated)"),
            Self::LocalHle => write!(f, "Local (High-Level Emulator)"),
            Self::RemoteStream => write!(f, "Remote / Device Stream"),
            Self::Unsupported => write!(f, "Unsupported"),
        }
    }
}

/// Static capabilities advertised by an execution backend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackendCapabilities {
    pub backend_id: String,
    pub backend_version: String,
    pub execution_mode: ExecutionMode,
    pub supported_abis: Vec<TargetAbi>,
    pub supported_architectures: Vec<CpuArchitecture>,
    pub min_api_level: Option<u32>,
    pub max_api_level: Option<u32>,
    pub requires_hardware_virtualization: bool,
    pub requires_external_device: bool,
}

/// Abstract contract for Android execution backends.
pub trait IAndroidBackend: Send + Sync {
    fn id(&self) -> &str;
    fn capabilities(&self) -> &BackendCapabilities;
    fn prepare(&mut self, session: &mut SessionRecord, package: &PackageMetadata) -> UmrResult<()>;
    fn launch(&mut self, session: &mut SessionRecord) -> UmrResult<()>;
    fn poll_status(&self, session: &SessionRecord) -> UmrResult<LifecycleState>;
    fn stop(&mut self, session: &mut SessionRecord) -> UmrResult<()>;
    fn collect_diagnostics(&self, session: &SessionRecord) -> UmrResult<Vec<String>>;

    /// Dispatches an input event (pointer touch, keycode, or navigation) to the active session.
    fn send_input(&mut self, session: &SessionRecord, event: InputEvent) -> UmrResult<()> {
        let _ = (session, event);
        Ok(())
    }

    /// Captures the latest generated frame buffer from the active session.
    fn capture_frame(&self, session: &SessionRecord) -> UmrResult<Option<FrameBuffer>> {
        let _ = session;
        Ok(None)
    }

    /// Completely purges user and cache data for the application in this session.
    fn wipe_storage(&mut self, session: &SessionRecord) -> UmrResult<()> {
        let _ = session;
        Ok(())
    }
}

/// Abstract contract for iOS execution backends.
pub trait IIosBackend: Send + Sync {
    fn id(&self) -> &str;
    fn capabilities(&self) -> &BackendCapabilities;
    fn prepare(&mut self, session: &mut SessionRecord, package: &PackageMetadata) -> UmrResult<()>;
    fn launch(&mut self, session: &mut SessionRecord) -> UmrResult<()>;
    fn poll_status(&self, session: &SessionRecord) -> UmrResult<LifecycleState>;
    fn stop(&mut self, session: &mut SessionRecord) -> UmrResult<()>;
    fn collect_diagnostics(&self, session: &SessionRecord) -> UmrResult<Vec<String>>;

    /// Dispatches an input event to the active session.
    fn send_input(&mut self, session: &SessionRecord, event: InputEvent) -> UmrResult<()> {
        let _ = (session, event);
        Ok(())
    }

    /// Captures the latest generated frame buffer from the active session.
    fn capture_frame(&self, session: &SessionRecord) -> UmrResult<Option<FrameBuffer>> {
        let _ = session;
        Ok(None)
    }
}

/// Result of negotiating package requirements against available backends.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum NegotiationResult {
    /// A compatible backend is available for local execution.
    Compatible {
        backend_id: String,
        execution_mode: ExecutionMode,
    },
    /// The application requires an external connected device or remote Mac.
    RemoteRequired {
        reason: String,
        suggested_mode: ExecutionMode,
    },
    /// The application is rejected or completely unsupported on this host.
    Incompatible {
        error: DiagnosticError,
    },
}

/// Capability negotiator matching inspected package requirements to registered backends.
pub struct CapabilityNegotiator;

impl CapabilityNegotiator {
    /// Negotiates an appropriate execution strategy for an inspected package.
    pub fn negotiate(
        package: &PackageMetadata,
        android_backends: &[&dyn IAndroidBackend],
        ios_backends: &[&dyn IIosBackend],
        is_macos_host: bool,
    ) -> NegotiationResult {
        match &package.details {
            PackageDetails::Android(info) => {
                let mut compatible_candidates = Vec::new();

                // Check all registered Android backends and rank by execution tier
                for backend in android_backends {
                    let caps = backend.capabilities();

                    // Check API levels if specified
                    if let Some(min_req) = info.min_sdk_version {
                        if let Some(backend_max) = caps.max_api_level {
                            if min_req > backend_max {
                                continue;
                            }
                        }
                    }

                    // Check ABI compatibility:
                    // If pure DEX (no native libs), any Android backend matches.
                    // If native libraries exist, check if backend supports at least one ABI.
                    let matches_abi = info.is_pure_dex
                        || info.native_abis.is_empty()
                        || info.native_abis.iter().any(|abi| caps.supported_abis.contains(abi));

                    if matches_abi {
                        let rank = match caps.execution_mode {
                            ExecutionMode::LocalNative => 1,
                            ExecutionMode::LocalVirtualized => 2,
                            ExecutionMode::LocalTranslated => 3,
                            ExecutionMode::LocalHle => 4,
                            ExecutionMode::RemoteStream => 5,
                            ExecutionMode::Unsupported => 99,
                        };
                        compatible_candidates.push((rank, caps.backend_id.clone(), caps.execution_mode));
                    }
                }

                if !compatible_candidates.is_empty() {
                    compatible_candidates.sort_by_key(|c| c.0);
                    let (_, best_backend_id, best_mode) = compatible_candidates.remove(0);
                    return NegotiationResult::Compatible {
                        backend_id: best_backend_id,
                        execution_mode: best_mode,
                    };
                }

                NegotiationResult::Incompatible {
                    error: DiagnosticError::new(
                        ErrorCode::ErrBackendIncompatibleArchitecture,
                        format!(
                            "No registered Android backend supports native ABIs [{:?}]",
                            info.native_abis
                        ),
                    )
                    .with_suggestion("Provide an APK containing x86_64 or pure Dex code, or enable translation."),
                }
            }

            PackageDetails::Ios(info) => {
                // 1. Check for FairPlay encryption
                if let EncryptionStatus::FairPlayEncrypted { cryptid } = info.encryption {
                    return NegotiationResult::Incompatible {
                        error: DiagnosticError::drm_protected(&info.bundle_identifier)
                            .with_context(format!("FairPlay cryptid: {}", cryptid)),
                    };
                }

                // 2. Track A (macOS Simulator)
                if is_macos_host && info.is_simulator_build {
                    for backend in ios_backends {
                        let caps = backend.capabilities();
                        if caps.execution_mode == ExecutionMode::LocalNative || caps.execution_mode == ExecutionMode::LocalVirtualized {
                            return NegotiationResult::Compatible {
                                backend_id: caps.backend_id.clone(),
                                execution_mode: ExecutionMode::LocalNative,
                            };
                        }
                    }
                }

                // 3. Track B (32-bit Legacy HLE)
                let is_legacy_32bit = info.architectures.contains(&CpuArchitecture::Armv7)
                    && !info.architectures.contains(&CpuArchitecture::Arm64);
                if is_legacy_32bit {
                    for backend in ios_backends {
                        let caps = backend.capabilities();
                        if caps.execution_mode == ExecutionMode::LocalHle {
                            return NegotiationResult::Compatible {
                                backend_id: caps.backend_id.clone(),
                                execution_mode: ExecutionMode::LocalHle,
                            };
                        }
                    }
                }

                // 4. Non-macOS host or modern iOS on non-Mac -> Requires Remote/Tethered Device
                if !is_macos_host {
                    NegotiationResult::RemoteRequired {
                        reason: format!(
                            "Modern iOS app '{}' cannot execute locally on non-Apple desktop hosts. Tethered physical device or remote runner required.",
                            info.bundle_identifier
                        ),
                        suggested_mode: ExecutionMode::RemoteStream,
                    }
                } else {
                    NegotiationResult::Incompatible {
                        error: DiagnosticError::new(
                            ErrorCode::ErrBackendUnavailable,
                            format!(
                                "No compatible iOS backend available for bundle '{}' on this macOS configuration",
                                info.bundle_identifier
                            ),
                        )
                        .with_suggestion("Ensure Xcode Simulator runtime is installed and package is built for simulator SDK."),
                    }
                }
            }
        }
    }
}

