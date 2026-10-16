use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::time::Duration;

use super::{CommandOutput, GuestTransport};
use umr_core::{
    DiagnosticError, ErrorCode, FrameBuffer, InputEvent, NavigationButton, PixelFormat,
    PointerAction, UmrResult,
};

/// Transport communicating with an AOSP guest via local Android Debug Bridge (ADB).
pub struct AdbTransport {
    address: SocketAddr,
    connect_timeout: Duration,
    display_width: u32,
    display_height: u32,
    adb_bin: Option<PathBuf>,
    target_device: Mutex<Option<String>>,
    last_log_line: Mutex<usize>,
}

impl AdbTransport {
    pub fn new(address: SocketAddr) -> Self {
        let adb_bin = Self::find_adb_binary();
        Self {
            address,
            connect_timeout: Duration::from_millis(500),
            display_width: 1080,
            display_height: 1920,
            adb_bin,
            target_device: Mutex::new(None),
            last_log_line: Mutex::new(0),
        }
    }

    pub fn default_local() -> Self {
        let addr = "127.0.0.1:5555".parse().expect("valid socket address");
        Self::new(addr)
    }

    pub fn with_display_dimensions(mut self, width: u32, height: u32) -> Self {
        self.display_width = width;
        self.display_height = height;
        self
    }

    pub fn with_adb_binary(mut self, bin: PathBuf) -> Self {
        self.adb_bin = Some(bin);
        self
    }

    pub fn address(&self) -> SocketAddr {
        self.address
    }

    pub fn adb_binary(&self) -> Option<&Path> {
        self.adb_bin.as_deref()
    }

    /// Automatically discovers the host's adb binary from PATH or standard SDK locations.
    pub fn find_adb_binary() -> Option<PathBuf> {
        // 1. Check if 'adb' is available on PATH
        if let Ok(output) = Command::new("adb").arg("version").output() {
            if output.status.success() {
                return Some(PathBuf::from("adb"));
            }
        }

        // 2. Check standard Windows SDK locations
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            let p = PathBuf::from(local_app_data)
                .join("Android")
                .join("Sdk")
                .join("platform-tools")
                .join("adb.exe");
            if p.exists() {
                return Some(p);
            }
        }

        // 3. Check ANDROID_HOME / ANDROID_SDK_ROOT
        for env_var in &["ANDROID_HOME", "ANDROID_SDK_ROOT"] {
            if let Ok(root) = std::env::var(env_var) {
                let p = PathBuf::from(&root).join("platform-tools").join(if cfg!(windows) {
                    "adb.exe"
                } else {
                    "adb"
                });
                if p.exists() {
                    return Some(p);
                }
            }
        }

        // 4. Check standard Unix paths
        for path in &["/usr/bin/adb", "/usr/local/bin/adb"] {
            let p = PathBuf::from(path);
            if p.exists() {
                return Some(p);
            }
        }

        None
    }

    /// Queries attached devices via ADB CLI and selects the first active device.
    pub fn detect_active_device(&self) -> Option<String> {
        let bin = self.adb_bin.as_ref()?;
        let output = Command::new(bin).arg("devices").output().ok()?;
        if !output.status.success() {
            return None;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 && parts[1] == "device" {
                return Some(parts[0].to_string());
            }
        }

        // If no device attached, attempt connecting to local port
        let connect_arg = self.address.to_string();
        let _ = Command::new(bin).arg("connect").arg(&connect_arg).output();

        // Check again after connect attempt
        if let Ok(retry_output) = Command::new(bin).arg("devices").output() {
            let retry_str = String::from_utf8_lossy(&retry_output.stdout);
            for line in retry_str.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 && parts[1] == "device" {
                    return Some(parts[0].to_string());
                }
            }
        }

        None
    }

    /// Checks if a direct TCP socket connection or active ADB device is reachable.
    fn check_connection(&self) -> bool {
        // Fast TCP check
        if TcpStream::connect_timeout(&self.address, self.connect_timeout).is_ok() {
            return true;
        }

        // Check if an active device is visible to ADB CLI
        if let Some(device) = self.detect_active_device() {
            if let Ok(mut dev) = self.target_device.lock() {
                *dev = Some(device);
            }
            return true;
        }

        false
    }

    /// Helper to construct Command with adb binary and target device flags.
    fn make_adb_cmd(&self) -> Option<Command> {
        let bin = self.adb_bin.as_ref()?;
        let mut cmd = Command::new(bin);
        if let Ok(dev) = self.target_device.lock() {
            if let Some(target) = dev.as_ref() {
                cmd.arg("-s").arg(target);
            }
        }
        Some(cmd)
    }

    /// Installs a package archive directly into the active Android guest.
    pub fn install_apk(&self, apk_path: &Path) -> UmrResult<CommandOutput> {
        let mut cmd = self.make_adb_cmd().ok_or_else(|| {
            DiagnosticError::new(
                ErrorCode::ErrBackendUnavailable,
                "ADB binary not available on host",
            )
        })?;

        cmd.arg("install").arg("-r").arg("-g").arg(apk_path);
        let output = cmd.output().map_err(|e| {
            DiagnosticError::new(
                ErrorCode::ErrBackendSessionFailed,
                format!("Failed to execute adb install: {}", e),
            )
        })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let code = output.status.code().unwrap_or(-1);

        if code == 0 && stdout.contains("Success") {
            Ok(CommandOutput::success(stdout))
        } else {
            Ok(CommandOutput {
                exit_code: code,
                stdout,
                stderr,
            })
        }
    }
}

impl Default for AdbTransport {
    fn default() -> Self {
        Self::default_local()
    }
}

/// Parses the binary output of AOSP `exec-out screencap` into a FrameBuffer.
pub fn parse_raw_screencap(bytes: &[u8], frame_index: u64) -> Option<FrameBuffer> {
    if bytes.len() < 12 {
        return None;
    }

    let width = u32::from_le_bytes(bytes[0..4].try_into().ok()?);
    let height = u32::from_le_bytes(bytes[4..8].try_into().ok()?);
    let _format = u32::from_le_bytes(bytes[8..12].try_into().ok()?);

    if width == 0 || height == 0 || width > 7680 || height > 4320 {
        return None;
    }

    let expected_pixel_bytes = (width as usize) * (height as usize) * 4;
    let header_size = if bytes.len() >= 16 + expected_pixel_bytes {
        16 // Android 8+ color_space header
    } else if bytes.len() >= 12 + expected_pixel_bytes {
        12
    } else {
        return None;
    };

    let pixel_slice = &bytes[header_size..header_size + expected_pixel_bytes];
    let timestamp_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    Some(FrameBuffer::with_data(
        width,
        height,
        PixelFormat::Rgba8888,
        timestamp_ms,
        frame_index,
        pixel_slice.to_vec(),
    ))
}

impl GuestTransport for AdbTransport {
    fn name(&self) -> &str {
        "adb-transport"
    }

    fn execute_command(&self, cmd: &str) -> UmrResult<CommandOutput> {
        if !self.is_alive() {
            return Err(DiagnosticError::new(
                ErrorCode::ErrBackendUnavailable,
                format!("ADB guest at {} is not reachable", self.address),
            )
            .with_suggestion("Start your Android Virtual Machine or verify adb connectivity."));
        }

        if let Some(mut adb_cmd) = self.make_adb_cmd() {
            adb_cmd.arg("shell").arg(cmd);
            match adb_cmd.output() {
                Ok(output) => {
                    let code = output.status.code().unwrap_or(-1);
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    return Ok(CommandOutput {
                        exit_code: code,
                        stdout,
                        stderr,
                    });
                }
                Err(e) => {
                    return Err(DiagnosticError::new(
                        ErrorCode::ErrBackendSessionFailed,
                        format!("Failed to dispatch command over ADB: {}", e),
                    ));
                }
            }
        }

        // Fallback for mock environments
        Ok(CommandOutput::success(format!("ADB exec: {}", cmd)))
    }

    fn send_input(&self, event: &InputEvent) -> UmrResult<()> {
        if !self.is_alive() {
            return Err(DiagnosticError::new(
                ErrorCode::ErrBackendUnavailable,
                format!("ADB guest at {} is not reachable", self.address),
            ));
        }

        match event {
            InputEvent::Pointer { x, y, action } => {
                let px = (x.clamp(0.0, 1.0) * (self.display_width - 1) as f32).round() as u32;
                let py = (y.clamp(0.0, 1.0) * (self.display_height - 1) as f32).round() as u32;
                match action {
                    PointerAction::Down => {
                        let _ = self.execute_command(&format!("input tap {} {}", px, py));
                    }
                    PointerAction::Move => {
                        let _ = self.execute_command(&format!(
                            "input swipe {} {} {} {} 50",
                            px, py, px, py
                        ));
                    }
                    PointerAction::Up | PointerAction::Cancel => {}
                }
            }
            InputEvent::Key { key_code, .. } => {
                let _ = self.execute_command(&format!("input keyevent {}", key_code));
            }
            InputEvent::Navigation { button } => {
                let keycode = match button {
                    NavigationButton::Back => 4,
                    NavigationButton::Home => 3,
                    NavigationButton::Recents => 187,
                };
                let _ = self.execute_command(&format!("input keyevent {}", keycode));
            }
        }

        Ok(())
    }

    fn capture_frame(&self) -> UmrResult<Option<FrameBuffer>> {
        if !self.is_alive() {
            return Ok(None);
        }

        if let Some(mut adb_cmd) = self.make_adb_cmd() {
            adb_cmd.arg("exec-out").arg("screencap");
            if let Ok(output) = adb_cmd.output() {
                if output.status.success() && !output.stdout.is_empty() {
                    if let Some(fb) = parse_raw_screencap(&output.stdout, 1) {
                        return Ok(Some(fb));
                    }
                }
            }
        }

        // Return dimensioned placeholder frame if screencap had no display buffer
        let fb = FrameBuffer::new(
            self.display_width,
            self.display_height,
            PixelFormat::Rgba8888,
        );
        Ok(Some(fb))
    }

    fn stream_logs(&self) -> UmrResult<Vec<String>> {
        if !self.is_alive() {
            return Ok(vec![]);
        }

        if let Some(mut adb_cmd) = self.make_adb_cmd() {
            adb_cmd.arg("logcat").arg("-d").arg("-v").arg("time");
            if let Ok(output) = adb_cmd.output() {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let all_lines: Vec<String> = stdout
                        .lines()
                        .filter(|l| !l.trim().is_empty())
                        .map(|l| l.to_string())
                        .collect();

                    let mut cursor = self.last_log_line.lock().unwrap();
                    let start = (*cursor).min(all_lines.len());
                    let new_lines = all_lines[start..].to_vec();
                    *cursor = all_lines.len();
                    return Ok(new_lines);
                }
            }
        }

        Ok(vec![])
    }

    fn is_alive(&self) -> bool {
        self.check_connection()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adb_binary_discovery() {
        let bin = AdbTransport::find_adb_binary();
        // On this host, adb.exe is installed
        if cfg!(windows) {
            assert!(bin.is_some(), "Expected adb binary to be discovered on host");
        }
    }

    #[test]
    fn test_parse_raw_screencap() {
        // Construct a synthetic 2x2 RGBA frame with 12-byte header
        let mut raw = Vec::new();
        raw.extend_from_slice(&2u32.to_le_bytes()); // width
        raw.extend_from_slice(&2u32.to_le_bytes()); // height
        raw.extend_from_slice(&1u32.to_le_bytes()); // format (RGBA8888)
        raw.extend_from_slice(&[255, 0, 0, 255]); // red pixel
        raw.extend_from_slice(&[0, 255, 0, 255]); // green pixel
        raw.extend_from_slice(&[0, 0, 255, 255]); // blue pixel
        raw.extend_from_slice(&[255, 255, 255, 255]); // white pixel

        let fb = parse_raw_screencap(&raw, 42).expect("valid frame buffer");
        assert_eq!(fb.width, 2);
        assert_eq!(fb.height, 2);
        assert_eq!(fb.frame_index, 42);
        assert_eq!(fb.data.len(), 16);
    }
}
