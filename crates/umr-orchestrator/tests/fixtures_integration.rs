use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::PathBuf;

use tempfile::tempdir;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

use umr_backend_reference::ReferenceAndroidBackend;
use umr_core::{
    ErrorCode, ExecutionMode, LifecycleState, NegotiationResult, PackageFormat,
};
use umr_intake_worker::inspect_package;
use umr_orchestrator::RuntimeOrchestrator;

fn project_fixtures_dir() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // crates/umr-orchestrator -> ../../tests/fixtures
    dir.pop();
    dir.pop();
    dir.join("tests").join("fixtures")
}

fn create_synthetic_fixtures() {
    let dir = project_fixtures_dir();
    let _ = create_dir_all(&dir);

    // 1. tests/fixtures/synthetic-app.apk
    let apk_path = dir.join("synthetic-app.apk");
    let file = File::create(&apk_path).unwrap();
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default();

    zip.start_file("AndroidManifest.xml", options).unwrap();
    let manifest_xml = r#"<manifest xmlns:android="http://schemas.android.com/apk/res/android"
        package="org.umr.synthetic.hello"
        versionCode="1"
        versionName="1.0.0">
        <uses-sdk android:minSdkVersion="21" android:targetSdkVersion="34" />
        <application android:name=".App" android:label="SyntheticHello">
            <activity android:name=".MainActivity" android:exported="true">
                <intent-filter>
                    <action android:name="android.intent.action.MAIN" />
                    <category android:name="android.intent.category.LAUNCHER" />
                </intent-filter>
            </activity>
        </application>
    </manifest>"#;
    zip.write_all(manifest_xml.as_bytes()).unwrap();

    zip.start_file("classes.dex", options).unwrap();
    zip.write_all(b"dex\n035\0\x12\x34\x56\x78synthetic_dalvik_bytecode").unwrap();

    zip.start_file("res/values/strings.xml", options).unwrap();
    zip.write_all(b"<resources><string name=\"app_name\">SyntheticHello</string></resources>").unwrap();

    zip.finish().unwrap();

    // 2. tests/fixtures/malformed-slip.apk (Zip Slip traversal)
    let slip_path = dir.join("malformed-slip.apk");
    let file = File::create(&slip_path).unwrap();
    let mut zip = ZipWriter::new(file);
    zip.start_file("../../etc/malicious_payload.sh", options).unwrap();
    zip.write_all(b"#!/bin/sh\nrm -rf /\n").unwrap();
    zip.finish().unwrap();

    // 3. tests/fixtures/encrypted-mock.ipa (FairPlay encrypted)
    let enc_ipa_path = dir.join("encrypted-mock.ipa");
    let file = File::create(&enc_ipa_path).unwrap();
    let mut zip = ZipWriter::new(file);

    zip.start_file("Payload/EncryptedApp.app/Info.plist", options).unwrap();
    let plist_enc = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleIdentifier</key>
    <string>com.umr.encrypted.mock</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0</string>
    <key>CFBundleExecutable</key>
    <string>EncryptedApp</string>
</dict>
</plist>"#;
    zip.write_all(plist_enc.as_bytes()).unwrap();

    zip.start_file("Payload/EncryptedApp.app/EncryptedApp", options).unwrap();
    let mut macho_enc = Vec::new();
    macho_enc.extend_from_slice(&0xfeedfacfu32.to_le_bytes()); // MH_MAGIC_64
    macho_enc.extend_from_slice(&0x0100000cu32.to_le_bytes()); // CPU_TYPE_ARM64
    macho_enc.extend_from_slice(&0u32.to_le_bytes());          // subtype
    macho_enc.extend_from_slice(&2u32.to_le_bytes());          // MH_EXECUTE
    macho_enc.extend_from_slice(&1u32.to_le_bytes());          // ncmds = 1
    macho_enc.extend_from_slice(&32u32.to_le_bytes());         // sizeofcmds = 32
    macho_enc.extend_from_slice(&0u32.to_le_bytes());          // flags
    macho_enc.extend_from_slice(&0u32.to_le_bytes());          // reserved
    // LC_ENCRYPTION_INFO_64
    macho_enc.extend_from_slice(&0x2cu32.to_le_bytes());       // cmd (0x2c)
    macho_enc.extend_from_slice(&32u32.to_le_bytes());         // cmdsize (32)
    macho_enc.extend_from_slice(&4096u32.to_le_bytes());       // cryptoff
    macho_enc.extend_from_slice(&16384u32.to_le_bytes());      // cryptsize
    macho_enc.extend_from_slice(&1u32.to_le_bytes());          // cryptid = 1 (ENCRYPTED)
    macho_enc.extend_from_slice(&0u32.to_le_bytes());          // pad
    zip.write_all(&macho_enc).unwrap();
    zip.finish().unwrap();

    // 4. tests/fixtures/valid-sim.ipa (iOS Simulator unencrypted build)
    let sim_ipa_path = dir.join("valid-sim.ipa");
    let file = File::create(&sim_ipa_path).unwrap();
    let mut zip = ZipWriter::new(file);

    zip.start_file("Payload/SimApp.app/Info.plist", options).unwrap();
    let plist_sim = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleIdentifier</key>
    <string>com.umr.sim.mock</string>
    <key>CFBundleVersion</key>
    <string>2</string>
    <key>CFBundleShortVersionString</key>
    <string>2.0.0</string>
    <key>MinimumOSVersion</key>
    <string>16.0</string>
    <key>CFBundleExecutable</key>
    <string>SimApp</string>
</dict>
</plist>"#;
    zip.write_all(plist_sim.as_bytes()).unwrap();

    zip.start_file("Payload/SimApp.app/SimApp", options).unwrap();
    let mut macho_sim = Vec::new();
    macho_sim.extend_from_slice(&0xfeedfacfu32.to_le_bytes()); // MH_MAGIC_64
    macho_sim.extend_from_slice(&0x01000007u32.to_le_bytes()); // CPU_TYPE_X86_64
    macho_sim.extend_from_slice(&3u32.to_le_bytes());          // subtype
    macho_sim.extend_from_slice(&2u32.to_le_bytes());          // MH_EXECUTE
    macho_sim.extend_from_slice(&0u32.to_le_bytes());          // ncmds
    macho_sim.extend_from_slice(&0u32.to_le_bytes());          // sizeofcmds
    macho_sim.extend_from_slice(&0u32.to_le_bytes());          // flags
    macho_sim.extend_from_slice(&0u32.to_le_bytes());          // reserved
    zip.write_all(&macho_sim).unwrap();
    zip.finish().unwrap();
}

#[test]
fn test_fixtures_generation_and_full_vertical_slice() {
    create_synthetic_fixtures();
    let fixtures_dir = project_fixtures_dir();

    // 1. Test Valid Synthetic Android APK
    let apk_path = fixtures_dir.join("synthetic-app.apk");
    assert!(apk_path.exists());

    let metadata = inspect_package(&apk_path).expect("Must inspect valid synthetic APK");
    assert_eq!(metadata.format, PackageFormat::Apk);
    assert_eq!(metadata.package_id(), "org.umr.synthetic.hello");
    assert!(!metadata.sha256_hash.is_empty());

    // Initialize Orchestrator with Reference Backend
    let sandbox_dir = tempdir().unwrap();
    let backend = ReferenceAndroidBackend::new(sandbox_dir.path());
    let mut orch = RuntimeOrchestrator::new(PathBuf::from("mock-worker")).with_macos_override(false);
    orch.register_android(Box::new(backend));

    // Negotiate
    let negotiation = orch.negotiate(&metadata);
    match negotiation {
        NegotiationResult::Compatible {
            backend_id,
            execution_mode,
        } => {
            assert_eq!(backend_id, "android-reference-backend");
            assert_eq!(execution_mode, ExecutionMode::LocalNative);
        }
        _ => panic!("Expected compatible negotiation for synthetic APK"),
    }

    // Execute full lifecycle
    let session_id = orch
        .prepare_session(metadata, "android-reference-backend")
        .expect("Prepare must succeed");
    assert_eq!(orch.poll_session(&session_id).unwrap(), LifecycleState::Preparing);

    orch.launch_session(&session_id).expect("Launch must succeed");
    assert_eq!(orch.poll_session(&session_id).unwrap(), LifecycleState::Running);

    orch.stop_session(&session_id).expect("Stop must succeed");
    assert_eq!(orch.poll_session(&session_id).unwrap(), LifecycleState::Terminated);

    let report = orch.get_report(&session_id).unwrap();
    assert!(!report.entries.is_empty());

    // 2. Test Negative: Malformed Zip Slip APK
    let slip_path = fixtures_dir.join("malformed-slip.apk");
    assert!(slip_path.exists());
    let slip_res = inspect_package(&slip_path);
    assert!(slip_res.is_err());
    let slip_err = slip_res.unwrap_err();
    assert_eq!(slip_err.code, ErrorCode::ErrIntakeZipTraversal);

    // 3. Test Negative: FairPlay Encrypted IPA
    let enc_path = fixtures_dir.join("encrypted-mock.ipa");
    assert!(enc_path.exists());
    let enc_meta = inspect_package(&enc_path).expect("Must parse metadata of encrypted IPA");
    assert!(enc_meta.is_ios_encrypted());
    let enc_neg = orch.negotiate(&enc_meta);
    match enc_neg {
        NegotiationResult::Incompatible { error } => {
            assert_eq!(error.code, ErrorCode::ErrIntakeDrmProtected);
        }
        _ => panic!("Expected incompatible negotiation for FairPlay encrypted IPA"),
    }

    // 4. Test iOS Simulator IPA on Non-macOS
    let sim_path = fixtures_dir.join("valid-sim.ipa");
    assert!(sim_path.exists());
    let sim_meta = inspect_package(&sim_path).expect("Must parse simulator IPA");
    let sim_neg = orch.negotiate(&sim_meta);
    match sim_neg {
        NegotiationResult::RemoteRequired { suggested_mode, .. } => {
            assert_eq!(suggested_mode, ExecutionMode::RemoteStream);
        }
        _ => panic!("Expected RemoteRequired classification on Windows"),
    }
}

