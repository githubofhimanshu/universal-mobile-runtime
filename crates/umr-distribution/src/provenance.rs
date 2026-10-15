use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Source of an imported application package.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProvenanceOrigin {
    CuratedCatalog { catalog_version: String },
    DeveloperSideload { developer_tag: String },
    LocalImport { source_path: String },
    RemoteStream { host_endpoint: String },
}

/// Verification status of an application package.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProvenanceStatus {
    /// Verified against an official or recognized publisher with valid signature.
    CatalogVerified,
    /// Unsigned or self-signed developer package allowed for testing.
    DeveloperSideload,
    /// Package has a signature, but verification failed (tampered content or key mismatch).
    SignatureInvalid { reason: String },
    /// Unknown, untrusted, or unregistered source.
    Untrusted { reason: String },
}

/// A cryptographic provenance receipt for an installed or imported package.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProvenanceRecord {
    pub app_id: String,
    pub version: String,
    pub version_code: u64,
    pub origin: ProvenanceOrigin,
    pub package_hash_sha256: String,
    pub publisher_id: Option<String>,
    pub status: ProvenanceStatus,
    pub imported_at: DateTime<Utc>,
    pub verified: bool,
}

/// Persistent/in-memory ledger of package provenance receipts.
#[derive(Debug, Clone, Default)]
pub struct ProvenanceLedger {
    records: HashMap<String, ProvenanceRecord>,
}

impl ProvenanceLedger {
    /// Creates a new provenance ledger.
    pub fn new() -> Self {
        Self {
            records: HashMap::new(),
        }
    }

    /// Stores or updates a provenance record.
    pub fn record(&mut self, record: ProvenanceRecord) {
        self.records.insert(record.app_id.clone(), record);
    }

    /// Retrieves a provenance record by app ID.
    pub fn get(&self, app_id: &str) -> Option<&ProvenanceRecord> {
        self.records.get(app_id)
    }

    /// Removes a provenance record.
    pub fn remove(&mut self, app_id: &str) -> Option<ProvenanceRecord> {
        self.records.remove(app_id)
    }

    /// Lists all provenance records.
    pub fn list(&self) -> Vec<&ProvenanceRecord> {
        self.records.values().collect()
    }
}

