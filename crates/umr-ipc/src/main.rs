use std::path::PathBuf;
use std::process::exit;
use std::sync::{Arc, Mutex};

use clap::Parser;
use umr_core::{IAndroidBackend, IIosBackend};

use umr_backend_android::AndroidVirtualizationBackend;
use umr_backend_ios::{IosHleBackend, IosRemoteStreamBackend, IosSimulatorBackend};
use umr_backend_reference::ReferenceAndroidBackend;
use umr_ipc::{run_stdio_server, HttpServer, RpcHandler};
use umr_orchestrator::{find_intake_binary, RuntimeOrchestrator};

#[derive(Parser, Debug)]
#[command(name = "umr-ipc")]
#[command(about = "Universal Mobile Runtime JSON-RPC 2.0 IPC Gateway", long_about = None)]
struct Cli {
    /// Listen for JSON-RPC messages on standard input/output
    #[arg(long, default_value_t = false)]
    stdio: bool,

    /// Start localhost HTTP/JSON-RPC server
    #[arg(long, default_value_t = true)]
    http: bool,

    /// Port to bind the HTTP server (default: 9090)
    #[arg(short, long, default_value_t = 9090)]
    port: u16,

    /// Sandbox root directory for session workspaces
    #[arg(long, default_value = "target/umr-sandbox")]
    sandbox: PathBuf,
}

fn main() {
    let cli = Cli::parse();

    let intake_bin = match find_intake_binary() {
        Ok(bin) => bin,
        Err(err) => {
            eprintln!("Warning: Failed to locate intake worker binary: {}", err);
            PathBuf::from("umr-intake-worker.exe")
        }
    };

    let mut orch = RuntimeOrchestrator::new(intake_bin);
    // Register reference Android backend
    let ref_backend = ReferenceAndroidBackend::new(&cli.sandbox);
    orch.register_android(Box::new(ref_backend));

    // Register production Android virtualization backend
    match AndroidVirtualizationBackend::new_auto(&cli.sandbox) {
        Ok(virt_backend) => {
            println!("[umr-ipc] Registered virtualization backend: {}", virt_backend.id());
            orch.register_android(Box::new(virt_backend));
        }
        Err(err) => {
            eprintln!("[umr-ipc] Warning: Virtualization backend unavailable: {}", err);
        }
    }

    // Register iOS tiered backends
    let sim_backend = IosSimulatorBackend::new(&cli.sandbox);
    println!("[umr-ipc] Registered iOS backend: {}", sim_backend.id());
    orch.register_ios(Box::new(sim_backend));

    let hle_backend = IosHleBackend::new(&cli.sandbox);
    println!("[umr-ipc] Registered iOS backend: {}", hle_backend.id());
    orch.register_ios(Box::new(hle_backend));

    let remote_backend = IosRemoteStreamBackend::new();
    println!("[umr-ipc] Registered iOS backend: {}", remote_backend.id());
    orch.register_ios(Box::new(remote_backend));

    let handler = Arc::new(RpcHandler::with_sandbox(Arc::new(Mutex::new(orch)), cli.sandbox.clone()));

    if cli.stdio {
        eprintln!("[umr-ipc] Running in STDIO mode...");
        run_stdio_server(handler);
        exit(0);
    }

    if cli.http {
        println!("[umr-ipc] Starting HTTP JSON-RPC gateway on 127.0.0.1:{}", cli.port);
        match HttpServer::start(handler, cli.port) {
            Ok(server) => {
                println!("[umr-ipc] Listening on http://127.0.0.1:{}/rpc", server.port());
                println!("[umr-ipc] Health check at http://127.0.0.1:{}/health", server.port());
                println!("[umr-ipc] Press Ctrl+C to terminate gateway.");

                // Keep main thread alive
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }
            }
            Err(e) => {
                eprintln!("[umr-ipc] Failed to start HTTP server: {}", e);
                exit(1);
            }
        }
    }
}

