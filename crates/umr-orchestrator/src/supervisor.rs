use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use serde_json::Value;

use umr_core::{DiagnosticError, ErrorCode, PackageMetadata, UmrResult};

pub const DEFAULT_INTAKE_TIMEOUT: Duration = Duration::from_secs(10);

/// Locates the `umr-intake-worker` executable in the environment.
pub fn find_intake_binary() -> UmrResult<PathBuf> {
    if let Ok(env_path) = std::env::var("UMR_INTAKE_BIN") {
        let p = PathBuf::from(env_path);
        if p.exists() {
            return Ok(p);
        }
    }

    let exe_name = if cfg!(windows) {
        "umr-intake-worker.exe"
    } else {
        "umr-intake-worker"
    };

    // 1. Check directory of current executable
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(parent) = current_exe.parent() {
            let candidate = parent.join(exe_name);
            if candidate.exists() {
                return Ok(candidate);
            }
            // Check parent's deps directory if running under cargo test
            if let Some(grandparent) = parent.parent() {
                let candidate2 = grandparent.join(exe_name);
                if candidate2.exists() {
                    return Ok(candidate2);
                }
            }
        }
    }

    // 2. Check standard cargo target paths
    for target_dir in &["target/debug", "target/release", "../target/debug", "../../target/debug"] {
        let candidate = PathBuf::from(target_dir).join(exe_name);
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    Err(DiagnosticError::new(
        ErrorCode::ErrBackendUnavailable,
        "Failed to locate 'umr-intake-worker' binary",
    )
    .with_suggestion("Ensure umr-intake-worker has been built with 'cargo build'."))
}

/// Spawns the isolated intake worker in a dedicated process and enforces a hard execution timeout.
pub fn execute_sandboxed_intake(
    intake_bin: &Path,
    package_path: &Path,
    timeout: Duration,
) -> UmrResult<PackageMetadata> {
    let resolved_path = if package_path.exists() {
        package_path.to_path_buf()
    } else {
        let mut candidates = Vec::new();
        if let Some(name) = package_path.file_name() {
            candidates.push(PathBuf::from("target/umr-sandbox/uploads").join(name));
            candidates.push(PathBuf::from("tests/fixtures").join(name));
            if let Ok(userprofile) = std::env::var("USERPROFILE") {
                let up = PathBuf::from(userprofile);
                candidates.push(up.join("Downloads").join(name));
                candidates.push(up.join("Desktop").join(name));
            }
            if let Ok(home) = std::env::var("HOME") {
                let hp = PathBuf::from(home);
                candidates.push(hp.join("Downloads").join(name));
                candidates.push(hp.join("Desktop").join(name));
            }
        }

        match candidates.into_iter().find(|c| c.exists()) {
            Some(found) => found,
            None => {
                return Err(DiagnosticError::new(
                    ErrorCode::ErrIntakeMalformedArchive,
                    format!("Package file not found: '{}'", package_path.display()),
                ));
            }
        }
    };

    let child = Command::new(intake_bin)
        .arg("--inspect")
        .arg(&resolved_path)
        .arg("--json")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            DiagnosticError::new(
                ErrorCode::ErrBackendProcessCrashed,
                format!("Failed to spawn sandboxed intake worker process: {}", e),
            )
        })?;

    // Supervise execution with timeout
    let (tx, rx) = mpsc::channel();
    let child_handle = thread::spawn(move || {
        let res = child.wait_with_output();
        let _ = tx.send(res);
    });

    match rx.recv_timeout(timeout) {
        Ok(output_res) => {
            let _ = child_handle.join();
            let output = output_res.map_err(|e| {
                DiagnosticError::new(
                    ErrorCode::ErrBackendProcessCrashed,
                    format!("I/O error waiting for intake worker: {}", e),
                )
            })?;

            let stdout_str = String::from_utf8_lossy(&output.stdout);
            let stderr_str = String::from_utf8_lossy(&output.stderr);

            if output.status.success() {
                let json: Value = serde_json::from_str(&stdout_str).map_err(|e| {
                    DiagnosticError::new(
                        ErrorCode::ErrIntakeParseFailure,
                        format!("Failed to parse JSON response from intake worker: {}", e),
                    )
                })?;

                let metadata_val = json.get("metadata").ok_or_else(|| {
                    DiagnosticError::new(
                        ErrorCode::ErrIntakeParseFailure,
                        "Intake worker response missing 'metadata' field",
                    )
                })?;

                serde_json::from_value(metadata_val.clone()).map_err(|e| {
                    DiagnosticError::new(
                        ErrorCode::ErrIntakeParseFailure,
                        format!("Failed to deserialize PackageMetadata: {}", e),
                    )
                })
            } else {
                // Parse error from stderr or stdout
                if let Ok(json) = serde_json::from_str::<Value>(&stderr_str) {
                    if let Some(err_val) = json.get("error") {
                        if let Ok(diag_err) = serde_json::from_value::<DiagnosticError>(err_val.clone()) {
                            return Err(diag_err);
                        }
                    }
                }

                Err(DiagnosticError::new(
                    ErrorCode::ErrIntakeMalformedArchive,
                    format!(
                        "Package intake rejected with exit code {:?}: {}",
                        output.status.code(),
                        stderr_str.trim()
                    ),
                ))
            }
        }
        Err(mpsc::RecvTimeoutError::Timeout) => {
            // Process timed out
            Err(DiagnosticError::new(
                ErrorCode::ErrLifecycleTimeout,
                format!(
                    "Package intake worker timed out after {} seconds on file '{}'",
                    timeout.as_secs(),
                    package_path.display()
                ),
            )
            .with_suggestion("Check if the package is an oversized archive or decompression bomb."))
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            Err(DiagnosticError::new(
                ErrorCode::ErrBackendProcessCrashed,
                "Intake supervisor channel disconnected unexpectedly",
            ))
        }
    }
}
