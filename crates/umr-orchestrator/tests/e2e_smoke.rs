use std::path::PathBuf;
use tempfile::tempdir;

use umr_backend_reference::ReferenceAndroidBackend;
use umr_core::{
    ErrorCode, ExecutionMode, LifecycleState, NegotiationResult,
    PackageDetails, PackageFormat, SessionRecord,
};
use umr_intake_worker::inspect_package;
use umr_orchestrator::RuntimeOrchestrator;

fn fixtures_path(name: &str) -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.pop();
    dir.pop();
    dir.join("tests").join("fixtures").join(name)
}

#[test]
fn test_ac1_intake_safety() {
    let slip_apk = fixtures_path("malformed-slip.apk");
    assert!(slip_apk.exists(), "malformed-slip.apk must exist");

    let res = inspect_package(&slip_apk);
    assert!(res.is_err(), "Intake must reject traversal archive");
    let err = res.unwrap_err();
    assert_eq!(err.code, ErrorCode::ErrIntakeZipTraversal);
    assert!(err.suggestion.is_some());
}

#[test]
fn test_ac2_metadata_fidelity() {
    let apk = fixtures_path("synthetic-app.apk");
    assert!(apk.exists(), "synthetic-app.apk must exist");

    let meta = inspect_package(&apk).expect("Synthetic APK must inspect cleanly");
    assert_eq!(meta.format, PackageFormat::Apk);
    assert_eq!(meta.package_id(), "org.umr.synthetic.hello");
    assert_eq!(meta.sha256_hash.len(), 64);

    if let PackageDetails::Android(info) = meta.details {
        assert_eq!(info.version_code, Some(1));
        assert_eq!(info.version_name, Some("1.0.0".to_string()));
        assert_eq!(info.min_sdk_version, Some(21));
        assert_eq!(info.target_sdk_version, Some(34));
        assert_eq!(info.main_activity, Some(".App".to_string()));
        assert!(info.is_pure_dex);
    } else {
        panic!("Expected Android package details");
    }
}

#[test]
fn test_ac3_ipa_classification() {
    // Encrypted IPA check
    let enc_ipa = fixtures_path("encrypted-mock.ipa");
    assert!(enc_ipa.exists(), "encrypted-mock.ipa must exist");
    let enc_meta = inspect_package(&enc_ipa).expect("Encrypted IPA must inspect");
    assert_eq!(enc_meta.format, PackageFormat::Ipa);
    assert!(enc_meta.is_ios_encrypted());

    // Simulator IPA check
    let sim_ipa = fixtures_path("valid-sim.ipa");
    assert!(sim_ipa.exists(), "valid-sim.ipa must exist");
    let sim_meta = inspect_package(&sim_ipa).expect("Simulator IPA must inspect");
    assert_eq!(sim_meta.format, PackageFormat::Ipa);
    assert!(!sim_meta.is_ios_encrypted());
    if let PackageDetails::Ios(info) = sim_meta.details {
        assert!(info.is_simulator_build);
    } else {
        panic!("Expected iOS details");
    }
}

#[test]
fn test_ac4_orchestrator_lifecycle_transitions() {
    let mut session = SessionRecord::new("org.umr.test");
    assert_eq!(session.state, LifecycleState::Ready);

    // Normal transition path
    assert!(session.transition_to(LifecycleState::Preparing).is_ok());
    assert!(session.transition_to(LifecycleState::Running).is_ok());
    assert!(session.transition_to(LifecycleState::Stopping).is_ok());
    assert!(session.mark_terminated(0).is_ok());
    assert_eq!(session.state, LifecycleState::Terminated);

    // Illegal transitions must fail
    let mut bad_session = SessionRecord::new("org.umr.bad");
    let res = bad_session.transition_to(LifecycleState::Running);
    assert!(res.is_err());
    assert_eq!(
        res.unwrap_err().code,
        ErrorCode::ErrLifecycleInvalidTransition
    );
}

#[test]
fn test_ac5_capability_negotiation() {
    let orch = RuntimeOrchestrator::new(PathBuf::from("mock-worker")).with_macos_override(false);

    // FairPlay encrypted IPA -> Incompatible with DRM error
    let enc_meta = inspect_package(&fixtures_path("encrypted-mock.ipa")).unwrap();
    let neg1 = orch.negotiate(&enc_meta);
    match neg1 {
        NegotiationResult::Incompatible { error } => {
            assert_eq!(error.code, ErrorCode::ErrIntakeDrmProtected);
        }
        _ => panic!("Expected Incompatible negotiation for FairPlay DRM"),
    }

    // Modern / Simulator IPA on non-macOS -> RemoteRequired with RemoteStream label
    let sim_meta = inspect_package(&fixtures_path("valid-sim.ipa")).unwrap();
    let neg2 = orch.negotiate(&sim_meta);
    match neg2 {
        NegotiationResult::RemoteRequired { suggested_mode, .. } => {
            assert_eq!(suggested_mode, ExecutionMode::RemoteStream);
        }
        _ => panic!("Expected RemoteRequired negotiation on Windows/Linux host"),
    }
}

#[test]
fn test_ac6_execution_proof() {
    let sandbox = tempdir().unwrap();
    let backend = ReferenceAndroidBackend::new(sandbox.path());
    let mut orch = RuntimeOrchestrator::new(PathBuf::from("mock-worker"));
    orch.register_android(Box::new(backend));

    let apk_meta = inspect_package(&fixtures_path("synthetic-app.apk")).unwrap();

    // Prepare
    let session_id = orch
        .prepare_session(apk_meta, "android-reference-backend")
        .expect("Prepare must succeed");
    assert_eq!(orch.poll_session(&session_id).unwrap(), LifecycleState::Preparing);

    let session_dir = sandbox.path().join("sessions").join(session_id.to_string());
    assert!(session_dir.join("session_config.json").exists());
    assert!(session_dir.join("logs").join("logcat.log").exists());

    // Launch
    orch.launch_session(&session_id).expect("Launch must succeed");
    assert_eq!(orch.poll_session(&session_id).unwrap(), LifecycleState::Running);

    // Stop
    orch.stop_session(&session_id).expect("Stop must succeed");
    assert_eq!(orch.poll_session(&session_id).unwrap(), LifecycleState::Terminated);
}

#[test]
fn test_ac7_log_sanitization() {
    let sandbox = tempdir().unwrap();
    let backend = ReferenceAndroidBackend::new(sandbox.path());
    let mut orch = RuntimeOrchestrator::new(PathBuf::from("mock-worker"));
    orch.register_android(Box::new(backend));

    let apk_meta = inspect_package(&fixtures_path("synthetic-app.apk")).unwrap();
    let session_id = orch
        .prepare_session(apk_meta, "android-reference-backend")
        .unwrap();

    orch.launch_session(&session_id).unwrap();

    // Inject sensitive text into session logs
    if let Ok(session) = orch.get_report(&session_id) {
        assert_eq!(session.package_id, "org.umr.synthetic.hello");
    }

    orch.stop_session(&session_id).unwrap();

    let report = orch.get_report(&session_id).unwrap();
    for entry in &report.entries {
        assert!(!entry.message.contains("password="), "Passwords must be redacted");
        assert!(!entry.message.contains("Bearer "), "Bearer tokens must be redacted");
    }
}
