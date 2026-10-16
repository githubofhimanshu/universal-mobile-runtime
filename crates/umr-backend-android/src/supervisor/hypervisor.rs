use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use umr_core::{DiagnosticError, ErrorCode, UmrResult};

use super::avd::AvdProvisioner;
use crate::transport::GuestTransport;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Autonomous supervisor for managing guest virtualization processes.
pub struct GuestSupervisor {
    avd_name: String,
    boot_timeout_secs: u64,
    enabled: bool,
    child_process: Arc<Mutex<Option<Child>>>,
}

impl Default for GuestSupervisor {
    fn default() -> Self {
        Self::new("umr_guest")
    }
}

impl GuestSupervisor {
    pub fn new(avd_name: &str) -> Self {
        Self {
            avd_name: avd_name.to_string(),
            boot_timeout_secs: 90,
            enabled: true,
            child_process: Arc::new(Mutex::new(None)),
        }
    }

    pub fn disabled() -> Self {
        Self {
            avd_name: "umr_guest".to_string(),
            boot_timeout_secs: 90,
            enabled: false,
            child_process: Arc::new(Mutex::new(None)),
        }
    }

    pub fn with_timeout(avd_name: &str, boot_timeout_secs: u64) -> Self {
        Self {
            avd_name: avd_name.to_string(),
            boot_timeout_secs,
            enabled: true,
            child_process: Arc::new(Mutex::new(None)),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Automatically locates the Android SDK root directory from environment or standard paths.
    pub fn find_sdk_root() -> Option<PathBuf> {
        if let Ok(path) = std::env::var("ANDROID_HOME") {
            let p = PathBuf::from(path);
            if p.exists() {
                return Some(p);
            }
        }
        if let Ok(path) = std::env::var("ANDROID_SDK_ROOT") {
            let p = PathBuf::from(path);
            if p.exists() {
                return Some(p);
            }
        }
        if cfg!(windows) {
            if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
                let p = PathBuf::from(local_app_data).join("Android").join("Sdk");
                if p.exists() {
                    return Some(p);
                }
            }
        } else {
            if let Ok(home) = std::env::var("HOME") {
                let p = PathBuf::from(home).join("Android").join("Sdk");
                if p.exists() {
                    return Some(p);
                }
            }
        }
        None
    }

    /// Automatically discovers the Android emulator executable.
    pub fn find_emulator_binary(sdk_root: Option<&Path>) -> Option<PathBuf> {
        let binary_name = if cfg!(windows) {
            "emulator.exe"
        } else {
            "emulator"
        };

        if let Some(sdk) = sdk_root {
            let emu = sdk.join("emulator").join(binary_name);
            if emu.exists() {
                return Some(emu);
            }
        }

        if let Some(sdk) = Self::find_sdk_root() {
            let emu = sdk.join("emulator").join(binary_name);
            if emu.exists() {
                return Some(emu);
            }
        }

        // Try system PATH
        if let Ok(path_var) = std::env::var("PATH") {
            let sep = if cfg!(windows) { ';' } else { ':' };
            for part in path_var.split(sep) {
                let candidate = PathBuf::from(part).join(binary_name);
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }

        None
    }

    /// Checks if a guest is currently alive and responsive.
    pub fn is_alive(&self, transport: &dyn GuestTransport) -> bool {
        transport.is_alive()
    }

    /// Ensures that an Android guest environment is running.
    /// If no guest is online, it provisions the AVD, boots the hypervisor headlessly,
    /// and waits for the guest OS to complete initialization.
    pub fn ensure_running(
        &self,
        transport: &dyn GuestTransport,
        sdk_root: Option<&Path>,
    ) -> UmrResult<()> {
        if transport.is_alive() || !self.enabled {
            return Ok(());
        }

        let resolved_sdk = sdk_root
            .map(|p| p.to_path_buf())
            .or_else(Self::find_sdk_root)
            .ok_or_else(|| {
                DiagnosticError::new(
                    ErrorCode::ErrBackendUnavailable,
                    "Android SDK root directory could not be located on the host",
                )
                .with_suggestion("Ensure Android SDK is installed or set ANDROID_HOME environment variable.")
            })?;

        let emu_bin = Self::find_emulator_binary(Some(&resolved_sdk)).ok_or_else(|| {
            DiagnosticError::new(
                ErrorCode::ErrBackendUnavailable,
                "Android emulator executable could not be located",
            )
            .with_suggestion("Install the Android Emulator via Android Studio SDK Manager or CLI.")
        })?;

        // 1. Check if AVD is provisioned. If not, auto-provision from installed system-images.
        let avd_root = AvdProvisioner::get_avd_root()?;
        let avd_ini = avd_root.join(format!("{}.ini", self.avd_name));
        if !avd_ini.exists() {
            let sys_images = AvdProvisioner::discover_system_images(&resolved_sdk);
            if sys_images.is_empty() {
                return Err(DiagnosticError::new(
                    ErrorCode::ErrBackendUnavailable,
                    "No Android system images found to provision autonomous guest",
                )
                .with_suggestion(
                    "Install an x86_64 system image in the Android SDK (e.g., android-34 or android-36).",
                ));
            }
            AvdProvisioner::provision_avd_if_missing(&self.avd_name, &sys_images[0], &resolved_sdk)?;
        }

        // 2. Spawn headless emulator process
        let mut cmd = Command::new(&emu_bin);
        cmd.arg("-avd")
            .arg(&self.avd_name)
            .arg("-no-window")
            .arg("-no-audio")
            .arg("-no-boot-anim")
            .arg("-accel")
            .arg("on")
            .arg("-gpu")
            .arg("host");

        #[cfg(windows)]
        {
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        let child = cmd.spawn().map_err(|e| {
            DiagnosticError::new(
                ErrorCode::ErrBackendSessionFailed,
                format!("Failed to spawn Android emulator process: {}", e),
            )
        })?;

        if let Ok(mut lock) = self.child_process.lock() {
            *lock = Some(child);
        }

        // 3. Poll ADB until boot is completed
        let start = Instant::now();
        let timeout = Duration::from_secs(self.boot_timeout_secs);

        while start.elapsed() < timeout {
            // Check if guest transport detected the device and system completed boot
            if transport.is_alive() {
                if let Ok(output) = transport.execute_command("getprop sys.boot_completed") {
                    if output.stdout.trim() == "1" {
                        return Ok(());
                    }
                }
            }

            thread::sleep(Duration::from_millis(1500));
        }

        // If we timed out, check if child exited unexpectedly
        if let Ok(mut lock) = self.child_process.lock() {
            if let Some(child) = lock.as_mut() {
                if let Ok(Some(status)) = child.try_wait() {
                    return Err(DiagnosticError::new(
                        ErrorCode::ErrBackendSessionFailed,
                        format!("Android emulator process terminated unexpectedly with code: {}", status),
                    ));
                }
            }
        }

        Err(DiagnosticError::new(
            ErrorCode::ErrBackendTimeout,
            format!(
                "Timed out waiting for autonomous guest '{}' to finish booting ({}s limit)",
                self.avd_name, self.boot_timeout_secs
            ),
        )
        .with_suggestion("Verify host hardware acceleration (WHPX/KVM) settings and system image integrity."))
    }

    /// Gracefully terminates the supervised guest process if active.
    pub fn shutdown(&self) -> UmrResult<()> {
        if let Ok(mut lock) = self.child_process.lock() {
            if let Some(mut child) = lock.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
        Ok(())
    }
}

impl Drop for GuestSupervisor {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}
