use crate::provenance::{ProvenanceOrigin, ProvenanceRecord, ProvenanceStatus};
use crate::publisher::PublisherRegistry;
use crate::DistributionError;
use chrono::Utc;
use sha2::{Digest, Sha256};

/// Computes a hexadecimal SHA-256 hash of arbitrary bytes.
pub fn compute_package_hash(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// Generates a cryptographic signature token over canonical metadata and package hash
/// using a publisher key secret.
pub fn generate_signature(
    publisher_key: &str,
    app_id: &str,
    version_code: u64,
    package_hash: &str,
) -> String {
    let canonical = format!("{app_id}:{version_code}:{package_hash}");
    let mut hasher = Sha256::new();
    hasher.update(publisher_key.as_bytes());
    hasher.update(b":umr_sig_v1:");
    hasher.update(canonical.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Verifies a signature token against canonical metadata, package hash, and publisher key.
pub fn verify_signature(
    publisher_key: &str,
    app_id: &str,
    version_code: u64,
    package_hash: &str,
    signature: &str,
) -> bool {
    let expected = generate_signature(publisher_key, app_id, version_code, package_hash);
    // Constant-time style comparison
    if expected.len() != signature.len() {
        return false;
    }
    expected.as_bytes().iter().zip(signature.as_bytes()).fold(0, |acc, (a, b)| acc | (a ^ b)) == 0
}

/// Provenance and integrity verifier.
pub struct ProvenanceVerifier;

impl ProvenanceVerifier {
    /// Verifies a package binary against a publisher identity and signature.
    pub fn verify_package(
        app_id: &str,
        version: &str,
        version_code: u64,
        package_bytes: &[u8],
        publisher_id: &str,
        signature: &str,
        registry: &PublisherRegistry,
    ) -> Result<ProvenanceRecord, DistributionError> {
        let actual_hash = compute_package_hash(package_bytes);

        let publisher = match registry.get(publisher_id) {
            Some(p) => p,
            None => {
                return Ok(ProvenanceRecord {
                    app_id: app_id.to_string(),
                    version: version.to_string(),
                    version_code,
                    origin: ProvenanceOrigin::LocalImport {
                        source_path: "unknown".to_string(),
                    },
                    package_hash_sha256: actual_hash,
                    publisher_id: Some(publisher_id.to_string()),
                    status: ProvenanceStatus::Untrusted {
                        reason: format!("Publisher '{publisher_id}' not found in registry"),
                    },
                    imported_at: Utc::now(),
                    verified: false,
                });
            }
        };

        let sig_valid = verify_signature(
            &publisher.public_key,
            app_id,
            version_code,
            &actual_hash,
            signature,
        );

        if !sig_valid {
            return Ok(ProvenanceRecord {
                app_id: app_id.to_string(),
                version: version.to_string(),
                version_code,
                origin: ProvenanceOrigin::LocalImport {
                    source_path: "unknown".to_string(),
                },
                package_hash_sha256: actual_hash,
                publisher_id: Some(publisher_id.to_string()),
                status: ProvenanceStatus::SignatureInvalid {
                    reason: "Cryptographic signature check failed against publisher public key".to_string(),
                },
                imported_at: Utc::now(),
                verified: false,
            });
        }

        let status = if publisher.verified {
            ProvenanceStatus::CatalogVerified
        } else {
            ProvenanceStatus::DeveloperSideload
        };

        Ok(ProvenanceRecord {
            app_id: app_id.to_string(),
            version: version.to_string(),
            version_code,
            origin: ProvenanceOrigin::CuratedCatalog {
                catalog_version: "1.0".to_string(),
            },
            package_hash_sha256: actual_hash,
            publisher_id: Some(publisher_id.to_string()),
            status,
            imported_at: Utc::now(),
            verified: true,
        })
    }

    /// Verifies an application update package against an existing installed application.
    ///
    /// Guarantees:
    /// 1. Publisher identity matches existing installation.
    /// 2. Rollback/downgrade attacks are rejected (`update_version_code >= current.version_code`).
    /// 3. Package signature is valid.
    pub fn verify_update(
        current: &ProvenanceRecord,
        update_version: &str,
        update_version_code: u64,
        update_package_bytes: &[u8],
        update_publisher_id: &str,
        update_signature: &str,
        registry: &PublisherRegistry,
    ) -> Result<ProvenanceRecord, DistributionError> {
        // Check 1: Publisher continuity
        if let Some(curr_pub) = &current.publisher_id {
            if curr_pub != update_publisher_id {
                return Err(DistributionError::PublisherMismatch {
                    current: curr_pub.clone(),
                    requested: update_publisher_id.to_string(),
                });
            }
        }

        // Check 2: Downgrade / rollback prevention
        if update_version_code < current.version_code {
            return Err(DistributionError::DowngradeDisallowed {
                current: current.version_code,
                requested: update_version_code,
            });
        }

        // Check 3: Cryptographic verification of update package
        let new_record = Self::verify_package(
            &current.app_id,
            update_version,
            update_version_code,
            update_package_bytes,
            update_publisher_id,
            update_signature,
            registry,
        )?;

        if !new_record.verified {
            return Err(DistributionError::InvalidSignature(format!(
                "Update package verification failed: {:?}",
                new_record.status
            )));
        }

        Ok(new_record)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signature_roundtrip() {
        let key = "sample_pub_key_1234";
        let app_id = "com.example.app";
        let vcode = 200;
        let hash = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

        let sig = generate_signature(key, app_id, vcode, hash);
        assert!(verify_signature(key, app_id, vcode, hash, &sig));

        // Tamper with hash
        assert!(!verify_signature(key, app_id, vcode, "tampered_hash", &sig));
        // Tamper with version_code
        assert!(!verify_signature(key, app_id, 201, hash, &sig));
        // Tamper with key
        assert!(!verify_signature("wrong_key", app_id, vcode, hash, &sig));
    }
}

