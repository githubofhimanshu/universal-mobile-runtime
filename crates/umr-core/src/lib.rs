pub mod backend;
pub mod error;
pub mod lifecycle;
pub mod metadata;

pub use backend::{
    BackendCapabilities, CapabilityNegotiator, ExecutionMode, IAndroidBackend, IIosBackend,
    NegotiationResult,
};
pub use error::{DiagnosticError, ErrorCode, UmrResult};
pub use lifecycle::{LifecycleState, SessionId, SessionRecord};
pub use metadata::{
    AndroidPackageInfo, CpuArchitecture, EncryptionStatus, IosPackageInfo, PackageDetails,
    PackageFormat, PackageMetadata, TargetAbi,
};

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_lifecycle_valid_transitions() {
        let mut session = SessionRecord::new("com.example.testapp");
        assert_eq!(session.state, LifecycleState::Ready);

        // Ready -> Preparing
        assert!(session.transition_to(LifecycleState::Preparing).is_ok());
        assert_eq!(session.state, LifecycleState::Preparing);

        // Preparing -> Running
        assert!(session.transition_to(LifecycleState::Running).is_ok());
        assert_eq!(session.state, LifecycleState::Running);

        // Running -> Stopping
        assert!(session.transition_to(LifecycleState::Stopping).is_ok());
        assert_eq!(session.state, LifecycleState::Stopping);

        // Stopping -> Terminated
        assert!(session.mark_terminated(0).is_ok());
        assert_eq!(session.state, LifecycleState::Terminated);
        assert_eq!(session.exit_code, Some(0));

        // Terminated -> Ready (reset for reuse)
        assert!(session.transition_to(LifecycleState::Ready).is_ok());
    }

    #[test]
    fn test_lifecycle_invalid_transitions() {
        let mut session = SessionRecord::new("com.example.testapp");
        assert_eq!(session.state, LifecycleState::Ready);

        // Ready -> Running without preparing must be rejected
        let res = session.transition_to(LifecycleState::Running);
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert_eq!(err.code, ErrorCode::ErrLifecycleInvalidTransition);

        // Ready -> Terminated directly must be rejected
        assert!(session.transition_to(LifecycleState::Terminated).is_err());
    }

    #[test]
    fn test_lifecycle_failure_transition() {
        let mut session = SessionRecord::new("com.example.testapp");
        session.transition_to(LifecycleState::Preparing).unwrap();

        // Failure during preparing
        session.mark_failed(DiagnosticError::new(
            ErrorCode::ErrBackendProcessCrashed,
            "Guest process failed to initialize",
        ));

        assert_eq!(session.state, LifecycleState::Failed);
        assert!(session.last_error.is_some());
        assert_eq!(
            session.last_error.as_ref().unwrap().code,
            ErrorCode::ErrBackendProcessCrashed
        );

        // Can reset from Failed to Ready
        assert!(session.transition_to(LifecycleState::Ready).is_ok());
    }

    #[test]
    fn test_diagnostic_error_serialization() {
        let err = DiagnosticError::zip_traversal("../../etc/shadow");
        let json = serde_json::to_string(&err).expect("serialize error");
        assert!(json.contains("ERR_INTAKE_ZIP_TRAVERSAL"));
        assert!(json.to_lowercase().contains("path traversal"));

        let deserialized: DiagnosticError =
            serde_json::from_str(&json).expect("deserialize error");
        assert_eq!(deserialized.code, ErrorCode::ErrIntakeZipTraversal);
        assert_eq!(deserialized.context, Some("../../etc/shadow".to_string()));
    }

    struct MockAndroidBackend {
        caps: BackendCapabilities,
    }

    impl IAndroidBackend for MockAndroidBackend {
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
            Ok(())
        }
        fn poll_status(&self, _: &SessionRecord) -> UmrResult<LifecycleState> {
            Ok(LifecycleState::Running)
        }
        fn stop(&mut self, _: &mut SessionRecord) -> UmrResult<()> {
            Ok(())
        }
        fn collect_diagnostics(&self, _: &SessionRecord) -> UmrResult<Vec<String>> {
            Ok(vec!["log line 1".to_string()])
        }
    }

    #[test]
    fn test_capability_negotiator_android_pure_dex() {
        let backend = MockAndroidBackend {
            caps: BackendCapabilities {
                backend_id: "android-whpx-vm".to_string(),
                backend_version: "0.1.0".to_string(),
                execution_mode: ExecutionMode::LocalVirtualized,
                supported_abis: vec![TargetAbi::X86_64],
                supported_architectures: vec![CpuArchitecture::X86_64],
                min_api_level: Some(21),
                max_api_level: Some(34),
                requires_hardware_virtualization: true,
                requires_external_device: false,
            },
        };

        let pkg = PackageMetadata {
            format: PackageFormat::Apk,
            file_path: "/path/to/app.apk".to_string(),
            file_size_bytes: 1024 * 1024,
            sha256_hash: "abc123hash".to_string(),
            details: PackageDetails::Android(AndroidPackageInfo {
                package_name: "com.example.pureapp".to_string(),
                version_code: Some(1),
                version_name: Some("1.0.0".to_string()),
                min_sdk_version: Some(24),
                target_sdk_version: Some(33),
                main_activity: Some("MainActivity".to_string()),
                native_abis: vec![],
                is_pure_dex: true,
            }),
            inspected_at: Utc::now(),
        };

        let result = CapabilityNegotiator::negotiate(&pkg, &[&backend], &[], false);
        match result {
            NegotiationResult::Compatible {
                backend_id,
                execution_mode,
            } => {
                assert_eq!(backend_id, "android-whpx-vm");
                assert_eq!(execution_mode, ExecutionMode::LocalVirtualized);
            }
            _ => panic!("Expected compatible result for pure dex app"),
        }
    }

    #[test]
    fn test_capability_negotiator_ios_fairplay_rejection() {
        let pkg = PackageMetadata {
            format: PackageFormat::Ipa,
            file_path: "/path/to/store.ipa".to_string(),
            file_size_bytes: 50 * 1024 * 1024,
            sha256_hash: "store_hash".to_string(),
            details: PackageDetails::Ios(IosPackageInfo {
                bundle_identifier: "com.example.storegame".to_string(),
                bundle_version: Some("1.0".to_string()),
                bundle_short_version: Some("1.0".to_string()),
                minimum_os_version: Some("15.0".to_string()),
                architectures: vec![CpuArchitecture::Arm64],
                encryption: EncryptionStatus::FairPlayEncrypted { cryptid: 1 },
                is_simulator_build: false,
            }),
            inspected_at: Utc::now(),
        };

        let result = CapabilityNegotiator::negotiate(&pkg, &[], &[], false);
        match result {
            NegotiationResult::Incompatible { error } => {
                assert_eq!(error.code, ErrorCode::ErrIntakeDrmProtected);
                assert!(error.message.contains("FairPlay DRM"));
            }
            _ => panic!("Expected encrypted IPA to be rejected as incompatible"),
        }
    }

    #[test]
    fn test_capability_negotiator_ios_remote_stream_label() {
        let pkg = PackageMetadata {
            format: PackageFormat::Ipa,
            file_path: "/path/to/modern.ipa".to_string(),
            file_size_bytes: 20 * 1024 * 1024,
            sha256_hash: "modern_hash".to_string(),
            details: PackageDetails::Ios(IosPackageInfo {
                bundle_identifier: "com.example.modernapp".to_string(),
                bundle_version: Some("2.0".to_string()),
                bundle_short_version: Some("2.0".to_string()),
                minimum_os_version: Some("16.0".to_string()),
                architectures: vec![CpuArchitecture::Arm64],
                encryption: EncryptionStatus::Unencrypted,
                is_simulator_build: false,
            }),
            inspected_at: Utc::now(),
        };

        // On non-macOS (e.g. Windows), modern iOS app must yield RemoteRequired
        let result = CapabilityNegotiator::negotiate(&pkg, &[], &[], false);
        match result {
            NegotiationResult::RemoteRequired {
                suggested_mode,
                reason,
            } => {
                assert_eq!(suggested_mode, ExecutionMode::RemoteStream);
                assert!(reason.contains("Tethered physical device or remote runner required"));
            }
            _ => panic!("Expected remote stream classification on non-macOS host"),
        }
    }
}

