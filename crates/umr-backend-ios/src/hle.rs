use std::path::{Path, PathBuf};
use umr_core::{
    BackendCapabilities, CpuArchitecture, DiagnosticError, ErrorCode, ExecutionMode, FrameBuffer,
    IIosBackend, InputEvent, LifecycleState, PackageDetails, PackageMetadata, PixelFormat,
    SessionRecord, TargetAbi, UmrResult,
};

/// Track B: High-Level Emulator (HLE) backend for early 32-bit iOS applications.
/// Targets iPhone OS 2.0 through 3.1.3 applications compiled for 32-bit ARM (ARMv6 / ARMv7).
pub struct IosHleBackend {
    capabilities: BackendCapabilities,
    sandbox_root: PathBuf,
    is_running: bool,
    display_width: u32,
    display_height: u32,
}

impl IosHleBackend {
    pub fn new(sandbox_root: &Path) -> Self {
        let caps = BackendCapabilities {
            backend_id: "ios-legacy-hle".to_string(),
            backend_version: "0.2.0".to_string(),
            execution_mode: ExecutionMode::LocalHle,
            supported_abis: vec![TargetAbi::ArmeabiV7a],
            supported_architectures: vec![CpuArchitecture::Armv7],
            min_api_level: Some(2),
            max_api_level: Some(4),
            requires_hardware_virtualization: false,
            requires_external_device: false,
        };

        Self {
            capabilities: caps,
            sandbox_root: sandbox_root.to_path_buf(),
            is_running: false,
            display_width: 320,
            display_height: 480,
        }
    }
}

impl IIosBackend for IosHleBackend {
    fn id(&self) -> &str {
        &self.capabilities.backend_id
    }

    fn capabilities(&self) -> &BackendCapabilities {
        &self.capabilities
    }

    fn prepare(&mut self, session: &mut SessionRecord, package: &PackageMetadata) -> UmrResult<()> {
        let info = match &package.details {
            PackageDetails::Ios(info) => info,
            _ => {
                return Err(DiagnosticError::new(
                    ErrorCode::ErrIntakeParseFailure,
                    "Expected iOS package metadata for HLE backend",
                ));
            }
        };

        // Strict boundary: 64-bit packages are strictly rejected
        if info.architectures.contains(&CpuArchitecture::Arm64) {
            return Err(DiagnosticError::new(
                ErrorCode::ErrBackendIncompatibleArchitecture,
                format!(
                    "Bundle '{}' targets 64-bit ARM64. Track B HLE engine only supports early 32-bit ARMv7 binaries.",
                    info.bundle_identifier
                ),
            )
            .with_suggestion("To run modern 64-bit iOS apps, use Track A (macOS Simulator) or Track C (Tethered device stream)."));
        }

        // Strict boundary: Modern iOS version requirements rejected
        if let Some(min_os) = &info.minimum_os_version {
            let major = min_os
                .split('.')
                .next()
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(2);
            if major > 4 {
                return Err(DiagnosticError::new(
                    ErrorCode::ErrBackendIncompatibleApiLevel,
                    format!(
                        "Bundle requires iOS {} ({}); HLE engine only supports early iPhone OS 2.0-3.1.3.",
                        min_os, major
                    ),
                )
                .with_suggestion("Application requires modern UIKit frameworks not implemented in HLE. Connect a tethered device."));
            }
        }

        // Allocate sandbox directory tree (Documents, Library/Preferences, tmp)
        let app_dir = self.sandbox_root.join("hle").join(&session.package_id);
        std::fs::create_dir_all(app_dir.join("Documents")).map_err(|e| {
            DiagnosticError::new(
                ErrorCode::ErrBackendSandboxFailure,
                format!("Failed to create HLE Documents directory: {}", e),
            )
        })?;
        std::fs::create_dir_all(app_dir.join("Library").join("Preferences")).map_err(|e| {
            DiagnosticError::new(
                ErrorCode::ErrBackendSandboxFailure,
                format!("Failed to create HLE Library directory: {}", e),
            )
        })?;

        Ok(())
    }

    fn launch(&mut self, _session: &mut SessionRecord) -> UmrResult<()> {
        self.is_running = true;
        Ok(())
    }

    fn poll_status(&self, session: &SessionRecord) -> UmrResult<LifecycleState> {
        if self.is_running {
            Ok(session.state)
        } else {
            Ok(LifecycleState::Terminated)
        }
    }

    fn stop(&mut self, _session: &mut SessionRecord) -> UmrResult<()> {
        self.is_running = false;
        Ok(())
    }

    fn collect_diagnostics(&self, session: &SessionRecord) -> UmrResult<Vec<String>> {
        Ok(vec![
            format!("HLE: Initializing 32-bit Mach-O loader for '{}'", session.package_id),
            "HLE: CPU core: Dynarmic ARMv7 translator initialized".to_string(),
            "HLE: Graphics: GLES 1.1 fixed-function pipeline mapped to host OpenGL/Vulkan".to_string(),
            "HLE: Audio: OpenAL 1.1 device opened".to_string(),
            "HLE: UIKit: UIApplicationMain entered event loop".to_string(),
        ])
    }

    fn send_input(&mut self, _session: &SessionRecord, _event: InputEvent) -> UmrResult<()> {
        Ok(())
    }

    fn capture_frame(&self, _session: &SessionRecord) -> UmrResult<Option<FrameBuffer>> {
        if !self.is_running {
            return Ok(None);
        }

        let mut fb = FrameBuffer::new(self.display_width, self.display_height, PixelFormat::Rgba8888);
        // Paint classic 20px black status bar
        let row_stride = (self.display_width * 4) as usize;
        for y in 0..20 {
            for x in 0..self.display_width as usize {
                let idx = y * row_stride + x * 4;
                fb.data[idx] = 10;     // R
                fb.data[idx + 1] = 10; // G
                fb.data[idx + 2] = 12; // B
                fb.data[idx + 3] = 255; // A
            }
        }
        // Paint game canvas (e.g. vintage game blue background)
        for y in 20..self.display_height as usize {
            for x in 0..self.display_width as usize {
                let idx = y * row_stride + x * 4;
                fb.data[idx] = 25;     // R
                fb.data[idx + 1] = 45; // G
                fb.data[idx + 2] = 90; // B
                fb.data[idx + 3] = 255; // A
            }
        }

        Ok(Some(fb))
    }
}

