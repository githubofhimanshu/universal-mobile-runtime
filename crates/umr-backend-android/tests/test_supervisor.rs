use std::fs;
use tempfile::tempdir;

use umr_backend_android::supervisor::{AvdProvisioner, GuestSupervisor, SystemImageInfo};
use umr_backend_android::transport::MockGuestTransport;

#[test]
fn test_discover_synthetic_system_images() {
    let tmp = tempdir().unwrap();
    let sdk_root = tmp.path();

    // Create synthetic system image tree: system-images/android-34/google_apis/x86_64/
    let img_dir = sdk_root
        .join("system-images")
        .join("android-34")
        .join("google_apis")
        .join("x86_64");
    fs::create_dir_all(&img_dir).unwrap();
    fs::write(img_dir.join("system.img"), b"dummy-system-img").unwrap();

    let discovered = AvdProvisioner::discover_system_images(sdk_root);
    assert_eq!(discovered.len(), 1);
    assert_eq!(discovered[0].api_level, 34);
    assert_eq!(discovered[0].tag, "google_apis");
    assert_eq!(discovered[0].abi, "x86_64");
}

#[test]
fn test_provision_avd_if_missing() {
    let tmp_sdk = tempdir().unwrap();
    let tmp_avd = tempdir().unwrap();

    // Set ANDROID_AVD_HOME to test directory
    std::env::set_var("ANDROID_AVD_HOME", tmp_avd.path());

    let sys_image = SystemImageInfo {
        api_level: 34,
        tag: "google_apis".to_string(),
        abi: "x86_64".to_string(),
        path: tmp_sdk.path().join("system-images/android-34/google_apis/x86_64"),
    };

    let avd_dir = AvdProvisioner::provision_avd_if_missing(
        "test_guest_avd",
        &sys_image,
        tmp_sdk.path(),
    )
    .expect("provisioning must succeed");

    assert!(avd_dir.exists());
    let ini_file = tmp_avd.path().join("test_guest_avd.ini");
    assert!(ini_file.exists());
    let ini_content = fs::read_to_string(&ini_file).unwrap();
    assert!(ini_content.contains("test_guest_avd.avd"));
    assert!(ini_content.contains("target=android-34"));

    let config_file = avd_dir.join("config.ini");
    assert!(config_file.exists());
    let config_content = fs::read_to_string(&config_file).unwrap();
    assert!(config_content.contains("hw.gpu.enabled=yes"));
    assert!(config_content.contains("hw.cpu.ncore=4"));
    assert!(config_content.contains("hw.ramSize=2048"));
    assert!(config_content.contains("abi.type=x86_64"));
}

#[test]
fn test_guest_supervisor_online_bypass() {
    let mock_transport = MockGuestTransport::new();
    let supervisor = GuestSupervisor::new("test_avd");

    // If transport is already alive, ensure_running completes immediately without spawning
    let res = supervisor.ensure_running(&mock_transport, None);
    assert!(res.is_ok());
}

#[test]
fn test_guest_supervisor_interactive_configuration() {
    let sup_default = GuestSupervisor::new("test_avd");
    assert!(sup_default.is_interactive());
    assert!(sup_default.is_enabled());

    let sup_headless = GuestSupervisor::headless("test_avd");
    assert!(!sup_headless.is_interactive());
    assert!(sup_headless.is_enabled());

    let sup_disabled = GuestSupervisor::disabled();
    assert!(!sup_disabled.is_enabled());
}

