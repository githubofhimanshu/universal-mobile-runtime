use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Structured log level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// A sanitized, structured log entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticLogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub source: String,
    pub message: String,
}

/// Sanitizer that automatically redacts secrets and credentials from logs.
pub struct LogSanitizer;

impl LogSanitizer {
    /// Redacts sensitive keywords, bearer tokens, and secret patterns.
    pub fn sanitize(input: &str) -> String {
        let mut sanitized = input.to_string();

        let sensitive_keywords = [
            "password=",
            "password:",
            "passwd=",
            "token=",
            "token:",
            "bearer ",
            "secret=",
            "api_key=",
            "apikey=",
            "authorization: ",
        ];

        for keyword in sensitive_keywords {
            let mut search_from = 0;
            while let Some(pos) = sanitized[search_from..].to_lowercase().find(keyword) {
                let abs_pos = search_from + pos;
                let val_start = abs_pos + keyword.len();
                // Find end of token / word (space, quote, comma, semicolon, newline)
                let val_len = sanitized[val_start..]
                    .find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == ',' || c == ';')
                    .unwrap_or(sanitized.len() - val_start);

                if val_len > 0 {
                    sanitized.replace_range(val_start..val_start + val_len, "[REDACTED]");
                    search_from = val_start + "[REDACTED]".len();
                } else {
                    search_from = val_start;
                }
            }
        }

        sanitized
    }
}

/// A collection of diagnostic logs for a session.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DiagnosticReport {
    pub session_id: String,
    pub package_id: String,
    pub created_at: DateTime<Utc>,
    pub entries: Vec<DiagnosticLogEntry>,
}

impl DiagnosticReport {
    pub fn new(session_id: impl Into<String>, package_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            package_id: package_id.into(),
            created_at: Utc::now(),
            entries: Vec::new(),
        }
    }

    pub fn log(&mut self, level: LogLevel, source: impl Into<String>, message: impl AsRef<str>) {
        let clean_msg = LogSanitizer::sanitize(message.as_ref());
        self.entries.push(DiagnosticLogEntry {
            timestamp: Utc::now(),
            level,
            source: source.into(),
            message: clean_msg,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_redaction() {
        let raw = "User authenticated with Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9 and password=SuperSecret123; host=localhost";
        let cleaned = LogSanitizer::sanitize(raw);
        assert!(!cleaned.contains("SuperSecret123"));
        assert!(!cleaned.contains("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9"));
        assert!(cleaned.contains("password=[REDACTED]"));
        assert!(cleaned.contains("Bearer [REDACTED]"));
    }
}
