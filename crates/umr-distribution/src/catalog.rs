use crate::publisher::PublisherRegistry;
use crate::verifier::verify_signature;
use crate::DistributionError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Application package family.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PackageFamily {
    Android,
    Ios,
}

/// A curated catalog entry representing a verified mobile application.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CatalogEntry {
    pub app_id: String,
    pub publisher_id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub version_code: u64,
    pub package_family: PackageFamily,
    pub package_hash_sha256: String,
    pub signature: String,
    pub min_runtime_version: String,
    pub license_type: String,
    pub privacy_disclosures: Vec<String>,
    pub supported_hosts: Vec<String>,
    pub supported_architectures: Vec<String>,
}

/// Filter options for querying the curated catalog.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CatalogFilter {
    pub query: Option<String>,
    pub package_family: Option<PackageFamily>,
    pub host_os: Option<String>,
    pub host_arch: Option<String>,
    pub verified_publisher_only: bool,
}

/// Curated catalog engine managing the directory of verified applications.
#[derive(Debug, Clone)]
pub struct CuratedCatalog {
    entries: HashMap<String, CatalogEntry>,
}

impl Default for CuratedCatalog {
    fn default() -> Self {
        Self::new()
    }
}

impl CuratedCatalog {
    /// Creates a new empty catalog.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Seeds a catalog with standard sample apps verified against official publishers.
    pub fn with_sample_apps(registry: &PublisherRegistry) -> Result<Self, DistributionError> {
        let mut catalog = Self::new();

        // Sample Android App 1
        let a1_id = "com.umr.sample.calculator";
        let a1_vcode = 100;
        let a1_hash = "a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90";
        let a1_pub = registry.get("pub_umr_official").ok_or_else(|| {
            DistributionError::PublisherNotFound("pub_umr_official".to_string())
        })?;
        let a1_sig = crate::verifier::generate_signature(&a1_pub.public_key, a1_id, a1_vcode, a1_hash);

        catalog.add_entry(
            CatalogEntry {
                app_id: a1_id.to_string(),
                publisher_id: "pub_umr_official".to_string(),
                name: "Universal Calculator".to_string(),
                description: "A fast, cross-platform desktop calculator app.".to_string(),
                version: "1.0.0".to_string(),
                version_code: a1_vcode,
                package_family: PackageFamily::Android,
                package_hash_sha256: a1_hash.to_string(),
                signature: a1_sig,
                min_runtime_version: "0.1.0".to_string(),
                license_type: "Apache-2.0".to_string(),
                privacy_disclosures: vec!["Offline Only".to_string()],
                supported_hosts: vec!["windows".to_string(), "macos".to_string(), "linux".to_string()],
                supported_architectures: vec!["x86_64".to_string(), "arm64".to_string()],
            },
            registry,
        )?;

        // Sample iOS 32-bit Vintage App
        let i1_id = "com.umr.vintage.clock";
        let i1_vcode = 10;
        let i1_hash = "b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90a1";
        let i1_pub = registry.get("pub_sample_corp").ok_or_else(|| {
            DistributionError::PublisherNotFound("pub_sample_corp".to_string())
        })?;
        let i1_sig = crate::verifier::generate_signature(&i1_pub.public_key, i1_id, i1_vcode, i1_hash);

        catalog.add_entry(
            CatalogEntry {
                app_id: i1_id.to_string(),
                publisher_id: "pub_sample_corp".to_string(),
                name: "Vintage Clock (iOS 3.1 HLE)".to_string(),
                description: "Classic iPhone OS 3.1 clock utility for vintage emulation.".to_string(),
                version: "1.0".to_string(),
                version_code: i1_vcode,
                package_family: PackageFamily::Ios,
                package_hash_sha256: i1_hash.to_string(),
                signature: i1_sig,
                min_runtime_version: "0.1.0".to_string(),
                license_type: "MIT".to_string(),
                privacy_disclosures: vec!["Offline Only".to_string()],
                supported_hosts: vec!["windows".to_string(), "macos".to_string(), "linux".to_string()],
                supported_architectures: vec!["x86_64".to_string(), "arm64".to_string()],
            },
            registry,
        )?;

        Ok(catalog)
    }

    /// Adds a catalog entry after verifying publisher authenticity and cryptographic signature.
    pub fn add_entry(
        &mut self,
        entry: CatalogEntry,
        registry: &PublisherRegistry,
    ) -> Result<(), DistributionError> {
        let publisher = registry
            .get(&entry.publisher_id)
            .ok_or_else(|| DistributionError::PublisherNotFound(entry.publisher_id.clone()))?;

        let is_valid = verify_signature(
            &publisher.public_key,
            &entry.app_id,
            entry.version_code,
            &entry.package_hash_sha256,
            &entry.signature,
        );

        if !is_valid {
            return Err(DistributionError::InvalidSignature(format!(
                "Signature verification failed for catalog entry '{}' by publisher '{}'",
                entry.app_id, entry.publisher_id
            )));
        }

        self.entries.insert(entry.app_id.clone(), entry);
        Ok(())
    }

    /// Retrieves an entry by app ID.
    pub fn get_entry(&self, app_id: &str) -> Option<&CatalogEntry> {
        self.entries.get(app_id)
    }

    /// Searches the catalog applying the given filter criteria.
    pub fn search(&self, filter: &CatalogFilter, registry: &PublisherRegistry) -> Vec<&CatalogEntry> {
        self.entries
            .values()
            .filter(|entry| {
                if let Some(query) = &filter.query {
                    let q = query.to_lowercase();
                    if !entry.app_id.to_lowercase().contains(&q)
                        && !entry.name.to_lowercase().contains(&q)
                        && !entry.description.to_lowercase().contains(&q)
                    {
                        return false;
                    }
                }

                if let Some(family) = &filter.package_family {
                    if &entry.package_family != family {
                        return false;
                    }
                }

                if let Some(host_os) = &filter.host_os {
                    if !entry.supported_hosts.iter().any(|h| h.eq_ignore_ascii_case(host_os)) {
                        return false;
                    }
                }

                if let Some(host_arch) = &filter.host_arch {
                    if !entry.supported_architectures.iter().any(|a| a.eq_ignore_ascii_case(host_arch)) {
                        return false;
                    }
                }

                if filter.verified_publisher_only && !registry.is_verified(&entry.publisher_id) {
                    return false;
                }

                true
            })
            .collect()
    }

    /// Returns all catalog entries.
    pub fn list_all(&self) -> Vec<&CatalogEntry> {
        self.entries.values().collect()
    }

    /// Returns count of apps in catalog.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns true if empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
