use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{DiagnosticError, UmrResult};

/// Canonical application lifecycle states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleState {
    /// Backend or runtime environment is unavailable or lacks prerequisites.
    Unavailable,
    /// Backend is initialized and ready to prepare and execute applications.
    Ready,
    /// Staging sandbox, resolving dependencies, and configuring session.
    Preparing,
    /// Application is actively executing.
    Running,
    /// Teardown initiated; signals sent; awaiting process exit.
    Stopping,
    /// Normal, graceful shutdown completed; resources released.
    Terminated,
    /// Execution or preparation failed; diagnostics captured.
    Failed,
}

impl std::fmt::Display for LifecycleState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable => write!(f, "unavailable"),
            Self::Ready => write!(f, "ready"),
            Self::Preparing => write!(f, "preparing"),
            Self::Running => write!(f, "running"),
            Self::Stopping => write!(f, "stopping"),
            Self::Terminated => write!(f, "terminated"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

impl LifecycleState {
    /// Validates whether a state transition from `self` to `target` is legally permitted.
    pub fn can_transition_to(&self, target: LifecycleState) -> bool {
        match (self, target) {
            // Unavailable can become Ready once prerequisites are fulfilled, or Failed
            (Self::Unavailable, Self::Ready) => true,
            (Self::Unavailable, Self::Failed) => true,

            // Ready can start Preparing a session
            (Self::Ready, Self::Preparing) => true,
            (Self::Ready, Self::Unavailable) => true,
            (Self::Ready, Self::Failed) => true,

            // Preparing can succeed into Running, or fail, or be aborted
            (Self::Preparing, Self::Running) => true,
            (Self::Preparing, Self::Stopping) => true,
            (Self::Preparing, Self::Failed) => true,

            // Running can transition to Stopping on request or exit, or Failed on crash
            (Self::Running, Self::Stopping) => true,
            (Self::Running, Self::Failed) => true,

            // Stopping transitions to Terminated on clean exit, or Failed on timeout/kill
            (Self::Stopping, Self::Terminated) => true,
            (Self::Stopping, Self::Failed) => true,

            // Terminated can reset back to Ready for a new session
            (Self::Terminated, Self::Ready) => true,

            // Failed can reset back to Ready after diagnostic inspection
            (Self::Failed, Self::Ready) => true,

            // Self-transitions are no-ops
            (a, b) if *a == b => true,

            // All other transitions are illegal
            _ => false,
        }
    }
}

/// Unique session identifier for a running application instance.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub String);

impl SessionId {
    pub fn new(package_id: &str) -> Self {
        let ts = Utc::now().timestamp_millis();
        let rand_suffix = (ts % 10000) as u16;
        Self(format!("{}-{}-{}", package_id, ts, rand_suffix))
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// An active or completed application execution session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionRecord {
    pub session_id: SessionId,
    pub package_id: String,
    pub state: LifecycleState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub exit_code: Option<i32>,
    pub last_error: Option<DiagnosticError>,
}

impl SessionRecord {
    pub fn new(package_id: impl Into<String>) -> Self {
        let id_str = package_id.into();
        let now = Utc::now();
        Self {
            session_id: SessionId::new(&id_str),
            package_id: id_str,
            state: LifecycleState::Ready,
            created_at: now,
            updated_at: now,
            exit_code: None,
            last_error: None,
        }
    }

    /// Transitions the session state, enforcing valid lifecycle rules.
    pub fn transition_to(&mut self, next: LifecycleState) -> UmrResult<()> {
        if !self.state.can_transition_to(next) {
            return Err(DiagnosticError::invalid_transition(self.state, next));
        }
        self.state = next;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Marks the session as failed with structured diagnostics.
    pub fn mark_failed(&mut self, err: DiagnosticError) {
        let _ = self.transition_to(LifecycleState::Failed);
        self.last_error = Some(err);
        self.updated_at = Utc::now();
    }

    /// Marks the session as terminated with an exit code.
    pub fn mark_terminated(&mut self, code: i32) -> UmrResult<()> {
        self.transition_to(LifecycleState::Terminated)?;
        self.exit_code = Some(code);
        self.updated_at = Utc::now();
        Ok(())
    }
}

