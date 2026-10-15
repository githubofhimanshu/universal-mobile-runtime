use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use umr_core::{
    DiagnosticError, ErrorCode, PackageDetails, PackageMetadata, UmrResult,
};

/// Record of an installed Android application package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledPackageRecord {
    pub package_name: String,
    pub version_code: u64,
    pub version_name: String,
    pub min_sdk_version: u32,
    pub target_sdk_version: u32,
    pub apk_path: PathBuf,
    pub sha256_hash: String,
    pub installed_at: DateTime<Utc>,
    pub last_launched_at: Option<DateTime<Utc>>,
    pub data_dir: PathBuf,
    pub sdcard_dir: PathBuf,
    pub storage_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PackageCatalog {
    packages: HashMap<String, InstalledPackageRecord>,
}

/// Package manager governing application installation, upgrades, downgrades, and uninstallation.
pub struct AndroidPackageManager {
    sandbox_root: PathBuf,
    catalog_path: PathBuf,
    catalog: PackageCatalog,
}

impl AndroidPackageManager {
    pub fn new(sandbox_root: &Path) -> UmrResult<Self> {
        let pkg_dir = sandbox_root.join("packages");
        fs::create_dir_all(&pkg_dir).map_err(|e| {
            DiagnosticError::new(
                ErrorCode::ErrBackendSandboxFailure,
                format!("Failed to create packages directory: {}", e),
            )
        })?;

        let catalog_path = pkg_dir.join("catalog.json");
        let catalog = if catalog_path.exists() {
            let data = fs::read_to_string(&catalog_path).unwrap_or_default();
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            PackageCatalog::default()
        };

        Ok(Self {
            sandbox_root: sandbox_root.to_path_buf(),
            catalog_path,
            catalog,
        })
    }

    /// Persists the package catalog to disk.
    fn save_catalog(&self) -> UmrResult<()> {
        let json = serde_json::to_string_pretty(&self.catalog).map_err(|e| {
            DiagnosticError::new(
                ErrorCode::ErrBackendSandboxFailure,
                format!("Failed to serialize package catalog: {}", e),
            )
        })?;

        fs::write(&self.catalog_path, json).map_err(|e| {
            DiagnosticError::new(
                ErrorCode::ErrBackendSandboxFailure,
                format!("Failed to write package catalog: {}", e),
            )
        })
    }

    /// Installs a new application package into the runtime environment.
    pub fn install_package(&mut self, meta: &PackageMetadata) -> UmrResult<InstalledPackageRecord> {
        let info = match &meta.details {
            PackageDetails::Android(info) => info,
            _ => {
                return Err(DiagnosticError::new(
                    ErrorCode::ErrIntakeParseFailure,
                    "Cannot install non-Android package into Android runtime",
                ));
            }
        };

        let pkg_name = &info.package_name;
        if self.catalog.packages.contains_key(pkg_name) {
            // If already installed, route through upgrade
            return self.upgrade_package(meta, false);
        }

        // Allocate sandboxed directories
        let data_dir = self.sandbox_root.join("data").join("data").join(pkg_name);
        let sdcard_dir = self
            .sandbox_root
            .join("sdcard")
            .join("Android")
            .join("data")
            .join(pkg_name);

        fs::create_dir_all(&data_dir).map_err(|e| {
            DiagnosticError::new(
                ErrorCode::ErrBackendSandboxFailure,
                format!("Failed to create package data dir: {}", e),
            )
        })?;
        fs::create_dir_all(&sdcard_dir).map_err(|e| {
            DiagnosticError::new(
                ErrorCode::ErrBackendSandboxFailure,
                format!("Failed to create package sdcard dir: {}", e),
            )
        })?;

        let record = InstalledPackageRecord {
            package_name: pkg_name.clone(),
            version_code: info.version_code.unwrap_or(1) as u64,
            version_name: info.version_name.clone().unwrap_or_else(|| "1.0".to_string()),
            min_sdk_version: info.min_sdk_version.unwrap_or(21),
            target_sdk_version: info.target_sdk_version.unwrap_or(33),
            apk_path: PathBuf::from(&meta.file_path),
            sha256_hash: meta.sha256_hash.clone(),
            installed_at: Utc::now(),
            last_launched_at: None,
            data_dir,
            sdcard_dir,
            storage_bytes: meta.file_size_bytes,
        };

        self.catalog.packages.insert(pkg_name.clone(), record.clone());
        self.save_catalog()?;
        Ok(record)
    }

    /// Upgrades an existing application package, validating version monotonicity and preserving user data.
    pub fn upgrade_package(
        &mut self,
        meta: &PackageMetadata,
        force: bool,
    ) -> UmrResult<InstalledPackageRecord> {
        let info = match &meta.details {
            PackageDetails::Android(info) => info,
            _ => {
                return Err(DiagnosticError::new(
                    ErrorCode::ErrIntakeParseFailure,
                    "Cannot upgrade non-Android package into Android runtime",
                ));
            }
        };

        let pkg_name = &info.package_name;
        let new_version_code = info.version_code.unwrap_or(1) as u64;

        let existing = self.catalog.packages.get_mut(pkg_name).ok_or_else(|| {
            DiagnosticError::new(
                ErrorCode::ErrBackendSessionFailed,
                format!("Package '{}' is not installed; use install instead", pkg_name),
            )
        })?;

        // Version monotonicity check: reject downgrade unless force == true
        if new_version_code < existing.version_code && !force {
            return Err(DiagnosticError::new(
                ErrorCode::ErrIntakeParseFailure,
                format!(
                    "Downgrade rejected: attempted to install version code {} over existing version code {}",
                    new_version_code, existing.version_code
                ),
            ));
        }

        // Update metadata while preserving data and sdcard directories
        existing.version_code = new_version_code;
        existing.version_name = info.version_name.clone().unwrap_or_else(|| "1.0".to_string());
        existing.min_sdk_version = info.min_sdk_version.unwrap_or(21);
        existing.target_sdk_version = info.target_sdk_version.unwrap_or(33);
        existing.apk_path = PathBuf::from(&meta.file_path);
        existing.sha256_hash = meta.sha256_hash.clone();
        existing.storage_bytes = meta.file_size_bytes + Self::dir_size(&existing.data_dir);

        let updated_record = existing.clone();
        self.save_catalog()?;
        Ok(updated_record)
    }

    /// Completely uninstalls a package and purges all of its sandboxed storage directories.
    pub fn uninstall_package(&mut self, package_name: &str) -> UmrResult<()> {
        let record = self.catalog.packages.remove(package_name).ok_or_else(|| {
            DiagnosticError::new(
                ErrorCode::ErrBackendSessionFailed,
                format!("Package '{}' is not installed", package_name),
            )
        })?;

        // Purge sandboxed storage directories
        if record.data_dir.exists() {
            let _ = fs::remove_dir_all(&record.data_dir);
        }
        if record.sdcard_dir.exists() {
            let _ = fs::remove_dir_all(&record.sdcard_dir);
        }

        self.save_catalog()?;
        Ok(())
    }

    /// Completely purges user data for an installed package without removing the package record.
    pub fn wipe_storage(&mut self, package_name: &str) -> UmrResult<()> {
        let record = self.catalog.packages.get_mut(package_name).ok_or_else(|| {
            DiagnosticError::new(
                ErrorCode::ErrBackendSessionFailed,
                format!("Package '{}' is not installed", package_name),
            )
        })?;

        if record.data_dir.exists() {
            let _ = fs::remove_dir_all(&record.data_dir);
            let _ = fs::create_dir_all(&record.data_dir);
        }
        if record.sdcard_dir.exists() {
            let _ = fs::remove_dir_all(&record.sdcard_dir);
            let _ = fs::create_dir_all(&record.sdcard_dir);
        }

        record.storage_bytes = Self::dir_size(&record.data_dir);
        self.save_catalog()?;
        Ok(())
    }

    pub fn get_package(&self, package_name: &str) -> Option<&InstalledPackageRecord> {
        self.catalog.packages.get(package_name)
    }

    pub fn list_packages(&self) -> Vec<InstalledPackageRecord> {
        self.catalog.packages.values().cloned().collect()
    }

    pub fn record_launch(&mut self, package_name: &str) {
        if let Some(record) = self.catalog.packages.get_mut(package_name) {
            record.last_launched_at = Some(Utc::now());
            let _ = self.save_catalog();
        }
    }

    fn dir_size(path: &Path) -> u64 {
        if !path.exists() {
            return 0;
        }
        let mut total = 0;
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                if let Ok(meta) = entry.metadata() {
                    if meta.is_file() {
                        total += meta.len();
                    } else if meta.is_dir() {
                        total += Self::dir_size(&entry.path());
                    }
                }
            }
        }
        total
    }
}
