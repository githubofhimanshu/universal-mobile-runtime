pub mod logger;
pub mod registry;
pub mod session;
pub mod supervisor;

use std::path::{Path, PathBuf};
use std::time::Duration;

use umr_core::{
    BackendCapabilities, DiagnosticError, ErrorCode, FrameBuffer, IAndroidBackend,
    IIosBackend, InputEvent, LifecycleState, NegotiationResult, PackageMetadata, SessionId,
    UmrResult,
};

pub use logger::{DiagnosticLogEntry, DiagnosticReport, LogLevel, LogSanitizer};
pub use registry::BackendRegistry;
pub use session::{ActiveSession, SessionManager};
pub use supervisor::{execute_sandboxed_intake, find_intake_binary, DEFAULT_INTAKE_TIMEOUT};

/// The primary UMR core service coordinating package intake, capability negotiation, and runtime execution.
pub struct RuntimeOrchestrator {
    intake_bin: PathBuf,
    intake_timeout: Duration,
    registry: BackendRegistry,
    sessions: SessionManager,
    is_macos: bool,
}

impl RuntimeOrchestrator {
    pub fn new(intake_bin: PathBuf) -> Self {
        Self {
            intake_bin,
            intake_timeout: DEFAULT_INTAKE_TIMEOUT,
            registry: BackendRegistry::new(),
            sessions: SessionManager::new(),
            is_macos: cfg!(target_os = "macos"),
        }
    }

    pub fn with_macos_override(mut self, is_macos: bool) -> Self {
        self.is_macos = is_macos;
        self
    }

    pub fn with_intake_timeout(mut self, timeout: Duration) -> Self {
        self.intake_timeout = timeout;
        self
    }

    pub fn register_android(&mut self, backend: Box<dyn IAndroidBackend>) {
        self.registry.register_android(backend);
    }

    pub fn register_ios(&mut self, backend: Box<dyn IIosBackend>) {
        self.registry.register_ios(backend);
    }

    pub fn list_backends(&self) -> Vec<BackendCapabilities> {
        self.registry.list_capabilities()
    }

    /// Spawns the isolated intake worker to defensively inspect a package archive.
    pub fn inspect_package(&self, package_path: &Path) -> UmrResult<PackageMetadata> {
        execute_sandboxed_intake(&self.intake_bin, package_path, self.intake_timeout)
    }

    /// Evaluates inspected package requirements against registered host backends.
    pub fn negotiate(&self, package: &PackageMetadata) -> NegotiationResult {
        self.registry.negotiate(package, self.is_macos)
    }

    /// Prepares an application for launch by initializing a session and dispatching to the selected backend.
    pub fn prepare_session(
        &mut self,
        package: PackageMetadata,
        backend_id: &str,
    ) -> UmrResult<SessionId> {
        // Find matching backend
        if let Some(backend) = self.registry.get_android_mut(backend_id) {
            let exec_mode = backend.capabilities().execution_mode;
            let session_id = self.sessions.create_session(package.clone(), backend_id.to_string(), exec_mode);
            let session = self.sessions.get_mut(&session_id)?;

            session.record.transition_to(LifecycleState::Preparing)?;
            session.log(LogLevel::Info, "orchestrator", format!("Preparing session with backend '{}'", backend_id));

            if let Err(e) = backend.prepare(&mut session.record, &package) {
                session.record.mark_failed(e.clone());
                session.log(LogLevel::Error, "backend", format!("Preparation failed: {}", e));
                return Err(e);
            }

            Ok(session_id)
        } else if let Some(backend) = self.registry.get_ios_mut(backend_id) {
            let exec_mode = backend.capabilities().execution_mode;
            let session_id = self.sessions.create_session(package.clone(), backend_id.to_string(), exec_mode);
            let session = self.sessions.get_mut(&session_id)?;

            session.record.transition_to(LifecycleState::Preparing)?;
            session.log(LogLevel::Info, "orchestrator", format!("Preparing iOS session with backend '{}'", backend_id));

            if let Err(e) = backend.prepare(&mut session.record, &package) {
                session.record.mark_failed(e.clone());
                session.log(LogLevel::Error, "backend", format!("Preparation failed: {}", e));
                return Err(e);
            }

            Ok(session_id)
        } else {
            Err(DiagnosticError::new(
                ErrorCode::ErrBackendUnavailable,
                format!("Backend with ID '{}' is not registered", backend_id),
            ))
        }
    }

    /// Launches an application whose session has been prepared.
    pub fn launch_session(&mut self, session_id: &SessionId) -> UmrResult<()> {
        let session = self.sessions.get_mut(session_id)?;
        let backend_id = session.backend_id.clone();

        session.record.transition_to(LifecycleState::Running)?;
        session.log(LogLevel::Info, "orchestrator", "Launching application");

        if let Some(backend) = self.registry.get_android_mut(&backend_id) {
            if let Err(e) = backend.launch(&mut session.record) {
                session.record.mark_failed(e.clone());
                session.log(LogLevel::Error, "backend", format!("Launch failed: {}", e));
                return Err(e);
            }
            Ok(())
        } else if let Some(backend) = self.registry.get_ios_mut(&backend_id) {
            if let Err(e) = backend.launch(&mut session.record) {
                session.record.mark_failed(e.clone());
                session.log(LogLevel::Error, "backend", format!("Launch failed: {}", e));
                return Err(e);
            }
            Ok(())
        } else {
            Err(DiagnosticError::new(
                ErrorCode::ErrBackendUnavailable,
                format!("Backend '{}' no longer available", backend_id),
            ))
        }
    }

    /// Polls the status of an active session.
    pub fn poll_session(&self, session_id: &SessionId) -> UmrResult<LifecycleState> {
        let session = self.sessions.get(session_id)?;
        Ok(session.record.state)
    }

    /// Stops an active application, reaps processes, and collects diagnostics.
    pub fn stop_session(&mut self, session_id: &SessionId) -> UmrResult<()> {
        let session = self.sessions.get_mut(session_id)?;
        let backend_id = session.backend_id.clone();

        session.record.transition_to(LifecycleState::Stopping)?;
        session.log(LogLevel::Info, "orchestrator", "Teardown requested; stopping backend");

        let stop_res = if let Some(backend) = self.registry.get_android_mut(&backend_id) {
            let res = backend.stop(&mut session.record);
            if let Ok(diag_lines) = backend.collect_diagnostics(&session.record) {
                for line in diag_lines {
                    session.log(LogLevel::Debug, "guest_logcat", line);
                }
            }
            res
        } else if let Some(backend) = self.registry.get_ios_mut(&backend_id) {
            let res = backend.stop(&mut session.record);
            if let Ok(diag_lines) = backend.collect_diagnostics(&session.record) {
                for line in diag_lines {
                    session.log(LogLevel::Debug, "guest_syslog", line);
                }
            }
            res
        } else {
            Ok(())
        };

        match stop_res {
            Ok(()) => {
                session.record.mark_terminated(0)?;
                session.log(LogLevel::Info, "orchestrator", "Session terminated cleanly");
                Ok(())
            }
            Err(e) => {
                session.record.mark_failed(e.clone());
                session.log(LogLevel::Error, "backend", format!("Stop error: {}", e));
                Err(e)
            }
        }
    }

    /// Retrieves the sanitized diagnostic report for a session.
    pub fn get_report(&self, session_id: &SessionId) -> UmrResult<&DiagnosticReport> {
        let session = self.sessions.get(session_id)?;
        Ok(&session.report)
    }

    /// Dispatches an input event (pointer touch, keycode, navigation) to an active session.
    pub fn send_input(&mut self, session_id: &SessionId, event: InputEvent) -> UmrResult<()> {
        let session = self.sessions.get(session_id)?;
        let backend_id = session.backend_id.clone();
        let record = session.record.clone();

        if let Some(backend) = self.registry.get_android_mut(&backend_id) {
            backend.send_input(&record, event)
        } else if let Some(backend) = self.registry.get_ios_mut(&backend_id) {
            backend.send_input(&record, event)
        } else {
            Err(DiagnosticError::new(
                ErrorCode::ErrBackendUnavailable,
                format!("Backend '{}' not found", backend_id),
            ))
        }
    }

    /// Captures the latest generated frame buffer from an active session.
    pub fn capture_frame(&self, session_id: &SessionId) -> UmrResult<Option<FrameBuffer>> {
        let session = self.sessions.get(session_id)?;
        let backend_id = &session.backend_id;

        if let Some(backend) = self.registry.get_android(backend_id) {
            backend.capture_frame(&session.record)
        } else if let Some(backend) = self.registry.get_ios(backend_id) {
            backend.capture_frame(&session.record)
        } else {
            Err(DiagnosticError::new(
                ErrorCode::ErrBackendUnavailable,
                format!("Backend '{}' not found", backend_id),
            ))
        }
    }

    /// Purges application data and caches for a session.
    pub fn wipe_storage(&mut self, session_id: &SessionId) -> UmrResult<()> {
        let session = self.sessions.get_mut(session_id)?;
        let backend_id = session.backend_id.clone();
        let record = session.record.clone();

        session.log(LogLevel::Info, "orchestrator", "Storage wipe requested");

        if let Some(backend) = self.registry.get_android_mut(&backend_id) {
            backend.wipe_storage(&record)
        } else {
            Err(DiagnosticError::new(
                ErrorCode::ErrBackendUnavailable,
                format!("Backend '{}' not found or does not support storage wiping", backend_id),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use umr_core::{
        AndroidPackageInfo, CpuArchitecture, ExecutionMode, PackageDetails, PackageFormat,
        SessionRecord, TargetAbi,
    };

    struct TestAndroidRunner {
        caps: BackendCapabilities,
        launched: bool,
        stopped: bool,
    }

    impl IAndroidBackend for TestAndroidRunner {
        fn id(&self) -> &str {
            &self.caps.backend_id
        }
        fn capabilities(&self) -> &BackendCapabilities {
            &self.caps
        }
        fn prepare(&mut self, _: &mut SessionRecord, _: &PackageMetadata) -> UmrResult<()> {
            Ok(())
        }
        fn launch(&mut self, _: &mut SessionRecord) -> UmrResult<()> {
            self.launched = true;
            Ok(())
        }
        fn poll_status(&self, _: &SessionRecord) -> UmrResult<LifecycleState> {
            if self.launched && !self.stopped {
                Ok(LifecycleState::Running)
            } else if self.stopped {
                Ok(LifecycleState::Terminated)
            } else {
                Ok(LifecycleState::Ready)
            }
        }
        fn stop(&mut self, _: &mut SessionRecord) -> UmrResult<()> {
            self.stopped = true;
            Ok(())
        }
        fn collect_diagnostics(&self, _: &SessionRecord) -> UmrResult<Vec<String>> {
            Ok(vec![
                "ActivityManager: Start proc 1234:com.test/u0a12".to_string(),
                "AuthService: user token=SECRET_TOKEN_XYZ authenticated".to_string(),
            ])
        }
    }

    #[test]
    fn test_orchestrator_lifecycle_flow() {
        let mut orch = RuntimeOrchestrator::new(PathBuf::from("fake-intake"));
        orch.register_android(Box::new(TestAndroidRunner {
            caps: BackendCapabilities {
                backend_id: "android-ref-runner".to_string(),
                backend_version: "0.1.0".to_string(),
                execution_mode: ExecutionMode::LocalNative,
                supported_abis: vec![TargetAbi::X86_64],
                supported_architectures: vec![CpuArchitecture::X86_64],
                min_api_level: Some(21),
                max_api_level: Some(34),
                requires_hardware_virtualization: false,
                requires_external_device: false,
            },
            launched: false,
            stopped: false,
        }));

        let pkg = PackageMetadata {
            format: PackageFormat::Apk,
            file_path: "/test/app.apk".to_string(),
            file_size_bytes: 1024,
            sha256_hash: "hash123".to_string(),
            details: PackageDetails::Android(AndroidPackageInfo {
                package_name: "org.umr.test".to_string(),
                version_code: Some(1),
                version_name: Some("1.0".to_string()),
                min_sdk_version: Some(26),
                target_sdk_version: Some(33),
                main_activity: Some("Main".to_string()),
                native_abis: vec![],
                is_pure_dex: true,
            }),
            inspected_at: Utc::now(),
        };

        // 1. Capability Negotiation
        let neg = orch.negotiate(&pkg);
        match neg {
            NegotiationResult::Compatible { backend_id, .. } => {
                assert_eq!(backend_id, "android-ref-runner");
            }
            _ => panic!("Expected compatible negotiation"),
        }

        // 2. Prepare Session
        let session_id = orch
            .prepare_session(pkg, "android-ref-runner")
            .expect("Preparation must succeed");
        assert_eq!(orch.poll_session(&session_id).unwrap(), LifecycleState::Preparing);

        // 3. Launch Session
        orch.launch_session(&session_id).expect("Launch must succeed");
        assert_eq!(orch.poll_session(&session_id).unwrap(), LifecycleState::Running);

        // 4. Stop Session
        orch.stop_session(&session_id).expect("Stop must succeed");
        assert_eq!(orch.poll_session(&session_id).unwrap(), LifecycleState::Terminated);

        // 5. Inspect Sanitized Diagnostics
        let report = orch.get_report(&session_id).expect("Report must exist");
        let has_redacted_token = report
            .entries
            .iter()
            .any(|e| e.message.contains("token=[REDACTED]"));
        assert!(has_redacted_token, "Secret token must be sanitized in logs");
    }
}
