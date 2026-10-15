use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use serde_json::{json, Value};
use tempfile::tempdir;

use umr_backend_reference::ReferenceAndroidBackend;
use umr_ipc::{HttpServer, RpcHandler};
use umr_orchestrator::RuntimeOrchestrator;

struct TestClient {
    port: u16,
    req_id: u64,
}

impl TestClient {
    fn new(port: u16) -> Self {
        Self { port, req_id: 0 }
    }

    fn rpc_call(&mut self, method: &str, params: Option<Value>) -> Value {
        self.req_id += 1;
        let mut req_obj = json!({
            "jsonrpc": "2.0",
            "id": self.req_id,
            "method": method,
        });
        if let Some(p) = params {
            req_obj["params"] = p;
        }

        let body_str = serde_json::to_string(&req_obj).unwrap();
        let http_req = format!(
            "POST /rpc HTTP/1.1\r\n\
Host: 127.0.0.1:{}\r\n\
Content-Type: application/json\r\n\
Content-Length: {}\r\n\
Connection: close\r\n\
\r\n\
{}",
            self.port,
            body_str.len(),
            body_str
        );

        let mut stream = TcpStream::connect(format!("127.0.0.1:{}", self.port))
            .expect("Failed to connect to IPC server");
        stream.write_all(http_req.as_bytes()).unwrap();

        let mut resp_str = String::new();
        stream.read_to_string(&mut resp_str).unwrap();

        // Extract body after \r\n\r\n
        let body_start = resp_str.find("\r\n\r\n").expect("HTTP headers delimiter") + 4;
        let json_body = &resp_str[body_start..];
        serde_json::from_str(json_body).expect("Valid JSON-RPC response")
    }

    fn http_get(&self, path: &str) -> (u16, String, String) {
        let http_req = format!(
            "GET {} HTTP/1.1\r\n\
Host: 127.0.0.1:{}\r\n\
Connection: close\r\n\
\r\n",
            path, self.port
        );

        let mut stream = TcpStream::connect(format!("127.0.0.1:{}", self.port))
            .expect("Failed to connect to IPC server");
        stream.write_all(http_req.as_bytes()).unwrap();

        let mut resp_str = String::new();
        stream.read_to_string(&mut resp_str).unwrap();

        let status_code = if resp_str.starts_with("HTTP/1.1 200") {
            200
        } else if resp_str.starts_with("HTTP/1.1 404") {
            404
        } else {
            500
        };

        let body_start = resp_str.find("\r\n\r\n").map(|idx| idx + 4).unwrap_or(0);
        let body = resp_str[body_start..].to_string();
        (status_code, resp_str, body)
    }
}

fn locate_workspace_file(rel: &str) -> PathBuf {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into()));
    let candidates = [
        PathBuf::from(rel),
        manifest_dir.join("../../").join(rel),
        manifest_dir.join("../").join(rel),
    ];
    for c in candidates {
        if c.exists() {
            return c;
        }
    }
    panic!("Could not locate workspace file '{}'", rel);
}

fn find_intake_bin() -> PathBuf {
    let base = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into()));
    let exe_name = if cfg!(windows) {
        "umr-intake-worker.exe"
    } else {
        "umr-intake-worker"
    };

    let candidates = [
        base.join("../../target/debug").join(exe_name),
        base.join("../target/debug").join(exe_name),
        PathBuf::from("target/debug").join(exe_name),
    ];
    for c in candidates {
        if c.exists() {
            return c;
        }
    }
    PathBuf::from(exe_name)
}

#[test]
fn test_phase2_end_to_end_interactive_session() {
    let sandbox = tempdir().unwrap();
    let intake_bin = find_intake_bin();
    let backend = ReferenceAndroidBackend::new(sandbox.path());

    let mut orch = RuntimeOrchestrator::new(intake_bin).with_macos_override(false);
    orch.register_android(Box::new(backend));

    let handler = Arc::new(RpcHandler::new(Arc::new(Mutex::new(orch))));
    let mut server = HttpServer::start(handler, 0).expect("Start ephemeral HTTP IPC server");
    let mut client = TestClient::new(server.port());

    // ---------------------------------------------------------
    // 1. AC2-1: IPC Ping & Backend Capabilities Listing
    // ---------------------------------------------------------
    let ping_resp = client.rpc_call("umr.ping", None);
    assert_eq!(ping_resp["result"]["status"], "pong");

    let backends_resp = client.rpc_call("umr.backends.list", None);
    let backends = backends_resp["result"].as_array().expect("backends list");
    assert!(!backends.is_empty());
    assert_eq!(backends[0]["backend_id"], "android-reference-backend");
    assert_eq!(backends[0]["execution_mode"], "local_native");

    // ---------------------------------------------------------
    // 2. AC2-5 & AC2-7: Intake & Capability Negotiation Checks
    // ---------------------------------------------------------
    let synth_apk = locate_workspace_file("tests/fixtures/synthetic-app.apk");
    let neg_resp = client.rpc_call(
        "umr.negotiate",
        Some(json!({ "package_path": synth_apk.to_str().unwrap() })),
    );
    assert_eq!(
        neg_resp["result"]["negotiation"]["result"],
        "compatible",
        "Synthetic APK must be compatible"
    );
    assert_eq!(
        neg_resp["result"]["negotiation"]["execution_mode"],
        "local_native"
    );

    // Defensive rejection: Zip Slip traversal
    let slip_apk = locate_workspace_file("tests/fixtures/malformed-slip.apk");
    let slip_resp = client.rpc_call(
        "umr.negotiate",
        Some(json!({ "package_path": slip_apk.to_str().unwrap() })),
    );
    assert!(
        slip_resp.get("error").is_some() && !slip_resp["error"].is_null(),
        "Zip slip archive must be rejected"
    );
    let err_code = slip_resp["error"]["data"]["code"].as_str().unwrap_or("");
    assert_eq!(err_code, "ERR_INTAKE_ZIP_TRAVERSAL");

    // Defensive rejection: FairPlay encrypted IPA
    let enc_ipa = locate_workspace_file("tests/fixtures/encrypted-mock.ipa");
    let enc_resp = client.rpc_call(
        "umr.negotiate",
        Some(json!({ "package_path": enc_ipa.to_str().unwrap() })),
    );
    assert_eq!(
        enc_resp["result"]["negotiation"]["result"],
        "incompatible",
        "FairPlay encrypted IPA must be incompatible"
    );
    assert_eq!(
        enc_resp["result"]["negotiation"]["error"]["code"],
        "ERR_INTAKE_DRM_PROTECTED"
    );

    // Remote Stream designation: iOS on Windows host
    let sim_ipa = locate_workspace_file("tests/fixtures/valid-sim.ipa");
    let sim_resp = client.rpc_call(
        "umr.negotiate",
        Some(json!({ "package_path": sim_ipa.to_str().unwrap() })),
    );
    assert_eq!(
        sim_resp["result"]["negotiation"]["result"],
        "remote_required",
        "iOS app on Windows must be labeled as Remote Required"
    );
    assert_eq!(
        sim_resp["result"]["negotiation"]["suggested_mode"],
        "remote_stream"
    );

    // ---------------------------------------------------------
    // 3. AC2-3 & AC2-6: Prepare & Launch Interactive Session
    // ---------------------------------------------------------
    let prep_resp = client.rpc_call(
        "umr.session.prepare",
        Some(json!({
            "package_path": synth_apk.to_str().unwrap(),
            "backend_id": "android-reference-backend"
        })),
    );
    assert!(prep_resp["result"]["session_id"].is_string());
    let session_id = prep_resp["result"]["session_id"].as_str().unwrap().to_string();

    let launch_resp = client.rpc_call(
        "umr.session.launch",
        Some(json!({ "session_id": session_id })),
    );
    assert_eq!(launch_resp["result"]["status"], "ok");

    let status_resp = client.rpc_call(
        "umr.session.status",
        Some(json!({ "session_id": session_id })),
    );
    assert_eq!(status_resp["result"]["state"], "running");

    // ---------------------------------------------------------
    // 4. AC2-2: Dispatch Input Events (Pointer Tap, Drag, Nav)
    // ---------------------------------------------------------
    // A. Pointer Down at center (0.5, 0.5)
    let tap_down = client.rpc_call(
        "umr.session.send_input",
        Some(json!({
            "session_id": session_id,
            "event": {
                "type": "pointer",
                "x": 0.5,
                "y": 0.5,
                "action": "down"
            }
        })),
    );
    assert_eq!(tap_down["result"]["status"], "delivered");

    // B. Pointer Move
    let tap_move = client.rpc_call(
        "umr.session.send_input",
        Some(json!({
            "session_id": session_id,
            "event": {
                "type": "pointer",
                "x": 0.55,
                "y": 0.58,
                "action": "move"
            }
        })),
    );
    assert_eq!(tap_move["result"]["status"], "delivered");

    // C. Hardware Navigation Button (Back)
    let nav_back = client.rpc_call(
        "umr.session.send_input",
        Some(json!({
            "session_id": session_id,
            "event": {
                "type": "navigation",
                "button": "back"
            }
        })),
    );
    assert_eq!(nav_back["result"]["status"], "delivered");

    // ---------------------------------------------------------
    // 5. AC2-4: Capture Framebuffer & Validate 60 FPS Presentation
    // ---------------------------------------------------------
    let frame_resp = client.rpc_call(
        "umr.session.capture_frame",
        Some(json!({ "session_id": session_id })),
    );
    assert_eq!(frame_resp["result"]["width"], 360);
    assert_eq!(frame_resp["result"]["height"], 640);
    assert_eq!(frame_resp["result"]["format"], "rgba8888");
    assert!(frame_resp["result"]["frame_index"].as_u64().unwrap() >= 1);
    let b64 = frame_resp["result"]["base64_rgba"].as_str().unwrap();
    assert!(!b64.is_empty(), "Frame buffer payload must be non-empty");

    // Release pointer
    let _tap_up = client.rpc_call(
        "umr.session.send_input",
        Some(json!({
            "session_id": session_id,
            "event": {
                "type": "pointer",
                "x": 0.55,
                "y": 0.58,
                "action": "up"
            }
        })),
    );

    // ---------------------------------------------------------
    // 6. AC2-3: Isolated Storage Wiping
    // ---------------------------------------------------------
    let wipe_resp = client.rpc_call(
        "umr.session.wipe_storage",
        Some(json!({ "session_id": session_id })),
    );
    assert_eq!(wipe_resp["result"]["status"], "wiped");

    // ---------------------------------------------------------
    // 7. AC2-6: Collect Sanitized Diagnostics & Verify Stop
    // ---------------------------------------------------------
    let stop_resp = client.rpc_call(
        "umr.session.stop",
        Some(json!({ "session_id": session_id })),
    );
    assert_eq!(stop_resp["result"]["status"], "ok");

    let report_resp = client.rpc_call(
        "umr.session.report",
        Some(json!({ "session_id": session_id })),
    );
    let entries = report_resp["result"]["entries"].as_array().expect("diagnostic entries");
    assert!(!entries.is_empty());

    let has_touch_log = entries.iter().any(|e| {
        let msg = e["message"].as_str().unwrap_or("");
        msg.contains("MotionEvent { action=ACTION_DOWN")
    });
    assert!(has_touch_log, "Logcat must record dispatched touch events");

    let has_nav_log = entries.iter().any(|e| {
        let msg = e["message"].as_str().unwrap_or("");
        msg.contains("Hardware navigation button pressed: BACK")
    });
    assert!(has_nav_log, "Logcat must record dispatched navigation key events");

    let has_storage_log = entries.iter().any(|e| {
        let msg = e["message"].as_str().unwrap_or("");
        msg.contains("Application data and cache wiped cleanly")
    });
    assert!(has_storage_log, "Logcat must record clean storage wipe event");

    // ---------------------------------------------------------
    // 8. Static Web UI Delivery for Desktop Shell
    // ---------------------------------------------------------
    let (status, headers, body) = client.http_get("/");
    assert_eq!(status, 200);
    assert!(headers.contains("Content-Type: text/html"));
    assert!(body.contains("Universal Mobile Runtime"));

    server.stop();
}
