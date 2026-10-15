use chrono::Utc;
use tempfile::tempdir;
use umr_backend_ios::gap_tracker::{CompatibilityVerdict, FrameworkGapTracker, FrameworkSupportLevel};
use umr_backend_ios::{IosHleBackend, IosRemoteStreamBackend, IosSimulatorBackend};
use umr_core::{
    CapabilityNegotiator, CpuArchitecture, EncryptionStatus, ErrorCode, ExecutionMode,
    IIosBackend, IosPackageInfo, LifecycleState, NegotiationResult, PackageDetails, PackageFormat,
    PackageMetadata, SessionRecord,
};

fn create_ios_package(
    bundle_id: &str,
    archs: Vec<CpuArchitecture>,
    is_sim: bool,
    min_os: &str,
    encryption: EncryptionStatus,
) -> PackageMetadata {
    PackageMetadata {
        format: PackageFormat::Ipa,
        file_path: format!("/tmp/{}.ipa", bundle_id),
        file_size_bytes: 10 * 1024 * 1024,
        sha256_hash: "mockhash1234567890".to_string(),
        details: PackageDetails::Ios(IosPackageInfo {
            bundle_identifier: bundle_id.to_string(),
            bundle_version: Some("1.0.0".to_string()),
            bundle_short_version: Some("1.0".to_string()),
            minimum_os_version: Some(min_os.to_string()),
            architectures: archs,
            encryption,
            is_simulator_build: is_sim,
        }),
        inspected_at: Utc::now(),
    }
}

#[test]
fn test_gap_tracker_drm_fairplay_rejection() {
    let pkg = create_ios_package(
        "com.store.encrypted",
        vec![CpuArchitecture::Arm64],
        false,
        "15.0",
        EncryptionStatus::FairPlayEncrypted { cryptid: 1 },
    );

    let report = FrameworkGapTracker::audit_package(&pkg).expect("Audit should succeed");
    assert_eq!(report.verdict, CompatibilityVerdict::DrmBlocked);
    assert!(report.missing_services.iter().any(|s| s.contains("FairPlay")));
    assert!(report.actionable_recommendations.iter().any(|r| r.contains("FairPlay DRM")));
}

#[test]
fn test_gap_tracker_legacy_32bit_classification() {
    let pkg = create_ios_package(
        "com.retro.game",
        vec![CpuArchitecture::Armv7],
        false,
        "3.1",
        EncryptionStatus::Unencrypted,
    );

    let report = FrameworkGapTracker::audit_package(&pkg).expect("Audit should succeed");
    assert_eq!(report.verdict, CompatibilityVerdict::Compatible);
    assert!(report.actionable_recommendations.iter().any(|r| r.contains("Track B")));
}

#[test]
fn test_gap_tracker_simulator_build_classification() {
    let pkg = create_ios_package(
        "com.dev.simulatorapp",
        vec![CpuArchitecture::X86_64, CpuArchitecture::Arm64],
        true,
        "16.0",
        EncryptionStatus::Unencrypted,
    );

    let report = FrameworkGapTracker::audit_package(&pkg).expect("Audit should succeed");
    assert!(report.actionable_recommendations.iter().any(|r| r.contains("Track A")));
}

#[test]
fn test_gap_tracker_modern_device_gap_detection() {
    let pkg = create_ios_package(
        "com.modern.metalapp",
        vec![CpuArchitecture::Arm64],
        false,
        "16.0",
        EncryptionStatus::Unencrypted,
    );

    let report = FrameworkGapTracker::audit_package(&pkg).expect("Audit should succeed");
    // Modern iOS app will have Metal (UnsupportedGap) and StoreKit (CloudServiceBlocked)
    assert!(report.framework_audits.iter().any(|f| f.name == "Metal" && f.support_level == FrameworkSupportLevel::UnsupportedGap));
    assert!(report.framework_audits.iter().any(|f| f.name == "StoreKit" && f.support_level == FrameworkSupportLevel::CloudServiceBlocked));
    assert!(report.actionable_recommendations.iter().any(|r| r.contains("Track C")));
}

#[test]
fn test_ios_simulator_backend_lifecycle() {
    let tmp = tempdir().unwrap();
    let mut backend = IosSimulatorBackend::with_host_override(tmp.path(), false); // mock run on non-macOS
    assert_eq!(backend.id(), "ios-mac-simulator");
    assert_eq!(backend.capabilities().execution_mode, ExecutionMode::LocalNative);

    let pkg = create_ios_package(
        "com.test.simapp",
        vec![CpuArchitecture::X86_64],
        true,
        "14.0",
        EncryptionStatus::Unencrypted,
    );
    let mut session = SessionRecord::new("com.test.simapp");

    // 1. Prepare
    session.transition_to(LifecycleState::Preparing).unwrap();
    backend.prepare(&mut session, &pkg).unwrap();

    // 2. Launch
    session.transition_to(LifecycleState::Running).unwrap();
    backend.launch(&mut session).unwrap();

    // 3. Status
    assert_eq!(backend.poll_status(&session).unwrap(), LifecycleState::Running);

    // 4. Capture Frame
    let frame = backend.capture_frame(&session).unwrap();
    assert!(frame.is_some());
    let fb = frame.unwrap();
    assert_eq!(fb.width, 1170);
    assert_eq!(fb.height, 2532);

    // 5. Diagnostics
    let logs = backend.collect_diagnostics(&session).unwrap();
    assert!(logs.iter().any(|l| l.contains("CoreSimulator")));

    // 6. Stop
    backend.stop(&mut session).unwrap();
    assert_eq!(backend.poll_status(&session).unwrap(), LifecycleState::Terminated);

    // 7. Negative case: Reject non-simulator package
    let non_sim_pkg = create_ios_package(
        "com.test.deviceapp",
        vec![CpuArchitecture::Arm64],
        false,
        "14.0",
        EncryptionStatus::Unencrypted,
    );
    let mut bad_session = SessionRecord::new("com.test.deviceapp");
    bad_session.transition_to(LifecycleState::Preparing).unwrap();
    let err = backend.prepare(&mut bad_session, &non_sim_pkg).unwrap_err();
    assert_eq!(err.code, ErrorCode::ErrBackendIncompatibleArchitecture);
}

#[test]
fn test_ios_hle_backend_lifecycle_and_cohort_rejection() {
    let tmp = tempdir().unwrap();
    let mut backend = IosHleBackend::new(tmp.path());
    assert_eq!(backend.id(), "ios-legacy-hle");
    assert_eq!(backend.capabilities().execution_mode, ExecutionMode::LocalHle);

    let pkg = create_ios_package(
        "com.vintage.app",
        vec![CpuArchitecture::Armv7],
        false,
        "3.0",
        EncryptionStatus::Unencrypted,
    );
    let mut session = SessionRecord::new("com.vintage.app");

    // 1. Prepare
    session.transition_to(LifecycleState::Preparing).unwrap();
    backend.prepare(&mut session, &pkg).unwrap();

    // 2. Launch
    session.transition_to(LifecycleState::Running).unwrap();
    backend.launch(&mut session).unwrap();
    assert_eq!(backend.poll_status(&session).unwrap(), LifecycleState::Running);

    // 3. Frame buffer (320x480 vintage iPhone resolution)
    let frame = backend.capture_frame(&session).unwrap().expect("Frame exists");
    assert_eq!(frame.width, 320);
    assert_eq!(frame.height, 480);

    // 4. Diagnostics
    let logs = backend.collect_diagnostics(&session).unwrap();
    assert!(logs.iter().any(|l| l.contains("Dynarmic ARMv7")));

    // 5. Stop
    backend.stop(&mut session).unwrap();
    assert_eq!(backend.poll_status(&session).unwrap(), LifecycleState::Terminated);

    // 6. Negative case 1: Reject ARM64 binary
    let arm64_pkg = create_ios_package(
        "com.modern.app",
        vec![CpuArchitecture::Arm64],
        false,
        "3.0",
        EncryptionStatus::Unencrypted,
    );
    let mut s2 = SessionRecord::new("com.modern.app");
    s2.transition_to(LifecycleState::Preparing).unwrap();
    let err1 = backend.prepare(&mut s2, &arm64_pkg).unwrap_err();
    assert_eq!(err1.code, ErrorCode::ErrBackendIncompatibleArchitecture);

    // 7. Negative case 2: Reject iOS 15.0 minimum requirement
    let modern_os_pkg = create_ios_package(
        "com.modern.os",
        vec![CpuArchitecture::Armv7],
        false,
        "15.0",
        EncryptionStatus::Unencrypted,
    );
    let mut s3 = SessionRecord::new("com.modern.os");
    s3.transition_to(LifecycleState::Preparing).unwrap();
    let err2 = backend.prepare(&mut s3, &modern_os_pkg).unwrap_err();
    assert_eq!(err2.code, ErrorCode::ErrBackendIncompatibleApiLevel);
}

#[test]
fn test_ios_remote_stream_backend_labeling_and_lifecycle() {
    let mut backend = IosRemoteStreamBackend::with_device(true, Some("UDID-TEST-12345".to_string()));
    assert_eq!(backend.id(), "ios-remote-stream");
    // MANDATORY explicit labeling check
    assert_eq!(backend.capabilities().execution_mode, ExecutionMode::RemoteStream);

    let pkg = create_ios_package(
        "com.device.app",
        vec![CpuArchitecture::Arm64],
        false,
        "16.0",
        EncryptionStatus::Unencrypted,
    );
    let mut session = SessionRecord::new("com.device.app");

    // 1. Prepare & Launch
    session.transition_to(LifecycleState::Preparing).unwrap();
    backend.prepare(&mut session, &pkg).unwrap();
    session.transition_to(LifecycleState::Running).unwrap();
    backend.launch(&mut session).unwrap();

    // 2. Diagnostics contain explicit [REMOTE] label
    let logs = backend.collect_diagnostics(&session).unwrap();
    assert!(logs.iter().any(|l| l.contains("[REMOTE]") && l.contains("RemoteStream")));

    // 3. Watermarked frame buffer
    let frame = backend.capture_frame(&session).unwrap().expect("Frame exists");
    assert_eq!(frame.width, 1170);
    assert_eq!(frame.height, 2532);

    // 4. Disconnected device rejection
    let mut disconnected_backend = IosRemoteStreamBackend::with_device(false, None);
    let mut s_bad = SessionRecord::new("com.device.app");
    s_bad.transition_to(LifecycleState::Preparing).unwrap();
    let err = disconnected_backend.prepare(&mut s_bad, &pkg).unwrap_err();
    assert_eq!(err.code, ErrorCode::ErrBackendRemoteDeviceRequired);
}

#[test]
fn test_tiered_capability_negotiator_ios() {
    let tmp = tempdir().unwrap();
    let sim_backend = IosSimulatorBackend::with_host_override(tmp.path(), true);
    let hle_backend = IosHleBackend::new(tmp.path());
    let remote_backend = IosRemoteStreamBackend::new();

    let ios_backends: Vec<&dyn IIosBackend> = vec![&sim_backend, &hle_backend, &remote_backend];

    // Case 1: Simulator package on macOS host -> Track A (LocalNative)
    let sim_pkg = create_ios_package("com.app.sim", vec![CpuArchitecture::Arm64], true, "15.0", EncryptionStatus::Unencrypted);
    let res1 = CapabilityNegotiator::negotiate(&sim_pkg, &[], &ios_backends, true);
    match res1 {
        NegotiationResult::Compatible { backend_id, execution_mode } => {
            assert_eq!(backend_id, "ios-mac-simulator");
            assert_eq!(execution_mode, ExecutionMode::LocalNative);
        }
        _ => panic!("Expected Track A compatibility on macOS"),
    }

    // Case 2: Legacy 32-bit package on Windows/Linux host -> Track B (LocalHle)
    let hle_pkg = create_ios_package("com.app.vintage", vec![CpuArchitecture::Armv7], false, "3.1", EncryptionStatus::Unencrypted);
    let res2 = CapabilityNegotiator::negotiate(&hle_pkg, &[], &ios_backends, false);
    match res2 {
        NegotiationResult::Compatible { backend_id, execution_mode } => {
            assert_eq!(backend_id, "ios-legacy-hle");
            assert_eq!(execution_mode, ExecutionMode::LocalHle);
        }
        _ => panic!("Expected Track B compatibility for 32-bit app"),
    }

    // Case 3: Modern 64-bit package on non-macOS host -> Track C (RemoteRequired / RemoteStream)
    let modern_pkg = create_ios_package("com.app.modern", vec![CpuArchitecture::Arm64], false, "16.0", EncryptionStatus::Unencrypted);
    let res3 = CapabilityNegotiator::negotiate(&modern_pkg, &[], &ios_backends, false);
    match res3 {
        NegotiationResult::RemoteRequired { suggested_mode, reason } => {
            assert_eq!(suggested_mode, ExecutionMode::RemoteStream);
            assert!(reason.contains("Tethered physical device"));
        }
        _ => panic!("Expected RemoteRequired for modern iOS on non-macOS host"),
    }

    // Case 4: Commercial FairPlay DRM encrypted package -> Incompatible rejection
    let drm_pkg = create_ios_package("com.app.store", vec![CpuArchitecture::Arm64], false, "15.0", EncryptionStatus::FairPlayEncrypted { cryptid: 1 });
    let res4 = CapabilityNegotiator::negotiate(&drm_pkg, &[], &ios_backends, true);
    match res4 {
        NegotiationResult::Incompatible { error } => {
            assert_eq!(error.code, ErrorCode::ErrIntakeDrmProtected);
        }
        _ => panic!("Expected DRM rejection"),
    }
}

