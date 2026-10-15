use serde::{Deserialize, Serialize};
use umr_core::{
    CpuArchitecture, DiagnosticError, EncryptionStatus, ErrorCode, IosPackageInfo, PackageDetails,
    PackageMetadata, UmrResult,
};

/// Support level of a framework or system capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrameworkSupportLevel {
    /// Fully implemented or natively available on target runtime.
    Supported,
    /// Partially shimmed with known stubbed methods or degraded fidelity.
    Partial,
    /// Known missing framework or API gap preventing local execution.
    UnsupportedGap,
    /// Proprietary Apple cloud or hardware service requiring live Apple servers / Secure Enclave.
    CloudServiceBlocked,
}

impl std::fmt::Display for FrameworkSupportLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Supported => write!(f, "Supported"),
            Self::Partial => write!(f, "Partial / Shimmed"),
            Self::UnsupportedGap => write!(f, "Unsupported Gap"),
            Self::CloudServiceBlocked => write!(f, "Blocked (Requires External Apple Service)"),
        }
    }
}

/// Audit entry for an individual framework dependency.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameworkAuditEntry {
    pub name: String,
    pub support_level: FrameworkSupportLevel,
    pub description: String,
    pub mitigation: Option<String>,
}

/// Overall verdict of package compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompatibilityVerdict {
    /// Fully compatible with local runtime.
    Compatible,
    /// Partially compatible; some non-critical framework calls may be stubbed.
    Degraded,
    /// Cannot execute locally due to missing framework APIs or architecture mismatch.
    Incompatible,
    /// Blocked by commercial FairPlay DRM encryption.
    DrmBlocked,
}

/// Comprehensive gap analysis report detailing framework, signing, and hardware dependencies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GapAnalysisReport {
    pub bundle_identifier: String,
    pub bundle_version: String,
    pub minimum_os_version: String,
    pub architectures: Vec<CpuArchitecture>,
    pub is_simulator_build: bool,
    pub encryption_status: EncryptionStatus,
    pub verdict: CompatibilityVerdict,
    pub framework_audits: Vec<FrameworkAuditEntry>,
    pub missing_services: Vec<String>,
    pub actionable_recommendations: Vec<String>,
}

/// System analyzing iOS application dependencies and cataloging runtime feasibility gaps.
pub struct FrameworkGapTracker;

impl FrameworkGapTracker {
    /// Analyzes an inspected package and produces a detailed gap report.
    pub fn audit_package(pkg: &PackageMetadata) -> UmrResult<GapAnalysisReport> {
        let info = match &pkg.details {
            PackageDetails::Ios(info) => info,
            _ => {
                return Err(DiagnosticError::new(
                    ErrorCode::ErrIntakeParseFailure,
                    "Expected iOS package metadata for gap analysis",
                ));
            }
        };

        // 1. Check DRM encryption
        let mut verdict = CompatibilityVerdict::Compatible;
        let mut recommendations = Vec::new();
        let mut missing_services = Vec::new();

        if let EncryptionStatus::FairPlayEncrypted { cryptid } = info.encryption {
            return Ok(GapAnalysisReport {
                bundle_identifier: info.bundle_identifier.clone(),
                bundle_version: info.bundle_version.clone().unwrap_or_else(|| "1.0".to_string()),
                minimum_os_version: info.minimum_os_version.clone().unwrap_or_else(|| "1.0".to_string()),
                architectures: info.architectures.clone(),
                is_simulator_build: info.is_simulator_build,
                encryption_status: info.encryption.clone(),
                verdict: CompatibilityVerdict::DrmBlocked,
                framework_audits: vec![],
                missing_services: vec!["FairPlay DRM Decryption".to_string()],
                actionable_recommendations: vec![
                    format!("Binary is encrypted with Apple FairPlay DRM (cryptid: {}).", cryptid),
                    "Commercial App Store packages cannot execute on non-Apple hardware without hardware decryption keys.".to_string(),
                    "Provide an unencrypted development build or simulator package.".to_string(),
                ],
            });
        }

        // 2. Perform framework audit
        let detected_frameworks = Self::infer_framework_dependencies(info);
        let mut audits = Vec::new();

        for fw in &detected_frameworks {
            let audit = Self::classify_framework(fw);
            if audit.support_level == FrameworkSupportLevel::UnsupportedGap {
                verdict = CompatibilityVerdict::Incompatible;
                missing_services.push(fw.clone());
            } else if audit.support_level == FrameworkSupportLevel::CloudServiceBlocked {
                missing_services.push(fw.clone());
                if verdict != CompatibilityVerdict::Incompatible {
                    verdict = CompatibilityVerdict::Degraded;
                }
            } else if audit.support_level == FrameworkSupportLevel::Partial && verdict == CompatibilityVerdict::Compatible {
                verdict = CompatibilityVerdict::Degraded;
            }
            audits.push(audit);
        }

        // 3. Architecture & OS checks
        let is_legacy_32bit = info.architectures.contains(&CpuArchitecture::Armv7)
            && !info.architectures.contains(&CpuArchitecture::Arm64);

        if info.is_simulator_build {
            if verdict == CompatibilityVerdict::Degraded && missing_services.is_empty() {
                verdict = CompatibilityVerdict::Compatible;
            }
            recommendations.push("Package contains x86_64/ARM64 simulator symbols. Executable on macOS via Track A (Apple Simulator runtime).".to_string());
        } else if is_legacy_32bit {
            if verdict == CompatibilityVerdict::Degraded && missing_services.is_empty() {
                verdict = CompatibilityVerdict::Compatible;
            }
            recommendations.push("Package contains legacy 32-bit ARMv7 symbols. Executable via Track B High-Level Emulation (HLE) on Windows/macOS/Linux.".to_string());
        } else {
            recommendations.push("Package contains modern 64-bit ARM device symbols. On non-macOS hosts, execution requires Track C (Tethered physical device or remote Mac stream).".to_string());
            if verdict == CompatibilityVerdict::Compatible {
                verdict = CompatibilityVerdict::Degraded;
            }
        }

        Ok(GapAnalysisReport {
            bundle_identifier: info.bundle_identifier.clone(),
            bundle_version: info.bundle_version.clone().unwrap_or_else(|| "1.0".to_string()),
            minimum_os_version: info.minimum_os_version.clone().unwrap_or_else(|| "1.0".to_string()),
            architectures: info.architectures.clone(),
            is_simulator_build: info.is_simulator_build,
            encryption_status: info.encryption.clone(),
            verdict,
            framework_audits: audits,
            missing_services,
            actionable_recommendations: recommendations,
        })
    }

    fn infer_framework_dependencies(info: &IosPackageInfo) -> Vec<String> {
        let mut frameworks = vec![
            "Foundation".to_string(),
            "CoreGraphics".to_string(),
            "UIKit".to_string(),
        ];

        // For modern iOS packages (iOS 8+), add standard modern framework dependencies
        let min_os_major = info
            .minimum_os_version
            .as_deref()
            .and_then(|v| v.split('.').next())
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(2);

        if min_os_major >= 8 {
            frameworks.push("Metal".to_string());
            frameworks.push("CoreData".to_string());
        }

        if min_os_major >= 11 {
            frameworks.push("StoreKit".to_string());
            frameworks.push("UserNotifications".to_string());
        }

        frameworks
    }

    fn classify_framework(name: &str) -> FrameworkAuditEntry {
        match name {
            "Foundation" | "CoreFoundation" => FrameworkAuditEntry {
                name: name.to_string(),
                support_level: FrameworkSupportLevel::Supported,
                description: "Fundamental Objective-C / Swift data structures and runtime primitives.".to_string(),
                mitigation: None,
            },
            "CoreGraphics" => FrameworkAuditEntry {
                name: name.to_string(),
                support_level: FrameworkSupportLevel::Supported,
                description: "2D rendering, pathing, and geometry primitives.".to_string(),
                mitigation: None,
            },
            "UIKit" => FrameworkAuditEntry {
                name: name.to_string(),
                support_level: FrameworkSupportLevel::Partial,
                description: "Windowing, views, touch dispatch, and UI components.".to_string(),
                mitigation: Some("Core views and controllers supported in HLE; full UIKit available in Simulator and Remote tracks.".to_string()),
            },
            "OpenGLES" => FrameworkAuditEntry {
                name: name.to_string(),
                support_level: FrameworkSupportLevel::Supported,
                description: "OpenGL ES 1.1 / 2.0 graphics pipeline.".to_string(),
                mitigation: None,
            },
            "Metal" => FrameworkAuditEntry {
                name: name.to_string(),
                support_level: FrameworkSupportLevel::UnsupportedGap,
                description: "Hardware-accelerated Apple Metal shading and compute API.".to_string(),
                mitigation: Some("Requires Apple Silicon / macOS Metal driver or remote device stream.".to_string()),
            },
            "CoreData" => FrameworkAuditEntry {
                name: name.to_string(),
                support_level: FrameworkSupportLevel::Partial,
                description: "Object relational mapping and SQLite database layer.".to_string(),
                mitigation: Some("Standard SQLite queries supported; advanced CoreData schema migrations unsupported in HLE.".to_string()),
            },
            "StoreKit" => FrameworkAuditEntry {
                name: name.to_string(),
                support_level: FrameworkSupportLevel::CloudServiceBlocked,
                description: "Apple In-App Purchases and receipt validation.".to_string(),
                mitigation: Some("Requires StoreKit test configuration file or remote runner session.".to_string()),
            },
            "UserNotifications" | "PushKit" => FrameworkAuditEntry {
                name: name.to_string(),
                support_level: FrameworkSupportLevel::CloudServiceBlocked,
                description: "Apple Push Notification Service (APNs).".to_string(),
                mitigation: Some("Live push notifications require Apple APNs gateway; local notifications only in simulator.".to_string()),
            },
            other => FrameworkAuditEntry {
                name: other.to_string(),
                support_level: FrameworkSupportLevel::UnsupportedGap,
                description: format!("Unshimmed framework '{}'", other),
                mitigation: Some("Execute in Apple Simulator runtime or tethered device.".to_string()),
            },
        }
    }
}
