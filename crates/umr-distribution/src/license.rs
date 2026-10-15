use crate::DistributionError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// An open source license attribution notice for third-party dependencies.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AttributionNotice {
    pub component_name: String,
    pub license_spdx: String,
    pub source_url: String,
    pub notice_text: String,
}

/// Record tracking user acceptance of license terms or EULA.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LicenseAcceptanceRecord {
    pub app_id: String,
    pub version: String,
    pub license_type: String,
    pub accepted_at: DateTime<Utc>,
    pub terms_hash: String,
    pub user_consent: bool,
}

/// Licensing and compliance manager.
#[derive(Debug, Clone, Default)]
pub struct LicenseManager {
    acceptance_ledger: HashMap<String, LicenseAcceptanceRecord>,
}

impl LicenseManager {
    /// Creates a new license manager.
    pub fn new() -> Self {
        Self {
            acceptance_ledger: HashMap::new(),
        }
    }

    /// Records user acceptance of application license terms or EULA.
    pub fn record_acceptance(
        &mut self,
        app_id: &str,
        version: &str,
        license_type: &str,
        terms_text: &str,
    ) -> LicenseAcceptanceRecord {
        let mut hasher = Sha256::new();
        hasher.update(terms_text.as_bytes());
        let terms_hash = format!("{:x}", hasher.finalize());

        let record = LicenseAcceptanceRecord {
            app_id: app_id.to_string(),
            version: version.to_string(),
            license_type: license_type.to_string(),
            accepted_at: Utc::now(),
            terms_hash,
            user_consent: true,
        };

        let key = format!("{app_id}:{version}");
        self.acceptance_ledger.insert(key, record.clone());
        record
    }

    /// Checks if a license has been accepted for a specific application version.
    pub fn is_license_accepted(&self, app_id: &str, version: &str) -> bool {
        let key = format!("{app_id}:{version}");
        self.acceptance_ledger
            .get(&key)
            .map_or(false, |r| r.user_consent)
    }

    /// Gating check: verifies whether launch is permitted under licensing terms.
    pub fn check_launch_permitted(
        &self,
        app_id: &str,
        version: &str,
        requires_explicit_eula: bool,
    ) -> Result<(), DistributionError> {
        if requires_explicit_eula && !self.is_license_accepted(app_id, version) {
            return Err(DistributionError::LicenseConsentRequired {
                app_id: app_id.to_string(),
                version: version.to_string(),
            });
        }
        Ok(())
    }

    /// Validates package compliance against FairPlay DRM.
    /// Strictly rejects encrypted App Store packages (NFR-LIC-01, ADR 0006).
    pub fn check_fairplay_compliance(&self, is_drm_protected: bool) -> Result<(), DistributionError> {
        if is_drm_protected {
            return Err(DistributionError::DrmEncountered(
                "Apple FairPlay DRM detected (cryptid != 0). Execution of encrypted commercial App Store packages is prohibited.".to_string(),
            ));
        }
        Ok(())
    }

    /// Generates third-party open-source attribution notices.
    pub fn get_open_source_attributions(&self) -> Vec<AttributionNotice> {
        vec![
            AttributionNotice {
                component_name: "Rust Standard Library".to_string(),
                license_spdx: "MIT OR Apache-2.0".to_string(),
                source_url: "https://www.rust-lang.org/".to_string(),
                notice_text: "Copyright (c) The Rust Project Developers".to_string(),
            },
            AttributionNotice {
                component_name: "serde".to_string(),
                license_spdx: "MIT OR Apache-2.0".to_string(),
                source_url: "https://github.com/serde-rs/serde".to_string(),
                notice_text: "Copyright (c) 2014-2024 Erick Tryzelaar and David Tolnay".to_string(),
            },
            AttributionNotice {
                component_name: "sha2".to_string(),
                license_spdx: "MIT OR Apache-2.0".to_string(),
                source_url: "https://github.com/RustCrypto/hashes".to_string(),
                notice_text: "Copyright (c) 2016-2024 RustCrypto Project".to_string(),
            },
            AttributionNotice {
                component_name: "chrono".to_string(),
                license_spdx: "MIT OR Apache-2.0".to_string(),
                source_url: "https://github.com/chronotope/chrono".to_string(),
                notice_text: "Copyright (c) 2014-2024 Kang Seonghoon and Chrono contributors".to_string(),
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_license_acceptance_flow() {
        let mut manager = LicenseManager::new();
        let app_id = "com.sample.app";
        let version = "1.0.0";

        assert!(!manager.is_license_accepted(app_id, version));
        assert!(manager.check_launch_permitted(app_id, version, false).is_ok());
        assert!(manager.check_launch_permitted(app_id, version, true).is_err());

        manager.record_acceptance(app_id, version, "Proprietary-EULA", "Agreement terms text...");
        assert!(manager.is_license_accepted(app_id, version));
        assert!(manager.check_launch_permitted(app_id, version, true).is_ok());
    }

    #[test]
    fn test_fairplay_drm_rejection() {
        let manager = LicenseManager::new();
        assert!(manager.check_fairplay_compliance(false).is_ok());
        let res = manager.check_fairplay_compliance(true);
        assert!(matches!(res, Err(DistributionError::DrmEncountered(_))));
    }
}

