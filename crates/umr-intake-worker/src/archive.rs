use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use sha2::{Digest, Sha256};
use zip::ZipArchive;

use umr_core::{DiagnosticError, ErrorCode, UmrResult};

pub const MAX_ARCHIVE_FILE_SIZE: u64 = 4 * 1024 * 1024 * 1024; // 4 GB
pub const MAX_ARCHIVE_ENTRIES: usize = 20_000;
pub const MAX_TOTAL_UNCOMPRESSED_SIZE: u64 = 4 * 1024 * 1024 * 1024; // 4 GB
pub const MAX_COMPRESSION_RATIO: u64 = 100;
pub const MAX_SINGLE_FILE_EXTRACT_SIZE: u64 = 50 * 1024 * 1024; // 50 MB

/// Validates entry path against Zip Slip directory traversal and dangerous characters.
pub fn validate_entry_path(path: &str) -> UmrResult<()> {
    // 1. Disallow null bytes
    if path.contains('\0') {
        return Err(DiagnosticError::new(
            ErrorCode::ErrIntakeZipTraversal,
            "Archive entry contains illegal null byte",
        ));
    }

    // 2. Disallow absolute paths (Windows and Unix)
    if path.starts_with('/') || path.starts_with('\\') {
        return Err(DiagnosticError::zip_traversal(path));
    }
    if path.len() >= 2 && path.chars().nth(1) == Some(':') {
        return Err(DiagnosticError::zip_traversal(path));
    }

    // 3. Disallow directory traversal components ('..')
    let normalized = path.replace('\\', "/");
    for segment in normalized.split('/') {
        if segment == ".." {
            return Err(DiagnosticError::zip_traversal(path));
        }
    }

    Ok(())
}

/// Computes the SHA-256 checksum and file size of an archive.
pub fn hash_and_measure_file(path: &Path) -> UmrResult<(String, u64)> {
    let file = File::open(path).map_err(|e| {
        DiagnosticError::new(
            ErrorCode::ErrIntakeMalformedArchive,
            format!("Failed to open package file '{}': {}", path.display(), e),
        )
    })?;

    let metadata = file.metadata().map_err(|e| {
        DiagnosticError::new(
            ErrorCode::ErrIntakeMalformedArchive,
            format!("Failed to query package file metadata: {}", e),
        )
    })?;

    let file_size = metadata.len();
    if file_size > MAX_ARCHIVE_FILE_SIZE {
        return Err(DiagnosticError::new(
            ErrorCode::ErrIntakeLimitExceeded,
            format!(
                "Package file size ({} bytes) exceeds maximum limit of {} bytes",
                file_size, MAX_ARCHIVE_FILE_SIZE
            ),
        ));
    }

    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];

    loop {
        let count = reader.read(&mut buffer).map_err(|e| {
            DiagnosticError::new(
                ErrorCode::ErrIntakeMalformedArchive,
                format!("I/O error while reading package: {}", e),
            )
        })?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }

    let hash_hex = format!("{:x}", hasher.finalize());
    Ok((hash_hex, file_size))
}

/// A validated, defensive ZIP archive inspector.
pub struct SafeZipReader {
    archive: ZipArchive<BufReader<File>>,
    entry_names: Vec<String>,
}

impl SafeZipReader {
    /// Opens and performs exhaustive security quota checks on the ZIP central directory.
    pub fn open(path: &Path) -> UmrResult<Self> {
        let file = File::open(path).map_err(|e| {
            DiagnosticError::new(
                ErrorCode::ErrIntakeMalformedArchive,
                format!("Failed to open package: {}", e),
            )
        })?;

        let reader = BufReader::new(file);
        let mut archive = ZipArchive::new(reader).map_err(|e| {
            DiagnosticError::new(
                ErrorCode::ErrIntakeMalformedArchive,
                format!("Invalid ZIP archive format: {}", e),
            )
        })?;

        let entry_count = archive.len();
        if entry_count > MAX_ARCHIVE_ENTRIES {
            return Err(DiagnosticError::new(
                ErrorCode::ErrIntakeLimitExceeded,
                format!(
                    "Archive entry count ({}) exceeds maximum limit ({})",
                    entry_count, MAX_ARCHIVE_ENTRIES
                ),
            ));
        }

        let mut total_uncompressed_size: u64 = 0;
        let mut total_compressed_size: u64 = 0;
        let mut entry_names = Vec::with_capacity(entry_count);

        for i in 0..entry_count {
            let entry = archive.by_index_raw(i).map_err(|e| {
                DiagnosticError::new(
                    ErrorCode::ErrIntakeMalformedArchive,
                    format!("Corrupted archive entry header at index {}: {}", i, e),
                )
            })?;

            let name = entry.name().to_string();
            validate_entry_path(&name)?;

            let uncompressed = entry.size();
            let compressed = entry.compressed_size();

            // Check single entry compression ratio if compressed > 0
            if compressed > 0 && (uncompressed / compressed) > MAX_COMPRESSION_RATIO {
                return Err(DiagnosticError::zip_bomb(format!(
                    "Entry '{}' ratio exceeds 100:1 ({} bytes compressed vs {} uncompressed)",
                    name, compressed, uncompressed
                )));
            }

            total_uncompressed_size = total_uncompressed_size.saturating_add(uncompressed);
            total_compressed_size = total_compressed_size.saturating_add(compressed);

            if total_uncompressed_size > MAX_TOTAL_UNCOMPRESSED_SIZE {
                return Err(DiagnosticError::zip_bomb(format!(
                    "Total uncompressed archive size exceeds {} bytes",
                    MAX_TOTAL_UNCOMPRESSED_SIZE
                )));
            }

            entry_names.push(name);
        }

        Ok(Self {
            archive,
            entry_names,
        })
    }

    pub fn entries(&self) -> &[String] {
        &self.entry_names
    }

    /// Safely reads the full content of an entry by name, enforcing single-file size limits.
    pub fn read_entry(&mut self, name: &str) -> UmrResult<Vec<u8>> {
        let mut entry = self.archive.by_name(name).map_err(|_| {
            DiagnosticError::new(
                ErrorCode::ErrIntakeMissingManifest,
                format!("Entry '{}' not found in package", name),
            )
        })?;

        let size = entry.size();
        if size > MAX_SINGLE_FILE_EXTRACT_SIZE {
            return Err(DiagnosticError::new(
                ErrorCode::ErrIntakeLimitExceeded,
                format!(
                    "Entry '{}' size ({} bytes) exceeds single-file extraction limit ({} bytes)",
                    name, size, MAX_SINGLE_FILE_EXTRACT_SIZE
                ),
            ));
        }

        let mut data = Vec::with_capacity(size.min(1024 * 1024) as usize);
        entry.read_to_end(&mut data).map_err(|e| {
            DiagnosticError::new(
                ErrorCode::ErrIntakeMalformedArchive,
                format!("Failed to extract entry '{}': {}", name, e),
            )
        })?;

        Ok(data)
    }

    /// Finds the first entry matching a predicate.
    pub fn find_entry<F>(&self, predicate: F) -> Option<String>
    where
        F: Fn(&str) -> bool,
    {
        self.entry_names.iter().find(|name| predicate(name)).cloned()
    }
}

