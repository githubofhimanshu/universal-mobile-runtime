use umr_distribution::{
    compute_package_hash, generate_signature, CatalogEntry, CatalogFilter, CuratedCatalog,
    DistributionError, LicenseManager, PackageFamily, ProvenanceLedger, ProvenanceOrigin,
    ProvenanceStatus, ProvenanceVerifier, Publisher, PublisherRegistry, RemotePrivacyGate,
    SupportReportGenerator,
};

#[test]
fn test_publisher_registry_and_verification() {
    let mut registry = PublisherRegistry::with_official_publishers();
    assert!(registry.is_verified("pub_umr_official"));
    assert!(registry.is_verified("pub_sample_corp"));
    assert!(!registry.is_verified("pub_unverified_dev"));

    registry.register(Publisher {
        id: "pub_verified_partner".to_string(),
        name: "Verified Partner Inc.".to_string(),
        public_key: "partner_public_key_abc123".to_string(),
        contact_uri: "https://partner.com".to_string(),
        verified: true,
        created_at: chrono::Utc::now(),
    });

    assert!(registry.is_verified("pub_verified_partner"));
    assert_eq!(registry.list().len(), 4);
}

#[test]
fn test_curated_catalog_query_and_filtering() {
    let registry = PublisherRegistry::with_official_publishers();
    let catalog = CuratedCatalog::with_sample_apps(&registry).expect("Catalog initialization failed");

    assert_eq!(catalog.len(), 2);

    // Search by query "calculator"
    let calc_filter = CatalogFilter {
        query: Some("calculator".to_string()),
        ..Default::default()
    };
    let results = catalog.search(&calc_filter, &registry);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].app_id, "com.umr.sample.calculator");

    // Filter by Android package family
    let android_filter = CatalogFilter {
        package_family: Some(PackageFamily::Android),
        ..Default::default()
    };
    let android_results = catalog.search(&android_filter, &registry);
    assert_eq!(android_results.len(), 1);
    assert_eq!(android_results[0].package_family, PackageFamily::Android);

    // Filter by iOS package family
    let ios_filter = CatalogFilter {
        package_family: Some(PackageFamily::Ios),
        ..Default::default()
    };
    let ios_results = catalog.search(&ios_filter, &registry);
    assert_eq!(ios_results.len(), 1);
    assert_eq!(ios_results[0].app_id, "com.umr.vintage.clock");

    // Filter by Windows host
    let win_filter = CatalogFilter {
        host_os: Some("windows".to_string()),
        ..Default::default()
    };
    assert_eq!(catalog.search(&win_filter, &registry).len(), 2);
}

#[test]
fn test_catalog_tampered_signature_rejected() {
    let registry = PublisherRegistry::with_official_publishers();
    let mut catalog = CuratedCatalog::new();

    let invalid_entry = CatalogEntry {
        app_id: "com.malicious.app".to_string(),
        publisher_id: "pub_umr_official".to_string(),
        name: "Fake App".to_string(),
        description: "Spoofed signature app".to_string(),
        version: "1.0".to_string(),
        version_code: 1,
        package_family: PackageFamily::Android,
        package_hash_sha256: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
        signature: "invalid_tampered_signature_token".to_string(),
        min_runtime_version: "0.1.0".to_string(),
        license_type: "MIT".to_string(),
        privacy_disclosures: vec![],
        supported_hosts: vec!["windows".to_string()],
        supported_architectures: vec!["x86_64".to_string()],
    };

    let result = catalog.add_entry(invalid_entry, &registry);
    assert!(result.is_err());
    assert!(matches!(result, Err(DistributionError::InvalidSignature(_))));
}

#[test]
fn test_provenance_verification_and_ledger() {
    let registry = PublisherRegistry::with_official_publishers();
    let pub_key = &registry.get("pub_umr_official").unwrap().public_key;

    let app_id = "com.trusted.app";
    let version = "1.0.0";
    let version_code = 100;
    let package_bytes = b"fake binary apk content payload";
    let hash = compute_package_hash(package_bytes);
    let valid_sig = generate_signature(pub_key, app_id, version_code, &hash);

    // Verify package
    let prov = ProvenanceVerifier::verify_package(
        app_id,
        version,
        version_code,
        package_bytes,
        "pub_umr_official",
        &valid_sig,
        &registry,
    ).expect("verification failed");

    assert!(prov.verified);
    assert_eq!(prov.status, ProvenanceStatus::CatalogVerified);
    assert_eq!(prov.package_hash_sha256, hash);

    // Add to ledger
    let mut ledger = ProvenanceLedger::new();
    ledger.record(prov.clone());
    assert_eq!(ledger.list().len(), 1);
    assert_eq!(ledger.get(app_id).unwrap().version, "1.0.0");
}

#[test]
fn test_update_verification_and_downgrade_prevention() {
    let registry = PublisherRegistry::with_official_publishers();
    let pub_key = &registry.get("pub_umr_official").unwrap().public_key;

    let current = umr_distribution::ProvenanceRecord {
        app_id: "com.trusted.app".to_string(),
        version: "2.0.0".to_string(),
        version_code: 200,
        origin: ProvenanceOrigin::CuratedCatalog { catalog_version: "1.0".to_string() },
        package_hash_sha256: "hash_v2".to_string(),
        publisher_id: Some("pub_umr_official".to_string()),
        status: ProvenanceStatus::CatalogVerified,
        imported_at: chrono::Utc::now(),
        verified: true,
    };

    // Valid upgrade to version_code 201
    let upgrade_bytes = b"upgraded binary v2.1";
    let up_hash = compute_package_hash(upgrade_bytes);
    let up_sig = generate_signature(pub_key, "com.trusted.app", 201, &up_hash);

    let upgrade_res = ProvenanceVerifier::verify_update(
        &current,
        "2.1.0",
        201,
        upgrade_bytes,
        "pub_umr_official",
        &up_sig,
        &registry,
    );
    assert!(upgrade_res.is_ok());
    let upgraded = upgrade_res.unwrap();
    assert_eq!(upgraded.version_code, 201);

    // Rollback / Downgrade Attack attempt: trying to install version_code 199 over 200
    let downgrade_bytes = b"vulnerable older binary";
    let down_hash = compute_package_hash(downgrade_bytes);
    let down_sig = generate_signature(pub_key, "com.trusted.app", 199, &down_hash);

    let downgrade_res = ProvenanceVerifier::verify_update(
        &current,
        "1.9.0",
        199,
        downgrade_bytes,
        "pub_umr_official",
        &down_sig,
        &registry,
    );
    assert!(downgrade_res.is_err());
    match downgrade_res.unwrap_err() {
        DistributionError::DowngradeDisallowed { current, requested } => {
            assert_eq!(current, 200);
            assert_eq!(requested, 199);
        }
        other => panic!("Expected DowngradeDisallowed, got {:?}", other),
    }

    // Publisher mismatch attempt: update signed by different publisher
    let spoof_res = ProvenanceVerifier::verify_update(
        &current,
        "2.2.0",
        202,
        upgrade_bytes,
        "pub_sample_corp",
        &up_sig,
        &registry,
    );
    assert!(matches!(spoof_res, Err(DistributionError::PublisherMismatch { .. })));
}

#[test]
fn test_licensing_and_drm_compliance() {
    let mut lic_mgr = LicenseManager::new();

    // FairPlay DRM rejection test
    assert!(lic_mgr.check_fairplay_compliance(false).is_ok());
    assert!(matches!(
        lic_mgr.check_fairplay_compliance(true),
        Err(DistributionError::DrmEncountered(_))
    ));

    // Mandatory EULA gating
    let app_id = "com.acme.enterprise";
    let version = "3.4.0";

    assert!(lic_mgr.check_launch_permitted(app_id, version, false).is_ok());
    assert!(lic_mgr.check_launch_permitted(app_id, version, true).is_err());

    let record = lic_mgr.record_acceptance(app_id, version, "Commercial-EULA", "Confidential terms text");
    assert!(record.user_consent);
    assert!(lic_mgr.check_launch_permitted(app_id, version, true).is_ok());

    // Attribution generation
    let attributions = lic_mgr.get_open_source_attributions();
    assert!(!attributions.is_empty());
    assert!(attributions.iter().any(|a| a.component_name == "Rust Standard Library"));
}

#[test]
fn test_remote_privacy_gating_and_labeling() {
    let mut gate = RemotePrivacyGate::new();
    let runner_endpoint = "https://mac-mini-runner.corp.internal:9000";

    // Track C session without consent must be blocked
    let unconsented = gate.check_remote_execution_permitted(runner_endpoint);
    assert!(matches!(unconsented, Err(DistributionError::RemoteConsentRequired { .. })));

    // Record consent
    let consent = gate.record_consent("test_user", runner_endpoint, true, true);
    assert_eq!(consent.terms_version, RemotePrivacyGate::CURRENT_TERMS_VERSION);
    assert!(gate.check_remote_execution_permitted(runner_endpoint).is_ok());

    // Watermark check
    assert_eq!(gate.get_watermark_label(false), None);
    assert_eq!(
        gate.get_watermark_label(true),
        Some("RemoteStream (Device Track C)".to_string())
    );
}

#[test]
fn test_support_report_scrubbing_and_generation() {
    let raw_logs = vec![
        "[INFO] Session initialized for user bob@company.com on host".to_string(),
        "[DEBUG] Auth token used: Bearer secret_jwt_token_xyz12345 in header".to_string(),
        "[DEBUG] Saved config to C:\\Users\\Administrator\\AppData\\Local\\UMR\\config.json".to_string(),
        "[ERROR] Database login failed with password=SuperTopSecretPwd! at 127.0.0.1".to_string(),
    ];

    let report = SupportReportGenerator::generate(
        "windows",
        "x86_64",
        "0.1.0",
        vec![],
        &raw_logs,
    );

    assert_eq!(report.host_os, "windows");
    assert_eq!(report.host_cpu, "x86_64");
    assert_eq!(report.sanitized_logs.len(), 4);

    let joined = report.sanitized_logs.join("\n");
    assert!(!joined.contains("bob@company.com"));
    assert!(joined.contains("[REDACTED_EMAIL]"));

    assert!(!joined.contains("secret_jwt_token_xyz12345"));
    assert!(joined.contains("Bearer [REDACTED_SECRET]"));

    assert!(!joined.contains("C:\\Users\\Administrator"));
    assert!(joined.contains("[REDACTED_PATH]"));

    assert!(!joined.contains("SuperTopSecretPwd!"));
    assert!(joined.contains("password=[REDACTED_SECRET]"));
}
