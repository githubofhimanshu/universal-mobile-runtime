use chrono::Utc;
use tempfile::tempdir;
use umr_backend_android::host_detector::{HostOs, HostPlatformDetector, HypervisorKind};
use umr_backend_android::hypervisor::{
    IPartitionAdapter, LinuxContainerAdapter, MacVirtAdapter, WhpxAdapter,
};
use umr_backend_android::package_manager::AndroidPackageManager;
use umr_backend_android::transport::{AdbTransport, GuestTransport, MockGuestTransport, VsockTransport};
use umr_backend_android::watchdog::WatchdogSupervisor;
use umr_backend_android::AndroidVirtualizationBackend;
use umr_core::{
    AndroidPackageInfo, CpuArchitecture, ErrorCode, ExecutionMode, IAndroidBackend, InputEvent,
    LifecycleState, NavigationButton, PackageDetails, PackageFormat, PackageMetadata, PointerAction,
    SessionRecord, TargetAbi,
};

#[test]
fn test_host_platform_detection_live() {
    let caps = HostPlatformDetector::detect_current();
    assert!(caps.logical_cpu_cores > 0);
    assert!(caps.total_memory_mb > 0);
    assert!(!caps.supported_guest_abis.is_empty());
    
    // On Windows, host OS should be Windows
    if cfg!(target_os = "windows") {
        assert_eq!(caps.host_os, HostOs::Windows);
    }
}

#[test]
fn test_host_platform_simulation_cross_host() {
    // 1. Windows x86-64 with WHPX
    let win_caps = HostPlatformDetector::evaluate_custom(
        HostOs::Windows,
        CpuArchitecture::X86_64,
        true,
        false,
        8,
    );
    assert_eq!(win_caps.host_os, HostOs::Windows);
    assert_eq!(win_caps.hypervisor_kind, HypervisorKind::Whpx);
    assert_eq!(win_caps.recommended_execution_mode, ExecutionMode::LocalVirtualized);
    assert!(win_caps.supported_guest_abis.contains(&TargetAbi::X86_64));

    // 2. Linux ARM64 with Container/Binderfs
    let linux_caps = HostPlatformDetector::evaluate_custom(
        HostOs::Linux,
        CpuArchitecture::Arm64,
        false,
        true,
        16,
    );
    assert_eq!(linux_caps.host_os, HostOs::Linux);
    assert_eq!(linux_caps.hypervisor_kind, HypervisorKind::LinuxContainer);
    assert_eq!(linux_caps.recommended_execution_mode, ExecutionMode::LocalNative);
    assert!(linux_caps.supported_guest_abis.contains(&TargetAbi::Arm64V8a));

    // 3. macOS Apple Silicon with Virtualization.framework
    let mac_caps = HostPlatformDetector::evaluate_custom(
        HostOs::MacOS,
        CpuArchitecture::Arm64,
        true,
        false,
        10,
    );
    assert_eq!(mac_caps.host_os, HostOs::MacOS);
    assert_eq!(mac_caps.hypervisor_kind, HypervisorKind::MacVirtualization);
    assert_eq!(mac_caps.recommended_execution_mode, ExecutionMode::LocalVirtualized);
    // Rosetta 2 enabled -> both ARM64 and x86_64 guest abis supported
    assert!(mac_caps.supported_guest_abis.contains(&TargetAbi::Arm64V8a));
    assert!(mac_caps.supported_guest_abis.contains(&TargetAbi::X86_64));
}

#[test]
fn test_hypervisor_partition_adapters() {
    // 1. WHPX Adapter
    let whpx = WhpxAdapter::with_availability(true);
    assert!(whpx.is_supported());
    let win_config = whpx.create_partition_config("sess-win-1", 4, 4096);
    assert_eq!(win_config.vcpu_count, 4);
    assert_eq!(win_config.memory_mb, 4096);
    assert_eq!(win_config.virtio_vsock_cid, 3);
    assert!(win_config.kernel_cmdline.contains("androidboot.hardware=virtio"));
    assert!(win_config.mount_points.iter().any(|m| m.guest_path == "/data"));

    // 2. Linux Container Adapter
    let lxc = LinuxContainerAdapter::with_support(true);
    assert!(lxc.is_supported());
    let linux_config = lxc.create_partition_config("sess-lxc-1", 8, 2048);
    assert_eq!(linux_config.vcpu_count, 8);
    assert!(linux_config.mount_points.iter().any(|m| m.host_path == "/dev/binderfs"));

    // 3. macOS Virtualization Adapter with Rosetta 2
    let mac = MacVirtAdapter::with_settings(true, true);
    assert!(mac.is_supported());
    assert!(mac.is_rosetta_directory_share_enabled());
    let mac_config = mac.create_partition_config("sess-mac-1", 4, 4096);
    assert!(mac_config.mount_points.iter().any(|m| m.guest_path == "/rosetta"));
}

#[test]
fn test_guest_transports() {
    // 1. Mock Transport
    let mock = MockGuestTransport::new();
    assert!(mock.is_alive());
    let out = mock.execute_command("pm list packages").unwrap();
    assert!(out.is_success());
    assert_eq!(mock.executed_commands().len(), 1);

    mock.send_input(&InputEvent::pointer(0.5, 0.5, PointerAction::Down)).unwrap();
    assert_eq!(mock.received_inputs().len(), 1);

    let frame = mock.capture_frame().unwrap();
    assert!(frame.is_some());
    let logs = mock.stream_logs().unwrap();
    assert!(!logs.is_empty());

    // 2. Vsock Transport
    let mut vsock = VsockTransport::new(3, 5555);
    assert_eq!(vsock.guest_cid(), 3);
    assert_eq!(vsock.port(), 5555);
    assert!(vsock.is_alive());
    assert!(vsock.execute_command("echo test").unwrap().is_success());
    vsock.set_connected(false);
    assert!(!vsock.is_alive());
    assert!(!vsock.execute_command("echo test").unwrap().is_success());

    // 3. ADB Transport
    let adb = AdbTransport::default_local().with_display_dimensions(1080, 1920);
    assert_eq!(adb.address().port(), 5555);
}

fn create_test_package(pkg_name: &str, version_code: u64, path: &str) -> PackageMetadata {
    PackageMetadata {
        format: PackageFormat::Apk,
        file_path: path.to_string(),
        file_size_bytes: 4096,
        sha256_hash: "mockhash1234567890".to_string(),
        details: PackageDetails::Android(AndroidPackageInfo {
            package_name: pkg_name.to_string(),
            version_code: Some(version_code),
            version_name: Some("1.0".to_string()),
            min_sdk_version: Some(24),
            target_sdk_version: Some(33),
            main_activity: Some("MainActivity".to_string()),
            native_abis: vec![TargetAbi::X86_64],
            is_pure_dex: false,
        }),
        inspected_at: Utc::now(),
    }
}

#[test]
fn test_package_manager_lifecycle_and_persistence() {
    let tmp = tempdir().unwrap();
    let sandbox_path = tmp.path();

    let pkg_meta = create_test_package("org.umr.sample.app", 100, "/tmp/app_v1.apk");

    // Phase 1: Install
    {
        let mut mgr = AndroidPackageManager::new(sandbox_path).unwrap();
        assert_eq!(mgr.list_packages().len(), 0);

        let installed = mgr.install_package(&pkg_meta).unwrap();
        assert_eq!(installed.package_name, "org.umr.sample.app");
        assert_eq!(installed.version_code, 100);
        assert!(installed.data_dir.exists());
        assert!(installed.sdcard_dir.exists());

        // Create dummy app state inside data directory
        std::fs::write(installed.data_dir.join("shared_prefs.xml"), "<map><bool name='seen' value='true'/></map>").unwrap();
        assert_eq!(mgr.list_packages().len(), 1);
    }

    // Phase 2: Persistence verification
    {
        let mut mgr = AndroidPackageManager::new(sandbox_path).unwrap();
        assert_eq!(mgr.list_packages().len(), 1);
        let data_dir = {
            let rec = mgr.get_package("org.umr.sample.app").unwrap();
            assert_eq!(rec.version_code, 100);
            assert!(rec.data_dir.join("shared_prefs.xml").exists());
            rec.data_dir.clone()
        };

        // Phase 3: Upgrade Package
        let upgrade_meta = create_test_package("org.umr.sample.app", 101, "/tmp/app_v2.apk");
        let upgraded = mgr.upgrade_package(&upgrade_meta, false).unwrap();
        assert_eq!(upgraded.version_code, 101);
        // Preserves existing data!
        assert!(upgraded.data_dir.join("shared_prefs.xml").exists());

        // Phase 4: Reject Downgrade without force flag
        let downgrade_meta = create_test_package("org.umr.sample.app", 99, "/tmp/app_v0.apk");
        let down_res = mgr.upgrade_package(&downgrade_meta, false);
        assert!(down_res.is_err());
        assert_eq!(down_res.unwrap_err().code, ErrorCode::ErrIntakeParseFailure);

        // Phase 5: Wipe Storage
        mgr.wipe_storage("org.umr.sample.app").unwrap();
        assert!(!data_dir.join("shared_prefs.xml").exists());
        assert!(data_dir.exists()); // Directory still exists, clean

        // Phase 6: Uninstall Package
        mgr.uninstall_package("org.umr.sample.app").unwrap();
        assert_eq!(mgr.list_packages().len(), 0);
        assert!(!data_dir.exists());
    }
}

#[test]
fn test_watchdog_resilience_and_crash_detection() {
    let mut watchdog = WatchdogSupervisor::default_monitor();
    watchdog.set_active_pid(2048);

    // No ANR when active
    assert!(watchdog.check_anr().is_none());

    // Inspect crash logs
    let raw_logs = vec![
        "ActivityManager: Process org.umr.app started".to_string(),
        "FATAL EXCEPTION: main".to_string(),
        "SIGSEGV (signal 11) at 0x00000000".to_string(),
        "backtrace: #00 pc 0000000000049280 /apex/com.android.runtime/lib64/bionic/libc.so".to_string(),
        "backtrace: #01 pc 0000000000085430 /system/lib64/libart.so".to_string(),
    ];

    let crash = watchdog.inspect_crash_logs(&raw_logs).expect("Must detect crash");
    assert_eq!(crash.signal, Some(11));
    assert!(crash.reason.contains("FATAL EXCEPTION") || crash.reason.contains("SIGSEGV"));
    assert!(!crash.stack_trace.is_empty());

    // Crash containment
    let mut session = SessionRecord::new("org.umr.app");
    session.transition_to(LifecycleState::Preparing).unwrap();
    session.transition_to(LifecycleState::Running).unwrap();

    watchdog.handle_crash(&mut session, crash).unwrap();
    assert_eq!(session.state, LifecycleState::Failed);
    assert!(session.last_error.is_some());
    let err = session.last_error.unwrap();
    assert_eq!(err.code, ErrorCode::ErrBackendProcessCrashed);
    assert!(err.message.contains("signal 11"));
}

#[test]
fn test_android_virtualization_backend_full_lifecycle() {
    let tmp = tempdir().unwrap();
    let mock_transport = Box::new(MockGuestTransport::new());

    let mut backend = AndroidVirtualizationBackend::with_transport(tmp.path(), mock_transport).unwrap();
    assert!(backend.id().starts_with("android-virt-"));

    let pkg = create_test_package("org.umr.synthetic.e2e", 1, "/test/path/app.apk");
    let mut session = SessionRecord::new("org.umr.synthetic.e2e");

    // 1. Prepare
    session.transition_to(LifecycleState::Preparing).unwrap();
    backend.prepare(&mut session, &pkg).unwrap();
    assert!(backend.partition_config().is_some());

    // 2. Launch
    session.transition_to(LifecycleState::Running).unwrap();
    backend.launch(&mut session).unwrap();

    // 3. Poll Status
    let status = backend.poll_status(&session).unwrap();
    assert_eq!(status, LifecycleState::Running);

    // 4. Send Input (touch + back key)
    backend.send_input(&session, InputEvent::pointer(0.5, 0.5, PointerAction::Down)).unwrap();
    backend.send_input(&session, InputEvent::navigation(NavigationButton::Back)).unwrap();

    // 5. Capture Frame
    let frame = backend.capture_frame(&session).unwrap();
    assert!(frame.is_some());
    let fb = frame.unwrap();
    assert_eq!(fb.width, 1080);
    assert_eq!(fb.height, 1920);

    // 6. Collect Diagnostics
    let diags = backend.collect_diagnostics(&session).unwrap();
    assert!(!diags.is_empty());

    // 7. Stop Session
    session.transition_to(LifecycleState::Stopping).unwrap();
    backend.stop(&mut session).unwrap();
    session.mark_terminated(0).unwrap();
    assert_eq!(session.state, LifecycleState::Terminated);

    // 8. Wipe Storage
    backend.wipe_storage(&session).unwrap();
}
