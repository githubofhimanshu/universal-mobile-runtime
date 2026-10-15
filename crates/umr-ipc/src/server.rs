use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::handler::RpcHandler;

/// Runs a stdio-based IPC server reading newline-delimited JSON-RPC from stdin.
pub fn run_stdio_server(handler: Arc<RpcHandler>) {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let reader = stdin.lock();

    for line_result in reader.lines() {
        if let Ok(line) = line_result {
            if line.trim().is_empty() {
                continue;
            }
            let resp = handler.handle_raw_message(&line);
            let _ = writeln!(stdout, "{}", resp);
            let _ = stdout.flush();
        } else {
            break;
        }
    }
}

/// An embedded HTTP / JSON-RPC server listening on localhost.
pub struct HttpServer {
    port: u16,
    is_running: Arc<AtomicBool>,
    server_thread: Option<JoinHandle<()>>,
}

impl HttpServer {
    /// Starts the HTTP server on `127.0.0.1:<port>`. If `port` is 0, an ephemeral port is selected.
    pub fn start(handler: Arc<RpcHandler>, port: u16) -> std::io::Result<Self> {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", port))?;
        listener.set_nonblocking(true)?;
        let bound_port = listener.local_addr()?.port();

        let is_running = Arc::new(AtomicBool::new(true));
        let running_clone = Arc::clone(&is_running);

        let thread_handle = thread::spawn(move || {
            while running_clone.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let h = Arc::clone(&handler);
                        thread::spawn(move || {
                            let _ = handle_http_client(stream, h);
                        });
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(Self {
            port: bound_port,
            is_running,
            server_thread: Some(thread_handle),
        })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn stop(&mut self) {
        self.is_running.store(false, Ordering::SeqCst);
        if let Some(th) = self.server_thread.take() {
            let _ = th.join();
        }
    }
}

impl Drop for HttpServer {
    fn drop(&mut self) {
        self.stop();
    }
}

fn handle_http_client(mut stream: TcpStream, handler: Arc<RpcHandler>) -> std::io::Result<()> {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));

    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request_line = String::new();
    if reader.read_line(&mut request_line)? == 0 {
        return Ok(());
    }

    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        return Ok(());
    }

    let method = parts[0];
    let path = parts[1];

    let mut content_length: usize = 0;
    loop {
        let mut header_line = String::new();
        if reader.read_line(&mut header_line)? == 0 {
            break;
        }
        let trimmed = header_line.trim();
        if trimmed.is_empty() {
            break;
        }
        if trimmed.to_lowercase().starts_with("content-length:") {
            if let Some(len_str) = trimmed.split(':').nth(1) {
                content_length = len_str.trim().parse().unwrap_or(0);
            }
        }
    }

    // Handle CORS preflight
    if method == "OPTIONS" {
        let response = "HTTP/1.1 204 No Content\r\n\
Access-Control-Allow-Origin: *\r\n\
Access-Control-Allow-Methods: POST, GET, OPTIONS\r\n\
Access-Control-Allow-Headers: Content-Type, Authorization\r\n\
\r\n";
        stream.write_all(response.as_bytes())?;
        return Ok(());
    }

    // Handle Health check
    if method == "GET" && path == "/health" {
        let body = "{\"status\":\"healthy\",\"service\":\"umr-ipc\"}";
        let response = format!(
            "HTTP/1.1 200 OK\r\n\
Content-Type: application/json\r\n\
Access-Control-Allow-Origin: *\r\n\
Content-Length: {}\r\n\
\r\n\
{}",
            body.len(),
            body
        );
        stream.write_all(response.as_bytes())?;
        return Ok(());
    }

    // Handle Static UI file serving for Desktop Shell
    if method == "GET" {
        let clean_path = path.split('?').next().unwrap_or(path);
        let rel_path = if clean_path == "/" {
            "index.html"
        } else {
            clean_path.trim_start_matches('/')
        };

        // Guard against directory traversal
        if !rel_path.contains("..") {
            let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
            let base_manifest = std::path::PathBuf::from(manifest_dir);

            let candidates = [
                std::path::PathBuf::from("apps/desktop-shell/dist").join(rel_path),
                std::path::PathBuf::from("apps/desktop-shell").join(rel_path),
                std::path::PathBuf::from("../../apps/desktop-shell/dist").join(rel_path),
                std::path::PathBuf::from("../../apps/desktop-shell").join(rel_path),
                base_manifest.join("../../apps/desktop-shell/dist").join(rel_path),
                base_manifest.join("../../apps/desktop-shell").join(rel_path),
            ];

            for candidate in &candidates {
                if candidate.exists() && candidate.is_file() {
                    if let Ok(bytes) = std::fs::read(candidate) {
                        let content_type = mime_for_path(candidate);
                        let response = format!(
                            "HTTP/1.1 200 OK\r\n\
Content-Type: {}\r\n\
Access-Control-Allow-Origin: *\r\n\
Content-Length: {}\r\n\
\r\n",
                            content_type,
                            bytes.len()
                        );
                        stream.write_all(response.as_bytes())?;
                        stream.write_all(&bytes)?;
                        return Ok(());
                    }
                }
            }
        }
    }

    // Handle RPC POST
    if method == "POST" && (path == "/rpc" || path == "/") {
        let mut body_bytes = vec![0u8; content_length];
        reader.read_exact(&mut body_bytes)?;
        let body_str = String::from_utf8_lossy(&body_bytes);

        let rpc_res = handler.handle_raw_message(&body_str);

        let response = format!(
            "HTTP/1.1 200 OK\r\n\
Content-Type: application/json\r\n\
Access-Control-Allow-Origin: *\r\n\
Content-Length: {}\r\n\
\r\n\
{}",
            rpc_res.len(),
            rpc_res
        );
        stream.write_all(response.as_bytes())?;
        return Ok(());
    }

    // Default 404
    let not_found = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n";
    stream.write_all(not_found.as_bytes())?;
    Ok(())
}

fn mime_for_path(path: &std::path::Path) -> &'static str {
    match path.extension().and_then(|s| s.to_str()).unwrap_or("") {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "application/javascript; charset=utf-8",
        "json" => "application/json",
        "png" => "image/png",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        _ => "application/octet-stream",
    }
}

