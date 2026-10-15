pub mod runner;

pub use runner::{ReferenceAndroidBackend, SessionSandboxConfig};

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tempfile::tempdir;
    use umr_core::{
        AndroidPackageInfo, IAndroidBackend, LifecycleState, PackageDetails, PackageFormat,
        PackageMetadata, SessionRecord, TargetAbi,
    };

    #[test]
    fn test_reference_backend_full_lifecycle() {
        let temp = tempdir().unwrap();
        let mut backend = ReferenceAndroidBackend::new(temp.path());

        let pkg = PackageMetadata {
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
        };

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
}

