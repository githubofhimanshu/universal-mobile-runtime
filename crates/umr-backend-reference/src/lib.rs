pub mod framebuffer;
pub mod input;
pub mod runner;
pub mod storage;

pub use framebuffer::{DisplayMetrics, FrameBufferPipeline};
pub use input::{ActiveTouchState, AndroidInputSubsystem};
pub use runner::{ReferenceAndroidBackend, SessionSandboxConfig};
pub use storage::{AndroidStorageSandbox, StorageUsage};

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tempfile::tempdir;
    use umr_core::{
        AndroidPackageInfo, IAndroidBackend, InputEvent, LifecycleState, NavigationButton,
        PackageDetails, PackageFormat, PackageMetadata, PointerAction, SessionRecord, TargetAbi,
    };

    fn make_test_pkg() -> PackageMetadata {
        PackageMetadata {
            format: PackageFormat::Apk,
            file_path: "/dummy/path.apk".to_string(),
            file_size_bytes: 512,
            sha256_hash: "dummy_hash_123".to_string(),
            details: PackageDetails::Android(AndroidPackageInfo {
                package_name: "org.umr.synthetic.hello".to_string(),
                version_code: Some(1),
                version_name: Some("1.0".to_string()),
                min_sdk_version: Some(21),
                target_sdk_version: Some(34),
                main_activity: Some("MainActivity".to_string()),
                native_abis: vec![TargetAbi::X86_64],
                is_pure_dex: false,
            }),
            inspected_at: Utc::now(),
        }
    }

    #[test]
    fn test_reference_backend_full_lifecycle() {
        let temp = tempdir().unwrap();
        let mut backend = ReferenceAndroidBackend::new(temp.path());
        let pkg = make_test_pkg();
        let mut session = SessionRecord::new("org.umr.synthetic.hello");

        // 1. Prepare
        backend.prepare(&mut session, &pkg).expect("Prepare must succeed");
        let session_dir = temp
            .path()
            .join("sessions")
            .join(session.session_id.to_string());
        assert!(session_dir.exists());
        assert!(session_dir.join("session_config.json").exists());
        assert!(session_dir.join("logs").join("logcat.log").exists());

        // Verify storage directories were initialized
        let data_dir = session_dir
            .join("storage")
            .join("data")
            .join("data")
            .join("org.umr.synthetic.hello");
        assert!(data_dir.join("files").exists());
        assert!(data_dir.join("cache").exists());

        // 2. Launch
        backend.launch(&mut session).expect("Launch must succeed");
        assert_eq!(
            backend.poll_status(&session).unwrap(),
            LifecycleState::Running
        );

        // 3. Stop
        backend.stop(&mut session).expect("Stop must succeed");

        // 4. Collect Diagnostics
        let logs = backend.collect_diagnostics(&session).expect("Must read logs");
        assert!(!logs.is_empty());
        assert!(logs.iter().any(|l| l.contains("ActivityManager: Start proc")));
        assert!(logs.iter().any(|l| l.contains("end of session")));
    }

    #[test]
    fn test_reference_backend_subsystems() {
        let temp = tempdir().unwrap();
        let mut backend = ReferenceAndroidBackend::new(temp.path());
        let pkg = make_test_pkg();
        let mut session = SessionRecord::new("org.umr.synthetic.hello");

        backend.prepare(&mut session, &pkg).expect("Prepare must succeed");
        backend.launch(&mut session).expect("Launch must succeed");

        // A. Input Subsystem Test: Send pointer down at (0.5, 0.5)
        let touch_ev = InputEvent::pointer(0.5, 0.5, PointerAction::Down);
        backend.send_input(&session, touch_ev).expect("send input must succeed");

        // Send hardware Back button
        let back_ev = InputEvent::navigation(NavigationButton::Back);
        backend.send_input(&session, back_ev).expect("send navigation key must succeed");

        // B. Framebuffer Subsystem Test: Capture frame while touch is active
        let frame = backend.capture_frame(&session).expect("capture frame").expect("frame buffer");
        assert_eq!(frame.width, 360);
        assert_eq!(frame.height, 640);
        assert_eq!(frame.data.len(), 360 * 640 * 4);
        assert_eq!(frame.frame_index, 1);

        let stats = backend.get_frame_stats(&session.session_id).expect("frame stats");
        assert_eq!(stats.total_frames, 1);

        // C. Storage Subsystem Test: Write mock app data, check usage, then wipe
        let session_dir = temp
            .path()
            .join("sessions")
            .join(session.session_id.to_string());
        let files_dir = session_dir
            .join("storage")
            .join("data")
            .join("data")
            .join("org.umr.synthetic.hello")
            .join("files");
        std::fs::write(files_dir.join("saved_game.bin"), vec![1, 2, 3, 4, 5, 6, 7, 8]).unwrap();

        let usage_before = backend.get_storage_usage(&session.session_id).expect("storage usage");
        assert_eq!(usage_before.internal_data_bytes, 8);
        assert_eq!(usage_before.total_bytes, 8);

        // Wipe storage
        backend.wipe_storage(&session).expect("wipe storage");
        let usage_after = backend.get_storage_usage(&session.session_id).expect("storage usage after wipe");
        assert_eq!(usage_after.total_bytes, 0);

        // Stop session
        backend.stop(&mut session).expect("stop");

        // Verify logcat contains input and storage logs
        let logs = backend.collect_diagnostics(&session).expect("logs");
        assert!(logs.iter().any(|l| l.contains("MotionEvent { action=ACTION_DOWN")));
        assert!(logs.iter().any(|l| l.contains("Hardware navigation button pressed: BACK")));
        assert!(logs.iter().any(|l| l.contains("Application data and cache wiped cleanly")));
    }
}
