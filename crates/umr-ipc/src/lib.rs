pub mod handler;
pub mod protocol;
pub mod server;

pub use handler::RpcHandler;
pub use protocol::{
    InputEventPayload, PackagePathParams, PrepareSessionParams, RpcError, RpcRequest,
    RpcResponse, SendInputParams, SessionIdParams,
};
pub use server::{run_stdio_server, HttpServer};

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};
    use serde_json::Value;
    use tempfile::tempdir;

    use umr_backend_reference::ReferenceAndroidBackend;
    use umr_orchestrator::RuntimeOrchestrator;

    fn setup_test_handler() -> Arc<RpcHandler> {
        let sandbox = tempdir().unwrap();
        let backend = ReferenceAndroidBackend::new(sandbox.path());
        let mut orch = RuntimeOrchestrator::new(PathBuf::from("mock-worker")).with_macos_override(false);
        orch.register_android(Box::new(backend));
        Arc::new(RpcHandler::new(Arc::new(Mutex::new(orch))))
    }

    #[test]
    fn test_rpc_ping_and_unknown_method() {
        let handler = setup_test_handler();

        // 1. Valid umr.ping
        let raw_ping = r#"{"jsonrpc":"2.0","id":1,"method":"umr.ping"}"#;
        let resp_str = handler.handle_raw_message(raw_ping);
        let resp: Value = serde_json::from_str(&resp_str).unwrap();
        assert_eq!(resp["jsonrpc"], "2.0");
        assert_eq!(resp["id"], 1);
        assert_eq!(resp["result"]["status"], "pong");

        // 2. Unknown method
        let raw_unknown = r#"{"jsonrpc":"2.0","id":2,"method":"non.existent"}"#;
        let err_resp_str = handler.handle_raw_message(raw_unknown);
        let err_resp: Value = serde_json::from_str(&err_resp_str).unwrap();
        assert_eq!(err_resp["error"]["code"], -32601);
        assert!(err_resp["error"]["message"].to_string().contains("Method 'non.existent' not found"));
    }

    #[test]
    fn test_rpc_backends_list() {
        let handler = setup_test_handler();
        let raw_req = r#"{"jsonrpc":"2.0","id":10,"method":"umr.backends.list"}"#;
        let resp_str = handler.handle_raw_message(raw_req);
        let resp: Value = serde_json::from_str(&resp_str).unwrap();
        assert!(resp["result"].is_array());
        let backends = resp["result"].as_array().unwrap();
        assert_eq!(backends.len(), 1);
        assert_eq!(backends[0]["backend_id"], "android-reference-backend");
    }

    #[test]
    fn test_rpc_session_subsystems() {
        let sandbox = tempdir().unwrap();
        let backend = ReferenceAndroidBackend::new(sandbox.path());
        let mut orch = RuntimeOrchestrator::new(PathBuf::from("mock-worker")).with_macos_override(false);
        orch.register_android(Box::new(backend));

        let pkg = umr_core::PackageMetadata {
            format: umr_core::PackageFormat::Apk,
            file_path: "/dummy.apk".to_string(),
            file_size_bytes: 100,
            sha256_hash: "hash".to_string(),
            details: umr_core::PackageDetails::Android(umr_core::AndroidPackageInfo {
                package_name: "org.umr.test".to_string(),
                version_code: Some(1),
                version_name: Some("1.0".to_string()),
                min_sdk_version: Some(21),
                target_sdk_version: Some(34),
                main_activity: Some("Main".to_string()),
                native_abis: vec![umr_core::TargetAbi::X86_64],
                is_pure_dex: false,
            }),
            inspected_at: chrono::Utc::now(),
        };
        let session_id = orch
            .prepare_session(pkg, "android-reference-backend")
            .expect("prepare session");
        orch.launch_session(&session_id).expect("launch session");

        let handler = Arc::new(RpcHandler::new(Arc::new(Mutex::new(orch))));

        // 1. umr.session.send_input
        let raw_input = format!(
            r#"{{
                "jsonrpc": "2.0",
                "id": 42,
                "method": "umr.session.send_input",
                "params": {{
                    "session_id": "{}",
                    "event": {{
                        "type": "pointer",
                        "x": 0.521,
                        "y": 0.812,
                        "action": "down"
                    }}
                }}
            }}"#,
            session_id
        );

        let resp_str = handler.handle_raw_message(&raw_input);
        let resp: Value = serde_json::from_str(&resp_str).unwrap();
        assert_eq!(resp["result"]["status"], "delivered");

        // 2. umr.session.capture_frame
        let raw_capture = format!(
            r#"{{
                "jsonrpc": "2.0",
                "id": 43,
                "method": "umr.session.capture_frame",
                "params": {{
                    "session_id": "{}"
                }}
            }}"#,
            session_id
        );

        let frame_resp_str = handler.handle_raw_message(&raw_capture);
        let frame_resp: Value = serde_json::from_str(&frame_resp_str).unwrap();
        assert_eq!(frame_resp["result"]["width"], 360);
        assert_eq!(frame_resp["result"]["height"], 640);
        assert_eq!(frame_resp["result"]["format"], "rgba8888");
        assert!(!frame_resp["result"]["base64_rgba"].as_str().unwrap().is_empty());

        // 3. umr.session.wipe_storage
        let raw_wipe = format!(
            r#"{{
                "jsonrpc": "2.0",
                "id": 44,
                "method": "umr.session.wipe_storage",
                "params": {{
                    "session_id": "{}"
                }}
            }}"#,
            session_id
        );

        let wipe_resp_str = handler.handle_raw_message(&raw_wipe);
        let wipe_resp: Value = serde_json::from_str(&wipe_resp_str).unwrap();
        assert_eq!(wipe_resp["result"]["status"], "wiped");
    }

    #[test]
    fn test_http_server_rpc_flow() {
        let handler = setup_test_handler();
        let mut server = HttpServer::start(handler, 0).expect("Start HTTP server on port 0");
        let port = server.port();
        assert!(port > 0);

        // Make HTTP POST request to /rpc over loopback TCP
        let mut stream = TcpStream::connect(format!("127.0.0.1:{}", port)).expect("Connect to HTTP server");
        let req_body = r#"{"jsonrpc":"2.0","id":99,"method":"umr.ping"}"#;
        let http_req = format!(
            "POST /rpc HTTP/1.1\r\n\
Host: 127.0.0.1:{}\r\n\
Content-Type: application/json\r\n\
Content-Length: {}\r\n\
Connection: close\r\n\
\r\n\
{}",
            port,
            req_body.len(),
            req_body
        );

        stream.write_all(http_req.as_bytes()).unwrap();

        let mut response_buf = String::new();
        stream.read_to_string(&mut response_buf).unwrap();

        assert!(response_buf.contains("200 OK"));
        assert!(response_buf.contains("\"status\":\"pong\""));
        assert!(response_buf.contains("Access-Control-Allow-Origin: *"));

        server.stop();
    }

    #[test]
    fn test_http_server_static_file_flow() {
        let handler = setup_test_handler();
        let mut server = HttpServer::start(handler, 0).expect("Start HTTP server on port 0");
        let port = server.port();

        let mut stream = TcpStream::connect(format!("127.0.0.1:{}", port)).expect("Connect to HTTP server");
        let http_req = format!(
            "GET /index.html HTTP/1.1\r\n\
Host: 127.0.0.1:{}\r\n\
Connection: close\r\n\
\r\n",
            port
        );
        stream.write_all(http_req.as_bytes()).unwrap();

        let mut response_buf = String::new();
        stream.read_to_string(&mut response_buf).unwrap();

        assert!(response_buf.contains("200 OK"));
        assert!(response_buf.contains("text/html"));
        assert!(response_buf.contains("Universal Mobile Runtime"));

        server.stop();
    }

    #[test]
    fn test_rpc_host_capabilities() {
        let handler = setup_test_handler();
        let raw_req = r#"{"jsonrpc":"2.0","id":100,"method":"umr.host.capabilities"}"#;
        let resp_str = handler.handle_raw_message(raw_req);
        let resp: Value = serde_json::from_str(&resp_str).unwrap();
        assert_eq!(resp["jsonrpc"], "2.0");
        assert_eq!(resp["id"], 100);
        assert!(resp["result"]["host_os"].is_string());
        assert!(resp["result"]["logical_cpu_cores"].as_u64().unwrap() > 0);
        assert!(resp["result"]["supported_guest_abis"].is_array());
    }

    #[test]
    fn test_rpc_package_management() {
        let sandbox = tempdir().unwrap();
        let orch = RuntimeOrchestrator::new(PathBuf::from("mock-worker")).with_macos_override(false);
        let handler = Arc::new(RpcHandler::with_sandbox(Arc::new(Mutex::new(orch)), sandbox.path().to_path_buf()));

        // 1. List packages (initially empty)
        let list_req = r#"{"jsonrpc":"2.0","id":101,"method":"umr.packages.list"}"#;
        let list_resp_str = handler.handle_raw_message(list_req);
        let list_resp: Value = serde_json::from_str(&list_resp_str).unwrap();
        assert_eq!(list_resp["result"].as_array().unwrap().len(), 0);

        // 2. Pre-populate a package in catalog
        let mut mgr = umr_backend_android::AndroidPackageManager::new(sandbox.path()).unwrap();
        let pkg = umr_core::PackageMetadata {
            format: umr_core::PackageFormat::Apk,
            file_path: "/sample/app.apk".to_string(),
            file_size_bytes: 2048,
            sha256_hash: "mockhash".to_string(),
            details: umr_core::PackageDetails::Android(umr_core::AndroidPackageInfo {
                package_name: "org.umr.catalog.test".to_string(),
                version_code: Some(1),
                version_name: Some("1.0".to_string()),
                min_sdk_version: Some(21),
                target_sdk_version: Some(34),
                main_activity: Some("MainActivity".to_string()),
                native_abis: vec![umr_core::TargetAbi::X86_64],
                is_pure_dex: false,
            }),
            inspected_at: chrono::Utc::now(),
        };
        mgr.install_package(&pkg).unwrap();

        // 3. List packages again (now has 1)
        let list_resp_str2 = handler.handle_raw_message(list_req);
        let list_resp2: Value = serde_json::from_str(&list_resp_str2).unwrap();
        assert_eq!(list_resp2["result"].as_array().unwrap().len(), 1);
        assert_eq!(list_resp2["result"][0]["package_name"], "org.umr.catalog.test");

        // 4. Uninstall package via RPC
        let uninst_req = r#"{"jsonrpc":"2.0","id":102,"method":"umr.packages.uninstall","params":{"package_name":"org.umr.catalog.test"}}"#;
        let uninst_resp_str = handler.handle_raw_message(uninst_req);
        let uninst_resp: Value = serde_json::from_str(&uninst_resp_str).unwrap();
        assert_eq!(uninst_resp["result"]["status"], "uninstalled");

        // 5. Verify catalog empty again
        let list_resp_str3 = handler.handle_raw_message(list_req);
        let list_resp3: Value = serde_json::from_str(&list_resp_str3).unwrap();
        assert_eq!(list_resp3["result"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_rpc_ios_gap_report_error_path() {
        let handler = setup_test_handler();
        let raw_req = r#"{"jsonrpc":"2.0","id":103,"method":"umr.ios.gap_report","params":{"package_path":"/nonexistent/app.ipa"}}"#;
        let resp_str = handler.handle_raw_message(raw_req);
        let resp: Value = serde_json::from_str(&resp_str).unwrap();
        assert_eq!(resp["jsonrpc"], "2.0");
        assert_eq!(resp["id"], 103);
        assert!(resp["error"].is_object());
    }
}
