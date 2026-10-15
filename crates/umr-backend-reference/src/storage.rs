use std::fs::{create_dir_all, read_dir, remove_dir_all, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use umr_core::{DiagnosticError, ErrorCode, UmrResult};

/// Aggregated disk usage statistics for an application storage sandbox.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageUsage {
    pub internal_data_bytes: u64,
    pub cache_bytes: u64,
    pub external_data_bytes: u64,
    pub total_bytes: u64,
}

/// Manages isolated Android application storage directories.
pub struct AndroidStorageSandbox {
    package_name: String,
    data_dir: PathBuf,
    sdcard_dir: PathBuf,
    logcat_path: PathBuf,
}

impl AndroidStorageSandbox {
    /// Initializes the isolated Android storage hierarchy for a package.
    pub fn initialize(
        session_dir: impl AsRef<Path>,
        package_name: &str,
        logcat_path: impl AsRef<Path>,
    ) -> UmrResult<Self> {
        let base = session_dir.as_ref();
        let data_dir = base
            .join("storage")
            .join("data")
            .join("data")
            .join(package_name);
        let sdcard_dir = base
            .join("storage")
            .join("sdcard")
            .join("Android")
            .join("data")
            .join(package_name);
        let logcat_path = logcat_path.as_ref().to_path_buf();

        // Create internal data hierarchy
        create_dir_all(data_dir.join("files")).map_err(io_err)?;
        create_dir_all(data_dir.join("cache")).map_err(io_err)?;
        create_dir_all(data_dir.join("shared_prefs")).map_err(io_err)?;
        create_dir_all(data_dir.join("databases")).map_err(io_err)?;

        // Create external data hierarchy
        create_dir_all(sdcard_dir.join("files")).map_err(io_err)?;
        create_dir_all(sdcard_dir.join("cache")).map_err(io_err)?;

        let sandbox = Self {
            package_name: package_name.to_string(),
            data_dir,
            sdcard_dir,
            logcat_path,
        };

        sandbox.append_log(&format!(
            "I/StorageManager: Mounting isolated sandbox for '{}' (/data/data/{})",
            package_name, package_name
        ))?;
        sandbox.append_log(
            "I/StorageManager: External storage emulation mounted at /sdcard/Android/data",
        )?;

        Ok(sandbox)
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn sdcard_dir(&self) -> &Path {
        &self.sdcard_dir
    }

    /// Completely purges user data, cache, and preferences, recreating pristine skeleton.
    pub fn wipe(&self) -> UmrResult<()> {
        // Clear internal data directories
        self.clear_dir_contents(&self.data_dir.join("files"))?;
        self.clear_dir_contents(&self.data_dir.join("cache"))?;
        self.clear_dir_contents(&self.data_dir.join("shared_prefs"))?;
        self.clear_dir_contents(&self.data_dir.join("databases"))?;

        // Clear external data directories
        self.clear_dir_contents(&self.sdcard_dir.join("files"))?;
        self.clear_dir_contents(&self.sdcard_dir.join("cache"))?;

        self.append_log(&format!(
            "I/StorageManager: Application data and cache wiped cleanly for package '{}'",
            self.package_name
        ))?;

        Ok(())
    }

    /// Computes disk usage of the internal and external sandboxes.
    pub fn calculate_usage(&self) -> UmrResult<StorageUsage> {
        let internal_files = dir_size(&self.data_dir.join("files"))?;
        let internal_prefs = dir_size(&self.data_dir.join("shared_prefs"))?;
        let internal_db = dir_size(&self.data_dir.join("databases"))?;
        let internal_cache = dir_size(&self.data_dir.join("cache"))?;

        let external_files = dir_size(&self.sdcard_dir.join("files"))?;
        let external_cache = dir_size(&self.sdcard_dir.join("cache"))?;

        let internal_data_bytes = internal_files + internal_prefs + internal_db;
        let cache_bytes = internal_cache + external_cache;
        let external_data_bytes = external_files;
        let total_bytes = internal_data_bytes + cache_bytes + external_data_bytes;

        Ok(StorageUsage {
            internal_data_bytes,
            cache_bytes,
            external_data_bytes,
            total_bytes,
        })
    }

    fn clear_dir_contents(&self, dir: &Path) -> UmrResult<()> {
        if !dir.exists() {
            create_dir_all(dir).map_err(io_err)?;
            return Ok(());
        }
        let entries = read_dir(dir).map_err(io_err)?;
        for entry in entries {
            let entry = entry.map_err(io_err)?;
            let p = entry.path();
            if p.is_dir() {
                remove_dir_all(&p).map_err(io_err)?;
            } else {
                std::fs::remove_file(&p).map_err(io_err)?;
            }
        }
        Ok(())
    }

    fn append_log(&self, line: &str) -> UmrResult<()> {
        if !self.logcat_path.exists() {
            return Ok(());
        }
        let mut file = OpenOptions::new()
            .append(true)
            .open(&self.logcat_path)
            .map_err(io_err)?;
        writeln!(file, "{}", line).map_err(io_err)?;
        Ok(())
    }
}

fn dir_size(dir: &Path) -> UmrResult<u64> {
    if !dir.exists() {
        return Ok(0);
    }
    let mut total = 0;
    for entry in read_dir(dir).map_err(io_err)? {
        let entry = entry.map_err(io_err)?;
        let p = entry.path();
        if p.is_dir() {
            total += dir_size(&p)?;
        } else if let Ok(meta) = entry.metadata() {
            total += meta.len();
        }
    }
    Ok(total)
}

fn io_err(e: std::io::Error) -> DiagnosticError {
    DiagnosticError::new(
        ErrorCode::ErrBackendInternal,
        format!("Storage sandbox I/O error: {}", e),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_storage_sandbox_lifecycle() {
        let temp = tempdir().unwrap();
        let logcat_path = temp.path().join("logcat.log");
        std::fs::write(&logcat_path, "header\n").unwrap();

        let sandbox = AndroidStorageSandbox::initialize(
            temp.path(),
            "org.umr.demo",
            &logcat_path,
        )
        .expect("Initialize sandbox");

        assert!(sandbox.data_dir().join("files").exists());
        assert!(sandbox.data_dir().join("cache").exists());
        assert!(sandbox.sdcard_dir().join("files").exists());

        // Write sample files
        let test_file = sandbox.data_dir().join("files").join("config.dat");
        std::fs::write(&test_file, b"sample_internal_data_1234").unwrap();

        let cache_file = sandbox.data_dir().join("cache").join("temp.cache");
        std::fs::write(&cache_file, b"cached_items").unwrap();

        let usage = sandbox.calculate_usage().expect("calculate usage");
        assert_eq!(usage.internal_data_bytes, 25);
        assert_eq!(usage.cache_bytes, 12);
        assert_eq!(usage.total_bytes, 37);

        // Wipe data
        sandbox.wipe().expect("wipe sandbox");

        assert!(!test_file.exists());
        assert!(!cache_file.exists());
        assert!(sandbox.data_dir().join("files").exists());

        let usage_after = sandbox.calculate_usage().expect("calculate usage after wipe");
        assert_eq!(usage_after.total_bytes, 0);

        let logs = std::fs::read_to_string(&logcat_path).unwrap();
        assert!(logs.contains("Mounting isolated sandbox for 'org.umr.demo'"));
        assert!(logs.contains("Application data and cache wiped cleanly"));
    }
}

