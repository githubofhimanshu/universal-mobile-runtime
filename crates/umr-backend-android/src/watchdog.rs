use std::time::{Duration, Instant};
use umr_core::{DiagnosticError, ErrorCode, SessionRecord, UmrResult};

/// Crash event extracted from guest execution logs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestCrashInfo {
    pub signal: Option<i32>,
    pub reason: String,
    pub stack_trace: Vec<String>,
}

/// Watchdog monitoring guest health, detecting ANRs (Application Not Responding), and isolating crashes.
pub struct WatchdogSupervisor {
    anr_timeout: Duration,
    last_heartbeat: Instant,
    active_pid: Option<u32>,
}

impl WatchdogSupervisor {
    pub fn new(anr_timeout: Duration) -> Self {
        Self {
            anr_timeout,
            last_heartbeat: Instant::now(),
            active_pid: None,
        }
    }

    pub fn default_monitor() -> Self {
        Self::new(Duration::from_secs(5))
    }

    pub fn set_active_pid(&mut self, pid: u32) {
        self.active_pid = Some(pid);
        self.last_heartbeat = Instant::now();
    }

    pub fn record_heartbeat(&mut self) {
        self.last_heartbeat = Instant::now();
    }

    /// Evaluates if an ANR has occurred due to lack of responsiveness.
    pub fn check_anr(&self) -> Option<DiagnosticError> {
        if self.last_heartbeat.elapsed() > self.anr_timeout {
            Some(DiagnosticError::new(
                ErrorCode::ErrBackendTimeout,
                format!(
                    "ANR detected: Guest application unresponsive for {:.1}s (timeout was {:.1}s)",
                    self.last_heartbeat.elapsed().as_secs_f32(),
                    self.anr_timeout.as_secs_f32()
                ),
            ))
        } else {
            None
        }
    }

    /// Scans logcat/syslog output for crash signatures and stack traces.
    pub fn inspect_crash_logs(&self, logs: &[String]) -> Option<GuestCrashInfo> {
        let mut in_crash = false;
        let mut reason = String::new();
        let mut trace = Vec::new();
        let mut signal = None;

        for line in logs {
            if line.contains("FATAL EXCEPTION") || line.contains("SIGSEGV") || line.contains("SIGABRT") {
                in_crash = true;
                reason = line.clone();
                if line.contains("signal 11") {
                    signal = Some(11);
                } else if line.contains("signal 6") {
                    signal = Some(6);
                }
            } else if in_crash {
                if line.contains("backtrace:") || line.contains("at ") || line.contains("#0") {
                    trace.push(line.clone());
                } else if trace.len() > 10 {
                    break;
                }
            }
        }

        if in_crash {
            Some(GuestCrashInfo {
                signal,
                reason,
                stack_trace: trace,
            })
        } else {
            None
        }
    }

    /// Isolates a guest crash: transitions session to Failed state with structured diagnostics.
    pub fn handle_crash(
        &self,
        session: &mut SessionRecord,
        crash_info: GuestCrashInfo,
    ) -> UmrResult<()> {
        let mut message = format!("Guest process crashed: {}", crash_info.reason);
        if !crash_info.stack_trace.is_empty() {
            message.push_str("\nStack trace:\n");
            for line in crash_info.stack_trace.iter().take(5) {
                message.push_str("  ");
                message.push_str(line);
                message.push('\n');
            }
        }

        let err = DiagnosticError::new(ErrorCode::ErrBackendProcessCrashed, message);
        session.mark_failed(err);
        Ok(())
    }
}
