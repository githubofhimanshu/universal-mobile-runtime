//! # UMR Distribution Subsystem
//!
//! Provides curated application catalog indexing, publisher key registration,
//! cryptographic provenance verification, anti-downgrade protections,
//! licensing compliance, remote execution privacy gating, and sanitized
//! support report generation.

pub mod catalog;
pub mod license;
pub mod privacy;
pub mod provenance;
pub mod publisher;
pub mod support_report;
pub mod verifier;

pub use catalog::{CatalogEntry, CatalogFilter, CuratedCatalog, PackageFamily};
pub use license::{AttributionNotice, LicenseAcceptanceRecord, LicenseManager};
pub use privacy::{RemoteConsentRecord, RemotePrivacyGate};
pub use provenance::{ProvenanceLedger, ProvenanceOrigin, ProvenanceRecord, ProvenanceStatus};
pub use publisher::{Publisher, PublisherRegistry};
pub use support_report::{SupportReport, SupportReportGenerator};
pub use verifier::{
    compute_package_hash, generate_signature, verify_signature, ProvenanceVerifier,
};

use thiserror::Error;

/// Error taxonomy for the distribution and ecosystem subsystem.
#[derive(Debug, Error)]
pub enum DistributionError {
    #[error("Publisher '{0}' not found in registry")]
    PublisherNotFound(String),

    #[error("Cryptographic signature verification failed: {0}")]
    InvalidSignature(String),

    #[error("Publisher identity mismatch for update: currently signed by '{current}', but update is signed by '{requested}'")]
    PublisherMismatch { current: String, requested: String },

    #[error("Downgrade attack prevented: cannot install version code {requested} over installed version code {current}")]
    DowngradeDisallowed { current: u64, requested: u64 },

    #[error("Explicit EULA / license terms acceptance required for '{app_id}' version {version}")]
    LicenseConsentRequired { app_id: String, version: String },

    #[error("Explicit user consent required before starting remote execution with host '{host_endpoint}'")]
    RemoteConsentRequired { host_endpoint: String },

    #[error("DRM protection violation: {0}")]
    DrmEncountered(String),

    #[error("Package data corrupted or malformed: {0}")]
    CorruptedPackage(String),
}
