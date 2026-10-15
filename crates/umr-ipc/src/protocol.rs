use serde::{Deserialize, Serialize};
use serde_json::Value;

/// JSON-RPC 2.0 Request object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

/// Standard JSON-RPC 2.0 Error object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl RpcError {
    pub fn parse_error(msg: impl Into<String>) -> Self {
        Self {
            code: -32700,
            message: msg.into(),
            data: None,
        }
    }

    pub fn invalid_request(msg: impl Into<String>) -> Self {
        Self {
            code: -32600,
            message: msg.into(),
            data: None,
        }
    }

    pub fn method_not_found(method: impl Into<String>) -> Self {
        Self {
            code: -32601,
            message: format!("Method '{}' not found", method.into()),
            data: None,
        }
    }

    pub fn invalid_params(msg: impl Into<String>) -> Self {
        Self {
            code: -32602,
            message: msg.into(),
            data: None,
        }
    }

    pub fn internal_error(msg: impl Into<String>, data: Option<Value>) -> Self {
        Self {
            code: -32603,
            message: msg.into(),
            data,
        }
    }

    pub fn execution_error(msg: impl Into<String>, data: Option<Value>) -> Self {
        Self {
            code: -32000,
            message: msg.into(),
            data,
        }
    }
}

/// JSON-RPC 2.0 Response object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcResponse {
    pub jsonrpc: String,
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
}

impl RpcResponse {
    pub fn success(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Option<Value>, error: RpcError) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(error),
        }
    }
}

/// Parameters for `umr.intake.inspect` and `umr.negotiate`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackagePathParams {
    pub package_path: String,
}

/// Parameters for `umr.session.prepare`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrepareSessionParams {
    pub package_path: String,
    pub backend_id: Option<String>,
}

/// Parameters for session actions (`launch`, `stop`, `status`, `report`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionIdParams {
    pub session_id: String,
}

/// Parameters for `umr.packages.uninstall`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageNameParams {
    pub package_name: String,
}

/// Parameters for `umr.packages.upgrade`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradePackageParams {
    pub package_path: String,
    #[serde(default)]
    pub force: bool,
}

/// Interactive input payload sent from Desktop Shell to an active session.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InputEventPayload {
    Pointer {
        x: f32, // normalized coordinate [0.0 .. 1.0]
        y: f32, // normalized coordinate [0.0 .. 1.0]
        action: String, // "down" | "move" | "up"
    },
    Key {
        key_code: u32,
        action: String, // "down" | "up"
    },
    Navigation {
        button: String, // "back" | "home" | "recents"
    },
}

/// Parameters for `umr.session.send_input`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendInputParams {
    pub session_id: String,
    pub event: InputEventPayload,
}

impl TryFrom<InputEventPayload> for umr_core::InputEvent {
    type Error = String;

    fn try_from(p: InputEventPayload) -> Result<Self, Self::Error> {
        match p {
            InputEventPayload::Pointer { x, y, action } => {
                let act = match action.to_lowercase().as_str() {
                    "down" => umr_core::PointerAction::Down,
                    "move" => umr_core::PointerAction::Move,
                    "up" => umr_core::PointerAction::Up,
                    "cancel" => umr_core::PointerAction::Cancel,
                    other => return Err(format!("Unknown pointer action '{}'", other)),
                };
                Ok(umr_core::InputEvent::pointer(x, y, act))
            }
            InputEventPayload::Key { key_code, action } => {
                let act = match action.to_lowercase().as_str() {
                    "down" => umr_core::KeyAction::Down,
                    "up" => umr_core::KeyAction::Up,
                    other => return Err(format!("Unknown key action '{}'", other)),
                };
                Ok(umr_core::InputEvent::key(key_code, act))
            }
            InputEventPayload::Navigation { button } => {
                let btn = match button.to_lowercase().as_str() {
                    "back" => umr_core::NavigationButton::Back,
                    "home" => umr_core::NavigationButton::Home,
                    "recents" | "app_switch" => umr_core::NavigationButton::Recents,
                    other => return Err(format!("Unknown navigation button '{}'", other)),
                };
                Ok(umr_core::InputEvent::navigation(btn))
            }
        }
    }
}

/// Structured frame buffer data returned by `umr.session.capture_frame`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameDataResponse {
    pub width: u32,
    pub height: u32,
    pub format: String,
    pub timestamp_ms: u64,
    pub frame_index: u64,
    pub base64_rgba: String,
}

/// Parameters for `umr.catalog.get`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogGetParams {
    pub app_id: String,
}

/// Parameters for `umr.provenance.verify`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyProvenanceParams {
    pub package_path: String,
    pub publisher_id: String,
    pub signature: String,
    pub version_code: u64,
}

/// Parameters for `umr.privacy.get_consent`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetConsentParams {
    pub host_endpoint: String,
}

/// Parameters for `umr.privacy.record_consent`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordConsentParams {
    pub user_id: String,
    pub host_endpoint: String,
    pub allow_input_streaming: bool,
    pub allow_frame_streaming: bool,
}

/// Parameters for `umr.license.accept`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseAcceptParams {
    pub app_id: String,
    pub version: String,
    pub license_type: String,
    pub terms_text: String,
}

/// Parameters for `umr.support.generate_report`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GenerateSupportReportParams {
    pub host_os: Option<String>,
    pub host_cpu: Option<String>,
    pub logs: Option<Vec<String>>,
}

