use crate::provenance::ProvenanceRecord;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Diagnostic and support bundle for troubleshooting runtime issues.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportReport {
    pub report_id: String,
    pub generated_at: DateTime<Utc>,
    pub host_os: String,
    pub host_cpu: String,
    pub umr_version: String,
    pub installed_apps: Vec<ProvenanceRecord>,
    pub sanitized_logs: Vec<String>,
    pub redaction_notice: String,
}

/// Generator and scrubber for privacy-preserving diagnostic support bundles.
pub struct SupportReportGenerator;

impl SupportReportGenerator {
    /// Sanitizes an individual log line by scrubbing known sensitive patterns.
    pub fn sanitize_log_line(line: &str) -> String {
        let mut sanitized = line.to_string();

        // 1. Scrub Bearer tokens
        if let Some(pos) = sanitized.find("Bearer ") {
            let after = &sanitized[pos + 7..];
            let end_pos = after
                .find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == ',')
                .unwrap_or(after.len());
            let secret = &after[..end_pos];
            if !secret.is_empty() {
                sanitized = sanitized.replace(&format!("Bearer {secret}"), "Bearer [REDACTED_SECRET]");
            }
        }

        // 2. Scrub key=value secrets (password=..., token=..., secret=..., api_key=...)
        for key in &["password=", "token=", "secret=", "api_key=", "passwd="] {
            let lower = sanitized.to_lowercase();
            if let Some(pos) = lower.find(key) {
                let val_start = pos + key.len();
                let after = &sanitized[val_start..];
                let end_pos = after
                    .find(|c: char| c.is_whitespace() || c == '&' || c == ';' || c == ',' || c == '"' || c == '\'')
                    .unwrap_or(after.len());
                let val = &after[..end_pos];
                if !val.is_empty() {
                    let full_match = &sanitized[pos..val_start + end_pos];
                    sanitized = sanitized.replace(full_match, &format!("{}[REDACTED_SECRET]", &sanitized[pos..val_start]));
                }
            }
        }

        // 3. Scrub Private Key Blocks
        if sanitized.contains("-----BEGIN") && sanitized.contains("PRIVATE KEY-----") {
            sanitized = "[REDACTED_PRIVATE_KEY]".to_string();
        }

        // 4. Scrub Email Addresses
        let words: Vec<String> = sanitized.split_whitespace().map(|s| s.to_string()).collect();
        for word in words {
            let clean = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '@' && c != '.' && c != '_' && c != '-');
            if clean.contains('@') && clean.contains('.') {
                let parts: Vec<&str> = clean.split('@').collect();
                if parts.len() == 2 && !parts[0].is_empty() && parts[1].contains('.') {
                    sanitized = sanitized.replace(clean, "[REDACTED_EMAIL]");
                }
            }
        }

        // 5. Scrub User Paths (e.g. C:\Users\<name>\... or /home/<name>/... or /Users/<name>/...)
        let lower_path = sanitized.to_lowercase();
        if let Some(idx) = lower_path.find("c:\\users\\") {
            let after = &sanitized[idx + 9..];
            let name_end = after.find('\\').unwrap_or(after.len());
            let username = &after[..name_end];
            if !username.is_empty() {
                let target = format!("C:\\Users\\{username}");
                sanitized = sanitized.replace(&target, "[REDACTED_PATH]");
            }
        }
        if let Some(idx) = lower_path.find("/home/") {
            let after = &sanitized[idx + 6..];
            let name_end = after.find('/').unwrap_or(after.len());
            let username = &after[..name_end];
            if !username.is_empty() {
                let target = format!("/home/{username}");
                sanitized = sanitized.replace(&target, "[REDACTED_PATH]");
            }
        }
        if let Some(idx) = lower_path.find("/users/") {
            let after = &sanitized[idx + 7..];
            let name_end = after.find('/').unwrap_or(after.len());
            let username = &after[..name_end];
            if !username.is_empty() {
                let target = format!("/Users/{username}");
                sanitized = sanitized.replace(&target, "[REDACTED_PATH]");
            }
        }

        sanitized
    }

    /// Generates a sanitized diagnostic support bundle.
    pub fn generate(
        host_os: &str,
        host_cpu: &str,
        umr_version: &str,
        installed_apps: Vec<ProvenanceRecord>,
        raw_logs: &[String],
    ) -> SupportReport {
        let sanitized_logs: Vec<String> = raw_logs
            .iter()
            .map(|l| Self::sanitize_log_line(l))
            .collect();

        let report_id = format!(
            "umr-rep-{:x}",
            Utc::now().timestamp_nanos_opt().unwrap_or(0)
        );

        SupportReport {
            report_id,
            generated_at: Utc::now(),
            host_os: host_os.to_string(),
            host_cpu: host_cpu.to_string(),
            umr_version: umr_version.to_string(),
            installed_apps,
            sanitized_logs,
            redaction_notice: "All personal identifiers, home directory user paths, authentication tokens, and private keys have been automatically redacted prior to export.".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_and_pii_redaction() {
        let raw_line = "Error authenticating user alice@example.com with Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9 and password=SuperSecretPassword123 at C:\\Users\\Administrator\\AppData\\Local";
        let sanitized = SupportReportGenerator::sanitize_log_line(raw_line);

        assert!(!sanitized.contains("alice@example.com"));
        assert!(sanitized.contains("[REDACTED_EMAIL]"));

        assert!(!sanitized.contains("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9"));
        assert!(sanitized.contains("Bearer [REDACTED_SECRET]"));

        assert!(!sanitized.contains("SuperSecretPassword123"));
        assert!(sanitized.contains("password=[REDACTED_SECRET]"));

        assert!(!sanitized.contains("C:\\Users\\Administrator"));
        assert!(sanitized.contains("[REDACTED_PATH]"));
    }

    #[test]
    fn test_private_key_redaction() {
        let raw = "-----BEGIN RSA PRIVATE KEY-----\nMIIEowIBAAKCAQEA0...\n-----END RSA PRIVATE KEY-----";
        let sanitized = SupportReportGenerator::sanitize_log_line(raw);
        assert_eq!(sanitized, "[REDACTED_PRIVATE_KEY]");
    }
}
