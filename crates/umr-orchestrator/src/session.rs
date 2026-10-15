use std::collections::HashMap;

use umr_core::{
    DiagnosticError, ErrorCode, ExecutionMode, PackageMetadata, SessionId,
    SessionRecord, UmrResult,
};

use crate::logger::{DiagnosticReport, LogLevel};

/// An actively managed execution session tracked by the orchestrator.
pub struct ActiveSession {
    pub record: SessionRecord,
    pub metadata: PackageMetadata,
    pub backend_id: String,
    pub execution_mode: ExecutionMode,
    pub report: DiagnosticReport,
}

impl ActiveSession {
    pub fn new(
        metadata: PackageMetadata,
        backend_id: String,
        execution_mode: ExecutionMode,
    ) -> Self {
        let pkg_id = metadata.package_id().to_string();
        let record = SessionRecord::new(&pkg_id);
        let report = DiagnosticReport::new(record.session_id.to_string(), &pkg_id);

        Self {
            record,
            metadata,
            backend_id,
            execution_mode,
            report,
        }
    }

    pub fn log(&mut self, level: LogLevel, source: impl Into<String>, msg: impl AsRef<str>) {
        self.report.log(level, source, msg);
    }
}

/// In-memory manager for active and historical runtime sessions.
pub struct SessionManager {
    sessions: HashMap<SessionId, ActiveSession>,
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    pub fn create_session(
        &mut self,
        metadata: PackageMetadata,
        backend_id: String,
        execution_mode: ExecutionMode,
    ) -> SessionId {
        let session = ActiveSession::new(metadata, backend_id, execution_mode);
        let id = session.record.session_id.clone();
        self.sessions.insert(id.clone(), session);
        id
    }

    pub fn get(&self, id: &SessionId) -> UmrResult<&ActiveSession> {
        self.sessions.get(id).ok_or_else(|| {
            DiagnosticError::new(
                ErrorCode::ErrLifecycleSessionNotFound,
                format!("Session '{}' not found", id),
            )
        })
    }

    pub fn get_mut(&mut self, id: &SessionId) -> UmrResult<&mut ActiveSession> {
        self.sessions.get_mut(id).ok_or_else(|| {
            DiagnosticError::new(
                ErrorCode::ErrLifecycleSessionNotFound,
                format!("Session '{}' not found", id),
            )
        })
    }

    pub fn list_sessions(&self) -> Vec<&SessionRecord> {
        self.sessions.values().map(|s| &s.record).collect()
    }

    pub fn remove_session(&mut self, id: &SessionId) -> Option<ActiveSession> {
        self.sessions.remove(id)
    }
}
