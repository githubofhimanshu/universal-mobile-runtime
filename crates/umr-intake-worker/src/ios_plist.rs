use std::io::Cursor;

use plist::Value;

use umr_core::{DiagnosticError, ErrorCode, UmrResult};

#[derive(Debug, Clone)]
pub struct ParsedInfoPlist {
    pub bundle_identifier: String,
    pub bundle_version: Option<String>,
    pub bundle_short_version: Option<String>,
    pub minimum_os_version: Option<String>,
    pub executable_name: Option<String>,
}

/// Parses an iOS `Info.plist` (handling both binary and XML plist formats).
pub fn parse_info_plist(bytes: &[u8]) -> UmrResult<ParsedInfoPlist> {
    if bytes.is_empty() {
        return Err(DiagnosticError::new(
            ErrorCode::ErrIntakeMissingManifest,
            "Info.plist is empty",
        ));
    }

    let cursor = Cursor::new(bytes);
    let value = Value::from_reader(cursor).map_err(|e| {
        DiagnosticError::new(
            ErrorCode::ErrIntakeParseFailure,
            format!("Failed to parse Info.plist: {}", e),
        )
    })?;

    let dict = value.as_dictionary().ok_or_else(|| {
        DiagnosticError::new(
            ErrorCode::ErrIntakeParseFailure,
            "Root element of Info.plist is not a dictionary",
        )
    })?;

    let bundle_identifier = dict
        .get("CFBundleIdentifier")
        .and_then(|v| v.as_string())
        .ok_or_else(|| {
            DiagnosticError::new(
                ErrorCode::ErrIntakeParseFailure,
                "Missing 'CFBundleIdentifier' in Info.plist",
            )
        })?
        .to_string();

    let bundle_version = dict
        .get("CFBundleVersion")
        .and_then(|v| v.as_string())
        .map(|s| s.to_string());

    let bundle_short_version = dict
        .get("CFBundleShortVersionString")
        .and_then(|v| v.as_string())
        .map(|s| s.to_string());

    let minimum_os_version = dict
        .get("MinimumOSVersion")
        .and_then(|v| v.as_string())
        .map(|s| s.to_string());

    let executable_name = dict
        .get("CFBundleExecutable")
        .and_then(|v| v.as_string())
        .map(|s| s.to_string());

    Ok(ParsedInfoPlist {
        bundle_identifier,
        bundle_version,
        bundle_short_version,
        minimum_os_version,
        executable_name,
    })
}

