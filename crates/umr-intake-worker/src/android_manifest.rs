use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Cursor;

use umr_core::{AndroidPackageInfo, DiagnosticError, ErrorCode, TargetAbi, UmrResult};

const AXML_MAGIC: u32 = 0x00080003;
const CHUNK_STRING_POOL: u32 = 0x001c0001;
const CHUNK_START_ELEMENT: u32 = 0x00100102;

/// Parses `AndroidManifest.xml` (either compiled AXML or plain text XML) and extracts package metadata.
pub fn parse_manifest(manifest_bytes: &[u8], all_entries: &[String]) -> UmrResult<AndroidPackageInfo> {
    if manifest_bytes.is_empty() {
        return Err(DiagnosticError::new(
            ErrorCode::ErrIntakeMissingManifest,
            "AndroidManifest.xml is empty",
        ));
    }

    // Detect native ABIs from lib/ entries in the APK archive
    let mut native_abis = Vec::new();
    for entry in all_entries {
        if entry.starts_with("lib/") {
            let parts: Vec<&str> = entry.split('/').collect();
            if parts.len() >= 3 && !parts[1].is_empty() {
                let abi = TargetAbi::from_lib_folder_name(parts[1]);
                if abi != TargetAbi::Unknown && !native_abis.contains(&abi) {
                    native_abis.push(abi);
                }
            }
        }
    }
    let is_pure_dex = native_abis.is_empty();

    // Check if binary AXML format or plain text
    if manifest_bytes.len() >= 4 {
        let magic = (&manifest_bytes[0..4]).read_u32::<LittleEndian>().unwrap_or(0);
        if magic == AXML_MAGIC || magic == 0x00080000 || magic == 0x00080001 {
            return parse_axml(manifest_bytes, native_abis, is_pure_dex);
        }
    }

    // Fallback: Plain text XML manifest
    parse_text_manifest(manifest_bytes, native_abis, is_pure_dex)
}

/// Fallback text manifest parser for uncompiled or synthetic test APKs.
fn parse_text_manifest(
    bytes: &[u8],
    native_abis: Vec<TargetAbi>,
    is_pure_dex: bool,
) -> UmrResult<AndroidPackageInfo> {
    let content = String::from_utf8_lossy(bytes);

    let package_name = extract_xml_attr(&content, "package")
        .ok_or_else(|| {
            DiagnosticError::new(
                ErrorCode::ErrIntakeParseFailure,
                "Failed to find 'package' attribute in AndroidManifest.xml",
            )
        })?;

    let version_code = extract_xml_attr(&content, "versionCode")
        .and_then(|v| v.parse::<u64>().ok());

    let version_name = extract_xml_attr(&content, "versionName");

    let min_sdk_version = extract_xml_attr(&content, "minSdkVersion")
        .and_then(|v| v.parse::<u32>().ok());

    let target_sdk_version = extract_xml_attr(&content, "targetSdkVersion")
        .and_then(|v| v.parse::<u32>().ok());

    let main_activity = extract_xml_attr(&content, "android:name");

    Ok(AndroidPackageInfo {
        package_name,
        version_code,
        version_name,
        min_sdk_version,
        target_sdk_version,
        main_activity,
        native_abis,
        is_pure_dex,
    })
}

fn extract_xml_attr(xml: &str, attr: &str) -> Option<String> {
    let pattern = format!("{}=\"", attr);
    if let Some(pos) = xml.find(&pattern) {
        let start = pos + pattern.len();
        if let Some(end) = xml[start..].find('"') {
            return Some(xml[start..start + end].to_string());
        }
    }
    let alt_pattern = format!("{}='", attr);
    if let Some(pos) = xml.find(&alt_pattern) {
        let start = pos + alt_pattern.len();
        if let Some(end) = xml[start..].find('\'') {
            return Some(xml[start..start + end].to_string());
        }
    }
    None
}

/// Binary AXML (Android Binary XML) Parser
fn parse_axml(
    bytes: &[u8],
    native_abis: Vec<TargetAbi>,
    is_pure_dex: bool,
) -> UmrResult<AndroidPackageInfo> {
    let mut cursor = Cursor::new(bytes);
    let total_len = bytes.len() as u64;

    // Read header: magic (4 bytes) and file size (4 bytes)
    let _magic = cursor.read_u32::<LittleEndian>().map_err(read_err)?;
    let _file_size = cursor.read_u32::<LittleEndian>().map_err(read_err)?;

    let mut string_pool = Vec::new();
    let mut package_name = None;
    let mut version_code = None;
    let mut version_name = None;
    let mut min_sdk_version = None;
    let mut target_sdk_version = None;

    while cursor.position() < total_len {
        let chunk_start = cursor.position();
        let chunk_type = match cursor.read_u32::<LittleEndian>() {
            Ok(t) => t,
            Err(_) => break,
        };
        let chunk_size = match cursor.read_u32::<LittleEndian>() {
            Ok(s) => s as u64,
            Err(_) => break,
        };

        if chunk_size < 8 || chunk_start + chunk_size > total_len {
            break;
        }

        match chunk_type {
            CHUNK_STRING_POOL => {
                string_pool = parse_string_pool(&bytes[chunk_start as usize..(chunk_start + chunk_size) as usize])?;
            }
            CHUNK_START_ELEMENT => {
                let _line_number = cursor.read_u32::<LittleEndian>().map_err(read_err)?;
                let _comment_idx = cursor.read_u32::<LittleEndian>().map_err(read_err)?;
                let _ns_idx = cursor.read_u32::<LittleEndian>().map_err(read_err)?;
                let name_idx = cursor.read_u32::<LittleEndian>().map_err(read_err)?;
                let _attr_start = cursor.read_u16::<LittleEndian>().map_err(read_err)?;
                let _attr_size = cursor.read_u16::<LittleEndian>().map_err(read_err)?;
                let attr_count = cursor.read_u16::<LittleEndian>().map_err(read_err)? as usize;
                let _id_index = cursor.read_u16::<LittleEndian>().map_err(read_err)?;
                let _class_index = cursor.read_u16::<LittleEndian>().map_err(read_err)?;
                let _style_index = cursor.read_u16::<LittleEndian>().map_err(read_err)?;

                let elem_name = string_pool.get(name_idx as usize).cloned().unwrap_or_default();

                for _ in 0..attr_count {
                    let _attr_ns = cursor.read_u32::<LittleEndian>().map_err(read_err)?;
                    let attr_name_idx = cursor.read_u32::<LittleEndian>().map_err(read_err)?;
                    let attr_raw_val_idx = cursor.read_u32::<LittleEndian>().map_err(read_err)?;
                    let _attr_type = cursor.read_u32::<LittleEndian>().map_err(read_err)?;
                    let attr_data = cursor.read_u32::<LittleEndian>().map_err(read_err)?;

                    let attr_name = string_pool.get(attr_name_idx as usize).map(|s| s.as_str()).unwrap_or("");
                    let attr_raw = string_pool.get(attr_raw_val_idx as usize).cloned();

                    if elem_name == "manifest" {
                        if attr_name == "package" {
                            package_name = attr_raw;
                        } else if attr_name == "versionCode" {
                            version_code = Some(attr_data as u64);
                        } else if attr_name == "versionName" {
                            version_name = attr_raw;
                        }
                    } else if elem_name == "uses-sdk" {
                        if attr_name == "minSdkVersion" {
                            min_sdk_version = Some(attr_data);
                        } else if attr_name == "targetSdkVersion" {
                            target_sdk_version = Some(attr_data);
                        }
                    }
                }
            }
            _ => {
                // Skip unknown chunk
            }
        }

        let _ = cursor.set_position(chunk_start + chunk_size);
    }

    let pkg = package_name.unwrap_or_else(|| "unknown.package".to_string());

    Ok(AndroidPackageInfo {
        package_name: pkg,
        version_code,
        version_name,
        min_sdk_version,
        target_sdk_version,
        main_activity: None,
        native_abis,
        is_pure_dex,
    })
}

fn parse_string_pool(chunk: &[u8]) -> UmrResult<Vec<String>> {
    let mut cursor = Cursor::new(chunk);
    let _chunk_type = cursor.read_u32::<LittleEndian>().map_err(read_err)?;
    let _chunk_size = cursor.read_u32::<LittleEndian>().map_err(read_err)?;
    let string_count = cursor.read_u32::<LittleEndian>().map_err(read_err)? as usize;
    let _style_count = cursor.read_u32::<LittleEndian>().map_err(read_err)?;
    let flags = cursor.read_u32::<LittleEndian>().map_err(read_err)?;
    let strings_start = cursor.read_u32::<LittleEndian>().map_err(read_err)? as usize;
    let _styles_start = cursor.read_u32::<LittleEndian>().map_err(read_err)?;

    let is_utf8 = (flags & (1 << 8)) != 0;

    let mut offsets = Vec::with_capacity(string_count);
    for _ in 0..string_count {
        let offset = cursor.read_u32::<LittleEndian>().map_err(read_err)? as usize;
        offsets.push(offset);
    }

    let mut strings = Vec::with_capacity(string_count);
    for offset in offsets {
        let abs_start = strings_start + offset;
        if abs_start >= chunk.len() {
            strings.push(String::new());
            continue;
        }

        if is_utf8 {
            // Read UTF-8 string: skip length header, read null-terminated or bounded bytes
            let slice = &chunk[abs_start..];
            let mut i = 0;
            // Skip 1 or 2 byte length indicators
            if !slice.is_empty() && slice[0] >= 0x80 {
                i += 2;
            } else {
                i += 1;
            }
            if i < slice.len() && slice[i] >= 0x80 {
                i += 2;
            } else {
                i += 1;
            }
            let str_bytes = &slice[i..];
            let len = str_bytes.iter().position(|&b| b == 0).unwrap_or(str_bytes.len());
            strings.push(String::from_utf8_lossy(&str_bytes[..len]).to_string());
        } else {
            // Read UTF-16LE string
            let slice = &chunk[abs_start..];
            let mut u16_chars = Vec::new();
            let mut i = 2; // skip length header
            while i + 1 < slice.len() {
                let code = (&slice[i..i + 2]).read_u16::<LittleEndian>().unwrap_or(0);
                if code == 0 {
                    break;
                }
                u16_chars.push(code);
                i += 2;
            }
            strings.push(String::from_utf16_lossy(&u16_chars));
        }
    }

    Ok(strings)
}

fn read_err(e: std::io::Error) -> DiagnosticError {
    DiagnosticError::new(
        ErrorCode::ErrIntakeParseFailure,
        format!("Error parsing binary AndroidManifest.xml: {}", e),
    )
}

