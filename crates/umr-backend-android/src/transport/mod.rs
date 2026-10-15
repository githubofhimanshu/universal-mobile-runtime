pub mod adb;
pub mod mock;
pub mod vsock;

pub use adb::AdbTransport;
pub use mock::MockGuestTransport;
pub use vsock::VsockTransport;

use serde::{Deserialize, Serialize};
use umr_core::{FrameBuffer, InputEvent, UmrResult};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandOutput {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl CommandOutput {
    pub fn success(stdout: impl Into<String>) -> Self {
        Self {
            exit_code: 0,
            stdout: stdout.into(),
            stderr: String::new(),
        }
    }

    pub fn failure(code: i32, stderr: impl Into<String>) -> Self {
        Self {
            exit_code: code,
            stdout: String::new(),
            stderr: stderr.into(),
        }
    }

    pub fn is_success(&self) -> bool {
        self.exit_code == 0
    }
}

/// Abstract communication transport between host runtime and guest environment.
pub trait GuestTransport: Send + Sync {
    /// Friendly transport name (e.g. "vsock", "adb-socket", "mock-transport").
    fn name(&self) -> &str;

    /// Dispatches a shell command to the guest environment.
    fn execute_command(&self, cmd: &str) -> UmrResult<CommandOutput>;

    /// Dispatches an input event (pointer touch, keycode, navigation) to the guest.
    fn send_input(&self, event: &InputEvent) -> UmrResult<()>;

    /// Captures the latest framebuffer render surface from the guest.
    fn capture_frame(&self) -> UmrResult<Option<FrameBuffer>>;

    /// Retrieves accumulated or streamed logcat lines from the guest.
    fn stream_logs(&self) -> UmrResult<Vec<String>>;

    /// Checks whether the transport connection to the guest is alive and responsive.
    fn is_alive(&self) -> bool;
}

