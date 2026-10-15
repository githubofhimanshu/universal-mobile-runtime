pub mod archive;
pub mod android_manifest;
pub mod ios_plist;
pub mod macho;

use std::path::Path;
use chrono::Utc;

use umr_core::{
    CpuArchitecture, DiagnosticError, EncryptionStatus, ErrorCode, IosPackageInfo,
    PackageDetails, PackageFormat, PackageMetadata, UmrResult,
};

use archive::{hash_and_measure_file, SafeZipReader};

/// Inspects a candidate mobile application package (APK or IPA) with defensive isolation checks.
pub fn inspect_package(path: &Path) -> UmrResult<PackageMetadata> {
    // 1. Measure and compute SHA-256
    let (sha256_hash, file_size_bytes) = hash_and_measure_file(path)?;

    // 2. Open and perform quota-enforced archive inspection
    let mut zip = SafeZipReader::open(path)?;

    // 3. Detect format and dispatch to specific parser
    let has_android_manifest = zip.entries().iter().any(|e| e == "AndroidManifest.xml");
    let has_payload_folder = zip.entries().iter().any(|e| e.starts_with("Payload/") && e.contains(".app/"));

    let (format, details) = if has_android_manifest {
        let manifest_bytes = zip.read_entry("AndroidManifest.xml")?;
        let info = android_manifest::parse_manifest(&manifest_bytes, zip.entries())?;
        (PackageFormat::Apk, PackageDetails::Android(info))
    } else if has_payload_folder {
        // Find Info.plist
        let plist_path = zip
            .find_entry(|e| e.starts_with("Payload/") && e.ends_with(".app/Info.plist"))
            .ok_or_else(|| {
                DiagnosticError::new(
                    ErrorCode::ErrIntakeMissingManifest,
                    "IPA is missing Info.plist inside Payload/*.app",
                )
            })?;

        let plist_bytes = zip.read_entry(&plist_path)?;
        let parsed_plist = ios_plist::parse_info_plist(&plist_bytes)?;

        // Locate main Mach-O executable
        let app_dir = plist_path.trim_end_matches("Info.plist");
        let (archs, encryption, is_sim) = if let Some(exec_name) = &parsed_plist.executable_name {
            let exec_path = format!("{}{}", app_dir, exec_name);
            if let Ok(exec_bytes) = zip.read_entry(&exec_path) {
                let inspection = macho::inspect_macho(&exec_bytes)?;
                (
                    inspection.architectures,
                    inspection.encryption,
                    inspection.is_simulator_build,
                )
            } else {
                (vec![CpuArchitecture::Arm64], EncryptionStatus::Unknown, false)
            }
        } else {
            (vec![CpuArchitecture::Arm64], EncryptionStatus::Unknown, false)
        };

        let ios_info = IosPackageInfo {
            bundle_identifier: parsed_plist.bundle_identifier,
            bundle_version: parsed_plist.bundle_version,
            bundle_short_version: parsed_plist.bundle_short_version,
            minimum_os_version: parsed_plist.minimum_os_version,
            architectures: archs,
            encryption,
            is_simulator_build: is_sim,
        };

        (PackageFormat::Ipa, PackageDetails::Ios(ios_info))
    } else {
        return Err(DiagnosticError::new(
            ErrorCode::ErrIntakeUnsupportedFormat,
            format!(
                "Package '{}' does not contain an AndroidManifest.xml or iOS Payload directory",
                path.display()
            ),
        )
        .with_suggestion("Ensure the package is a valid .apk or .ipa archive."));
    };

    Ok(PackageMetadata {
        format,
        file_path: path.to_string_lossy().to_string(),
        file_size_bytes,
        sha256_hash,
        details,
        inspected_at: Utc::now(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    #[test]
    fn test_zip_slip_traversal_rejection() {
        let temp = NamedTempFile::new().unwrap();
        let mut zip = ZipWriter::new(temp.as_file());

        // Attempt writing traversal entry
        let options = SimpleFileOptions::default();
        zip.start_file("../../etc/malicious.txt", options).unwrap();
        zip.write_all(b"evil").unwrap();
        zip.finish().unwrap();

        let result = inspect_package(temp.path());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, ErrorCode::ErrIntakeZipTraversal);
    }

    #[test]
    fn test_valid_synthetic_apk_inspection() {
        let temp = NamedTempFile::new().unwrap();
        let mut zip = ZipWriter::new(temp.as_file());
        let options = SimpleFileOptions::default();

        // Add text AndroidManifest.xml
        zip.start_file("AndroidManifest.xml", options).unwrap();
        let manifest_xml = r#"<manifest xmlns:android="http://schemas.android.com/apk/res/android"
            package="org.umr.synthetic"
            versionCode="42"
            versionName="1.2.3">
            <uses-sdk android:minSdkVersion="26" android:targetSdkVersion="34" />
        </manifest>"#;
        zip.write_all(manifest_xml.as_bytes()).unwrap();

        // Add pure dex file
        zip.start_file("classes.dex", options).unwrap();
        zip.write_all(b"dex\n035\0mockdexcode").unwrap();

        zip.finish().unwrap();

        let metadata = inspect_package(temp.path()).expect("Valid APK must parse");
        assert_eq!(metadata.format, PackageFormat::Apk);
        assert_eq!(metadata.package_id(), "org.umr.synthetic");

        if let PackageDetails::Android(info) = metadata.details {
            assert_eq!(info.version_code, Some(42));
            assert_eq!(info.version_name, Some("1.2.3".to_string()));
            assert_eq!(info.min_sdk_version, Some(26));
            assert_eq!(info.target_sdk_version, Some(34));
            assert!(info.is_pure_dex);
        } else {
            panic!("Expected Android package details");
        }
    }

    #[test]
    fn test_valid_synthetic_ipa_inspection() {
        let temp = NamedTempFile::new().unwrap();
        let mut zip = ZipWriter::new(temp.as_file());
        let options = SimpleFileOptions::default();

        // Add Info.plist
        zip.start_file("Payload/DemoApp.app/Info.plist", options).unwrap();
        let plist_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleIdentifier</key>
    <string>com.umr.iosdemo</string>
    <key>CFBundleVersion</key>
    <string>101</string>
    <key>CFBundleShortVersionString</key>
    <string>2.1.0</string>
    <key>MinimumOSVersion</key>
    <string>15.0</string>
    <key>CFBundleExecutable</key>
    <string>DemoApp</string>
</dict>
</plist>"#;
        zip.write_all(plist_content.as_bytes()).unwrap();

        // Add mock Mach-O (thin 64-bit x86_64 simulator)
        zip.start_file("Payload/DemoApp.app/DemoApp", options).unwrap();
        let mut macho_bytes = Vec::new();
        // Magic MH_MAGIC_64 (0xfeedfacf)
        macho_bytes.extend_from_slice(&0xfeedfacfu32.to_le_bytes());
        // CPU_TYPE_X86_64 (0x01000007)
        macho_bytes.extend_from_slice(&0x01000007u32.to_le_bytes());
        // subtype (3)
        macho_bytes.extend_from_slice(&3u32.to_le_bytes());
        // filetype (2 = MH_EXECUTE)
        macho_bytes.extend_from_slice(&2u32.to_le_bytes());
        // ncmds (0)
        macho_bytes.extend_from_slice(&0u32.to_le_bytes());
        // sizeofcmds (0)
        macho_bytes.extend_from_slice(&0u32.to_le_bytes());
        // flags (0)
        macho_bytes.extend_from_slice(&0u32.to_le_bytes());
        // reserved (0)
        macho_bytes.extend_from_slice(&0u32.to_le_bytes());

        zip.write_all(&macho_bytes).unwrap();
        zip.finish().unwrap();

        let metadata = inspect_package(temp.path()).expect("Valid IPA must parse");
        assert_eq!(metadata.format, PackageFormat::Ipa);
        assert_eq!(metadata.package_id(), "com.umr.iosdemo");

        if let PackageDetails::Ios(info) = metadata.details {
            assert_eq!(info.bundle_version, Some("101".to_string()));
            assert_eq!(info.bundle_short_version, Some("2.1.0".to_string()));
            assert_eq!(info.minimum_os_version, Some("15.0".to_string()));
            assert!(info.is_simulator_build);
            assert_eq!(info.encryption, EncryptionStatus::Unencrypted);
        } else {
            panic!("Expected iOS package details");
        }
    }

    #[test]
    fn test_apk_with_native_abis() {
        let temp = NamedTempFile::new().unwrap();
        let mut zip = ZipWriter::new(temp.as_file());
        let options = SimpleFileOptions::default();

        zip.start_file("AndroidManifest.xml", options).unwrap();
        let manifest = r#"<manifest package="org.umr.nativeapp" versionCode="1" versionName="1.0" />"#;
        zip.write_all(manifest.as_bytes()).unwrap();

        // Add native libraries
        zip.start_file("lib/arm64-v8a/libnative.so", options).unwrap();
        zip.write_all(b"\x7fELFfakebinary").unwrap();
        zip.start_file("lib/x86_64/libnative.so", options).unwrap();
        zip.write_all(b"\x7fELFfakebinary").unwrap();

        zip.finish().unwrap();

        let metadata = inspect_package(temp.path()).unwrap();
        if let PackageDetails::Android(info) = metadata.details {
            assert!(!info.is_pure_dex);
            assert!(info.native_abis.contains(&umr_core::TargetAbi::Arm64V8a));
            assert!(info.native_abis.contains(&umr_core::TargetAbi::X86_64));
        } else {
            panic!("Expected Android metadata");
        }
    }

    #[test]
    fn test_encrypted_ipa_detection() {
        let temp = NamedTempFile::new().unwrap();
        let mut zip = ZipWriter::new(temp.as_file());
        let options = SimpleFileOptions::default();

        zip.start_file("Payload/App.app/Info.plist", options).unwrap();
        let plist = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleIdentifier</key>
    <string>com.umr.encrypted</string>
    <key>CFBundleExecutable</key>
    <string>App</string>
</dict>
</plist>"#;
        zip.write_all(plist.as_bytes()).unwrap();

        // Add Mach-O with LC_ENCRYPTION_INFO_64
        zip.start_file("Payload/App.app/App", options).unwrap();
        let mut macho = Vec::new();
        // Magic MH_MAGIC_64 (0xfeedfacf)
        macho.extend_from_slice(&0xfeedfacfu32.to_le_bytes());
        // CPU_TYPE_ARM64 (0x0100000c)
        macho.extend_from_slice(&0x0100000cu32.to_le_bytes());
        // subtype
        macho.extend_from_slice(&0u32.to_le_bytes());
        // filetype (MH_EXECUTE = 2)
        macho.extend_from_slice(&2u32.to_le_bytes());
        // ncmds (1)
        macho.extend_from_slice(&1u32.to_le_bytes());
        // sizeofcmds (32)
        macho.extend_from_slice(&32u32.to_le_bytes());
        // flags (0)
        macho.extend_from_slice(&0u32.to_le_bytes());
        // reserved (0)
        macho.extend_from_slice(&0u32.to_le_bytes());

        // Load Command: LC_ENCRYPTION_INFO_64 (0x2c)
        macho.extend_from_slice(&0x2cu32.to_le_bytes()); // cmd
        macho.extend_from_slice(&32u32.to_le_bytes());   // cmdsize (32 bytes)
        macho.extend_from_slice(&4096u32.to_le_bytes()); // cryptoff
        macho.extend_from_slice(&8192u32.to_le_bytes()); // cryptsize
        macho.extend_from_slice(&1u32.to_le_bytes());    // cryptid = 1 (ENCRYPTED)
        macho.extend_from_slice(&0u32.to_le_bytes());    // pad

        zip.write_all(&macho).unwrap();
        zip.finish().unwrap();

        let metadata = inspect_package(temp.path()).unwrap();
        assert!(metadata.is_ios_encrypted());
        if let PackageDetails::Ios(info) = metadata.details {
            assert_eq!(info.encryption, EncryptionStatus::FairPlayEncrypted { cryptid: 1 });
        } else {
            panic!("Expected iOS metadata");
        }
    }
}

