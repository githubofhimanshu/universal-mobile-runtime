use crate::DistributionError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Mandatory privacy consent record for remote streaming / Track C execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteConsentRecord {
    pub user_id: String,
    pub host_endpoint: String,
    pub consented_at: DateTime<Utc>,
    pub terms_version: String,
    pub allow_input_streaming: bool,
    pub allow_frame_streaming: bool,
}

/// Remote execution privacy gate managing explicit user opt-ins and UI disclosure terms.
#[derive(Debug, Clone, Default)]
pub struct RemotePrivacyGate {
    consents: HashMap<String, RemoteConsentRecord>,
}

impl RemotePrivacyGate {
    pub const CURRENT_TERMS_VERSION: &'static str = "1.0.0-2026";

    /// Creates a new remote privacy gate.
    pub fn new() -> Self {
        Self {
            consents: HashMap::new(),
        }
    }

    /// Records explicit user consent for remote device execution.
    pub fn record_consent(
        &mut self,
        user_id: &str,
        host_endpoint: &str,
        allow_input_streaming: bool,
        allow_frame_streaming: bool,
    ) -> RemoteConsentRecord {
        let record = RemoteConsentRecord {
            user_id: user_id.to_string(),
            host_endpoint: host_endpoint.to_string(),
            consented_at: Utc::now(),
            terms_version: Self::CURRENT_TERMS_VERSION.to_string(),
            allow_input_streaming,
            allow_frame_streaming,
        };

        self.consents.insert(host_endpoint.to_string(), record.clone());
        record
    }

    /// Revokes user consent for a host endpoint.
    pub fn revoke_consent(&mut self, host_endpoint: &str) -> Option<RemoteConsentRecord> {
        self.consents.remove(host_endpoint)
    }

    /// Checks if a valid consent record exists for the endpoint.
    pub fn has_consent(&self, host_endpoint: &str) -> bool {
        self.consents.get(host_endpoint).map_or(false, |r| {
            r.allow_input_streaming && r.allow_frame_streaming
        })
    }

    /// Verification check: rejects unconsented remote streaming sessions.
    pub fn check_remote_execution_permitted(&self, host_endpoint: &str) -> Result<(), DistributionError> {
        if !self.has_consent(host_endpoint) {
            return Err(DistributionError::RemoteConsentRequired {
                host_endpoint: host_endpoint.to_string(),
            });
        }
        Ok(())
    }

    /// Returns the mandatory UI watermark / badge label for execution sessions.
    pub fn get_watermark_label(&self, is_remote: bool) -> Option<String> {
        if is_remote {
            Some("RemoteStream (Device Track C)".to_string())
        } else {
            None
        }
    }

    /// Canonical privacy and data residency disclosure text for remote streaming.
    pub fn get_privacy_disclosure_text() -> &'static str {
        "REMOTE EXECUTION PRIVACY DISCLOSURE:\n\
         Connecting to a remote device or runner streams screen frames and forwards touch/keyboard input.\n\
         1. Screen pixels are transmitted over the configured network bridge.\n\
         2. Touch coordinates and key events are forwarded to the remote device.\n\
         3. Application state and user data remain on the remote device unless exported.\n\
         4. Keystroke passwords and clipboard contents are not logged by UMR.\n\
         Explicit consent is required to start this session."
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remote_privacy_consent_gate() {
        let mut gate = RemotePrivacyGate::new();
        let endpoint = "https://runner-mac-01.internal:8443";

        // Without consent, remote execution must be blocked
        assert!(!gate.has_consent(endpoint));
        let err = gate.check_remote_execution_permitted(endpoint).unwrap_err();
        assert!(matches!(err, DistributionError::RemoteConsentRequired { .. }));

        // Provide partial consent (frame only)
        gate.record_consent("user1", endpoint, false, true);
        assert!(!gate.has_consent(endpoint));

        // Provide full consent
        gate.record_consent("user1", endpoint, true, true);
        assert!(gate.has_consent(endpoint));
        assert!(gate.check_remote_execution_permitted(endpoint).is_ok());

        // Revoke consent
        gate.revoke_consent(endpoint);
        assert!(!gate.has_consent(endpoint));
        assert!(gate.check_remote_execution_permitted(endpoint).is_err());
    }

    #[test]
    fn test_watermark_labels() {
        let gate = RemotePrivacyGate::new();
        assert_eq!(gate.get_watermark_label(false), None);
        assert_eq!(
            gate.get_watermark_label(true),
            Some("RemoteStream (Device Track C)".to_string())
        );
    }
}

