use std::sync::{Arc, Mutex};
use super::{CommandOutput, GuestTransport};
use umr_core::{DiagnosticError, ErrorCode, FrameBuffer, InputEvent, UmrResult};

#[derive(Debug, Clone, Default)]
struct MockState {
    executed_commands: Vec<String>,
    received_inputs: Vec<InputEvent>,
    simulated_logs: Vec<String>,
    is_alive: bool,
    should_crash: bool,
    should_timeout: bool,
}

/// In-memory deterministic mock transport for automated verification and test pipelines.
#[derive(Clone)]
pub struct MockGuestTransport {
    state: Arc<Mutex<MockState>>,
    display_width: u32,
    display_height: u32,
}

impl MockGuestTransport {
    pub fn new() -> Self {
        let mut logs = Vec::new();
        logs.push("init: Entering Android init stage 2".to_string());
        logs.push("Zygote: Preloading classes and system resources".to_string());
        logs.push("ActivityManager: System server ready".to_string());

        Self {
            state: Arc::new(Mutex::new(MockState {
                executed_commands: Vec::new(),
                received_inputs: Vec::new(),
                simulated_logs: logs,
                is_alive: true,
                should_crash: false,
                should_timeout: false,
            })),
            display_width: 1080,
            display_height: 1920,
        }
    }

    pub fn set_alive(&self, alive: bool) {
        if let Ok(mut s) = self.state.lock() {
            s.is_alive = alive;
        }
    }

    pub fn inject_crash(&self) {
        if let Ok(mut s) = self.state.lock() {
            s.should_crash = true;
            s.is_alive = false;
            s.simulated_logs.push("FATAL EXCEPTION in main thread: SIGSEGV (signal 11)".to_string());
            s.simulated_logs.push("DEBUG: pid: 1421, tid: 1421, name: main >>> org.umr.app <<<".to_string());
            s.simulated_logs.push("backtrace: #00 pc 0000000000049280 /apex/com.android.runtime/lib64/bionic/libc.so".to_string());
        }
    }

    pub fn inject_timeout(&self) {
        if let Ok(mut s) = self.state.lock() {
            s.should_timeout = true;
            s.simulated_logs.push("ActivityManager: ANR in org.umr.app (com.umr/.MainActivity)".to_string());
            s.simulated_logs.push("PID: 1421, Reason: Input dispatching timed out (Waiting to process key event)".to_string());
        }
    }

    pub fn executed_commands(&self) -> Vec<String> {
        self.state.lock().map(|s| s.executed_commands.clone()).unwrap_or_default()
    }

    pub fn received_inputs(&self) -> Vec<InputEvent> {
        self.state.lock().map(|s| s.received_inputs.clone()).unwrap_or_default()
    }
}

impl Default for MockGuestTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl GuestTransport for MockGuestTransport {
    fn name(&self) -> &str {
        "mock-guest-transport"
    }

    fn execute_command(&self, cmd: &str) -> UmrResult<CommandOutput> {
        let mut s = self.state.lock().map_err(|_| {
            DiagnosticError::new(ErrorCode::ErrBackendInternal, "Mock state lock poisoned")
        })?;

        if s.should_crash {
            return Err(DiagnosticError::new(
                ErrorCode::ErrBackendProcessCrashed,
                "Guest process crashed during command execution",
            ));
        }

        if s.should_timeout {
            return Err(DiagnosticError::new(
                ErrorCode::ErrBackendTimeout,
                "Command execution timed out (ANR detected)",
            ));
        }

        s.executed_commands.push(cmd.to_string());
        Ok(CommandOutput::success(format!("mock-out: {}", cmd)))
    }

    fn send_input(&self, event: &InputEvent) -> UmrResult<()> {
        let mut s = self.state.lock().map_err(|_| {
            DiagnosticError::new(ErrorCode::ErrBackendInternal, "Mock state lock poisoned")
        })?;

        if s.should_crash {
            return Err(DiagnosticError::new(
                ErrorCode::ErrBackendProcessCrashed,
                "Failed to send input: guest crashed",
            ));
        }

        s.received_inputs.push(event.clone());
        Ok(())
    }

    fn capture_frame(&self) -> UmrResult<Option<FrameBuffer>> {
        let s = self.state.lock().map_err(|_| {
            DiagnosticError::new(ErrorCode::ErrBackendInternal, "Mock state lock poisoned")
        })?;

        if !s.is_alive || s.should_crash {
            return Ok(None);
        }

        // Return valid RGBA frame
        let mut fb = FrameBuffer::new(self.display_width, self.display_height, umr_core::PixelFormat::Rgba8888);
        // Paint top bar green (mock active app)
        let row_stride = (self.display_width * 4) as usize;
        for y in 0..60 {
            for x in 0..self.display_width as usize {
                let idx = y * row_stride + x * 4;
                fb.data[idx] = 40;     // R
                fb.data[idx + 1] = 160; // G
                fb.data[idx + 2] = 220; // B
                fb.data[idx + 3] = 255; // A
            }
        }
        Ok(Some(fb))
    }

    fn stream_logs(&self) -> UmrResult<Vec<String>> {
        let s = self.state.lock().map_err(|_| {
            DiagnosticError::new(ErrorCode::ErrBackendInternal, "Mock state lock poisoned")
        })?;
        Ok(s.simulated_logs.clone())
    }

    fn is_alive(&self) -> bool {
        self.state.lock().map(|s| s.is_alive).unwrap_or(false)
    }
}
