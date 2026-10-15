use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use umr_core::{
    BackendCapabilities, DiagnosticError, ErrorCode, FrameBuffer, IAndroidBackend, InputEvent,
    LifecycleState, PackageDetails, PackageMetadata, SessionRecord, UmrResult,
};

use crate::host_detector::{HostCapabilities, HostPlatformDetector};
use crate::hypervisor::{
    IPartitionAdapter, LinuxContainerAdapter, MacVirtAdapter, PartitionConfig, WhpxAdapter,
};
use crate::package_manager::AndroidPackageManager;
use crate::transport::{AdbTransport, GuestTransport, MockGuestTransport};
use crate::watchdog::WatchdogSupervisor;

/// Production Android backend supporting cross-host virtualization and container partitions.
pub struct AndroidVirtualizationBackend {
    capabilities: BackendCapabilities,
    sandbox_root: PathBuf,
    package_manager: Arc<Mutex<AndroidPackageManager>>,
    transport: Arc<Mutex<Box<dyn GuestTransport>>>,
    watchdog: WatchdogSupervisor,
    partition_config: Option<PartitionConfig>,
}

impl AndroidVirtualizationBackend {
    /// Creates a production backend automatically detecting the current host environment.
    pub fn new_auto(sandbox_root: &Path) -> UmrResult<Self> {
        let host_caps = HostPlatformDetector::detect_current();
        Self::from_host_caps(sandbox_root, host_caps)
    }

    /// Creates a backend using custom host capabilities for deterministic testing and cross-host simulation.
    pub fn from_host_caps(sandbox_root: &Path, host_caps: HostCapabilities) -> UmrResult<Self> {
        let backend_id = format!("android-virt-{}", host_caps.host_os.to_string().to_lowercase());
        let backend_caps = BackendCapabilities {
            backend_id: backend_id.clone(),
            backend_version: "0.2.0".to_string(),
            execution_mode: host_caps.recommended_execution_mode,
            supported_abis: host_caps.supported_guest_abis,
            supported_architectures: vec![host_caps.host_cpu],
            min_api_level: Some(21),
            max_api_level: Some(34),
            requires_hardware_virtualization: host_caps.hardware_virtualization_available,
            requires_external_device: false,
        };

        let pkg_mgr = AndroidPackageManager::new(sandbox_root)?;

        // Default transport selection based on host environment
        let transport: Box<dyn GuestTransport> = if host_caps.hardware_virtualization_available {
            Box::new(AdbTransport::default_local())
        } else {
            Box::new(MockGuestTransport::new())
        };

        Ok(Self {
            capabilities: backend_caps,
            sandbox_root: sandbox_root.to_path_buf(),
            package_manager: Arc::new(Mutex::new(pkg_mgr)),
            transport: Arc::new(Mutex::new(transport)),
            watchdog: WatchdogSupervisor::default_monitor(),
            partition_config: None,
        })
    }

    /// Creates a backend explicitly wired with a custom transport (e.g. for testing).
    pub fn with_transport(
        sandbox_root: &Path,
        transport: Box<dyn GuestTransport>,
    ) -> UmrResult<Self> {
        let host_caps = HostPlatformDetector::detect_current();
        let mut backend = Self::from_host_caps(sandbox_root, host_caps)?;
        backend.transport = Arc::new(Mutex::new(transport));
        Ok(backend)
    }

    pub fn package_manager(&self) -> Arc<Mutex<AndroidPackageManager>> {
        self.package_manager.clone()
    }

    pub fn partition_config(&self) -> Option<&PartitionConfig> {
        self.partition_config.as_ref()
    }

    pub fn sandbox_root(&self) -> &Path {
        &self.sandbox_root
    }
}

impl IAndroidBackend for AndroidVirtualizationBackend {
    fn id(&self) -> &str {
        &self.capabilities.backend_id
    }

    fn capabilities(&self) -> &BackendCapabilities {
        &self.capabilities
    }

    fn prepare(&mut self, session: &mut SessionRecord, package: &PackageMetadata) -> UmrResult<()> {
        let _pkg_info = match &package.details {
            PackageDetails::Android(info) => info,
            _ => {
                return Err(DiagnosticError::new(
                    ErrorCode::ErrIntakeParseFailure,
                    "Expected Android package metadata",
                ));
            }
        };

        // 1. Install or update package in persistent catalog
        let mut mgr = self.package_manager.lock().map_err(|_| {
            DiagnosticError::new(ErrorCode::ErrBackendInternal, "Package manager lock poisoned")
        })?;
        let _record = mgr.install_package(package)?;

        // 2. Formulate partition configuration based on host OS
        let session_id_str = session.session_id.to_string();
        let host_caps = HostPlatformDetector::detect_current();
        let part_config = match host_caps.host_os {
            crate::host_detector::HostOs::Windows => {
                let adapter = WhpxAdapter::new();
                adapter.create_partition_config(&session_id_str, 2, 2048)
            }
            crate::host_detector::HostOs::Linux => {
                let adapter = LinuxContainerAdapter::new();
                adapter.create_partition_config(&session_id_str, 4, 1024)
            }
            crate::host_detector::HostOs::MacOS => {
                let adapter = MacVirtAdapter::new();
                adapter.create_partition_config(&session_id_str, 2, 2048)
            }
            crate::host_detector::HostOs::Unknown => {
                let adapter = WhpxAdapter::new();
                adapter.create_partition_config(&session_id_str, 2, 2048)
            }
        };
        self.partition_config = Some(part_config);

        Ok(())
    }

    fn launch(&mut self, session: &mut SessionRecord) -> UmrResult<()> {
        let t = self.transport.lock().map_err(|_| {
            DiagnosticError::new(ErrorCode::ErrBackendInternal, "Transport lock poisoned")
        })?;

        // Launch target activity via transport
        let cmd = format!("am start -n {}/MainActivity", session.package_id);
        let output = t.execute_command(&cmd)?;
        if !output.is_success() {
            return Err(DiagnosticError::new(
                ErrorCode::ErrBackendSessionFailed,
                format!("Failed to launch activity: {}", output.stderr),
            ));
        }

        // Record launch in package manager and initialize watchdog
        if let Ok(mut mgr) = self.package_manager.lock() {
            mgr.record_launch(&session.package_id);
        }
        self.watchdog.set_active_pid(1001);
        Ok(())
    }

    fn poll_status(&self, session: &SessionRecord) -> UmrResult<LifecycleState> {
        let t = self.transport.lock().map_err(|_| {
            DiagnosticError::new(ErrorCode::ErrBackendInternal, "Transport lock poisoned")
        })?;

        // 1. Health check transport
        if !t.is_alive() {
            let logs = t.stream_logs().unwrap_or_default();
            if let Some(crash) = self.watchdog.inspect_crash_logs(&logs) {
                return Err(DiagnosticError::new(
                    ErrorCode::ErrBackendProcessCrashed,
                    format!("Guest process crashed: {}", crash.reason),
                ));
            }
            return Ok(LifecycleState::Terminated);
        }

        // 2. Check ANR watchdog
        if let Some(anr_err) = self.watchdog.check_anr() {
            return Err(anr_err);
        }

        Ok(session.state)
    }

    fn stop(&mut self, session: &mut SessionRecord) -> UmrResult<()> {
        let t = self.transport.lock().map_err(|_| {
            DiagnosticError::new(ErrorCode::ErrBackendInternal, "Transport lock poisoned")
        })?;

        let cmd = format!("am force-stop {}", session.package_id);
        let _ = t.execute_command(&cmd);
        Ok(())
    }

    fn collect_diagnostics(&self, _session: &SessionRecord) -> UmrResult<Vec<String>> {
        let t = self.transport.lock().map_err(|_| {
            DiagnosticError::new(ErrorCode::ErrBackendInternal, "Transport lock poisoned")
        })?;
        t.stream_logs()
    }

    fn send_input(&mut self, _session: &SessionRecord, event: InputEvent) -> UmrResult<()> {
        self.watchdog.record_heartbeat();
        let t = self.transport.lock().map_err(|_| {
            DiagnosticError::new(ErrorCode::ErrBackendInternal, "Transport lock poisoned")
        })?;
        t.send_input(&event)
    }

    fn capture_frame(&self, _session: &SessionRecord) -> UmrResult<Option<FrameBuffer>> {
        let t = self.transport.lock().map_err(|_| {
            DiagnosticError::new(ErrorCode::ErrBackendInternal, "Transport lock poisoned")
        })?;
        t.capture_frame()
    }

    fn wipe_storage(&mut self, session: &SessionRecord) -> UmrResult<()> {
        let mut mgr = self.package_manager.lock().map_err(|_| {
            DiagnosticError::new(ErrorCode::ErrBackendInternal, "Package manager lock poisoned")
        })?;
        mgr.wipe_storage(&session.package_id)
    }
}
