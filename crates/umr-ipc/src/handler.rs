use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use umr_core::{NegotiationResult, SessionId};
use umr_orchestrator::RuntimeOrchestrator;

use crate::protocol::{
    PackageNameParams, PackagePathParams, PrepareSessionParams, RpcError, RpcRequest,
    RpcResponse, SendInputParams, SessionIdParams, UpgradePackageParams,
};

/// Thread-safe RPC Request Dispatcher interfacing with the Runtime Orchestrator.
pub struct RpcHandler {
    orchestrator: Arc<Mutex<RuntimeOrchestrator>>,
    sandbox_root: PathBuf,
}

impl RpcHandler {
    pub fn new(orchestrator: Arc<Mutex<RuntimeOrchestrator>>) -> Self {
        Self::with_sandbox(orchestrator, PathBuf::from("target/umr-sandbox"))
    }

    pub fn with_sandbox(orchestrator: Arc<Mutex<RuntimeOrchestrator>>, sandbox_root: PathBuf) -> Self {
        Self {
            orchestrator,
            sandbox_root,
        }
    }

    /// Dispatches a raw JSON string and returns a formatted JSON-RPC response string.
    pub fn handle_raw_message(&self, raw: &str) -> String {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return String::new();
        }

        let request: RpcRequest = match serde_json::from_str(trimmed) {
            Ok(req) => req,
            Err(e) => {
                let err_resp = RpcResponse::error(None, RpcError::parse_error(format!("Invalid JSON: {}", e)));
                return serde_json::to_string(&err_resp).unwrap();
            }
        };

        let response = self.handle_request(request);
        serde_json::to_string(&response).unwrap_or_else(|_| "{\"jsonrpc\":\"2.0\",\"error\":{\"code\":-32603,\"message\":\"Serialization error\"}}".to_string())
    }

    /// Processes a single parsed JSON-RPC request.
    pub fn handle_request(&self, req: RpcRequest) -> RpcResponse {
        if req.jsonrpc != "2.0" {
            return RpcResponse::error(
                req.id,
                RpcError::invalid_request("Invalid JSON-RPC protocol version; expected '2.0'"),
            );
        }

        let id = req.id;
        let method = req.method.as_str();

        match method {
            "umr.ping" => RpcResponse::success(id, json!({ "status": "pong", "version": "0.1.0" })),

            "umr.backends.list" => {
                let orch = self.orchestrator.lock().unwrap();
                let backends = orch.list_backends();
                RpcResponse::success(id, json!(backends))
            }

            "umr.intake.inspect" => {
                let params: PackagePathParams = match parse_params(req.params) {
                    Ok(p) => p,
                    Err(e) => return RpcResponse::error(id, e),
                };

                let orch = self.orchestrator.lock().unwrap();
                match orch.inspect_package(Path::new(&params.package_path)) {
                    Ok(metadata) => RpcResponse::success(id, json!(metadata)),
                    Err(err) => RpcResponse::error(
                        id,
                        RpcError::execution_error(err.message.clone(), Some(json!(err))),
                    ),
                }
            }

            "umr.negotiate" => {
                let params: PackagePathParams = match parse_params(req.params) {
                    Ok(p) => p,
                    Err(e) => return RpcResponse::error(id, e),
                };

                let orch = self.orchestrator.lock().unwrap();
                match orch.inspect_package(Path::new(&params.package_path)) {
                    Ok(metadata) => {
                        let neg = orch.negotiate(&metadata);
                        RpcResponse::success(
                            id,
                            json!({
                                "metadata": metadata,
                                "negotiation": neg
                            }),
                        )
                    }
                    Err(err) => RpcResponse::error(
                        id,
                        RpcError::execution_error(err.message.clone(), Some(json!(err))),
                    ),
                }
            }

            "umr.session.prepare" => {
                let params: PrepareSessionParams = match parse_params(req.params) {
                    Ok(p) => p,
                    Err(e) => return RpcResponse::error(id, e),
                };

                let mut orch = self.orchestrator.lock().unwrap();
                let metadata = match orch.inspect_package(Path::new(&params.package_path)) {
                    Ok(m) => m,
                    Err(err) => {
                        return RpcResponse::error(
                            id,
                            RpcError::execution_error(err.message.clone(), Some(json!(err))),
                        );
                    }
                };

                let backend_id = if let Some(bid) = params.backend_id {
                    bid
                } else {
                    match orch.negotiate(&metadata) {
                        NegotiationResult::Compatible { backend_id, .. } => backend_id,
                        NegotiationResult::RemoteRequired { reason, .. } => {
                            return RpcResponse::error(
                                id,
                                RpcError::execution_error(
                                    format!("Remote device required: {}", reason),
                                    Some(json!({ "reason": reason })),
                                ),
                            );
                        }
                        NegotiationResult::Incompatible { error } => {
                            return RpcResponse::error(
                                id,
                                RpcError::execution_error(error.message.clone(), Some(json!(error))),
                            );
                        }
                    }
                };

                match orch.prepare_session(metadata, &backend_id) {
                    Ok(session_id) => RpcResponse::success(
                        id,
                        json!({
                            "session_id": session_id.to_string(),
                            "backend_id": backend_id,
                            "state": "preparing"
                        }),
                    ),
                    Err(err) => RpcResponse::error(
                        id,
                        RpcError::execution_error(err.message.clone(), Some(json!(err))),
                    ),
                }
            }

            "umr.session.launch" => {
                let params: SessionIdParams = match parse_params(req.params) {
                    Ok(p) => p,
                    Err(e) => return RpcResponse::error(id, e),
                };

                let mut orch = self.orchestrator.lock().unwrap();
                let session_id = SessionId(params.session_id);
                match orch.launch_session(&session_id) {
                    Ok(()) => RpcResponse::success(id, json!({ "status": "ok", "state": "running" })),
                    Err(err) => RpcResponse::error(
                        id,
                        RpcError::execution_error(err.message.clone(), Some(json!(err))),
                    ),
                }
            }

            "umr.session.stop" => {
                let params: SessionIdParams = match parse_params(req.params) {
                    Ok(p) => p,
                    Err(e) => return RpcResponse::error(id, e),
                };

                let mut orch = self.orchestrator.lock().unwrap();
                let session_id = SessionId(params.session_id);
                match orch.stop_session(&session_id) {
                    Ok(()) => RpcResponse::success(id, json!({ "status": "ok", "state": "terminated" })),
                    Err(err) => RpcResponse::error(
                        id,
                        RpcError::execution_error(err.message.clone(), Some(json!(err))),
                    ),
                }
            }

            "umr.session.status" => {
                let params: SessionIdParams = match parse_params(req.params) {
                    Ok(p) => p,
                    Err(e) => return RpcResponse::error(id, e),
                };

                let orch = self.orchestrator.lock().unwrap();
                let session_id = SessionId(params.session_id);
                match orch.poll_session(&session_id) {
                    Ok(state) => RpcResponse::success(id, json!({ "state": state })),
                    Err(err) => RpcResponse::error(
                        id,
                        RpcError::execution_error(err.message.clone(), Some(json!(err))),
                    ),
                }
            }

            "umr.session.report" => {
                let params: SessionIdParams = match parse_params(req.params) {
                    Ok(p) => p,
                    Err(e) => return RpcResponse::error(id, e),
                };

                let orch = self.orchestrator.lock().unwrap();
                let session_id = SessionId(params.session_id);
                match orch.get_report(&session_id) {
                    Ok(report) => RpcResponse::success(id, json!(report)),
                    Err(err) => RpcResponse::error(
                        id,
                        RpcError::execution_error(err.message.clone(), Some(json!(err))),
                    ),
                }
            }

            "umr.session.send_input" => {
                let params: SendInputParams = match parse_params(req.params) {
                    Ok(p) => p,
                    Err(e) => return RpcResponse::error(id, e),
                };

                let core_event: umr_core::InputEvent = match params.event.try_into() {
                    Ok(ev) => ev,
                    Err(msg) => return RpcResponse::error(id, RpcError::invalid_params(msg)),
                };

                let mut orch = self.orchestrator.lock().unwrap();
                let session_id = SessionId(params.session_id);
                match orch.send_input(&session_id, core_event) {
                    Ok(()) => RpcResponse::success(id, json!({ "status": "delivered" })),
                    Err(err) => RpcResponse::error(
                        id,
                        RpcError::execution_error(err.message.clone(), Some(json!(err))),
                    ),
                }
            }

            "umr.session.capture_frame" => {
                let params: SessionIdParams = match parse_params(req.params) {
                    Ok(p) => p,
                    Err(e) => return RpcResponse::error(id, e),
                };

                let orch = self.orchestrator.lock().unwrap();
                let session_id = SessionId(params.session_id);
                match orch.capture_frame(&session_id) {
                    Ok(Some(frame)) => {
                        let b64 = frame.to_base64();
                        RpcResponse::success(
                            id,
                            json!({
                                "width": frame.width,
                                "height": frame.height,
                                "format": "rgba8888",
                                "timestamp_ms": frame.timestamp_ms,
                                "frame_index": frame.frame_index,
                                "base64_rgba": b64,
                            }),
                        )
                    }
                    Ok(None) => RpcResponse::error(
                        id,
                        RpcError::execution_error("No frame buffer available for this session", None),
                    ),
                    Err(err) => RpcResponse::error(
                        id,
                        RpcError::execution_error(err.message.clone(), Some(json!(err))),
                    ),
                }
            }

            "umr.session.wipe_storage" => {
                let params: SessionIdParams = match parse_params(req.params) {
                    Ok(p) => p,
                    Err(e) => return RpcResponse::error(id, e),
                };

                let mut orch = self.orchestrator.lock().unwrap();
                let session_id = SessionId(params.session_id);
                match orch.wipe_storage(&session_id) {
                    Ok(()) => RpcResponse::success(id, json!({ "status": "wiped" })),
                    Err(err) => RpcResponse::error(
                        id,
                        RpcError::execution_error(err.message.clone(), Some(json!(err))),
                    ),
                }
            }

            "umr.host.capabilities" => {
                let caps = umr_backend_android::HostPlatformDetector::detect_current();
                RpcResponse::success(id, json!(caps))
            }

            "umr.packages.list" => {
                match umr_backend_android::AndroidPackageManager::new(&self.sandbox_root) {
                    Ok(mgr) => {
                        let pkgs = mgr.list_packages();
                        RpcResponse::success(id, json!(pkgs))
                    }
                    Err(err) => RpcResponse::error(
                        id,
                        RpcError::execution_error(err.message.clone(), Some(json!(err))),
                    ),
                }
            }

            "umr.packages.install" => {
                let params: PackagePathParams = match parse_params(req.params) {
                    Ok(p) => p,
                    Err(e) => return RpcResponse::error(id, e),
                };

                let orch = self.orchestrator.lock().unwrap();
                let meta = match orch.inspect_package(Path::new(&params.package_path)) {
                    Ok(m) => m,
                    Err(err) => {
                        return RpcResponse::error(
                            id,
                            RpcError::execution_error(err.message.clone(), Some(json!(err))),
                        );
                    }
                };

                match umr_backend_android::AndroidPackageManager::new(&self.sandbox_root) {
                    Ok(mut mgr) => match mgr.install_package(&meta) {
                        Ok(record) => RpcResponse::success(id, json!(record)),
                        Err(err) => RpcResponse::error(
                            id,
                            RpcError::execution_error(err.message.clone(), Some(json!(err))),
                        ),
                    },
                    Err(err) => RpcResponse::error(
                        id,
                        RpcError::execution_error(err.message.clone(), Some(json!(err))),
                    ),
                }
            }

            "umr.packages.upgrade" => {
                let params: UpgradePackageParams = match parse_params(req.params) {
                    Ok(p) => p,
                    Err(e) => return RpcResponse::error(id, e),
                };

                let orch = self.orchestrator.lock().unwrap();
                let meta = match orch.inspect_package(Path::new(&params.package_path)) {
                    Ok(m) => m,
                    Err(err) => {
                        return RpcResponse::error(
                            id,
                            RpcError::execution_error(err.message.clone(), Some(json!(err))),
                        );
                    }
                };

                match umr_backend_android::AndroidPackageManager::new(&self.sandbox_root) {
                    Ok(mut mgr) => match mgr.upgrade_package(&meta, params.force) {
                        Ok(record) => RpcResponse::success(id, json!(record)),
                        Err(err) => RpcResponse::error(
                            id,
                            RpcError::execution_error(err.message.clone(), Some(json!(err))),
                        ),
                    },
                    Err(err) => RpcResponse::error(
                        id,
                        RpcError::execution_error(err.message.clone(), Some(json!(err))),
                    ),
                }
            }

            "umr.packages.uninstall" => {
                let params: PackageNameParams = match parse_params(req.params) {
                    Ok(p) => p,
                    Err(e) => return RpcResponse::error(id, e),
                };

                match umr_backend_android::AndroidPackageManager::new(&self.sandbox_root) {
                    Ok(mut mgr) => match mgr.uninstall_package(&params.package_name) {
                        Ok(()) => RpcResponse::success(
                            id,
                            json!({ "status": "uninstalled", "package": params.package_name }),
                        ),
                        Err(err) => RpcResponse::error(
                            id,
                            RpcError::execution_error(err.message.clone(), Some(json!(err))),
                        ),
                    },
                    Err(err) => RpcResponse::error(
                        id,
                        RpcError::execution_error(err.message.clone(), Some(json!(err))),
                    ),
                }
            }

            unknown => RpcResponse::error(id, RpcError::method_not_found(unknown)),
        }
    }
}

fn parse_params<T: serde::de::DeserializeOwned>(params: Option<Value>) -> Result<T, RpcError> {
    match params {
        Some(val) => serde_json::from_value(val).map_err(|e| {
            RpcError::invalid_params(format!("Invalid parameters structure: {}", e))
        }),
        None => Err(RpcError::invalid_params("Missing required 'params' field")),
    }
}
